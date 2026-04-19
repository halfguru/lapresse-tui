use crate::app::*;
use crossterm::event::{KeyCode, KeyEvent, MouseEvent, MouseEventKind};

pub(super) fn handle_mouse(app: &mut App, event: MouseEvent) {
    match event.kind {
        MouseEventKind::ScrollDown => handle_scroll(app, 1),
        MouseEventKind::ScrollUp => handle_scroll(app, -1),
        MouseEventKind::Down(crossterm::event::MouseButton::Left) => {
            handle_click(app, event.column, event.row);
        }
        _ => {}
    }
}

fn handle_scroll(app: &mut App, delta: i32) {
    match app.focus {
        Focus::Calendar => {
            if delta > 0 {
                app.move_day(1);
            } else {
                app.move_day(-1);
            }
        }
        Focus::ArticleList => {
            let filtered = app.filtered_articles();
            let filtered_len = filtered.len();
            if filtered_len == 0 {
                return;
            }
            if delta > 0 {
                app.article_list_selected = (app.article_list_selected + 1).min(filtered_len - 1);
            } else {
                app.article_list_selected = app.article_list_selected.saturating_sub(1);
            }
        }
        Focus::ArticleReader => {
            if let Some(ref mut reader) = app.reader {
                if delta > 0 {
                    reader.scroll_offset = reader.scroll_offset.saturating_add(3);
                } else {
                    reader.scroll_offset = reader.scroll_offset.saturating_sub(3);
                }
            }
        }
        Focus::Search => {
            if app.search_results.is_empty() {
                return;
            }
            if delta > 0 {
                app.search_selected = (app.search_selected + 1).min(app.search_results.len() - 1);
            } else {
                app.search_selected = app.search_selected.saturating_sub(1);
            }
        }
    }
}

fn handle_click(app: &mut App, col: u16, row: u16) {
    if app.show_help {
        app.show_help = false;
        return;
    }
    if app.show_section_picker {
        app.show_section_picker = false;
        return;
    }

    if app.focus == Focus::ArticleReader && app.reader.is_some() {
        return;
    }
    if app.focus == Focus::Search {
        return;
    }

    if let Some(ref areas) = app.layout_areas {
        if col >= areas.calendar.x
            && col < areas.calendar.x + areas.calendar.width
            && row >= areas.calendar.y
            && row < areas.calendar.y + areas.calendar.height
        {
            app.focus = Focus::Calendar;
        } else if col >= areas.article_list.x
            && col < areas.article_list.x + areas.article_list.width
            && row >= areas.article_list.y
            && row < areas.article_list.y + areas.article_list.height
        {
            app.focus = Focus::ArticleList;
            let item_height = 4u16;
            let list_inner_y = row.saturating_sub(areas.article_list.y + 1);
            let clicked_idx = list_inner_y / item_height;
            let filtered = app.filtered_articles();
            if (clicked_idx as usize) < filtered.len() {
                app.article_list_selected = clicked_idx as usize;
            }
        }
    }
}

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
        KeyCode::Char('o') => {
            let filtered = app.filtered_articles();
            if let Some(article) = filtered.get(app.article_list_selected) {
                let url = article.url.clone();
                app.open_url_in_browser(&url);
            }
        }
        KeyCode::Char('y') => {
            let filtered = app.filtered_articles();
            if let Some(article) = filtered.get(app.article_list_selected) {
                let url = article.url.clone();
                app.copy_url_to_clipboard(&url);
            }
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
        KeyCode::Char('o') => {
            if let Some(ref reader) = app.reader {
                let url = reader.article.url.clone();
                app.open_url_in_browser(&url);
            }
        }
        KeyCode::Char('y') => {
            if let Some(ref reader) = app.reader {
                let url = reader.article.url.clone();
                app.copy_url_to_clipboard(&url);
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
