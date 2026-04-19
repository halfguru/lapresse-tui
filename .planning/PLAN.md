---
phase: 99-refactor
plan: 00
type: overview
wave: N/A
depends_on: []
files_modified:
  - src/app.rs → src/app/
  - src/db.rs → src/db/
  - src/sync.rs → src/sync/
  - src/ui.rs → src/ui/
  - src/main.rs
autonomous: true
requirements: [REFACTOR-01, REFACTOR-02, REFACTOR-03, REFACTOR-04, REFACTOR-05, REFACTOR-06, REFACTOR-07, REFACTOR-08, REFACTOR-09]

must_haves:
  truths:
    - "Zero #[allow(dead_code)] annotations in codebase"
    - "Zero _for_test() wrapper functions in codebase"
    - "5-file flat layout decomposed into module directories"
    - "All 13 tests live in per-module test blocks, not main.rs"
    - "Zero lock().unwrap() in db module"
    - "SyncStats derives Default"
    - "All 13 tests pass after every wave"
  artifacts:
    - path: "src/app/mod.rs"
      provides: "App struct, enums, core methods"
    - path: "src/app/handlers.rs"
      provides: "Key handling for all focus modes"
    - path: "src/app/image_loader.rs"
      provides: "Image loading thread and poll logic"
    - path: "src/db/mod.rs"
      provides: "Db struct, all query methods, 7 DB tests"
    - path: "src/db/types.rs"
      provides: "Article, FullArticle, ArticleImage, NewArticle, NewImage"
    - path: "src/sync/mod.rs"
      provides: "run_sync orchestration, 3 sync tests"
    - path: "src/sync/scraping.rs"
      provides: "parse_day_page, parse_article_page, data structs"
    - path: "src/sync/download.rs"
      provides: "fetch_page, fetch_image, fetch_and_store_image"
    - path: "src/sync/progress.rs"
      provides: "SyncStats with Default derive"
    - path: "src/ui/mod.rs"
      provides: "render() dispatcher, shared helpers, 2 UI tests"
    - path: "src/ui/calendar.rs"
      provides: "Calendar view rendering"
    - path: "src/ui/article_list.rs"
      provides: "Article list view"
    - path: "src/ui/article_reader.rs"
      provides: "ContentBlock, virtual scrolling, image rendering"
    - path: "src/ui/search.rs"
      provides: "Search input and results"
    - path: "src/ui/help.rs"
      provides: "Help overlay"
    - path: "src/main.rs"
      provides: "CLI entry point and TUI event loop (~130 lines)"
  key_links:
    - from: "All modules"
      to: "src/main.rs"
      via: "Module tree via mod declarations"
      pattern: "mod (app|db|sync|ui)"
    - from: "src/*/mod.rs"
      to: "submodules"
      via: "pub use re-exports preserving external API"
      pattern: "pub use"
---

# Refactor Phase — Master Plan

## Overview

Refactor the entire `lapresse-tui` codebase to follow best Rust practices and modern module architecture. Both idiomatic Rust improvements AND structural decomposition.

## Current State (3,446 lines, 5 files)

```
src/main.rs   — 464 lines: CLI entry + ALL 13 tests mixed in
src/app.rs    — 809 lines: 30-field App struct, 5 key handlers, sync/search/image loading
src/ui.rs     — 1142 lines: 4 view renderers + calendar + search + help + section picker
src/sync.rs   — 733 lines: CLI sync runner, day sync, HTML parsing, image downloading
src/db.rs     — 298 lines: SQLite wrapper, 6 data structs, 13 query methods
```

## Target State (~3,446 lines, 16 files in 4 module directories)

```
src/main.rs              (~130 lines) — CLI parsing, TUI init/restore
src/app/
  mod.rs                 (~400 lines) — App struct, enums, constructor, core methods
  handlers.rs            (~250 lines) — key handling for all focus modes
  image_loader.rs        (~120 lines) — image load thread and poll logic
src/ui/
  mod.rs                 (~130 lines) — render() dispatcher, shared helpers, 2 tests
  calendar.rs            (~250 lines) — calendar view
  article_list.rs        (~150 lines) — article list view
  article_reader.rs      (~350 lines) — ContentBlock, virtual scrolling, images
  search.rs              (~150 lines) — search input + results
  help.rs                (~100 lines) — help overlay
src/sync/
  mod.rs                 (~200 lines) — run_sync, sync_single_day, 3 tests
  scraping.rs            (~250 lines) — parse_day_page, parse_article_page, structs
  download.rs            (~150 lines) — fetch_page, fetch_image, fetch_and_store_image
  progress.rs            (~100 lines) — SyncStats (with Default derive)
src/db/
  mod.rs                 (~250 lines) — Db struct, all methods, 7 tests
  types.rs               (~100 lines) — Article, FullArticle, ArticleImage, NewArticle, NewImage
```

## Wave Structure (5 waves, 8 plans)

```
Wave 1 ─── Plan 01 (dead code + _for_test elimination)
           │
Wave 2 ─── Plan 02 (Default derives)
           │
Wave 3 ─── Plan 03 (sync/ decomposition) ─┐
           Plan 04 (db/ decomposition)    ─┤  ← ALL PARALLEL
           Plan 05 (ui/ decomposition)    ─┤
           Plan 06 (app/ decomposition)   ─┘
           │
Wave 4 ─── Plan 07 (test migration into modules)
           │
Wave 5 ─── Plan 08 (DB error propagation: lock().unwrap() → ?)
```

## Invariants (verified after EVERY wave)

1. `cargo test` — all 13 tests pass
2. `cargo clippy --all-targets -- -D warnings` — zero warnings
3. `cargo fmt --all -- --check` — formatted
4. No comments in code (project convention)
5. Binary name remains `lapresse-tui`

## Plan Summary

| Plan | Wave | Files | Risk | Depends On | What |
|------|------|-------|------|------------|------|
| 01 | 1 | 5 src files | LOW | — | Remove dead code, eliminate _for_test() wrappers |
| 02 | 2 | src/sync.rs | LOW | 01 | Default derives on SyncStats |
| 03 | 3 | src/sync.rs → src/sync/ | MED | 01 | Decompose sync into module directory |
| 04 | 3 | src/db.rs → src/db/ | LOW | 01 | Decompose db into module directory |
| 05 | 3 | src/ui.rs → src/ui/ | MED | 01 | Decompose ui into module directory |
| 06 | 3 | src/app.rs → src/app/ | MED | 01 | Decompose app into module directory |
| 07 | 4 | main.rs + 3 mod.rs | LOW | 03-06 | Move tests into per-module blocks |
| 08 | 5 | db/mod.rs + callers | HIGH | 04 | Replace lock().unwrap() with proper error propagation |

## Key Risks and Mitigations

| Risk | Mitigation |
|------|------------|
| Circular module dependencies | Decomposition preserves API via `pub use` re-exports |
| Import path breakage | Re-exports in mod.rs keep `crate::sync::*` etc. working |
| Test visibility changes | Plan 01 makes functions `pub(crate)` BEFORE decomposition |
| Cascading type changes (Wave 8) | Db methods already return `Result` — internal change only |
| Mutex panic under contention | Plan 08 replaces with `?` propagation |

## Decision Coverage Matrix

Every research finding maps to a plan task:

| Finding | Plan | Task | Coverage |
|---------|------|------|----------|
| 9x `#[allow(dead_code)]` | 01 | Task 1 | FULL |
| 2 unused fields (db_path, article_list_offset) | 01 | Task 1 | FULL |
| 1 unused method (get_pending_dates) | 01 | Task 1 | FULL |
| 4x `_for_test()` wrappers | 01 | Task 2 | FULL |
| Make parse functions `pub(crate)` | 01 | Task 2 | FULL |
| SyncStats missing Default | 02 | Task 1 | FULL |
| sync.rs decomposition | 03 | Task 1 | FULL |
| db.rs decomposition | 04 | Task 1 | FULL |
| ui.rs decomposition | 05 | Task 1 | FULL |
| app.rs decomposition | 06 | Task 1 | FULL |
| Tests in main.rs | 07 | Task 1 | FULL |
| 14x `lock().unwrap()` in db | 08 | Task 1 | FULL |

## Execution

Run plans in wave order:
```
/gsd-execute-phase 99-refactor
```

The executor will run Wave 1 first, then Wave 2, then Wave 3 (Plans 03-06 can run in parallel since they touch different files), then Wave 4, then Wave 5.
</objective>
