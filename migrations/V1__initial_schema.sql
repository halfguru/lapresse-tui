CREATE TABLE IF NOT EXISTS articles (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    url TEXT NOT NULL UNIQUE,
    title TEXT NOT NULL,
    section TEXT,
    author TEXT,
    published_at TEXT NOT NULL,
    content_text TEXT,
    content_html TEXT,
    scraped_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS images (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    article_id INTEGER NOT NULL REFERENCES articles(id) ON DELETE CASCADE,
    url TEXT NOT NULL,
    alt_text TEXT,
    data BLOB,
    format TEXT,
    width INTEGER,
    height INTEGER,
    scraped_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS sync_state (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    date TEXT NOT NULL UNIQUE,
    status TEXT NOT NULL DEFAULT 'pending' CHECK(status IN ('pending', 'in_progress', 'complete', 'failed')),
    articles_found INTEGER NOT NULL DEFAULT 0,
    articles_scraped INTEGER NOT NULL DEFAULT 0,
    last_attempt_at TEXT
);

CREATE INDEX IF NOT EXISTS idx_articles_published_at ON articles(published_at);
CREATE INDEX IF NOT EXISTS idx_articles_section ON articles(section);
CREATE INDEX IF NOT EXISTS idx_images_article_id ON images(article_id);
CREATE INDEX IF NOT EXISTS idx_sync_state_date ON sync_state(date);
CREATE INDEX IF NOT EXISTS idx_sync_state_status ON sync_state(status);
