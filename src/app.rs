use crate::db::{Article, Db};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui_image::picker::ProtocolType;
use std::path::PathBuf;
use time::{Date, Month, OffsetDateTime};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Focus {
    Calendar,
    ArticleList,
}

#[allow(dead_code)]
pub struct App {
    pub should_quit: bool,
    pub db: Db,
    #[allow(dead_code)]
    pub db_path: PathBuf,
    pub protocol_type: ProtocolType,
    pub article_count: u32,
    pub selected_date: Date,
    pub focus: Focus,
    pub show_help: bool,
    pub articles: Vec<Article>,
    #[allow(dead_code)]
    pub article_list_offset: usize,
    pub article_list_selected: usize,
}

impl App {
    pub fn new(db: Db, db_path: PathBuf, protocol_type: ProtocolType) -> anyhow::Result<Self> {
        let article_count = db.article_count()?;
        let selected_date = OffsetDateTime::now_utc().date();
        let articles = db.articles_by_date(&selected_date.to_string())?;
        Ok(Self {
            should_quit: false,
            db,
            db_path,
            protocol_type,
            article_count,
            selected_date,
            focus: Focus::Calendar,
            show_help: false,
            article_list_offset: 0,
            article_list_selected: 0,
            articles,
        })
    }

    pub fn handle_key(&mut self, key: KeyEvent) {
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
            KeyCode::Char('?') => {
                self.show_help = true;
            }
            _ => {}
        }
    }

    fn handle_article_list_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => {
                self.focus = Focus::Calendar;
            }
            KeyCode::Char('j') => {
                if !self.articles.is_empty() {
                    self.article_list_selected =
                        (self.article_list_selected + 1).min(self.articles.len() - 1);
                }
            }
            KeyCode::Char('k') => {
                if !self.articles.is_empty() {
                    self.article_list_selected =
                        self.article_list_selected.saturating_sub(1);
                }
            }
            KeyCode::Char('g') => {
                if key.modifiers.contains(crossterm::event::KeyModifiers::SHIFT) {
                    self.article_list_selected = self.articles.len().saturating_sub(1);
                } else {
                    self.article_list_selected = 0;
                }
            }
            KeyCode::Char('?') => {
                self.show_help = true;
            }
            _ => {}
        }
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

    fn refresh_articles(&mut self) {
        let date_str = self.selected_date.to_string();
        self.articles = self.db.articles_by_date(&date_str).unwrap_or_default();
        self.article_count = self.db.article_count().unwrap_or(0);
        self.article_list_selected = 0;
        self.article_list_offset = 0;
    }
}

fn days_in_month(year: i32, month: u8) -> u8 {
    Date::from_calendar_date(year, Month::try_from(month).unwrap(), 1)
        .map(|d| d.month().length(d.year()))
        .unwrap_or(28)
}
