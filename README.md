# fff-cli

A small terminal file picker using [`fff-search`](https://github.com/dmtrKovalenko/fff)
for indexing, query parsing, ranking, frecency, and query history.

This first implementation is intentionally a **file picker**, not a general
`stdin` fuzzy filter. The FFF index is built once and kept alive for the entire
interactive session.

*AI WARNING:* This is *entirely* vibe-coded (sorry, I don't have time, just want the tool)
Any contribution is of course welcomed

## Features

- live FFF file queries;
- FFF glob/exclusion/path query syntax;
- warm in-memory index with filesystem watching;
- persistent frecency and query history;
- keyboard navigation;
- plain-text preview pane;
- shell-safe output: TUI on `stderr`, selected path on `stdout`;
- relative, absolute, newline-delimited, or NUL-delimited output.

## Build

```sh
cargo build --release
```

With Nix:

```sh
nix develop -c cargo build --release
```

The executable is `target/release/fff`.

## Usage

```sh
# Current directory
fff

# Another root
fff ~/code/project

# Start with an FFF query
fff -q '*.rs !target/ picker'

# Emit a path usable outside the selected root
fff ~/code/project --absolute

# Shell composition
file=$(fff) && "$EDITOR" "$file"

# Safe handling of unusual path characters
fff --print0 | while IFS= read -r -d '' file; do
  printf '%s\n' "$file"
done
```

### Keys

| Key | Action |
|---|---|
| `Enter` | select current file |
| `Esc`, `Ctrl-c` | cancel |
| `Up`, `Ctrl-p` | previous result |
| `Down`, `Ctrl-n` | next result |
| `PageUp`, `PageDown` | move by ten results |
| `Home`, `End` | first/last result |
| `Backspace` | delete one query character |
| `Ctrl-w` | delete previous query word |
| `Ctrl-u` | clear query |
| `F2` | toggle preview |

## CLI

```text
Usage: fff [OPTIONS] [ROOT]

Arguments:
  [ROOT]  Directory indexed by FFF [default: .]

Options:
  -q, --query <QUERY>                  Initial FFF query
  -n, --limit <LIMIT>                  Maximum matches retained [default: 200]
      --absolute                       Print an absolute path
      --print0                         Terminate output with NUL
      --no-preview                     Disable preview
      --follow-symlinks                Follow symbolic links
      --scan-timeout <SCAN_TIMEOUT>    Initial scan timeout in seconds [default: 30]
  -h, --help                           Print help
  -V, --version                        Print version
```

## Architecture

```text
src/
├── main.rs      CLI, exit status, stdout contract
├── engine.rs    FFF index, search, frecency, query history
├── app.rs       UI state and query mutations
├── preview.rs   bounded text preview
└── ui.rs        crossterm event loop and ratatui rendering
```

The UI performs a synchronous FFF search after each query mutation. This is
adequate for a first version because FFF searches a warm in-memory index. A
future version should add generation-tagged worker requests so stale searches
can be discarded without blocking input.

## Current limitations

- files only; no directories or content search mode;
- single selection only;
- no generic `command | fff` backend;
- no configurable key bindings or preview command;
- preview is bounded plain text, without syntax highlighting;
- initial scan must complete before the UI opens.
