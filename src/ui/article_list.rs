use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, HighlightSpacing, List, ListItem, ListState, Wrap},
};

use super::{
    ACCENT, BG, BG_LIGHTER, BORDER_FOCUSED, BORDER_UNFOCUSED, HIGHLIGHT_BG, TEXT_DIM, TEXT_META,
    TEXT_PRIMARY, section_color,
};
use crate::app::App;

const SNIPPET_MAX: usize = 120;
const ACCENT_BAR: &str = "▎";

fn format_time(published_at: &str) -> String {
    published_at
        .split('T')
        .nth(1)
        .map(|t| {
            let parts: Vec<&str> = t.split(':').collect();
            if parts.len() >= 2 {
                format!("{}:{}", parts[0], parts[1])
            } else {
                String::new()
            }
        })
        .unwrap_or_default()
}

fn truncate(s: &str, max: usize) -> &str {
    if s.chars().count() <= max {
        s
    } else {
        let end = s
            .char_indices()
            .take(max)
            .last()
            .map(|(i, _)| i)
            .unwrap_or(0);
        &s[..end]
    }
}

pub fn render_article_list(frame: &mut Frame, app: &App, area: Rect) {
    let border_style = if app.focus == crate::app::Focus::ArticleList {
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
                Style::default()
                    .fg(ratatui::style::Color::Rgb(22, 27, 44))
                    .bg(color),
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
            let sync_date = app.syncing_date.map(|d| d.to_string()).unwrap_or_default();
            if let Some(phase) = &app.sync_phase {
                if phase.total > 0 {
                    format!(
                        " {spinner} {} — {}/{} for {sync_date}",
                        phase.phase, phase.current, phase.total
                    )
                } else {
                    format!(" {spinner} {} for {sync_date}...", phase.phase)
                }
            } else {
                format!(" {spinner} Syncing {sync_date} from lapresse.ca...")
            }
        } else if app.article_count == 0 {
            "No articles cached. Navigating to a date will auto-fetch.".to_string()
        } else {
            "No articles for this date.".to_string()
        };

        let style = if app.syncing {
            Style::default().fg(super::TAG_YELLOW)
        } else {
            Style::default().fg(TEXT_DIM)
        };

        let paragraph = ratatui::widgets::Paragraph::new(Line::from(Span::styled(msg, style)))
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

        let paragraph = ratatui::widgets::Paragraph::new(msg)
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
            let section_color = article
                .section
                .as_deref()
                .map(section_color)
                .unwrap_or(TEXT_DIM);
            let row_bg = if i % 2 == 0 { BG } else { BG_LIGHTER };

            let accent_span =
                Span::styled(ACCENT_BAR, Style::default().fg(section_color).bg(row_bg));

            let title_span = Span::styled(
                format!(" {}", article.title),
                Style::default().fg(TEXT_PRIMARY).bg(row_bg),
            );

            let time = format_time(&article.published_at);
            let mut meta_parts: Vec<Span> = vec![Span::styled("   ", Style::default().bg(row_bg))];
            if !time.is_empty() {
                meta_parts.push(Span::styled(
                    time,
                    Style::default().fg(TEXT_META).bg(row_bg),
                ));
                if article.author.is_some() || article.section.is_some() {
                    meta_parts.push(Span::styled(
                        " · ",
                        Style::default().fg(TEXT_DIM).bg(row_bg),
                    ));
                }
            }
            if let Some(ref author) = article.author {
                meta_parts.push(Span::styled(
                    author.as_str(),
                    Style::default().fg(TEXT_META).bg(row_bg),
                ));
                if article.section.is_some() {
                    meta_parts.push(Span::styled(
                        " · ",
                        Style::default().fg(TEXT_DIM).bg(row_bg),
                    ));
                }
            }
            if let Some(ref section) = article.section {
                meta_parts.push(Span::styled(
                    section.as_str(),
                    Style::default().fg(section_color).bg(row_bg),
                ));
            }

            let mut lines = vec![
                Line::from(vec![accent_span, title_span]),
                Line::from(meta_parts),
            ];

            if let Some(ref content) = article.snippet {
                let snippet = truncate(content, SNIPPET_MAX);
                if !snippet.is_empty() {
                    lines.push(Line::from(vec![
                        Span::styled("   ", Style::default().bg(row_bg)),
                        Span::styled(
                            format!("{snippet}…"),
                            Style::default().fg(TEXT_DIM).bg(row_bg),
                        ),
                    ]));
                }
            }

            lines.push(Line::from(Span::styled("", Style::default().bg(BG))));

            ListItem::new(lines).style(Style::default().bg(row_bg))
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
                .fg(ratatui::style::Color::Rgb(255, 255, 255))
                .bg(HIGHLIGHT_BG)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_spacing(HighlightSpacing::Always)
        .highlight_symbol(" ▶ ");

    let mut state = ListState::default();
    state.select(Some(app.article_list_selected));

    frame.render_stateful_widget(list, area, &mut state);
}
