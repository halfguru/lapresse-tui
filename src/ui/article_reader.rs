use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};
use ratatui_image::StatefulImage;

use super::{
    ACCENT, BG, BORDER_UNFOCUSED, TAG_YELLOW, TEXT_DIM, TEXT_META, TEXT_PRIMARY,
    format_scroll_indicator,
};
use crate::app::{App, ImageLoadState};

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
                    count += (line.len() as u16).div_ceil(area_width);
                } else {
                    count += 1;
                }
            }
            count.max(1)
        }
        ContentBlock::Image(_) => 12,
    }
}

pub fn render_article_reader(frame: &mut Frame, app: &mut App) {
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
        .border_type(ratatui::widgets::BorderType::Rounded)
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
                match &mut reader.images[*idx] {
                    ImageLoadState::Loaded(state) => {
                        let widget = StatefulImage::default();
                        frame.render_stateful_widget(widget, img_area, &mut state.protocol);
                    }
                    ImageLoadState::Loading => {
                        let placeholder = Span::styled(
                            format!("  ⏳ Loading image {}...", *idx + 1),
                            Style::default().fg(TAG_YELLOW).bg(BG),
                        );
                        frame.render_widget(
                            Paragraph::new(Line::from(placeholder)),
                            Rect::new(inner.x, inner.y + vblock.screen_y, text_width, 1),
                        );
                    }
                    ImageLoadState::Failed => {
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
    }

    let scroll_pct = if max_scroll > 0 {
        (scroll as u32 * 100 / max_scroll as u32).min(100)
    } else {
        0
    };
    let scroll_bar = format_scroll_indicator(scroll_pct, max_scroll > 0);

    let footer = Line::from(vec![
        Span::styled(
            " j/k:scroll  Ctrl-d/u:half-page  g/G:top/bottom  o:open  y:copy  q/Esc:back ",
            Style::default().fg(TEXT_DIM),
        ),
        Span::styled(scroll_bar, Style::default().fg(TEXT_META)),
    ]);
    frame.render_widget(footer, footer_area);
}
