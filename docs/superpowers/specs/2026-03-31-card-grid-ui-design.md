# Card Grid UI Design

> Replace list-based views in Character, Skill Library, and Profession Library tabs with a reusable card grid layout.

---

## Overview

All three browsing tabs (Character, Skill Library, Profession Library) currently display items as simple text lines in a scrollable list. This design replaces those lists with a responsive grid of fixed-size cards, providing a richer visual representation of each item at a glance.

**Note:** The data model changes (Tier -> Grade, Trees -> Professions, adding race to Character) are a prerequisite tracked separately in `Docs/System Rules.md` and `Docs/Technical Details.md`. This spec assumes those model changes are in place.

## Card Component (`src/ui/card_grid.rs`)

A reusable module that any tab can use to render a grid of cards.

### CardData Struct

```rust
pub struct CardData {
    pub title: String,       // Top line, rendered bold
    pub lines: Vec<String>,  // 2-4 content lines
    pub is_action: bool,     // True for "+ New" style cards (styled differently)
}
```

### Grid Layout

- **Card size:** fixed 28 characters wide, 7 lines tall (including borders).
- **Columns:** calculated from available terminal width: `cols = area.width / (card_width + gap)`, minimum 1.
- **Gap:** 2 characters between cards horizontally, 1 line between rows vertically.
- **Rows visible:** `visible_rows = area.height / (card_height + vertical_gap)`, minimum 1.
- **Scrolling:** vertical. If rows exceed visible area, the grid scrolls to keep the selected card visible. Each tab state struct needs a `scroll_offset: usize` field added.
- **Action card:** each tab appends a `+ New` card at the end of the grid. It uses `is_action: true` and is styled with dimmed/secondary colors.

### Card Rendering

- Selected card: accent-colored border (theme.accent).
- Unselected cards: normal border color (theme.border).
- Action cards (`is_action: true`): secondary color border with centered text.
- Title line: bold, theme.fg.
- Content lines: normal, theme.fg. Truncated to 23 chars + "..." if they exceed the 26-char inner width.

### Navigation

- **h/l:** move left/right within a row. Wraps to previous/next row at edges.
- **j/k:** move down/up by one row, same column. Clamps to valid indices.
- **Enter:** triggers the tab's action for the selected card (load, view, or create).
- **a:** shortcut to create new (same as navigating to `+ New` card and pressing Enter).
- **d:** triggers delete for the selected card (ignored on action cards).
- **e:** triggers edit for the selected card (ignored on action cards). Note: Character tab currently has no edit wizard — `e` is a no-op there until character editing is implemented.
- **/:** activates search filter (filters which cards appear in the grid).

### Public API

```rust
/// Render a grid of cards into the given area.
/// Returns nothing; mutates frame directly.
pub fn render_card_grid(
    f: &mut Frame,
    area: Rect,
    cards: &[CardData],
    selected: usize,
    scroll_offset: usize,
    theme: &Theme,
);

/// Calculate grid dimensions for input handling.
pub fn grid_columns(area_width: u16, card_width: u16, gap: u16) -> usize;

/// Calculate visible rows for scroll management.
pub fn grid_visible_rows(area_height: u16, card_height: u16, gap: u16) -> usize;

/// Move selection in the grid. Returns new (selected, scroll_offset).
pub fn grid_navigate(
    direction: Direction,
    selected: usize,
    total: usize,
    columns: usize,
    scroll_offset: usize,
    visible_rows: usize,
) -> (usize, usize);

pub enum Direction { Left, Right, Up, Down }
```

## Card Content Per Tab

### Character Card

```
+----------------------------+
|  Kael                      |
|  Human [Grade F]           |
|  Lv 42                     |
|  Blacksmith                |
+----------------------------+
```

- **Title:** character name
- **Line 1:** race + grade (e.g., "Human [Grade F]"). Uses `character.race` and `character.grade` fields.
- **Line 2:** level (e.g., "Lv 42")
- **Line 3:** first profession name, or "(none)". Uses `character.professions` field.

Data source: loaded from `json_store::load_character()` for each saved name.

**Prerequisite model fields:** `Character` must have `race: String`, `grade: Grade` (enum G/F/E/D/C/B/A/S), and `professions: Vec<CharacterProfession>`. These are defined in `Docs/Technical Details.md` but not yet implemented in code.

### Skill Card

```
+----------------------------+
|  Fireball                  |
|  Active | Acquired         |
|  3 effects                 |
|  Launches a ball of fire   |
+----------------------------+
```

- **Title:** skill name
- **Line 1:** type + category (e.g., "Active | Acquired")
- **Line 2:** effect count from first rank definition (e.g., "3 effects")
- **Line 3:** description (truncated to fit card width)

Data source: `app.skill_library` Vec. All fields exist in the current `SkillDefinition` model.

### Profession Card

```
+----------------------------+
|  Blacksmith                |
|  2 base skills             |
|  Craft weapons and armor   |
|                            |
+----------------------------+
```

- **Title:** profession name
- **Line 1:** number of granted skills (e.g., "2 base skills")
- **Line 2:** description (truncated to fit card width)

Data source: `app.profession_library` Vec (replaces `app.tree_library`).

**Prerequisite model:** `ProfessionDefinition` must have `name: String`, `description: String`, and `skills: Vec<String>` (or similar). Defined in `Docs/Technical Details.md` but not yet implemented in code.

## Integration With Existing Tabs

### What Changes

- `character_creation.rs`: `render_list()` replaced with card grid rendering. The `selected` field maps to grid cursor. `char_info_string()` replaced by building `CardData` per character. Add `scroll_offset: usize` to `CharacterCreationState`.
- `skill_library.rs`: `render_list()` replaced with card grid rendering. Filter logic stays; it filters which cards are built. Add `scroll_offset: usize` to `SkillLibraryState`.
- `tree_library.rs` renamed to `profession_library.rs`. List replaced with card grid. State struct updated accordingly.
- `app.rs`: `Tab::TreeLibrary` renamed to `Tab::ProfessionLibrary`. `tree_library: Vec<TreeChain>` replaced with `profession_library: Vec<ProfessionDefinition>`.
- `main.rs`: update module references and tab dispatch.

### What Stays The Same

- All wizard modals (creation, edit) render as popups over the grid, unchanged.
- Delete confirmation popups unchanged.
- Detail views (skill detail, etc.) unchanged.
- Search/filter logic unchanged, just filters the `Vec<CardData>` before passing to grid.
- All keybindings for wizards/modals unchanged.
- The `a` shortcut for creating new items remains (in addition to the `+ New` action card).

### Navigation Change

Current list navigation uses only j/k (up/down). The grid adds h/l (left/right). This does not conflict with existing usage since h/l are only used inside wizard steps (for changing values), and the grid is only active in Browse mode.

## Files Affected

- **New:** `src/ui/card_grid.rs` — reusable card + grid module
- **Modified:** `src/ui/mod.rs` — add `pub mod card_grid`
- **Modified:** `src/ui/character_creation.rs` — replace `render_list()` with card grid, add `scroll_offset`
- **Modified:** `src/ui/skill_library.rs` — replace `render_list()` with card grid, add `scroll_offset`
- **Renamed:** `src/ui/tree_library.rs` -> `src/ui/profession_library.rs` — replace list with card grid
- **Modified:** `src/ui/app.rs` — rename `Tab::TreeLibrary` to `Tab::ProfessionLibrary`, replace `tree_library` field
- **Modified:** `src/main.rs` — update module references and tab dispatch

**Prerequisite changes (not part of this spec):**
- `src/models/character.rs` — add `race`, `grade`, `professions` fields
- `src/models/` — add profession model (ProfessionDefinition, etc.)
- `src/storage/json_store.rs` — add profession save/load
