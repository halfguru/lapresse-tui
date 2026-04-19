<div align="center">

# lapresse-tui

A terminal-based La Presse archive reader built in Rust with ratatui.

Browse 20 years of Quebec French newspaper articles (2005–2026) through a calendar-driven interface, reading full articles with inline images rendered in the terminal.

[![Rust](https://img.shields.io/badge/rust-2024-orange?logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![CI](https://github.com/halfguru/lapresse-tui/actions/workflows/ci.yml/badge.svg)](https://github.com/halfguru/lapresse-tui/actions/workflows/ci.yml)

![demo](demo.gif)

</div>

---

## Features

- **Calendar-driven navigation** — browse by month/day, see article counts at a glance
- **Inline image rendering** — articles display photos inline using Sixel, Kitty, iTerm2, or half-block fallback
- **Full-text search** — search across all cached articles with SQLite FTS5
- **Offline-capable** — local SQLite cache means browsing works without network after initial sync
- **Auto-sync on date select** — navigating to a date automatically fetches articles from lapresse.ca
- **Lazy image loading** — images are only downloaded when you open an article, not during sync
- **Concurrent scraping** — 4 article workers + 8 image workers with exponential backoff + jitter for rate-limit resilience
- **Live sync progress** — animated spinner, phase tracking, and article counts in real-time during CLI sync

## Usage

```bash
# Launch the TUI
cargo run

# Bulk sync from CLI (full date range, with images)
cargo run -- sync --from 2025-01-01 --to 2025-01-31

# Metadata-only sync (faster; images load on-demand in TUI)
cargo run -- sync --from 2005-01-01 --to 2026-12-31 --metadata-only
```

### TUI Keybindings

| Key | Action |
|-----|--------|
| `h/l` | Previous/next month |
| `H/L` | Previous/next year |
| `j/k` | Move selection |
| `Enter` | Select date / open article |
| `c` | Switch to calendar |
| `f` / `F` | Filter / clear section filter |
| `/` | Search articles |
| `?` | Help |
| `q` | Quit |

## Architecture

```
src/
  main.rs                     — CLI entry point (TUI or bulk sync)
  app/
    mod.rs                    — App state, event loop, sync orchestration
    handlers.rs               — Key handlers for each view
    image_loader.rs           — Background image fetch + decode
  ui/
    mod.rs                    — Render dispatcher, shared helpers
    calendar.rs               — Calendar view
    article_list.rs           — Article list view
    article_reader.rs         — Article reader with virtual scrolling
    search.rs                 — Search view
    help.rs                   — Help overlay
  sync/
    mod.rs                    — Sync orchestration
    scraping.rs               — HTML parsing (day pages, article pages)
    download.rs               — HTTP fetch with retry
    progress.rs               — SyncStats
  db/
    mod.rs                    — SQLite persistence (articles, images, FTS5)
    types.rs                  — Data structs
migrations/
  V1__initial_schema.sql
```

## Tech Stack

| Library | Purpose |
|---------|---------|
| [ratatui](https://crates.io/crates/ratatui) 0.30 | TUI framework |
| [ratatui-image](https://crates.io/crates/ratatui-image) 10.0 | Inline image rendering |
| [crossterm](https://crates.io/crates/crossterm) 0.29 | Terminal backend |
| [rusqlite](https://crates.io/crates/rusqlite) 0.39 | SQLite cache |
| [reqwest](https://crates.io/crates/reqwest) 0.13 | HTTP client |
| [scraper](https://crates.io/crates/scraper) 0.26 | HTML parsing |
| [tokio](https://crates.io/crates/tokio) 1.x | Async runtime |
| [clap](https://crates.io/crates/clap) 4.x | CLI arguments |

## License

MIT
