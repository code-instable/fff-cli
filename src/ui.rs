use std::io;
use std::time::{Duration, Instant};

use anyhow::Result;
use crossterm::event::{
    self, DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture,
    Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph, Wrap};
use ratatui::{Frame, Terminal};

use crate::app::{App, AppOutcome, SearchMode};
use crate::engine::SearchEngine;
use crate::theme;

#[derive(Default)]
struct LayoutInfo {
    matches_area: Rect,
    matches_inner: Rect,
    preview_area: Option<Rect>,
    mode_button: Rect,
    preview_button: Rect,
}

pub fn run(app: &mut App, engine: &SearchEngine) -> Result<AppOutcome> {
    install_panic_hook();
    let mut session = TerminalSession::new()?;
    let mut layout = LayoutInfo::default();
    let mut last_click: Option<(Instant, u16, u16)> = None;

    loop {
        app.tick_preview();
        session
            .terminal
            .draw(|frame| draw(frame, app, engine, &mut layout))?;

        if !event::poll(Duration::from_millis(100))? {
            continue;
        }

        match event::read()? {
            Event::Key(key) if key.kind == KeyEventKind::Press => {
                if let Some(outcome) = handle_key(key, app, engine)? {
                    return Ok(outcome);
                }
            }
            Event::Mouse(mouse) => {
                if let Some(outcome) = handle_mouse(mouse, app, engine, &layout, &mut last_click)? {
                    return Ok(outcome);
                }
            }
            Event::Paste(text) => app.append_query(&text, engine)?,
            Event::Resize(_, _) => {}
            _ => {}
        }
    }
}

fn handle_key(key: KeyEvent, app: &mut App, engine: &SearchEngine) -> Result<Option<AppOutcome>> {
    let control = key.modifiers.contains(KeyModifiers::CONTROL);
    let alt = key.modifiers.contains(KeyModifiers::ALT);

    match (key.code, app.mode) {
        (KeyCode::Enter, SearchMode::Content) => {
            if let Some(outcome) = app.select_current() {
                return Ok(Some(outcome));
            }
            app.expand_file_group();
            Ok(None)
        }
        (KeyCode::Enter, _) => Ok(app.select_current()),

        (KeyCode::Esc, SearchMode::Content) => {
            if app.is_expanded() {
                app.collapse_content_view();
                Ok(None)
            } else {
                Ok(Some(AppOutcome::Cancelled))
            }
        }
        (KeyCode::Esc, _) => Ok(Some(AppOutcome::Cancelled)),

        (KeyCode::Char('c'), _) if control => Ok(Some(AppOutcome::Cancelled)),

        (KeyCode::Tab, _) => {
            app.toggle_mode();
            app.refresh(engine)?;
            Ok(None)
        }

        (KeyCode::Up, _) => {
            app.move_selection(-1);
            Ok(None)
        }
        (KeyCode::Down, _) => {
            app.move_selection(1);
            Ok(None)
        }
        (KeyCode::Char('p'), _) if control => {
            app.move_selection(-1);
            Ok(None)
        }
        (KeyCode::Char('n'), _) if control => {
            app.move_selection(1);
            Ok(None)
        }

        (KeyCode::PageUp, _) => {
            app.move_selection(-10);
            Ok(None)
        }
        (KeyCode::PageDown, _) => {
            app.move_selection(10);
            Ok(None)
        }
        (KeyCode::Home, _) => {
            app.select_first();
            Ok(None)
        }
        (KeyCode::End, _) => {
            app.select_last();
            Ok(None)
        }

        (KeyCode::Backspace, SearchMode::Content) => {
            if app.is_expanded() {
                app.collapse_content_view();
                Ok(None)
            } else {
                app.backspace(engine)?;
                Ok(None)
            }
        }
        (KeyCode::Backspace, _) => {
            app.backspace(engine)?;
            Ok(None)
        }

        (KeyCode::Char('w'), _) if control => {
            app.delete_word(engine)?;
            Ok(None)
        }
        (KeyCode::Char('u'), _) if control => {
            app.clear_query(engine)?;
            Ok(None)
        }
        (KeyCode::F(2), _) => {
            app.toggle_preview();
            Ok(None)
        }
        (KeyCode::Char(character), _) if !control && !alt => {
            app.append_query(&character.to_string(), engine)?;
            Ok(None)
        }
        _ => Ok(None),
    }
}

fn handle_mouse(
    mouse: MouseEvent,
    app: &mut App,
    engine: &SearchEngine,
    layout: &LayoutInfo,
    last_click: &mut Option<(Instant, u16, u16)>,
) -> Result<Option<AppOutcome>> {
    match mouse.kind {
        MouseEventKind::ScrollUp => {
            if layout
                .preview_area
                .is_some_and(|a| contains(a, mouse.column, mouse.row))
            {
                app.scroll_preview(-3);
            } else {
                app.move_selection(-3);
            }
            Ok(None)
        }
        MouseEventKind::ScrollDown => {
            if layout
                .preview_area
                .is_some_and(|a| contains(a, mouse.column, mouse.row))
            {
                app.scroll_preview(3);
            } else {
                app.move_selection(3);
            }
            Ok(None)
        }
        MouseEventKind::Down(MouseButton::Left) => {
            let now = Instant::now();
            let is_double = last_click.as_ref().is_some_and(|(when, col, row)| {
                now.duration_since(*when) < Duration::from_millis(400)
                    && *col == mouse.column
                    && *row == mouse.row
            });
            *last_click = Some((now, mouse.column, mouse.row));

            if contains(layout.mode_button, mouse.column, mouse.row) {
                app.toggle_mode();
                app.refresh(engine)?;
                return Ok(None);
            }

            if contains(layout.preview_button, mouse.column, mouse.row) {
                app.toggle_preview();
                return Ok(None);
            }

            if contains(layout.matches_inner, mouse.column, mouse.row) {
                let row_in_list = (mouse.row - layout.matches_inner.y) as usize;
                let scroll_offset = app.list_state.offset();
                let item_index = scroll_offset + row_in_list;

                if item_index < app.items.len() {
                    app.select_item_at(item_index);

                    if is_double {
                        return Ok(app.select_current());
                    }
                }
            }

            Ok(None)
        }
        _ => Ok(None),
    }
}

fn contains(area: Rect, col: u16, row: u16) -> bool {
    col >= area.x && col < area.x + area.width && row >= area.y && row < area.y + area.height
}

fn draw(frame: &mut Frame, app: &mut App, engine: &SearchEngine, layout: &mut LayoutInfo) {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(3),
            Constraint::Length(2),
        ])
        .split(frame.area());

    draw_query(frame, app, vertical[0]);
    draw_body(frame, app, vertical[1], layout);
    draw_footer(frame, app, engine, vertical[2], layout);
}

fn draw_query(frame: &mut Frame, app: &App, area: Rect) {
    let title = match app.mode {
        SearchMode::File => "FFF query",
        SearchMode::Content => "FFF grep",
    };
    let block = Block::default().borders(Borders::ALL).title(title);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    frame.render_widget(Paragraph::new(app.query.as_str()), inner);
}

fn draw_body(frame: &mut Frame, app: &mut App, area: Rect, layout: &mut LayoutInfo) {
    if app.preview_enabled && area.width >= 100 {
        let horizontal = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(45), Constraint::Percentage(55)])
            .split(area);
        draw_matches(frame, app, horizontal[0], layout);
        draw_preview(frame, app, horizontal[1], layout);
    } else {
        layout.preview_area = None;
        draw_matches(frame, app, area, layout);
    }
}

fn draw_matches(frame: &mut Frame, app: &mut App, area: Rect, layout: &mut LayoutInfo) {
    let entries = app
        .items
        .iter()
        .map(|item| ListItem::new(item.relative.clone()))
        .collect::<Vec<_>>();

    let title = match app.mode {
        SearchMode::File => format!(
            "Matches {}/{} ({} indexed)",
            app.items.len(),
            app.total_matched,
            app.total_files
        ),
        SearchMode::Content => {
            let suffix = if app.is_expanded() {
                "".to_string()
            } else {
                format!(" in {} files", app.items.len())
            };
            format!(
                "Content matches ({}{} of {} files searched)",
                app.total_matched, suffix, app.total_files
            )
        }
    };

    let block = Block::default().borders(Borders::ALL).title(title);
    layout.matches_area = area;
    layout.matches_inner = block.inner(area);

    let list = List::new(entries)
        .block(block)
        .highlight_symbol("› ")
        .highlight_style(Style::default().add_modifier(Modifier::BOLD | Modifier::REVERSED));

    frame.render_stateful_widget(list, area, &mut app.list_state);
}

fn draw_preview(frame: &mut Frame, app: &mut App, area: Rect, layout: &mut LayoutInfo) {
    app.preview_area_height = area.height;

    let title = app
        .selected_item()
        .map(|item| format!("Preview: {}", item.relative))
        .unwrap_or_else(|| "Preview".to_owned());

    layout.preview_area = Some(area);

    let preview = Paragraph::new(app.preview_lines.clone())
        .block(Block::default().borders(Borders::ALL).title(title))
        .style(Style::default().bg(theme::BG))
        .wrap(Wrap { trim: false })
        .scroll((app.preview_scroll, 0u16));

    frame.render_widget(preview, area);
}

fn draw_footer(
    frame: &mut Frame,
    app: &App,
    engine: &SearchEngine,
    area: Rect,
    layout: &mut LayoutInfo,
) {
    let horizontal = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(14),
            Constraint::Length(1),
            Constraint::Length(16),
            Constraint::Length(1),
            Constraint::Min(10),
        ])
        .split(area);

    let mode_text = match app.mode {
        SearchMode::File => " File ",
        SearchMode::Content => " Content ",
    };
    let mode_style = match app.mode {
        SearchMode::File => theme::BUTTON_ACTIVE_STYLE,
        SearchMode::Content => theme::BUTTON_ACTIVE_STYLE,
    };
    let mode_line = Line::from(vec![Span::styled(mode_text, mode_style)]);
    layout.mode_button = horizontal[0];
    frame.render_widget(Paragraph::new(mode_line), horizontal[0]);

    let preview_text = if app.preview_enabled {
        " Preview: On "
    } else {
        " Preview: Off "
    };
    let preview_style = if app.preview_enabled {
        theme::BUTTON_ACTIVE_STYLE
    } else {
        theme::BUTTON_INACTIVE_STYLE
    };
    let preview_line = Line::from(vec![Span::styled(preview_text, preview_style)]);
    layout.preview_button = horizontal[2];
    frame.render_widget(Paragraph::new(preview_line), horizontal[2]);

    let help = match (app.mode, app.is_expanded()) {
        (SearchMode::Content, true) => {
            "Enter select · Esc back · ↑/↓ move · Tab file mode · F2 preview"
        }
        (SearchMode::Content, false) => {
            "Enter expand/select · Esc cancel · ↑/↓ move · Tab file mode · F2 preview"
        }
        _ => "Enter select · Esc/C-c cancel · ↑/↓ or C-p/C-n move · Tab grep mode · F2 preview",
    };

    let footer = vec![
        Line::from(format!("root: {}", engine.root().display())),
        Line::from(help),
    ];
    frame.render_widget(Paragraph::new(footer), horizontal[4]);
}

struct TerminalSession {
    terminal: Terminal<CrosstermBackend<io::Stderr>>,
}

impl TerminalSession {
    fn new() -> io::Result<Self> {
        enable_raw_mode()?;
        let mut stderr = io::stderr();
        if let Err(error) = execute!(
            stderr,
            EnterAlternateScreen,
            EnableBracketedPaste,
            EnableMouseCapture
        ) {
            let _ = disable_raw_mode();
            return Err(error);
        }

        let backend = CrosstermBackend::new(stderr);
        let mut terminal = match Terminal::new(backend) {
            Ok(terminal) => terminal,
            Err(error) => {
                restore_terminal();
                return Err(error);
            }
        };
        if let Err(error) = terminal.hide_cursor().and_then(|_| terminal.clear()) {
            let _ = terminal.show_cursor();
            restore_terminal();
            return Err(error);
        }

        Ok(Self { terminal })
    }
}

impl Drop for TerminalSession {
    fn drop(&mut self) {
        let _ = self.terminal.show_cursor();
        restore_terminal();
    }
}

fn restore_terminal() {
    let mut stderr = io::stderr();
    let _ = execute!(
        stderr,
        DisableBracketedPaste,
        DisableMouseCapture,
        LeaveAlternateScreen
    );
    let _ = disable_raw_mode();
}

fn install_panic_hook() {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        restore_terminal();
        previous(panic_info);
    }));
}
