use anyhow::{Context, Result};
use chrono::NaiveDate;
use scraper::{Html, Selector};
use std::sync::Arc;
use std::sync::mpsc::Sender;
use std::time::Duration;
use tokio::sync::Semaphore;
use tokio::task::JoinSet;
use url::Url;

use crate::app::{SyncMsg, SyncPhase, SyncPhaseKind};
use crate::db::Db;

const BASE_URL: &str = "https://www.lapresse.ca";
const REQUEST_DELAY: Duration = Duration::from_millis(500);
const MAX_CONCURRENT_DAYS: usize = 8;
const ARTICLE_CONCURRENCY: usize = 8;
const IMAGE_CONCURRENCY: usize = 8;

pub struct SyncStats {
    pub days_total: u32,
    pub days_scraped: u32,
    pub days_failed: u32,
    pub articles_total: u32,
    pub images_total: u32,
}

impl SyncStats {
    pub fn new() -> Self {
        Self {
            days_total: 0,
            days_scraped: 0,
            days_failed: 0,
            articles_total: 0,
            images_total: 0,
        }
    }
}

pub async fn run_sync(db: Arc<Db>, from: NaiveDate, to: NaiveDate) -> Result<SyncStats> {
    let client = reqwest::Client::builder()
        .cookie_store(true)
        .user_agent(
            "Mozilla/5.0 (compatible; lapresse-tui/0.1; +https://github.com/halfguru/lapresse-tui)",
        )
        .build()?;

    let mut dates = Vec::new();
    let mut current = from;
    while current <= to {
        let date_str = current.format("%Y-%m-%d").to_string();
        match db.get_sync_state(&date_str)? {
            Some(status) if status == "complete" => {
                println!("  ✓ {} — already synced, skipping", date_str);
            }
            Some(_) => {
                println!("  ⟳ {} — queued (retry)", date_str);
                dates.push(current);
            }
            None => {
                println!("  → {} — queued", date_str);
                dates.push(current);
            }
        }
        current = current.succ_opt().unwrap();
    }

    if dates.is_empty() {
        println!("\n  All days already synced.");
        return Ok(SyncStats::new());
    }

    let total = dates.len();
    println!(
        "\n  Syncing {} day(s) with {MAX_CONCURRENT_DAYS} concurrent workers...\n",
        total
    );

    let semaphore = Arc::new(Semaphore::new(MAX_CONCURRENT_DAYS));
    let client = Arc::new(client);
    let mut set: JoinSet<Result<DayResult, anyhow::Error>> = JoinSet::new();

    for date in dates {
        let db = db.clone();
        let client = client.clone();
        let sem = semaphore.clone();
        set.spawn(async move {
            let _permit = sem.acquire().await.unwrap();
            let date_str = date.format("%Y-%m-%d").to_string();
            db.upsert_sync_state(&date_str, "in_progress", 0, 0)?;
            match sync_day(client, db.clone(), &date, None).await {
                Ok((article_count, image_count)) => {
                    db.upsert_sync_state(&date_str, "complete", article_count, article_count)?;
                    Ok(DayResult {
                        date: date_str,
                        articles: article_count,
                        images: image_count,
                        failed: false,
                    })
                }
                Err(e) => {
                    db.upsert_sync_state(&date_str, "failed", 0, 0)?;
                    tracing::warn!("Failed to sync {}: {e:#}", date_str);
                    Ok(DayResult {
                        date: date_str,
                        articles: 0,
                        images: 0,
                        failed: true,
                    })
                }
            }
        });
    }

    let mut stats = SyncStats {
        days_total: total as u32,
        ..SyncStats::new()
    };

    while let Some(result) = set.join_next().await {
        let day = result??;
        if day.failed {
            stats.days_failed += 1;
            println!("    ✗ {} — failed", day.date);
        } else {
            stats.days_scraped += 1;
            stats.articles_total += day.articles;
            stats.images_total += day.images;
            println!(
                "    ✓ {} — {} articles, {} images",
                day.date, day.articles, day.images
            );
        }
    }

    Ok(stats)
}

struct DayResult {
    date: String,
    articles: u32,
    images: u32,
    failed: bool,
}

async fn sync_day(
    client: Arc<reqwest::Client>,
    db: Arc<Db>,
    date: &NaiveDate,
    tx: Option<&Sender<SyncMsg>>,
) -> Result<(u32, u32)> {
    if let Some(tx) = tx {
        let _ = tx.send(SyncMsg::Progress(SyncPhase {
            phase: SyncPhaseKind::FetchingIndex,
            current: 0,
            total: 0,
        }));
    }

    let url = format!(
        "{}/archives/{}/{}.php",
        BASE_URL,
        date.format("%Y/%-m"),
        date.format("%-d")
    );

    let html = fetch_page(&client, &url).await?;
    let article_links = parse_day_page(&html)?;

    if article_links.is_empty() {
        return Ok((0, 0));
    }

    let article_count = article_links.len() as u32;

    if let Some(tx) = tx {
        let _ = tx.send(SyncMsg::Progress(SyncPhase {
            phase: SyncPhaseKind::ScrapingArticles,
            current: 0,
            total: article_count,
        }));
    }

    let sem = Arc::new(tokio::sync::Semaphore::new(ARTICLE_CONCURRENCY));
    let mut scrape_set: JoinSet<Result<(u32, Vec<PendingImage>), anyhow::Error>> = JoinSet::new();

    for link in article_links {
        let sem = sem.clone();
        let client = client.clone();
        let db = db.clone();
        scrape_set.spawn(async move {
            let _permit = sem.acquire().await.unwrap();
            tokio::time::sleep(REQUEST_DELAY).await;
            scrape_article_metadata(&client, &db, &link).await
        });
    }

    let mut total_articles = 0u32;
    let mut articles_done = 0u32;
    let mut all_pending_images: Vec<PendingImage> = Vec::new();

    while let Some(result) = scrape_set.join_next().await {
        articles_done += 1;
        match result? {
            Ok((_, pending_images)) => {
                total_articles += 1;
                all_pending_images.extend(pending_images);
            }
            Err(e) => {
                tracing::warn!("Failed to scrape article: {e:#}");
            }
        }
        if let Some(tx) = tx {
            let _ = tx.send(SyncMsg::Progress(SyncPhase {
                phase: SyncPhaseKind::ScrapingArticles,
                current: articles_done,
                total: article_count,
            }));
        }
    }

    let total_images = all_pending_images.len() as u32;

    if !all_pending_images.is_empty() {
        if let Some(tx) = tx {
            let _ = tx.send(SyncMsg::Progress(SyncPhase {
                phase: SyncPhaseKind::DownloadingImages,
                current: 0,
                total: total_images,
            }));
        }

        let img_sem = Arc::new(tokio::sync::Semaphore::new(IMAGE_CONCURRENCY));
        let mut img_set: JoinSet<Result<(), anyhow::Error>> = JoinSet::new();
        let mut images_done = 0u32;

        for pending in all_pending_images {
            let img_sem = img_sem.clone();
            let client = client.clone();
            let db = db.clone();
            img_set.spawn(async move {
                let _permit = img_sem.acquire().await.unwrap();
                download_and_store_image(&client, &db, pending).await
            });
        }

        while let Some(result) = img_set.join_next().await {
            images_done += 1;
            if let Err(e) = result? {
                tracing::warn!("Image download failed: {e:#}");
            }
            if let Some(tx) = tx {
                let _ = tx.send(SyncMsg::Progress(SyncPhase {
                    phase: SyncPhaseKind::DownloadingImages,
                    current: images_done,
                    total: total_images,
                }));
            }
        }
    }

    Ok((total_articles, total_images))
}

#[allow(dead_code)]
struct ArticleLink {
    url: String,
    title: String,
    time: Option<String>,
}

fn parse_day_page(html: &str) -> Result<Vec<ArticleLink>> {
    let document = Html::parse_document(html);
    let item_selector = Selector::parse("article.storyTextList__item").unwrap();
    let link_selector = Selector::parse("a.storyTextList__itemLink").unwrap();
    let title_selector = Selector::parse("span.storyTextList__itemTitle").unwrap();
    let time_selector = Selector::parse("span.storyTextList__itemTime").unwrap();

    let mut links = Vec::new();

    for item in document.select(&item_selector) {
        let Some(link_el) = item.select(&link_selector).next() else {
            continue;
        };
        let href = match link_el.value().attr("href") {
            Some(h) => h.to_string(),
            None => continue,
        };

        let full_url = if href.starts_with("http") {
            href
        } else {
            format!("{BASE_URL}{href}")
        };

        let title = item
            .select(&title_selector)
            .next()
            .map(|el| el.text().collect::<String>().trim().to_string())
            .unwrap_or_default();

        let time = item
            .select(&time_selector)
            .next()
            .map(|el| el.text().collect::<String>().trim().to_string());

        links.push(ArticleLink {
            url: full_url,
            title,
            time,
        });
    }

    Ok(links)
}

struct PendingImage {
    article_id: u32,
    url: String,
    alt_text: Option<String>,
}

async fn scrape_article_metadata(
    client: &reqwest::Client,
    db: &Db,
    link: &ArticleLink,
) -> Result<(u32, Vec<PendingImage>)> {
    let html = fetch_page(client, &link.url).await?;
    let parsed = parse_article_page(&html, &link.url)?;

    let section = parsed.section.as_deref();
    let author = parsed.author.as_deref();
    let content_text = parsed.content_text.as_deref();
    let content_html = parsed.content_html.as_deref();

    let article_id = db.insert_article(
        &link.url,
        &parsed.title,
        section,
        author,
        &parsed.published_at,
        content_text,
        content_html,
    )?;

    let pending_images: Vec<PendingImage> = parsed
        .images
        .into_iter()
        .map(|img| PendingImage {
            article_id,
            url: img.url,
            alt_text: img.alt_text,
        })
        .collect();

    Ok((article_id, pending_images))
}

async fn download_and_store_image(
    client: &reqwest::Client,
    db: &Db,
    pending: PendingImage,
) -> Result<()> {
    let image_data = fetch_image(client, &pending.url).await.ok();
    let blob_data = image_data.as_deref();

    let (width, height) = if let Some(data) = blob_data {
        image::ImageReader::new(std::io::Cursor::new(data))
            .with_guessed_format()
            .ok()
            .and_then(|reader| reader.into_dimensions().ok())
            .map(|(w, h)| (Some(w), Some(h)))
            .unwrap_or((None, None))
    } else {
        (None, None)
    };

    db.insert_image(
        pending.article_id,
        &pending.url,
        pending.alt_text.as_deref(),
        blob_data,
        None,
        width,
        height,
    )?;

    Ok(())
}

struct ParsedArticle {
    title: String,
    section: Option<String>,
    author: Option<String>,
    published_at: String,
    content_text: Option<String>,
    content_html: Option<String>,
    images: Vec<ParsedImage>,
}

struct ParsedImage {
    url: String,
    alt_text: Option<String>,
}

fn parse_article_page(html: &str, article_url: &str) -> Result<ParsedArticle> {
    let document = Html::parse_document(html);

    let title = document
        .select(&Selector::parse("meta[property='og:title']").unwrap())
        .next()
        .and_then(|el| el.value().attr("content"))
        .unwrap_or("")
        .to_string();

    let section = document
        .select(&Selector::parse("meta[property='article:section']").unwrap())
        .next()
        .and_then(|el| el.value().attr("content"))
        .map(|s| s.to_string());

    let published_at = document
        .select(&Selector::parse("meta[property='article:published_time']").unwrap())
        .next()
        .and_then(|el| el.value().attr("content"))
        .unwrap_or("")
        .to_string();

    let author = document
        .select(&Selector::parse("div.authorModule").unwrap())
        .next()
        .map(|el| el.text().collect::<String>().trim().to_string());

    let paragraph_selector = Selector::parse("p.paragraph.textModule").unwrap();
    let paragraphs: Vec<String> = document
        .select(&paragraph_selector)
        .map(|p| p.text().collect::<String>().trim().to_string())
        .filter(|t| !t.is_empty())
        .collect();

    let content_text = if paragraphs.is_empty() {
        None
    } else {
        Some(paragraphs.join("\n\n"))
    };

    let body_selector = Selector::parse("div.articleBody").unwrap();
    let content_html = document
        .select(&body_selector)
        .next()
        .map(|el| el.inner_html());

    let img_selector = Selector::parse("img.photoModule__visual").unwrap();
    let base = Url::parse(BASE_URL)?;
    let article_base = Url::parse(article_url)?;

    let images: Vec<ParsedImage> = document
        .select(&img_selector)
        .filter_map(|img| {
            let src = img
                .value()
                .attr("data-src")
                .or_else(|| img.value().attr("src"))?;
            let resolved = if src.starts_with("http") {
                src.to_string()
            } else {
                base.join(src)
                    .or_else(|_| article_base.join(src))
                    .ok()?
                    .to_string()
            };
            let alt_text = img.value().attr("alt").map(|s| s.to_string());
            Some(ParsedImage {
                url: resolved,
                alt_text,
            })
        })
        .collect();

    Ok(ParsedArticle {
        title,
        section,
        author,
        published_at,
        content_text,
        content_html,
        images,
    })
}

async fn fetch_page(client: &reqwest::Client, url: &str) -> Result<String> {
    tracing::debug!("Fetching {url}");
    let response = client
        .get(url)
        .send()
        .await
        .with_context(|| format!("Failed to fetch {url}"))?;
    let status = response.status();
    if !status.is_success() {
        anyhow::bail!("HTTP {status} for {url}");
    }
    let text = response
        .text()
        .await
        .with_context(|| format!("Failed to read response from {url}"))?;
    Ok(text)
}

async fn fetch_image(client: &reqwest::Client, url: &str) -> Result<Vec<u8>> {
    tracing::debug!("Fetching image {url}");
    let response = client
        .get(url)
        .send()
        .await
        .with_context(|| format!("Failed to fetch image {url}"))?;
    let status = response.status();
    if !status.is_success() {
        anyhow::bail!("HTTP {status} for image {url}");
    }
    let bytes = response
        .bytes()
        .await
        .with_context(|| format!("Failed to read image from {url}"))?;
    Ok(bytes.to_vec())
}

pub async fn sync_single_day_with_progress(
    db: Arc<Db>,
    date: NaiveDate,
    tx: Sender<SyncMsg>,
) -> Result<()> {
    let client = Arc::new(
        reqwest::Client::builder()
            .cookie_store(true)
            .user_agent("Mozilla/5.0 (compatible; lapresse-tui/0.1; +https://github.com/halfguru/lapresse-tui)")
            .build()?,
    );

    let date_str = date.format("%Y-%m-%d").to_string();
    db.upsert_sync_state(&date_str, "in_progress", 0, 0)?;

    let _ = tx.send(SyncMsg::Started);

    match sync_day(client, db.clone(), &date, Some(&tx)).await {
        Ok((articles, images)) => {
            let date_str = date.format("%Y-%m-%d").to_string();
            db.upsert_sync_state(&date_str, "complete", articles, articles)?;
            let _ = tx.send(SyncMsg::Done(articles, images));
            Ok(())
        }
        Err(e) => {
            let date_str = date.format("%Y-%m-%d").to_string();
            db.upsert_sync_state(&date_str, "failed", 0, 0)?;
            let _ = tx.send(SyncMsg::Failed);
            Err(e)
        }
    }
}
