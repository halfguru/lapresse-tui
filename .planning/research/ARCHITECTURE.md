# Architecture Research

**Domain:** Rust TUI newspaper archive reader (web scraper + local cache + image rendering)
**Researched:** 2026-04-13
**Confidence:** HIGH

## Recommended Architecture

### System Overview

```
┌──────────────────────────────────────────────────────────────────────┐
│                         Presentation Layer                           │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐               │
│  │   Calendar    │  │  ArticleList  │  │   Reader      │               │
│  │   Component   │  │  Component    │  │   Component   │               │
│  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘               │
│         │                  │                  │                       │
│  ┌──────────────┐                              ┌──────────────┐      │
│  │  Analytics    │                              │  Image        │      │
│  │  Dashboard    │                              │  Renderer     │      │
│  └──────┬───────┘                              └──────┬───────┘      │
│         │                  │                  │                       │
├─────────┴──────────────────┴──────────────────┴───────────────────────┤
│                        Application Core                              │
│  ┌───────────────────────────────────────────────────────────────┐   │
│  │   App State (TEA Model)                                       │   │
│  │   • current_view (enum: Calendar | List | Reader | Analytics) │   │
│  │   • navigation stack                                          │   │
│  │   • selected date / article                                   │   │
│  │   • sync status                                               │   │
│  └───────────────────────────┬───────────────────────────────────┘   │
│                              │                                       │
│  ┌───────────────────────────────────────────────────────────────┐   │
│  │   Message Bus (tokio::sync::mpsc)                             │   │
│  │   • Key events → Actions                                      │   │
│  │   • Background task results → State updates                   │   │
│  │   • Sync progress → UI notifications                          │   │
│  └───────────────────────────┬───────────────────────────────────┘   │
│                              │                                       │
├──────────────────────────────┴───────────────────────────────────────┤
│                        Service Layer                                 │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐               │
│  │   Scraper     │  │   Storage     │  │   Image       │              │
│  │   Service     │  │   Service     │  │   Service     │              │
│  │              │  │              │  │              │               │
│  │ • reqwest    │  │ • rusqlite   │  │ • image crate│               │
│  │ • scraper    │  │ • migrations │  │ • Picker     │               │
│  │ • tokio spawn│  │ • thread pool│  │ • ThreadProto│               │
│  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘               │
│         │                  │                  │                       │
├─────────┴──────────────────┴──────────────────┴───────────────────────┤
│                        Data Layer                                    │
│  ┌──────────────────┐  ┌──────────────────┐                          │
│  │   SQLite Cache    │  │   Image Cache     │                         │
│  │   (articles.db)   │  │   (filesystem)    │                         │
│  │                  │  │                  │                          │
│  │ • articles       │  │   ~/.cache/       │                          │
│  │ • images (refs)  │  │   lpresse/img/    │                          │
│  │ • sync state     │  │                  │                          │
│  │ • analytics      │  │   (JPEG/PNG blobs)│                          │
│  └──────────────────┘  └──────────────────┘                          │
└──────────────────────────────────────────────────────────────────────┘
```

### Component Responsibilities

| Component | Responsibility | Implementation |
|-----------|----------------|----------------|
| **Calendar Component** | Year/month/day picker, date-driven navigation | ratatui `Table`/custom widget, delegates to Storage Service for date queries |
| **Article List Component** | Displays articles for a selected day, scrollable list | ratatui `List` widget with custom items showing title, section, read status |
| **Reader Component** | Full article rendering with inline images and styled text | Custom ratatui widget composing `Paragraph` (text) + `StatefulImage` (images) |
| **Image Renderer** | Terminal image rendering with protocol detection and fallback | ratatui-image `Picker` + `StatefulImage`, wraps `ThreadProtocol` for async encoding |
| **Analytics Dashboard** | Reading stats, article frequency charts, section breakdown | ratatui `Chart`, `BarChart`, `Table` widgets querying SQLite aggregates |
| **App State** | Central model — current view, navigation stack, selection, sync status | Rust struct following TEA (The Elm Architecture) pattern |
| **Message Bus** | Decouples async background work from synchronous UI rendering | `tokio::sync::mpsc` unbounded channel carrying `AppMessage` enum |
| **Scraper Service** | HTTP fetching, HTML parsing, article extraction, rate limiting | `reqwest` (async HTTP) + `scraper` (CSS selectors), spawned on tokio runtime |
| **Storage Service** | SQLite CRUD, schema migrations, query interface | `rusqlite` with `rusqlite_migration`, runs on `std::thread` (blocking) |
| **Image Service** | Download images, decode, create protocol state for rendering | `reqwest` + `image` crate + ratatui-image `Picker`/`ThreadProtocol` |

## Recommended Project Structure

```
src/
├── main.rs                  # Entry point: tokio::main, CLI args, app bootstrap
├── app.rs                   # App struct (TEA Model), run loop, message dispatch
├── action.rs                # Action enum (all possible state transitions)
├── event.rs                 # Event handling: crossterm events → Actions
│
├── ui/                      # Presentation layer (views/components)
│   ├── mod.rs               # View dispatcher (routes to active view)
│   ├── calendar.rs          # Calendar navigation component
│   ├── article_list.rs      # Article list for selected day
│   ├── reader.rs            # Full article reader with inline images
│   ├── analytics.rs         # Stats dashboard
│   ├── status_bar.rs        # Bottom status bar (sync state, key hints)
│   └── theme.rs             # Colors, styles, borders — central theme
│
├── scraper/                 # Web scraping layer
│   ├── mod.rs               # ScraperService public API
│   ├── archive.rs           # Archive page parser (calendar → article URLs)
│   ├── article.rs           # Article page parser (title, body, images, metadata)
│   └── client.rs            # reqwest client builder, rate limiting, retry logic
│
├── storage/                 # Data persistence layer
│   ├── mod.rs               # StorageService public API
│   ├── db.rs                # SQLite connection management, migrations
│   ├── models.rs            # Data types: Article, ImageRef, SyncState, DayIndex
│   └── queries.rs           # All SQL queries, typed result mappings
│
├── image/                   # Image handling layer
│   ├── mod.rs               # ImageService public API
│   ├── cache.rs             # Filesystem image cache (download → disk → decode)
│   └── protocol.rs          # ratatui-image Picker setup, ThreadProtocol wrapper
│
└── config.rs                # User configuration (paths, keybindings, etc.)
```

### Structure Rationale

- **`ui/`**: Each view is a separate file. The `mod.rs` dispatches to the active view based on `App.current_view`. Views are pure rendering — they take `&App` state and draw.
- **`scraper/`**: Isolated from everything else. Only produces domain models (`Article`, etc.) that go into storage. Never talks to UI directly.
- **`storage/`**: Central source of truth. Scraper writes to it, UI reads from it. rusqlite is synchronous — calls happen on a dedicated thread or via `tokio::task::spawn_blocking`.
- **`image/`**: Bridges storage (image blobs on disk) and UI (ratatui-image protocol state). Owns the `Picker` singleton and manages `ThreadProtocol` for async encoding.
- **`app.rs`**: The heart. Owns the TEA loop: `event → action → update state → render`. Does NOT contain business logic — delegates to services.

## Architectural Patterns

### Pattern 1: The Elm Architecture (TEA) with Async Extensions

**What:** The main loop follows Model → Update → View cycle. Events become typed `Action` enums. The update function produces new state. The view is a pure function of state.

**When to use:** This is the core application pattern for the entire app. Ratatui's official docs recommend TEA as a primary pattern.

**Why for this project:** TEA gives predictable state management across multiple views (Calendar, List, Reader, Analytics). The `Action` enum makes it trivial to add new interactions without touching unrelated code. Combined with an async message channel, background scraper tasks can feed results back into the update loop cleanly.

**Trade-offs:**
- Pro: Centralized state makes debugging and testing straightforward
- Pro: Easy to add new views — just add a variant to `View` enum and a render function
- Con: The `Action` enum can grow large; mitigate by grouping related actions
- Con: State must be mutable in the update function — this is idiomatic Rust TEA, not a real problem

**Example:**
```rust
// action.rs
#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    // Navigation
    SelectDate(chrono::NaiveDate),
    OpenArticle(ArticleId),
    GoBack,
    ShowAnalytics,
    Quit,

    // Sync
    SyncStart(chrono::NaiveDate),
    SyncComplete(Vec<Article>),
    SyncError(String),

    // Rendering
    Render,
    Tick,
    Resize(u16, u16),
}

// app.rs
pub struct App {
    pub current_view: View,
    pub nav_stack: Vec<View>,
    pub selected_date: Option<chrono::NaiveDate>,
    pub articles: Vec<Article>,
    pub reader_state: ReaderState,
    pub sync_status: SyncStatus,
    pub should_quit: bool,
    // Services (injected)
    pub action_tx: mpsc::UnboundedSender<Action>,
}

pub enum View {
    Calendar,
    ArticleList,
    Reader(ArticleId),
    Analytics,
}
```

### Pattern 2: Channel-Based Background Workers

**What:** Long-running tasks (scraping, image processing) run on tokio tasks and communicate results back through the `mpsc` channel as `Action` variants. The main loop receives these alongside UI events.

**When to use:** Any I/O that would block the render loop (network requests, heavy image encoding, database writes).

**Why for this project:** Scraping La Presse's archive pages takes 100ms–several seconds per page. Image decoding and Sixel/Kitty encoding is CPU-intensive. Both must happen off the main thread, and both need to update the UI when done.

**Trade-offs:**
- Pro: UI stays responsive at 30–60 FPS regardless of background work
- Pro: Scraper can be canceled by dropping the channel sender
- Con: Need to handle stale results (user navigated away before scrape finished)
- Con: Slightly more complex than synchronous calls

**Example:**
```rust
// In the main run loop:
loop {
    tokio::select! {
        // UI events (crossterm)
        event = tui.next() => {
            if let Some(action) = handle_event(event) {
                action_tx.send(action)?;
            }
        }
        // Background results
        action = action_rx.recv() => {
            if let Some(action) = action {
                update(&mut app, action);
            }
        }
    }
    if app.should_quit { break; }
    tui.draw(|f| view(&app, f))?;
}

// Spawning a scrape task:
fn scrape_day(date: NaiveDate, tx: &UnboundedSender<Action>) {
    let tx = tx.clone();
    tokio::spawn(async move {
        match scraper.fetch_day(date).await {
            Ok(articles) => tx.send(Action::SyncComplete(articles)).ok(),
            Err(e) => tx.send(Action::SyncError(e.to_string())).ok(),
        };
    });
}
```

### Pattern 3: ThreadProtocol for Non-Blocking Image Encoding

**What:** ratatui-image provides `ThreadProtocol` — a built-in mechanism to offload image resize+encoding to a background thread. The main thread sends `ResizeRequest`, the thread sends back `ResizeResponse`, and the widget updates on the next render.

**When to use:** Every image displayed in the reader view. Sixel/Kitty encoding for a 500×300 image can take 10–50ms — that's a visible frame drop if done synchronously.

**Why for this project:** Articles contain multiple inline images. The reader view must scroll smoothly. ThreadProtocol is the officially recommended approach by ratatui-image for async apps.

**Trade-offs:**
- Pro: Image encoding never blocks the render loop
- Pro: Built-in resize handling — images automatically adapt to terminal width changes
- Con: Must manage the channel lifecycle and handle encoding errors gracefully
- Con: First render of an image may show blank until encoding completes (design: show placeholder)

**Example:**
```rust
use ratatui_image::thread::{ThreadProtocol, ResizeRequest};
use ratatui_image::{picker::Picker, StatefulImage};

struct ImageManager {
    picker: Picker,
    // Map from image URL/index to ThreadProtocol state
    protocols: HashMap<usize, ThreadProtocol>,
}

impl ImageManager {
    fn new() -> Self {
        // Detect terminal protocol (Sixel, Kitty, iTerm2, halfblocks)
        let picker = Picker::from_query_stdio()
            .unwrap_or_else(|_| Picker::halfblocks());
        Self { picker, protocols: HashMap::new() }
    }

    fn load_image(&mut self, idx: usize, data: Vec<u8>, rect: Rect) {
        let img = image::load_from_memory(&data).ok();
        if let Some(img) = img {
            let protocol = self.picker.new_resize_protocol(img);
            // ThreadProtocol wraps StatefulProtocol with background encoding
            let thread_proto = ThreadProtocol::new(protocol, rect.as_size());
            self.protocols.insert(idx, thread_proto);
        }
    }
}
```

## Data Flow

### Primary Flow: Browse and Read

```
User presses arrow keys in Calendar
    ↓
event.rs maps key → Action::SelectDate(date)
    ↓
app.rs update() sets app.selected_date, switches to ArticleList view
    ↓
app.rs dispatches: storage::fetch_articles(date) (sync, fast SQLite read)
    ↓
app.rs update() sets app.articles = result
    ↓
ui/article_list.rs renders the list
    ↓
User presses Enter on an article
    ↓
event.rs maps Enter → Action::OpenArticle(id)
    ↓
app.rs update() checks if article is cached
    ├─ YES: Switch to Reader view with cached content
    └─ NO:  Dispatch Action::SyncStart(id), show loading state
              ↓
         scraper::fetch_article(id)  ← async tokio::spawn
              ↓
         storage::save_article(article)  ← spawn_blocking
              ↓
         Action::SyncComplete(article) → sent through channel
              ↓
         app.rs update() switches to Reader view
```

### Background Sync Flow

```
User navigates to a day that hasn't been synced
    ↓
app.rs detects cache miss → spawns scraper task
    ↓
scraper/client.rs GET https://lapresse.ca/archives/YYYY/MM/DD
    ↓
scraper/archive.rs parses HTML → extracts article URLs
    ↓
For each article URL (rate-limited, e.g., 2 concurrent):
    scraper/client.rs GET article URL
        ↓
    scraper/article.rs parses HTML → extracts:
        • title, author, date, section
        • body paragraphs (with image positions)
        • image URLs
        ↓
    storage::save_article(article)
    image::cache::download_images(image_urls)
        ↓
    When all done: Action::SyncComplete(articles)
```

### State Management

```
┌──────────────┐
│  App (Model)  │  ← Single source of truth
│              │
│  current_view│──→ ui/mod.rs dispatches to active view
│  nav_stack   │──→ for GoBack support
│  selected_   │──→ drives calendar highlight + storage queries
│  articles    │──→ displayed in ArticleList
│  reader_state│──→ scroll position, image protocols for Reader
│  sync_status │──→ status bar shows sync progress
└──────────────┘
       ↑
       │  Actions (via mpsc channel)
       │
┌──────┴──────┐
│  Event Src   │  crossterm key/mouse events
│  Background  │  scraper results, image encoding results
│  Tick        │  periodic timer (for animations, sync polling)
└──────────────┘
```

### Key Data Flows

1. **Calendar → Storage:** User selects a date → App queries Storage for cached articles for that date → if cache miss, triggers Scraper → Scraper writes to Storage → App re-queries Storage.

2. **Article List → Reader:** User selects an article → App queries Storage for full article content (body + image refs) → Image Service loads/decodes images → Reader view renders text + images.

3. **Scraper → Storage → UI:** Scraper fetches HTML → parses into domain models → saves to SQLite → sends Action to App → App re-queries Storage and updates view.

4. **Image lifecycle:** Image URL in article → Scraper downloads to filesystem cache → Image Service decodes → Picker creates ThreadProtocol → Reader renders via StatefulImage widget.

## Anti-Patterns to Avoid

### Anti-Pattern 1: Synchronous Network Calls in the Render Loop

**What people do:** Call `reqwest::blocking` or `.await` on network requests inside the event handler or update function.
**Why it's wrong:** Blocks the entire TUI. Terminal freezes, no input processed, looks crashed.
**Do this instead:** Always `tokio::spawn` network work. Send results back via the `mpsc` channel. The update function only handles the result, never initiates the wait.

### Anti-Pattern 2: Giant App Struct with All State Flattened

**What people do:** Put every field (calendar state, reader scroll, analytics data) flat on the `App` struct.
**Why it's wrong:** Unmanageable as the app grows. Fields for inactive views sit unused. State for different views bleeds together.
**Do this instead:** Use per-view state structs. The `App` holds `Option<ReaderState>`, `Option<CalendarState>`, etc. Only the active view's state is `Some`. Alternatively, use an enum: `ViewData::Reader(ReaderState)`.

### Anti-Pattern 3: Storing Raw HTML in the Database

**What people do:** Save the raw HTML response from the scraper into SQLite, parse on read.
**Why it's wrong:** Wastes disk space, forces parsing on every article open, couples storage format to website layout changes.
**Do this instead:** Parse HTML once during scraping. Store structured data: title (text), body paragraphs (Vec<String>), image references (URL + position). The database should store the domain model, not the transport format.

### Anti-Pattern 4: Blocking SQLite on the Tokio Runtime

**What people do:** Call `rusqlite::Connection` methods directly from async tasks on the tokio runtime.
**Why it's wrong:** rusqlite is synchronous and blocking. Calling it on a tokio worker thread steals that thread from the runtime, potentially starving other tasks.
**Do this instead:** Use `tokio::task::spawn_blocking` for all database calls, or create a dedicated DB thread that receives commands via a channel. For this project's scale, `spawn_blocking` is simpler and sufficient.

### Anti-Pattern 5: Ignoring Terminal Image Protocol Fallback

**What people do:** Assume Sixel/Kitty support, crash or show nothing on unsupported terminals.
**Why it's wrong:** Most Linux terminals don't support Sixel. macOS Terminal.app doesn't support any protocol. Users on tmux/screen may have degraded support.
**Do this instead:** Always call `Picker::from_query_stdio()` with fallback to `Picker::halfblocks()`. Check the encoding result after each render. Show meaningful placeholder text when images can't render. The half-block fallback renders a coarse but recognizable image in ANY terminal.

### Anti-Pattern 6: Monolithic View Rendering

**What people do:** One giant `render` function with `match current_view` containing all layout code.
**Why it's wrong:** Becomes a 500-line function that's impossible to navigate. Calendar layout, article list layout, reader layout, and analytics layout have zero overlap.
**Do this instead:** Each view is a separate module with its own `render` function. The `ui/mod.rs` dispatcher calls `calendar::render`, `article_list::render`, etc. based on `app.current_view`. Each view only knows about the state it needs.

## Integration Points

### External Services

| Service | Integration Pattern | Notes |
|---------|---------------------|-------|
| **lapresse.ca/archives** | Async HTTP via `reqwest` + HTML parsing via `scraper` | No API — pure web scraping. Must handle site layout changes. Rate limit to be respectful (2 req/sec max). Handle 404s for future dates gracefully. |
| **Terminal graphics protocols** | ratatui-image `Picker::from_query_stdio()` | Auto-detects Sixel, Kitty, iTerm2, falls back to half-blocks. Detection happens once at startup. |

### Internal Boundaries

| Boundary | Communication | Notes |
|----------|---------------|-------|
| **UI ↔ App State** | Direct `&App` reference in render | Render functions are pure — they read state, never mutate. All mutations go through `update()`. |
| **App State ↔ Services** | Method calls from `update()` to service APIs | `update()` calls `storage.fetch_articles()`, `scraper.fetch_day()`, etc. Results come back synchronously (from storage) or via channel (from async scraper). |
| **Scraper ↔ Storage** | Scraper calls storage API directly | After parsing, scraper saves to DB before sending completion Action. This ensures data is always persisted before UI tries to read it. |
| **Image Service ↔ Storage** | Image service reads from filesystem cache | Storage holds image paths/references. Image service manages the actual image files on disk and creates rendering protocol state. |
| **UI ↔ Image Service** | UI borrows `ThreadProtocol` for rendering | The `StatefulImage` widget needs `&mut ThreadProtocol`. This must live in the App state so the render function can access it. |

## Scaling Considerations

This is a single-user, local TUI application. Scaling concerns are about data volume, not concurrent users.

| Concern | 100 articles | 10K articles | 200K articles (full 20yr archive) |
|---------|-------------|--------------|-----------------------------------|
| **SQLite query speed** | No index needed | Index on `date` column | Index on `date`, `section`, `year`. Consider `LIMIT` on list queries. |
| **Image cache size** | ~50MB | ~5GB | ~100GB. Implement LRU eviction by access time. |
| **Startup time** | Instant | < 100ms | Must lazy-load. Don't preload all article metadata at startup. Query per-view. |
| **Memory usage** | Negligible | ~20MB | Keep only active view data in memory. Decode images on demand, not upfront. |

### Scaling Priorities

1. **First bottleneck: Image memory.** Each decoded image is uncompressed RGBA pixels. A 1000×600 image is ~2.4MB in memory. With 5 images in a reader view, that's 12MB. Solution: Only decode images that are currently visible in the scroll viewport. Use `ThreadProtocol` which handles this.

2. **Second bottleneck: Scraping throughput.** 20 years × ~50 articles/day × 365 days = ~365K pages. Don't scrape everything upfront. Scrape on demand (navigate → scrape → cache). Background sync can progressively fill the cache.

## Build Order (Dependency-Based)

The components must be built in this order because of hard dependencies:

```
Phase 1: Foundation
  1. storage/     — Database schema, migrations, models, basic CRUD
  2. app.rs       — App struct, Action enum, main loop skeleton
  3. event.rs     — Crossterm event → Action mapping
  4. ui/theme.rs  — Shared styles and colors

Phase 2: Core Views (can render with dummy data)
  5. ui/calendar.rs     — Calendar component (uses dummy data initially)
  6. ui/article_list.rs  — Article list component
  7. ui/status_bar.rs    — Status bar

Phase 3: Data Pipeline
  8. scraper/client.rs   — HTTP client with rate limiting
  9. scraper/archive.rs  — Archive page parser
  10. scraper/article.rs — Article page parser

Phase 4: Full Loop (views + real data)
  11. Wire calendar → storage query → article list (real data flow)
  12. Background sync: navigate to uncached day → scrape → save → display

Phase 5: Reader with Images
  13. image/protocol.rs  — Picker setup, ThreadProtocol wrapper
  14. image/cache.rs     — Image download and filesystem cache
  15. ui/reader.rs       — Article reader with inline images
  16. Wire reader view: select article → load content + images → render

Phase 6: Polish + Analytics
  17. ui/analytics.rs    — Dashboard with charts
  18. config.rs          — User configuration
  19. Error handling, edge cases, loading states, empty states
```

**Rationale:** Storage must come first because everything reads/writes through it. The App struct and event handling define the skeleton that all views plug into. Views can be built with dummy data before the scraper exists. The scraper is independent but needs storage to persist results. Images come last because they depend on both the scraper (to download) and the reader view (to display). Analytics is last because it's a read-only view over accumulated data.

## Sources

- Ratatui official docs: Application Patterns — TEA, Component, Flux architectures (https://ratatui.rs/concepts/application-patterns/)
- Ratatui official async tutorial: Full async events pattern with tokio (https://ratatui.rs/tutorials/counter-async-app/full-async-events)
- ratatui-image README: ThreadProtocol, Picker, StatefulImage usage (https://github.com/benjajaja/ratatui-image)
- ratatui-image tokio example: `examples/tokio.rs` — async image rendering with ThreadProtocol
- eilmeldung project structure: Real-world Rust TUI news reader with ratatui-image (https://github.com/christo-auer/eilmeldung)
- spotatui project: `CoverArt` pattern for managing StatefulProtocol in TUI (https://github.com/LargeModGames/spotatui)
- ChrisTitusTech/linutil: Logo component using ratatui-image Picker (https://github.com/ChrisTitusTech/linutil)
- rusqlite docs: Connection management, OpenFlags, thread safety (https://docs.rs/rusqlite/0.39.0)

---
*Architecture research for: Rust TUI newspaper archive reader*
*Researched: 2026-04-13*
