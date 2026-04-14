use crate::app::{App, Focus};
use ratatui::{
    layout::{Constraint, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, Borders, Clear, HighlightSpacing, List, ListItem, ListState, Paragraph, Wrap,
    },
    Frame,
};
use ratatui::widgets::calendar::{CalendarEventStore, Monthly};
use time::Date;

const SECTION_COLORS: [Color; 8] = [
    Color::Cyan,
    Color::Magenta,
    Color::Yellow,
    Color::Green,
    Color::Blue,
    Color::Red,
    Color::LightCyan,
    Color::LightMagenta,
];

fn section_color(section: &str) -> Color {
    let hash = section
        .bytes()
        .fold(0usize, |acc, b| acc.wrapping_mul(31).wrapping_add(b as usize));
    SECTION_COLORS[hash % SECTION_COLORS.len()]
}

pub fn render(frame: &mut Frame, app: &App) {
    let [main_area, status_area] =
        Layout::vertical([Constraint::Fill(1), Constraint::Length(1)]).areas(frame.area());

    let [cal_area, list_area] =
        Layout::horizontal([Constraint::Percentage(35), Constraint::Percentage(65)])
            .areas(main_area);

    render_calendar(frame, app, cal_area);
    render_article_list(frame, app, list_area);
    render_status(frame, app, status_area);

    if app.show_help {
        render_help(frame);
    }
}

fn render_calendar(frame: &mut Frame, app: &App, area: Rect) {
    let date = app.selected_date;
    let year = date.year();
    let month = date.month();

    let mut events = CalendarEventStore::today(Style::default().bg(Color::DarkGray));

    if let Ok(counts) = app.db.article_counts_by_month(year, month as u8) {
        let density_style = Style::default().fg(Color::Black).bg(Color::Cyan);
        let high_style = Style::default().fg(Color::Black).bg(Color::Green);
        for (day, count) in &counts {
            if let Ok(d) = Date::from_calendar_date(year, month, *day) {
                let style = if *count >= 10 { high_style } else { density_style };
                events.add(d, style);
            }
        }
    }

    let selected_style = Style::default()
        .fg(Color::White)
        .bg(Color::Red)
        .add_modifier(Modifier::BOLD);
    events.add(date, selected_style);

    let border_style = if app.focus == Focus::Calendar {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let title = format!(" {} {} ", month_name(month as u8), year);

    let calendar = Monthly::new(date, events)
        .default_style(Style::default().fg(Color::White))
        .show_month_header(Style::default().add_modifier(Modifier::BOLD))
        .show_weekdays_header(Style::default().fg(Color::Yellow))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .border_style(border_style),
        );

    frame.render_widget(calendar, area);

    let nav_hint = Line::from(vec![
        Span::styled(" h/l", Style::default().fg(Color::DarkGray)),
        Span::styled(":month", Style::default().fg(Color::DarkGray)),
        Span::styled(" H/L", Style::default().fg(Color::DarkGray)),
        Span::styled(":year", Style::default().fg(Color::DarkGray)),
    ]);
    frame.render_widget(
        Paragraph::new(nav_hint),
        area.inner(Margin::new(1, 1)).intersection(Rect::new(
            area.x,
            area.bottom().saturating_sub(2),
            area.width,
            1,
        )),
    );
}

fn render_article_list(frame: &mut Frame, app: &App, area: Rect) {
    let border_style = if app.focus == Focus::ArticleList {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let date_str = app.selected_date.to_string();

    if app.articles.is_empty() {
        let block = Block::default()
            .borders(Borders::ALL)
            .title(format!(" {date_str} "))
            .border_style(border_style);

        let msg = if app.article_count == 0 {
            "Run sync to fetch articles"
        } else {
            "No articles for this date"
        };

        let paragraph = Paragraph::new(Line::from(Span::styled(
            msg,
            Style::default().fg(Color::DarkGray),
        )))
        .block(block)
        .wrap(Wrap { trim: true });

        frame.render_widget(paragraph, area);
        return;
    }

    let items: Vec<ListItem> = app
        .articles
        .iter()
        .map(|article| {
            let section_span = article.section.as_ref().map_or_else(
                Span::default,
                |s| {
                    Span::styled(
                        format!(" {s} "),
                        Style::default().fg(Color::Black).bg(section_color(s)),
                    )
                },
            );

            let title = Span::styled(&article.title, Style::default().fg(Color::White));

            ListItem::new(Line::from(vec![title, Span::raw("  "), section_span]))
        })
        .collect();

    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!(
            " {date_str} · {} articles ",
            app.articles.len()
        ))
        .border_style(border_style);

    let list = List::new(items)
        .block(block)
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_spacing(HighlightSpacing::Always)
        .highlight_symbol(">> ");

    let mut state = ListState::default();
    state.select(Some(app.article_list_selected));

    frame.render_stateful_widget(list, area, &mut state);
}

fn render_status(frame: &mut Frame, app: &App, area: Rect) {
    let protocol_label = format!("{:?}", app.protocol_type);
    let date_str = app.selected_date.to_string();

    let focus_label = match app.focus {
        Focus::Calendar => "CALENDAR",
        Focus::ArticleList => "ARTICLES",
    };

    let status = Line::from(vec![
        Span::styled(
            format!(" {date_str} "),
            Style::default().fg(Color::Black).bg(Color::White),
        ),
        Span::raw(" "),
        Span::styled(
            format!(" {focus_label} "),
            Style::default().fg(Color::Black).bg(Color::Cyan),
        ),
        Span::raw(" "),
        Span::styled(
            format!(" {} articles ", app.article_count),
            Style::default().fg(Color::Black).bg(Color::Green),
        ),
        Span::raw(" "),
        Span::styled(
            format!(" {protocol_label} "),
            Style::default().fg(Color::Black).bg(Color::Yellow),
        ),
        Span::raw("  "),
        Span::styled(
            " ?:help q:quit ",
            Style::default().fg(Color::DarkGray),
        ),
    ]);

    frame.render_widget(status, area);
}

fn render_help(frame: &mut Frame) {
    let area = centered_rect(48, 18, frame.area());

    frame.render_widget(Clear, area);

    let lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            "  Keybindings",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  Navigation",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from("  h/l    prev/next month"),
        Line::from("  H/L    prev/next year"),
        Line::from("  j/k    move day (calendar) or scroll (list)"),
        Line::from("  g/G    jump to top/bottom"),
        Line::from(""),
        Line::from(Span::styled(
            "  Actions",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from("  Tab    switch to article list"),
        Line::from("  Enter  switch to article list"),
        Line::from("  q/Esc  back / quit"),
        Line::from(""),
        Line::from(Span::styled(
            "  Views",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from("  ?      toggle this help"),
    ];

    let paragraph = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan))
            .title(" Help ")
            .title_style(Style::default().fg(Color::Cyan)),
    );

    frame.render_widget(paragraph, area);
}

fn centered_rect(width: u16, height: u16, r: Rect) -> Rect {
    let x = r.x + r.width.saturating_sub(width) / 2;
    let y = r.y + r.height.saturating_sub(height) / 2;
    Rect::new(x, y, width.min(r.width), height.min(r.height))
}

fn month_name(month: u8) -> &'static str {
    match month {
        1 => "January",
        2 => "February",
        3 => "March",
        4 => "April",
        5 => "May",
        6 => "June",
        7 => "July",
        8 => "August",
        9 => "September",
        10 => "October",
        11 => "November",
        12 => "December",
        _ => "???",
    }
}
