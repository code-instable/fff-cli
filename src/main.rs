mod app;
mod engine;
mod preview;
mod theme;
mod ui;

use std::io::{self, Write};
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Duration;

use anyhow::{Context, Result};
use clap::Parser;

use crate::app::{App, AppOutcome, SearchMode};
use crate::engine::SearchEngine;

#[derive(Debug, Parser)]
#[command(
    name = "fff",
    version,
    about = "Interactive file search powered by fff-search"
)]
struct Cli {
    /// Directory indexed by FFF.
    #[arg(default_value = ".")]
    root: PathBuf,

    /// Initial FFF query.
    #[arg(short, long, default_value = "")]
    query: String,

    /// Maximum number of matches retained by the UI.
    #[arg(short = 'n', long, default_value_t = 200)]
    limit: usize,

    /// Print an absolute path instead of a path relative to ROOT.
    #[arg(long)]
    absolute: bool,

    /// Terminate selected output with NUL instead of a newline.
    #[arg(long)]
    print0: bool,

    /// Disable the text preview pane.
    #[arg(long)]
    no_preview: bool,

    /// Follow symbolic links while indexing.
    #[arg(long)]
    follow_symlinks: bool,

    /// Maximum number of seconds to wait for the initial scan.
    #[arg(long, default_value_t = 30)]
    scan_timeout: u64,

    /// Start in content (grep) search mode. Uses fuzzy matching against file contents.
    #[arg(short = 'g', long)]
    grep: bool,

    /// Show status messages on stderr (e.g., indexing progress).
    #[arg(short, long)]
    verbose: bool,
}

fn main() -> ExitCode {
    match run() {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::from(130),
        Err(error) => {
            eprintln!("fff: {error:#}");
            ExitCode::FAILURE
        }
    }
}

/// Returns `true` when a selection was emitted and `false` when the picker was
/// cancelled.
fn run() -> Result<bool> {
    let cli = Cli::parse();

    let mode = if cli.grep {
        SearchMode::Content
    } else {
        SearchMode::File
    };

    if cli.verbose {
        eprintln!("Indexing {} …", cli.root.display());
    }
    let (engine, warnings) = SearchEngine::new(
        &cli.root,
        cli.follow_symlinks,
        Duration::from_secs(cli.scan_timeout),
    )?;
    if cli.verbose {
        for warning in warnings {
            eprintln!("fff: warning: {warning}");
        }
    }

    let mut app = App::new(cli.query, cli.limit, !cli.no_preview, mode);
    app.refresh(&engine)?;

    let outcome = ui::run(&mut app, &engine).context("terminal UI failed")?;

    let mut stdout = io::stdout().lock();

    match outcome {
        AppOutcome::Selected(item) => {
            if cli.verbose {
                for warning in engine.record_selection(&app.query, &item) {
                    eprintln!("fff: warning: {warning}");
                }
            } else {
                let _ = engine.record_selection(&app.query, &item);
            }

            let path = if cli.absolute {
                item.absolute
            } else {
                item.relative.into()
            };

            write!(stdout, "{}", path.display())?;
            if cli.print0 {
                stdout.write_all(&[0])?;
            } else {
                stdout.write_all(b"\n")?;
            }
            stdout.flush()?;

            Ok(true)
        }
        AppOutcome::SelectedContent(content_match) => {
            let path = if cli.absolute {
                content_match.absolute.display().to_string()
            } else {
                content_match.relative.clone()
            };

            write!(
                stdout,
                "{}:{}:{}",
                path,
                content_match.line_number,
                content_match.col_1based()
            )?;
            if cli.print0 {
                stdout.write_all(&[0])?;
            } else {
                stdout.write_all(b"\n")?;
            }
            stdout.flush()?;

            Ok(true)
        }
        AppOutcome::Cancelled => Ok(false),
    }
}
