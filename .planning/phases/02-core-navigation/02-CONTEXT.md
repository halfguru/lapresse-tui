# Phase 2: Core Navigation - Context

**Gathered:** 2026-04-13
**Status:** Ready for planning

<domain>
## Phase Boundary

Users browse the 2005–2026 archive by date through a calendar-driven interface and see scrollable article lists for any selected day. Includes status bar with contextual info and a help overlay.

Covers requirements: NAV-01 (calendar navigation), NAV-02 (article list), NAV-03 (status bar), and the help overlay from READ-03.

</domain>

<decisions>
## Implementation Decisions

### Calendar Layout & Split
- **D-01:** Side-by-side horizontal split layout — calendar on the left (~35% width), article list on the right (~65% width). Both always visible.
- **D-02:** Calendar renders a single month using ratatui's `Monthly` widget. Note: ratatui's calendar depends on the `time` crate, not `chrono` — may need `time` as an additional dependency or a custom calendar renderer.
- **D-03:** Days with articles should show a visual density indicator (dots or highlights) on the calendar grid.

### Navigation Keybindings
- **D-04:** h/l navigate prev/next month. H/L navigate prev/next year. Fast traversal across the 252-month range (2005–2026).
- **D-05:** j/k moves the highlighted day within the calendar grid. Articles for the highlighted day appear automatically in the list (no Enter needed to load).
- **D-06:** Unified context-aware j/k — when calendar is focused, j/k moves days. When article list is focused, j/k scrolls articles. Tab or Enter switches focus to the list.
- **D-07:** In article list: j/k scroll, Enter opens article (reader opens in Phase 4, placeholder behavior for now), gg/G jump to top/bottom.
- **D-08:** q exits the app from calendar view. From article list, q returns to calendar focus (if list was focused via Tab) or exits (if from top-level).

### Article List Presentation
- **D-09:** Each article row shows: title as main text + section name as a colored badge/tag on the right side. Compact but informative.
- **D-10:** Friendly empty state message when a day has no articles: "No articles for this date" or "Run sync to fetch articles" when DB is empty.
- **D-11:** Article list scrolls smoothly. Highlighted article is visually distinct.

### Help Overlay
- **D-12:** Press ? to toggle a centered popup overlay listing all keybindings. Background content still visible. ? or Esc dismisses it.
- **D-13:** Help overlay shows categorized keybindings: Navigation (h/l/H/L/j/k), Actions (Enter, Tab, q/Esc), Views (?).

### Status Bar
- **D-14:** Bottom status bar (already exists from Phase 1) updates to show: current selected date, article count for that date, detected image protocol, keybinding hints.

### the agent's Discretion
- Exact percentage split for calendar vs list (35/65 is a guideline)
- Color scheme for section badges
- Calendar day highlight style
- Smooth scroll vs jump scroll in article list

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Existing Code
- `src/app.rs` — Current App struct and key handling (will be extended)
- `src/ui.rs` — Current render function (will be replaced with split layout)
- `src/db.rs` — Database layer (needs articles-by-date query added)
- `src/main.rs` — Event loop (will need Picker stored for later phases)
- `migrations/V1__initial_schema.sql` — Schema for articles table (columns: url, title, section, author, published_at, content_text, content_html)

### ratatui Calendar
- ratatui `widgets::calendar::{CalendarEventStore, Monthly}` — built-in calendar widget
- ratatui calendar-explorer example app — reference implementation for interactive calendar
- Note: calendar widget uses `time` crate for `Date`/`Month` types

### Research
- `.planning/research/ARCHITECTURE.md` — TEA pattern, component boundaries
- `.planning/research/FEATURES.md` — Feature prioritization, competitor analysis

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `App` struct: Has `should_quit`, `db`, `protocol_type`, `article_count` — will need `selected_date`, `focus`, `calendar_state`, `article_list_state` fields added
- `Db` struct: Has `article_count()` — needs `articles_by_date(date: &str)` query added
- Status bar rendering in `ui.rs`: Already shows protocol and article count — will be extended with date display
- Key handling in `app.rs`: Currently only q/Esc — will be extended with full vim-style bindings

### Established Patterns
- TEA pattern: event loop in main.rs → `app.handle_key()` → `ui::render()` — clean separation, will scale well
- Constraint-based layout using `Layout::vertical` — can add `Layout::horizontal` for side-by-side split
- `Paragraph` widget for text display — can use `List` widget for article list

### Integration Points
- `ui::render()` function: Currently renders single placeholder — will become the split layout dispatcher
- `App::handle_key()`: Currently flat match on KeyCode — will need focus-aware key routing
- `Db`: Needs date-based article query method for list population
- Article list Enter key: Placeholder for Phase 4 article reader integration

</code_context>

<specifics>
## Specific Ideas

- Calendar day dots/highlights to show article density is a nice touch for a showcase project
- Section badges with distinct colors make the article list visually rich
- Status bar keybinding hints (like `q:quit h/l:month`) help discoverability without the help overlay

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope.

</deferred>

---

*Phase: 02-core-navigation*
*Context gathered: 2026-04-13*
