mod app;
mod db;
mod ui;

use anyhow::Result;
use app::App;
use clap::Parser;
use crossterm::event;
use db::Db;
use ratatui_image::picker::Picker;
use std::path::PathBuf;
use std::time::Duration;

#[derive(Parser)]
#[command(name = "lpresse", about = "La Presse archive reader")]
struct Cli {
    #[arg(long)]
    db: Option<PathBuf>,
}

fn default_db_path() -> PathBuf {
    dirs::cache_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("lpresse")
        .join("lpresse.db")
}

fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();
    let db_path = cli.db.unwrap_or_else(default_db_path);

    let db = Db::open(&db_path)?;
    tracing::info!("Database opened at {}", db_path.display());

    let mut terminal = ratatui::init();
    tracing::info!("Terminal initialized");

    let picker = Picker::from_query_stdio().unwrap_or_else(|_| Picker::halfblocks());
    let protocol_type = picker.protocol_type();
    tracing::info!("Image protocol: {protocol_type:?}");

    let mut app = App::new(db, db_path, protocol_type)?;

    while !app.should_quit {
        terminal.draw(|frame| ui::render(frame, &app))?;

        if event::poll(Duration::from_millis(250))?
            && let event::Event::Key(key_event) = event::read()?
        {
            app.handle_key(key_event);
        }
    }

    ratatui::restore();
    Ok(())
}
