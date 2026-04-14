use anyhow::Result;
use rusqlite::Connection;
use std::collections::HashMap;
use std::path::Path;

const SCHEMA_SQL: &str = include_str!("../migrations/V1__initial_schema.sql");

#[allow(dead_code)]
pub struct Article {
    pub id: u32,
    pub title: String,
    pub section: Option<String>,
    pub author: Option<String>,
    pub published_at: String,
}

pub struct Db {
    conn: Connection,
}

impl Db {
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(path)?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;
        conn.execute_batch(SCHEMA_SQL)?;
        Ok(Self { conn })
    }

    pub fn article_count(&self) -> Result<u32> {
        let count: u32 = self
            .conn
            .query_row("SELECT COUNT(*) FROM articles", [], |row| row.get(0))?;
        Ok(count)
    }

    pub fn articles_by_date(&self, date: &str) -> Result<Vec<Article>> {
        let mut stmt = self.conn.prepare(
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
        let mut stmt = self.conn.prepare(
            "SELECT CAST(strftime('%d', published_at) AS INTEGER) as day, COUNT(*) FROM articles WHERE published_at >= ? AND published_at < ? GROUP BY day",
        )?;
        let start = format!("{year:04}-{month:02}-01T00:00:00");
        let next_month = if month == 12 {
            format!("{:04}-01-01T00:00:00", year + 1)
        } else {
            format!("{year:04}-{:02}-01T00:00:00", month + 1)
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
}
