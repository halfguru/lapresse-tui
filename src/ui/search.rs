use ratatui::{
    Frame,
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, HighlightSpacing, List, ListItem, ListState},
};

use super::{
    ACCENT, BG, BG_LIGHTER, BORDER_FOCUSED, BORDER_UNFOCUSED, HIGHLIGHT_BG, TEXT_DIM, TEXT_META,
    TEXT_PRIMARY, section_color,
};
use crate::app::App;

pub fn render_search(frame: &mut Frame, app: &mut App) {
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

    let spinner_chars = "⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏";
    let spinner = if app.searching {
        spinner_chars
            .chars()
            .nth(app.search_spinner as usize % spinner_chars.len())
            .unwrap()
            .to_string()
    } else {
        String::new()
    };

    let query_line = Line::from(vec![
        Span::styled("> ", Style::default().fg(ACCENT)),
        Span::styled(&app.search_query, Style::default().fg(TEXT_PRIMARY)),
        Span::styled("▎", Style::default().fg(ACCENT)),
        if app.searching {
            Span::styled(
                format!(" {spinner} searching..."),
                Style::default().fg(ACCENT),
            )
        } else {
            Span::raw("")
        },
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
        let empty = ratatui::widgets::Paragraph::new(Line::from(Span::styled(
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

            let mut spans = vec![
                title,
                Span::raw("  "),
                section_span,
                Span::raw(" "),
                date_span,
            ];
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
