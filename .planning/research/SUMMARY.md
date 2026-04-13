# Project Research Summary

**Project:** lpresse — Terminal-based La Presse archive reader with image rendering
**Domain:** Rust TUI application (web scraper + local cache + terminal image rendering)
**Researched:** 2026-04-13
**Confidence:** HIGH

## Executive Summary

lpresse is a terminal-based newspaper archive reader for La Presse (2005–2026), built with Rust. The project centers on a unique combination of calendar-driven navigation and inline image rendering — no existing TUI news reader renders images, making this a showcase application. The architecture follows The Elm Architecture (TEA) pattern on top of ratatui, with tokio async workers for background scraping and image encoding, and SQLite for local offline caching. All core technologies (ratatui 0.30, ratatui-image 10.0, crossterm 0.29, rusqlite 0.39, reqwest 0.13) are version-verified with confirmed compatibility.

The recommended approach is to build in five phases, starting with the data layer and TUI scaffolding (storage schema, event loop, panic hooks), then core views (calendar, article list, status bar), then the scraping pipeline, then the reader with inline images (the capstone feature), and finally polish and analytics. The critical constraint is that ratatui, ratatui-image, and crossterm form a locked version trio — upgrading any one independently will break image rendering. The highest-risk area is the web scraping layer: La Presse has no public API, so HTML scraping must be resilient, rate-limited, and resume-capable for 365K+ potential page fetches across 20 years.

Key risks include: terminal state corruption on panics (mitigated by panic hooks), SQLite blocking the async runtime (mitigated by `spawn_blocking`), image protocol detection ordering (must happen after alternate screen but before event stream), and scraping rate-limiting/IP bans (mitigated by polite delays, sync_state tracking, and incremental scraping). All critical pitfalls have well-documented prevention patterns from real-world ratatui-image applications.

## Key Findings

### Recommended Stack

The stack is built around the ratatui ecosystem with tokio for async I/O and rusqlite for local storage. All versions were verified via crates.io API and Context7 documentation. The three critical version locks are ratatui 0.30 + ratatui-image 10.0 + crossterm 0.29 — these must be upgraded as a unit.

**Core technologies:**
- **ratatui 0.30 + crossterm 0.29:** TUI framework and terminal backend — the standard Rust TUI stack, required by ratatui-image
- **ratatui-image 10.0:** Image widget supporting Sixel, Kitty, iTerm2, and half-block fallback — the headline feature enabling inline images
- **tokio 1.x (rt-multi-thread + macros):** Async runtime — required for background scraping, image encoding, and channel-based UI communication
- **rusqlite 0.39 (bundled):** SQLite bindings — local article/image cache for offline browsing, FTS5 search support
- **reqwest 0.13 (cookies) + scraper 0.26:** HTTP client + HTML parser — web scraping pipeline for lapresse.ca/archives
- **image 0.25:** Image decoding — required by ratatui-image, handles JPEG/PNG/WebP from scraped content
- **refinery 0.8:** Database migrations — declarative SQL migration runner for rusqlite
- **chrono 0.4:** Date handling — calendar navigation across 2005–2026 date range
- **clap 4 (derive):** CLI argument parser — subcommands for sync, read, stats
- **anyhow + thiserror:** Error handling — anyhow for application errors, thiserror for domain error types
- **tracing + tracing-subscriber:** Structured logging — instrument scraping, sync progress, image rendering

### Expected Features

Feature research identified a clear MVP scope and a natural progression from core browsing to analytics. The calendar-first navigation model is the key differentiator from existing TUI news readers (newsboat, eilmeldung) which use feed-list-first navigation.

**Must have (table stakes — P1):**
- Calendar-driven date navigation — the primary entry point, maps to lapresse.ca/archives metaphor
- Article list for selected date — scrollable with title, section, read status
- Full article reader with text — scrollable, proper French Unicode wrapping
- Inline image rendering — the showcase feature; Sixel/Kitty/iTerm2/half-block with graceful degradation
- SQLite cache — offline browsing; articles, images, metadata
- Basic scraping from lapresse.ca/archives — background fetch, rate-limited, resume-capable
- Vim keybindings (j/k/g/G/Ctrl-d/Ctrl-u/q) — terminal user expectation
- Help screen (`?`), status bar, scroll indicators, read/unread tracking

**Should have (differentiators — P2):**
- Full-text search (FTS5) — search across all cached articles
- "On this day" historical view — high emotional engagement, low implementation cost
- Bookmarking/favorites, section browsing, author tracking
- Async background sync with progress indicator
- Image gallery mode

**Defer (v2+):**
- Analytics dashboard with ratatui charts (BarChart, Sparkline, Chart)
- Term frequency trends over time
- Article export (markdown/text), configuration file, mouse support

**Anti-features (explicitly out of scope):**
- Live RSS feed, real-time updates, user accounts, commenting, PDF rendering, AI/LLM summarization, multi-newspaper support, plugin system

### Architecture Approach

The architecture uses The Elm Architecture (TEA) with async extensions: a central `App` model holds all UI state, events are mapped to typed `Action` enums, an update function produces new state, and views are pure rendering functions. Background workers (scraper, image encoder) communicate results back through a `tokio::sync::mpsc` channel, keeping the UI responsive. Image encoding uses ratatui-image's `ThreadProtocol` for non-blocking resize/encode operations.

**Major components:**
1. **App State (app.rs)** — Central TEA model: current view, navigation stack, selected date/article, sync status, action channel
2. **Presentation Layer (ui/)** — Separate modules per view (calendar, article_list, reader, analytics, status_bar), dispatched by current view
3. **Scraper Service (scraper/)** — Async HTTP + HTML parsing pipeline: client with rate limiting, archive page parser, article page parser
4. **Storage Service (storage/)** — SQLite CRUD with refinery migrations, runs via `spawn_blocking` to avoid blocking tokio
5. **Image Service (image/)** — Bridges storage (image blobs on disk) and UI (ratatui-image ThreadProtocol state)

**Critical initialization sequence:** Enter alternate screen → `Picker::from_query_stdio()` → start crossterm event stream → enter main loop. Wrong order causes deadlock, corrupted detection, or no image support.

### Critical Pitfalls

1. **Terminal left in raw mode on panic** — Install panic hook at app start that calls `disable_raw_mode()` + `LeaveAlternateScreen` + `Show` before delegating to original handler. Every production ratatui app does this.
2. **SQLite blocking the tokio runtime** — Always wrap rusqlite calls in `tokio::task::spawn_blocking()`. Use WAL mode, bulk insert transactions, and create indexes after data loads.
3. **Image protocol detection ordering** — `Picker::from_query_stdio()` must be called after alternate screen but before event stream starts. Always fallback to `Picker::halfblocks()`.
4. **Scraping without rate limiting or resume** — Store per-day sync state in SQLite, use 500ms+ delay between requests, scrape incrementally (discover URLs then download content separately), implement adaptive backoff on 429/503.
5. **Sixel images on last terminal line cause scroll corruption** — Add 1-row bottom margin to image layouts. Known ratatui-image issue #57, affects only Sixel terminals (xterm, foot).

## Implications for Roadmap

Based on combined research, five phases are recommended:

### Phase 1: Foundation (Scaffolding + Data Layer)
**Rationale:** Everything depends on the data layer and the app skeleton. Storage schema, migrations, and the TEA event loop must exist before any views or scraping can be built. Panic hooks and Picker initialization order are one-time setup decisions that affect the entire app lifecycle.
**Delivers:** Working TUI skeleton with empty views, SQLite schema with migrations, event loop with action dispatch, panic hook, image protocol detection
**Addresses:** SQLite cache (P1), panic hook prevention, Picker init ordering, WAL mode setup
**Avoids:** Terminal corruption on panic, blocked async runtime, wrong Picker initialization

### Phase 2: Core Views (Calendar + Article List + Status Bar)
**Rationale:** Views can be built with dummy data before the scraper exists. Calendar and article list are the primary navigation surfaces — getting them right shapes the entire UX. Status bar provides the sync/status feedback loop needed before implementing actual scraping.
**Delivers:** Calendar navigation component, scrollable article list, status bar, vim keybindings, help screen (`?`)
**Uses:** ratatui widgets (Table/List/Paragraph/Block), chrono for date iteration
**Implements:** Presentation layer components from ARCHITECTURE.md

### Phase 3: Scraping Pipeline (Data Population)
**Rationale:** With views and storage ready, the scraper populates the database and connects everything. This is the highest-risk phase — web scraping of a site with no API requires resilient selectors, rate limiting, and resume capability. The sync_state table enables crash recovery.
**Delivers:** Working scraper for lapresse.ca/archives, rate-limited HTTP client, HTML parsers for archive and article pages, background sync with progress reporting, offline capability
**Uses:** reqwest (cookies), scraper (CSS selectors), tokio spawn + channels
**Avoids:** IP bans (rate limiting), data loss on crash (sync_state), blocking UI (async workers)

### Phase 4: Reader with Inline Images (The Capstone)
**Rationale:** The article reader with inline images is the showcase feature. It depends on the scraper (to download images) and the reader view (for layout). Image encoding via ThreadProtocol must not block rendering. This phase requires testing across terminal types (Kitty, Sixel, iTerm2, half-block fallback).
**Delivers:** Full article reader with scrollable text + inline images, image download and filesystem cache, ThreadProtocol-based async encoding, graceful degradation across terminals
**Uses:** ratatui-image (StatefulImage, ThreadProtocol, Picker), image crate (decode), spawn_blocking for encoding
**Avoids:** UI freeze during image decode, Sixel last-line scroll bug, resize state corruption

### Phase 5: Polish + Enhanced Features
**Rationale:** Full-text search, "on this day", bookmarking, and analytics are all additive features that build on the stable core. They require a populated database to be meaningful and a working reader to integrate into.
**Delivers:** FTS5 search, "on this day" view, bookmarking, section/author browsing, read/unread tracking persistence, loading states, error handling, edge cases
**Uses:** SQLite FTS5, ratatui BarChart/Chart/Sparkline for analytics

### Phase Ordering Rationale

- **Storage before views:** The schema defines the data contract. Building views without knowing the data shape leads to rework.
- **Views before scraping:** Views can be prototyped with dummy data. Getting the UX right first avoids building a scraper that produces data in the wrong shape.
- **Scraping before images:** Images depend on the scraper to download them and the storage to cache paths. The scraper must work before image rendering is meaningful.
- **Images last among P1 features:** Image rendering is the most complex integration point (ThreadProtocol, async encoding, terminal protocol detection). It should be built on a stable foundation.
- **Analytics deferred:** Analytics needs a populated database and working views to provide value. Building it earlier means testing with empty data.

### Research Flags

Phases likely needing deeper research during planning:
- **Phase 3 (Scraping):** La Presse's archive HTML structure is unknown — selectors must be discovered by inspecting actual pages. This is the highest-uncertainty phase. Strongly recommend `/gsd-research-phase` to analyze actual HTML structure before implementation.
- **Phase 4 (Reader with Images):** ThreadProtocol integration patterns are documented but complex. The inline text+image interleaving layout has no direct reference implementation. Research ratatui-image's tokio example and spotatui's CoverArt pattern in detail.

Phases with standard patterns (skip research-phase):
- **Phase 1 (Foundation):** Well-documented TEA pattern, panic hook recipe from ratatui docs, rusqlite WAL setup from Anki/NX patterns.
- **Phase 2 (Core Views):** Standard ratatui widgets (List, Paragraph, Block). Calendar widget has a reference example in ratatui repo.

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Stack | HIGH | All versions verified via crates.io API. ratatui-image dependency tree confirmed. Alternative analysis thorough. |
| Features | HIGH | Competitor analysis across 3 reference TUI apps (newsboat, eilmeldung, arxivlens). Feature dependencies clearly mapped. MVP scope well-defined. |
| Architecture | HIGH | TEA pattern is ratatui's recommended approach. ThreadProtocol documented in ratatui-image README. Real-world examples from eilmeldung, spotatui, linutil. |
| Pitfalls | HIGH | 8 critical pitfalls with prevention code. Sourced from official ratatui recipes, ratatui-image issues, and real-world Rust TUI projects. |

**Overall confidence:** HIGH

### Gaps to Address

- **La Presse archive HTML structure:** Unknown until we inspect actual pages. Selectors for article titles, body content, image URLs, dates, sections, and authors must be discovered empirically. This is the single biggest implementation unknown. Plan to fetch and analyze sample archive pages before Phase 3 implementation.
- **Archive date range coverage:** The archive may not have consistent HTML structure across all 20 years. Older articles (2005–2010) may have different page layouts than recent ones. Scrapers need resilient selectors that handle variation.
- **Image count per article:** Unknown whether articles typically have 0, 1, or many images. The reader layout (text-only vs interleaved) depends on this. Assume 0–3 images per article, design for graceful handling of all cases.
- **Terminal image quality in practice:** Half-block fallback looks significantly worse than Sixel/Kitty. The UX impact of degraded images hasn't been validated. Consider showing a text placeholder ("📷 [image: caption]") instead of half-block rendering for a cleaner experience in unsupported terminals.

## Sources

### Primary (HIGH confidence)
- ratatui 0.30.0 — crates.io (verified 2025-12-26), Context7 docs (application patterns, async tutorial, widget reference)
- ratatui-image 10.0.6 — crates.io (verified 2026-02-19), GitHub README (ThreadProtocol, Picker, compatibility matrix, known issues)
- crossterm 0.29.0 — crates.io (verified 2025-04-05)
- rusqlite 0.39.0 — crates.io (verified 2026-03-15), Context7 docs (FTS5, pragma, transactions)
- ratatui official recipes — panic hooks, async patterns (ratatui.rs)
- ratatui calendar-explorer example — reference implementation for calendar navigation
- ratatui-image issue #57 — Sixel last-line scroll bug documentation

### Secondary (MEDIUM confidence)
- eilmeldung (746 stars) — Real-world ratatui news reader with image support, project structure reference
- spotatui — CoverArt pattern for StatefulProtocol management in TUI
- ChrisTitusTech/linutil — Logo component using ratatui-image Picker
- Anki SQLite setup — WAL mode, cache_size, busy_handler production patterns
- newsboat (3.8k stars) — Reference TUI news reader, feature comparison baseline

### Tertiary (LOW confidence)
- La Presse archive HTML structure — Not inspected; requires empirical analysis
- Archive date range consistency — Assumed varied structure across 20 years
- Image density per article — Assumed 0–3 images; needs validation

---
*Research completed: 2026-04-13*
*Ready for roadmap: yes*
