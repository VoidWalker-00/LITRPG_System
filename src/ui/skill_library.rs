/// Skill Library tab — browse, create, edit, and delete skill definitions.
///
/// Default view: filterable list with text search + category/type dropdowns.
/// Press 'a' to open a skill form modal, Enter to view/edit,
/// 'd' to delete with confirmation, '/' to focus filter input.
///
/// Skill form shows name, category/type, description, and effects list.
/// Press 'a' within the form to open an effect sub-modal.
/// Ctrl+S saves and closes the current modal.

use crate::models::skill::{
    SkillCategory, SkillType, MasteryRank, SkillEffect, RankDefinition, SkillDefinition,
};
use crate::ui::card_grid::{self, CardData, Direction};
use crate::formulas::skill_scaling;
use crate::storage::json_store;
use crate::ui::app::App;
use crate::ui::popup;
use crate::ui::theme::Theme;
use crate::ui::divider;
use ratatui::{
    Frame,
    widgets::{Block, Borders, Paragraph},
    layout::Rect,
    style::{Style, Modifier},
    text::Line,
};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// Current mode of the skill library.
#[derive(Debug, Clone, PartialEq)]
enum Mode {
    Browse,
    Search,
    ViewDetail,
    SkillForm,
    EffectForm,
    ConfirmDelete,
}

/// Which field is active in the skill form.
#[derive(Debug, Clone, Copy, PartialEq)]
enum SkillField {
    Name,
    Category,
    Type,
    Description,
    Effects,
}

impl SkillField {
    const ALL: [SkillField; 5] = [
        SkillField::Name,
        SkillField::Category,
        SkillField::Type,
        SkillField::Description,
        SkillField::Effects,
    ];

    fn next(self) -> SkillField {
        match self {
            SkillField::Name => SkillField::Category,
            SkillField::Category => SkillField::Type,
            SkillField::Type => SkillField::Description,
            SkillField::Description => SkillField::Effects,
            SkillField::Effects => SkillField::Name,
        }
    }

    fn prev(self) -> SkillField {
        match self {
            SkillField::Name => SkillField::Effects,
            SkillField::Category => SkillField::Name,
            SkillField::Type => SkillField::Category,
            SkillField::Description => SkillField::Type,
            SkillField::Effects => SkillField::Description,
        }
    }
}

/// Which field is active in the effect sub-modal.
#[derive(Debug, Clone, Copy, PartialEq)]
enum EffectField {
    Name,
    Description,
    BaseValue,
    UnlockLevel,
}

impl EffectField {
    fn next(self) -> EffectField {
        match self {
            EffectField::Name => EffectField::Description,
            EffectField::Description => EffectField::BaseValue,
            EffectField::BaseValue => EffectField::UnlockLevel,
            EffectField::UnlockLevel => EffectField::Name,
        }
    }

    fn prev(self) -> EffectField {
        match self {
            EffectField::Name => EffectField::UnlockLevel,
            EffectField::Description => EffectField::Name,
            EffectField::BaseValue => EffectField::Description,
            EffectField::UnlockLevel => EffectField::BaseValue,
        }
    }
}

/// Filter state for category and type dropdowns.
#[derive(Debug, Clone, Copy, PartialEq)]
#[allow(dead_code)]
enum FilterCategory { All, Acquired, Innate, Profession }
#[derive(Debug, Clone, Copy, PartialEq)]
#[allow(dead_code)]
enum FilterType { All, Active, Passive }

/// All state for the Skill Library tab.
pub struct SkillLibraryState {
    mode: Mode,
    cursor: usize,
    search_query: String,
    filter_category: FilterCategory,
    filter_type: FilterType,
    // Skill form state.
    skill_name: String,
    skill_category: SkillCategory,
    skill_type: SkillType,
    skill_description: String,
    skill_effects: Vec<SkillEffect>,
    skill_ranks: Vec<RankDefinition>,
    skill_field: SkillField,
    // Effect sub-modal state.
    effect_name: String,
    effect_desc: String,
    effect_base: String,
    effect_unlock: String,
    effect_field: EffectField,
    // Edit tracking.
    editing_index: Option<usize>,
    /// Which effect is being edited (None = adding new).
    editing_effect_index: Option<usize>,
    /// Cursor for selecting effects in the effects list.
    effect_cursor: usize,
    /// Scroll offset for the card grid.
    scroll_offset: usize,
}

impl SkillLibraryState {
    pub fn new() -> Self {
        Self {
            mode: Mode::Browse,
            cursor: 0,
            search_query: String::new(),
            filter_category: FilterCategory::All,
            filter_type: FilterType::All,
            skill_name: String::new(),
            skill_category: SkillCategory::Acquired,
            skill_type: SkillType::Active,
            skill_description: String::new(),
            skill_effects: Vec::new(),
            skill_ranks: Vec::new(),
            skill_field: SkillField::Name,
            effect_name: String::new(),
            effect_desc: String::new(),
            effect_base: String::new(),
            effect_unlock: String::new(),
            effect_field: EffectField::Name,
            editing_index: None,
            editing_effect_index: None,
            effect_cursor: 0,
            scroll_offset: 0,
        }
    }

    /// Returns true when the skill/effect form needs Tab for field cycling.
    pub fn wants_tab(&self) -> bool {
        self.mode == Mode::SkillForm || self.mode == Mode::EffectForm
    }

    /// Reset skill form fields for a new skill.
    fn reset_skill_form(&mut self) {
        self.skill_name.clear();
        self.skill_category = SkillCategory::Acquired;
        self.skill_type = SkillType::Active;
        self.skill_description.clear();
        self.skill_effects.clear();
        self.skill_ranks.clear();
        self.skill_field = SkillField::Name;
        self.editing_index = None;
    }

    /// Load an existing skill into the form for editing.
    fn load_skill_into_form(&mut self, skill: &SkillDefinition) {
        self.skill_name = skill.name.clone();
        self.skill_category = skill.category;
        self.skill_type = skill.skill_type;
        self.skill_description = skill.description.clone();
        self.skill_effects = skill.ranks.first().map(|r| r.effects.clone()).unwrap_or_default();
        self.skill_ranks = skill.ranks.clone();
        self.skill_field = SkillField::Name;
    }

    /// Reset effect sub-modal fields.
    fn reset_effect_form(&mut self) {
        self.effect_name.clear();
        self.effect_desc.clear();
        self.effect_base.clear();
        self.effect_unlock.clear();
        self.effect_field = EffectField::Name;
        self.editing_effect_index = None;
    }

    /// Load an existing effect into the effect form for editing.
    fn load_effect_into_form(&mut self, effect: &SkillEffect) {
        self.effect_name = effect.name.clone().unwrap_or_default();
        self.effect_desc = effect.description.clone();
        self.effect_base = format!("{}", effect.base_value);
        self.effect_unlock = format!("{}", effect.unlock_level);
        self.effect_field = EffectField::Name;
    }

    /// Build a SkillDefinition from the current form state.
    fn build_skill(&self) -> SkillDefinition {
        let mut ranks = self.skill_ranks.clone();
        if ranks.is_empty() && !self.skill_effects.is_empty() {
            ranks.push(RankDefinition {
                rank: MasteryRank::Novice,
                description: self.skill_description.clone(),
                effects: self.skill_effects.clone(),
            });
        }
        SkillDefinition {
            name: self.skill_name.clone(),
            category: self.skill_category,
            skill_type: self.skill_type,
            description: self.skill_description.clone(),
            ranks,
        }
    }

    /// Get filtered skill indices based on search + filters.
    fn filtered_indices(&self, app: &App) -> Vec<usize> {
        app.skill_library.iter().enumerate()
            .filter(|(_, s)| {
                let name_match = self.search_query.is_empty()
                    || s.name.to_lowercase().contains(&self.search_query.to_lowercase());
                let cat_match = match self.filter_category {
                    FilterCategory::All => true,
                    FilterCategory::Acquired => s.category == SkillCategory::Acquired,
                    FilterCategory::Innate => s.category == SkillCategory::Innate,
                    FilterCategory::Profession => s.category == SkillCategory::Profession,
                };
                let type_match = match self.filter_type {
                    FilterType::All => true,
                    FilterType::Active => s.skill_type == SkillType::Active,
                    FilterType::Passive => s.skill_type == SkillType::Passive,
                };
                name_match && cat_match && type_match
            })
            .map(|(i, _)| i)
            .collect()
    }

    /// Render the Skill Library tab.
    pub fn render(&mut self, f: &mut Frame, area: Rect, app: &App, theme: &Theme) {
        // Always render list or detail as background.
        match &self.mode {
            Mode::ViewDetail => self.render_detail(f, area, app, theme),
            _ => self.render_list(f, area, app, theme),
        }

        // Overlay modals.
        match &self.mode {
            Mode::SkillForm => {
                self.render_skill_form(f, area, theme);
            }
            Mode::EffectForm => {
                // Render skill form underneath, then effect form on top.
                self.render_skill_form(f, area, theme);
                self.render_effect_form(f, area, theme);
            }
            Mode::ConfirmDelete => {
                let name = self.filtered_indices(app).get(self.cursor)
                    .and_then(|&i| Some(app.skill_library[i].name.clone()))
                    .unwrap_or_default();
                popup::render_popup(
                    f, area, theme,
                    "Delete Skill",
                    &[Line::from(format!("Delete \"{}\"?", name))],
                    "[y] Yes  [n] No",
                );
            }
            _ => {}
        }
    }

    /// Build card data from filtered skill indices, with a "+ New Skill" action card.
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

    /// Render the filter bar + skill card grid.
    fn render_list(&self, f: &mut Frame, area: Rect, app: &App, theme: &Theme) {
        let indices = self.filtered_indices(app);

        // Filter bar.
        let cat_label = match self.filter_category {
            FilterCategory::All => "All",
            FilterCategory::Acquired => "Acquired",
            FilterCategory::Innate => "Innate",
            FilterCategory::Profession => "Profession",
        };
        let type_label = match self.filter_type {
            FilterType::All => "All",
            FilterType::Active => "Active",
            FilterType::Passive => "Passive",
        };
        let search_display = if self.mode == Mode::Search {
            format!("{}_", self.search_query)
        } else if self.search_query.is_empty() {
            "___________".to_string()
        } else {
            self.search_query.clone()
        };
        let filter_lines = vec![
            Line::styled(
                format!("  Filter: {}  [Category: {}\u{25be}] [Type: {}\u{25be}]", search_display, cat_label, type_label),
                Style::default().fg(theme.fg),
            ),
            Line::styled(
                divider(area.width),
                Style::default().fg(theme.border),
            ),
        ];
        let filter_bar = Paragraph::new(filter_lines).block(Block::default().borders(Borders::NONE));
        let filter_area = Rect::new(area.x, area.y, area.width, 2);
        f.render_widget(filter_bar, filter_area);

        // Card grid area: below the filter bar.
        let grid_y = area.y + 2;
        let grid_height = area.height.saturating_sub(4); // 2 for filter bar, 2 for help hint
        if grid_height > 0 {
            let grid_area = Rect::new(area.x + 1, grid_y, area.width.saturating_sub(2), grid_height);
            let cards = Self::build_skill_cards(&app.skill_library, &indices);
            card_grid::render_card_grid(f, grid_area, &cards, self.cursor, self.scroll_offset, theme);
        }

        // Help hint at bottom.
        let hint_y = area.y + area.height.saturating_sub(2);
        let hint = Paragraph::new(Line::styled(
            "  h/j/k/l: Navigate  Enter: View  a: Add  e: Edit  d: Delete  /: Search",
            Style::default().fg(theme.dimmed),
        ));
        f.render_widget(hint, Rect::new(area.x, hint_y, area.width, 1));
    }

    /// Render detail view for the selected skill.
    fn render_detail(&self, f: &mut Frame, area: Rect, app: &App, theme: &Theme) {
        let indices = self.filtered_indices(app);
        let Some(&skill_idx) = indices.get(self.cursor) else {
            let msg = Paragraph::new("  No skill selected.")
                .style(Style::default().fg(theme.dimmed));
            f.render_widget(msg, area);
            return;
        };
        let skill = &app.skill_library[skill_idx];
        let mut lines: Vec<Line> = Vec::new();

        lines.push(Line::styled(
            format!("  Name: {}", skill.name),
            Style::default().fg(theme.fg).add_modifier(Modifier::BOLD),
        ));
        lines.push(Line::styled(
            format!("  Category: {:?} | Type: {:?}", skill.category, skill.skill_type),
            Style::default().fg(theme.fg),
        ));
        lines.push(Line::styled(
            format!("  Description: {}", skill.description),
            Style::default().fg(theme.fg),
        ));
        lines.push(Line::from(""));

        for rank_def in &skill.ranks {
            lines.push(Line::styled(
                format!("  \u{2500}\u{2500}\u{2500} {} \u{2500}\u{2500}\u{2500}", rank_def.rank.name()),
                Style::default().fg(theme.secondary),
            ));
            lines.push(Line::styled(
                format!("    {}", rank_def.description),
                Style::default().fg(theme.dimmed),
            ));
            for effect in &rank_def.effects {
                let previews: Vec<String> = [1, 5, 10, 25].iter()
                    .filter(|&&l| l <= rank_def.rank.max_level())
                    .map(|&l| {
                        let val = skill_scaling::acquired_effect_value(
                            effect.base_value, l, rank_def.rank, skill.skill_type,
                        );
                        format!("L{}: {:.1}", l, val)
                    })
                    .collect();
                let effect_label = effect.name.as_deref().unwrap_or("Effect");
                lines.push(Line::styled(
                    format!("    {}: base {:.1}  |  {}", effect_label, effect.base_value, previews.join("  ")),
                    Style::default().fg(theme.fg),
                ));
                // Show effect description if present.
                if !effect.description.is_empty() {
                    lines.push(Line::styled(
                        format!("      {}", effect.description),
                        Style::default().fg(theme.dimmed),
                    ));
                }
            }
            lines.push(Line::from(""));
        }

        lines.push(Line::styled("  Esc: Back to list", Style::default().fg(theme.dimmed)));

        let content = Paragraph::new(lines).block(Block::default().borders(Borders::NONE));
        f.render_widget(content, area);
    }

    /// Render the skill form modal.
    fn render_skill_form(&self, f: &mut Frame, area: Rect, theme: &Theme) {
        let title = if self.editing_index.is_some() { " Edit Skill " } else { " New Skill " };
        let mut body: Vec<Line> = Vec::new();

        body.push(Line::from(""));

        // Name field.
        let name_marker = if self.skill_field == SkillField::Name { " > " } else { "   " };
        let name_style = if self.skill_field == SkillField::Name {
            Style::default().fg(theme.accent)
        } else {
            Style::default().fg(theme.fg)
        };
        body.push(Line::styled(
            format!("{}Name: {}{}", name_marker, self.skill_name, if self.skill_field == SkillField::Name { "_" } else { "" }),
            name_style,
        ));

        // Category field.
        let cat_marker = if self.skill_field == SkillField::Category { " > " } else { "   " };
        let cat_style = if self.skill_field == SkillField::Category {
            Style::default().fg(theme.accent)
        } else {
            Style::default().fg(theme.fg)
        };
        body.push(Line::styled(
            format!("{}Category: {:?}{}", cat_marker, self.skill_category,
                if self.skill_field == SkillField::Category { "  (h/l)" } else { "" }),
            cat_style,
        ));

        // Type field.
        let type_marker = if self.skill_field == SkillField::Type { " > " } else { "   " };
        let type_style = if self.skill_field == SkillField::Type {
            Style::default().fg(theme.accent)
        } else {
            Style::default().fg(theme.fg)
        };
        body.push(Line::styled(
            format!("{}Type: {:?}{}", type_marker, self.skill_type,
                if self.skill_field == SkillField::Type { "  (h/l)" } else { "" }),
            type_style,
        ));

        // Description field.
        let desc_marker = if self.skill_field == SkillField::Description { " > " } else { "   " };
        let desc_style = if self.skill_field == SkillField::Description {
            Style::default().fg(theme.accent)
        } else {
            Style::default().fg(theme.fg)
        };
        body.push(Line::styled(
            format!("{}Desc: {}{}", desc_marker, self.skill_description,
                if self.skill_field == SkillField::Description { "_" } else { "" }),
            desc_style,
        ));

        body.push(Line::from(""));

        // Effects section.
        let effects_marker = if self.skill_field == SkillField::Effects { " > " } else { "   " };
        let effects_style = if self.skill_field == SkillField::Effects {
            Style::default().fg(theme.accent)
        } else {
            Style::default().fg(theme.secondary)
        };
        body.push(Line::styled(
            format!("{}Effects ({}):", effects_marker, self.skill_effects.len()),
            effects_style,
        ));

        if self.skill_effects.is_empty() {
            body.push(Line::styled("     (none)", Style::default().fg(theme.dimmed)));
        } else {
            let on_effects = self.skill_field == SkillField::Effects;
            for (i, e) in self.skill_effects.iter().enumerate() {
                let label = e.name.as_deref().unwrap_or("Effect");
                let prefix = if on_effects && i == self.effect_cursor { "  >> " } else { "     " };
                let style = if on_effects && i == self.effect_cursor {
                    Style::default().fg(theme.accent)
                } else {
                    Style::default().fg(theme.fg)
                };
                body.push(Line::styled(
                    format!("{}{}. {} (base: {:.1}, unlock: L{})", prefix, i + 1, label, e.base_value, e.unlock_level),
                    style,
                ));
                if !e.description.is_empty() {
                    body.push(Line::styled(
                        format!("        {}", e.description),
                        Style::default().fg(theme.dimmed),
                    ));
                }
            }
        }

        body.push(Line::from(""));
        popup::render_modal(f, area, theme, title, &body, " Tab: field | a: add effect | e: edit effect | Ctrl+S: save | Esc: cancel");
    }

    /// Render the effect sub-modal on top of the skill form.
    fn render_effect_form(&self, f: &mut Frame, area: Rect, theme: &Theme) {
        let title = if self.editing_effect_index.is_some() { " Edit Effect " } else { " Add Effect " };
        let mut body: Vec<Line> = Vec::new();

        body.push(Line::from(""));

        let fields: [(EffectField, &str, &str); 4] = [
            (EffectField::Name, "Name", &self.effect_name),
            (EffectField::Description, "Desc", &self.effect_desc),
            (EffectField::BaseValue, "Base Value", &self.effect_base),
            (EffectField::UnlockLevel, "Unlock Lv", &self.effect_unlock),
        ];

        for (field, label, value) in &fields {
            let marker = if self.effect_field == *field { " > " } else { "   " };
            let style = if self.effect_field == *field {
                Style::default().fg(theme.accent)
            } else {
                Style::default().fg(theme.fg)
            };
            body.push(Line::styled(
                format!("{}{}: {}{}", marker, label, value, if self.effect_field == *field { "_" } else { "" }),
                style,
            ));
        }

        body.push(Line::from(""));
        popup::render_modal(f, area, theme, title, &body, " Tab: next field | Ctrl+S: save effect | Esc: cancel");
    }

    /// Handle key input. Returns true (always consumed by tab).
    pub fn handle_input(&mut self, key: KeyEvent, app: &mut App) -> bool {
        match &self.mode.clone() {
            Mode::Browse => self.handle_browse(key, app),
            Mode::Search => self.handle_search(key),
            Mode::ViewDetail => self.handle_detail(key, app),
            Mode::SkillForm => self.handle_skill_form(key, app),
            Mode::EffectForm => self.handle_effect_form(key),
            Mode::ConfirmDelete => self.handle_delete(key, app),
        }
        true
    }

    /// Browse mode input with grid navigation.
    fn handle_browse(&mut self, key: KeyEvent, app: &App) {
        let indices = self.filtered_indices(app);
        let cards_total = indices.len() + 1; // +1 for action card
        let cols = card_grid::grid_columns(80, card_grid::CARD_WIDTH, card_grid::GAP_H);
        let vis_rows = card_grid::grid_visible_rows(20, card_grid::CARD_HEIGHT, card_grid::GAP_V);

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
            KeyCode::Char('a') => {
                self.reset_skill_form();
                self.mode = Mode::SkillForm;
            }
            KeyCode::Char('e') => {
                if self.cursor < indices.len() {
                    if let Some(&idx) = indices.get(self.cursor) {
                        self.reset_skill_form();
                        self.load_skill_into_form(&app.skill_library[idx]);
                        self.editing_index = Some(idx);
                        self.mode = Mode::SkillForm;
                    }
                }
            }
            KeyCode::Char('d') => {
                if self.cursor < indices.len() && !indices.is_empty() {
                    self.mode = Mode::ConfirmDelete;
                }
            }
            KeyCode::Char('/') => {
                self.mode = Mode::Search;
            }
            KeyCode::Esc => {
                if !self.search_query.is_empty() {
                    self.search_query.clear();
                    self.cursor = 0;
                    self.scroll_offset = 0;
                }
            }
            _ => {}
        }
    }

    /// Search mode: type to filter.
    fn handle_search(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char(c) => { self.search_query.push(c); }
            KeyCode::Backspace => { self.search_query.pop(); }
            KeyCode::Enter | KeyCode::Esc => {
                self.mode = Mode::Browse;
                self.cursor = 0;
            }
            _ => {}
        }
    }

    /// Detail view: Esc to go back, e to edit.
    fn handle_detail(&mut self, key: KeyEvent, app: &App) {
        match key.code {
            KeyCode::Esc => { self.mode = Mode::Browse; }
            KeyCode::Char('e') => {
                let indices = self.filtered_indices(app);
                if let Some(&idx) = indices.get(self.cursor) {
                    self.reset_skill_form();
                    self.load_skill_into_form(&app.skill_library[idx]);
                    self.editing_index = Some(idx);
                    self.mode = Mode::SkillForm;
                }
            }
            _ => {}
        }
    }

    /// Skill form modal input.
    fn handle_skill_form(&mut self, key: KeyEvent, app: &mut App) {
        // Ctrl+S saves the skill.
        if key.code == KeyCode::Char('s') && key.modifiers.contains(KeyModifiers::CONTROL) {
            if !self.skill_name.is_empty() {
                let skill = self.build_skill();
                if let Some(idx) = self.editing_index {
                    app.skill_library[idx] = skill;
                } else {
                    app.skill_library.push(skill);
                }
                let path = app.data_dir.join("skills.json");
                let _ = json_store::save_json(&path, &app.skill_library);
                self.mode = Mode::Browse;
                self.cursor = 0;
            }
            return;
        }

        // Tab cycles fields.
        if key.code == KeyCode::Tab {
            if key.modifiers.contains(KeyModifiers::SHIFT) {
                self.skill_field = self.skill_field.prev();
            } else {
                self.skill_field = self.skill_field.next();
            }
            return;
        }

        // Esc cancels.
        if key.code == KeyCode::Esc {
            self.mode = Mode::Browse;
            return;
        }

        // Field-specific input.
        match self.skill_field {
            SkillField::Name => match key.code {
                KeyCode::Char(c) => { self.skill_name.push(c); }
                KeyCode::Backspace => { self.skill_name.pop(); }
                _ => {}
            },
            SkillField::Category => match key.code {
                KeyCode::Left | KeyCode::Char('h') | KeyCode::Right | KeyCode::Char('l') => {
                    self.skill_category = match self.skill_category {
                        SkillCategory::Acquired => SkillCategory::Innate,
                        SkillCategory::Innate => SkillCategory::Profession,
                        SkillCategory::Profession => SkillCategory::Acquired,
                    };
                }
                _ => {}
            },
            SkillField::Type => match key.code {
                KeyCode::Left | KeyCode::Char('h') | KeyCode::Right | KeyCode::Char('l') => {
                    self.skill_type = match self.skill_type {
                        SkillType::Active => SkillType::Passive,
                        SkillType::Passive => SkillType::Active,
                    };
                }
                _ => {}
            },
            SkillField::Description => match key.code {
                KeyCode::Char(c) => { self.skill_description.push(c); }
                KeyCode::Backspace => { self.skill_description.pop(); }
                _ => {}
            },
            SkillField::Effects => match key.code {
                KeyCode::Char('a') => {
                    self.reset_effect_form();
                    self.mode = Mode::EffectForm;
                }
                KeyCode::Char('e') => {
                    if let Some(effect) = self.skill_effects.get(self.effect_cursor) {
                        let effect = effect.clone();
                        self.load_effect_into_form(&effect);
                        self.editing_effect_index = Some(self.effect_cursor);
                        self.mode = Mode::EffectForm;
                    }
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    if self.effect_cursor > 0 { self.effect_cursor -= 1; }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if self.effect_cursor < self.skill_effects.len().saturating_sub(1) {
                        self.effect_cursor += 1;
                    }
                }
                KeyCode::Char('d') => {
                    if !self.skill_effects.is_empty() {
                        self.skill_effects.remove(self.effect_cursor);
                        if self.effect_cursor > 0 && self.effect_cursor >= self.skill_effects.len() {
                            self.effect_cursor = self.skill_effects.len().saturating_sub(1);
                        }
                    }
                }
                _ => {}
            },
        }
    }

    /// Effect sub-modal input.
    fn handle_effect_form(&mut self, key: KeyEvent) {
        // Ctrl+S saves the effect and returns to skill form.
        if key.code == KeyCode::Char('s') && key.modifiers.contains(KeyModifiers::CONTROL) {
            if !self.effect_base.is_empty() {
                let name = if self.effect_name.is_empty() {
                    None
                } else {
                    Some(self.effect_name.clone())
                };
                let base = self.effect_base.parse::<f64>().unwrap_or(0.0);
                let unlock = self.effect_unlock.parse::<u32>().unwrap_or(0);
                let effect = SkillEffect {
                    name,
                    description: self.effect_desc.clone(),
                    base_value: base,
                    unlock_level: unlock,
                };
                if let Some(idx) = self.editing_effect_index {
                    // Replace existing effect.
                    self.skill_effects[idx] = effect;
                } else {
                    // Add new effect.
                    self.skill_effects.push(effect);
                }
            }
            self.editing_effect_index = None;
            self.mode = Mode::SkillForm;
            return;
        }

        // Tab cycles fields.
        if key.code == KeyCode::Tab {
            if key.modifiers.contains(KeyModifiers::SHIFT) {
                self.effect_field = self.effect_field.prev();
            } else {
                self.effect_field = self.effect_field.next();
            }
            return;
        }

        // Esc cancels without saving.
        if key.code == KeyCode::Esc {
            self.mode = Mode::SkillForm;
            return;
        }

        // Field-specific input.
        match self.effect_field {
            EffectField::Name => match key.code {
                KeyCode::Char(c) => { self.effect_name.push(c); }
                KeyCode::Backspace => { self.effect_name.pop(); }
                _ => {}
            },
            EffectField::Description => match key.code {
                KeyCode::Char(c) => { self.effect_desc.push(c); }
                KeyCode::Backspace => { self.effect_desc.pop(); }
                _ => {}
            },
            EffectField::BaseValue => match key.code {
                KeyCode::Char(c) if c.is_ascii_digit() || c == '.' => { self.effect_base.push(c); }
                KeyCode::Backspace => { self.effect_base.pop(); }
                _ => {}
            },
            EffectField::UnlockLevel => match key.code {
                KeyCode::Char(c) if c.is_ascii_digit() => { self.effect_unlock.push(c); }
                KeyCode::Backspace => { self.effect_unlock.pop(); }
                _ => {}
            },
        }
    }

    /// Delete confirmation.
    fn handle_delete(&mut self, key: KeyEvent, app: &mut App) {
        match key.code {
            KeyCode::Char('y') => {
                let indices = self.filtered_indices(app);
                if let Some(&idx) = indices.get(self.cursor) {
                    app.skill_library.remove(idx);
                    let path = app.data_dir.join("skills.json");
                    let _ = json_store::save_json(&path, &app.skill_library);
                    self.cursor = self.cursor.min(app.skill_library.len().saturating_sub(1));
                }
                self.mode = Mode::Browse;
            }
            KeyCode::Char('n') | KeyCode::Esc => { self.mode = Mode::Browse; }
            _ => {}
        }
    }
}
