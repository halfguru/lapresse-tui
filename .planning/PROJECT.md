# lpresse

## What This Is

A terminal-based La Presse archive reader and analytics dashboard built in Rust with ratatui. Users browse 20 years of Quebec French newspaper articles (2005–2026) through a calendar-driven interface, reading full articles with inline images rendered in the terminal. A showcase GitHub project demonstrating that rich, image-capable TUIs are possible in Rust.

## Core Value

Reading La Presse articles with images, right in your terminal.

## Requirements

### Validated

(None yet — ship to validate)

### Active

- [ ] Calendar-driven navigation across the full 2005–2026 archive
- [ ] Article list view for any selected day
- [ ] Rich article reader with full text and inline images
- [ ] Image rendering via ratatui-image (Sixel, Kitty, half-block fallback)
- [ ] Local SQLite cache of scraped articles and images
- [ ] Background scraping/sync from lapresse.ca/archives
- [ ] Analytics/stats dashboard (secondary feature — scope TBD)

### Out of Scope

- User accounts or authentication — no login, it's a local tool
- Article commenting or interaction — read-only experience
- Real-time news feed — this is an archive browser, not a live feed
- Mobile or web version — terminal only
- Paid/paywall content handling — only freely available archive content

## Context

- La Presse is a major Quebec French-language newspaper with a public archive at lapresse.ca/archives spanning 2005–2026
- The archive is calendar-based: navigate by year/month/day to see published articles
- There is no public API — articles are scraped from HTML
- ratatui-image provides real image rendering in terminals via Sixel, Kitty, iTerm2, and half-block protocols
- This is a showcase project — code quality, polish, and presentation matter
- Rust ecosystem: ratatui (TUI), ratatui-image (images), reqwest (HTTP), scraper (HTML), rusqlite (SQLite), tokio (async)

## Constraints

- **Tech stack**: Rust with ratatui — decided, non-negotiable
- **Image rendering**: Must use ratatui-image crate — the headline feature
- **Data source**: lapresse.ca/archives — scraping only, no API available
- **Offline-capable**: Local SQLite cache means browsing works without network after initial sync
- **Terminal compatibility**: Must gracefully degrade images for terminals without Sixel/Kitty support

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| Rust + ratatui | Performance, single binary, ratatui-image ecosystem | — Pending |
| Calendar-first navigation | Mirrors the archive structure, intuitive newspaper metaphor | — Pending |
| SQLite local cache | Enables offline browsing, powers future analytics | — Pending |
| Reader-first, analytics-second | Reading experience is the draw, analytics is the bonus | — Pending |
| Showcase quality | This is a portfolio piece — polish, docs, and presentation matter | — Pending |

## Evolution

This document evolves at phase transitions and milestone boundaries.

**After each phase transition** (via `/gsd-transition`):
1. Requirements invalidated? → Move to Out of Scope with reason
2. Requirements validated? → Move to Validated with phase reference
3. New requirements emerged? → Add to Active
4. Decisions to log? → Add to Key Decisions
5. "What This Is" still accurate? → Update if drifted

**After each milestone** (via `/gsd-complete-milestone`):
1. Full review of all sections
2. Core Value check — still the right priority?
3. Audit Out of Scope — reasons still valid?
4. Update Context with current state

---
*Last updated: 2026-04-13 after initialization*
