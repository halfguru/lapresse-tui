# Feature Research

**Domain:** Terminal-based newspaper archive reader with image rendering
**Researched:** 2026-04-13
**Confidence:** HIGH

## Feature Landscape

### Table Stakes (Users Expect These)

Features users assume exist. Missing these = product feels incomplete.

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| Calendar-driven date navigation | This is how lapresse.ca/archives works — users expect the same metaphor | MEDIUM | Ratatui has a built-in `Calendar` widget with a calendar-explorer example app. Year/month/day navigation with vim-like keys (h/l for month, j/k for week). |
| Article list for selected date | The fundamental browsing pattern: pick a day, see what was published | LOW | Ratatui `List` widget with `ListState` for selection. Scrollable, highlight current. Straightforward. |
| Full article reader with text | The whole point of the app — reading articles | MEDIUM | Ratatui `Paragraph` widget with wrapping, scrolling. Need to handle long-form French text with proper Unicode. `Scrollable` or manual scroll offset tracking. |
| Vim-like keybindings (j/k/g/G/Ctrl-d/Ctrl-u) | Terminal users expect vim motions; every reference TUI (newsboat, eilmeldung, arxivlens) uses them | LOW | Key event matching in the event loop. Standard pattern. |
| Help screen (?) | Users need discoverability; eilmeldung, arxivlens, newsboat all have `?` for help | LOW | Popup/overlay rendered on top. Ratatui `Paragraph` in a centered `Rect` with `Clear` widget underneath. |
| Image rendering in terminal | This is the headline feature — it's why this project exists | HIGH | ratatui-image crate. `Picker::from_query_stdio()` detects terminal protocol. `StatefulImage` widget for render state. Must handle Sixel (xterm, foot), Kitty (kitty, ghostty), iTerm2 (wezterm, iTerm2), half-block fallback. |
| Graceful image degradation | Not every terminal supports Sixel/Kitty; half-block fallback must work | MEDIUM | ratatui-image handles this via `Picker` protocol detection. Half-block renders everywhere but looks pixelated. Must show meaningful placeholder when images can't load at all. |
| Status bar / info line | Users need context: current date, article count, sync status, terminal protocol | LOW | Fixed-height bottom or top `Paragraph` or `Line` in a `Block`. Standard pattern. |
| Scroll indicators | Users need to know where they are in a long article or list | LOW | Ratatui `Scrollbar` widget. `ScrollbarState` tracks position. |
| Read/unread or visited tracking | Basic reading state — newsboat and eilmeldung both track this | LOW | Boolean flag in SQLite. Visual indicator (unread bold, read normal). Persisted across sessions. |
| Sync/scrape from archive | Data must come from somewhere; background fetch is expected | HIGH | `reqwest` for HTTP, `scraper` for HTML parsing. Tokio async runtime. Archive structure is calendar-based so scrape by date range. Must handle rate limiting, network errors. |
| Offline capability via SQLite cache | PROJECT.md requires it — browse without network after initial sync | MEDIUM | `rusqlite` for local cache. Articles, images, metadata. FTS5 for search. WAL mode for concurrent reads during sync. |

### Differentiators (Competitive Advantage)

Features that set the product apart. Not required, but valuable.

| Feature | Value Proposition | Complexity | Notes |
|---------|-------------------|------------|-------|
| Inline images in article reader | **THE differentiator.** No other TUI news reader renders images inline. This is what makes the project a showcase. | HIGH | ratatui-image `StatefulImage` widget interleaved with `Paragraph` text blocks. Must compute layout: text chunk → image → text chunk. Async image loading prevents UI freeze. |
| Full-text search across 20 years (FTS5) | Search is power. Finding every article mentioning "Référendum" across 2005–2026 is compelling. | MEDIUM | SQLite FTS5 virtual table. `CREATE VIRTUAL TABLE articles_fts USING fts5(title, body, content='articles')`. rusqlite supports this via raw SQL. French text — consider unicode tokenizer or simple tokenization. |
| Analytics dashboard with ratatui charts | 20 years of data is a treasure trove. Visualizing trends makes the archive come alive. | MEDIUM | Ratatui `BarChart` (articles per month), `Sparkline` (daily volume), `Chart` with `Axis` (trends over years), `Table` (top authors). Secondary view behind a tab or keybinding. |
| "On this day" historical view | Emotional hook: "What was La Presse writing on April 13 across all years?" High engagement, low complexity. | LOW | SQLite query: `SELECT * FROM articles WHERE month=? AND day=? AND year != ?`. Display as a special view or modal. |
| Section/category browsing | Newspapers have sections (International, Sports, Culture). Browsing by section is natural. | MEDIUM | Depends on scraping section metadata from archive HTML. Filter articles by section in SQLite. Could be tabs or a sidebar. |
| Author tracking | "Show me everything by this journalist" — deep engagement with archive content | LOW | Author field in SQLite. Author index. List view filtered by author. |
| Image gallery mode | Browse all images for a date/article in a grid. Showcases the image rendering feature. | MEDIUM | Grid layout of `StatefulImage` widgets. Ratatui `Layout` with `Constraint::Length` for cells. Navigate with hjkl. |
| Article export (markdown/text) | Save an article for reference. Practical utility. | LOW | Write article content + metadata to a file. Straightforward I/O. |
| Bookmarking/favorites | Mark articles for later reading. Basic but valued. | LOW | SQLite boolean or separate bookmarks table. Visual indicator. Dedicated view. |
| Term frequency trends over time | "How often did La Presse mention 'climat' over 20 years?" — analytics as storytelling. | MEDIUM | FTS5 can count matches. Aggregate by year/month. Display as `BarChart` or `Chart`. Requires careful query optimization for 20 years of data. |
| Async background sync with progress | Sync shouldn't block the UI. Show progress. | MEDIUM | Tokio channels. `mpsc` for sync progress updates. `Gauge` or `LineGauge` widget for progress bar. Throbber/spinner for indeterminate progress. |

### Anti-Features (Commonly Requested, Often Problematic)

Features that seem good but create problems.

| Feature | Why Requested | Why Problematic | Alternative |
|---------|---------------|-----------------|-------------|
| Live RSS feed / real-time updates | "Wouldn't it be cool to see new articles appear?" | This is an **archive browser**, not a live feed. Adding real-time changes the architecture (need polling, websockets, notification system). Scope creep that dilutes the core value. | Keep it archive-only. Users who want live news use newsboat or eilmeldung. |
| Complex query/filter language | newsboat has filter expressions; eilmeldung has a query language | Over-engineering for an archive. Users browse by date or search by keyword. A query language parser adds hundreds of lines of code for edge cases nobody hits in a newspaper archive. | Simple text search (FTS5 MATCH) + date range filters + section filters. Cover 95% of use cases. |
| User accounts / authentication | "Sync reading state across devices" | The project is a local tool (PROJECT.md: Out of Scope). Auth adds backend infrastructure, security concerns, and maintenance burden. | Local SQLite is the sync mechanism. Export/import bookmarks if needed. |
| Commenting / social features | "Discuss articles with other readers" | Requires backend, moderation, accounts. Completely changes the project scope. | Read-only experience. Open article in browser if user wants the web experience. |
| PDF/newspaper page rendering | "Show the actual newspaper layout" | Completely different rendering challenge. ratatui-image handles photos, not full-page newspaper PDFs at readable resolution. Terminal cells are too coarse for newspaper layout fidelity. | Focus on article-level content with inline images. That's the sweet spot for terminal rendering. |
| AI/LLM summarization | "Summarize 20 years of articles" | Adds heavy dependencies, API costs, latency, and hallucination risk. Not a reading tool problem. | Keep it a reading tool. Analytics dashboards provide human-computed insights. |
| Plugin/extension system | "Let users extend functionality" | Premature abstraction. Adds complexity without proven need. The project needs to validate its core first. | Build features directly. If patterns emerge, extract later. |
| Multi-newspaper support | "Add Le Devoir, Le Soleil..." | Each archive has different HTML structure, URL patterns, section taxonomy. Scraping is the hardest part — multiplying sources multiplies the hardest problem. | Focus on La Presse. The architecture can support multi-source later, but don't design for it now. |

## Feature Dependencies

```
[Calendar Navigation]
    └──requires──> [SQLite Cache + Schema]
                        └──requires──> [Background Scraping/Sync]

[Article List]
    └──requires──> [Calendar Navigation]
    └──requires──> [SQLite Cache + Schema]

[Article Reader (text)]
    └──requires──> [Article List]

[Inline Image Rendering]
    └──requires──> [Article Reader (text)]
    └──requires──> [Image Download + Cache]
                        └──requires──> [Background Scraping/Sync]

[Full-Text Search (FTS5)]
    └──requires──> [SQLite Cache + Schema]
    └──requires──> [Background Scraping/Sync]

[Analytics Dashboard]
    └──requires──> [SQLite Cache + Schema]
    └──requires──> [Calendar Navigation] (for date-range context)

["On This Day" View]
    └──requires──> [SQLite Cache + Schema]

[Image Gallery]
    └──requires──> [Inline Image Rendering]
    └──requires──> [Article Reader (text)]

[Section Browsing]
    └──requires──> [SQLite Cache + Schema]
    └──requires──> [Scraping with section metadata]

[Bookmarking]
    └──requires──> [SQLite Cache + Schema]

[Async Background Sync]
    └──requires──> [Background Scraping/Sync]
    └──requires──> [SQLite Cache + Schema]

[Term Frequency Trends]
    └──requires──> [Full-Text Search (FTS5)]
    └──requires──> [Analytics Dashboard]

[Article Export]
    └──requires──> [Article Reader (text)]
```

### Dependency Notes

- **SQLite Cache + Schema is the foundation:** Everything depends on having article data locally. Schema design must support FTS5, sections, authors, images, and read state from day one.
- **Background Scraping enables offline:** The sync system populates the cache. Without it, there's nothing to browse. Must run async and not block UI.
- **Image Rendering is the capstone:** It depends on both the article reader (for layout) and the image cache (for data). It's the last feature to integrate but the most visible.
- **Analytics is a late-phase addition:** Needs a populated database to be meaningful. Build after the core reader works.
- **FTS5 can be added incrementally:** The articles table exists from day one. FTS5 virtual table can be created and populated later without schema migration.

## MVP Definition

### Launch With (v1)

Minimum viable product — what's needed to validate the concept.

- [ ] **Calendar navigation** — Year/month/day browsing using ratatui Calendar widget. The primary entry point.
- [ ] **Article list** — Scrollable list of articles for selected date. Title, section, read state.
- [ ] **Article reader with text** — Full article body, scrollable, with proper French text wrapping.
- [ ] **Inline image rendering** — The headline feature. At least one image per article rendered inline via ratatui-image. Graceful degradation.
- [ ] **SQLite cache** — Local database with articles, images, metadata. Populated by scraping.
- [ ] **Basic scraping** — Fetch articles from lapresse.ca/archives for a date range. Store in SQLite.
- [ ] **Vim keybindings** — hjkl navigation, g/G, Ctrl-d/Ctrl-u, q to quit.
- [ ] **Help screen (?)** — Discoverable keybinding reference.
- [ ] **Status bar** — Current date, article count, image protocol detected.
- [ ] **Read/unread tracking** — Visual indicator, persisted in SQLite.

### Add After Validation (v1.x)

Features to add once core is working.

- [ ] **Full-text search (FTS5)** — ` / ` to search across all cached articles. Results as article list.
- [ ] **"On this day" view** — Trigger: core reading works, enough data cached.
- [ ] **Bookmarking** — `m` to mark, `M` to view marks. Trigger: users want to save articles.
- [ ] **Section browsing** — Filter by newspaper section. Trigger: section data available from scraping.
- [ ] **Async background sync with progress** — Non-blocking sync with Gauge progress. Trigger: initial sync is too slow/hangs UI.
- [ ] **Image gallery mode** — Grid view of all images for an article/date. Trigger: image rendering works well, users want more.
- [ ] **Author browsing** — Filter by journalist name. Trigger: author data extracted from scraping.

### Future Consideration (v2+)

Features to defer until product-market fit is established.

- [ ] **Analytics dashboard** — BarChart/Sparkline/Chart views of publication trends. Defer: needs significant data and UI design.
- [ ] **Term frequency trends** — "How often was X mentioned?" over time. Defer: needs FTS5 + analytics dashboard.
- [ ] **Article export** — Save as markdown/text file. Defer: low priority, easy to add.
- [ ] **Configuration file** — Customizable keybindings, colors, default date. Defer: sane defaults first.
- [ ] **Mouse support** — Click navigation, scroll. Defer: ratatui supports it but it's not table stakes for terminal power users.

## Feature Prioritization Matrix

| Feature | User Value | Implementation Cost | Priority |
|---------|------------|---------------------|----------|
| Calendar navigation | HIGH | MEDIUM | P1 |
| Article list | HIGH | LOW | P1 |
| Article reader (text) | HIGH | MEDIUM | P1 |
| Inline image rendering | HIGH | HIGH | P1 |
| SQLite cache + schema | HIGH | MEDIUM | P1 |
| Basic scraping | HIGH | HIGH | P1 |
| Vim keybindings | HIGH | LOW | P1 |
| Help screen | MEDIUM | LOW | P1 |
| Status bar | MEDIUM | LOW | P1 |
| Read/unread tracking | MEDIUM | LOW | P1 |
| Graceful image degradation | HIGH | MEDIUM | P1 |
| Full-text search (FTS5) | HIGH | MEDIUM | P2 |
| "On this day" view | HIGH | LOW | P2 |
| Bookmarking | MEDIUM | LOW | P2 |
| Async background sync | MEDIUM | MEDIUM | P2 |
| Section browsing | MEDIUM | MEDIUM | P2 |
| Author browsing | MEDIUM | LOW | P2 |
| Image gallery | MEDIUM | MEDIUM | P2 |
| Analytics dashboard | MEDIUM | HIGH | P3 |
| Term frequency trends | MEDIUM | MEDIUM | P3 |
| Article export | LOW | LOW | P3 |
| Configuration file | LOW | LOW | P3 |
| Mouse support | LOW | LOW | P3 |

**Priority key:**
- P1: Must have for launch
- P2: Should have, add when possible
- P3: Nice to have, future consideration

## Competitor Feature Analysis

| Feature | newsboat | eilmeldung | arxivlens | Our Approach |
|---------|----------|------------|-----------|--------------|
| Navigation model | Feed list → Article list → Content | Feed list → Article list → Content | Article list → Detail | **Calendar → Article list → Reader** (unique: date-driven, not feed-driven) |
| Image rendering | None | None | None | **Inline via ratatui-image** (unique differentiator) |
| Search | Filter expressions | Query language | Keyword search | **FTS5 full-text** (simpler than query language, more powerful than keyword) |
| Keybindings | vim-like | vim-like + customizable | vim-like | **vim-like** (standard for TUI) |
| Offline | Yes (cache) | Yes (sync) | No (live API) | **Yes (SQLite)** (mandatory per PROJECT.md) |
| Read tracking | Read/unread | Read/unread | None | **Read/unread** (standard) |
| Tagging/bookmarking | Star, flags | Tags, marks, flags | Pin authors | **Bookmarks** (simple, sufficient) |
| Help | `?` key | `?` key (searchable) | `?` key | **`?` key** (standard) |
| Sync | RSS fetch | RSS sync | API query | **HTML scraping** (harder but only option) |
| Analytics | None | None | None | **Dashboard with charts** (unique: 20-year dataset enables this) |
| Theming | Config colors | Full config | Dark/light | **Defer to v2** (sane defaults first) |

### Key Insight: Calendar-First = Differentiated

Every competitor uses a feed/list-first navigation model (subscribe to feeds, browse articles). This project uses **calendar-first** navigation because:
1. The data source (lapresse.ca/archives) is calendar-structured
2. Newspaper archives are inherently temporal — "what happened on this date?"
3. Calendar navigation maps to the newspaper metaphor (today's paper, yesterday's paper)
4. Ratatui has a built-in Calendar widget making this natural to implement

This is genuinely different from feed readers and creates a unique UX.

### Key Insight: Images = The Showcase Feature

No TUI news/media reader renders images inline. This is the project's entire reason to exist as a showcase. The implementation must be excellent:
- Correct protocol detection (Sixel/Kitty/iTerm2/half-block)
- Smooth rendering without flickering
- Meaningful fallback for unsupported terminals
- Async loading to prevent UI freeze

## Analytics Feature Deep-Dive

A 20-year newspaper archive (2005–2026) is a uniquely rich dataset. These analytics features leverage what makes this project special.

### Feasible with Ratatui Widgets

| Analytics Feature | Ratatui Widget | Data Source | Complexity |
|-------------------|---------------|-------------|------------|
| Articles per month/year | `BarChart` | `SELECT COUNT(*) FROM articles GROUP BY month` | LOW |
| Daily publication volume | `Sparkline` | `SELECT COUNT(*) FROM articles GROUP BY date` | LOW |
| Publication trend over years | `Chart` with `Axis` | Yearly counts as (x, y) data points | MEDIUM |
| Top authors by article count | `Table` | `SELECT author, COUNT(*) GROUP BY author ORDER BY count DESC` | LOW |
| Section distribution | `BarChart` | `SELECT section, COUNT(*) GROUP BY section` | LOW |
| Reading stats (how many read) | `Gauge` | `SELECT COUNT(read) / COUNT(*)` | LOW |
| Sync progress | `LineGauge` | Background sync state | LOW |
| Term search frequency by year | `Chart` or `BarChart` | FTS5 `bm25()` + GROUP BY year | MEDIUM |

### "On This Day" — High Value, Low Cost

The "on this day" feature deserves special attention because:
- **Emotional resonance:** Users connect with historical parallels
- **Implementation simplicity:** Single SQL query, display as article list
- **Engagement driver:** Encourages browsing deeper into the archive
- **Unique to archives:** Feed readers can't do this (they show current content only)

### Analytics Dashboard Layout Concept

```
┌─────────────────────────────────────────────────┐
│ 📊 La Presse Archive Analytics: 2005–2026       │
├──────────────────────┬──────────────────────────┤
│ Articles by Year     │ Top 10 Authors           │
│ ▇▇▇▇ 2012 (peak)    │ 1. Journalist A (1,243)  │
│ ▇▇▇  2019           │ 2. Journalist B (987)    │
│ ▇▇   2005           │ 3. Journalist C (856)    │
│                      │ ...                      │
├──────────────────────┴──────────────────────────┤
│ Monthly Volume (Sparkline: ▁▂▃▅▇▆▅▃▂▁▂▃)       │
├─────────────────────────────────────────────────┤
│ Sections: International ████████ Sports ██████  │
│          Culture    █████    Business ████      │
└─────────────────────────────────────────────────┘
```

## Sources

- **newsboat** (3.8k GitHub stars): RSS/Atom feed reader, reference TUI news reader — [github.com/newsboat/newsboat](https://github.com/newsboat/newsboat)
- **eilmeldung** (746 stars): Ratatui-based RSS reader, closest comparable TUI — [github.com/christo-auer/eilmeldung](https://github.com/christo-auer/eilmeldung)
- **arxivlens** (43 stars): Ratatui arXiv browser, similar article-list TUI — [github.com/AlMrvn/arxivlens](https://github.com/AlMrvn/arxivlens)
- **Ratatui widget documentation**: Built-in widgets (Calendar, BarChart, Chart, Sparkline, Table, etc.) — [ratatui.rs/concepts/widgets](https://ratatui.rs/concepts/widgets) (Context7 verified)
- **ratatui Calendar example**: calendar-explorer app in ratatui repo — [github.com/ratatui/ratatui/examples/apps/calendar-explorer](https://github.com/ratatui/ratatui/blob/main/examples/apps/calendar-explorer) (GitHub verified)
- **ratatui-image**: Image rendering widget with Sixel/Kitty/iTerm2/half-block protocols — [github.com/benjajaja/ratatui-image](https://github.com/benjajaja/ratatui-image) (Context7 verified)
- **rusqlite FTS5**: Full-text search via SQLite FTS5 virtual tables — [docs.rs/rusqlite](https://docs.rs/rusqlite/0.39.0/rusqlite/vtab/index.html) (Context7 verified)

---
*Feature research for: Rust TUI newspaper archive reader with image rendering*
*Researched: 2026-04-13*
