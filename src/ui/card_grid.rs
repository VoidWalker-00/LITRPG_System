use crate::ui::theme::Theme;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Style, Modifier},
    text::Line,
    widgets::{Block, Borders, Paragraph},
};

/// Fixed card dimensions (including borders).
pub const CARD_WIDTH: u16 = 28;
pub const CARD_HEIGHT: u16 = 7;
/// Gaps between cards.
pub const GAP_H: u16 = 2;
pub const GAP_V: u16 = 1;

/// Data for a single card in the grid.
pub struct CardData {
    pub title: String,
    pub lines: Vec<String>,
    pub is_action: bool,
}

/// Navigation direction within the grid.
pub enum Direction {
    Left,
    Right,
    Up,
    Down,
}

/// Truncate a string to fit within max_width, adding "..." if needed.
pub fn truncate_line(s: &str, max_width: usize) -> String {
    if s.len() <= max_width {
        s.to_string()
    } else {
        format!("{}...", &s[..max_width - 3])
    }
}

/// How many card columns fit in the given width.
pub fn grid_columns(area_width: u16, card_width: u16, gap: u16) -> usize {
    let total_per_card = card_width + gap;
    let cols = area_width / total_per_card;
    (cols as usize).max(1)
}

/// How many card rows are visible in the given height.
pub fn grid_visible_rows(area_height: u16, card_height: u16, gap: u16) -> usize {
    let total_per_row = card_height + gap;
    let rows = area_height / total_per_row;
    (rows as usize).max(1)
}

/// Move selection in the grid. Returns (new_selected, new_scroll_offset).
pub fn grid_navigate(
    direction: Direction,
    selected: usize,
    total: usize,
    columns: usize,
    scroll_offset: usize,
    visible_rows: usize,
) -> (usize, usize) {
    if total == 0 {
        return (0, 0);
    }

    let new_selected = match direction {
        Direction::Right => {
            if selected + 1 < total { selected + 1 } else { selected }
        }
        Direction::Left => {
            if selected > 0 { selected - 1 } else { 0 }
        }
        Direction::Down => {
            let target = selected + columns;
            if target < total { target } else { (total - 1).min(selected + columns) }
        }
        Direction::Up => {
            if selected >= columns { selected - columns } else { 0 }
        }
    };

    // Adjust scroll to keep selected card visible.
    let selected_row = new_selected / columns;
    let mut new_scroll = scroll_offset;
    if selected_row < new_scroll {
        new_scroll = selected_row;
    } else if selected_row >= new_scroll + visible_rows {
        new_scroll = selected_row - visible_rows + 1;
    }

    (new_selected, new_scroll)
}

/// Render a grid of cards into the given area.
pub fn render_card_grid(
    f: &mut Frame,
    area: Rect,
    cards: &[CardData],
    selected: usize,
    scroll_offset: usize,
    theme: &Theme,
) {
    let cols = grid_columns(area.width, CARD_WIDTH, GAP_H);
    let vis_rows = grid_visible_rows(area.height, CARD_HEIGHT, GAP_V);
    let inner_width = (CARD_WIDTH - 2) as usize; // 26 chars inside borders

    for row in 0..vis_rows {
        let actual_row = scroll_offset + row;
        for col in 0..cols {
            let idx = actual_row * cols + col;
            if idx >= cards.len() {
                break;
            }
            let card = &cards[idx];
            let is_selected = idx == selected;

            // Calculate card position.
            let x = area.x + (col as u16) * (CARD_WIDTH + GAP_H);
            let y = area.y + (row as u16) * (CARD_HEIGHT + GAP_V);

            // Skip if card would be outside area.
            if x + CARD_WIDTH > area.x + area.width || y + CARD_HEIGHT > area.y + area.height {
                continue;
            }

            let card_area = Rect::new(x, y, CARD_WIDTH, CARD_HEIGHT);

            // Choose border color.
            let border_color = if is_selected {
                theme.accent
            } else if card.is_action {
                theme.secondary
            } else {
                theme.border
            };

            // Build card content lines.
            let mut lines: Vec<Line> = Vec::new();

            if card.is_action {
                // Action card: centered title, empty lines.
                let padding = inner_width.saturating_sub(card.title.len()) / 2;
                lines.push(Line::styled(
                    format!("{}{}", " ".repeat(padding), card.title),
                    Style::default().fg(theme.secondary),
                ));
                // Fill remaining lines.
                for _ in 0..(CARD_HEIGHT as usize - 3) {
                    lines.push(Line::from(""));
                }
            } else {
                // Normal card: bold title + content lines.
                lines.push(Line::styled(
                    format!(" {}", truncate_line(&card.title, inner_width - 1)),
                    Style::default().fg(theme.fg).add_modifier(Modifier::BOLD),
                ));
                for line in &card.lines {
                    lines.push(Line::styled(
                        format!(" {}", truncate_line(line, inner_width - 1)),
                        Style::default().fg(theme.fg),
                    ));
                }
                // Pad to fill card height (CARD_HEIGHT - 2 for borders).
                while lines.len() < (CARD_HEIGHT as usize - 2) {
                    lines.push(Line::from(""));
                }
            }

            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(border_color));

            let paragraph = Paragraph::new(lines).block(block);
            f.render_widget(paragraph, card_area);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grid_columns_wide_terminal() {
        // 120 width, 28-wide cards, 2-char gap = 120 / 30 = 4
        assert_eq!(grid_columns(120, CARD_WIDTH, GAP_H), 4);
    }

    #[test]
    fn test_grid_columns_narrow_terminal() {
        // 30 width = 1 column (minimum)
        assert_eq!(grid_columns(30, CARD_WIDTH, GAP_H), 1);
    }

    #[test]
    fn test_grid_columns_exact_fit() {
        // 60 width = 60 / 30 = 2
        assert_eq!(grid_columns(60, CARD_WIDTH, GAP_H), 2);
    }

    #[test]
    fn test_grid_visible_rows() {
        // 24 height, 7-high cards, 1-line gap = 24 / 8 = 3
        assert_eq!(grid_visible_rows(24, CARD_HEIGHT, GAP_V), 3);
    }

    #[test]
    fn test_grid_visible_rows_small() {
        // 5 height = minimum 1
        assert_eq!(grid_visible_rows(5, CARD_HEIGHT, GAP_V), 1);
    }

    #[test]
    fn test_navigate_right_wraps_to_next_row() {
        // 6 items, 3 columns. At index 2 (end of row 0), Right goes to 3 (start of row 1).
        let (sel, _) = grid_navigate(Direction::Right, 2, 6, 3, 0, 3);
        assert_eq!(sel, 3);
    }

    #[test]
    fn test_navigate_left_wraps_to_prev_row() {
        // At index 3 (start of row 1), Left goes to 2 (end of row 0).
        let (sel, _) = grid_navigate(Direction::Left, 3, 6, 3, 0, 3);
        assert_eq!(sel, 2);
    }

    #[test]
    fn test_navigate_down_same_column() {
        // 3 columns, at index 1, Down goes to 4.
        let (sel, _) = grid_navigate(Direction::Down, 1, 6, 3, 0, 3);
        assert_eq!(sel, 4);
    }

    #[test]
    fn test_navigate_down_clamps_to_last() {
        // 3 columns, at index 4, Down would go to 7 but total is 6. Clamp to 5.
        let (sel, _) = grid_navigate(Direction::Down, 4, 6, 3, 0, 3);
        assert_eq!(sel, 5);
    }

    #[test]
    fn test_navigate_up_clamps_to_zero() {
        // At index 1, Up would go to -2. Clamp to 0.
        let (sel, _) = grid_navigate(Direction::Up, 1, 6, 3, 0, 3);
        assert_eq!(sel, 0);
    }

    #[test]
    fn test_navigate_right_clamps_at_end() {
        // At last item (index 5), Right stays at 5.
        let (sel, _) = grid_navigate(Direction::Right, 5, 6, 3, 0, 3);
        assert_eq!(sel, 5);
    }

    #[test]
    fn test_navigate_left_clamps_at_zero() {
        // At index 0, Left stays at 0.
        let (sel, _) = grid_navigate(Direction::Left, 0, 6, 3, 0, 3);
        assert_eq!(sel, 0);
    }

    #[test]
    fn test_scroll_adjusts_when_moving_below_visible() {
        // 9 items, 3 cols, visible 1 row (3 items visible). At index 2, Down goes to 5.
        // Row 1 is not visible (scroll_offset=0, visible_rows=1), so scroll should advance.
        let (sel, scroll) = grid_navigate(Direction::Down, 2, 9, 3, 0, 1);
        assert_eq!(sel, 5);
        assert_eq!(scroll, 1);
    }

    #[test]
    fn test_scroll_adjusts_when_moving_above_visible() {
        // At index 3 (row 1), scroll_offset=1, Up goes to 0 (row 0). Scroll back.
        let (sel, scroll) = grid_navigate(Direction::Up, 3, 9, 3, 1, 1);
        assert_eq!(sel, 0);
        assert_eq!(scroll, 0);
    }

    #[test]
    fn test_truncate_line() {
        assert_eq!(truncate_line("short", 26), "short");
        assert_eq!(truncate_line("this is a very long string that exceeds", 26), "this is a very long str...");
    }
}
