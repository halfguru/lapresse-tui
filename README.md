# lapresse-tui

A terminal-based La Presse archive reader built in Rust with ratatui. Browse 20 years of Quebec French newspaper articles (2005–2026) through a calendar-driven interface, reading full articles with inline images rendered in the terminal.

![demo](demo.gif)

## Features

- **Calendar-driven navigation** — browse by month/day, see article counts at a glance
- **Inline image rendering** — articles display photos inline using Sixel, Kitty, iTerm2, or half-block fallback
- **Full-text search** — search across all cached articles with SQLite FTS5
- **Offline-capable** — local SQLite cache means browsing works without network after initial sync
- **Concurrent syncing** — 8 parallel workers for both article scraping and image downloads
- **Live sync progress** — see phase (fetching/scraping/images) and article counts in real-time
- **Auto-sync** — first launch automatically syncs today's articles

## Usage

```bash
# Launch the TUI
cargo run

# Bulk sync from CLI (e.g. a date range)
cargo run -- sync --from 2025-01-01 --to 2025-01-31
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
  app.rs     — Application state, event handling, sync orchestration
  ui.rs      — ratatui rendering (calendar, article list, reader)
  sync.rs    — Scraping engine with concurrent article/image fetching
  db.rs      — SQLite persistence (articles, images, sync state)
```

Sync flow: `sync_day` fetches the archive index page, scrapes articles concurrently (8 workers), then downloads images concurrently (8 workers). Progress is reported via an mpsc channel back to the TUI.

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
