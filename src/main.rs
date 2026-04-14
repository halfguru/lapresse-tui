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
#[    command(name = "lapresse-tui", about = "La Presse archive reader")]
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
        Some(Commands::Sync { from, to }) => {
            tracing_subscriber::fmt::init();
            let from_date = match from {
                Some(s) => parse_date(&s)?,
                None => NaiveDate::from_ymd_opt(2005, 1, 1).unwrap(),
            };
            let to_date = match to {
                Some(s) => parse_date(&s)?,
                None => chrono::Local::now().date_naive(),
            };

            println!("Syncing La Presse archives: {} to {}\n", from_date, to_date);

            let stats = sync::run_sync(Arc::new(db), from_date, to_date).await?;

            println!("\n── Sync Complete ──");
            println!("  Days:     {} total, {} scraped, {} failed", stats.days_total, stats.days_scraped, stats.days_failed);
            println!("  Articles: {}", stats.articles_total);
            println!("  Images:   {}", stats.images_total);
        }
        None => {
            let log_file = std::fs::File::create(
                dirs::cache_dir().unwrap_or_else(|| PathBuf::from(".")).join("lapresse-tui").join("lapresse-tui.log")
            ).ok();
            if let Some(f) = log_file {
                tracing_subscriber::fmt().with_writer(f).with_ansi(false).init();
            }

            let mut terminal = ratatui::init();

            let picker = Picker::from_query_stdio().unwrap_or_else(|_| Picker::halfblocks());
            let protocol_type = picker.protocol_type();
            tracing::info!("Image protocol: {protocol_type:?}");

            let mut app = App::new(db, db_path, picker, protocol_type)?;

            while !app.should_quit {
                app.poll_sync();
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
