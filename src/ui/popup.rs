/// Reusable centered popup overlay for confirmations and data entry.
///
/// Two sizes: small popups for confirmations (40-col fixed) and
/// large modals for creation wizards (70% of screen width).

use ratatui::{
    Frame,
    widgets::{Block, Borders, Clear, Paragraph},
    layout::Rect,
    style::{Style, Modifier},
    text::Line,
};
use crate::ui::theme::Theme;

/// Render a small centered popup (confirmations, simple input).
/// Fixed width of 40 columns.
pub fn render_popup(
    f: &mut Frame,
    area: Rect,
    theme: &Theme,
    title: &str,
    body: &[Line],
    actions: &str,
) {
    render_overlay(f, area, theme, title, body, actions, 40);
}

/// Render a large centered modal (creation wizards, multi-field forms).
/// Uses 70% of the available width.
pub fn render_modal(
    f: &mut Frame,
    area: Rect,
    theme: &Theme,
    title: &str,
    body: &[Line],
    actions: &str,
) {
    let width = ((area.width as f32) * 0.7) as u16;
    render_overlay(f, area, theme, title, body, actions, width);
}

/// Shared overlay renderer with configurable max width.
fn render_overlay(
    f: &mut Frame,
    area: Rect,
    theme: &Theme,
    title: &str,
    body: &[Line],
    actions: &str,
    max_width: u16,
) {
    // Size to fit content, capped to available area.
    let content_height = body.len() as u16 + 4; // +2 border +1 title padding +1 actions
    let popup_width = max_width.min(area.width.saturating_sub(4));
    let popup_height = content_height.min(area.height.saturating_sub(2)).max(5);

    let x = area.x + (area.width.saturating_sub(popup_width)) / 2;
    let y = area.y + (area.height.saturating_sub(popup_height)) / 2;
    let popup_area = Rect::new(x, y, popup_width, popup_height);

    // Clear the area behind the popup.
    f.render_widget(Clear, popup_area);

    // Build the full content: body + empty line + actions.
    let mut lines: Vec<Line> = body.to_vec();
    lines.push(Line::from(""));
    lines.push(Line::from(actions.to_string()));

    let popup = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .border_style(Style::default().fg(theme.border))
                .title_style(Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
        )
        .style(Style::default().fg(theme.fg));

    f.render_widget(popup, popup_area);
}
