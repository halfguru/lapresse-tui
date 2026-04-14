use anyhow::Result;
use rusqlite::Connection;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Mutex;

const SCHEMA_SQL: &str = include_str!("../migrations/V1__initial_schema.sql");

#[allow(dead_code)]
pub struct Article {
    pub id: u32,
    pub title: String,
    pub section: Option<String>,
    pub author: Option<String>,
    pub published_at: String,
}

#[allow(dead_code)]
pub struct FullArticle {
    pub id: u32,
    pub title: String,
    pub section: Option<String>,
    pub author: Option<String>,
    pub published_at: String,
    pub content_text: Option<String>,
    pub images: Vec<ArticleImage>,
}

#[allow(dead_code)]
pub struct ArticleImage {
    pub id: u32,
    pub url: String,
    pub alt_text: Option<String>,
    pub data: Option<Vec<u8>>,
    pub width: Option<u32>,
    pub height: Option<u32>,
}

pub struct Db {
    conn: Mutex<Connection>,
}

impl Db {
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(path)?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;
        conn.execute_batch(SCHEMA_SQL)?;
        conn.execute(
            "UPDATE sync_state SET status = 'pending' WHERE status = 'in_progress'",
            [],
        )?;
        conn.execute(
            "INSERT INTO articles_fts(articles_fts) VALUES ('rebuild')",
            [],
        )?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    pub fn article_count(&self) -> Result<u32> {
        let conn = self.conn.lock().unwrap();
        let count: u32 =
            conn.query_row("SELECT COUNT(*) FROM articles", [], |row| row.get(0))?;
        Ok(count)
    }

    pub fn articles_by_date(&self, date: &str) -> Result<Vec<Article>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, title, section, author, published_at FROM articles WHERE published_at >= ? AND published_at < ? ORDER BY published_at",
        )?;
        let start = format!("{date}T00:00:00");
        let end = format!("{date}T23:59:59");
        let articles = stmt
            .query_map([&start, &end], |row| {
                Ok(Article {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    section: row.get(2)?,
                    author: row.get(3)?,
                    published_at: row.get(4)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(articles)
    }

    pub fn article_counts_by_month(&self, year: i32, month: u8) -> Result<HashMap<u8, u32>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT CAST(strftime('%d', published_at) AS INTEGER) as day, COUNT(*) FROM articles WHERE published_at >= ? AND published_at < ? GROUP BY day",
        )?;
        let start = format!("{year:04}-{month:02}-01T00:00:00");
        let next_month = if month == 12 {
            format!("{:04}-01-01T00:00:00", year + 1)
        } else {
            let next = month + 1;
            format!("{year:04}-{next:02}-01T00:00:00")
        };
        let counts: HashMap<u8, u32> = stmt
            .query_map([&start, &next_month], |row| {
                let day: u8 = row.get(0)?;
                let count: u32 = row.get(1)?;
                Ok((day, count))
            })?
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .collect();
        Ok(counts)
    }

    pub fn insert_article(
        &self,
        url: &str,
        title: &str,
        section: Option<&str>,
        author: Option<&str>,
        published_at: &str,
        content_text: Option<&str>,
        content_html: Option<&str>,
    ) -> Result<u32> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR IGNORE INTO articles (url, title, section, author, published_at, content_text, content_html) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![url, title, section, author, published_at, content_text, content_html],
        )?;
        let id: u32 = conn.query_row(
            "SELECT id FROM articles WHERE url = ?1",
            [url],
            |row| row.get(0),
        )?;
        Ok(id)
    }

    pub fn insert_image(
        &self,
        article_id: u32,
        url: &str,
        alt_text: Option<&str>,
        data: Option<&[u8]>,
        format: Option<&str>,
        width: Option<u32>,
        height: Option<u32>,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR IGNORE INTO images (article_id, url, alt_text, data, format, width, height) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![article_id, url, alt_text, data, format, width, height],
        )?;
        Ok(())
    }

    pub fn upsert_sync_state(
        &self,
        date: &str,
        status: &str,
        articles_found: u32,
        articles_scraped: u32,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO sync_state (date, status, articles_found, articles_scraped, last_attempt_at) VALUES (?1, ?2, ?3, ?4, datetime('now')) \
             ON CONFLICT(date) DO UPDATE SET status = ?2, articles_found = ?3, articles_scraped = ?4, last_attempt_at = datetime('now')",
            rusqlite::params![date, status, articles_found, articles_scraped],
        )?;
        Ok(())
    }

    pub fn get_sync_state(&self, date: &str) -> Result<Option<String>> {
        let conn = self.conn.lock().unwrap();
        let result = conn.query_row(
            "SELECT status FROM sync_state WHERE date = ?1",
            [date],
            |row| row.get::<_, String>(0),
        );
        match result {
            Ok(status) => Ok(Some(status)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    #[allow(dead_code)]
    pub fn get_pending_dates(&self) -> Result<Vec<String>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT date FROM sync_state WHERE status IN ('pending', 'failed') ORDER BY date",
        )?;
        let dates: Vec<String> = stmt
            .query_map([], |row| row.get(0))?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(dates)
    }

    pub fn search_articles(&self, query: &str) -> Result<Vec<Article>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT a.id, a.title, a.section, a.author, a.published_at
             FROM articles a
             JOIN articles_fts fts ON a.id = fts.rowid
             WHERE articles_fts MATCH ?
             ORDER BY rank
             LIMIT 200",
        )?;
        let articles = stmt
            .query_map([query], |row| {
                Ok(Article {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    section: row.get(2)?,
                    author: row.get(3)?,
                    published_at: row.get(4)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(articles)
    }

    pub fn get_full_article(&self, article_id: u32) -> Result<Option<FullArticle>> {
        let conn = self.conn.lock().unwrap();
        let article = conn.query_row(
            "SELECT id, title, section, author, published_at, content_text FROM articles WHERE id = ?1",
            [article_id],
            |row| {
                Ok(FullArticle {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    section: row.get(2)?,
                    author: row.get(3)?,
                    published_at: row.get(4)?,
                    content_text: row.get(5)?,
                    images: Vec::new(),
                })
            },
        );
        match article {
            Ok(mut a) => {
                let mut stmt = conn.prepare(
                    "SELECT id, url, alt_text, data, width, height FROM images WHERE article_id = ?1",
                )?;
                a.images = stmt
                    .query_map([article_id], |row| {
                        Ok(ArticleImage {
                            id: row.get(0)?,
                            url: row.get(1)?,
                            alt_text: row.get(2)?,
                            data: row.get(3)?,
                            width: row.get(4)?,
                            height: row.get(5)?,
                        })
                    })?
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(Some(a))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }
}
