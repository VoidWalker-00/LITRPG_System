/// Character tab — browse saved characters and create new ones.
///
/// Default view: scrollable character list with race/grade/level info.
/// Press 'a' to enter a 4-step creation wizard:
///   1. Name entry
///   2. Attribute allocation (1–10, with progress bars)
///   3. Optional innate skill selection from the library
///   4. Confirmation and save
///
/// On confirm, creates a Grade G / Level 0 character and saves to disk.

use crate::models::attribute::Attributes;
use crate::models::character::{Character, CharacterInnateSkill};
use crate::models::skill::SkillCategory;
use crate::storage::json_store;
use crate::ui::app::App;
use crate::ui::card_grid::{self, CardData, Direction};
use crate::ui::popup;
use crate::ui::theme::Theme;
use ratatui::{
    Frame,
    widgets::{Block, Borders, Paragraph},
    layout::Rect,
    style::{Style, Modifier},
    text::Line,
};
use crate::ui::divider;
use crossterm::event::{KeyCode, KeyEvent};

/// Whether we're browsing the character list or running the creation wizard.
#[derive(Debug, Clone, PartialEq)]
enum Mode {
    List,
    Wizard(CreationStep),
    ConfirmDelete,
}

/// Steps in the creation wizard.
#[derive(Debug, Clone, PartialEq)]
enum CreationStep {
    Name,
    Attributes,
    InnateSkill,
    Confirm,
}

/// All state for the Character tab.
pub struct CharacterCreationState {
    mode: Mode,
    /// Currently selected character in the list.
    selected: usize,
    /// Scroll offset for the card grid.
    scroll_offset: usize,
    /// Cached list of saved character names.
    saved_names: Vec<String>,

    // Wizard fields.
    name: String,
    attribute_values: [u32; 6],
    selected_attribute: usize,
    /// Index into innate skill list (None = no innate selected).
    innate_skill_index: Option<usize>,
    /// Which item is highlighted in the innate selection list.
    innate_cursor: usize,
}

impl CharacterCreationState {
    pub fn new() -> Self {
        Self {
            mode: Mode::List,
            selected: 0,
            scroll_offset: 0,
            saved_names: Vec::new(),
            name: String::new(),
            attribute_values: [5; 6],
            selected_attribute: 0,
            innate_skill_index: None,
            innate_cursor: 0,
        }
    }

    /// Refresh the cached character name list from disk.
    pub fn refresh_characters(&mut self, app: &App) {
        let char_dir = app.data_dir.join("characters");
        self.saved_names = if char_dir.exists() {
            json_store::list_characters(&char_dir).unwrap_or_default()
        } else {
            Vec::new()
        };
    }

    /// Render the Character tab content into the given area.
    /// Wizard steps and delete confirmation render as modal popups over the list.
    pub fn render(&mut self, f: &mut Frame, area: Rect, app: &App, theme: &Theme) {
        // Always render the list as background.
        self.render_list(f, area, app, theme);

        // Overlay wizard or delete confirmation as a modal.
        match &self.mode {
            Mode::List => {}
            Mode::Wizard(step) => {
                let step = step.clone();
                self.render_wizard_modal(f, area, app, theme, &step);
            }
            Mode::ConfirmDelete => {
                popup::render_popup(
                    f, area, theme,
                    "Delete Character",
                    &[Line::from(format!(
                        "Delete \"{}\"?",
                        self.saved_names.get(self.selected).unwrap_or(&String::new())
                    ))],
                    "[y] Yes  [n] No",
                );
            }
        }
    }

    /// Render the character card grid (default view).
    fn render_list(&self, f: &mut Frame, area: Rect, app: &App, theme: &Theme) {
        let mut lines: Vec<Line> = Vec::new();

        // Section header with count.
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
        let hint_y = area.y + area.height.saturating_sub(2);
        let hint = Paragraph::new(Line::styled(
            "  h/j/k/l: Navigate  Enter: Load  a: New  d: Delete",
            Style::default().fg(theme.dimmed),
        ));
        f.render_widget(hint, Rect::new(area.x, hint_y, area.width, 1));
    }

    /// Render the creation wizard as a modal popup over the list.
    fn render_wizard_modal(
        &self, f: &mut Frame, area: Rect, app: &App, theme: &Theme, step: &CreationStep,
    ) {
        let step_num = match step {
            CreationStep::Name => 1,
            CreationStep::Attributes => 2,
            CreationStep::InnateSkill => 3,
            CreationStep::Confirm => 4,
        };

        let title = format!(" New Character \u{2014} Step {} of 4 ", step_num);
        let mut body: Vec<Line> = Vec::new();

        match step {
            CreationStep::Name => {
                body.push(Line::from(""));
                body.push(Line::styled(
                    format!(" Character Name: {}_", self.name),
                    Style::default().fg(theme.fg),
                ));
                body.push(Line::from(""));
                popup::render_modal(f, area, theme, &title, &body, " Enter: Next  Esc: Cancel");
            }
            CreationStep::Attributes => {
                body.push(Line::from(""));
                let labels = ["Strength", "Agility", "Endurance", "Intelligence", "Wisdom", "Perception"];
                for (i, label) in labels.iter().enumerate() {
                    let prefix = if i == self.selected_attribute { " >> " } else { "    " };
                    let val = self.attribute_values[i];
                    let bar = progress_bar(val, 10);
                    let style = if i == self.selected_attribute {
                        Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(theme.fg)
                    };
                    body.push(Line::styled(
                        format!("{}{:<14}: {}  {}", prefix, label, bar, val),
                        style,
                    ));
                }
                body.push(Line::from(""));
                popup::render_modal(f, area, theme, &title, &body, " j/k: Select  h/l: Adjust  Enter: Next  Esc: Back");
            }
            CreationStep::InnateSkill => {
                body.push(Line::from(""));

                // Build the innate skill list: [None] + all innate skills.
                let innate_skills: Vec<&str> = app.skill_library
                    .iter()
                    .filter(|s| s.category == SkillCategory::Innate)
                    .map(|s| s.name.as_str())
                    .collect();

                let none_style = if self.innate_cursor == 0 {
                    Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme.fg)
                };
                let none_prefix = if self.innate_cursor == 0 { " >> " } else { "    " };
                body.push(Line::styled(format!("{}[None]", none_prefix), none_style));

                for (i, skill_name) in innate_skills.iter().enumerate() {
                    let cursor_idx = i + 1;
                    let prefix = if self.innate_cursor == cursor_idx { " >> " } else { "    " };
                    let skill_def = app.skill_library.iter()
                        .filter(|s| s.category == SkillCategory::Innate)
                        .nth(i);
                    let type_str = skill_def.map(|s| format!("[Innate | {:?}]", s.skill_type))
                        .unwrap_or_default();
                    let style = if self.innate_cursor == cursor_idx {
                        Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(theme.fg)
                    };
                    body.push(Line::styled(
                        format!("{}{:<20} {}", prefix, skill_name, type_str),
                        style,
                    ));
                }
                body.push(Line::from(""));
                popup::render_modal(f, area, theme, &title, &body, " j/k: Select  Enter: Next  Esc: Back");
            }
            CreationStep::Confirm => {
                body.push(Line::from(""));
                body.push(Line::styled(format!(" Name  : {}", self.name), Style::default().fg(theme.fg)));
                body.push(Line::styled(" Race  : Human [Grade G]", Style::default().fg(theme.fg)));
                body.push(Line::styled(" Level : 0", Style::default().fg(theme.fg)));
                body.push(Line::from(""));

                let labels_short = ["STR", "AGI", "END", "INT", "WIS", "PER"];
                body.push(Line::styled(
                    format!(" {}: {}  {}: {}  {}: {}",
                        labels_short[0], self.attribute_values[0],
                        labels_short[1], self.attribute_values[1],
                        labels_short[2], self.attribute_values[2],
                    ),
                    Style::default().fg(theme.fg),
                ));
                body.push(Line::styled(
                    format!(" {}: {}  {}: {}  {}: {}",
                        labels_short[3], self.attribute_values[3],
                        labels_short[4], self.attribute_values[4],
                        labels_short[5], self.attribute_values[5],
                    ),
                    Style::default().fg(theme.fg),
                ));
                body.push(Line::from(""));

                let innate_text = match self.innate_skill_index {
                    Some(idx) => {
                        let innate_names: Vec<&str> = app.skill_library.iter()
                            .filter(|s| s.category == SkillCategory::Innate)
                            .map(|s| s.name.as_str())
                            .collect();
                        format!(" Innate: {}", innate_names.get(idx).unwrap_or(&"None"))
                    }
                    None => " Innate: None".to_string(),
                };
                body.push(Line::styled(innate_text, Style::default().fg(theme.fg)));
                body.push(Line::from(""));
                popup::render_modal(f, area, theme, &title, &body, " Enter: Create  Esc: Back");
            }
        }
    }

    /// Handle key input. Returns true if the tab consumed the event (no global handling).
    pub fn handle_input(&mut self, key: KeyEvent, app: &mut App) -> bool {
        match &self.mode.clone() {
            Mode::List => self.handle_list_input(key, app),
            Mode::Wizard(step) => {
                let step = step.clone();
                self.handle_wizard_input(key, app, &step);
            }
            Mode::ConfirmDelete => self.handle_delete_input(key, app),
        }
        true
    }

    /// List mode: navigate grid, load, create, delete.
    fn handle_list_input(&mut self, key: KeyEvent, app: &mut App) {
        let cards_total = self.saved_names.len() + 1; // +1 for action card
        let cols = card_grid::grid_columns(80, card_grid::CARD_WIDTH, card_grid::GAP_H);
        let vis_rows = card_grid::grid_visible_rows(20, card_grid::CARD_HEIGHT, card_grid::GAP_V);

        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                let (sel, scroll) = card_grid::grid_navigate(
                    Direction::Up, self.selected, cards_total, cols, self.scroll_offset, vis_rows,
                );
                self.selected = sel;
                self.scroll_offset = scroll;
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let (sel, scroll) = card_grid::grid_navigate(
                    Direction::Down, self.selected, cards_total, cols, self.scroll_offset, vis_rows,
                );
                self.selected = sel;
                self.scroll_offset = scroll;
            }
            KeyCode::Left | KeyCode::Char('h') => {
                let (sel, scroll) = card_grid::grid_navigate(
                    Direction::Left, self.selected, cards_total, cols, self.scroll_offset, vis_rows,
                );
                self.selected = sel;
                self.scroll_offset = scroll;
            }
            KeyCode::Right | KeyCode::Char('l') => {
                let (sel, scroll) = card_grid::grid_navigate(
                    Direction::Right, self.selected, cards_total, cols, self.scroll_offset, vis_rows,
                );
                self.selected = sel;
                self.scroll_offset = scroll;
            }
            KeyCode::Enter => {
                // If on action card (last card), enter creation wizard.
                if self.selected == self.saved_names.len() {
                    self.name.clear();
                    self.attribute_values = [5; 6];
                    self.selected_attribute = 0;
                    self.innate_skill_index = None;
                    self.innate_cursor = 0;
                    self.mode = Mode::Wizard(CreationStep::Name);
                } else if let Some(name) = self.saved_names.get(self.selected) {
                    let char_dir = app.data_dir.join("characters");
                    if let Ok(character) = json_store::load_character(&char_dir, name) {
                        app.current_character = Some(character);
                        app.active_tab = crate::ui::app::Tab::SystemPanel;
                    }
                }
            }
            KeyCode::Char('a') => {
                // Enter creation wizard.
                self.name.clear();
                self.attribute_values = [5; 6];
                self.selected_attribute = 0;
                self.innate_skill_index = None;
                self.innate_cursor = 0;
                self.mode = Mode::Wizard(CreationStep::Name);
            }
            KeyCode::Char('d') => {
                if self.selected < self.saved_names.len() && !self.saved_names.is_empty() {
                    self.mode = Mode::ConfirmDelete;
                }
            }
            _ => {}
        }
    }

    /// Wizard input dispatched by step.
    fn handle_wizard_input(&mut self, key: KeyEvent, app: &mut App, step: &CreationStep) {
        match step {
            CreationStep::Name => match key.code {
                KeyCode::Char(c) => { self.name.push(c); }
                KeyCode::Backspace => { self.name.pop(); }
                KeyCode::Enter if !self.name.is_empty() => {
                    self.mode = Mode::Wizard(CreationStep::Attributes);
                }
                KeyCode::Esc => { self.mode = Mode::List; }
                _ => {}
            },
            CreationStep::Attributes => match key.code {
                KeyCode::Up | KeyCode::Char('k') => {
                    if self.selected_attribute > 0 { self.selected_attribute -= 1; }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if self.selected_attribute < 5 { self.selected_attribute += 1; }
                }
                KeyCode::Right | KeyCode::Char('l') => {
                    if self.attribute_values[self.selected_attribute] < 10 {
                        self.attribute_values[self.selected_attribute] += 1;
                    }
                }
                KeyCode::Left | KeyCode::Char('h') => {
                    if self.attribute_values[self.selected_attribute] > 1 {
                        self.attribute_values[self.selected_attribute] -= 1;
                    }
                }
                KeyCode::Enter => { self.mode = Mode::Wizard(CreationStep::InnateSkill); }
                KeyCode::Esc => { self.mode = Mode::Wizard(CreationStep::Name); }
                _ => {}
            },
            CreationStep::InnateSkill => {
                let innate_count = app.skill_library.iter()
                    .filter(|s| s.category == SkillCategory::Innate).count();
                let max_cursor = innate_count; // 0 = [None], 1..=count = skills

                match key.code {
                    KeyCode::Up | KeyCode::Char('k') => {
                        if self.innate_cursor > 0 { self.innate_cursor -= 1; }
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        if self.innate_cursor < max_cursor { self.innate_cursor += 1; }
                    }
                    KeyCode::Enter => {
                        // 0 = None, 1+ = skill index.
                        self.innate_skill_index = if self.innate_cursor == 0 {
                            None
                        } else {
                            Some(self.innate_cursor - 1)
                        };
                        self.mode = Mode::Wizard(CreationStep::Confirm);
                    }
                    KeyCode::Esc => { self.mode = Mode::Wizard(CreationStep::Attributes); }
                    _ => {}
                }
            }
            CreationStep::Confirm => match key.code {
                KeyCode::Enter => {
                    let attrs = Attributes::new_clamped(
                        self.attribute_values[0],
                        self.attribute_values[1],
                        self.attribute_values[2],
                        self.attribute_values[3],
                        self.attribute_values[4],
                        self.attribute_values[5],
                    );
                    let innate = self.innate_skill_index.map(|idx| {
                        let innate_names: Vec<&str> = app.skill_library.iter()
                            .filter(|s| s.category == SkillCategory::Innate)
                            .map(|s| s.name.as_str())
                            .collect();
                        CharacterInnateSkill {
                            definition_name: innate_names[idx].to_string(),
                            level: 1,
                        }
                    });
                    let character = Character::new(self.name.clone(), attrs, innate);
                    let char_dir = app.data_dir.join("characters");
                    let _ = json_store::save_character(&char_dir, &character);
                    app.current_character = Some(character);
                    self.mode = Mode::List;
                    self.refresh_characters(app);
                }
                KeyCode::Esc => { self.mode = Mode::Wizard(CreationStep::InnateSkill); }
                _ => {}
            },
        }
    }

    /// Delete confirmation: y to delete, n/Esc to cancel.
    fn handle_delete_input(&mut self, key: KeyEvent, app: &mut App) {
        match key.code {
            KeyCode::Char('y') => {
                if let Some(name) = self.saved_names.get(self.selected) {
                    let char_dir = app.data_dir.join("characters");
                    let _ = json_store::delete_character(&char_dir, name);
                    // If the deleted character was loaded, unload it.
                    if app.current_character.as_ref().map(|c| &c.name) == Some(name) {
                        app.current_character = None;
                    }
                }
                self.mode = Mode::List;
                self.refresh_characters(app);
                if self.selected > 0 && self.selected >= self.saved_names.len() {
                    self.selected = self.saved_names.len().saturating_sub(1);
                }
            }
            KeyCode::Char('n') | KeyCode::Esc => {
                self.mode = Mode::List;
            }
            _ => {}
        }
    }
}

/// Build card data for each saved character, with a "+ New Character" action card.
fn build_character_cards(app: &App, names: &[String]) -> Vec<CardData> {
    let char_dir = app.data_dir.join("characters");
    let mut cards: Vec<CardData> = names.iter().map(|name| {
        match json_store::load_character(&char_dir, name) {
            Ok(c) => {
                let race_grade = format!("{} [Grade {}]", c.race, c.grade.name());
                let level = format!("Lv {}", c.level);
                let class = c.classes.first()
                    .map(|p| p.definition_name.clone())
                    .unwrap_or_else(|| "(none)".to_string());
                CardData {
                    title: c.name.clone(),
                    lines: vec![race_grade, level, class],
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

/// Build a progress bar string: filled blocks + empty blocks.
/// Example for value=7, max=10: "███████░░░"
fn progress_bar(value: u32, max: u32) -> String {
    let filled = value as usize;
    let empty = (max as usize).saturating_sub(filled);
    format!("{}{}", "\u{2588}".repeat(filled), "\u{2591}".repeat(empty))
}
