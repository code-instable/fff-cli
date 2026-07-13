use std::path::PathBuf;
use std::time::{Duration, Instant};

use anyhow::Result;
use ratatui::text::Line;
use ratatui::widgets::ListState;
use syntect::parsing::SyntaxSet;
use two_face::syntax::extra_newlines;

use crate::engine::{ContentMatch, FileGroup, SearchEngine, SearchItem};
use crate::preview::{self, PreviewMatch};

#[derive(Debug)]
pub enum AppOutcome {
    Selected(SearchItem),
    SelectedContent(ContentMatch),
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchMode {
    File,
    Content,
}

pub struct App {
    pub query: String,
    pub items: Vec<SearchItem>,
    pub total_matched: usize,
    pub total_files: usize,
    pub list_state: ListState,
    pub preview_enabled: bool,
    pub preview_lines: Vec<Line<'static>>,
    pub preview_scroll: u16,
    pub preview_area_height: u16,
    syntax_set: SyntaxSet,
    limit: usize,
    pub mode: SearchMode,
    file_groups: Vec<FileGroup>,
    expanded_file_index: Option<usize>,
    cached_preview_path: Option<PathBuf>,
    cached_base_lines: Vec<Line<'static>>,
    preview_dirty: bool,
    preview_dirty_since: Option<Instant>,
}

impl App {
    pub fn new(query: String, limit: usize, preview_enabled: bool, mode: SearchMode) -> Self {
        Self {
            query,
            items: Vec::new(),
            total_matched: 0,
            total_files: 0,
            list_state: ListState::default(),
            preview_enabled,
            preview_lines: Vec::new(),
            preview_scroll: 0,
            preview_area_height: 20,
            syntax_set: extra_newlines(),
            limit: limit.max(1),
            mode,
            file_groups: Vec::new(),
            expanded_file_index: None,
            cached_preview_path: None,
            cached_base_lines: Vec::new(),
            preview_dirty: false,
            preview_dirty_since: None,
        }
    }

    pub fn refresh(&mut self, engine: &SearchEngine) -> Result<()> {
        match self.mode {
            SearchMode::File => self.refresh_file_mode(engine),
            SearchMode::Content => self.refresh_content_mode(engine),
        }
    }

    fn refresh_file_mode(&mut self, engine: &SearchEngine) -> Result<()> {
        let snapshot = engine.search(&self.query, self.limit)?;
        self.items = snapshot.items;
        self.total_matched = snapshot.total_matched;
        self.total_files = snapshot.total_files;
        self.list_state = ListState::default();
        self.list_state
            .select((!self.items.is_empty()).then_some(0));
        self.update_preview();
        Ok(())
    }

    fn refresh_content_mode(&mut self, engine: &SearchEngine) -> Result<()> {
        let snapshot = engine.grep_search(&self.query, self.limit)?;
        self.file_groups = snapshot.file_groups;
        self.total_matched = snapshot.total_matches;
        self.total_files = snapshot.total_files_searched;
        self.expanded_file_index = None;
        self.build_content_items();
        self.list_state = ListState::default();
        self.list_state
            .select((!self.items.is_empty()).then_some(0));
        self.update_preview();
        Ok(())
    }

    fn build_content_items(&mut self) {
        self.items = match self.expanded_file_index {
            Some(file_idx) => {
                let group = &self.file_groups[file_idx];
                group
                    .matches
                    .iter()
                    .map(|m| {
                        let display = format!(
                            "  {}:{}  {}",
                            m.line_number,
                            m.col_1based(),
                            m.line_content.trim_start()
                        );
                        SearchItem {
                            relative: display,
                            absolute: m.absolute.clone(),
                        }
                    })
                    .collect()
            }
            None => self
                .file_groups
                .iter()
                .map(|group| {
                    let count = group.matches.len();
                    SearchItem {
                        relative: format!(
                            "{}  ({} match{})",
                            group.relative,
                            count,
                            if count == 1 { "" } else { "es" }
                        ),
                        absolute: group.absolute.clone(),
                    }
                })
                .collect(),
        };
    }

    pub fn selected_file_group(&self) -> Option<&FileGroup> {
        self.list_state
            .selected()
            .and_then(|index| self.file_groups.get(index))
    }

    pub fn selected_content_match(&self) -> Option<&ContentMatch> {
        if let Some(file_idx) = self.expanded_file_index {
            self.list_state
                .selected()
                .and_then(|index| self.file_groups[file_idx].matches.get(index))
        } else {
            None
        }
    }

    pub fn selected_item(&self) -> Option<&SearchItem> {
        self.list_state
            .selected()
            .and_then(|index| self.items.get(index))
    }

    pub fn select_current(&self) -> Option<AppOutcome> {
        match self.mode {
            SearchMode::File => self.selected_item().cloned().map(AppOutcome::Selected),
            SearchMode::Content => match self.expanded_file_index {
                None => {
                    let group = self.selected_file_group()?;
                    if group.matches.len() == 1 {
                        Some(AppOutcome::SelectedContent(group.matches[0].clone()))
                    } else {
                        None
                    }
                }
                Some(_) => self
                    .selected_content_match()
                    .cloned()
                    .map(AppOutcome::SelectedContent),
            },
        }
    }

    pub fn expand_file_group(&mut self) {
        if self.mode == SearchMode::Content && self.expanded_file_index.is_none() {
            if let Some(group) = self.selected_file_group() {
                if group.matches.len() > 1 {
                    let index = self.list_state.selected().unwrap_or(0);
                    self.expanded_file_index = Some(index);
                    self.build_content_items();
                    self.list_state = ListState::default();
                    self.list_state.select(Some(0));
                    self.update_preview();
                }
            }
        }
    }

    pub fn collapse_content_view(&mut self) {
        if self.expanded_file_index.is_some() {
            self.expanded_file_index = None;
            self.build_content_items();
            self.list_state = ListState::default();
            self.list_state.select(Some(0));
            self.update_preview();
        }
    }

    pub fn is_expanded(&self) -> bool {
        self.expanded_file_index.is_some()
    }

    pub fn toggle_mode(&mut self) {
        self.mode = match self.mode {
            SearchMode::File => SearchMode::Content,
            SearchMode::Content => SearchMode::File,
        };
        self.expanded_file_index = None;
        self.file_groups.clear();
    }

    pub fn move_selection(&mut self, delta: isize) {
        if self.items.is_empty() {
            self.list_state.select(None);
            return;
        }

        let current = self.list_state.selected().unwrap_or(0) as isize;
        let last = self.items.len() as isize - 1;
        let next = (current + delta).clamp(0, last) as usize;
        self.list_state.select(Some(next));
        self.update_preview();
    }

    pub fn select_first(&mut self) {
        if !self.items.is_empty() {
            self.list_state.select(Some(0));
            self.update_preview();
        }
    }

    pub fn select_last(&mut self) {
        if let Some(last) = self.items.len().checked_sub(1) {
            self.list_state.select(Some(last));
            self.update_preview();
        }
    }

    pub fn select_item_at(&mut self, index: usize) {
        if index < self.items.len() {
            self.list_state.select(Some(index));
            self.update_preview();
        }
    }

    pub fn scroll_preview(&mut self, delta: i16) {
        if !self.preview_enabled || self.preview_lines.is_empty() {
            return;
        }
        let visible_height = self.preview_area_height.saturating_sub(2) as usize;
        let max_scroll = self.preview_lines.len().saturating_sub(visible_height);
        let new_scroll = (self.preview_scroll as i32 + delta as i32).clamp(0, max_scroll as i32);
        self.preview_scroll = new_scroll as u16;
    }

    pub fn append_query(&mut self, text: &str, engine: &SearchEngine) -> Result<()> {
        self.query.push_str(&normalise_input(text));
        self.refresh(engine)
    }

    pub fn backspace(&mut self, engine: &SearchEngine) -> Result<()> {
        self.query.pop();
        self.refresh(engine)
    }

    pub fn delete_word(&mut self, engine: &SearchEngine) -> Result<()> {
        while self.query.chars().last().is_some_and(char::is_whitespace) {
            self.query.pop();
        }
        while self
            .query
            .chars()
            .last()
            .is_some_and(|character| !character.is_whitespace())
        {
            self.query.pop();
        }
        self.refresh(engine)
    }

    pub fn clear_query(&mut self, engine: &SearchEngine) -> Result<()> {
        self.query.clear();
        self.refresh(engine)
    }

    pub fn toggle_preview(&mut self) {
        self.preview_enabled = !self.preview_enabled;
        if self.preview_enabled {
            self.load_preview_immediate();
        } else {
            self.preview_lines = Vec::new();
            self.preview_scroll = 0;
        }
    }

    fn current_preview_path(&self) -> Option<PathBuf> {
        match self.mode {
            SearchMode::File => self.selected_item().map(|item| item.absolute.clone()),
            SearchMode::Content => match self.expanded_file_index {
                Some(file_idx) => self.file_groups.get(file_idx).map(|g| g.absolute.clone()),
                None => self.selected_file_group().map(|g| g.absolute.clone()),
            },
        }
    }

    fn current_match_info(&self) -> Option<PreviewMatch> {
        match self.mode {
            SearchMode::Content => match self.expanded_file_index {
                Some(file_idx) => self
                    .list_state
                    .selected()
                    .and_then(|idx| self.file_groups[file_idx].matches.get(idx))
                    .map(|m| PreviewMatch {
                        line_number: m.line_number,
                        match_byte_offsets: m.match_byte_offsets.clone(),
                    }),
                None => None,
            },
            _ => None,
        }
    }

    fn update_preview(&mut self) {
        if !self.preview_enabled {
            self.preview_lines = Vec::new();
            self.preview_scroll = 0;
            self.cached_preview_path = None;
            self.cached_base_lines = Vec::new();
            return;
        }

        let path = self.current_preview_path();

        let Some(path) = path else {
            self.preview_lines = Vec::new();
            self.preview_scroll = 0;
            return;
        };

        let path_changed = self.cached_preview_path.as_ref() != Some(&path);

        if path_changed {
            self.preview_dirty = true;
            self.preview_dirty_since = Some(Instant::now());
        } else {
            self.apply_preview_highlight();
        }
    }

    fn apply_preview_highlight(&mut self) {
        let match_info = self.current_match_info();

        let mut lines = self.cached_base_lines.clone();
        if let Some(match_info) = match_info {
            preview::apply_match_highlight(&mut lines, &match_info);

            let visible_height = self.preview_area_height.saturating_sub(2) as u64;
            let center_offset = visible_height / 2;
            let target_scroll = match_info.line_number.saturating_sub(center_offset);
            let max_scroll = lines.len().saturating_sub(visible_height as usize);
            self.preview_scroll = target_scroll.min(max_scroll as u64) as u16;
        } else {
            self.preview_scroll = 0;
        }

        self.preview_lines = lines;
    }

    fn load_preview_immediate(&mut self) {
        let Some(path) = self.current_preview_path() else {
            self.preview_lines = Vec::new();
            self.preview_scroll = 0;
            return;
        };

        let result = preview::load_base(&self.syntax_set, &path);
        self.cached_base_lines = result.lines;
        self.cached_preview_path = Some(path);

        self.apply_preview_highlight();
    }

    pub fn tick_preview(&mut self) {
        if !self.preview_dirty || !self.preview_enabled {
            return;
        }

        let Some(since) = self.preview_dirty_since else {
            return;
        };

        if since.elapsed() < Duration::from_millis(80) {
            return;
        }

        self.preview_dirty = false;
        self.preview_dirty_since = None;

        let Some(path) = self.current_preview_path() else {
            return;
        };

        let result = preview::load_base(&self.syntax_set, &path);
        self.cached_base_lines = result.lines;
        self.cached_preview_path = Some(path);

        self.apply_preview_highlight();
    }
}

fn normalise_input(input: &str) -> String {
    input
        .chars()
        .map(|character| match character {
            '\n' | '\r' | '\t' => ' ',
            other => other,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::normalise_input;

    #[test]
    fn pasted_lines_become_one_query() {
        assert_eq!(normalise_input("src\n*.rs\ttest"), "src *.rs test");
    }
}
