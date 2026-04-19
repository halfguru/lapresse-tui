use crate::app::{App, Focus};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
};

mod article_list;
mod article_reader;
mod calendar;
mod help;
mod search;

pub(crate) use article_list::render_article_list;
pub(crate) use article_reader::render_article_reader;
pub(crate) use calendar::render_calendar;
pub(crate) use help::render_help;
pub(crate) use search::render_search;

pub(crate) const BG: Color = Color::Rgb(22, 27, 44);
pub(crate) const BG_LIGHTER: Color = Color::Rgb(30, 36, 56);
pub(crate) const BG_SELECTED: Color = Color::Rgb(40, 48, 75);
pub(crate) const BORDER_FOCUSED: Color = Color::Rgb(122, 162, 247);
pub(crate) const BORDER_UNFOCUSED: Color = Color::Rgb(52, 59, 88);
pub(crate) const ACCENT: Color = Color::Rgb(122, 162, 247);
pub(crate) const ACCENT2: Color = Color::Rgb(187, 154, 247);
pub(crate) const TEXT_PRIMARY: Color = Color::Rgb(192, 202, 245);
pub(crate) const TEXT_DIM: Color = Color::Rgb(86, 95, 137);
pub(crate) const TEXT_META: Color = Color::Rgb(128, 137, 178);
pub(crate) const HIGHLIGHT_BG: Color = Color::Rgb(55, 64, 105);
pub(crate) const TAG_GREEN: Color = Color::Rgb(77, 166, 101);
pub(crate) const TAG_YELLOW: Color = Color::Rgb(210, 178, 68);

pub(crate) const SECTION_COLORS: [Color; 8] = [
    Color::Rgb(122, 162, 247),
    Color::Rgb(187, 154, 247),
    Color::Rgb(77, 166, 101),
    Color::Rgb(210, 178, 68),
    Color::Rgb(224, 92, 104),
    Color::Rgb(68, 186, 186),
    Color::Rgb(232, 130, 77),
    Color::Rgb(160, 120, 232),
];

pub(crate) fn section_color(section: &str) -> Color {
    let hash = section.bytes().fold(0usize, |acc, b| {
        acc.wrapping_mul(31).wrapping_add(b as usize)
    });
    SECTION_COLORS[hash % SECTION_COLORS.len()]
}

fn centered_rect(width: u16, height: u16, r: Rect) -> Rect {
    let x = r.x + r.width.saturating_sub(width) / 2;
    let y = r.y + r.height.saturating_sub(height) / 2;
    Rect::new(x, y, width.min(r.width), height.min(r.height))
}

pub fn render(frame: &mut Frame, app: &mut App) {
    frame.render_widget(
        Block::default().style(Style::default().bg(BG)),
        frame.area(),
    );

    let area = frame.area();
    if area.width < 80 || area.height < 24 {
        let msg = format!(
            "Terminal too small: {}x{}\nMinimum: 80x24",
            area.width, area.height
        );
        let text = Paragraph::new(msg)
            .style(Style::default().fg(TEXT_PRIMARY).bg(BG))
            .alignment(ratatui::layout::Alignment::Center);
        frame.render_widget(text, area);
        return;
    }

    if app.focus == Focus::ArticleReader && app.reader.is_some() {
        render_article_reader(frame, app);
        if app.show_help {
            render_help(frame);
        }
        return;
    }

    if app.focus == Focus::Search {
        render_search(frame, app);
        if app.show_help {
            render_help(frame);
        }
        return;
    }

    let [header_area, main_area, status_area] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Fill(1),
        Constraint::Length(1),
    ])
    .areas(frame.area());

    let [cal_area, list_area] =
        Layout::horizontal([Constraint::Percentage(20), Constraint::Percentage(80)])
            .areas(main_area);

    render_header(frame, app, header_area);

    let cal_widget_width = 28u16.min(cal_area.width);
    let cal_centered = Rect {
        x: cal_area.x + cal_area.width.saturating_sub(cal_widget_width) / 2,
        y: cal_area.y,
        width: cal_widget_width,
        height: cal_area.height,
    };
    render_calendar(frame, app, cal_centered);

    app.layout_areas = Some(crate::app::LayoutAreas {
        calendar: cal_centered,
        article_list: list_area,
    });

    render_article_list(frame, app, list_area);
    render_status(frame, app, status_area);

    if app.show_section_picker {
        render_section_picker(frame, app);
    }
    if app.show_help {
        render_help(frame);
    }
}

fn render_header(frame: &mut Frame, app: &App, area: Rect) {
    let date_str = app.selected_date.to_string();

    let breadcrumb = match app.focus {
        Focus::Calendar => format!(" > {date_str} > Calendar"),
        Focus::ArticleList => {
            let count = app.articles.len();
            format!(" > {date_str} > Articles ({count})")
        }
        Focus::ArticleReader => {
            if let Some(ref reader) = app.reader {
                let title = truncate_str(&reader.article.title, 30);
                format!(" > {date_str} > {title}")
            } else {
                format!(" > {date_str} > Reading")
            }
        }
        Focus::Search => {
            format!(" > Search > \"{}\"", app.search_query)
        }
    };

    let sync_indicator = if app.syncing {
        let spinner = match app.sync_spinner {
            0 => "⣾",
            1 => "⣽",
            2 => "⣻",
            3 => "⢿",
            4 => "⡿",
            5 => "⣟",
            6 => "⣯",
            _ => "⣷",
        };
        let label = if let Some(phase) = &app.sync_phase {
            if phase.total > 0 {
                let filled = (phase.current as usize * 8 / phase.total as usize).min(8);
                let bar: String = "█".repeat(filled) + &"░".repeat(8 - filled);
                format!(
                    " {spinner} {}: {}/{} {bar} ",
                    phase.phase, phase.current, phase.total
                )
            } else {
                format!(" {spinner} {}... ", phase.phase)
            }
        } else {
            format!(" {spinner} SYNCING ")
        };
        vec![
            Span::raw(" "),
            Span::styled(
                label,
                Style::default().fg(Color::Rgb(22, 27, 44)).bg(TAG_YELLOW),
            ),
        ]
    } else {
        vec![]
    };

    let clipboard_msg = app
        .last_clipboard_msg
        .as_deref()
        .map(|msg| {
            vec![
                Span::raw(" "),
                Span::styled(
                    format!(" {msg} "),
                    Style::default().fg(Color::Rgb(22, 27, 44)).bg(TAG_GREEN),
                ),
            ]
        })
        .unwrap_or_default();

    let mut header_spans = vec![
        Span::styled(
            " lapresse-tui ",
            Style::default()
                .fg(Color::Rgb(22, 27, 44))
                .bg(ACCENT)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(breadcrumb, Style::default().fg(TEXT_PRIMARY).bg(BG_LIGHTER)),
        Span::raw(" "),
        Span::styled(
            format!(" {} articles ", app.article_count),
            Style::default().fg(Color::Rgb(22, 27, 44)).bg(TAG_GREEN),
        ),
        Span::raw(" "),
        Span::styled(
            format!(" {:?}", app.protocol_type),
            Style::default().fg(TEXT_DIM),
        ),
    ];
    header_spans.extend(sync_indicator);
    header_spans.extend(clipboard_msg);

    let header = Line::from(header_spans);

    frame.render_widget(header, area);
}

fn truncate_str(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let end = s
            .char_indices()
            .take(max - 1)
            .last()
            .map(|(i, _)| i)
            .unwrap_or(0);
        format!("{}…", &s[..end])
    }
}

fn render_section_picker(frame: &mut Frame, app: &App) {
    let max_visible = 18u16;
    let max_width = 36u16;
    let total_items = app.sections.len() as u16 + 1;
    let visible_count = total_items.min(max_visible);
    let height = visible_count + 2;
    let area = centered_rect(max_width, height, frame.area());

    frame.render_widget(Clear, area);

    let scroll = app.section_picker_scroll.min(app.sections.len());
    let visible_end = (scroll + visible_count as usize).min(app.sections.len() + 1);

    let mut lines: Vec<Line> = Vec::new();

    if scroll == 0 {
        let all_style = if app.section_picker_selected == 0 {
            Style::default()
                .fg(Color::Rgb(255, 255, 255))
                .bg(HIGHLIGHT_BG)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(TEXT_PRIMARY)
        };
        lines.push(Line::from(Span::styled(
            if app.section_filter.is_none() {
                "  ● All sections"
            } else {
                "  ○ All sections"
            },
            all_style,
        )));
    }

    let section_start = if scroll == 0 { 0 } else { scroll - 1 };
    let section_end = visible_end.saturating_sub(1);

    for i in section_start..section_end.min(app.sections.len()) {
        let section = &app.sections[i];
        let is_selected = app.section_picker_selected == i + 1;
        let is_active = app.section_filter == Some(i);
        let color = section_color(section);
        let row_style = if is_selected {
            Style::default()
                .fg(Color::Rgb(255, 255, 255))
                .bg(HIGHLIGHT_BG)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(TEXT_PRIMARY)
        };

        let bullet = if is_active { "  ● " } else { "  ○ " };
        let mut spans = vec![Span::styled(bullet, row_style)];
        spans.push(Span::styled(
            format!(" {} ", section),
            Style::default().fg(Color::Rgb(22, 27, 44)).bg(color),
        ));
        lines.push(Line::from(spans));
    }

    let scroll_indicator = if total_items > visible_count {
        let pct = (scroll as u32 * 100 / (total_items - visible_count) as u32).min(100);
        format!(" {pct}% ")
    } else {
        String::new()
    };

    let paragraph = Paragraph::new(lines)
        .style(Style::default().bg(BG))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(BORDER_FOCUSED))
                .title(" Sections ")
                .title_style(Style::default().fg(ACCENT))
                .style(Style::default().bg(BG)),
        )
        .scroll((0, 0));

    frame.render_widget(paragraph, area);

    if !scroll_indicator.is_empty() {
        let indicator_area = Rect::new(
            area.x + area.width.saturating_sub(scroll_indicator.len() as u16 + 2),
            area.y,
            scroll_indicator.len() as u16 + 1,
            1,
        );
        frame.render_widget(
            Paragraph::new(Span::styled(
                scroll_indicator,
                Style::default().fg(TEXT_DIM),
            )),
            indicator_area,
        );
    }
}

fn render_status(frame: &mut Frame, _app: &App, area: Rect) {
    let help_text = " ?:help c:cal f:filter /:search o:open y:copy q:quit ";
    let status = Line::from(Span::styled(
        help_text,
        Style::default().fg(TEXT_DIM).bg(BG),
    ));

    frame.render_widget(status, area);
}

pub(crate) fn month_name(month: u8) -> &'static str {
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

pub(crate) fn format_scroll_indicator(pct: u32, scrollable: bool) -> String {
    if !scrollable {
        return String::new();
    }
    let filled = (pct as usize * 10 / 100).min(10);
    let empty = 10 - filled;
    let bar: String = "█".repeat(filled) + &"░".repeat(empty);
    format!(" {pct}% {bar}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ui_month_name() {
        assert_eq!(month_name(1), "January");
        assert_eq!(month_name(12), "December");
        assert_eq!(month_name(13), "???");
    }

    #[test]
    fn ui_format_scroll_indicator() {
        assert_eq!(format_scroll_indicator(0, true), " 0% ░░░░░░░░░░");
        assert_eq!(format_scroll_indicator(50, true), " 50% █████░░░░░");
        assert_eq!(format_scroll_indicator(100, true), " 100% ██████████");
        assert!(format_scroll_indicator(50, false).is_empty());
    }
}
