# Requirements: lpresse

**Defined:** 2026-04-13
**Core Value:** Reading La Presse articles with images, right in your terminal.

## v1 Requirements

Requirements for initial release. Each maps to roadmap phases.

### Navigation

- [ ] **NAV-01**: User can browse the archive via calendar-driven navigation (year/month/day selection)
- [ ] **NAV-02**: User can view a scrollable article list for any selected date showing title, section, and read state
- [ ] **NAV-03**: User can see a status bar showing current date, article count, and detected image protocol

### Reading

- [ ] **READ-01**: User can read full article text with proper French text wrapping and scrolling
- [ ] **READ-02**: User sees inline images rendered in the article reader via ratatui-image, with graceful degradation for unsupported terminals
- [ ] **READ-03**: User can navigate with vim keybindings (j/k/g/G/Ctrl-d/Ctrl-u for scroll, q to quit, ? for help)

### Data & Sync

- [ ] **DATA-01**: App scrapes article content and metadata from lapresse.ca/archives for specified date ranges
- [ ] **DATA-02**: App caches all scraped data locally in SQLite for offline browsing

### Analytics

- [ ] **ANLY-01**: User can view an analytics dashboard showing article volume by year and section breakdown via ratatui charts

## v2 Requirements

Deferred to future release. Tracked but not in current roadmap.

### Search

- **SRCH-01**: User can full-text search across all cached articles via SQLite FTS5

### Navigation Enhancements

- **NAV-04**: User can filter articles by newspaper section (International, Sports, etc.)
- **NAV-05**: User can browse "on this day" — articles published on the same date across all years

### Reading Enhancements

- **READ-04**: User can track read/unread state per article with visual indicators
- **READ-05**: User can bookmark articles for later reading
- **READ-06**: User can browse all images for an article in a grid gallery view
- **READ-07**: User can export articles as markdown or text files

### Data & Sync Enhancements

- **DATA-03**: App syncs in the background with a progress indicator, without blocking the UI

### Analytics Enhancements

- **ANLY-02**: User can view term frequency trends over time ("how often was X mentioned?")
- **ANLY-03**: User can see top authors by article count
- **ANLY-04**: User can see monthly publication volume sparklines

## Out of Scope

Explicitly excluded. Documented to prevent scope creep.

| Feature | Reason |
|---------|--------|
| Live RSS feed / real-time updates | Archive browser, not a live feed reader |
| Complex query/filter language | FTS5 + date filters cover 95% of use cases |
| User accounts / authentication | Local tool, no backend needed |
| Commenting / social features | Read-only experience |
| PDF/newspaper page rendering | Terminal cells too coarse for full-page layout |
| AI/LLM summarization | Reading tool, not an AI product |
| Plugin/extension system | Premature abstraction |
| Multi-newspaper support | Focus on La Presse; architecture can support it later |
| Mouse support | Terminal power users; defer to v2 |
| Configuration file | Sane defaults first |

## Traceability

Which phases cover which requirements. Updated during roadmap creation.

| Requirement | Phase | Status |
|-------------|-------|--------|
| NAV-01 | Phase 2 | Pending |
| NAV-02 | Phase 2 | Pending |
| NAV-03 | Phase 2 | Pending |
| READ-01 | Phase 4 | Pending |
| READ-02 | Phase 4 | Pending |
| READ-03 | Phase 4 | Pending |
| DATA-01 | Phase 3 | Pending |
| DATA-02 | Phase 1 | Pending |
| ANLY-01 | Phase 5 | Pending |

**Coverage:**
- v1 requirements: 9 total
- Mapped to phases: 9
- Unmapped: 0 ✓

---
*Requirements defined: 2026-04-13*
*Last updated: 2026-04-13 after roadmap creation*
