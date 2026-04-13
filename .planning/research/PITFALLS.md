# Pitfalls Research

**Domain:** Rust TUI newspaper archive reader with web scraping and terminal image rendering
**Researched:** 2026-04-13
**Confidence:** HIGH

## Critical Pitfalls

### Pitfall 1: No Panic Hook — Terminal Left in Raw Mode

**What goes wrong:**
When a ratatui app panics (or any unwrapped error bubbles up), the terminal is left in raw mode with the alternate screen active. The user sees a completely broken shell — no echo, no cursor, gibberish key responses. They must `reset` or close the terminal window. This is the #1 most reported frustration with TUI applications.

**Why it happens:**
Ratatui's `init()` sets raw mode and enters alternate screen. If a panic occurs before `restore()` runs (which is the normal cleanup path), the terminal state is never restored. `Terminal::drop` does NOT restore the terminal — this is by design (the Drop trait cannot return errors).

**How to avoid:**
Install a panic hook at the very start of `main()` that restores the terminal before delegating to the original panic handler. Every production ratatui app does this:

```rust
fn install_panic_hook() {
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let _ = crossterm::terminal::disable_raw_mode();
        let _ = crossterm::execute!(
            std::io::stdout(),
            crossterm::terminal::LeaveAlternateScreen,
            crossterm::cursor::Show
        );
        original_hook(panic_info);
    }));
}
```

Also use `color_eyre` for better panic/error reporting that works with the terminal restoration.

**Warning signs:**
- Testing reveals terminal corruption after a crash
- No panic hook installed in main.rs
- Using `.unwrap()` on fallible operations inside the TUI loop

**Phase to address:**
Phase 1 (scaffolding) — This must be in place before any interactive TUI work begins. It's a one-time setup.

---

### Pitfall 2: Blocking the Tokio Runtime from SQLite Operations

**What goes wrong:**
The TUI freezes, keystrokes are dropped, and the app feels unresponsive. The async event loop is stalled because rusqlite's synchronous SQLite operations are running on a tokio worker thread. With 20 years of articles (potentially 100k+ rows), even simple queries can take 10-100ms, which is enough to cause visible UI stutter.

**Why it happens:**
rusqlite is a synchronous, blocking C library. There is no async SQLite driver in the Rust ecosystem that supports the full feature set. Calling `Connection::query()` or `Connection::execute()` directly from an async context blocks the tokio worker thread, preventing other tasks (event handling, rendering) from progressing.

**How to avoid:**
Always wrap SQLite operations in `tokio::task::spawn_blocking()`:

```rust
let articles = tokio::task::spawn_blocking(move || {
    let conn = Connection::open(&db_path)?;
    conn.pragma_update(None, "journal_mode", "WAL")?;
    let mut stmt = conn.prepare("SELECT * FROM articles WHERE date = ?1")?;
    stmt.query_map(params![date], |row| { /* ... */ })?.collect()
}).await??;
```

For the connection management pattern, use either:
1. `Mutex<Connection>` inside `spawn_blocking` closures (simple, correct)
2. A dedicated `std::sync::mpsc` channel to a background DB thread (higher throughput for bulk operations)
3. `r2d2` connection pool with `spawn_blocking` (if concurrent reads are needed)

**Warning signs:**
- UI stutters when navigating between days
- Key presses feel "laggy" after initial data load
- `Connection::open` or query calls are directly in async functions

**Phase to address:**
Phase 1 (data layer) — The DB access pattern must be established before building features on top of it.

---

### Pitfall 3: Image Protocol Detection Before Terminal Events Are Ready

**What goes wrong:**
`Picker::from_query_stdio()` writes escape sequences to stdout and reads the response from stdin to detect terminal capabilities. If called at the wrong time (before entering alternate screen, or after the crossterm event reader is already consuming stdin), it either corrupts the terminal output or gets no response, falling back to half-blocks unnecessarily — or worse, deadlocking on stdin.

**Why it happens:**
The method temporarily takes over stdio. The ratatui-image docs explicitly warn: "This writes and reads from stdio momentarily. WARNING: this method should be called after entering alternate screen but before reading terminal events." The crossterm `EventStream` and `Picker::from_query_stdio()` compete for the same stdin file descriptor.

**How to avoid:**
Call sequence MUST be:
1. Enter alternate screen (`enable_raw_mode` + `EnterAlternateScreen`)
2. Create `Picker::from_query_stdio()` — detects protocol & font size
3. Start the crossterm event stream (or `Tui::start()`)
4. Enter main event loop

```rust
// CORRECT ORDER:
enable_raw_mode()?;
execute!(stdout(), EnterAlternateScreen)?;
let picker = Picker::from_query_stdio().unwrap_or_else(|_| Picker::halfblocks());
// NOW start event handling
let mut tui = Tui::new()?;
tui.enter()?;
```

Always provide a fallback: `Picker::from_query_stdio().unwrap_or_else(|_| Picker::halfblocks())`. The `halfblocks()` method requires no terminal support and always works (it uses Unicode half-block characters).

**Warning signs:**
- Images always render as half-blocks even in Kitty/Sixel terminals
- App hangs on startup waiting for stdin
- Garbage escape sequences appear briefly at startup

**Phase to address:**
Phase 1 (TUI scaffolding) — The picker initialization order is fundamental architecture.

---

### Pitfall 4: Ignoring ratatui-image's `last_encoding_result()`

**What goes wrong:**
Image encoding failures are silently swallowed. The image simply doesn't render, and there's no indication why. This is especially insidious because the failure can be intermittent — a resize that produces a zero-area rect, or a protocol that hits a size limit. The app appears to work in testing but breaks on edge cases (small terminal, unusual image dimensions).

**Why it happens:**
`StatefulImage::render()` is non-blocking — it triggers encoding in the background and returns immediately. The encoding result is stored in the protocol state, not returned from render. The ratatui-image README explicitly says "It is recommended to handle the encoding result" via `protocol.last_encoding_result()`. Most beginners don't read this far.

**How to avoid:**
After each `terminal.draw()` call, check encoding results for all visible images:

```rust
terminal.draw(|f| ui(f, &mut app))?;
// Check for encoding failures
if let Some(Err(e)) = app.image_protocol.last_encoding_result() {
    // Log the error, show fallback, etc.
    log::warn!("Image encoding failed: {}", e);
}
```

In practice, for a multi-image article reader, maintain a collection of active protocols and check each after render. Consider an `ImageManager` that handles lifecycle.

**Warning signs:**
- Some images render, others show blank space
- No error logging for image failures
- Never calling `last_encoding_result()` anywhere

**Phase to address:**
Phase 2 (article reader) — When image rendering is first implemented.

---

### Pitfall 5: Scraping Without Rate Limiting or Resume Capability

**What goes wrong:**
Scraping 20 years of daily articles (~7,300 days × ~10-50 articles = 73k-365k page fetches) either: (a) gets IP-banned by the server, (b) takes days and crashes halfway through with no way to resume, or (c) overwhelms the user's network. This is the single most underestimated aspect of this project.

**Why it happens:**
Developers test with a single day's articles, see it works in 2 seconds, and assume scaling to 20 years is trivial. But 100k+ sequential HTTP requests at even modest speed will trigger rate limiting on most news sites. La Presse doesn't have a documented API, so there are no rate limit headers to guide you. A crash on day 3,472 means restarting from day 1 if there's no resume mechanism.

**How to avoid:**

1. **Store progress per-day, not per-scrape-run:**
   ```sql
   CREATE TABLE sync_state (
       date TEXT PRIMARY KEY,
       status TEXT DEFAULT 'pending',  -- pending, in_progress, complete, failed
       fetched_at TEXT,
       error_message TEXT
   );
   ```
   Before scraping a day, check if it's already `complete`. On startup, query `status = 'in_progress'` for crash recovery.

2. **Implement polite rate limiting:**
   ```rust
   // Minimum 500ms between requests, configurable
   tokio::time::sleep(Duration::from_millis(500)).await;
   ```
   Use an adaptive backoff: if you get HTTP 429 or 503, increase delay exponentially.

3. **Scrape incrementally, not all-at-once:**
   Start by fetching only the day-listing pages (which list article URLs). Store URLs. Then fetch article content in a second pass. This separates "discovery" from "download" and lets you resume either independently.

4. **Background scraping with progress reporting:**
   Run scraping in a spawned tokio task, send progress via a channel to the UI. The user should see "Syncing: 2005-03-14 (1,247/7,300 days)".

**Warning signs:**
- No `sync_state` table in schema
- HTTP 429/503 responses in logs
- No way to resume a partial scrape
- All scraping in a single blocking function

**Phase to address:**
Phase 1 (data layer/scraping) — The resume-capable scraping architecture must be designed before the first fetch.

---

### Pitfall 6: SQLite Without WAL Mode and Bulk Insert Transactions

**What goes wrong:**
Inserting 100k articles takes 30+ minutes instead of 30 seconds. Concurrent reads during scraping are blocked. The database file gets corrupted on crash during initial sync. These are all caused by SQLite's default journal mode (DELETE) and autocommit behavior.

**Why it happens:**
SQLite's default settings are optimized for compatibility, not performance. In DELETE journal mode, every write creates and deletes a rollback journal file. Without explicit transactions, each INSERT is its own transaction — meaning fsync() after every single row. At ~100k rows, that's 100k fsyncs.

**How to avoid:**

1. **Enable WAL mode immediately after opening:**
   ```rust
   conn.pragma_update(None, "journal_mode", "WAL")?;
   conn.pragma_update(None, "synchronous", "NORMAL")?; // Not FULL — NORMAL is safe with WAL
   conn.pragma_update(None, "cache_size", "-64000")?;   // 64MB cache
   conn.pragma_update(None, "foreign_keys", "ON")?;
   ```

2. **Use transactions for bulk inserts:**
   ```rust
   let tx = conn.transaction()?;
   for article in articles {
       tx.execute("INSERT INTO articles ...", params![...])?;
   }
   tx.commit()?;
   ```
   This turns 10,000 fsyncs into 1. Orders of magnitude faster.

3. **Create indexes AFTER bulk insert, not before:**
   ```sql
   -- Bad: index updated on every INSERT
   CREATE INDEX idx_articles_date ON articles(date);

   -- Good: create after all data is loaded
   BEGIN;
   INSERT INTO articles ...; -- bulk insert
   COMMIT;
   CREATE INDEX idx_articles_date ON articles(date);
   ```

4. **Use prepared statements:**
   ```rust
   let mut stmt = tx.prepare("INSERT INTO articles (...) VALUES (?1, ?2, ?3)")?;
   for article in articles {
       stmt.execute(params![article.title, article.date, article.body])?;
   }
   ```

**Warning signs:**
- Initial sync takes more than a few minutes for a year's data
- No WAL pragma in connection setup
- INSERTs outside transactions
- Indexes created before data loading

**Phase to address:**
Phase 1 (data layer) — WAL mode and transaction patterns are foundational.

---

### Pitfall 7: Sixel Images on the Last Terminal Line Cause Screen Corruption

**What goes wrong:**
When a Sixel image is rendered on the last line of the terminal, it triggers a scroll, which pushes all previously rendered content up by one line. The entire UI layout becomes misaligned. This is a documented, open bug in ratatui-image (issue #57).

**Why it happens:**
The Sixel protocol works by printing escape sequences that embed pixel data. When the sequence reaches the bottom of the terminal scrollback, the terminal auto-scrolls. This is fundamental to how terminals work — there's no workaround from the library side.

**How to avoid:**
1. Ensure image rendering areas never touch the last row of the terminal. Add a 1-row bottom margin to any layout containing images.
2. If using a `Paragraph` + `StatefulImage` mix in a scrolling article view, track the render rect and ensure it's at least 1 row above the terminal bottom.
3. Consider using Kitty protocol when available (it uses Unicode placeholders and doesn't have this issue).
4. Test explicitly with Sixel terminals (xterm, foot) — this bug doesn't manifest with Kitty or iTerm2.

**Warning signs:**
- UI layout shifts upward after image renders
- Only happens in xterm/foot (Sixel terminals), not in Kitty
- Screen "jumps" when scrolling to an image near the bottom

**Phase to address:**
Phase 2 (article reader) — When image layout is first designed, add bottom margin to image areas.

---

### Pitfall 8: Terminal Resize Breaks Image State

**What goes wrong:**
When the user resizes the terminal window, images either disappear, render at the wrong size, or leave ghost artifacts. The `StatefulProtocol` caches encoding for a specific size. A resize invalidates this cache, but the old encoding data lingers.

**Why it happens:**
`StatefulImage` encodes image data at a specific pixel size derived from the character cell area × font size. On resize, the render area changes but the protocol state still holds the old encoding. The widget detects the size mismatch and re-encodes, but this is asynchronous — there can be frames where the old image data is rendered at the new size, causing visual glitches.

**How to avoid:**
1. Listen for `Event::Resize` from crossterm and trigger a re-render.
2. After resize, call `last_encoding_result()` to catch re-encoding errors.
3. For the article reader specifically, consider using `Image` (stateless) instead of `StatefulImage` for images that are small enough — stateless widgets always re-encode and can't have stale state.
4. The async example in ratatui-image shows the proper pattern for offloading encoding to avoid blocking the render loop.

**Warning signs:**
- Images disappear after resize
- Ghost pixels remain where an image used to be
- Only tested at one terminal size

**Phase to address:**
Phase 2 (article reader) — Test resize behavior as part of image rendering acceptance criteria.

---

## Technical Debt Patterns

| Shortcut | Immediate Benefit | Long-term Cost | When Acceptable |
|----------|-------------------|----------------|-----------------|
| Storing raw HTML instead of parsed text in SQLite | Faster initial scraping (skip parsing) | Must re-parse on every read; can't search article text | Never — parse once at scrape time |
| Single monolithic `App` struct holding all state | Quick to prototype | Hard to test, hard to reason about, state mutation bugs | Phase 1 prototype only; refactor before Phase 2 |
| No image caching to disk (keep in memory only) | Simpler code | Memory explodes with hundreds of images; re-decodes on every navigation | Never — cache decoded images with LRU |
| Skipping schema migrations | Avoid migration framework complexity | Can't evolve the database; users lose data on updates | Phase 1 only (before any users) |
| Scraping article content inline during navigation | Simpler architecture | Each navigation triggers HTTP requests; slow and fragile | Never — always scrape to DB first, browse from DB |

## Integration Gotchas

| Integration | Common Mistake | Correct Approach |
|-------------|----------------|------------------|
| `Picker::from_query_stdio()` | Calling before alternate screen is entered | Call AFTER `EnterAlternateScreen` but BEFORE starting event stream |
| `ratatui-image` + crossterm | Using `termion` backend for ratatui-image but crossterm for ratatui | Use same backend for both — crossterm for this project |
| rusqlite + tokio | Calling SQLite operations directly in async context | Always wrap in `spawn_blocking()` |
| crossterm events + Picker | Starting event reader before Picker queries stdio | Picker first, then event stream |
| reqwest + scraping | No User-Agent header, or obvious bot User-Agent | Use a descriptive User-Agent like `lpresse/0.1 (terminal archive reader)` |
| La Presse HTML | Assuming stable CSS class names across 20 years of articles | Build resilient selectors, test against old archive pages, handle missing elements gracefully |

## Performance Traps

| Trap | Symptoms | Prevention | When It Breaks |
|------|----------|------------|----------------|
| Loading all articles for a year into memory | UI freezes on month navigation, RSS usage climbs | Paginate queries; load day-by-day | Navigating to any month |
| Decoding images on the main render thread | Frame drops when scrolling through images with thumbnails | Use `spawn_blocking` for image decode + resize; show placeholder while loading | More than 2-3 images in view |
| SQLite without indexes on date columns | Day lookups take 100ms+ instead of <1ms | `CREATE INDEX idx_articles_date ON articles(date)` | Above ~10k rows |
| No connection pooling / single connection shared across tasks | Contention between scraping writes and UI reads | Use WAL mode (allows concurrent reads during writes); consider separate read connection | As soon as background scraping + UI browsing overlap |
| Rendering every frame at 60fps with images | CPU pinned at 100%, fan spins up | Only re-render on events or explicit `Render` events; use `frame_rate` throttle | As soon as images are on screen |

## Security Mistakes

| Mistake | Risk | Prevention |
|---------|------|------------|
| No robots.txt check before scraping | IP ban, legal risk, ethical concern | Check and respect `robots.txt` for lapresse.ca/archives; add configurable crawl delay |
| Trusting scraped HTML content blindly | XSS if content is ever rendered in HTML context; SQL injection if building queries from scraped data | Parameterized queries always; sanitize HTML on parse; this is a local tool so risk is lower but good practice matters |
| Storing cookies/session data | Unnecessary attack surface; La Presse archive is public | Don't accept or store cookies; scrape only public archive pages |
| No TLS certificate validation | MitM could inject malicious content into local DB | Use reqwest's default TLS validation; don't disable `CertificateValidation` |

## UX Pitfalls

| Pitfall | User Impact | Better Approach |
|---------|-------------|-----------------|
| No loading/progress indicator during scraping | User thinks app is frozen; kills it; corrupts sync state | Show a status bar: "Syncing 2005-03-14 (1247/7300)" with progress percentage |
| Halfblock fallback looks terrible and no explanation | User thinks images are broken; files a bug | Show a small info line: "Images: half-block mode (use Kitty/Sixel terminal for full images)" |
| No keyboard hints | User can't figure out how to navigate; quits | Show keybindings in a help overlay (press `?`), key hints in footer |
| Blocking UI during article load | App feels broken when navigating to a new day | Load from SQLite (fast) but show placeholder during initial sync; background scrape is always async |
| Calendar navigation without date context | User gets lost in 20 years of dates | Show current month/year prominently; highlight days with articles vs empty days; show article count per day |

## "Looks Done But Isn't" Checklist

- [ ] **Terminal restoration:** Panic hook installed and tested (force a panic to verify) — not just `ratatui::restore()` on happy path
- [ ] **Image fallback:** Works in a terminal WITHOUT any image protocol support (e.g., basic Linux console, SSH without X forwarding) — test with `TERM=dumb`
- [ ] **Scraping resume:** Kill the app mid-sync, restart, verify it continues from where it left off
- [ ] **Resize handling:** Resize terminal while viewing an article with images — no crashes, no corruption, images re-render
- [ ] **Large dataset performance:** Navigate to a day with 50+ articles — UI must remain responsive (< 16ms frame time)
- [ ] **Offline mode:** Disconnect network after initial sync — all cached content must be browsable
- [ ] **Unicode/French content:** Articles contain accented characters (é, è, ê, ç, à) — verify no mojibake in TUI rendering
- [ ] **Database schema migration:** After shipping v0.1, adding a column in v0.2 must not lose existing data
- [ ] **Sixel last-line bug:** Test with Sixel terminal (foot/xterm) — image near bottom of screen doesn't cause scroll corruption

## Recovery Strategies

| Pitfall | Recovery Cost | Recovery Steps |
|---------|---------------|----------------|
| No panic hook (broken terminal) | LOW | Add panic hook in next commit; doesn't require data migration |
| Blocking tokio runtime | MEDIUM | Wrap all DB calls in `spawn_blocking`; refactor data access layer |
| No scrape resume capability | HIGH | Must re-scrape everything; add sync_state table and restart | Consider the DB a fresh start if partial data is corrupt |
| No WAL mode on existing DB | LOW | Run `PRAGMA journal_mode=WAL` on next connection open; SQLite handles migration |
| Wrong Picker init order | LOW | Reorder initialization sequence; no data impact |
| Ignored encoding errors | LOW | Add `last_encoding_result()` checks; add logging; no schema change needed |
| Missing SQLite indexes | LOW | Add `CREATE INDEX IF NOT EXISTS` statements; run on app startup; SQLite builds index online |

## Pitfall-to-Phase Mapping

| Pitfall | Prevention Phase | Verification |
|---------|------------------|--------------|
| Panic hook / terminal restore | Phase 1 (scaffolding) | Force a panic (`panic!("test")`) and verify terminal is usable |
| Blocking tokio with SQLite | Phase 1 (data layer) | Navigate UI during a large bulk insert — no frame drops |
| Picker init order | Phase 1 (scaffolding) | Test in Kitty, xterm, and a dumb terminal |
| Scrape rate limiting + resume | Phase 1 (scraping) | Kill app mid-scrape, restart, verify resume |
| WAL mode + transactions | Phase 1 (data layer) | Time bulk insert of 10k rows — should be < 5s |
| Sixel last-line bug | Phase 2 (article reader) | Test in foot terminal with image at bottom of viewport |
| Image encoding error handling | Phase 2 (article reader) | Load a corrupted/truncated image — graceful fallback |
| Terminal resize + images | Phase 2 (article reader) | Resize window while viewing article with 3+ images |
| French Unicode rendering | Phase 2 (article reader) | Load articles with heavy accented content — verify display |
| Loading indicators | Phase 2 (article reader) | Verify status bar during initial sync of 20 years |

## Sources

- Ratatui official docs: panic hooks recipe — https://ratatui.rs/recipes/apps/panic-hooks/
- Ratatui async app tutorial — https://ratatui.rs/tutorials/counter-async-app/full-async-events
- ratatui-image README (compatibility matrix, known issues, encoding result) — https://github.com/benjajaja/ratatui-image/blob/master/README.md
- ratatui-image issue #57 (Sixel last-line scroll bug) — https://github.com/benjajaja/ratatui-image/issues/57
- ratatui-image picker source (from_query_stdio warnings) — https://github.com/ratatui/ratatui-image/blob/master/src/picker.rs
- Tokio docs on `spawn_blocking` — https://docs.rs/tokio/latest/tokio/task/fn.spawn_blocking.html
- rusqlite docs (pragma operations, transactions) — https://docs.rs/rusqlite/0.39.0/rusqlite/
- Real-world patterns: Anki's SQLite setup (WAL, page_size, cache_size) — https://github.com/ankitects/anki/blob/main/rslib/src/storage/sqlite.rs
- Real-world patterns: NX's SQLite init (WAL fallback, busy_handler) — https://github.com/nrwl/nx/blob/master/packages/nx/src/native/db/initialize.rs
- Real-world patterns: ProteinView's Picker fallback — https://github.com/001TMF/ProteinView/blob/master/src/main.rs
- Real-world patterns: stu's image picker with disabled/default/error states — https://github.com/lusingander/stu/blob/master/src/environment.rs
- Scraper crate docs — https://docs.rs/scraper/0.23.1/scraper/

---
*Pitfalls research for: Rust TUI newspaper archive reader with image rendering*
*Researched: 2026-04-13*
