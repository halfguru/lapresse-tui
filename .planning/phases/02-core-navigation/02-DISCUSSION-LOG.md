# Phase 2: Core Navigation - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-04-13
**Phase:** 2-Core Navigation
**Areas discussed:** Calendar layout & split, Navigation keybindings, Article list density, Help overlay style

---

## Calendar Layout & Split

| Option | Description | Selected |
|--------|-------------|----------|
| Side-by-side split | Calendar left (~35%), article list right (~65%). Both always visible. | ✓ |
| Stacked vertical | Calendar on top (fixed height), article list below. | |
| Tabbed views | Switch between calendar and article list views. | |

**User's choice:** Side-by-side split
**Notes:** Classic TUI pattern, both views always visible, good use of wide terminals.

---

## Navigation Keybindings — Month/Year Navigation

| Option | Description | Selected |
|--------|-------------|----------|
| h/l prev/next month | vim-style: h=prev month, l=next month. H/L for prev/next year. | ✓ |
| Arrow keys + Shift | Left/Right=month, Shift+Left/Right=year. | |
| j/k browse days | Move cursor on individual days within calendar grid. | |

**User's choice:** h/l prev/next month
**Notes:** Familiar vim convention. H/L for year jumps covers the 2005–2026 range efficiently.

---

## Navigation Keybindings — Day Selection

| Option | Description | Selected |
|--------|-------------|----------|
| j/k moves day highlight | j/k moves highlighted day. Articles show automatically. | ✓ |
| Auto-select on month nav | Auto-select day 1, type number for specific day. | |
| Tab to switch focus | Separate focus areas for calendar and list. | |

**User's choice:** j/k moves day highlight
**Notes:** Articles update live as the day highlight moves — no extra keypress needed.

---

## Navigation Keybindings — List & Focus Model

| Option | Description | Selected |
|--------|-------------|----------|
| Enter to open article | j/k scrolls list when focused, Enter opens article. Tab switches focus. | ✓ |
| Auto-open on day highlight | No list navigation yet, just context display. | |
| Tab focus + full vim nav | Explicit mode switch between calendar and list. | |

**User's choice:** Enter to open article
**Notes:** Placeholder for Phase 4 reader. List navigation ready but reader deferred.

---

## Navigation Keybindings — Unified vs Modeled

| Option | Description | Selected |
|--------|-------------|----------|
| Unified context-aware j/k | j/k works based on current focus. Tab switches focus. | ✓ |
| Two modes | Explicit calendar mode vs list mode with mode switch. | |

**User's choice:** Unified context-aware j/k
**Notes:** Simpler mental model. Focus indicated visually (highlight border or similar).

---

## Article List Density — Info Per Row

| Option | Description | Selected |
|--------|-------------|----------|
| Title + section badge | Title as main text, section as colored badge on right. | ✓ |
| Title only | Minimal, maximum title space. | |
| Title + section + time + read dot | Full info, dense but complete. | |

**User's choice:** Title + section badge
**Notes:** Compact but informative. Section color-coding aids scanning.

---

## Article List Density — Empty State

| Option | Description | Selected |
|--------|-------------|----------|
| Friendly message | "No articles for this date" or "Run sync to fetch articles". | ✓ |
| Blank list | Empty bordered area, count shows 0. | |
| Calendar dot indicators | Dots on days with articles + message in empty list. | |

**User's choice:** Friendly message
**Notes:** Guides the user — especially useful before first sync when DB is empty.

---

## Help Overlay Style

| Option | Description | Selected |
|--------|-------------|----------|
| Centered popup overlay | Bordered box in center, background visible, ? toggles. | ✓ |
| Full-screen help page | Takes over entire screen. | |
| Compact hint bar | Replaces status bar temporarily. | |

**User's choice:** Centered popup overlay
**Notes:** Standard TUI pattern. Categorized keybindings for clarity.

---

## the agent's Discretion

- Exact split percentages (35/65 guideline)
- Section badge color scheme
- Calendar day highlight style
- Scroll behavior in article list

## Deferred Ideas

None — discussion stayed within phase scope.
