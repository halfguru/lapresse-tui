mod download;
mod progress;
mod scraping;

pub use download::fetch_and_store_image;
pub use progress::SyncStats;
#[cfg(test)]
pub use scraping::{parse_article_page, parse_day_page};

use anyhow::Result;
use chrono::NaiveDate;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::mpsc::Sender;
use std::time::Duration;
use tokio::task::JoinSet;

use crate::app::{SyncMsg, SyncPhase, SyncPhaseKind};
use crate::db::{Db, NewArticle, NewImage};

const BASE_URL: &str = "https://www.lapresse.ca";
const REQUEST_DELAY: Duration = Duration::from_millis(500);
const CLI_DELAY: Duration = Duration::from_millis(100);
const ARTICLE_CONCURRENCY: usize = 4;
const IMAGE_CONCURRENCY: usize = 8;

struct PendingImage {
    article_id: u32,
    url: String,
    alt_text: Option<String>,
}

fn progress_bar(pct: u32, width: usize) -> String {
    let filled = (pct as usize * width / 100).min(width);
    let empty = width - filled;
    "█".repeat(filled) + &"░".repeat(empty)
}

pub async fn run_sync(
    db: Arc<Db>,
    from: NaiveDate,
    to: NaiveDate,
    metadata_only: bool,
) -> Result<SyncStats> {
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
            ..Default::default()
        });
    }

    let retry_count = dates
        .iter()
        .filter(|d| {
            let s = d.format("%Y-%m-%d").to_string();
            db.get_sync_state(&s).ok().flatten().is_some()
        })
        .count();

    println!(
        "\r  Scanning done: {skipped} synced, {to_sync} to sync ({retry_count} retries)          "
    );

    if metadata_only {
        println!("  Mode: metadata only (images fetched on-demand in TUI)");
    }
    println!(
        "  Concurrency: {ARTICLE_CONCURRENCY} articles, {}ms delay",
        CLI_DELAY.as_millis()
    );
    println!();

    let client = Arc::new(client);
    let mut stats = SyncStats::default();
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
                let pct = (overall_done * 100).checked_div(total).unwrap_or(100);
                let bar = progress_bar(pct, 30);
                let fail_str = if fails > 0 {
                    format!(", {fails} failed")
                } else {
                    String::new()
                };
                let date_str = shared_date.lock().expect("date lock poisoned").clone();
                let spin = spinner_frames[frame % spinner_frames.len()];
                if date_str.is_empty() {
                    print!(
                        "\r  [{bar}] {pct:3}% — {batch_done}/{to_sync} days, {articles} articles{fail_str} — preparing...   "
                    );
                } else {
                    print!(
                        "\r  [{bar}] {pct:3}% — {batch_done}/{to_sync} days, {articles} articles{fail_str} {spin} {date_str}   "
                    );
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
            let mut d = shared_date.lock().expect("date lock poisoned");
            *d = date_str.clone();
        }

        db.upsert_sync_state(&date_str, "in_progress", 0, 0)?;
        let result = sync_day(
            client.clone(),
            db.clone(),
            date,
            None,
            metadata_only,
            true,
            retry_counter.clone(),
        )
        .await;

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

    let html = download::fetch_page(&client, &url, Some(&retry_counter)).await?;
    let article_links = scraping::parse_day_page(&html)?;

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

async fn scrape_article_metadata(
    client: &reqwest::Client,
    db: &Db,
    link: &scraping::ArticleLink,
    retry_counter: &AtomicU32,
) -> Result<(u32, Vec<PendingImage>)> {
    let html = download::fetch_page(client, &link.url, Some(retry_counter)).await?;
    let parsed = scraping::parse_article_page(&html, &link.url)?;

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
    let image_data = download::fetch_image(client, &pending.url).await.ok();
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
    match sync_day(
        client,
        db.clone(),
        &date,
        Some(&tx),
        true,
        false,
        retry_counter,
    )
    .await
    {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sync_parse_day_page() {
        let html = r#"
        <html><body>
        <article class="storyTextList__item">
            <a class="storyTextList__itemLink" href="/actualites/test-article">
                <span class="storyTextList__itemTitle">Test Article Title</span>
            </a>
            <span class="storyTextList__itemTime">10:30</span>
        </article>
        <article class="storyTextList__item">
            <a class="storyTextList__itemLink" href="https://www.lapresse.ca/sports/other">
                <span class="storyTextList__itemTitle">Other Article</span>
            </a>
        </article>
        <div class="not-an-article">ignore me</div>
        </body></html>
        "#;

        let links = parse_day_page(html).unwrap();
        assert_eq!(links.len(), 2);
        assert_eq!(
            links[0].url,
            "https://www.lapresse.ca/actualites/test-article"
        );
        assert_eq!(links[0].title, "Test Article Title");
        assert_eq!(links[1].url, "https://www.lapresse.ca/sports/other");
        assert_eq!(links[1].title, "Other Article");
    }

    #[test]
    fn sync_parse_article_page() {
        let html = r#"
        <html><head>
            <meta property="og:title" content="Breaking News in Montreal">
            <meta property="article:section" content="Actualites">
            <meta property="article:published_time" content="2025-06-15T10:30:00-04:00">
        </head><body>
            <div class="authorModule">Jean Tremblay</div>
            <div class="articleBody">
                <p class="paragraph textModule">First paragraph of the article.</p>
                <p class="paragraph textModule">Second paragraph here.</p>
            </div>
            <img class="photoModule__visual" src="https://images.lapresse.ca/photo.jpg" alt="A photo">
        </body></html>
        "#;

        let parsed = parse_article_page(html, "https://lapresse.ca/test").unwrap();
        assert_eq!(parsed.title, "Breaking News in Montreal");
        assert_eq!(parsed.section, Some("Actualites".to_string()));
        assert_eq!(parsed.author, Some("Jean Tremblay".to_string()));
        assert_eq!(parsed.published_at, "2025-06-15T10:30:00-04:00");
        assert!(
            parsed
                .content_text
                .as_ref()
                .unwrap()
                .contains("First paragraph")
        );
        assert!(
            parsed
                .content_text
                .as_ref()
                .unwrap()
                .contains("Second paragraph")
        );
        assert_eq!(parsed.images.len(), 1);
        assert_eq!(parsed.images[0].url, "https://images.lapresse.ca/photo.jpg");
        assert_eq!(parsed.images[0].alt_text, Some("A photo".to_string()));
    }

    #[test]
    fn sync_parse_empty_day_page() {
        let html = "<html><body><p>No articles today</p></body></html>";
        let links = parse_day_page(html).unwrap();
        assert!(links.is_empty());
    }
}
