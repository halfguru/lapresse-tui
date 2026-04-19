use ratatui::{
    Frame,
    layout::{Margin, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders},
};

use super::{
    ACCENT, ACCENT2, BG, BG_SELECTED, BORDER_FOCUSED, BORDER_UNFOCUSED, TEXT_META, TEXT_PRIMARY,
    month_name,
};
use crate::app::{App, Focus};

pub fn render_calendar(frame: &mut Frame, app: &App, area: Rect) {
    let date = app.selected_date;
    let year = date.year();
    let month = date.month();

    let mut events =
        ratatui::widgets::calendar::CalendarEventStore::today(Style::default().bg(BG_SELECTED));

    if let Ok(counts) = app.db.article_counts_by_month(year, month as u8) {
        let density_style = Style::default().fg(super::BG).bg(ACCENT);
        let high_style = Style::default().fg(super::BG).bg(super::TAG_GREEN);
        for (day, count) in &counts {
            if let Ok(d) = time::Date::from_calendar_date(year, month, *day) {
                let style = if *count >= 10 {
                    high_style
                } else {
                    density_style
                };
                events.add(d, style);
            }
        }
    }

    let selected_style = Style::default()
        .fg(super::BG)
        .bg(ACCENT2)
        .add_modifier(Modifier::BOLD);
    events.add(date, selected_style);

    let border_style = if app.focus == Focus::Calendar {
        Style::default().fg(BORDER_FOCUSED)
    } else {
        Style::default().fg(BORDER_UNFOCUSED)
    };

    let title = format!(" {} {} ", month_name(month as u8), year);

    let calendar = ratatui::widgets::calendar::Monthly::new(date, events)
        .default_style(Style::default().fg(TEXT_PRIMARY).bg(BG))
        .show_month_header(
            Style::default()
                .fg(ACCENT)
                .add_modifier(Modifier::BOLD)
                .bg(BG),
        )
        .show_weekdays_header(Style::default().fg(TEXT_META).bg(BG))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title(title)
                .title_style(Style::default().fg(ACCENT).add_modifier(Modifier::BOLD))
                .border_style(border_style)
                .style(Style::default().bg(BG)),
        );

    frame.render_widget(calendar, area);

    let nav_hint = Line::from(vec![
        Span::styled(" h/l", Style::default().fg(super::TEXT_DIM)),
        Span::styled(":month", Style::default().fg(super::TEXT_DIM)),
        Span::styled(" H/L", Style::default().fg(super::TEXT_DIM)),
        Span::styled(":year", Style::default().fg(super::TEXT_DIM)),
    ]);
    frame.render_widget(
        ratatui::widgets::Paragraph::new(nav_hint),
        area.inner(Margin::new(1, 1)).intersection(Rect::new(
            area.x,
            area.bottom().saturating_sub(2),
            area.width,
            1,
        )),
    );
}
