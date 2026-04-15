use anyhow::{Context, Result};
use chrono::NaiveDate;
use scraper::{Html, Selector};
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::time::Duration;
use tokio::task::JoinSet;
use url::Url;

use crate::app::{SyncMsg, SyncPhase, SyncPhaseKind};
use crate::db::{Db, NewArticle, NewImage};

const BASE_URL: &str = "https://www.lapresse.ca";
const REQUEST_DELAY: Duration = Duration::from_millis(500);
const CLI_DELAY: Duration = Duration::from_millis(100);
const ARTICLE_CONCURRENCY: usize = 4;
const IMAGE_CONCURRENCY: usize = 8;
const MAX_RETRIES: u32 = 3;
const RETRY_BASE_DELAY: Duration = Duration::from_secs(5);

pub struct SyncStats {
    pub days_scraped: u32,
    pub days_failed: u32,
    pub articles_total: u32,
    pub images_total: u32,
    pub retries: u32,
    pub articles_blocked: u32,
}

impl SyncStats {
    pub fn new() -> Self {
        Self {
            days_scraped: 0,
            days_failed: 0,
            articles_total: 0,
            images_total: 0,
            retries: 0,
            articles_blocked: 0,
        }
    }
}

pub async fn run_sync(db: Arc<Db>, from: NaiveDate, to: NaiveDate, metadata_only: bool) -> Result<SyncStats> {
    let total_days = (to - from).num_days() as u32 + 1;
    print!("  Scanning {} days ({} to {})...\r", total_days, from, to);
    std::io::Write::flush(&mut std::io::stdout()).ok();

    let client = reqwest::Client::builder()
        .cookie_store(true)
        .user_agent(
            "Mozilla/5.0 (compatible; lapresse-tui/0.1; +https://github.com/halfguru/lapresse-tui)",
        )
        .build()?;

    let mut dates = Vec::new();
    let mut skipped = 0u32;
    let mut current = from;
    let mut scan_i = 0u32;
    while current <= to {
        scan_i += 1;
        if scan_i.is_multiple_of(500) {
            print!("\r  Scanning: {scan_i}/{total_days} days checked...   ");
            std::io::Write::flush(&mut std::io::stdout()).ok();
        }
        let date_str = current.format("%Y-%m-%d").to_string();
        match db.get_sync_state(&date_str)? {
            Some(status) if status == "complete" => {
                skipped += 1;
            }
            _ => {
                dates.push(current);
            }
        }
        current = current.succ_opt().unwrap();
    }

    let to_sync = dates.len() as u32;

    if to_sync == 0 {
        println!("\r  ✓ All {total_days} days already synced.          ");
        return Ok(SyncStats {
            days_scraped: skipped,
            ..SyncStats::new()
        });
    }

    let retry_count = dates.iter().filter(|d| {
        let s = d.format("%Y-%m-%d").to_string();
        db.get_sync_state(&s).ok().flatten().is_some()
    }).count();

    println!(
        "\r  Scanning done: {skipped} synced, {to_sync} to sync ({retry_count} retries)          "
    );

    if metadata_only {
        println!("  Mode: metadata only (images fetched on-demand in TUI)");
    }
    println!("  Concurrency: {ARTICLE_CONCURRENCY} articles, {}ms delay", CLI_DELAY.as_millis());
    println!();

    let client = Arc::new(client);
    let mut stats = SyncStats::new();
    let retry_counter = Arc::new(AtomicU32::new(0));
    let blocked_counter = Arc::new(AtomicU32::new(0));

    let total = skipped + to_sync;
    let spinner_frames = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
    let sync_done = Arc::new(AtomicBool::new(false));
    let shared_batch_done = Arc::new(AtomicU32::new(0));
    let shared_articles = Arc::new(AtomicU32::new(0));
    let shared_failed = Arc::new(AtomicU32::new(0));
    let shared_date = Arc::new(std::sync::Mutex::new(String::new()));

    {
        let sync_done = sync_done.clone();
        let shared_batch_done = shared_batch_done.clone();
        let shared_articles = shared_articles.clone();
        let shared_failed = shared_failed.clone();
        let shared_date = shared_date.clone();
        tokio::spawn(async move {
            let mut frame = 0;
            loop {
                if sync_done.load(Ordering::Relaxed) {
                    break;
                }
                let batch_done = shared_batch_done.load(Ordering::Relaxed);
                let articles = shared_articles.load(Ordering::Relaxed);
                let fails = shared_failed.load(Ordering::Relaxed);
                let overall_done = skipped + batch_done;
                let pct = if total > 0 { overall_done * 100 / total } else { 100 };
                let bar = progress_bar(pct, 30);
                let fail_str = if fails > 0 { format!(", {fails} failed") } else { String::new() };
                let date_str = shared_date.lock().unwrap().clone();
                let spin = spinner_frames[frame % spinner_frames.len()];
                if date_str.is_empty() {
                    print!("\r  [{bar}] {pct:3}% — {batch_done}/{to_sync} days, {articles} articles{fail_str} — preparing...   ");
                } else {
                    print!("\r  [{bar}] {pct:3}% — {batch_done}/{to_sync} days, {articles} articles{fail_str} {spin} {date_str}   ");
                }
                std::io::Write::flush(&mut std::io::stdout()).ok();
                frame += 1;
                tokio::time::sleep(Duration::from_millis(80)).await;
            }
        });
    }

    for date in &dates {
        let date_str = date.format("%Y-%m-%d").to_string();
        {
            let mut d = shared_date.lock().unwrap();
            *d = date_str.clone();
        }

        db.upsert_sync_state(&date_str, "in_progress", 0, 0)?;
        let result = sync_day(client.clone(), db.clone(), date, None, metadata_only, true, retry_counter.clone()).await;

        match result {
            Ok((article_count, image_count)) => {
                db.upsert_sync_state(&date_str, "complete", article_count, article_count)?;
                stats.days_scraped += 1;
                stats.articles_total += article_count;
                stats.images_total += image_count;
            }
            Err(e) => {
                db.upsert_sync_state(&date_str, "failed", 0, 0)?;
                stats.days_failed += 1;
                tracing::debug!("Failed to sync {date_str}: {e:#}");
            }
        }

        shared_batch_done.store(stats.days_scraped + stats.days_failed, Ordering::Relaxed);
        shared_articles.store(stats.articles_total, Ordering::Relaxed);
        shared_failed.store(stats.days_failed, Ordering::Relaxed);
    }

    stats.retries = retry_counter.load(Ordering::Relaxed);
    stats.articles_blocked = blocked_counter.load(Ordering::Relaxed);
    sync_done.store(true, Ordering::Relaxed);
    println!();

    Ok(stats)
}

fn progress_bar(pct: u32, width: usize) -> String {
    let filled = (pct as usize * width / 100).min(width);
    let empty = width - filled;
    "█".repeat(filled) + &"░".repeat(empty)
}

async fn sync_day(
    client: Arc<reqwest::Client>,
    db: Arc<Db>,
    date: &NaiveDate,
    tx: Option<&Sender<SyncMsg>>,
    metadata_only: bool,
    skip_delay: bool,
    retry_counter: Arc<AtomicU32>,
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

    let html = fetch_page(&client, &url, Some(&retry_counter)).await?;
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
        let retry_counter = retry_counter.clone();
        scrape_set.spawn(async move {
            let _permit = sem.acquire().await.unwrap();
            if skip_delay {
                tokio::time::sleep(CLI_DELAY).await;
            } else {
                tokio::time::sleep(REQUEST_DELAY).await;
            }
            scrape_article_metadata(&client, &db, &link, &retry_counter).await
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
                tracing::debug!("Failed to scrape article: {e:#}");
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
        if metadata_only {
            for pending in all_pending_images {
                db.insert_image(&NewImage {
                    article_id: pending.article_id,
                    url: &pending.url,
                    alt_text: pending.alt_text.as_deref(),
                    data: None,
                    format: None,
                    width: None,
                    height: None,
                })?;
            }
        } else {
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
    retry_counter: &AtomicU32,
) -> Result<(u32, Vec<PendingImage>)> {
    let html = fetch_page(client, &link.url, Some(retry_counter)).await?;
    let parsed = parse_article_page(&html, &link.url)?;

    let article_id = db.insert_article(&NewArticle {
        url: &link.url,
        title: &parsed.title,
        section: parsed.section.as_deref(),
        author: parsed.author.as_deref(),
        published_at: &parsed.published_at,
        content_text: parsed.content_text.as_deref(),
        content_html: parsed.content_html.as_deref(),
    })?;

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

    db.insert_image(&NewImage {
        article_id: pending.article_id,
        url: &pending.url,
        alt_text: pending.alt_text.as_deref(),
        data: blob_data,
        format: None,
        width,
        height,
    })?;

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

async fn fetch_page(client: &reqwest::Client, url: &str, retry_counter: Option<&AtomicU32>) -> Result<String> {
    tracing::debug!("Fetching {url}");
    for attempt in 0..=MAX_RETRIES {
        let response = client
            .get(url)
            .send()
            .await
            .with_context(|| format!("Failed to fetch {url}"))?;
        let status = response.status();
        if status.is_success() {
            let text = response
                .text()
                .await
                .with_context(|| format!("Failed to read response from {url}"))?;
            return Ok(text);
        }
        if (status.as_u16() == 403 || status.as_u16() == 429) && attempt < MAX_RETRIES {
            if let Some(counter) = retry_counter {
                counter.fetch_add(1, Ordering::Relaxed);
            }
            let base_delay = RETRY_BASE_DELAY * 2u32.saturating_pow(attempt);
            let jitter = Duration::from_millis(rand::random::<u64>() % 3000);
            let delay = base_delay + jitter;
            tracing::debug!("HTTP {status} for {url}, retrying in {delay:?}...");
            tokio::time::sleep(delay).await;
            continue;
        }
        anyhow::bail!("HTTP {status} for {url}");
    }
    unreachable!()
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

    let retry_counter = Arc::new(AtomicU32::new(0));
    match sync_day(client, db.clone(), &date, Some(&tx), false, false, retry_counter).await {
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

pub async fn fetch_and_store_image(db: &Db, image_id: u32, url: &str) -> Result<()> {
    let client = reqwest::Client::builder()
        .cookie_store(true)

        .user_agent("Mozilla/5.0 (compatible; lapresse-tui/0.1; +https://github.com/halfguru/lapresse-tui)")
        .build()?;

    let image_data = fetch_image(&client, url).await?;
    let (width, height) = image::ImageReader::new(std::io::Cursor::new(&image_data))
        .with_guessed_format()
        .ok()
        .and_then(|reader| reader.into_dimensions().ok())
        .map(|(w, h)| (Some(w), Some(h)))
        .unwrap_or((None, None));

    db.update_image_data(image_id, &image_data, width, height)?;
    Ok(())
}

#[cfg(test)]
pub fn parse_day_page_for_test(html: &str) -> Result<Vec<(String, String, Option<String>)>> {
    let links = parse_day_page(html)?;
    Ok(links
        .into_iter()
        .map(|l| (l.url, l.title, l.time))
        .collect())
}

#[cfg(test)]
#[allow(clippy::type_complexity)]
pub fn parse_article_page_for_test(
    html: &str,
    url: &str,
) -> Result<(
    String,
    Option<String>,
    Option<String>,
    String,
    Option<String>,
    Vec<(String, Option<String>)>,
)> {
    let p = parse_article_page(html, url)?;
    let images = p.images.into_iter().map(|i| (i.url, i.alt_text)).collect();
    Ok((
        p.title,
        p.section,
        p.author,
        p.published_at,
        p.content_text,
        images,
    ))
}
