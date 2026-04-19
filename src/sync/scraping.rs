use anyhow::Result;
use scraper::{Html, Selector};
use url::Url;

use super::BASE_URL;

#[expect(dead_code)]
pub struct ArticleLink {
    pub url: String,
    pub title: String,
    pub time: Option<String>,
}

pub struct ParsedArticle {
    pub title: String,
    pub section: Option<String>,
    pub author: Option<String>,
    pub published_at: String,
    pub content_text: Option<String>,
    pub content_html: Option<String>,
    pub images: Vec<ParsedImage>,
}

pub struct ParsedImage {
    pub url: String,
    pub alt_text: Option<String>,
}

pub fn parse_day_page(html: &str) -> Result<Vec<ArticleLink>> {
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

pub fn parse_article_page(html: &str, article_url: &str) -> Result<ParsedArticle> {
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
