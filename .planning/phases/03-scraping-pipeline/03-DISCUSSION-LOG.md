# Phase 3: Scraping Pipeline - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in 03-CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-04-14
**Phase:** 03-scraping-pipeline
**Areas discussed:** Sync trigger & UX, Scrape scope & depth, Rate limiting & politeness, Resume & error handling

---

## Sync Trigger & UX

| Option | Description | Selected |
|--------|-------------|----------|
| CLI subcommand only | `lpresse sync` — TUI stays read-only, no background threading | ✓ |
| Both CLI + in-app hotkey | CLI for bulk, plus `s` key in TUI for current month sync | |
| In-app only | Sync only from within the TUI, no CLI subcommand | |

**User's choice:** CLI subcommand only
**Notes:** Simplest implementation. No need for tokio channels or background sync integration with the TUI event loop. Progress shown as terminal output to stdout.

## Scrape Scope & Depth

| Option | Description | Selected |
|--------|-------------|----------|
| Full text + images | Title, section, author, date, full text, HTML, AND image BLOBs | ✓ |
| Metadata + full text, no images | Skip image downloads; fetch on-demand in Phase 4 | |
| Metadata only | Just title, section, date, URL | |

**User's choice:** Full text + images
**Notes:** Phase 4 (reader with images) gets everything it needs. Images stored in the `images.data` BLOB column for true offline capability.

## Rate Limiting & Politeness

| Option | Description | Selected |
|--------|-------------|----------|
| Polite: 2-3 sec delay | Standard archival practice. ~4-6 hours for full 20-year archive | ✓ |
| Conservative: 5+ sec delay | Very safe but 10+ hours for full archive | |
| Adaptive | Start at 2 sec, back off to 5+ on 429s | |

**User's choice:** Polite: 2-3 sec delay
**Notes:** Fixed delay, no adaptive logic. Keep it simple.

## Resume & Error Handling

| Option | Description | Selected |
|--------|-------------|----------|
| Skip + retry later | Mark failed, continue, retry on next run. Print failure summary | ✓ |
| Stop on first error | Halt immediately on any failure | |
| Retry with backoff in-line | 3 retries per date with exponential backoff | |

**User's choice:** Skip + retry later
**Notes:** Resilient for a 20-year scrape. The sync_state table already has the right schema for tracking per-date status.

## the agent's Discretion

- HTML selectors for parsing (must be discovered from actual pages)
- HTTP client configuration
- Error classification
- Internal module structure

## Deferred Ideas

None — discussion stayed within phase scope.
