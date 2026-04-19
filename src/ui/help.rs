use ratatui::{
    Frame,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
};

use super::{ACCENT, ACCENT2, BG, BORDER_FOCUSED, TEXT_PRIMARY};

pub fn render_help(frame: &mut Frame) {
    let area = super::centered_rect(54, 24, frame.area());

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
