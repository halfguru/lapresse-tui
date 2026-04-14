# Phase 3: Scraping Pipeline - Context

**Gathered:** 2026-04-14
**Status:** Ready for planning

<domain>
## Phase Boundary

Populate the local SQLite cache from La Presse's public archive at lapresse.ca/archives. Covers requirement DATA-01: app scrapes article content and metadata for specified date ranges. Delivers a CLI sync command that fetches full article text and images, stores them locally, and enables offline browsing in the TUI.

The TUI remains read-only — sync is a separate CLI operation.

</domain>

<decisions>
## Implementation Decisions

### Sync Trigger & UX
- **D-01:** Sync triggered via CLI subcommand only (`lpresse sync`). TUI stays read-only — no in-app sync hotkey or background threading needed.
- **D-02:** Sync accepts optional date range arguments (e.g. `--from 2024-01-01 --to 2024-12-31`). Without args, sync all dates or resume from last synced date.
- **D-03:** Progress shown as terminal output to stdout: date being processed, day count, articles found. Simple progress bar style (no TUI integration).

### Scrape Scope & Depth
- **D-04:** Full scrape: article title, section, author, published date, full text content, HTML content, AND download images into the BLOB column. Phase 4 (reader with images) gets everything it needs.
- **D-05:** Images are downloaded and stored in the `images` table's `data` BLOB column during sync. This makes the archive truly offline-capable.

### Rate Limiting & Politeness
- **D-06:** 2-3 second polite delay between page requests. Standard archival scraping practice. Full 20-year archive (~7300 days) would take ~4-6 hours.
- **D-07:** No adaptive rate limiting — keep it simple with a fixed delay.

### Resume & Error Handling
- **D-08:** Skip-and-retry-later strategy. Failed dates are marked 'failed' in `sync_state`, sync continues to next date. Failed dates are retried on the next sync run.
- **D-09:** Print a summary at the end of sync: total days processed, articles found, failures.
- **D-10:** Resume support: sync_state tracks per-date status (pending/in_progress/complete/failed). Re-running sync skips already-complete dates.

### the agent's Discretion
- Exact HTML selectors for parsing (must be discovered by inspecting actual archive pages)
- HTTP client configuration (user agent, cookies, TLS settings)
- Error classification (network errors vs parse errors vs 404s)
- Internal module structure (how to organize scraper code)

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Existing Code
- `src/db.rs` — Database layer with read methods; needs write methods (insert article, insert image, upsert sync_state)
- `src/main.rs` — CLI entry with clap; needs sync subcommand added
- `src/app.rs` — App struct with db reference and article_count; needs refresh after sync
- `migrations/V1__initial_schema.sql` — Schema defines articles, images, sync_state tables (already has BLOB for image data)

### Research
- `.planning/research/ARCHITECTURE.md` — TEA pattern, component boundaries
- `.planning/research/PITFALLS.md` — Known scraping pitfalls and mitigations

### External (to discover during research)
- `https://lapresse.ca/archives` — Archive page structure, URL patterns, HTML selectors (MUST inspect before implementation)

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `Db` struct: Already has `open()`, `article_count()`, `articles_by_date()`, `article_counts_by_month()` — needs write methods
- `Cli` struct in `main.rs`: Already uses clap derive — extend with `Sync` subcommand
- `migrations/V1__initial_schema.sql`: `sync_state` table already has `date`, `status`, `articles_found`, `articles_scraped`, `last_attempt_at` columns — designed for resume

### Established Patterns
- Schema embedded via `include_str!` (no refinery — simpler)
- `anyhow::Result` for error propagation throughout
- `tracing` for structured logging — useful for scraper progress

### Integration Points
- `main.rs`: Needs `lpresse sync` subcommand that runs the scraper outside the TUI
- `db.rs`: Needs `insert_article()`, `insert_image()`, `upsert_sync_state()`, `get_sync_state()` methods
- `app.rs`: After external sync, `refresh_articles()` on next TUI launch will pick up new data automatically
- Dependencies needed: `reqwest` (async HTTP), `scraper` (HTML parsing), `url` (URL construction) — all in Cargo.toml STACK.md but not yet added

</code_context>

<specifics>
## Specific Ideas

- Progress output should feel like `wget` or `rsync` — clear, informative, not noisy
- The archive URL pattern is likely `lapresse.ca/archives/YYYY/MM/DD` but this MUST be verified during research
- French HTML content may have accented characters in URLs and text — ensure proper UTF-8 handling

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope.

</deferred>

---

*Phase: 03-scraping-pipeline*
*Context gathered: 2026-04-14*
