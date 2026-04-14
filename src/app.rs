use crate::db::{Article, Db, FullArticle};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui_image::picker::{Picker, ProtocolType};
use ratatui_image::protocol::StatefulProtocol;
use std::path::PathBuf;
use std::sync::mpsc::Receiver;
use std::sync::Arc;
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

#[allow(dead_code)]
pub struct ImageState {
    pub protocol: StatefulProtocol,
    pub alt_text: Option<String>,
}

pub struct ArticleReaderState {
    pub article: FullArticle,
    pub images: Vec<Option<ImageState>>,
    pub scroll_offset: u16,
}

#[allow(dead_code)]
pub struct App {
    pub should_quit: bool,
    pub db: Arc<Db>,
    #[allow(dead_code)]
    pub db_path: PathBuf,
    pub picker: Picker,
    pub protocol_type: ProtocolType,
    pub article_count: u32,
    pub selected_date: Date,
    pub focus: Focus,
    pub show_help: bool,
    pub articles: Vec<Article>,
    #[allow(dead_code)]
    pub article_list_offset: usize,
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
}

impl App {
    pub fn new(db: Db, db_path: PathBuf, picker: Picker, protocol_type: ProtocolType) -> anyhow::Result<Self> {
        let db = Arc::new(db);
        let article_count = db.article_count()?;
        let selected_date = OffsetDateTime::now_utc().date();
        let articles = db.articles_by_date(&selected_date.to_string())?;
        let mut app = Self {
            should_quit: false,
            db,
            db_path,
            picker,
            protocol_type,
            article_count,
            selected_date,
            focus: Focus::ArticleList,
            show_help: false,
            article_list_offset: 0,
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
        };
        app.populate_sections();
        if app.article_count == 0 {
            app.trigger_sync();
        }
        Ok(app)
    }

    pub fn poll_sync(&mut self) {
        if self.syncing {
            self.sync_spinner = (self.sync_spinner + 1) % 4;
        }
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
                    return;
                }
                Ok(SyncMsg::Failed) => {
                    self.syncing = false;
                    self.syncing_date = None;
                    self.sync_phase = None;
                    tracing::warn!("On-demand sync failed");
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
                    return;
                }
            }
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) {
        if self.show_section_picker {
            self.handle_section_picker_key(key);
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
            Focus::Calendar => self.handle_calendar_key(key),
            Focus::ArticleList => self.handle_article_list_key(key),
            Focus::ArticleReader => self.handle_reader_key(key),
            Focus::Search => self.handle_search_key(key),
        }
    }

    fn handle_calendar_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => {
                self.should_quit = true;
            }
            KeyCode::Char('h') => self.change_month(-1),
            KeyCode::Char('l') => self.change_month(1),
            KeyCode::Char('H') => self.change_year(-1),
            KeyCode::Char('L') => self.change_year(1),
            KeyCode::Char('j') => self.move_day(1),
            KeyCode::Char('k') => self.move_day(-1),
            KeyCode::Char('g') => self.move_day(-365 * 10),
            KeyCode::Char('G') => self.move_day(365 * 10),
            KeyCode::Tab | KeyCode::Enter => {
                self.focus = Focus::ArticleList;
                self.article_list_selected = 0;
                self.article_list_offset = 0;
            }
            KeyCode::Char('s') => {
                self.trigger_sync();
            }
            KeyCode::Char('?') => {
                self.show_help = true;
            }
            KeyCode::Char('/') => {
                self.focus = Focus::Search;
                self.search_query.clear();
                self.search_results.clear();
                self.search_selected = 0;
            }
            _ => {}
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

    fn handle_section_picker_key(&mut self, key: KeyEvent) {
        let total = self.sections.len() + 1;
        match key.code {
            KeyCode::Esc | KeyCode::Char('f') => {
                self.show_section_picker = false;
            }
            KeyCode::Char('j') | KeyCode::Down => {
                if self.section_picker_selected < total - 1 {
                    self.section_picker_selected += 1;
                    self.clamp_picker_scroll();
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if self.section_picker_selected > 0 {
                    self.section_picker_selected -= 1;
                    self.clamp_picker_scroll();
                }
            }
            KeyCode::Enter => {
                if self.section_picker_selected == 0 {
                    self.section_filter = None;
                } else {
                    self.section_filter = Some(self.section_picker_selected - 1);
                }
                self.article_list_selected = 0;
                self.show_section_picker = false;
            }
            _ => {}
        }
    }

    fn clamp_picker_scroll(&mut self) {
        let visible = 18usize;
        let total = self.sections.len() + 1;
        if total <= visible {
            self.section_picker_scroll = 0;
            return;
        }
        if self.section_picker_selected >= self.section_picker_scroll + visible {
            self.section_picker_scroll = self.section_picker_selected - visible + 1;
        } else if self.section_picker_selected < self.section_picker_scroll {
            self.section_picker_scroll = self.section_picker_selected;
        }
    }

    fn handle_article_list_key(&mut self, key: KeyEvent) {
        let filtered = self.filtered_articles();
        let filtered_len = filtered.len();
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => {
                self.should_quit = true;
            }
            KeyCode::Char('c') => {
                self.focus = Focus::Calendar;
            }
            KeyCode::Char('j') => {
                if filtered_len > 0 {
                    self.article_list_selected =
                        (self.article_list_selected + 1).min(filtered_len - 1);
                }
            }
            KeyCode::Char('k') => {
                if filtered_len > 0 {
                    self.article_list_selected =
                        self.article_list_selected.saturating_sub(1);
                }
            }
            KeyCode::Char('g') => {
                if key.modifiers.contains(crossterm::event::KeyModifiers::SHIFT) {
                    self.article_list_selected = filtered_len.saturating_sub(1);
                } else {
                    self.article_list_selected = 0;
                }
            }
            KeyCode::Enter => {
                self.open_article();
            }
            KeyCode::Char('s') => {
                self.trigger_sync();
            }
            KeyCode::Char('f') => {
                if !self.sections.is_empty() {
                    self.section_picker_selected = match self.section_filter {
                        Some(i) => i + 1,
                        None => 0,
                    };
                    self.section_picker_scroll = 0;
                    self.show_section_picker = true;
                }
            }
            KeyCode::Char('F') => {
                self.section_filter = None;
                self.article_list_selected = 0;
            }
            KeyCode::Char('/') => {
                self.focus = Focus::Search;
                self.search_query.clear();
                self.search_results.clear();
                self.search_selected = 0;
            }
            KeyCode::Char('?') => {
                self.show_help = true;
            }
            _ => {}
        }
    }

    fn handle_reader_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => {
                self.reader = None;
                self.focus = Focus::ArticleList;
            }
            KeyCode::Char('j') | KeyCode::Down => {
                if let Some(ref mut reader) = self.reader {
                    reader.scroll_offset = reader.scroll_offset.saturating_add(1);
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if let Some(ref mut reader) = self.reader {
                    reader.scroll_offset = reader.scroll_offset.saturating_sub(1);
                }
            }
            KeyCode::Char('d') => {
                if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) {
                    if let Some(ref mut reader) = self.reader {
                        reader.scroll_offset = reader.scroll_offset.saturating_add(10);
                    }
                }
            }
            KeyCode::Char('u') => {
                if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) {
                    if let Some(ref mut reader) = self.reader {
                        reader.scroll_offset = reader.scroll_offset.saturating_sub(10);
                    }
                }
            }
            KeyCode::Char('g') => {
                if let Some(ref mut reader) = self.reader {
                    reader.scroll_offset = 0;
                }
            }
            KeyCode::Char('G') => {
                if let Some(ref mut reader) = self.reader {
                    reader.scroll_offset = u16::MAX;
                }
            }
            KeyCode::Char('?') => {
                self.show_help = true;
            }
            _ => {}
        }
    }

    fn handle_search_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.focus = Focus::ArticleList;
                self.search_query.clear();
                self.search_results.clear();
            }
            KeyCode::Enter => {
                if !self.search_results.is_empty() {
                    self.open_search_article();
                }
            }
            KeyCode::Down => {
                if !self.search_results.is_empty() {
                    self.search_selected =
                        (self.search_selected + 1).min(self.search_results.len() - 1);
                }
            }
            KeyCode::Up => {
                if !self.search_results.is_empty() {
                    self.search_selected = self.search_selected.saturating_sub(1);
                }
            }
            KeyCode::Backspace => {
                self.search_query.pop();
                self.execute_search();
            }
            KeyCode::Char(c) => {
                self.search_query.push(c);
                self.execute_search();
            }
            _ => {}
        }
    }

    fn execute_search(&mut self) {
        if self.search_query.is_empty() {
            self.search_results.clear();
            self.search_selected = 0;
            return;
        }
        let query = format!("{}*", self.search_query);
        match self.db.search_articles(&query) {
            Ok(results) => {
                self.search_results = results;
                self.search_selected = 0;
            }
            Err(_) => {
                self.search_results.clear();
            }
        }
    }

    fn open_search_article(&mut self) {
        let article = match self.search_results.get(self.search_selected) {
            Some(a) => a,
            None => return,
        };
        match self.db.get_full_article(article.id) {
            Ok(Some(full)) => {
                let images: Vec<Option<ImageState>> = full
                    .images
                    .iter()
                    .filter_map(|img| {
                        img.data.as_ref().map(|data| {
                            match image::ImageReader::new(std::io::Cursor::new(data))
                                .with_guessed_format()
                            {
                                Ok(reader) => match reader.decode() {
                                    Ok(dyn_img) => {
                                        let protocol = self.picker.new_resize_protocol(dyn_img);
                                        Some(ImageState {
                                            protocol,
                                            alt_text: img.alt_text.clone(),
                                        })
                                    }
                                    Err(e) => {
                                        tracing::warn!("Failed to decode image {}: {e}", img.url);
                                        None
                                    }
                                },
                                Err(e) => {
                                    tracing::warn!("Failed to guess image format {}: {e}", img.url);
                                    None
                                }
                            }
                        })
                    })
                    .collect();

                self.reader = Some(ArticleReaderState {
                    article: full,
                    images,
                    scroll_offset: 0,
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
                let images: Vec<Option<ImageState>> = full
                    .images
                    .iter()
                    .filter_map(|img| {
                        img.data.as_ref().map(|data| {
                            match image::ImageReader::new(std::io::Cursor::new(data))
                                .with_guessed_format()
                            {
                                Ok(reader) => match reader.decode() {
                                    Ok(dyn_img) => {
                                        let protocol = self.picker.new_resize_protocol(dyn_img);
                                        Some(ImageState {
                                            protocol,
                                            alt_text: img.alt_text.clone(),
                                        })
                                    }
                                    Err(e) => {
                                        tracing::warn!("Failed to decode image {}: {e}", img.url);
                                        None
                                    }
                                },
                                Err(e) => {
                                    tracing::warn!("Failed to guess image format {}: {e}", img.url);
                                    None
                                }
                            }
                        })
                    })
                    .collect();

                self.reader = Some(ArticleReaderState {
                    article: full,
                    images,
                    scroll_offset: 0,
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

        let naive_date = chrono::NaiveDate::from_ymd_opt(
            date.year(),
            date.month() as u32,
            date.day() as u32,
        )
        .unwrap();

        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            let _ = rt.block_on(crate::sync::sync_single_day_with_progress(db, naive_date, tx));
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
            if let Ok(new_date) = Date::from_calendar_date(year, Month::try_from(month as u8).unwrap(), day) {
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
                    self.selected_date = Date::from_calendar_date(2026, Month::December, 31).unwrap();
                    break;
                }
            }
        } else {
            for _ in 0..delta.abs() {
                self.selected_date = self.selected_date.previous_day().unwrap_or(self.selected_date);
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
        self.article_list_offset = 0;
        self.populate_sections();
    }
}

fn days_in_month(year: i32, month: u8) -> u8 {
    Date::from_calendar_date(year, Month::try_from(month).unwrap(), 1)
        .map(|d| d.month().length(d.year()))
        .unwrap_or(28)
}
