<!-- GSD:project-start source:PROJECT.md -->
## Project

**lpresse**

A terminal-based La Presse archive reader and analytics dashboard built in Rust with ratatui. Users browse 20 years of Quebec French newspaper articles (2005–2026) through a calendar-driven interface, reading full articles with inline images rendered in the terminal. A showcase GitHub project demonstrating that rich, image-capable TUIs are possible in Rust.

**Core Value:** Reading La Presse articles with images, right in your terminal.

### Constraints

- **Tech stack**: Rust with ratatui — decided, non-negotiable
- **Image rendering**: Must use ratatui-image crate — the headline feature
- **Data source**: lapresse.ca/archives — scraping only, no API available
- **Offline-capable**: Local SQLite cache means browsing works without network after initial sync
- **Terminal compatibility**: Must gracefully degrade images for terminals without Sixel/Kitty support
<!-- GSD:project-end -->

<!-- GSD:stack-start source:research/STACK.md -->
## Technology Stack

## Recommended Stack
### Core Framework
| Technology | Version | Purpose | Why | Confidence |
|------------|---------|---------|-----|------------|
| **ratatui** | 0.30.0 | TUI framework | The standard Rust TUI library. Active development, massive ecosystem, `ratatui::init()`/`ratatui::restore()` simplifies setup. Required by ratatui-image. | HIGH |
| **crossterm** | 0.29.0 | Terminal backend | Default backend for ratatui 0.30. Cross-platform, handles raw mode/events. Pin to match ratatui's expected version. | HIGH |
| **tokio** | 1.51 | Async runtime | Required for reqwest async HTTP, background scraping tasks, and channel-based communication between UI and scraper. Use `rt-multi-thread` + `macros` features. | HIGH |
### Image Rendering
| Technology | Version | Purpose | Why | Confidence |
|------------|---------|---------|-----|------------|
| **ratatui-image** | 10.0.6 | Image widget for ratatui | The headline feature. Supports Sixel, Kitty, iTerm2, and half-block fallback. Depends on ratatui 0.30 — confirmed compatible. Uses `Picker::from_query_stdio()` for auto-detecting terminal protocol. | HIGH |
| **image** | 0.25.10 | Image decoding/processing | Required by ratatui-image (uses `^0.25.6`). Decodes JPEG/PNG/WebP from scraped content. Use `image/default` for broad format support. | HIGH |
### Web Scraping
| Technology | Version | Purpose | Why | Confidence |
|------------|---------|---------|-----|------------|
| **reqwest** | 0.13.2 | HTTP client | Standard async HTTP client. Cookie support for session management, redirect following, TLS built-in. Use `cookies` feature for archive navigation that may require sessions. | HIGH |
| **scraper** | 0.26.0 | HTML parsing + CSS selectors | Browser-grade HTML parsing via Servo's html5ever. CSS selector queries to extract article titles, dates, content, and image URLs. Latest version updated March 2026. | HIGH |
| **url** | 2.5.8 | URL parsing and construction | Parse and build archive URLs (`lapresse.ca/archives/...`). Handles relative→absolute URL resolution for scraped image paths. | HIGH |
### Database
| Technology | Version | Purpose | Why | Confidence |
|------------|---------|---------|-----|------------|
| **rusqlite** | 0.39.0 | SQLite bindings | Local article/image cache for offline browsing. Use `bundled` feature to statically link SQLite — avoids system dependency. Supports BLOB storage for image data. | HIGH |
| **refinery** | 0.8 | Database migrations | Declarative SQL migration runner for rusqlite. Embeds migration files, runs them in order, tracks version in SQLite. Better than hand-rolling migration logic. | MEDIUM |
### Date/Time
| Technology | Version | Purpose | Why | Confidence |
|------------|---------|---------|-----|------------|
| **chrono** | 0.4.44 | Date handling | Calendar-driven navigation requires parsing "2005-2026" date ranges, formatting display dates, iterating over months/days. The standard Rust datetime library. Use `serde` feature for DB serialization. | HIGH |
### CLI
| Technology | Version | Purpose | Why | Confidence |
|------------|---------|---------|-----|------------|
| **clap** | 4.5 | CLI argument parser | Subcommands for `sync`, `read`, `stats`. Derive API for clean definition. Auto-generated help. | HIGH |
| **dirs** | 6.0.0 | Platform directories | Locates cache directory (`~/.cache/lpresse/` on Linux, proper macOS/Windows equivalents). Tiny, zero-config. | HIGH |
### Error Handling & Logging
| Technology | Version | Purpose | Why | Confidence |
|------------|---------|---------|-----|------------|
| **anyhow** | 1.0.102 | Application error handling | Flexible error type with `.context()`. Perfect for a CLI tool where you want error chains but don't need a typed error hierarchy. | HIGH |
| **thiserror** | 1.0 | Library error types | Use for domain-specific error enums in the scraper/db modules where callers need to match on specific errors. ratatui-image already uses thiserror internally. | HIGH |
| **tracing** | 0.1.44 | Structured logging | Instrument scraping operations, track sync progress, debug image rendering issues. Use `tracing-subscriber` for output formatting. Better than `log` for async contexts. | HIGH |
### Serialization
| Technology | Version | Purpose | Why | Confidence |
|------------|---------|---------|-----|------------|
| **serde** | 1.0.228 | Serialization framework | DB record serialization, potential config file support. Required by many ecosystem crates. Use `derive` feature. | HIGH |
| **serde_json** | 1.0 | JSON handling | Parse any JSON embedded in archive pages, store structured metadata. | HIGH |
## Alternatives Considered
| Category | Recommended | Alternative | Why Not |
|----------|-------------|-------------|---------|
| TUI Framework | ratatui 0.30 | cursive | cursive doesn't support image rendering. ratatui-image only works with ratatui. No contest. |
| TUI Backend | crossterm | termion | ratatui's default backend is crossterm. ratatui-image's crossterm support is best-tested. termion backend exists but has known issues in ratatui-image. |
| HTTP Client | reqwest | ureq | ureq is sync-only. We need async for background scraping alongside the TUI event loop. reqwest is the standard. |
| HTML Parser | scraper | select.rs, lol_html | scraper uses Servo-grade parsing. select.rs is unmaintained. lol_html is streaming (overkill for page-level scraping). |
| Database | rusqlite | sled, redb | SQLite is the standard for local caching. rusqlite is mature, supports BLOB for images, and has `bundled` feature. sled/redb are key-value stores without SQL query power for analytics. |
| Image Decoding | image | No alternative | ratatui-image hard-depends on the `image` crate. Not a choice. |
| Date/Time | chrono | time | chrono is the ecosystem standard. time v0.3 has a cleaner API but chrono has broader ecosystem support and better date iteration for calendar navigation. |
| Error Handling | anyhow | eyre | eyre adds context traces but is slower to compile. anyhow is lighter and sufficient. |
| Async Runtime | tokio | async-std | reqwest, ratatui-image (via crossterm event-stream), and the broader ecosystem are tokio-first. async-std is viable but requires more compatibility shims. |
| DB Migrations | refinery | rusqlite_migration | rusqlite_migration is simpler but refinery has better support for embedded migrations and is more widely adopted. |
## Cargo.toml Template
# TUI
# Images
# Async
# Scraping
# Database
# Date/Time
# CLI
# Error handling
# Logging
# Serialization
## Version Pinning Strategy
- `ratatui` 0.30 + `ratatui-image` 10.0 + `crossterm` 0.29 — these are a matched set
- `image` 0.25 — pinned by ratatui-image's dependency
- reqwest, scraper, tokio, rusqlite, chrono, clap, anyhow, tracing, serde
## Key Architecture Implications
## Sources
- ratatui 0.30.0: crates.io (verified 2025-12-26), Context7 docs
- ratatui-image 10.0.6: crates.io (verified 2026-02-19), GitHub README with compatibility matrix
- crossterm 0.29.0: crates.io (verified 2025-04-05)
- tokio 1.51.1: crates.io (verified 2026-04-08)
- reqwest 0.13.2: crates.io (verified 2026-02-06)
- scraper 0.26.0: crates.io (verified 2026-03-18)
- rusqlite 0.39.0: crates.io (verified 2026-03-15)
- image 0.25.10: crates.io (verified 2026-03-10)
- ratatui-image dependency tree: crates.io API `/dependencies` endpoint (verified ratatui ^0.30.0, image ^0.25.6)
- chrono 0.4.44: crates.io (verified 2026-02-23)
- clap 4.5.x: crates.io
- anyhow 1.0.102: crates.io (verified 2026-02-20)
- dirs 6.0.0: crates.io (verified 2025-01-12)
- serde 1.0.228: crates.io (verified 2025-09-27)
- tracing 0.1.44: crates.io (verified 2025-12-18)
- url 2.5.8: crates.io (verified 2026-01-05)
<!-- GSD:stack-end -->

<!-- GSD:conventions-start source:CONVENTIONS.md -->
## Conventions

Conventions not yet established. Will populate as patterns emerge during development.
<!-- GSD:conventions-end -->

<!-- GSD:architecture-start source:ARCHITECTURE.md -->
## Architecture

Architecture not yet mapped. Follow existing patterns found in the codebase.
<!-- GSD:architecture-end -->

<!-- GSD:skills-start source:skills/ -->
## Project Skills

No project skills found. Add skills to any of: `.claude/skills/`, `.agents/skills/`, `.cursor/skills/`, or `.github/skills/` with a `SKILL.md` index file.
<!-- GSD:skills-end -->

<!-- GSD:workflow-start source:GSD defaults -->
## GSD Workflow Enforcement

Before using Edit, Write, or other file-changing tools, start work through a GSD command so planning artifacts and execution context stay in sync.

Use these entry points:
- `/gsd-quick` for small fixes, doc updates, and ad-hoc tasks
- `/gsd-debug` for investigation and bug fixing
- `/gsd-execute-phase` for planned phase work

Do not make direct repo edits outside a GSD workflow unless the user explicitly asks to bypass it.
<!-- GSD:workflow-end -->



<!-- GSD:profile-start -->
## Developer Profile

> Profile not yet configured. Run `/gsd-profile-user` to generate your developer profile.
> This section is managed by `generate-claude-profile` -- do not edit manually.
<!-- GSD:profile-end -->
