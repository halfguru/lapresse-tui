mod handlers;
mod image_loader;

use crate::db::{Article, Db, FullArticle};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui_image::picker::{Picker, ProtocolType};
use ratatui_image::protocol::StatefulProtocol;
use std::sync::Arc;
use std::sync::mpsc::Receiver;
use std::time::Instant;
use time::{Date, Month, OffsetDateTime};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Focus {
    Calendar,
    ArticleList,
    ArticleReader,
    Search,
}

#[derive(Debug)]
pub enum SyncMsg {
    Started,
    Progress(SyncPhase),
    Done(u32, u32),
    Failed,
}

#[derive(Debug, Clone)]
pub struct SyncPhase {
    pub phase: SyncPhaseKind,
    pub current: u32,
    pub total: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SyncPhaseKind {
    FetchingIndex,
    ScrapingArticles,
    DownloadingImages,
}

impl std::fmt::Display for SyncPhaseKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SyncPhaseKind::FetchingIndex => write!(f, "Fetching article list"),
            SyncPhaseKind::ScrapingArticles => write!(f, "Scraping articles"),
            SyncPhaseKind::DownloadingImages => write!(f, "Downloading images"),
        }
    }
}

#[derive(Debug)]
pub enum ImageLoadMsg {
    Loaded(usize, Vec<u8>),
    Failed(usize),
}

#[expect(dead_code)]
pub struct ImageState {
    pub protocol: StatefulProtocol,
    pub alt_text: Option<String>,
}

pub enum ImageLoadState {
    Loading,
    Loaded(Box<ImageState>),
    Failed,
}

pub struct ArticleReaderState {
    pub article: FullArticle,
    pub images: Vec<ImageLoadState>,
    pub scroll_offset: u16,
    pub image_load_rx: Option<Receiver<ImageLoadMsg>>,
}

pub struct App {
    pub should_quit: bool,
    pub db: Arc<Db>,
    pub picker: Picker,
    pub protocol_type: ProtocolType,
    pub article_count: u32,
    pub selected_date: Date,
    pub focus: Focus,
    pub show_help: bool,
    pub articles: Vec<Article>,
    pub article_list_selected: usize,
    pub sync_rx: Option<Receiver<SyncMsg>>,
    pub syncing: bool,
    pub syncing_date: Option<Date>,
    pub sync_spinner: u8,
    pub sync_phase: Option<SyncPhase>,
    pub reader: Option<ArticleReaderState>,
    pub sections: Vec<String>,
    pub section_filter: Option<usize>,
    pub show_section_picker: bool,
    pub section_picker_selected: usize,
    pub section_picker_scroll: usize,
    pub search_query: String,
    pub search_results: Vec<Article>,
    pub search_selected: usize,
    pub search_pending: bool,
    pub last_search_keystroke: Option<Instant>,
    pub search_rx: Option<Receiver<Vec<Article>>>,
    pub searching: bool,
    pub search_spinner: u8,
}

impl App {
    pub fn new(db: Db, picker: Picker, protocol_type: ProtocolType) -> anyhow::Result<Self> {
        let db = Arc::new(db);
        let article_count = db.article_count()?;
        let selected_date = OffsetDateTime::now_utc().date();
        let articles = db.articles_by_date(&selected_date.to_string())?;
        let mut app = Self {
            should_quit: false,
            db,
            picker,
            protocol_type,
            article_count,
            selected_date,
            focus: Focus::ArticleList,
            show_help: false,
            article_list_selected: 0,
            articles,
            sync_rx: None,
            syncing: false,
            syncing_date: None,
            sync_spinner: 0,
            sync_phase: None,
            reader: None,
            sections: Vec::new(),
            section_filter: None,
            show_section_picker: false,
            section_picker_selected: 0,
            section_picker_scroll: 0,
            search_query: String::new(),
            search_results: Vec::new(),
            search_selected: 0,
            search_pending: false,
            last_search_keystroke: None,
            search_rx: None,
            searching: false,
            search_spinner: 0,
        };
        app.populate_sections();
        app.maybe_auto_sync();
        Ok(app)
    }

    pub fn poll_sync(&mut self) {
        if self.syncing {
            self.sync_spinner = (self.sync_spinner + 1) % 4;
        }

        image_loader::poll_image_load(self);

        let rx = match self.sync_rx.take() {
            Some(rx) => rx,
            None => return,
        };
        loop {
            match rx.try_recv() {
                Ok(SyncMsg::Done(articles, _images)) => {
                    let synced_date = self.syncing_date;
                    self.syncing = false;
                    self.syncing_date = None;
                    self.sync_phase = None;
                    if synced_date == Some(self.selected_date) {
                        self.refresh_articles();
                    }
                    tracing::info!("On-demand sync complete: {articles} articles");
                    self.maybe_auto_sync();
                    return;
                }
                Ok(SyncMsg::Failed) => {
                    self.syncing = false;
                    self.syncing_date = None;
                    self.sync_phase = None;
                    tracing::warn!("On-demand sync failed");
                    self.maybe_auto_sync();
                    return;
                }
                Ok(SyncMsg::Started) => {}
                Ok(SyncMsg::Progress(phase)) => {
                    self.sync_phase = Some(phase);
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => {
                    self.sync_rx = Some(rx);
                    return;
                }
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    self.syncing = false;
                    self.sync_phase = None;
                    self.maybe_auto_sync();
                    return;
                }
            }
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) {
        if self.show_section_picker {
            handlers::handle_section_picker_key(self, key);
            return;
        }
        if self.show_help {
            match key.code {
                KeyCode::Char('?') | KeyCode::Esc => {
                    self.show_help = false;
                }
                _ => {}
            }
            return;
        }

        match self.focus {
            Focus::Calendar => handlers::handle_calendar_key(self, key),
            Focus::ArticleList => handlers::handle_article_list_key(self, key),
            Focus::ArticleReader => handlers::handle_reader_key(self, key),
            Focus::Search => handlers::handle_search_key(self, key),
        }
    }

    pub fn poll_search(&mut self) {
        if let Some(rx) = &self.search_rx {
            match rx.try_recv() {
                Ok(results) => {
                    self.search_results = results;
                    self.search_selected = 0;
                    self.searching = false;
                    self.search_rx = None;
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => {
                    self.search_spinner = (self.search_spinner + 1) % 10;
                }
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    self.searching = false;
                    self.search_rx = None;
                }
            }
            return;
        }
        if !self.search_pending {
            return;
        }
        if let Some(t) = self.last_search_keystroke
            && t.elapsed().as_millis() < 300
        {
            return;
        }
        self.search_pending = false;
        self.execute_search();
    }

    fn execute_search(&mut self) {
        if self.search_query.is_empty() {
            self.search_results.clear();
            self.search_selected = 0;
            return;
        }
        let query = format!("{}*", self.search_query);
        let db = Arc::clone(&self.db);
        let (tx, rx) = std::sync::mpsc::channel();
        self.search_rx = Some(rx);
        self.searching = true;
        std::thread::spawn(move || {
            let result = db.search_articles(&query);
            let _ = tx.send(result.unwrap_or_default());
        });
    }

    fn open_search_article(&mut self) {
        let article = match self.search_results.get(self.search_selected) {
            Some(a) => a,
            None => return,
        };
        match self.db.get_full_article(article.id) {
            Ok(Some(full)) => {
                let (images, rx) = image_loader::load_images(self, &full);
                self.reader = Some(ArticleReaderState {
                    article: full,
                    images,
                    scroll_offset: 0,
                    image_load_rx: rx,
                });
                self.focus = Focus::ArticleReader;
            }
            Ok(None) => {
                tracing::warn!("Article {} not found in DB", article.id);
            }
            Err(e) => {
                tracing::error!("Failed to load article {}: {e}", article.id);
            }
        }
    }

    fn open_article(&mut self) {
        let filtered = self.filtered_articles();
        let article = match filtered.get(self.article_list_selected) {
            Some(a) => *a,
            None => return,
        };
        match self.db.get_full_article(article.id) {
            Ok(Some(full)) => {
                let (images, rx) = image_loader::load_images(self, &full);
                self.reader = Some(ArticleReaderState {
                    article: full,
                    images,
                    scroll_offset: 0,
                    image_load_rx: rx,
                });
                self.focus = Focus::ArticleReader;
            }
            Ok(None) => {
                tracing::warn!("Article {} not found in DB", article.id);
            }
            Err(e) => {
                tracing::error!("Failed to load article {}: {e}", article.id);
            }
        }
    }

    fn filtered_articles(&self) -> Vec<&Article> {
        match self.section_filter {
            Some(idx) => {
                let section = &self.sections[idx];
                self.articles
                    .iter()
                    .filter(|a| a.section.as_deref() == Some(section.as_str()))
                    .collect()
            }
            None => self.articles.iter().collect(),
        }
    }

    fn trigger_sync(&mut self) {
        if self.syncing {
            return;
        }
        let date = self.selected_date;
        let (tx, rx) = std::sync::mpsc::channel();
        let db = self.db.clone();
        self.sync_rx = Some(rx);
        self.syncing = true;
        self.syncing_date = Some(date);
        self.sync_phase = None;

        let naive_date =
            chrono::NaiveDate::from_ymd_opt(date.year(), date.month() as u32, date.day() as u32)
                .unwrap();

        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            let _ = rt.block_on(crate::sync::sync_single_day_with_progress(
                db, naive_date, tx,
            ));
        });
    }

    fn change_month(&mut self, delta: i32) {
        let date = self.selected_date;
        let (mut year, mut month) = (date.year(), date.month() as i32);
        month += delta;
        while month < 1 {
            month += 12;
            year -= 1;
        }
        while month > 12 {
            month -= 12;
            year += 1;
        }
        year = year.clamp(2005, 2026);
        if let Ok(d) = Date::from_calendar_date(year, Month::try_from(month as u8).unwrap(), 1) {
            let day = date.day().min(days_in_month(year, month as u8));
            if let Ok(new_date) =
                Date::from_calendar_date(year, Month::try_from(month as u8).unwrap(), day)
            {
                self.selected_date = new_date;
                self.refresh_articles();
            } else {
                self.selected_date = d;
                self.refresh_articles();
            }
        }
    }

    fn change_year(&mut self, delta: i32) {
        let date = self.selected_date;
        let year = (date.year() + delta).clamp(2005, 2026);
        let month = date.month() as u8;
        let day = date.day().min(days_in_month(year, month));
        if let Ok(new_date) = Date::from_calendar_date(year, date.month(), day) {
            self.selected_date = new_date;
            self.refresh_articles();
        }
    }

    fn move_day(&mut self, delta: i32) {
        if delta > 0 {
            for _ in 0..delta {
                self.selected_date = self.selected_date.next_day().unwrap_or(self.selected_date);
                if self.selected_date.year() > 2026 {
                    self.selected_date =
                        Date::from_calendar_date(2026, Month::December, 31).unwrap();
                    break;
                }
            }
        } else {
            for _ in 0..delta.abs() {
                self.selected_date = self
                    .selected_date
                    .previous_day()
                    .unwrap_or(self.selected_date);
                if self.selected_date.year() < 2005 {
                    self.selected_date = Date::from_calendar_date(2005, Month::January, 1).unwrap();
                    break;
                }
            }
        }
        self.refresh_articles();
    }

    fn populate_sections(&mut self) {
        let mut sections: Vec<String> = self
            .articles
            .iter()
            .filter_map(|a| a.section.clone())
            .collect();
        sections.sort();
        sections.dedup();
        self.sections = sections;
        self.section_filter = None;
    }

    fn refresh_articles(&mut self) {
        let date_str = self.selected_date.to_string();
        self.articles = self.db.articles_by_date(&date_str).unwrap_or_default();
        self.article_count = self.db.article_count().unwrap_or(0);
        self.article_list_selected = 0;
        self.populate_sections();
    }

    pub(super) fn maybe_auto_sync(&mut self) {
        if self.syncing {
            self.sync_rx = None;
            self.syncing = false;
            self.syncing_date = None;
            self.sync_phase = None;
        }
        if !self.articles.is_empty() {
            return;
        }
        let date_str = self.selected_date.to_string();
        match self.db.get_sync_state(&date_str) {
            Ok(Some(status)) if status == "complete" || status == "failed" => {}
            _ => self.trigger_sync(),
        }
    }
}

fn days_in_month(year: i32, month: u8) -> u8 {
    Date::from_calendar_date(year, Month::try_from(month).unwrap(), 1)
        .map(|d| d.month().length(d.year()))
        .unwrap_or(28)
}
