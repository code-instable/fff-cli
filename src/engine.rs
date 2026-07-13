use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{bail, Context, Result};
use fff_search::file_picker::FilePicker;
use fff_search::frecency::FrecencyTracker;
use fff_search::query_tracker::QueryTracker;
use fff_search::{
    FFFMode, FilePickerOptions, FuzzySearchOptions, GrepConfig, GrepMode, GrepSearchOptions,
    PaginationArgs, QueryParser, SharedFilePicker, SharedFrecency, SharedQueryTracker,
};

#[derive(Clone, Debug)]
pub struct SearchItem {
    pub relative: String,
    pub absolute: PathBuf,
}

#[derive(Debug)]
pub struct SearchSnapshot {
    pub items: Vec<SearchItem>,
    pub total_matched: usize,
    pub total_files: usize,
}

#[derive(Clone, Debug)]
pub struct ContentMatch {
    pub relative: String,
    pub absolute: PathBuf,
    pub line_number: u64,
    pub col: usize,
    pub line_content: String,
    pub match_byte_offsets: Vec<(usize, usize)>,
}

impl ContentMatch {
    pub fn col_1based(&self) -> usize {
        self.col + 1
    }
}

#[derive(Clone, Debug)]
pub struct FileGroup {
    pub relative: String,
    pub absolute: PathBuf,
    pub matches: Vec<ContentMatch>,
}

#[derive(Debug)]
pub struct ContentSearchSnapshot {
    pub file_groups: Vec<FileGroup>,
    pub total_files_searched: usize,
    pub total_matches: usize,
}

pub struct SearchEngine {
    root: PathBuf,
    picker: SharedFilePicker,
    frecency: SharedFrecency,
    query_tracker: SharedQueryTracker,
}

impl SearchEngine {
    pub fn new(
        root: &Path,
        follow_symlinks: bool,
        scan_timeout: Duration,
    ) -> Result<(Self, Vec<String>)> {
        if !root.exists() {
            bail!("search root does not exist: {}", root.display());
        }
        if !root.is_dir() {
            bail!("search root is not a directory: {}", root.display());
        }

        let root = root
            .canonicalize()
            .with_context(|| format!("cannot canonicalize {}", root.display()))?;

        let picker = SharedFilePicker::default();
        let frecency = SharedFrecency::default();
        let query_tracker = SharedQueryTracker::default();
        let mut warnings = Vec::new();

        initialise_persistent_state(&frecency, &query_tracker, &mut warnings);

        FilePicker::new_with_shared_state(
            picker.clone(),
            frecency.clone(),
            FilePickerOptions {
                base_path: root.to_string_lossy().into_owned(),
                mode: FFFMode::Ai,
                enable_mmap_cache: false,
                enable_content_indexing: false,
                watch: true,
                follow_symlinks,
                ..Default::default()
            },
        )
        .context("could not start the FFF indexer")?;

        if !picker.wait_for_scan(scan_timeout) {
            bail!(
                "initial scan did not finish within {} seconds",
                scan_timeout.as_secs()
            );
        }

        Ok((
            Self {
                root,
                picker,
                frecency,
                query_tracker,
            },
            warnings,
        ))
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn search(&self, query: &str, limit: usize) -> Result<SearchSnapshot> {
        let picker_guard = self
            .picker
            .read()
            .context("FFF file index lock is poisoned")?;
        let picker = picker_guard
            .as_ref()
            .context("FFF file index is not initialised")?;

        let query_guard = self
            .query_tracker
            .read()
            .context("FFF query tracker lock is poisoned")?;
        let parser = QueryParser::default();
        let parsed = parser.parse(query);
        let result = picker.fuzzy_search(
            &parsed,
            query_guard.as_ref(),
            FuzzySearchOptions {
                max_threads: 0,
                current_file: None,
                project_path: Some(&self.root),
                pagination: PaginationArgs { offset: 0, limit },
                ..Default::default()
            },
        );

        let items = result
            .items
            .iter()
            .map(|file| {
                let relative = file.relative_path(picker);
                SearchItem {
                    absolute: self.root.join(&relative),
                    relative,
                }
            })
            .collect();

        Ok(SearchSnapshot {
            items,
            total_matched: result.total_matched,
            total_files: result.total_files,
        })
    }

    pub fn grep_search(&self, query: &str, limit: usize) -> Result<ContentSearchSnapshot> {
        let picker_guard = self
            .picker
            .read()
            .context("FFF file index lock is poisoned")?;
        let picker = picker_guard
            .as_ref()
            .context("FFF file index is not initialised")?;

        let parser = QueryParser::new(GrepConfig);
        let parsed = parser.parse(query);
        let result = picker.grep(
            &parsed,
            &GrepSearchOptions {
                mode: GrepMode::Fuzzy,
                page_limit: limit,
                smart_case: true,
                ..Default::default()
            },
        );

        let mut file_groups: Vec<FileGroup> = Vec::with_capacity(result.files.len());
        let mut total_matches = 0usize;
        for (file_index, file) in result.files.iter().enumerate() {
            let relative = file.relative_path(picker);
            let absolute = self.root.join(&relative);
            let mut matches: Vec<ContentMatch> = Vec::new();

            for m in &result.matches {
                if m.file_index == file_index {
                    matches.push(ContentMatch {
                        relative: relative.clone(),
                        absolute: absolute.clone(),
                        line_number: m.line_number,
                        col: m.col,
                        line_content: m.line_content.clone(),
                        match_byte_offsets: m
                            .match_byte_offsets
                            .iter()
                            .map(|(s, e)| (*s as usize, *e as usize))
                            .collect(),
                    });
                }
            }

            matches.sort_by_key(|m| m.line_number);
            total_matches += matches.len();
            file_groups.push(FileGroup {
                relative,
                absolute,
                matches,
            });
        }

        file_groups.sort_by(|a, b| {
            b.matches
                .len()
                .cmp(&a.matches.len())
                .then(a.relative.cmp(&b.relative))
        });

        Ok(ContentSearchSnapshot {
            total_files_searched: result.total_files_searched,
            total_matches,
            file_groups,
        })
    }

    /// Frecency and query-history updates are deliberately best-effort: a
    /// selected path must still be emitted when the persistent store is
    /// unavailable.
    pub fn record_selection(&self, query: &str, item: &SearchItem) -> Vec<String> {
        let mut warnings = Vec::new();

        match self.frecency.read() {
            Ok(guard) => {
                if let Some(tracker) = guard.as_ref() {
                    if let Err(error) = tracker.track_access(&item.absolute) {
                        warnings.push(format!("could not update frecency: {error}"));
                    }
                }
            }
            Err(error) => warnings.push(format!("frecency lock is poisoned: {error}")),
        }

        match self.query_tracker.write() {
            Ok(mut guard) => {
                if let Some(tracker) = guard.as_mut() {
                    if let Err(error) =
                        tracker.track_query_completion(query, &self.root, &item.absolute)
                    {
                        warnings.push(format!("could not update query history: {error}"));
                    }
                }
            }
            Err(error) => warnings.push(format!("query-history lock is poisoned: {error}")),
        }

        warnings
    }
}

fn initialise_persistent_state(
    frecency: &SharedFrecency,
    query_tracker: &SharedQueryTracker,
    warnings: &mut Vec<String>,
) {
    let state_dir = dirs::data_local_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("fff-cli");

    if let Err(error) = fs::create_dir_all(&state_dir) {
        warnings.push(format!(
            "cannot create state directory {}: {error}",
            state_dir.display()
        ));
        return;
    }

    match FrecencyTracker::open(state_dir.join("frecency")) {
        Ok(tracker) => {
            if let Err(error) = frecency.init(tracker) {
                warnings.push(format!("cannot initialise frecency: {error}"));
            }
        }
        Err(error) => warnings.push(format!("cannot open frecency database: {error}")),
    }

    match QueryTracker::open(state_dir.join("queries")) {
        Ok(tracker) => {
            if let Err(error) = query_tracker.init(tracker) {
                warnings.push(format!("cannot initialise query history: {error}"));
            }
        }
        Err(error) => warnings.push(format!("cannot open query-history database: {error}")),
    }
}
