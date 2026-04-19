use anyhow::{Context, Result};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

use crate::db::Db;

const MAX_RETRIES: u32 = 3;
const RETRY_BASE_DELAY: Duration = Duration::from_secs(5);

pub(super) async fn fetch_page(
    client: &reqwest::Client,
    url: &str,
    retry_counter: Option<&AtomicU32>,
) -> Result<String> {
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

pub(super) async fn fetch_image(client: &reqwest::Client, url: &str) -> Result<Vec<u8>> {
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

pub async fn fetch_and_store_image(db: &Db, image_id: u32, url: &str) -> Result<()> {
    let client = reqwest::Client::builder()
        .cookie_store(true)
        .user_agent(
            "Mozilla/5.0 (compatible; lapresse-tui/0.1; +https://github.com/halfguru/lapresse-tui)",
        )
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
