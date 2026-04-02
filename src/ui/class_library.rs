/// Class Library tab — browse, create, and delete class definitions.
///
/// Default view: card grid of classes with search filter.
/// Press 'a' to create a new class, Enter to view details,
/// 'd' to delete with confirmation, '/' to search.

use crate::models::class::ClassDefinition;
use crate::storage::json_store;
use crate::ui::app::App;
use crate::ui::card_grid::{self, CardData, Direction};
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
use crossterm::event::{KeyCode, KeyEvent};

/// Current mode of the class library.
#[derive(Debug, Clone, PartialEq)]
enum Mode {
    Browse,
    Search,
    ViewDetail,
    Wizard(WizardStep),
    ConfirmDelete,
}

/// Steps in the class creation wizard.
#[derive(Debug, Clone, PartialEq)]
enum WizardStep {
    Name,
    Description,
    Skills,
    Passive,
    BonusAttrs,
    Confirm,
}

/// All state for the Class Library tab.
pub struct ClassLibraryState {
    mode: Mode,
    cursor: usize,
    scroll_offset: usize,
    search_query: String,
    wizard: ClassWizard,
}

/// State for the class creation wizard.
struct ClassWizard {
    name: String,
    description: String,
    skills_input: String,
    skills: Vec<String>,
    passive_name: String,
    passive_description: String,
    // Per-stat bonus inputs (digits only); index = field cursor for BonusAttrs step.
    // Order: 0=STR 1=AGI 2=END 3=INT 4=WIS 5=PER 6=Free
    bonus_inputs: [String; 7],
    bonus_field: usize,
}

const BONUS_LABELS: [&str; 7] = ["STR", "AGI", "END", "INT", "WIS", "PER", "Free"];

impl ClassWizard {
    fn new() -> Self {
        Self {
            name: String::new(),
            description: String::new(),
            skills_input: String::new(),
            skills: Vec::new(),
            passive_name: String::new(),
            passive_description: String::new(),
            bonus_inputs: Default::default(),
            bonus_field: 0,
        }
    }

    fn bonus_val(&self, i: usize) -> u32 {
        self.bonus_inputs[i].parse().unwrap_or(0)
    }

    /// Build a ClassDefinition from wizard state.
    fn build(&self) -> ClassDefinition {
        ClassDefinition {
            name: self.name.clone(),
            description: self.description.clone(),
            skills: self.skills.clone(),
            passive_name: self.passive_name.clone(),
            passive_description: self.passive_description.clone(),
            bonus_str: self.bonus_val(0),
            bonus_agi: self.bonus_val(1),
            bonus_end: self.bonus_val(2),
            bonus_int: self.bonus_val(3),
            bonus_wis: self.bonus_val(4),
            bonus_per: self.bonus_val(5),
            bonus_free_points: self.bonus_val(6),
        }
    }
}

impl ClassLibraryState {
    pub fn new() -> Self {
        Self {
            mode: Mode::Browse,
            cursor: 0,
            scroll_offset: 0,
            search_query: String::new(),
            wizard: ClassWizard::new(),
        }
    }

    /// Get filtered class indices.
    fn filtered_indices(&self, app: &App) -> Vec<usize> {
        app.class_library.iter().enumerate()
            .filter(|(_, p)| {
                self.search_query.is_empty()
                    || p.name.to_lowercase().contains(&self.search_query.to_lowercase())
            })
            .map(|(i, _)| i)
            .collect()
    }

    /// Build card data for classes with a "+ New Class" action card.
    fn build_class_cards(classes: &[ClassDefinition], indices: &[usize]) -> Vec<CardData> {
        let mut cards: Vec<CardData> = indices.iter().map(|&i| {
            let cls = &classes[i];
            CardData {
                title: cls.name.clone(),
                lines: vec![
                    format!("{} base skills", cls.skills.len()),
                    card_grid::truncate_line(&cls.description, 24),
                ],
                is_action: false,
            }
        }).collect();
        cards.push(CardData {
            title: "+ New Class".to_string(),
            lines: vec![],
            is_action: true,
        });
        cards
    }

    /// Render the Class Library tab.
    pub fn render(&mut self, f: &mut Frame, area: Rect, app: &App, theme: &Theme) {
        // Always render list or detail as background.
        match &self.mode {
            Mode::ViewDetail => self.render_detail(f, area, app, theme),
            _ => self.render_list(f, area, app, theme),
        }

        // Overlay wizard or delete confirmation as modal.
        match &self.mode {
            Mode::Wizard(step) => {
                let step = step.clone();
                self.render_wizard_modal(f, area, theme, &step);
            }
            Mode::ConfirmDelete => {
                let name = self.filtered_indices(app).get(self.cursor)
                    .and_then(|&i| Some(app.class_library[i].name.clone()))
                    .unwrap_or_default();
                popup::render_popup(
                    f, area, theme,
                    "Delete Class",
                    &[Line::from(format!("Delete \"{}\"?", name))],
                    "[y] Yes  [n] No",
                );
            }
            _ => {}
        }
    }

    /// Render the filter bar + class card grid.
    fn render_list(&self, f: &mut Frame, area: Rect, app: &App, theme: &Theme) {
        let indices = self.filtered_indices(app);

        // Filter bar.
        let search_display = if self.mode == Mode::Search {
            format!("{}_", self.search_query)
        } else if self.search_query.is_empty() {
            "___________".to_string()
        } else {
            self.search_query.clone()
        };
        let filter_lines = vec![
            Line::styled(
                format!("  Filter: {}", search_display),
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

        // Card grid area.
        let grid_y = area.y + 2;
        let grid_height = area.height.saturating_sub(4);
        if grid_height > 0 {
            let grid_area = Rect::new(area.x + 1, grid_y, area.width.saturating_sub(2), grid_height);
            let cards = Self::build_class_cards(&app.class_library, &indices);
            card_grid::render_card_grid(f, grid_area, &cards, self.cursor, self.scroll_offset, theme);
        }

        // Help hint at bottom.
        let hint_y = area.y + area.height.saturating_sub(2);
        let hint = Paragraph::new(Line::styled(
            "  h/j/k/l: Navigate  Enter: View  a: Add  d: Delete  /: Search",
            Style::default().fg(theme.dimmed),
        ));
        f.render_widget(hint, Rect::new(area.x, hint_y, area.width, 1));
    }

    /// Render detail view for the selected class.
    fn render_detail(&self, f: &mut Frame, area: Rect, app: &App, theme: &Theme) {
        let indices = self.filtered_indices(app);
        let Some(&prof_idx) = indices.get(self.cursor) else {
            let msg = Paragraph::new("  No class selected.")
                .style(Style::default().fg(theme.dimmed));
            f.render_widget(msg, area);
            return;
        };
        let prof = &app.class_library[prof_idx];
        let mut lines: Vec<Line> = Vec::new();

        lines.push(Line::styled(
            format!("  {}", prof.name),
            Style::default().fg(theme.fg).add_modifier(Modifier::BOLD),
        ));
        lines.push(Line::styled(
            format!("  {}", prof.description),
            Style::default().fg(theme.fg),
        ));
        lines.push(Line::from(""));

        lines.push(Line::styled(
            "  Skills",
            Style::default().fg(theme.secondary).add_modifier(Modifier::BOLD),
        ));
        if prof.skills.is_empty() {
            lines.push(Line::styled("     (none)", Style::default().fg(theme.dimmed)));
        } else {
            for skill in &prof.skills {
                lines.push(Line::styled(
                    format!("     {}", skill),
                    Style::default().fg(theme.fg),
                ));
            }
        }
        lines.push(Line::from(""));

        lines.push(Line::styled(
            "  Passive",
            Style::default().fg(theme.secondary).add_modifier(Modifier::BOLD),
        ));
        lines.push(Line::styled(
            format!("     {}: {}", prof.passive_name, prof.passive_description),
            Style::default().fg(theme.fg),
        ));
        lines.push(Line::from(""));
        lines.push(Line::styled(
            "  Bonus Attribute Points",
            Style::default().fg(theme.secondary).add_modifier(Modifier::BOLD),
        ));
        let bonus_parts: Vec<String> = [
            (prof.bonus_str, "STR"), (prof.bonus_agi, "AGI"), (prof.bonus_end, "END"),
            (prof.bonus_int, "INT"), (prof.bonus_wis, "WIS"), (prof.bonus_per, "PER"),
            (prof.bonus_free_points, "Free"),
        ].iter().filter(|(v, _)| *v > 0).map(|(v, l)| format!("+{} {}", v, l)).collect();
        let bonus_display = if bonus_parts.is_empty() { "none".to_string() } else { bonus_parts.join(", ") };
        lines.push(Line::styled(
            format!("     {} per level", bonus_display),
            Style::default().fg(theme.fg),
        ));
        lines.push(Line::from(""));
        lines.push(Line::styled("  Esc: Back to list", Style::default().fg(theme.dimmed)));

        let content = Paragraph::new(lines).block(Block::default().borders(Borders::NONE));
        f.render_widget(content, area);
    }

    /// Render the wizard as a modal popup.
    fn render_wizard_modal(&self, f: &mut Frame, area: Rect, theme: &Theme, step: &WizardStep) {
        let step_num = match step {
            WizardStep::Name => 1,
            WizardStep::Description => 2,
            WizardStep::Skills => 3,
            WizardStep::Passive => 4,
            WizardStep::BonusAttrs => 5,
            WizardStep::Confirm => 6,
        };

        let title = format!(" New Class \u{2014} Step {} of 6 ", step_num);
        let mut body: Vec<Line> = Vec::new();

        match step {
            WizardStep::Name => {
                body.push(Line::from(""));
                body.push(Line::styled(
                    format!(" Name: {}_", self.wizard.name),
                    Style::default().fg(theme.fg),
                ));
                body.push(Line::from(""));
                popup::render_modal(f, area, theme, &title, &body, " Enter: Next  Esc: Cancel");
            }
            WizardStep::Description => {
                body.push(Line::from(""));
                body.push(Line::styled(
                    format!(" Description: {}_", self.wizard.description),
                    Style::default().fg(theme.fg),
                ));
                body.push(Line::from(""));
                popup::render_modal(f, area, theme, &title, &body, " Enter: Next  Esc: Back");
            }
            WizardStep::Skills => {
                body.push(Line::from(""));
                body.push(Line::styled(" Skills (Enter to add, then type next):", Style::default().fg(theme.secondary)));
                for (i, skill) in self.wizard.skills.iter().enumerate() {
                    body.push(Line::styled(
                        format!("   {}. {}", i + 1, skill),
                        Style::default().fg(theme.fg),
                    ));
                }
                body.push(Line::styled(
                    format!("   > {}_", self.wizard.skills_input),
                    Style::default().fg(theme.accent),
                ));
                body.push(Line::from(""));
                popup::render_modal(f, area, theme, &title, &body, " Enter: Add skill  Ctrl+N: Next step  Esc: Back");
            }
            WizardStep::Passive => {
                body.push(Line::from(""));
                body.push(Line::styled(
                    format!(" Passive Name: {}_", self.wizard.passive_name),
                    Style::default().fg(theme.fg),
                ));
                body.push(Line::styled(
                    format!(" Passive Desc: {}", self.wizard.passive_description),
                    Style::default().fg(theme.dimmed),
                ));
                body.push(Line::from(""));
                popup::render_modal(f, area, theme, &title, &body, " Tab: switch field  Enter: Next  Esc: Back");
            }
            WizardStep::BonusAttrs => {
                body.push(Line::from(""));
                body.push(Line::styled(
                    " Bonus per level-up (Tab: next field):",
                    Style::default().fg(theme.secondary),
                ));
                body.push(Line::from(""));
                for (i, label) in BONUS_LABELS.iter().enumerate() {
                    let val = &self.wizard.bonus_inputs[i];
                    let active = i == self.wizard.bonus_field;
                    let display = if active {
                        format!("  {:>4}: {}_", label, val)
                    } else {
                        format!("  {:>4}: {}", label, if val.is_empty() { "0" } else { val })
                    };
                    let style = if active {
                        Style::default().fg(theme.accent)
                    } else {
                        Style::default().fg(theme.fg)
                    };
                    body.push(Line::styled(display, style));
                }
                body.push(Line::from(""));
                popup::render_modal(f, area, theme, &title, &body, " Tab: next  Enter: Next  Esc: Back");
            }
            WizardStep::Confirm => {
                let bonus_parts: Vec<String> = BONUS_LABELS.iter().enumerate()
                    .filter_map(|(i, lbl)| {
                        let v = self.wizard.bonus_val(i);
                        if v > 0 { Some(format!("+{} {}", v, lbl)) } else { None }
                    })
                    .collect();
                let bonus_str = if bonus_parts.is_empty() { "none".to_string() } else { bonus_parts.join(", ") };
                body.push(Line::from(""));
                body.push(Line::styled(format!(" Name    : {}", self.wizard.name), Style::default().fg(theme.fg)));
                body.push(Line::styled(format!(" Desc    : {}", self.wizard.description), Style::default().fg(theme.fg)));
                body.push(Line::styled(format!(" Skills  : {}", self.wizard.skills.join(", ")), Style::default().fg(theme.fg)));
                body.push(Line::styled(format!(" Passive : {}", self.wizard.passive_name), Style::default().fg(theme.fg)));
                body.push(Line::styled(format!(" Bonuses : {}", bonus_str), Style::default().fg(theme.fg)));
                body.push(Line::from(""));
                popup::render_modal(f, area, theme, &title, &body, " Enter: Create  Esc: Back");
            }
        }
    }

    /// Handle key input.
    pub fn handle_input(&mut self, key: KeyEvent, app: &mut App) -> bool {
        match &self.mode.clone() {
            Mode::Browse => self.handle_browse(key, app),
            Mode::Search => self.handle_search(key),
            Mode::ViewDetail => self.handle_detail(key),
            Mode::Wizard(step) => {
                let step = step.clone();
                self.handle_wizard(key, app, &step);
            }
            Mode::ConfirmDelete => self.handle_delete(key, app),
        }
        true
    }

    /// Browse mode with grid navigation.
    fn handle_browse(&mut self, key: KeyEvent, app: &App) {
        let indices = self.filtered_indices(app);
        let cards_total = indices.len() + 1;
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
                if self.cursor == indices.len() {
                    self.wizard = ClassWizard::new();
                    self.mode = Mode::Wizard(WizardStep::Name);
                } else if !indices.is_empty() {
                    self.mode = Mode::ViewDetail;
                }
            }
            KeyCode::Char('a') => {
                self.wizard = ClassWizard::new();
                self.mode = Mode::Wizard(WizardStep::Name);
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

    /// Search mode.
    fn handle_search(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char(c) => { self.search_query.push(c); }
            KeyCode::Backspace => { self.search_query.pop(); }
            KeyCode::Enter | KeyCode::Esc => {
                self.mode = Mode::Browse;
                self.cursor = 0;
                self.scroll_offset = 0;
            }
            _ => {}
        }
    }

    /// Detail view.
    fn handle_detail(&mut self, key: KeyEvent) {
        if key.code == KeyCode::Esc { self.mode = Mode::Browse; }
    }

    /// Wizard input.
    fn handle_wizard(&mut self, key: KeyEvent, app: &mut App, step: &WizardStep) {
        match step {
            WizardStep::Name => match key.code {
                KeyCode::Char(c) => { self.wizard.name.push(c); }
                KeyCode::Backspace => { self.wizard.name.pop(); }
                KeyCode::Enter if !self.wizard.name.is_empty() => {
                    self.mode = Mode::Wizard(WizardStep::Description);
                }
                KeyCode::Esc => { self.mode = Mode::Browse; }
                _ => {}
            },
            WizardStep::Description => match key.code {
                KeyCode::Char(c) => { self.wizard.description.push(c); }
                KeyCode::Backspace => { self.wizard.description.pop(); }
                KeyCode::Enter if !self.wizard.description.is_empty() => {
                    self.mode = Mode::Wizard(WizardStep::Skills);
                }
                KeyCode::Esc => { self.mode = Mode::Wizard(WizardStep::Name); }
                _ => {}
            },
            WizardStep::Skills => {
                if key.code == KeyCode::Char('n') && key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) {
                    self.mode = Mode::Wizard(WizardStep::Passive);
                    return;
                }
                match key.code {
                    KeyCode::Char(c) => { self.wizard.skills_input.push(c); }
                    KeyCode::Backspace => { self.wizard.skills_input.pop(); }
                    KeyCode::Enter => {
                        if !self.wizard.skills_input.is_empty() {
                            let skill = self.wizard.skills_input.clone();
                            self.wizard.skills.push(skill);
                            self.wizard.skills_input.clear();
                        }
                    }
                    KeyCode::Esc => { self.mode = Mode::Wizard(WizardStep::Description); }
                    _ => {}
                }
            }
            WizardStep::Passive => match key.code {
                KeyCode::Char(c) => { self.wizard.passive_name.push(c); }
                KeyCode::Backspace => { self.wizard.passive_name.pop(); }
                KeyCode::Enter if !self.wizard.passive_name.is_empty() => {
                    self.mode = Mode::Wizard(WizardStep::BonusAttrs);
                }
                KeyCode::Esc => { self.mode = Mode::Wizard(WizardStep::Skills); }
                _ => {}
            },
            WizardStep::BonusAttrs => match key.code {
                KeyCode::Char(c) if c.is_ascii_digit() => {
                    self.wizard.bonus_inputs[self.wizard.bonus_field].push(c);
                }
                KeyCode::Backspace => {
                    self.wizard.bonus_inputs[self.wizard.bonus_field].pop();
                }
                KeyCode::Tab => {
                    self.wizard.bonus_field = (self.wizard.bonus_field + 1) % BONUS_LABELS.len();
                }
                KeyCode::Enter => {
                    self.mode = Mode::Wizard(WizardStep::Confirm);
                }
                KeyCode::Esc => { self.mode = Mode::Wizard(WizardStep::Passive); }
                _ => {}
            },
            WizardStep::Confirm => match key.code {
                KeyCode::Enter => {
                    let cls = self.wizard.build();
                    app.class_library.push(cls);
                    let path = app.data_dir.join("classes.json");
                    let _ = json_store::save_json(&path, &app.class_library);
                    self.mode = Mode::Browse;
                    self.cursor = 0;
                    self.scroll_offset = 0;
                }
                KeyCode::Esc => { self.mode = Mode::Wizard(WizardStep::BonusAttrs); }
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
                    app.class_library.remove(idx);
                    let path = app.data_dir.join("classes.json");
                    let _ = json_store::save_json(&path, &app.class_library);
                    self.cursor = self.cursor.min(app.class_library.len().saturating_sub(1));
                }
                self.mode = Mode::Browse;
            }
            KeyCode::Char('n') | KeyCode::Esc => { self.mode = Mode::Browse; }
            _ => {}
        }
    }
}
