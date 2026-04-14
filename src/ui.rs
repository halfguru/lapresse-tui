use crate::app::{App, Focus};
use ratatui::widgets::calendar::{CalendarEventStore, Monthly};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, BorderType, Borders, Clear, HighlightSpacing, List, ListItem, ListState, Paragraph,
        Wrap,
    },
};
use ratatui_image::StatefulImage;
use time::Date;

const BG: Color = Color::Rgb(22, 27, 44);
const BG_LIGHTER: Color = Color::Rgb(30, 36, 56);
const BG_SELECTED: Color = Color::Rgb(40, 48, 75);
const BORDER_FOCUSED: Color = Color::Rgb(122, 162, 247);
const BORDER_UNFOCUSED: Color = Color::Rgb(52, 59, 88);
const ACCENT: Color = Color::Rgb(122, 162, 247);
const ACCENT2: Color = Color::Rgb(187, 154, 247);
const TEXT_PRIMARY: Color = Color::Rgb(192, 202, 245);
const TEXT_DIM: Color = Color::Rgb(86, 95, 137);
const TEXT_META: Color = Color::Rgb(128, 137, 178);
const HIGHLIGHT_BG: Color = Color::Rgb(55, 64, 105);
const TAG_GREEN: Color = Color::Rgb(77, 166, 101);
const TAG_YELLOW: Color = Color::Rgb(210, 178, 68);

const SECTION_COLORS: [Color; 8] = [
    Color::Rgb(122, 162, 247),
    Color::Rgb(187, 154, 247),
    Color::Rgb(77, 166, 101),
    Color::Rgb(210, 178, 68),
    Color::Rgb(224, 92, 104),
    Color::Rgb(68, 186, 186),
    Color::Rgb(232, 130, 77),
    Color::Rgb(160, 120, 232),
];

fn section_color(section: &str) -> Color {
    let hash = section.bytes().fold(0usize, |acc, b| {
        acc.wrapping_mul(31).wrapping_add(b as usize)
    });
    SECTION_COLORS[hash % SECTION_COLORS.len()]
}

pub fn render(frame: &mut Frame, app: &mut App) {
    frame.render_widget(
        Block::default().style(Style::default().bg(BG)),
        frame.area(),
    );

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
    let focus_label = match app.focus {
        Focus::Calendar => "CALENDAR",
        Focus::ArticleList => "ARTICLES",
        Focus::ArticleReader => "READING",
        Focus::Search => "SEARCH",
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
                format!(" {spinner} {}: {}/{} ", phase.phase, phase.current, phase.total)
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

    let mut header_spans = vec![
        Span::styled(
            " lapresse-tui ",
            Style::default()
                .fg(Color::Rgb(22, 27, 44))
                .bg(ACCENT)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!(" {date_str} "),
            Style::default().fg(TEXT_PRIMARY).bg(BG_LIGHTER),
        ),
        Span::raw(" "),
        Span::styled(
            format!(" {focus_label} "),
            Style::default().fg(Color::Rgb(22, 27, 44)).bg(ACCENT2),
        ),
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

    let header = Line::from(header_spans);

    frame.render_widget(header, area);
}

fn render_calendar(frame: &mut Frame, app: &App, area: Rect) {
    let date = app.selected_date;
    let year = date.year();
    let month = date.month();

    let mut events = CalendarEventStore::today(Style::default().bg(BG_SELECTED));

    if let Ok(counts) = app.db.article_counts_by_month(year, month as u8) {
        let density_style = Style::default().fg(Color::Rgb(22, 27, 44)).bg(ACCENT);
        let high_style = Style::default().fg(Color::Rgb(22, 27, 44)).bg(TAG_GREEN);
        for (day, count) in &counts {
            if let Ok(d) = Date::from_calendar_date(year, month, *day) {
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
        .fg(Color::Rgb(22, 27, 44))
        .bg(ACCENT2)
        .add_modifier(Modifier::BOLD);
    events.add(date, selected_style);

    let border_style = if app.focus == Focus::Calendar {
        Style::default().fg(BORDER_FOCUSED)
    } else {
        Style::default().fg(BORDER_UNFOCUSED)
    };

    let title = format!(" {} {} ", month_name(month as u8), year);

    let calendar = Monthly::new(date, events)
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
        Span::styled(" h/l", Style::default().fg(TEXT_DIM)),
        Span::styled(":month", Style::default().fg(TEXT_DIM)),
        Span::styled(" H/L", Style::default().fg(TEXT_DIM)),
        Span::styled(":year", Style::default().fg(TEXT_DIM)),
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
        Style::default().fg(BORDER_FOCUSED)
    } else {
        Style::default().fg(BORDER_UNFOCUSED)
    };

    let date_str = app.selected_date.to_string();

    let filtered: Vec<&crate::db::Article> = match app.section_filter {
        Some(idx) => {
            let section = &app.sections[idx];
            app.articles
                .iter()
                .filter(|a| a.section.as_deref() == Some(section.as_str()))
                .collect()
        }
        None => app.articles.iter().collect(),
    };

    let filter_label = match app.section_filter {
        Some(idx) => {
            let section = &app.sections[idx];
            let color = section_color(section);
            Some(Span::styled(
                format!(" {} ", section),
                Style::default().fg(Color::Rgb(22, 27, 44)).bg(color),
            ))
        }
        None => None,
    };

    if filtered.is_empty() && app.articles.is_empty() {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .title(format!(" {date_str} "))
            .title_style(Style::default().fg(TEXT_PRIMARY))
            .border_style(border_style)
            .style(Style::default().bg(BG));

        let msg = if app.syncing {
            let spinner = match app.sync_spinner {
                0 => "⣾",
                1 => "⣽",
                2 => "⣻",
                _ => "⣷",
            };
            let sync_date = app
                .syncing_date
                .map(|d| d.to_string())
                .unwrap_or_default();
            if let Some(phase) = &app.sync_phase {
                if phase.total > 0 {
                    format!(" {spinner} {} — {}/{} for {sync_date}", phase.phase, phase.current, phase.total)
                } else {
                    format!(" {spinner} {} for {sync_date}...", phase.phase)
                }
            } else {
                format!(" {spinner} Syncing {sync_date} from lapresse.ca...")
            }
        } else if app.article_count == 0 {
            "No articles cached. Press 's' to sync this day.".to_string()
        } else {
            "No articles for this date. Press 's' to sync.".to_string()
        };

        let style = if app.syncing {
            Style::default().fg(TAG_YELLOW)
        } else {
            Style::default().fg(TEXT_DIM)
        };

        let paragraph = Paragraph::new(Line::from(Span::styled(msg, style)))
            .block(block)
            .wrap(Wrap { trim: true });

        frame.render_widget(paragraph, area);
        return;
    }

    if filtered.is_empty() {
        let mut title_parts = vec![Span::styled(
            format!(" {date_str} "),
            Style::default().fg(TEXT_PRIMARY),
        )];
        if let Some(label) = filter_label {
            title_parts.push(label);
        }
        title_parts.push(Span::styled(" 0 matching ", Style::default().fg(TEXT_DIM)));

        let msg = Line::from(Span::styled(
            " No articles in this section. Press 'f' to cycle, 'F' to clear.",
            Style::default().fg(TEXT_DIM),
        ));

        let paragraph = Paragraph::new(msg)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .title(Line::from(title_parts))
                    .border_style(border_style)
                    .style(Style::default().bg(BG)),
            )
            .wrap(Wrap { trim: true });

        frame.render_widget(paragraph, area);
        return;
    }

    let items: Vec<ListItem> = filtered
        .iter()
        .enumerate()
        .map(|(i, article)| {
            let section_span = article.section.as_ref().map_or_else(Span::default, |s| {
                Span::styled(
                    format!(" {s} "),
                    Style::default()
                        .fg(Color::Rgb(22, 27, 44))
                        .bg(section_color(s)),
                )
            });

            let row_bg = if i % 2 == 0 { BG } else { BG_LIGHTER };
            let title = Span::styled(&article.title, Style::default().fg(TEXT_PRIMARY).bg(row_bg));

            let mut spans = vec![title, Span::raw("  "), section_span];
            if i % 2 == 1 {
                let pad = Span::styled("", Style::default().bg(BG_LIGHTER));
                spans.push(pad);
            }

            ListItem::new(Line::from(spans)).style(Style::default().bg(row_bg))
        })
        .collect();

    let title_text = if app.section_filter.is_some() {
        format!(" {date_str} · {}/{} ", filtered.len(), app.articles.len())
    } else {
        format!(" {date_str} · {} articles ", filtered.len())
    };

    let mut title_spans = vec![Span::styled(title_text, Style::default().fg(TEXT_PRIMARY))];
    if let Some(label) = filter_label {
        title_spans.push(label);
    }

    let scroll_down = if filtered.len() > (area.height as usize).saturating_sub(2) {
        let visible = area.height as usize - 2;
        let max_top = filtered.len().saturating_sub(visible);
        if app.article_list_selected < max_top {
            Some(Span::styled(" ▼ ", Style::default().fg(TEXT_DIM)))
        } else {
            None
        }
    } else {
        None
    };

    if let Some(scroll) = scroll_down {
        title_spans.push(scroll);
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(Line::from(title_spans))
        .title_style(Style::default().fg(ACCENT))
        .border_style(border_style)
        .style(Style::default().bg(BG));

    let list = List::new(items)
        .block(block)
        .highlight_style(
            Style::default()
                .fg(Color::Rgb(255, 255, 255))
                .bg(HIGHLIGHT_BG)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_spacing(HighlightSpacing::Always)
        .highlight_symbol(" ▶ ");

    let mut state = ListState::default();
    state.select(Some(app.article_list_selected));

    frame.render_stateful_widget(list, area, &mut state);
}

enum ContentBlock {
    Text(Vec<String>, TextStyle),
    Image(usize),
}

#[derive(Clone, Copy)]
enum TextStyle {
    Normal,
    Meta,
    ImageSeparator,
    Title,
    HorizontalRule,
}

fn build_content_blocks(article: &crate::db::FullArticle, width: u16) -> Vec<ContentBlock> {
    let mut blocks = Vec::new();

    blocks.push(ContentBlock::Text(vec![String::new()], TextStyle::Normal));

    let title_lines = wrap_centered(&article.title, width);
    blocks.push(ContentBlock::Text(title_lines, TextStyle::Title));

    blocks.push(ContentBlock::Text(vec![String::new()], TextStyle::Normal));

    let mut meta_line = String::new();
    if let Some(ref section) = article.section {
        meta_line.push_str(&format!("■ {section}"));
    }
    if let Some(ref author) = article.author {
        if !meta_line.is_empty() {
            meta_line.push_str("   ");
        }
        meta_line.push_str(&format!("✎ {author}"));
    }
    if !meta_line.is_empty() {
        meta_line.push_str("   ");
    }
    meta_line.push_str(&format!("◷ {}", &article.published_at[..10]));
    blocks.push(ContentBlock::Text(vec![meta_line], TextStyle::Meta));

    blocks.push(ContentBlock::Text(
        vec!["─".repeat(width as usize)],
        TextStyle::HorizontalRule,
    ));

    if let Some(ref text) = article.content_text {
        for paragraph in text.split("\n\n") {
            let lines: Vec<String> = paragraph.lines().map(|l| l.to_string()).collect();
            if !lines.is_empty() {
                blocks.push(ContentBlock::Text(lines, TextStyle::Normal));
                blocks.push(ContentBlock::Text(vec![String::new()], TextStyle::Normal));
            }
        }
    } else {
        blocks.push(ContentBlock::Text(
            vec!["No article text available.".to_string()],
            TextStyle::Normal,
        ));
    }

    if !article.images.is_empty() {
        for (i, _img) in article.images.iter().enumerate() {
            blocks.push(ContentBlock::Text(
                vec![format!("──── Image {} ────", i + 1)],
                TextStyle::ImageSeparator,
            ));
            blocks.push(ContentBlock::Image(i));
            blocks.push(ContentBlock::Text(vec![String::new()], TextStyle::Normal));
        }
    }

    blocks
}

fn wrap_centered(text: &str, width: u16) -> Vec<String> {
    let w = width as usize;
    if w == 0 {
        return vec![text.to_string()];
    }
    let mut result = Vec::new();
    let chars: Vec<char> = text.chars().collect();
    let mut pos = 0;
    while pos < chars.len() {
        let end = (pos + w).min(chars.len());
        let line: String = chars[pos..end].iter().collect();
        let padding = w.saturating_sub(line.chars().count()) / 2;
        result.push(format!("{}{}", " ".repeat(padding), line));
        pos = end;
    }
    result
}

fn block_height(block: &ContentBlock, area_width: u16) -> u16 {
    match block {
        ContentBlock::Text(lines, _) => {
            let mut count = 0u16;
            for line in lines {
                if area_width > 0 {
                    count += (line.len() as u16 + area_width - 1) / area_width;
                } else {
                    count += 1;
                }
            }
            count.max(1)
        }
        ContentBlock::Image(_) => 12,
    }
}

struct VisibleBlock {
    screen_y: u16,
    height: u16,
    content: VisibleContent,
}

enum VisibleContent {
    Text {
        lines: Vec<(u16, String, TextStyle)>,
    },
    Image {
        idx: usize,
    },
}

fn render_article_reader(frame: &mut Frame, app: &mut App) {
    let reader = match app.reader.as_mut() {
        Some(r) => r,
        None => return,
    };

    let article = &reader.article;
    let area = frame.area();

    let [content_area, footer_area] =
        Layout::vertical([Constraint::Fill(1), Constraint::Length(1)]).areas(area);

    let inner = Rect {
        x: content_area.x + 2,
        y: content_area.y + 1,
        width: content_area.width.saturating_sub(4),
        height: content_area.height.saturating_sub(2),
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(BORDER_UNFOCUSED))
        .style(Style::default().bg(BG));

    frame.render_widget(block, content_area);

    let blocks = build_content_blocks(article, inner.width);
    let text_width = inner.width;

    let total_height: u16 = blocks.iter().map(|b| block_height(b, text_width)).sum();
    let max_scroll = total_height.saturating_sub(inner.height);
    if reader.scroll_offset > max_scroll {
        reader.scroll_offset = max_scroll;
    }

    let scroll = reader.scroll_offset;
    let mut cursor_y: u16 = 0;
    let mut visible: Vec<VisibleBlock> = Vec::new();

    for content_block in &blocks {
        let bh = block_height(content_block, text_width);

        if cursor_y + bh <= scroll {
            cursor_y += bh;
            continue;
        }

        let screen_y = cursor_y.saturating_sub(scroll);
        if screen_y >= inner.height {
            break;
        }

        let visible_height = bh.min(inner.height.saturating_sub(screen_y));

        match content_block {
            ContentBlock::Text(lines, style) => {
                let mut text_lines = Vec::new();
                let mut line_y = screen_y;
                for line in lines {
                    if line_y >= inner.height {
                        break;
                    }
                    let wrapped: Vec<String> = if text_width > 0 {
                        line.chars()
                            .collect::<Vec<char>>()
                            .chunks(text_width as usize)
                            .map(|c| c.iter().collect())
                            .collect()
                    } else {
                        vec![line.clone()]
                    };
                    for wrapped_line in &wrapped {
                        if line_y >= inner.height {
                            break;
                        }
                        text_lines.push((line_y, wrapped_line.clone(), *style));
                        line_y += 1;
                    }
                }
                visible.push(VisibleBlock {
                    screen_y,
                    height: visible_height,
                    content: VisibleContent::Text { lines: text_lines },
                });
            }
            ContentBlock::Image(idx) => {
                visible.push(VisibleBlock {
                    screen_y,
                    height: visible_height,
                    content: VisibleContent::Image { idx: *idx },
                });
            }
        }

        cursor_y += bh;
    }

    for vblock in &visible {
        match &vblock.content {
            VisibleContent::Text { lines } => {
                for (line_y, text, style) in lines {
                    let render_y = inner.y + line_y;
                    if render_y < inner.bottom() {
                        let span_style = match style {
                            TextStyle::Normal => Style::default().fg(TEXT_PRIMARY).bg(BG),
                            TextStyle::Meta => Style::default().fg(TEXT_META).bg(BG),
                            TextStyle::ImageSeparator => Style::default().fg(ACCENT).bg(BG),
                            TextStyle::Title => Style::default()
                                .fg(Color::Rgb(255, 255, 255))
                                .bg(BG)
                                .add_modifier(Modifier::BOLD),
                            TextStyle::HorizontalRule => {
                                Style::default().fg(BORDER_UNFOCUSED).bg(BG)
                            }
                        };
                        let span = Span::styled(text.clone(), span_style);
                        frame.render_widget(
                            Paragraph::new(Line::from(span)),
                            Rect::new(inner.x, render_y, text_width, 1),
                        );
                    }
                }
            }
            VisibleContent::Image { .. } => {}
        }
    }

    for vblock in &mut visible {
        match &mut vblock.content {
            VisibleContent::Text { .. } => {}
            VisibleContent::Image { idx } => {
                let img_area = Rect {
                    x: inner.x,
                    y: inner.y + vblock.screen_y,
                    width: text_width.min(80),
                    height: vblock.height,
                };
                if let Some(state) = &mut reader.images[*idx] {
                    let widget = StatefulImage::default();
                    frame.render_stateful_widget(widget, img_area, &mut state.protocol);
                } else {
                    let placeholder = Span::styled(
                        format!("  [Image {} - not available]", *idx + 1),
                        Style::default().fg(TEXT_DIM).bg(BG),
                    );
                    frame.render_widget(
                        Paragraph::new(Line::from(placeholder)),
                        Rect::new(inner.x, inner.y + vblock.screen_y, text_width, 1),
                    );
                }
            }
        }
    }

    let scroll_pct = if max_scroll > 0 {
        (scroll as u32 * 100 / max_scroll as u32).min(100)
    } else {
        0
    };
    let scroll_bar = format_scroll_indicator(scroll_pct, max_scroll > 0);

    let footer = Line::from(vec![
        Span::styled(
            " j/k:scroll  Ctrl-d/u:half-page  g/G:top/bottom  q/Esc:back ",
            Style::default().fg(TEXT_DIM),
        ),
        Span::styled(scroll_bar, Style::default().fg(TEXT_META)),
    ]);
    frame.render_widget(footer, footer_area);
}

fn format_scroll_indicator(pct: u32, scrollable: bool) -> String {
    if !scrollable {
        return String::new();
    }
    let filled = (pct as usize * 10 / 100).min(10);
    let empty = 10 - filled;
    let bar: String = "█".repeat(filled) + &"░".repeat(empty);
    format!(" {pct}% {bar}")
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
    let section_end = (visible_end as usize).saturating_sub(1);

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

fn render_search(frame: &mut Frame, app: &mut App) {
    let [header_area, main_area, status_area] = Layout::vertical([
        Constraint::Length(3),
        Constraint::Fill(1),
        Constraint::Length(1),
    ])
    .areas(frame.area());

    frame.render_widget(
        Block::default().style(Style::default().bg(BG)),
        frame.area(),
    );

    let input_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(BORDER_FOCUSED))
        .title(" Search ")
        .title_style(Style::default().fg(ACCENT))
        .style(Style::default().bg(BG));

    let input_inner = input_block.inner(header_area);
    frame.render_widget(input_block, header_area);

    let query_line = Line::from(vec![
        Span::styled("> ", Style::default().fg(ACCENT)),
        Span::styled(
            &app.search_query,
            Style::default().fg(TEXT_PRIMARY),
        ),
        Span::styled("▎", Style::default().fg(ACCENT)),
    ]);
    frame.render_widget(query_line, input_inner);

    let result_count = app.search_results.len();
    let hint = Line::from(Span::styled(
        format!(
            " {} result{}  ↑↓:navigate  Enter:open  Esc:back ",
            result_count,
            if result_count != 1 { "s" } else { "" }
        ),
        Style::default().fg(TEXT_DIM),
    ));

    if app.search_results.is_empty() {
        let msg = if app.search_query.is_empty() {
            " Type to search across all cached articles..."
        } else {
            " No results found."
        };
        let empty = Paragraph::new(Line::from(Span::styled(
            msg,
            Style::default().fg(TEXT_DIM),
        )))
        .style(Style::default().bg(BG))
        .block(
            Block::default()
                .borders(Borders::NONE)
                .style(Style::default().bg(BG)),
        );
        frame.render_widget(empty, main_area);
        frame.render_widget(hint, status_area);
        return;
    }

    let items: Vec<ListItem> = app
        .search_results
        .iter()
        .enumerate()
        .map(|(i, article)| {
            let section_span = article.section.as_ref().map_or_else(Span::default, |s| {
                Span::styled(
                    format!(" {s} "),
                    Style::default()
                        .fg(Color::Rgb(22, 27, 44))
                        .bg(section_color(s)),
                )
            });

            let date_span = Span::styled(
                format!(" {}", &article.published_at[..10]),
                Style::default().fg(TEXT_META),
            );

            let row_bg = if i % 2 == 0 { BG } else { BG_LIGHTER };
            let title = Span::styled(&article.title, Style::default().fg(TEXT_PRIMARY).bg(row_bg));

            let mut spans = vec![title, Span::raw("  "), section_span, Span::raw(" "), date_span];
            if i % 2 == 1 {
                spans.push(Span::styled("", Style::default().bg(BG_LIGHTER)));
            }

            ListItem::new(Line::from(spans)).style(Style::default().bg(row_bg))
        })
        .collect();

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(BORDER_UNFOCUSED))
        .title(format!(" {} results ", result_count))
        .title_style(Style::default().fg(TEXT_PRIMARY))
        .style(Style::default().bg(BG));

    let list = List::new(items)
        .block(block)
        .highlight_style(
            Style::default()
                .fg(Color::Rgb(255, 255, 255))
                .bg(HIGHLIGHT_BG)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_spacing(HighlightSpacing::Always)
        .highlight_symbol(" ▶ ");

    let mut state = ListState::default();
    state.select(Some(app.search_selected));

    frame.render_stateful_widget(list, main_area, &mut state);
    frame.render_widget(hint, status_area);
}

fn render_status(frame: &mut Frame, _app: &App, area: Rect) {
    let help_text = " ?:help c:cal f:filter /:search s:sync q:quit ";
    let status = Line::from(Span::styled(
        help_text,
        Style::default().fg(TEXT_DIM).bg(BG),
    ));

    frame.render_widget(status, area);
}

fn render_help(frame: &mut Frame) {
    let area = centered_rect(54, 24, frame.area());

    frame.render_widget(Clear, area);

    let lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            "  Keybindings",
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  Calendar",
            Style::default().fg(ACCENT2).add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            "  h/l    prev/next month",
            Style::default().fg(TEXT_PRIMARY),
        )),
        Line::from(Span::styled(
            "  H/L    prev/next year",
            Style::default().fg(TEXT_PRIMARY),
        )),
        Line::from(Span::styled(
            "  j/k    move day",
            Style::default().fg(TEXT_PRIMARY),
        )),
        Line::from(Span::styled(
            "  g/G    jump to top/bottom",
            Style::default().fg(TEXT_PRIMARY),
        )),
        Line::from(Span::styled(
            "  s      sync selected day",
            Style::default().fg(TEXT_PRIMARY),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  Article List",
            Style::default().fg(ACCENT2).add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            "  j/k    scroll list",
            Style::default().fg(TEXT_PRIMARY),
        )),
        Line::from(Span::styled(
            "  Enter  open article",
            Style::default().fg(TEXT_PRIMARY),
        )),
        Line::from(Span::styled(
            "  c      switch to calendar",
            Style::default().fg(TEXT_PRIMARY),
        )),
        Line::from(Span::styled(
            "  f/F    cycle/clear section filter",
            Style::default().fg(TEXT_PRIMARY),
        )),
        Line::from(Span::styled(
            "  /      search all cached articles",
            Style::default().fg(TEXT_PRIMARY),
        )),
        Line::from(Span::styled(
            "  s      sync selected day",
            Style::default().fg(TEXT_PRIMARY),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  Article Reader",
            Style::default().fg(ACCENT2).add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            "  j/k    scroll  Ctrl-d/u  half-page",
            Style::default().fg(TEXT_PRIMARY),
        )),
        Line::from(Span::styled(
            "  g/G    top/bottom  q/Esc  back",
            Style::default().fg(TEXT_PRIMARY),
        )),
        Line::from(Span::styled(
            "  ?      toggle this help",
            Style::default().fg(TEXT_PRIMARY),
        )),
    ];

    let paragraph = Paragraph::new(lines).style(Style::default().bg(BG)).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(BORDER_FOCUSED))
            .title(" Help ")
            .title_style(Style::default().fg(ACCENT))
            .style(Style::default().bg(BG)),
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
