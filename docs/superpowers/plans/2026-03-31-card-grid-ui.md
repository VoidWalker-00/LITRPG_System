# Card Grid UI Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace list-based views in Character, Skill Library, and Profession Library tabs with a reusable card grid layout.

**Architecture:** A new `card_grid.rs` module provides `CardData`, `render_card_grid()`, and `grid_navigate()`. Each tab builds a `Vec<CardData>` from its data and delegates rendering/navigation to the grid module. Prerequisites include renaming Tier→Grade, Trees→Professions in the data model.

**Tech Stack:** Rust, ratatui 0.29, crossterm 0.29, serde/serde_json

**Spec:** `docs/superpowers/specs/2026-03-31-card-grid-ui-design.md`

---

## Chunk 1: Data Model Prerequisites

The Character model still uses `tier: u32` and `trees: Vec<CharacterTree>`. The spec requires `grade`, `race`, and `professions`. This chunk updates the models and storage to match.

### Task 1: Add Grade Enum

**Files:**
- Create: `src/models/grade.rs`
- Modify: `src/models/mod.rs`

- [ ] **Step 1: Write the test**

In `src/models/grade.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grade_numeric() {
        assert_eq!(Grade::G.numeric(), 0);
        assert_eq!(Grade::F.numeric(), 1);
        assert_eq!(Grade::A.numeric(), 6);
        assert_eq!(Grade::S.numeric(), 7);
    }

    #[test]
    fn test_grade_display() {
        assert_eq!(Grade::G.name(), "G");
        assert_eq!(Grade::SS.name(), "SS");
    }

    #[test]
    fn test_grade_from_numeric() {
        assert_eq!(Grade::from_numeric(0), Grade::G);
        assert_eq!(Grade::from_numeric(6), Grade::A);
        assert_eq!(Grade::from_numeric(99), Grade::S); // clamp unknown to S
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test models::grade --no-run 2>&1`
Expected: Compile error — `Grade` not defined yet.

- [ ] **Step 3: Write the implementation**

In `src/models/grade.rs`:

```rust
use serde::{Deserialize, Serialize};

/// Grade represents a being's evolutionary rank.
/// G is the lowest (starting grade), progressing upward.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Grade {
    G,
    F,
    E,
    D,
    C,
    B,
    A,
    S,
    SS,
    SSS,
}

impl Grade {
    /// Numeric equivalent for use in formulas (replaces old tier number).
    pub fn numeric(&self) -> u32 {
        match self {
            Grade::G => 0,
            Grade::F => 1,
            Grade::E => 2,
            Grade::D => 3,
            Grade::C => 4,
            Grade::B => 5,
            Grade::A => 6,
            Grade::S => 7,
            Grade::SS => 8,
            Grade::SSS => 9,
        }
    }

    /// Display name for UI rendering.
    pub fn name(&self) -> &'static str {
        match self {
            Grade::G => "G",
            Grade::F => "F",
            Grade::E => "E",
            Grade::D => "D",
            Grade::C => "C",
            Grade::B => "B",
            Grade::A => "A",
            Grade::S => "S",
            Grade::SS => "SS",
            Grade::SSS => "SSS",
        }
    }

    /// Convert a numeric value back to a Grade. Unknown values clamp to S.
    pub fn from_numeric(n: u32) -> Grade {
        match n {
            0 => Grade::G,
            1 => Grade::F,
            2 => Grade::E,
            3 => Grade::D,
            4 => Grade::C,
            5 => Grade::B,
            6 => Grade::A,
            7 => Grade::S,
            8 => Grade::SS,
            9 => Grade::SSS,
            _ => Grade::S,
        }
    }
}
```

- [ ] **Step 4: Register the module**

In `src/models/mod.rs`, add:

```rust
pub mod grade;
```

- [ ] **Step 5: Run tests**

Run: `cargo test models::grade -- --nocapture`
Expected: All 3 tests pass.

- [ ] **Step 6: Commit**

```bash
git add src/models/grade.rs src/models/mod.rs
git commit -m "feat: add Grade enum (G through SSS) with numeric conversion"
```

---

### Task 2: Add Profession Model

**Files:**
- Create: `src/models/profession.rs`
- Modify: `src/models/mod.rs`

- [ ] **Step 1: Write the test**

In `src/models/profession.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_profession_definition_creation() {
        let prof = ProfessionDefinition {
            name: "Blacksmith".to_string(),
            description: "Craft weapons and armor".to_string(),
            skills: vec!["Forging".to_string(), "Tempering".to_string()],
            passive_name: "Forgeborn".to_string(),
            passive_description: "Increases crafting quality".to_string(),
        };
        assert_eq!(prof.name, "Blacksmith");
        assert_eq!(prof.skills.len(), 2);
    }

    #[test]
    fn test_profession_serialization() {
        let prof = ProfessionDefinition {
            name: "Herbalist".to_string(),
            description: "Gather herbs".to_string(),
            skills: vec!["Gathering".to_string()],
            passive_name: "Nature's Touch".to_string(),
            passive_description: "Improves herb yield".to_string(),
        };
        let json = serde_json::to_string(&prof).unwrap();
        let parsed: ProfessionDefinition = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.name, "Herbalist");
        assert_eq!(parsed.skills.len(), 1);
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test models::profession --no-run 2>&1`
Expected: Compile error — `ProfessionDefinition` not defined.

- [ ] **Step 3: Write the implementation**

In `src/models/profession.rs`:

```rust
use serde::{Deserialize, Serialize};

/// Definition of a profession in the library.
/// Professions grant skills and a passive that evolves through achievements.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfessionDefinition {
    pub name: String,
    pub description: String,
    /// Names of skills granted by this profession.
    pub skills: Vec<String>,
    /// Name of the passive skill this profession provides.
    pub passive_name: String,
    /// Description of the passive skill.
    pub passive_description: String,
}

/// A character's active profession instance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterProfession {
    /// Name of the ProfessionDefinition this refers to.
    pub definition_name: String,
    /// Current profession level (leveled through activity, not XP).
    pub level: u32,
    /// Current evolution rank of the profession's passive.
    pub passive_rank: u32,
}
```

- [ ] **Step 4: Register the module**

In `src/models/mod.rs`, add:

```rust
pub mod profession;
```

- [ ] **Step 5: Run tests**

Run: `cargo test models::profession -- --nocapture`
Expected: All 2 tests pass.

- [ ] **Step 6: Commit**

```bash
git add src/models/profession.rs src/models/mod.rs
git commit -m "feat: add ProfessionDefinition and CharacterProfession models"
```

---

### Task 3: Update Character Model

**Files:**
- Modify: `src/models/character.rs`

The current Character struct has `tier: u32` and `trees: Vec<CharacterTree>`. Replace with `grade`, `race`, `professions`.

- [ ] **Step 1: Update the Character struct**

In `src/models/character.rs`, update imports and struct:

```rust
use crate::models::grade::Grade;
use crate::models::profession::CharacterProfession;
```

Replace in the `Character` struct:
- `pub tier: u32` → `pub grade: Grade`
- `pub trees: Vec<CharacterTree>` → `pub professions: Vec<CharacterProfession>`
- `pub unspent_tree_points: u32` → remove this field
- Add: `pub race: String`
- Add: `pub profession_slots: u32`

- [ ] **Step 2: Update `Character::new()`**

The constructor should set `grade: Grade::G`, `race: "Human".to_string()`, `professions: Vec::new()`, `profession_slots: 1`.

- [ ] **Step 3: Update `attribute_points_per_level()`**

This method currently uses `self.tier`. Change to `self.grade.numeric()`:

```rust
pub fn attribute_points_per_level(&self) -> u32 {
    let base = 3 + self.bonus_attribute_points_per_level;
    base * 2u32.pow(self.grade.numeric())
}
```

- [ ] **Step 4: Fix all tests in character.rs**

Update test assertions to use `Grade::G` instead of `tier: 0`, remove `unspent_tree_points` references, and add `race`/`professions`/`profession_slots` fields to test character constructions.

- [ ] **Step 5: Fix compile errors across codebase**

Any file referencing `character.tier`, `character.trees`, or `character.unspent_tree_points` must be updated. Key files:
- `src/ui/character_creation.rs` — the confirm step shows "Tier: 0", change to "Race: Human [Grade G]"
- `src/ui/system_panel.rs` — displays tier/level header, tree sections
- `src/formulas/xp.rs` — uses `tier` parameter in formulas (rename param to `grade`)

- [ ] **Step 6: Run full test suite**

Run: `cargo test`
Expected: All tests pass (some XP tests may need parameter renames from `tier` to `grade`).

- [ ] **Step 7: Commit**

```bash
git add -A
git commit -m "refactor: update Character model — tier→grade, trees→professions, add race"
```

---

### Task 4: Update App State and Storage

**Files:**
- Modify: `src/ui/app.rs`
- Modify: `src/storage/json_store.rs`
- Modify: `src/main.rs`

- [ ] **Step 1: Update App struct**

In `src/ui/app.rs`:
- Change import from `crate::models::tree::TreeChain` to `crate::models::profession::ProfessionDefinition`
- Rename `Tab::TreeLibrary` to `Tab::ProfessionLibrary` and update `label()` to `"Profession Library"`
- Rename `tree_library: Vec<TreeChain>` to `profession_library: Vec<ProfessionDefinition>`
- Update `Tab::next()`: `SkillLibrary => ProfessionLibrary`, `ProfessionLibrary => Character`

- [ ] **Step 2: Add profession storage functions**

In `src/storage/json_store.rs`, add:

```rust
use crate::models::profession::ProfessionDefinition;

pub fn save_professions(path: &Path, professions: &[ProfessionDefinition]) -> Result<(), Box<dyn std::error::Error>> {
    save_json(path, professions)
}

pub fn load_professions(path: &Path) -> Result<Vec<ProfessionDefinition>, Box<dyn std::error::Error>> {
    load_json(path)
}
```

- [ ] **Step 3: Update main.rs references**

- Change `tree_state` / `TreeLibraryState` references to `profession_state` / `ProfessionLibraryState`
- Update `Tab::TreeLibrary` match arms to `Tab::ProfessionLibrary`
- Update startup loading: load `professions.json` instead of `trees.json`
- Update save-on-quit: save `professions.json` instead of `trees.json`

- [ ] **Step 4: Run full test suite**

Run: `cargo test`
Expected: All tests pass.

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "refactor: rename TreeLibrary→ProfessionLibrary in App, storage, and main"
```

---

## Chunk 2: Card Grid Module

### Task 5: Create card_grid.rs — Data Types and Layout Math

**Files:**
- Create: `src/ui/card_grid.rs`
- Modify: `src/ui/mod.rs`

- [ ] **Step 1: Write layout math tests**

In `src/ui/card_grid.rs`:

```rust
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
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test ui::card_grid --no-run 2>&1`
Expected: Compile error — nothing defined yet.

- [ ] **Step 3: Write the implementation — constants and types**

```rust
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
```

- [ ] **Step 4: Write the implementation — layout math**

```rust
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
```

- [ ] **Step 5: Register the module**

In `src/ui/mod.rs`, add:

```rust
pub mod card_grid;
```

- [ ] **Step 6: Run tests**

Run: `cargo test ui::card_grid -- --nocapture`
Expected: All 14 tests pass.

- [ ] **Step 7: Commit**

```bash
git add src/ui/card_grid.rs src/ui/mod.rs
git commit -m "feat: add card_grid module with layout math and navigation"
```

---

### Task 6: Card Grid Rendering

**Files:**
- Modify: `src/ui/card_grid.rs`

- [ ] **Step 1: Implement render_card_grid()**

Add to `src/ui/card_grid.rs`:

```rust
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
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo build`
Expected: No errors.

- [ ] **Step 3: Commit**

```bash
git add src/ui/card_grid.rs
git commit -m "feat: add render_card_grid() for visual card rendering"
```

---

## Chunk 3: Tab Integration — Skill Library

Start with Skill Library since it already has `skill_library` data in App and needs the fewest model changes.

### Task 7: Replace Skill Library List with Card Grid

**Files:**
- Modify: `src/ui/skill_library.rs`

- [ ] **Step 1: Add scroll_offset to SkillLibraryState**

Add `pub scroll_offset: usize` field to `SkillLibraryState`, initialize to `0` in `new()`.

- [ ] **Step 2: Add helper to build skill cards**

```rust
use crate::ui::card_grid::{self, CardData, Direction, CARD_WIDTH, GAP_H, GAP_V, CARD_HEIGHT};

fn build_skill_cards(skills: &[SkillDefinition], indices: &[usize]) -> Vec<CardData> {
    let mut cards: Vec<CardData> = indices.iter().map(|&i| {
        let skill = &skills[i];
        let effect_count = skill.ranks.first().map(|r| r.effects.len()).unwrap_or(0);
        let effect_text = if effect_count == 1 {
            "1 effect".to_string()
        } else {
            format!("{} effects", effect_count)
        };
        CardData {
            title: skill.name.clone(),
            lines: vec![
                format!("{:?} | {:?}", skill.skill_type, skill.category),
                effect_text,
                card_grid::truncate_line(&skill.description, 24),
            ],
            is_action: false,
        }
    }).collect();
    cards.push(CardData {
        title: "+ New Skill".to_string(),
        lines: vec![],
        is_action: true,
    });
    cards
}
```

- [ ] **Step 3: Replace render_list() body**

Replace the skill list rendering logic with:

```rust
fn render_list(&self, f: &mut Frame, area: Rect, app: &App, theme: &Theme) {
    let indices = self.filtered_indices(app);
    let mut lines: Vec<Line> = Vec::new();

    // Filter bar (keep existing).
    // ... (existing filter bar code stays unchanged) ...

    // Card grid area: below the filter bar (2 lines for filter + divider).
    let grid_y = area.y + 2;
    let grid_height = area.height.saturating_sub(4); // 2 for filter bar, 2 for help hint
    if grid_height > 0 {
        let grid_area = Rect::new(area.x + 1, grid_y, area.width.saturating_sub(2), grid_height);
        let cards = build_skill_cards(&app.skill_library, &indices);
        card_grid::render_card_grid(f, grid_area, &cards, self.cursor, self.scroll_offset, theme);
    }

    // Help hint at bottom.
    let hint_y = area.y + area.height - 1;
    let hint = Paragraph::new(Line::styled(
        "  h/j/k/l: Navigate  Enter: View  a: Add  d: Delete  /: Search",
        Style::default().fg(theme.dimmed),
    ));
    f.render_widget(hint, Rect::new(area.x, hint_y, area.width, 1));
}
```

Note: The filter bar rendering (lines 200–228 in current code) should still render as `Paragraph` into the top 2 lines of `area`. Only the skill entries section is replaced by the card grid.

- [ ] **Step 4: Update browse navigation to use grid**

In `handle_browse()`, replace j/k-only navigation with grid navigation:

```rust
fn handle_browse(&mut self, key: KeyEvent, app: &App) {
    let indices = self.filtered_indices(app);
    let cards_total = indices.len() + 1; // +1 for action card
    let cols = card_grid::grid_columns(80, card_grid::CARD_WIDTH, card_grid::GAP_H); // approximate
    let vis_rows = 3; // approximate, will be refined when area is available

    match key.code {
        KeyCode::Up | KeyCode::Char('k') => {
            let (sel, scroll) = card_grid::grid_navigate(
                Direction::Up, self.cursor, cards_total, cols, self.scroll_offset, vis_rows,
            );
            self.cursor = sel;
            self.scroll_offset = scroll;
        }
        KeyCode::Down | KeyCode::Char('j') => {
            let (sel, scroll) = card_grid::grid_navigate(
                Direction::Down, self.cursor, cards_total, cols, self.scroll_offset, vis_rows,
            );
            self.cursor = sel;
            self.scroll_offset = scroll;
        }
        KeyCode::Left | KeyCode::Char('h') => {
            let (sel, scroll) = card_grid::grid_navigate(
                Direction::Left, self.cursor, cards_total, cols, self.scroll_offset, vis_rows,
            );
            self.cursor = sel;
            self.scroll_offset = scroll;
        }
        KeyCode::Right | KeyCode::Char('l') => {
            let (sel, scroll) = card_grid::grid_navigate(
                Direction::Right, self.cursor, cards_total, cols, self.scroll_offset, vis_rows,
            );
            self.cursor = sel;
            self.scroll_offset = scroll;
        }
        KeyCode::Enter => {
            // If on action card (last card), open creation form.
            if self.cursor == indices.len() {
                self.reset_skill_form();
                self.mode = Mode::SkillForm;
            } else if !indices.is_empty() {
                self.mode = Mode::ViewDetail;
            }
        }
        // ... (a, e, d, /, Esc handlers stay the same) ...
    }
}
```

- [ ] **Step 5: Build and test**

Run: `cargo build && cargo test`
Expected: Compiles and all tests pass.

- [ ] **Step 6: Commit**

```bash
git add src/ui/skill_library.rs
git commit -m "feat: replace skill library list with card grid"
```

---

## Chunk 4: Tab Integration — Character and Profession

### Task 8: Replace Character List with Card Grid

**Files:**
- Modify: `src/ui/character_creation.rs`

- [ ] **Step 1: Add scroll_offset and imports**

Add `scroll_offset: usize` to `CharacterCreationState`, initialize to `0`.
Add import: `use crate::ui::card_grid::{self, CardData, Direction};`

- [ ] **Step 2: Add helper to build character cards**

```rust
fn build_character_cards(app: &App, names: &[String]) -> Vec<CardData> {
    let char_dir = app.data_dir.join("characters");
    let mut cards: Vec<CardData> = names.iter().map(|name| {
        match json_store::load_character(&char_dir, name) {
            Ok(c) => {
                let race_grade = format!("{} [Grade {}]", c.race, c.grade.name());
                let level = format!("Lv {}", c.level);
                let profession = c.professions.first()
                    .map(|p| p.definition_name.clone())
                    .unwrap_or_else(|| "(none)".to_string());
                CardData {
                    title: c.name.clone(),
                    lines: vec![race_grade, level, profession],
                    is_action: false,
                }
            }
            Err(_) => CardData {
                title: name.clone(),
                lines: vec!["(failed to load)".to_string()],
                is_action: false,
            },
        }
    }).collect();
    cards.push(CardData {
        title: "+ New Character".to_string(),
        lines: vec![],
        is_action: true,
    });
    cards
}
```

- [ ] **Step 3: Replace render_list() with card grid**

Replace the character list rendering with:

```rust
fn render_list(&self, f: &mut Frame, area: Rect, app: &App, theme: &Theme) {
    let mut lines: Vec<Line> = Vec::new();

    // Section header.
    let total = self.saved_names.len();
    lines.push(Line::styled(
        format!("  Characters ({})", total),
        Style::default().fg(theme.secondary).add_modifier(Modifier::BOLD),
    ));
    lines.push(Line::styled(
        divider(area.width),
        Style::default().fg(theme.border),
    ));

    // Render header.
    let header = Paragraph::new(lines).block(Block::default().borders(Borders::NONE));
    let header_area = Rect::new(area.x, area.y, area.width, 2);
    f.render_widget(header, header_area);

    // Card grid area.
    let grid_y = area.y + 2;
    let grid_height = area.height.saturating_sub(4);
    if grid_height > 0 {
        let grid_area = Rect::new(area.x + 1, grid_y, area.width.saturating_sub(2), grid_height);
        let cards = build_character_cards(app, &self.saved_names);
        card_grid::render_card_grid(f, grid_area, &cards, self.selected, self.scroll_offset, theme);
    }

    // Help hint.
    let hint_y = area.y + area.height - 1;
    let hint = Paragraph::new(Line::styled(
        "  h/j/k/l: Navigate  Enter: Load  a: New  d: Delete",
        Style::default().fg(theme.dimmed),
    ));
    f.render_widget(hint, Rect::new(area.x, hint_y, area.width, 1));
}
```

- [ ] **Step 4: Update handle_list_input() with grid navigation**

Replace j/k navigation with h/j/k/l grid navigation (same pattern as Task 7 Step 4). Handle Enter on the action card (last index) to trigger wizard.

- [ ] **Step 5: Build and test**

Run: `cargo build && cargo test`
Expected: Compiles and all tests pass.

- [ ] **Step 6: Commit**

```bash
git add src/ui/character_creation.rs
git commit -m "feat: replace character list with card grid"
```

---

### Task 9: Create Profession Library Tab with Card Grid

**Files:**
- Create: `src/ui/profession_library.rs` (replace `src/ui/tree_library.rs`)
- Modify: `src/ui/mod.rs`
- Modify: `src/main.rs`

- [ ] **Step 1: Create profession_library.rs**

Create a new file `src/ui/profession_library.rs` modeled on the skill library pattern but for professions. It should have:

- `ProfessionLibraryState` with: `mode`, `cursor`, `scroll_offset`, `search_query`, `wizard` fields
- `Mode` enum: `Browse`, `Search`, `ViewDetail`, `ProfessionForm`, `ConfirmDelete`
- `render()` that shows a card grid in Browse mode and detail/form as overlays
- `build_profession_cards()` that creates cards from `app.profession_library`
- `handle_input()` with grid navigation in Browse mode
- Wizard form for profession creation (name → description → skills → passive)

Card content per the spec:
```rust
CardData {
    title: prof.name.clone(),
    lines: vec![
        format!("{} base skills", prof.skills.len()),
        card_grid::truncate_line(&prof.description, 24),
    ],
    is_action: false,
}
```

- [ ] **Step 2: Update mod.rs**

Replace `pub mod tree_library;` with `pub mod profession_library;`

- [ ] **Step 3: Update main.rs**

- Import `profession_library::ProfessionLibraryState` instead of `tree_library::TreeLibraryState`
- Create `profession_state = ProfessionLibraryState::new()` instead of `tree_state`
- Match `Tab::ProfessionLibrary => profession_state.handle_input(key, &mut app)` in input dispatch
- Match `Tab::ProfessionLibrary => profession_state.render(...)` in render dispatch

- [ ] **Step 4: Optionally delete or keep tree_library.rs**

If `tree_library.rs` is no longer referenced anywhere, it can be deleted. Check with `cargo build` — if it compiles without it, remove it.

- [ ] **Step 5: Build and test**

Run: `cargo build && cargo test`
Expected: Compiles and all tests pass.

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "feat: add profession library tab with card grid, replace tree library"
```

---

### Task 10: Final Cleanup and Verification

**Files:**
- Various

- [ ] **Step 1: Remove dead tree code**

If `src/models/tree.rs` and `src/formulas/tree_points.rs` are no longer referenced, remove them and their `pub mod` declarations.

- [ ] **Step 2: Run full test suite**

Run: `cargo test`
Expected: All tests pass. Some tree-related tests will be gone; that's expected.

- [ ] **Step 3: Run the application**

Run: `cargo run`
Verify:
- Character tab shows cards in a grid
- Skill Library tab shows cards in a grid with filter bar
- Profession Library tab shows cards in a grid
- h/j/k/l navigation works in all grids
- Enter on `+ New` card opens creation wizard
- Enter on regular cards opens detail/loads character
- Tab key cycles through all 4 tabs
- Existing wizards, modals, and delete confirmations still work

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "chore: remove dead tree code, final cleanup"
```
