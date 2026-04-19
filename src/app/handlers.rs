use crate::app::*;
use crossterm::event::{KeyCode, KeyEvent};

pub(super) fn handle_calendar_key(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => {
            app.should_quit = true;
        }
        KeyCode::Char('h') => app.change_month(-1),
        KeyCode::Char('l') => app.change_month(1),
        KeyCode::Char('H') => app.change_year(-1),
        KeyCode::Char('L') => app.change_year(1),
        KeyCode::Char('j') => app.move_day(1),
        KeyCode::Char('k') => app.move_day(-1),
        KeyCode::Char('g') => app.move_day(-365 * 10),
        KeyCode::Char('G') => app.move_day(365 * 10),
        KeyCode::Tab | KeyCode::Enter => {
            app.focus = Focus::ArticleList;
            app.article_list_selected = 0;
            app.maybe_auto_sync();
        }
        KeyCode::Char('?') => {
            app.show_help = true;
        }
        KeyCode::Char('/') => {
            app.focus = Focus::Search;
            app.search_query.clear();
            app.search_results.clear();
            app.search_selected = 0;
        }
        _ => {}
    }
}

#[allow(clippy::collapsible_match)]
pub(super) fn handle_section_picker_key(app: &mut App, key: KeyEvent) {
    let total = app.sections.len() + 1;
    match key.code {
        KeyCode::Esc | KeyCode::Char('f') => {
            app.show_section_picker = false;
        }
        KeyCode::Char('j') | KeyCode::Down => {
            if app.section_picker_selected < total - 1 {
                app.section_picker_selected += 1;
                clamp_picker_scroll(app);
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if app.section_picker_selected > 0 {
                app.section_picker_selected -= 1;
                clamp_picker_scroll(app);
            }
        }
        KeyCode::Enter => {
            if app.section_picker_selected == 0 {
                app.section_filter = None;
            } else {
                app.section_filter = Some(app.section_picker_selected - 1);
            }
            app.article_list_selected = 0;
            app.show_section_picker = false;
        }
        _ => {}
    }
}

fn clamp_picker_scroll(app: &mut App) {
    let visible = 18usize;
    let total = app.sections.len() + 1;
    if total <= visible {
        app.section_picker_scroll = 0;
        return;
    }
    if app.section_picker_selected >= app.section_picker_scroll + visible {
        app.section_picker_scroll = app.section_picker_selected - visible + 1;
    } else if app.section_picker_selected < app.section_picker_scroll {
        app.section_picker_scroll = app.section_picker_selected;
    }
}

#[allow(clippy::collapsible_match)]
pub(super) fn handle_article_list_key(app: &mut App, key: KeyEvent) {
    let filtered = app.filtered_articles();
    let filtered_len = filtered.len();
    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => {
            app.should_quit = true;
        }
        KeyCode::Char('c') => {
            app.focus = Focus::Calendar;
        }
        KeyCode::Char('j') => {
            if filtered_len > 0 {
                app.article_list_selected = (app.article_list_selected + 1).min(filtered_len - 1);
            }
        }
        KeyCode::Char('k') => {
            if filtered_len > 0 {
                app.article_list_selected = app.article_list_selected.saturating_sub(1);
            }
        }
        KeyCode::Char('g') => {
            if key
                .modifiers
                .contains(crossterm::event::KeyModifiers::SHIFT)
            {
                app.article_list_selected = filtered_len.saturating_sub(1);
            } else {
                app.article_list_selected = 0;
            }
        }
        KeyCode::Enter => {
            app.open_article();
        }
        KeyCode::Char('f') => {
            if !app.sections.is_empty() {
                app.section_picker_selected = match app.section_filter {
                    Some(i) => i + 1,
                    None => 0,
                };
                app.section_picker_scroll = 0;
                app.show_section_picker = true;
            }
        }
        KeyCode::Char('F') => {
            app.section_filter = None;
            app.article_list_selected = 0;
        }
        KeyCode::Char('/') => {
            app.focus = Focus::Search;
            app.search_query.clear();
            app.search_results.clear();
            app.search_selected = 0;
        }
        KeyCode::Char('?') => {
            app.show_help = true;
        }
        _ => {}
    }
}

pub(super) fn handle_reader_key(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => {
            app.reader = None;
            app.focus = Focus::ArticleList;
        }
        KeyCode::Char('j') | KeyCode::Down => {
            if let Some(ref mut reader) = app.reader {
                reader.scroll_offset = reader.scroll_offset.saturating_add(1);
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if let Some(ref mut reader) = app.reader {
                reader.scroll_offset = reader.scroll_offset.saturating_sub(1);
            }
        }
        KeyCode::Char('d') => {
            if key
                .modifiers
                .contains(crossterm::event::KeyModifiers::CONTROL)
                && let Some(ref mut reader) = app.reader
            {
                reader.scroll_offset = reader.scroll_offset.saturating_add(10);
            }
        }
        KeyCode::Char('u') => {
            if key
                .modifiers
                .contains(crossterm::event::KeyModifiers::CONTROL)
                && let Some(ref mut reader) = app.reader
            {
                reader.scroll_offset = reader.scroll_offset.saturating_sub(10);
            }
        }
        KeyCode::Char('g') => {
            if let Some(ref mut reader) = app.reader {
                reader.scroll_offset = 0;
            }
        }
        KeyCode::Char('G') => {
            if let Some(ref mut reader) = app.reader {
                reader.scroll_offset = u16::MAX;
            }
        }
        KeyCode::Char('?') => {
            app.show_help = true;
        }
        _ => {}
    }
}

#[allow(clippy::collapsible_match)]
pub(super) fn handle_search_key(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            app.focus = Focus::ArticleList;
            app.search_query.clear();
            app.search_results.clear();
        }
        KeyCode::Enter => {
            if !app.search_results.is_empty() {
                app.open_search_article();
            }
        }
        KeyCode::Down => {
            if !app.search_results.is_empty() {
                app.search_selected = (app.search_selected + 1).min(app.search_results.len() - 1);
            }
        }
        KeyCode::Up => {
            if !app.search_results.is_empty() {
                app.search_selected = app.search_selected.saturating_sub(1);
            }
        }
        KeyCode::Backspace => {
            app.search_query.pop();
            app.last_search_keystroke = Some(std::time::Instant::now());
            app.search_pending = true;
        }
        KeyCode::Char(c) => {
            app.search_query.push(c);
            app.last_search_keystroke = Some(std::time::Instant::now());
            app.search_pending = true;
        }
        _ => {}
    }
}
