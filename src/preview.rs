use std::fs::File;
use std::io::Read;
use std::path::Path;

use ratatui::text::{Line, Span};
use syntect::parsing::{ParseState, ScopeStack, SyntaxDefinition, SyntaxReference, SyntaxSet};

use crate::just_syntax::JUST_SYNTAX_YAML;
use crate::kdl_syntax::KDL_SYNTAX_YAML;
use crate::mojo_syntax::MOJO_SYNTAX_YAML;
use crate::theme;

const MAX_BYTES: u64 = 128 * 1024;
const MAX_LINES: usize = 250;

/// Extensions that don't have (and aren't worth writing) a dedicated grammar,
/// but are a specific other language's syntax in disguise: `.sbatch` files
/// are SLURM batch scripts, i.e. bash with `#SBATCH` pragma comments.
const EXTENSION_ALIASES: &[(&str, &str)] = &[("sbatch", "sh")];

/// The base syntax set extended with languages `two-face` doesn't bundle
/// (currently KDL, justfiles, and Mojo). Falls back to the unmodified set if
/// a hand-written definition somehow fails to parse, rather than losing
/// every other language over one bad grammar.
///
/// Uses the `no_newlines` variant because `load_base` feeds `parse_line`
/// each line with its terminator already stripped (via `str::lines`). Some
/// grammars (e.g. bash's shebang/comment handling) rely on matching the
/// literal `\n` to pop out of a line-scoped context; paired with the
/// `newlines` variant, that pop never fires and the context leaks into every
/// subsequent line, silently swallowing the rest of the file as one comment.
pub fn build_syntax_set() -> SyntaxSet {
    let mut builder = two_face::syntax::extra_no_newlines().into_builder();
    let languages = [
        ("KDL", KDL_SYNTAX_YAML),
        ("Just", JUST_SYNTAX_YAML),
        ("Mojo", MOJO_SYNTAX_YAML),
    ];
    for (name, yaml) in languages {
        match SyntaxDefinition::load_from_str(yaml, false, None) {
            Ok(syntax) => builder.add(syntax),
            Err(error) => eprintln!("warning: could not load built-in {name} syntax: {error}"),
        }
    }
    builder.build()
}

/// Resolves the syntax for a file, trying (in order): the full file name
/// (so extensionless files like `justfile` or `Makefile` match by name),
/// the extension, an extension alias (e.g. `.sbatch` -> `.sh`), and finally
/// plain text.
fn resolve_syntax<'a>(syntax_set: &'a SyntaxSet, path: &Path) -> &'a SyntaxReference {
    let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");

    syntax_set
        .find_syntax_by_extension(file_name)
        .or_else(|| syntax_set.find_syntax_by_extension(ext))
        .or_else(|| {
            EXTENSION_ALIASES
                .iter()
                .find(|(from, _)| from.eq_ignore_ascii_case(ext))
                .and_then(|(_, to)| syntax_set.find_syntax_by_extension(to))
        })
        .unwrap_or_else(|| syntax_set.find_syntax_plain_text())
}

#[derive(Clone, Debug)]
pub struct PreviewMatch {
    pub line_number: u64,
    pub match_byte_offsets: Vec<(usize, usize)>,
}

pub struct PreviewResult {
    pub lines: Vec<Line<'static>>,
}

pub fn load_base(syntax_set: &SyntaxSet, path: &Path) -> PreviewResult {
    let metadata = match path.metadata() {
        Ok(metadata) => metadata,
        Err(error) => {
            return PreviewResult {
                lines: vec![Line::from(Span::styled(
                    format!("Cannot inspect file: {error}"),
                    theme::DEFAULT_STYLE,
                ))],
            };
        }
    };

    if !metadata.is_file() {
        return PreviewResult {
            lines: vec![Line::from(Span::styled(
                "Not a regular file",
                theme::DEFAULT_STYLE,
            ))],
        };
    }

    let mut file = match File::open(path) {
        Ok(file) => file,
        Err(error) => {
            return PreviewResult {
                lines: vec![Line::from(Span::styled(
                    format!("Cannot open file: {error}"),
                    theme::DEFAULT_STYLE,
                ))],
            };
        }
    };

    let mut bytes = Vec::new();
    if let Err(error) = file.by_ref().take(MAX_BYTES).read_to_end(&mut bytes) {
        return PreviewResult {
            lines: vec![Line::from(Span::styled(
                format!("Cannot read file: {error}"),
                theme::DEFAULT_STYLE,
            ))],
        };
    }

    if bytes.contains(&0) {
        return PreviewResult {
            lines: vec![Line::from(Span::styled(
                format!(
                    "Binary file ({} bytes{})",
                    metadata.len(),
                    if metadata.len() > MAX_BYTES {
                        ", preview truncated"
                    } else {
                        ""
                    }
                ),
                theme::DEFAULT_STYLE,
            ))],
        };
    }

    let content = String::from_utf8_lossy(&bytes);
    let raw_lines: Vec<&str> = content.lines().take(MAX_LINES).collect();

    let syntax = resolve_syntax(syntax_set, path);

    let mut parse_state = ParseState::new(syntax);
    let mut scope_stack = ScopeStack::new();
    let mut result: Vec<Line<'static>> = Vec::with_capacity(raw_lines.len());

    for (i, line) in raw_lines.iter().enumerate() {
        let mut spans: Vec<Span<'static>> = Vec::new();

        spans.push(Span::styled(
            format!("{:>4} │ ", i + 1),
            theme::LINE_NUMBER_STYLE,
        ));

        match parse_state.parse_line(line, syntax_set) {
            Ok(ops) => {
                let mut pos = 0;
                for (byte_pos, op) in &ops {
                    let byte_pos = *byte_pos;
                    if byte_pos > pos && pos < line.len() {
                        let end = byte_pos.min(line.len());
                        let text = &line[pos..end];
                        let style = theme::style_for_scopes(&scope_stack.scopes);
                        spans.push(Span::styled(text.to_string(), style));
                    }
                    let _ = scope_stack.apply(op);
                    pos = byte_pos;
                }
                if pos < line.len() {
                    let text = &line[pos..];
                    let style = theme::style_for_scopes(&scope_stack.scopes);
                    spans.push(Span::styled(text.to_string(), style));
                }
            }
            Err(_) => {
                spans.push(Span::styled((*line).to_string(), theme::DEFAULT_STYLE));
            }
        }

        result.push(Line::from(spans));
    }

    if result.is_empty() {
        result.push(Line::from(Span::styled("Empty file", theme::DEFAULT_STYLE)));
    } else if metadata.len() > MAX_BYTES || result.len() == MAX_LINES {
        result.push(Line::from(Span::styled(
            "… preview truncated …",
            theme::TRUNCATED_STYLE,
        )));
    }

    PreviewResult { lines: result }
}

/// A match whose characters form a single contiguous span reads as a real
/// literal substring hit; one broken into several ranges was pieced together
/// by the fuzzy matcher from scattered characters. The two are highlighted
/// differently so a glance at the preview shows which matches are "real".
fn is_exact_match(target: &PreviewMatch) -> bool {
    target.match_byte_offsets.len() == 1
}

/// Highlights every match in `matches` within the preview. `current_line`,
/// when set, marks the match the user has focused (e.g. via up/down through
/// an expanded file's matches) so it can be called out from the rest.
pub fn apply_match_highlight(
    lines: &mut [Line<'static>],
    matches: &[PreviewMatch],
    current_line: Option<u64>,
) {
    for target in matches {
        let is_current = current_line == Some(target.line_number);
        apply_single_match_highlight(lines, target, is_current);
    }
}

fn apply_single_match_highlight(
    lines: &mut [Line<'static>],
    target: &PreviewMatch,
    is_current: bool,
) {
    let target_idx = (target.line_number as usize).saturating_sub(1);
    if target_idx >= lines.len() {
        return;
    }

    let exact = is_exact_match(target);
    let line = &mut lines[target_idx];

    if let Some(span) = line.spans.first_mut() {
        span.style = span.style.bg(theme::MATCH_LINE_BG);
    }

    let mut pos = 0;
    for span in line.spans.iter_mut().skip(1) {
        let span_end = pos + span.content.len();

        let mut style = span.style.bg(theme::MATCH_LINE_BG);
        if span_overlaps_match(pos, span_end, target) {
            style = if exact {
                theme::exact_match_span_style(style)
            } else {
                theme::match_span_style(style)
            };
            if is_current {
                style = style.add_modifier(ratatui::style::Modifier::UNDERLINED);
            }
        }
        span.style = style;

        pos = span_end;
    }
}

fn span_overlaps_match(span_start: usize, span_end: usize, target: &PreviewMatch) -> bool {
    target
        .match_byte_offsets
        .iter()
        .any(|(match_start, match_end)| *match_start < span_end && *match_end > span_start)
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::{build_syntax_set, load_base};

    #[test]
    fn text_preview_has_line_numbers() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "fff-cli-preview-{}-{nonce}.txt",
            std::process::id()
        ));
        fs::write(&path, "alpha\nbeta\n").unwrap();
        let syntax_set = two_face::syntax::extra_no_newlines();
        let preview = load_base(&syntax_set, &path);
        let _ = fs::remove_file(path);

        assert_eq!(preview.lines.len(), 2);
        assert!(preview.lines[0]
            .spans
            .iter()
            .any(|s| s.content.contains("1 │")));
        assert!(preview.lines[1]
            .spans
            .iter()
            .any(|s| s.content.contains("2 │")));
    }

    #[test]
    fn kdl_syntax_is_registered() {
        let syntax_set = build_syntax_set();
        let syntax = syntax_set
            .find_syntax_by_extension("kdl")
            .expect("KDL syntax should be available");
        assert_eq!(syntax.name, "KDL");
    }

    #[test]
    fn kdl_file_is_tokenized_not_treated_as_plain_text() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "fff-cli-preview-{}-{nonce}.kdl",
            std::process::id()
        ));
        fs::write(&path, "node_name \"arg\" prop=1 {\n  child 1 2 3\n}\n").unwrap();
        let syntax_set = build_syntax_set();
        let preview = load_base(&syntax_set, &path);
        let _ = fs::remove_file(path);

        assert_eq!(preview.lines.len(), 3);
        // A plain-text fallback would render the whole line as one span
        // (after the line-number span); real KDL tokenization splits the
        // node name, string argument, and property into separate spans.
        assert!(
            preview.lines[0].spans.len() > 3,
            "expected the KDL line to be split into multiple styled spans, got {:?}",
            preview.lines[0]
                .spans
                .iter()
                .map(|s| s.content.as_ref())
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn kdl_handles_comments_raw_strings_and_slashdash_without_panicking() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "fff-cli-preview-{}-{nonce}-rich.kdl",
            std::process::id()
        ));
        fs::write(
            &path,
            concat!(
                "// top level comment\n",
                "title \"My App\" version=1.5\n",
                "/* block\n",
                "   comment */\n",
                "plugins {\n",
                "    plugin (path)\"./foo.so\" enabled=true\n",
                "    plugin2 r#\"C:\\raw\\path\"# retries=3 ratio=0x1F\n",
                "    /-disabled_node \"ignored\"\n",
                "    keybinds {\n",
                "        bind \"ctrl+c\" { quit; }\n",
                "        bind2 val=#null flag=#false\n",
                "    }\n",
                "}\n",
            ),
        )
        .unwrap();
        let syntax_set = build_syntax_set();
        let preview = load_base(&syntax_set, &path);
        let _ = fs::remove_file(path);

        assert_eq!(preview.lines.len(), 13);
        let total_spans: usize = preview.lines.iter().map(|line| line.spans.len()).sum();
        assert!(
            total_spans > preview.lines.len() * 2,
            "expected varied tokenization across the file, got {total_spans} spans across {} lines",
            preview.lines.len()
        );
    }

    #[test]
    fn just_syntax_is_registered() {
        let syntax_set = build_syntax_set();
        let syntax = syntax_set
            .find_syntax_by_extension("justfile")
            .expect("Just syntax should be available");
        assert_eq!(syntax.name, "Just");
    }

    #[test]
    fn justfile_resolves_by_filename_without_extension() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir =
            std::env::temp_dir().join(format!("fff-cli-preview-{}-{nonce}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("justfile");
        fs::write(
            &path,
            concat!(
                "# say hello\n",
                "default: build\n",
                "\n",
                "build target=\"release\":\n",
                "    echo \"building {{target}}\"\n",
            ),
        )
        .unwrap();
        let syntax_set = build_syntax_set();
        let preview = load_base(&syntax_set, &path);
        let _ = fs::remove_dir_all(&dir);

        assert_eq!(preview.lines.len(), 5);
        let total_spans: usize = preview.lines.iter().map(|line| line.spans.len()).sum();
        assert!(
            total_spans > preview.lines.len() * 2,
            "expected the justfile to be tokenized, not treated as plain text, got {total_spans} spans across {} lines",
            preview.lines.len()
        );
    }

    #[test]
    fn sbatch_extension_is_highlighted_as_bash() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "fff-cli-preview-{}-{nonce}.sbatch",
            std::process::id()
        ));
        fs::write(
            &path,
            "#!/bin/bash\n#SBATCH --job-name=test\necho \"hello\"\n",
        )
        .unwrap();
        let syntax_set = build_syntax_set();
        let preview = load_base(&syntax_set, &path);
        let _ = fs::remove_file(path);

        assert_eq!(preview.lines.len(), 3);
        // A plain-text fallback would render each line as a single span
        // after the line-number span; bash tokenization splits it further
        // (e.g. the shebang, the string in the echo command).
        assert!(
            preview.lines[2].spans.len() > 2,
            "expected the echo line to be tokenized as bash, got {:?}",
            preview.lines[2]
                .spans
                .iter()
                .map(|s| s.content.as_ref())
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn mojo_syntax_is_registered() {
        let syntax_set = build_syntax_set();
        let syntax = syntax_set
            .find_syntax_by_extension("mojo")
            .expect("Mojo syntax should be available");
        assert_eq!(syntax.name, "Mojo");
    }

    #[test]
    fn mojo_file_is_tokenized_not_treated_as_plain_text() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "fff-cli-preview-{}-{nonce}.mojo",
            std::process::id()
        ));
        fs::write(
            &path,
            concat!(
                "# greet someone\n",
                "fn greet(name: String) raises -> String:\n",
                "    var count: Int = 0\n",
                "    return f\"hello {name}, count={count}\"\n",
            ),
        )
        .unwrap();
        let syntax_set = build_syntax_set();
        let preview = load_base(&syntax_set, &path);
        let _ = fs::remove_file(path);

        assert_eq!(preview.lines.len(), 4);
        let total_spans: usize = preview.lines.iter().map(|line| line.spans.len()).sum();
        assert!(
            total_spans > preview.lines.len() * 2,
            "expected the Mojo file to be tokenized, not treated as plain text, got {total_spans} spans across {} lines",
            preview.lines.len()
        );
    }

    #[test]
    fn extended_syntaxes_available() {
        let syntax_set = two_face::syntax::extra_no_newlines();
        
        // Debug: print all syntaxes to see what's available
        eprintln!("\nTotal syntaxes: {}", syntax_set.syntaxes().len());
        eprintln!("\nSearching for Typst-related syntaxes:");
        for syntax in syntax_set.syntaxes() {
            let name_lower = syntax.name.to_lowercase();
            if name_lower.contains("typ") || name_lower.contains("typst") {
                eprintln!("  Found: {} (extensions: {:?})", syntax.name, syntax.file_extensions);
            }
        }
        
        // Verify new syntaxes from two-face are available
        assert!(syntax_set.find_syntax_by_extension("typ").is_some(), "Typst syntax should be available");
        assert!(syntax_set.find_syntax_by_extension("nix").is_some(), "Nix syntax should be available");
        assert!(syntax_set.find_syntax_by_extension("toml").is_some(), "TOML syntax should be available");
        assert!(syntax_set.find_syntax_by_extension("ts").is_some(), "TypeScript syntax should be available");
        
        // Verify we have significantly more syntaxes than default syntect
        assert!(syntax_set.syntaxes().len() > 100, "Should have 100+ syntax definitions");
    }
}
