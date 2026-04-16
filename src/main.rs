mod app;
mod db;
mod sync;
mod ui;

use anyhow::Result;
use app::App;
use chrono::NaiveDate;
use clap::{Parser, Subcommand};
use db::Db;
use ratatui_image::picker::Picker;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

#[derive(Parser)]
#[command(name = "lapresse-tui", about = "La Presse archive reader")]
struct Cli {
    #[arg(long)]
    db: Option<PathBuf>,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Scrape articles from lapresse.ca/archives
    Sync {
        /// Start date (YYYY-MM-DD). Defaults to 2005-01-01
        #[arg(long)]
        from: Option<String>,

        /// End date (YYYY-MM-DD). Defaults to today
        #[arg(long)]
        to: Option<String>,

        /// Skip image downloads (metadata only, images fetched on-demand in TUI)
        #[arg(long)]
        metadata_only: bool,
    },
}

fn default_db_path() -> PathBuf {
    dirs::cache_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("lapresse-tui")
        .join("lapresse-tui.db")
}

fn parse_date(s: &str) -> Result<NaiveDate> {
    NaiveDate::parse_from_str(s, "%Y-%m-%d").map_err(|e| anyhow::anyhow!("Invalid date '{s}': {e}"))
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let db_path = cli.db.unwrap_or_else(default_db_path);
    let db = Db::open(&db_path)?;

    match cli.command {
        Some(Commands::Sync {
            from,
            to,
            metadata_only,
        }) => {
            tracing_subscriber::fmt()
                .with_env_filter("lapresse_tui=warn")
                .init();
            let from_date = match from {
                Some(s) => parse_date(&s)?,
                None => NaiveDate::from_ymd_opt(2005, 1, 1).unwrap(),
            };
            let to_date = match to {
                Some(s) => parse_date(&s)?,
                None => chrono::Local::now().date_naive(),
            };

            if metadata_only {
                println!(
                    "Syncing La Presse archives (metadata only): {} to {}\n",
                    from_date, to_date
                );
            } else {
                println!("Syncing La Presse archives: {} to {}\n", from_date, to_date);
            }

            let db = Arc::new(db);
            let stats = sync::run_sync(db.clone(), from_date, to_date, metadata_only).await?;

            print!("  Rebuilding search index...");
            std::io::Write::flush(&mut std::io::stdout()).ok();
            db.rebuild_fts()?;
            println!("\r  Search index rebuilt.              ");

            println!("╭──────────────────────────────╮");
            println!("│       Sync Complete          │");
            println!("├──────────────────────────────┤");
            println!("│ Days synced:  {:>12} │", stats.days_scraped);
            println!("│ Articles:     {:>12} │", stats.articles_total);
            if !metadata_only {
                println!("│ Images:       {:>12} │", stats.images_total);
            }
            if stats.retries > 0 {
                println!("│ Retries:      {:>12} │", stats.retries);
            }
            if stats.days_failed > 0 {
                println!("│ Days failed:  {:>12} │", stats.days_failed);
            }
            println!("╰──────────────────────────────╯");
        }
        None => {
            let log_file = std::fs::File::create(
                dirs::cache_dir()
                    .unwrap_or_else(|| PathBuf::from("."))
                    .join("lapresse-tui")
                    .join("lapresse-tui.log"),
            )
            .ok();
            if let Some(f) = log_file {
                tracing_subscriber::fmt()
                    .with_writer(f)
                    .with_ansi(false)
                    .init();
            }

            let mut terminal = ratatui::init();

            let picker = Picker::from_query_stdio().unwrap_or_else(|_| Picker::halfblocks());
            let protocol_type = picker.protocol_type();
            tracing::info!("Image protocol: {protocol_type:?}");

            let mut app = App::new(db, db_path, picker, protocol_type)?;

            while !app.should_quit {
                app.poll_sync();
                app.poll_search();
                terminal.draw(|frame| ui::render(frame, &mut app))?;

                if crossterm::event::poll(Duration::from_millis(250))?
                    && let crossterm::event::Event::Key(key_event) = crossterm::event::read()?
                {
                    app.handle_key(key_event);
                }
            }

            ratatui::restore();
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::db::{Db, NewArticle, NewImage};

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

        let links = crate::sync::parse_day_page_for_test(html).unwrap();
        assert_eq!(links.len(), 2);
        assert_eq!(
            links[0].0,
            "https://www.lapresse.ca/actualites/test-article"
        );
        assert_eq!(links[0].1, "Test Article Title");
        assert_eq!(links[1].0, "https://www.lapresse.ca/sports/other");
        assert_eq!(links[1].1, "Other Article");
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

        let parsed =
            crate::sync::parse_article_page_for_test(html, "https://lapresse.ca/test").unwrap();
        assert_eq!(parsed.0, "Breaking News in Montreal");
        assert_eq!(parsed.1, Some("Actualites".to_string()));
        assert_eq!(parsed.2, Some("Jean Tremblay".to_string()));
        assert_eq!(parsed.3, "2025-06-15T10:30:00-04:00");
        assert!(parsed.4.as_ref().unwrap().contains("First paragraph"));
        assert!(parsed.4.as_ref().unwrap().contains("Second paragraph"));
        assert_eq!(parsed.5.len(), 1);
        assert_eq!(parsed.5[0].0, "https://images.lapresse.ca/photo.jpg");
        assert_eq!(parsed.5[0].1, Some("A photo".to_string()));
    }

    #[test]
    fn sync_parse_empty_day_page() {
        let html = "<html><body><p>No articles today</p></body></html>";
        let links = crate::sync::parse_day_page_for_test(html).unwrap();
        assert!(links.is_empty());
    }

    #[test]
    fn ui_month_name() {
        assert_eq!(crate::ui::month_name_for_test(1), "January");
        assert_eq!(crate::ui::month_name_for_test(12), "December");
        assert_eq!(crate::ui::month_name_for_test(13), "???");
    }

    #[test]
    fn ui_format_scroll_indicator() {
        assert_eq!(
            crate::ui::format_scroll_indicator_for_test(0, true),
            " 0% ░░░░░░░░░░"
        );
        assert_eq!(
            crate::ui::format_scroll_indicator_for_test(50, true),
            " 50% █████░░░░░"
        );
        assert_eq!(
            crate::ui::format_scroll_indicator_for_test(100, true),
            " 100% ██████████"
        );
        assert!(crate::ui::format_scroll_indicator_for_test(50, false).is_empty());
    }
}
