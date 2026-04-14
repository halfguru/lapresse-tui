# Roadmap: lpresse

## Overview

Build a terminal-based La Presse archive reader that renders articles with inline images. Start with the data layer and app skeleton, add navigation views, wire up the scraping pipeline to populate the database, deliver the image-capable reader as the capstone, and finish with analytics. Five phases deliver a coherent capability at each step.

## Phases

**Phase Numbering:**
- Integer phases (1, 2, 3): Planned milestone work
- Decimal phases (2.1, 2.2): Urgent insertions (marked with INSERTED)

Decimal phases appear between their surrounding integers in numeric order.

- [x] **Phase 1: Foundation & Data Layer** - App skeleton, SQLite schema, event loop, panic hooks, image protocol detection
- [x] **Phase 2: Core Navigation** - Calendar-driven browsing, article list, status bar, vim keybindings
- [ ] **Phase 3: Scraping Pipeline** - Rate-limited scraper for lapresse.ca/archives, sync state tracking, offline population
- [ ] **Phase 4: Article Reader with Images** - Full article text, inline images via ratatui-image, graceful degradation
- [ ] **Phase 5: Analytics Dashboard** - Article volume charts, section breakdown via ratatui widgets

## Phase Details

### Phase 1: Foundation & Data Layer
**Goal**: App boots into a terminal UI and has a working SQLite cache ready for data
**Depends on**: Nothing (first phase)
**Requirements**: DATA-02
**Success Criteria** (what must be TRUE):
  1. User can launch the app and see a terminal UI with an empty placeholder view
  2. App exits cleanly on q/Esc without leaving the terminal in a broken state (panic hook works)
  3. SQLite database is auto-created with schema for articles, images, and sync state on first launch
  4. Image protocol is detected at startup (Sixel/Kitty/half-block/none) and reported in the UI
**Plans**: TBD

### Phase 2: Core Navigation
**Goal**: Users can browse the 2005–2026 archive by date and see article lists
**Depends on**: Phase 1
**Requirements**: NAV-01, NAV-02, NAV-03
**Success Criteria** (what must be TRUE):
  1. User can navigate years and months via a calendar interface to select any date from 2005–2026
  2. User sees a scrollable list of articles for the selected date showing title, section, and read state
  3. A status bar displays the current selected date, article count for that date, and detected image protocol
  4. User can press ? to see a help overlay listing all available keybindings
**UI hint**: yes
**Plans**: TBD

### Phase 3: Scraping Pipeline
**Goal**: App can populate its local cache from La Presse's public archive
**Depends on**: Phase 1, Phase 2
**Requirements**: DATA-01
**Success Criteria** (what must be TRUE):
  1. User can trigger a sync that scrapes articles from lapresse.ca/archives for a specified date range
  2. Scraped articles and metadata are persisted to SQLite and immediately visible in the article list view
  3. Sync is rate-limited and can resume after interruption without re-fetching already-completed dates
  4. Previously synced articles are browsable without any network connection
**Plans**: TBD

### Phase 4: Article Reader with Images
**Goal**: Users can read full articles with inline images rendered directly in the terminal
**Depends on**: Phase 2, Phase 3
**Requirements**: READ-01, READ-02, READ-03
**Success Criteria** (what must be TRUE):
  1. User can open any cached article and read its full text with proper French Unicode wrapping
  2. Images within articles render inline using the detected terminal protocol (Sixel/Kitty/half-block)
  3. On terminals without image protocol support, images degrade gracefully (placeholder or text caption) without crashing
  4. User can scroll through articles using vim keybindings (j/k/g/G/Ctrl-d/Ctrl-u) and return to list with q
**UI hint**: yes
**Plans**: TBD

### Phase 5: Analytics Dashboard
**Goal**: Users can explore quantitative patterns in their cached article archive
**Depends on**: Phase 3, Phase 4
**Requirements**: ANLY-01
**Success Criteria** (what must be TRUE):
  1. User can switch to an analytics view showing article volume by year as a ratatui chart
  2. User can see a section breakdown (article count per newspaper section) in the dashboard
**UI hint**: yes
**Plans**: TBD

## Progress

**Execution Order:**
Phases execute in numeric order: 1 → 2 → 3 → 4 → 5

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 1. Foundation & Data Layer | direct | Complete | 2026-04-13 |
| 2. Core Navigation | direct | Complete | 2026-04-14 |
| 3. Scraping Pipeline | 0/? | Not started | - |
| 4. Article Reader with Images | 0/? | Not started | - |
| 5. Analytics Dashboard | 0/? | Not started | - |
