# lapresse-tui

A terminal-based La Presse archive reader built in Rust with ratatui. Browse 20 years of Quebec French newspaper articles (2005–2026) through a calendar-driven interface, reading full articles with inline images rendered in the terminal.

![demo](demo.gif)

## Features

- **Calendar-driven navigation** — browse by month/day, see article counts at a glance
- **Inline image rendering** — articles display photos inline using Sixel, Kitty, iTerm2, or half-block fallback
- **Full-text search** — search across all cached articles with SQLite FTS5
- **Offline-capable** — local SQLite cache means browsing works without network after initial sync
- **Lazy image loading** — `--metadata-only` sync stores article text + image metadata; images fetch on-demand in the TUI
- **Concurrent scraping** — 4 article workers + 8 image workers with exponential backoff + jitter for rate-limit resilience
- **Live sync progress** — animated spinner, phase tracking, and article counts in real-time during CLI sync
- **Auto-sync** — first launch automatically syncs today's articles

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
| `h/l` or `←/→` | Previous/next month |
| `j/k` or `↑/↓` | Move selection |
| `Enter` | Open article / select date |
| `s` | Sync selected day |
| `c` | Toggle calendar focus |
| `f` | Filter by section |
| `/` | Search articles |
| `?` | Help |
| `q` | Quit |

## Architecture

```
src/
  main.rs    — CLI entry point (TUI or bulk sync)
  app.rs     — Application state, event handling, lazy image loading
  ui.rs      — ratatui rendering (calendar, article list, reader, search)
  sync.rs    — Scraping engine with retries, spinner, and progress tracking
  db.rs      — SQLite persistence (articles, images, sync state, FTS5)
migrations/
  V1__initial_schema.sql
```

**Sync flow**: Days are processed sequentially. Each day scrapes articles concurrently (4 workers, 100ms delay), then downloads images concurrently (8 workers). Retries use exponential backoff (5s→10s→20s) with random jitter. A braille spinner animates during CLI sync.

**Image loading**: `--metadata-only` stores NULL image blobs. The TUI lazily fetches missing images in a background thread, showing ⏳ while loading.

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
| [rand](https://crates.io/crates/rand) 0.9 | Jitter for retry backoff |

## License

MIT
