# Phase 1: Foundation & Data Layer

**Status:** Complete
**Implemented:** 2026-04-13 (direct implementation, no formal plan)

## Context

Phase 1 was implemented directly during initial development. No formal discuss/plan cycle was run — the phase scope was clear enough from ROADMAP.md success criteria to proceed immediately.

## Implementation Summary

### Files Created
- `Cargo.toml` — 11 dependencies (ratatui 0.30, ratatui-image 10.0, crossterm 0.29, rusqlite 0.39, clap 4, anyhow, tracing, chrono, dirs, image)
- `migrations/V1__initial_schema.sql` — 3 tables, 5 indexes
- `src/main.rs` — CLI entry, terminal init, Picker protocol detection, event loop
- `src/app.rs` — App state struct with TEA-pattern key handling
- `src/ui.rs` — Placeholder view with protocol info, DB path, article count, status bar
- `src/db.rs` — SQLite open + schema bootstrap (WAL mode, foreign keys)

### Key Decisions
- `ratatui-image` uses `default-features = false` with only `["crossterm", "image-defaults"]` — avoids system dependency on `libchafa`
- Schema embedded via `include_str!` instead of refinery — simpler for a single migration
- `article_count` uses `u32` (rusqlite doesn't support `u64` from SQLite INTEGER)

## Success Criteria Verification

1. ✅ User can launch the app and see a terminal UI with an empty placeholder view
2. ✅ App exits cleanly on q/Esc without leaving the terminal in a broken state (panic hook via ratatui::init)
3. ✅ SQLite database is auto-created with schema for articles, images, and sync state on first launch
4. ✅ Image protocol is detected at startup (Sixel/Kitty/half-block/none) and reported in the UI

## Notes
- Zero clippy warnings
- CLI supports `--db` flag for custom database path, defaults to `~/.cache/lpresse/lpresse.db`
