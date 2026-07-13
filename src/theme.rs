use ratatui::style::{Color, Modifier, Style};
use syntect::parsing::Scope;

const COMMENT_DARK: Color = Color::Rgb(0x7B, 0x82, 0xA0);
const COMMENT: Color = Color::Rgb(0x8F, 0x97, 0xB6);
const MAGENTA: Color = Color::Rgb(0xEE, 0x82, 0xEE);
const RED: Color = Color::Rgb(0xFF, 0x7E, 0xA2);
const RED_MUTED: Color = Color::Rgb(0xF4, 0x97, 0xA6);
const ORANGE: Color = Color::Rgb(0xEA, 0x87, 0x43);
const YELLOW: Color = Color::Rgb(0xFF, 0xEA, 0x9B);
const GREEN: Color = Color::Rgb(0x9F, 0xEB, 0x99);
const TEAL: Color = Color::Rgb(0x59, 0xD2, 0xBE);
const SKY_BLUE: Color = Color::Rgb(0x7D, 0xE4, 0xF7);
const CYAN: Color = Color::Rgb(0x65, 0xCD, 0xFB);
const BLUE: Color = Color::Rgb(0x7B, 0xB1, 0xFF);
const INDIGO: Color = Color::Rgb(0xAB, 0xB7, 0xFF);
const FUCHSIA: Color = Color::Rgb(0xFF, 0xC1, 0xFC);
const PINK_LIGHT: Color = Color::Rgb(0xF8, 0xDE, 0xD9);
const PINK_HOT: Color = Color::Rgb(0xF7, 0xC8, 0xC8);
const FG: Color = Color::Rgb(0xCD, 0xD6, 0xF4);
pub const BG: Color = Color::Rgb(0x15, 0x15, 0x20);

pub const LINE_NUMBER_STYLE: Style = Style::new().fg(COMMENT);
pub const DEFAULT_STYLE: Style = Style::new().fg(FG);
pub const TRUNCATED_STYLE: Style = Style::new().fg(COMMENT).add_modifier(Modifier::ITALIC);

pub const MATCH_LINE_BG: Color = Color::Rgb(0x20, 0x22, 0x30);
pub const MATCH_HIGHLIGHT_BG: Color = Color::Rgb(0x55, 0x50, 0x20);
pub const MATCH_HIGHLIGHT_FG: Color = Color::Rgb(0xFF, 0xEA, 0x9B);

pub fn match_span_style(base: Style) -> Style {
    base.bg(MATCH_HIGHLIGHT_BG).fg(MATCH_HIGHLIGHT_FG)
}

pub const BUTTON_ACTIVE_BG: Color = Color::Rgb(0x40, 0x45, 0x60);
pub const BUTTON_INACTIVE_BG: Color = Color::Rgb(0x25, 0x28, 0x35);
pub const BUTTON_ACTIVE_STYLE: Style = Style::new().bg(BUTTON_ACTIVE_BG).fg(FG);
pub const BUTTON_INACTIVE_STYLE: Style = Style::new().bg(BUTTON_INACTIVE_BG).fg(COMMENT);

struct ScopeEntry {
    prefix: &'static str,
    style: Style,
}

impl ScopeEntry {
    const fn new(prefix: &'static str, fg: Color) -> Self {
        Self {
            prefix,
            style: Style::new().fg(fg),
        }
    }

    const fn with_modifiers(
        prefix: &'static str,
        fg: Color,
        bold: bool,
        italic: bool,
        underline: bool,
    ) -> Self {
        let mut modifiers = Modifier::empty();
        if bold {
            modifiers = modifiers.union(Modifier::BOLD);
        }
        if italic {
            modifiers = modifiers.union(Modifier::ITALIC);
        }
        if underline {
            modifiers = modifiers.union(Modifier::UNDERLINED);
        }
        Self {
            prefix,
            style: Style::new().fg(fg).add_modifier(modifiers),
        }
    }
}

static SCOPE_MAP: &[ScopeEntry] = &[
    ScopeEntry::new("constant.character.escape", FUCHSIA),
    ScopeEntry::new("constant.character", TEAL),
    ScopeEntry::new("constant.numeric", ORANGE),
    ScopeEntry::new("constant.language", ORANGE),
    ScopeEntry::new("constant.builtin", ORANGE),
    ScopeEntry::new("constant", ORANGE),
    ScopeEntry::new("string.regexp", ORANGE),
    ScopeEntry::new("string.special", BLUE),
    ScopeEntry::new("string", GREEN),
    ScopeEntry::new("entity.name.function", BLUE),
    ScopeEntry::new("entity.name.type", YELLOW),
    ScopeEntry::new("entity.name.tag", MAGENTA),
    ScopeEntry::new("entity.name.section", CYAN),
    ScopeEntry::new("entity.name.label", CYAN),
    ScopeEntry::new("entity.other.attribute-name", BLUE),
    ScopeEntry::new("entity.other.inherited-class", YELLOW),
    ScopeEntry::new("entity.name", BLUE),
    ScopeEntry::new("keyword.operator", RED),
    ScopeEntry::new("keyword.other", MAGENTA),
    ScopeEntry::new("keyword", MAGENTA),
    ScopeEntry::new("storage.type", YELLOW),
    ScopeEntry::new("storage.modifier", TEAL),
    ScopeEntry::new("storage", MAGENTA),
    ScopeEntry::new("support.function", BLUE),
    ScopeEntry::new("support.type", YELLOW),
    ScopeEntry::new("support.class", YELLOW),
    ScopeEntry::new("support.constant", ORANGE),
    ScopeEntry::new("support", BLUE),
    ScopeEntry::new("variable.parameter", RED_MUTED),
    ScopeEntry::new("variable.other.member", TEAL),
    ScopeEntry::new("variable.language", RED),
    ScopeEntry::new("variable.function", BLUE),
    ScopeEntry::new("variable.builtin", RED),
    ScopeEntry::new("variable", FG),
    ScopeEntry::new("punctuation.special", SKY_BLUE),
    ScopeEntry::new("punctuation.delimiter", RED),
    ScopeEntry::new("punctuation", COMMENT),
    ScopeEntry::with_modifiers("markup.heading", INDIGO, true, false, false),
    ScopeEntry::with_modifiers("markup.bold", FG, true, false, false),
    ScopeEntry::with_modifiers("markup.italic", PINK_LIGHT, false, true, false),
    ScopeEntry::new("markup.list", MAGENTA),
    ScopeEntry::new("markup.raw", PINK_HOT),
    ScopeEntry::with_modifiers("markup.link.url", PINK_LIGHT, false, false, true),
    ScopeEntry::new("markup.link.text", BLUE),
    ScopeEntry::new("markup", FG),
    ScopeEntry::new("diff.plus", GREEN),
    ScopeEntry::new("diff.minus", RED),
    ScopeEntry::new("diff.delta", BLUE),
    ScopeEntry::with_modifiers("comment", COMMENT_DARK, false, true, false),
    ScopeEntry::new("invalid", RED),
    ScopeEntry::new("meta", FG),
];

pub fn style_for_scopes(scopes: &[Scope]) -> Style {
    for scope in scopes.iter().rev() {
        let s = scope.to_string();
        let mut best: Option<&ScopeEntry> = None;
        let mut best_len = 0;

        for entry in SCOPE_MAP {
            if s.starts_with(entry.prefix) && entry.prefix.len() > best_len {
                best = Some(entry);
                best_len = entry.prefix.len();
            }
        }

        if let Some(entry) = best {
            return entry.style;
        }
    }

    DEFAULT_STYLE
}
