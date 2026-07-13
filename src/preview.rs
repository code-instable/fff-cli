use std::fs::File;
use std::io::Read;
use std::path::Path;

use ratatui::text::{Line, Span};
use syntect::parsing::{ParseState, ScopeStack, SyntaxSet};

use crate::theme;

const MAX_BYTES: u64 = 128 * 1024;
const MAX_LINES: usize = 250;

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

    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    let syntax = syntax_set
        .find_syntax_by_extension(ext)
        .unwrap_or_else(|| syntax_set.find_syntax_plain_text());

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

pub fn apply_match_highlight(lines: &mut [Line<'static>], target: &PreviewMatch) {
    let target_idx = (target.line_number as usize).saturating_sub(1);
    if target_idx >= lines.len() {
        return;
    }

    let line = &mut lines[target_idx];

    if let Some(span) = line.spans.first_mut() {
        span.style = span.style.bg(theme::MATCH_LINE_BG);
    }

    let mut pos = 0;
    for span in line.spans.iter_mut().skip(1) {
        let span_end = pos + span.content.len();

        let mut style = span.style.bg(theme::MATCH_LINE_BG);
        if span_overlaps_match(pos, span_end, Some(target)) {
            style = theme::match_span_style(style);
        }
        span.style = style;

        pos = span_end;
    }
}

fn span_overlaps_match(span_start: usize, span_end: usize, target: Option<&PreviewMatch>) -> bool {
    let Some(t) = target else {
        return false;
    };
    t.match_byte_offsets
        .iter()
        .any(|(match_start, match_end)| *match_start < span_end && *match_end > span_start)
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::load_base;

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
        let syntax_set = two_face::syntax::extra_newlines();
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
    fn extended_syntaxes_available() {
        let syntax_set = two_face::syntax::extra_newlines();
        
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
