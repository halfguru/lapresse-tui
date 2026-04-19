mod types;

pub use types::{Article, ArticleImage, FullArticle, NewArticle, NewImage};

use anyhow::Result;
use rusqlite::Connection;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Mutex;

const SCHEMA_SQL: &str = include_str!("../../migrations/V1__initial_schema.sql");

pub struct Db {
    conn: Mutex<Connection>,
}

impl Db {
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(path)?;
        conn.execute_batch(
            "PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON; PRAGMA busy_timeout=5000;",
        )?;
        conn.execute_batch(SCHEMA_SQL)?;
        conn.execute(
            "UPDATE sync_state SET status = 'pending' WHERE status = 'in_progress'",
            [],
        )?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    pub fn rebuild_fts(&self) -> Result<()> {
        let conn = self.conn.lock().expect("db connection poisoned");
        conn.execute(
            "INSERT INTO articles_fts(articles_fts) VALUES ('rebuild')",
            [],
        )?;
        Ok(())
    }

    pub fn article_count(&self) -> Result<u32> {
        let conn = self.conn.lock().expect("db connection poisoned");
        let count: u32 = conn.query_row("SELECT COUNT(*) FROM articles", [], |row| row.get(0))?;
        Ok(count)
    }

    pub fn articles_by_date(&self, date: &str) -> Result<Vec<Article>> {
        let conn = self.conn.lock().expect("db connection poisoned");
        let mut stmt = conn.prepare(
            "SELECT id, url, title, section, author, published_at, content_text FROM articles WHERE published_at >= ? AND published_at < ? ORDER BY published_at",
        )?;
        let start = format!("{date}T00:00:00");
        let end = format!("{date}T23:59:59");
        let articles = stmt
            .query_map([&start, &end], |row| {
                Ok(Article {
                    id: row.get(0)?,
                    url: row.get(1)?,
                    title: row.get(2)?,
                    section: row.get(3)?,
                    author: row.get(4)?,
                    published_at: row.get(5)?,
                    snippet: row.get(6)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(articles)
    }

    pub fn article_counts_by_month(&self, year: i32, month: u8) -> Result<HashMap<u8, u32>> {
        let conn = self.conn.lock().expect("db connection poisoned");
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

    pub fn insert_article(&self, article: &NewArticle) -> Result<u32> {
        let conn = self.conn.lock().expect("db connection poisoned");
        conn.execute(
            "INSERT OR IGNORE INTO articles (url, title, section, author, published_at, content_text, content_html) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![article.url, article.title, article.section, article.author, article.published_at, article.content_text, article.content_html],
        )?;
        let id: u32 = conn.query_row(
            "SELECT id FROM articles WHERE url = ?1",
            [article.url],
            |row| row.get(0),
        )?;
        Ok(id)
    }

    pub fn insert_image(&self, image: &NewImage) -> Result<()> {
        let conn = self.conn.lock().expect("db connection poisoned");
        conn.execute(
            "INSERT OR IGNORE INTO images (article_id, url, alt_text, data, format, width, height) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![image.article_id, image.url, image.alt_text, image.data, image.format, image.width, image.height],
        )?;
        Ok(())
    }

    pub fn update_image_data(
        &self,
        image_id: u32,
        data: &[u8],
        width: Option<u32>,
        height: Option<u32>,
    ) -> Result<()> {
        let conn = self.conn.lock().expect("db connection poisoned");
        conn.execute(
            "UPDATE images SET data = ?1, width = ?2, height = ?3 WHERE id = ?4",
            rusqlite::params![data, width, height, image_id],
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
        let conn = self.conn.lock().expect("db connection poisoned");
        conn.execute(
            "INSERT INTO sync_state (date, status, articles_found, articles_scraped, last_attempt_at) VALUES (?1, ?2, ?3, ?4, datetime('now')) \
             ON CONFLICT(date) DO UPDATE SET status = ?2, articles_found = ?3, articles_scraped = ?4, last_attempt_at = datetime('now')",
            rusqlite::params![date, status, articles_found, articles_scraped],
        )?;
        Ok(())
    }

    pub fn get_sync_state(&self, date: &str) -> Result<Option<String>> {
        let conn = self.conn.lock().expect("db connection poisoned");
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

    pub fn search_articles(&self, query: &str) -> Result<Vec<Article>> {
        let conn = self.conn.lock().expect("db connection poisoned");
        let mut stmt = conn.prepare(
            "SELECT a.id, a.url, a.title, a.section, a.author, a.published_at, a.content_text
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
                    url: row.get(1)?,
                    title: row.get(2)?,
                    section: row.get(3)?,
                    author: row.get(4)?,
                    published_at: row.get(5)?,
                    snippet: row.get(6)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(articles)
    }

    pub fn get_full_article(&self, article_id: u32) -> Result<Option<FullArticle>> {
        let conn = self.conn.lock().expect("db connection poisoned");
        let article = conn.query_row(
            "SELECT id, url, title, section, author, published_at, content_text FROM articles WHERE id = ?1",
            [article_id],
            |row| {
                Ok(FullArticle {
                    id: row.get(0)?,
                    url: row.get(1)?,
                    title: row.get(2)?,
                    section: row.get(3)?,
                    author: row.get(4)?,
                    published_at: row.get(5)?,
                    content_text: row.get(6)?,
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

    pub fn get_image_data(&self, image_id: u32) -> Result<Option<Vec<u8>>> {
        let conn = self.conn.lock().expect("db connection poisoned");
        let result = conn.query_row("SELECT data FROM images WHERE id = ?1", [image_id], |row| {
            row.get::<_, Vec<u8>>(0)
        });
        match result {
            Ok(data) => Ok(Some(data)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_db() -> Db {
        Db::open(std::path::Path::new(":memory:")).unwrap()
    }

    #[test]
    fn db_open_creates_schema() {
        let db = test_db();
        assert_eq!(db.article_count().unwrap(), 0);
    }

    #[test]
    fn db_insert_and_retrieve_article() {
        let db = test_db();

        let id = db
            .insert_article(&NewArticle {
                url: "https://lapresse.ca/test-article",
                title: "Test Article",
                section: Some("Actualites"),
                author: Some("Jean Tremblay"),
                published_at: "2025-06-15T10:30:00",
                content_text: Some("Hello world"),
                content_html: None,
            })
            .unwrap();

        assert!(id > 0);

        let full = db.get_full_article(id).unwrap().unwrap();
        assert_eq!(full.title, "Test Article");
        assert_eq!(full.section.as_deref(), Some("Actualites"));
        assert_eq!(full.author.as_deref(), Some("Jean Tremblay"));
        assert_eq!(full.content_text.as_deref(), Some("Hello world"));
    }

    #[test]
    fn db_insert_article_idempotent() {
        let db = test_db();
        let article = NewArticle {
            url: "https://lapresse.ca/dup",
            title: "Dup",
            section: None,
            author: None,
            published_at: "2025-01-01T00:00:00",
            content_text: None,
            content_html: None,
        };
        let id1 = db.insert_article(&article).unwrap();
        let id2 = db.insert_article(&article).unwrap();
        assert_eq!(id1, id2);
    }

    #[test]
    fn db_articles_by_date() {
        let db = test_db();
        db.insert_article(&NewArticle {
            url: "https://lapresse.ca/a1",
            title: "Article 1",
            section: None,
            author: None,
            published_at: "2025-06-15T08:00:00",
            content_text: None,
            content_html: None,
        })
        .unwrap();
        db.insert_article(&NewArticle {
            url: "https://lapresse.ca/a2",
            title: "Article 2",
            section: None,
            author: None,
            published_at: "2025-06-15T14:00:00",
            content_text: None,
            content_html: None,
        })
        .unwrap();
        db.insert_article(&NewArticle {
            url: "https://lapresse.ca/a3",
            title: "Other Day",
            section: None,
            author: None,
            published_at: "2025-06-16T10:00:00",
            content_text: None,
            content_html: None,
        })
        .unwrap();

        let articles = db.articles_by_date("2025-06-15").unwrap();
        assert_eq!(articles.len(), 2);
        assert_eq!(articles[0].title, "Article 1");
        assert_eq!(articles[1].title, "Article 2");
    }

    #[test]
    fn db_article_counts_by_month() {
        let db = test_db();
        db.insert_article(&NewArticle {
            url: "https://lapresse.ca/j1",
            title: "Day 1",
            section: None,
            author: None,
            published_at: "2025-06-01T10:00:00",
            content_text: None,
            content_html: None,
        })
        .unwrap();
        db.insert_article(&NewArticle {
            url: "https://lapresse.ca/j2",
            title: "Day 1b",
            section: None,
            author: None,
            published_at: "2025-06-01T12:00:00",
            content_text: None,
            content_html: None,
        })
        .unwrap();
        db.insert_article(&NewArticle {
            url: "https://lapresse.ca/j3",
            title: "Day 15",
            section: None,
            author: None,
            published_at: "2025-06-15T10:00:00",
            content_text: None,
            content_html: None,
        })
        .unwrap();

        let counts = db.article_counts_by_month(2025, 6).unwrap();
        assert_eq!(counts.get(&1), Some(&2));
        assert_eq!(counts.get(&15), Some(&1));
    }

    #[test]
    fn db_insert_image() {
        let db = test_db();
        let article_id = db
            .insert_article(&NewArticle {
                url: "https://lapresse.ca/img-test",
                title: "Img Test",
                section: None,
                author: None,
                published_at: "2025-01-01T00:00:00",
                content_text: None,
                content_html: None,
            })
            .unwrap();

        db.insert_image(&NewImage {
            article_id,
            url: "https://images.lapresse.ca/test.jpg",
            alt_text: Some("A test image"),
            data: Some(b"\xff\xd8\xff\xe0fake"),
            format: None,
            width: Some(800),
            height: Some(600),
        })
        .unwrap();

        let full = db.get_full_article(article_id).unwrap().unwrap();
        assert_eq!(full.images.len(), 1);
        assert_eq!(full.images[0].url, "https://images.lapresse.ca/test.jpg");
        assert_eq!(full.images[0].width, Some(800));
        assert_eq!(full.images[0].data.as_ref().unwrap().len(), 8);
    }

    #[test]
    fn db_sync_state() {
        let db = test_db();
        assert!(db.get_sync_state("2025-01-01").unwrap().is_none());

        db.upsert_sync_state("2025-01-01", "complete", 5, 5)
            .unwrap();
        assert_eq!(
            db.get_sync_state("2025-01-01").unwrap(),
            Some("complete".to_string())
        );

        db.upsert_sync_state("2025-01-01", "failed", 5, 3).unwrap();
        assert_eq!(
            db.get_sync_state("2025-01-01").unwrap(),
            Some("failed".to_string())
        );
    }

    #[test]
    fn db_search_articles() {
        let db = test_db();
        db.insert_article(&NewArticle {
            url: "https://lapresse.ca/s1",
            title: "Quebec politics update",
            section: Some("Politique"),
            author: Some("Marie Labrecque"),
            published_at: "2025-03-01T10:00:00",
            content_text: Some("The National Assembly debated new legislation today."),
            content_html: None,
        })
        .unwrap();
        db.insert_article(&NewArticle {
            url: "https://lapresse.ca/s2",
            title: "Sports highlights",
            section: Some("Sports"),
            author: None,
            published_at: "2025-03-01T12:00:00",
            content_text: Some("The Canadiens won a thrilling overtime game."),
            content_html: None,
        })
        .unwrap();

        let results = db.search_articles("Quebec*").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "Quebec politics update");

        let results = db.search_articles("Canadiens*").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "Sports highlights");
    }
}
