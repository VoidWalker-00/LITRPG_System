/// Tree Library tab — browse, create, edit, and delete tree chains.
///
/// Default view: filterable list of trees grouped by chain.
/// Press 'a' to create a new standalone tree, 'c' to add to an existing chain,
/// Enter to view/edit, 'd' to delete, '/' to search.

use crate::models::tree::{
    TreeChain, TreeDefinition, TreeMilestone, UnlockRequirement, Comparison,
};
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
use crossterm::event::{KeyCode, KeyEvent};

/// Current mode of the tree library.
#[derive(Debug, Clone, PartialEq)]
enum Mode {
    Browse,
    Search,
    ViewDetail,
    Wizard(WizardStep),
    ConfirmDelete,
}

/// Steps in the tree creation wizard.
#[derive(Debug, Clone, PartialEq)]
enum WizardStep {
    Name,
    BaseCost,
    Description,
    Milestones,
    Requirements,
}

/// All state for the Tree Library tab.
pub struct TreeLibraryState {
    mode: Mode,
    cursor: usize,
    search_query: String,
    wizard: TreeWizard,
    /// Which chain we're adding to (None = new chain).
    editing_chain_index: Option<usize>,
}

/// State for the tree creation wizard.
struct TreeWizard {
    chain_name: String,
    base_cost: String,
    tree_name: String,
    description: String,
    milestones: [String; 4],
    milestone_index: usize,
    requirements: Vec<UnlockRequirement>,
    req_input: String,
    req_type: usize,
}

impl TreeWizard {
    fn new() -> Self {
        Self {
            chain_name: String::new(),
            base_cost: String::new(),
            tree_name: String::new(),
            description: String::new(),
            milestones: [String::new(), String::new(), String::new(), String::new()],
            milestone_index: 0,
            requirements: Vec::new(),
            req_input: String::new(),
            req_type: 0,
        }
    }

    /// Build a TreeDefinition from wizard state.
    fn build_tree(&self) -> TreeDefinition {
        let cost = self.base_cost.parse::<u32>().unwrap_or(40);
        TreeDefinition {
            name: self.tree_name.clone(),
            description: self.description.clone(),
            max_points: cost,
            milestones: [
                TreeMilestone { description: self.milestones[0].clone() },
                TreeMilestone { description: self.milestones[1].clone() },
                TreeMilestone { description: self.milestones[2].clone() },
                TreeMilestone { description: self.milestones[3].clone() },
            ],
            requirements: self.requirements.clone(),
        }
    }
}

impl TreeLibraryState {
    pub fn new() -> Self {
        Self {
            mode: Mode::Browse,
            cursor: 0,
            search_query: String::new(),
            wizard: TreeWizard::new(),
            editing_chain_index: None,
        }
    }

    /// Get filtered chain indices.
    fn filtered_indices(&self, app: &App) -> Vec<usize> {
        app.tree_library.iter().enumerate()
            .filter(|(_, c)| {
                self.search_query.is_empty()
                    || c.name.to_lowercase().contains(&self.search_query.to_lowercase())
            })
            .map(|(i, _)| i)
            .collect()
    }

    /// Render the Tree Library tab.
    /// Wizard and delete confirmation render as modal popups over the list.
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
                    .and_then(|&i| Some(app.tree_library[i].name.clone()))
                    .unwrap_or_default();
                popup::render_popup(
                    f, area, theme,
                    "Delete Chain",
                    &[Line::from(format!("Delete chain \"{}\"?", name))],
                    "[y] Yes  [n] No",
                );
            }
            _ => {}
        }
    }

    /// Render the filter bar + tree list.
    fn render_list(&self, f: &mut Frame, area: Rect, app: &App, theme: &Theme) {
        let indices = self.filtered_indices(app);
        let mut lines: Vec<Line> = Vec::new();

        // Filter bar.
        let search_display = if self.mode == Mode::Search {
            format!("{}_", self.search_query)
        } else if self.search_query.is_empty() {
            "___________".to_string()
        } else {
            self.search_query.clone()
        };
        lines.push(Line::styled(
            format!("  Filter: {}  [Chains: All\u{25be}]", search_display),
            Style::default().fg(theme.fg),
        ));
        lines.push(Line::styled(
            divider(area.width),
            Style::default().fg(theme.border),
        ));

        // Build a flat list of all trees across chains.
        let mut flat_items: Vec<(usize, usize, String)> = Vec::new(); // (chain_idx, tree_idx, display)
        for &chain_idx in &indices {
            let chain = &app.tree_library[chain_idx];
            for (tree_idx, tree) in chain.trees.iter().enumerate() {
                let cost = chain.cost_for_step(tree_idx);
                let standalone = if chain.trees.len() == 1 { "  (standalone)" } else { "" };
                flat_items.push((
                    chain_idx,
                    tree_idx,
                    format!("{:<24}[{} pts]{}", tree.name, cost, standalone),
                ));
            }
        }

        if flat_items.is_empty() {
            lines.push(Line::styled("  No trees found.", Style::default().fg(theme.dimmed)));
        }
        for (i, (_, _, display)) in flat_items.iter().enumerate() {
            let prefix = if i == self.cursor { "  >> " } else { "     " };
            let style = if i == self.cursor {
                Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.fg)
            };
            lines.push(Line::styled(format!("{}{}", prefix, display), style));
        }
        lines.push(Line::from(""));
        lines.push(Line::styled(
            "  j/k: Navigate  Enter: View/Edit  a: Add  d: Delete",
            Style::default().fg(theme.dimmed),
        ));
        lines.push(Line::styled(
            "  c: New Chain",
            Style::default().fg(theme.dimmed),
        ));

        let content = Paragraph::new(lines).block(Block::default().borders(Borders::NONE));
        f.render_widget(content, area);
    }

    /// Render detail view for the selected chain.
    fn render_detail(&self, f: &mut Frame, area: Rect, app: &App, theme: &Theme) {
        let indices = self.filtered_indices(app);
        let Some(&chain_idx) = indices.get(self.cursor) else {
            let msg = Paragraph::new("  No chain selected.")
                .style(Style::default().fg(theme.dimmed));
            f.render_widget(msg, area);
            return;
        };
        let chain = &app.tree_library[chain_idx];
        let mut lines: Vec<Line> = Vec::new();

        lines.push(Line::styled(
            format!("  Chain: {}", chain.name),
            Style::default().fg(theme.fg).add_modifier(Modifier::BOLD),
        ));
        lines.push(Line::styled(
            format!("  Base Cost: {} pts", chain.base_cost),
            Style::default().fg(theme.fg),
        ));
        lines.push(Line::from(""));

        for (i, tree) in chain.trees.iter().enumerate() {
            let cost = chain.cost_for_step(i);
            lines.push(Line::styled(
                format!("  \u{2500}\u{2500}\u{2500} {} (Step {}, {} pts) \u{2500}\u{2500}\u{2500}", tree.name, i + 1, cost),
                Style::default().fg(theme.secondary),
            ));
            lines.push(Line::styled(
                format!("    {}", tree.description),
                Style::default().fg(theme.dimmed),
            ));
            for (mi, milestone) in tree.milestones.iter().enumerate() {
                lines.push(Line::styled(
                    format!("    {}% \u{2014} {}", (mi + 1) * 25, milestone.description),
                    Style::default().fg(theme.fg),
                ));
            }
            if !tree.requirements.is_empty() {
                lines.push(Line::styled("    Requirements:", Style::default().fg(theme.secondary)));
                for req in &tree.requirements {
                    let desc = match req {
                        UnlockRequirement::Level { value, .. } => format!("Level >= {}", value),
                        UnlockRequirement::Skill { skill_name, .. } => format!("Skill: {}", skill_name),
                        UnlockRequirement::Achievement { name } => format!("Achievement: {}", name),
                    };
                    lines.push(Line::styled(format!("      {}", desc), Style::default().fg(theme.fg)));
                }
            }
            lines.push(Line::from(""));
        }

        lines.push(Line::styled("  Esc: Back to list", Style::default().fg(theme.dimmed)));

        let content = Paragraph::new(lines).block(Block::default().borders(Borders::NONE));
        f.render_widget(content, area);
    }

    /// Render the wizard as a modal popup over the list.
    fn render_wizard_modal(&self, f: &mut Frame, area: Rect, theme: &Theme, step: &WizardStep) {
        let step_num = match step {
            WizardStep::Name => 1,
            WizardStep::BaseCost => 2,
            WizardStep::Description => 3,
            WizardStep::Milestones => 4,
            WizardStep::Requirements => 5,
        };

        let title = format!(" Tree Wizard \u{2014} Step {} of 5 ", step_num);
        let mut body: Vec<Line> = Vec::new();

        match step {
            WizardStep::Name => {
                body.push(Line::from(""));
                if !self.wizard.chain_name.is_empty() {
                    body.push(Line::styled(
                        format!(" Chain: {}", self.wizard.chain_name),
                        Style::default().fg(theme.dimmed),
                    ));
                }
                body.push(Line::styled(
                    format!(" Tree Name: {}_", self.wizard.tree_name),
                    Style::default().fg(theme.fg),
                ));
                body.push(Line::from(""));
                popup::render_modal(f, area, theme, &title, &body, " Enter: Next  Esc: Cancel");
            }
            WizardStep::BaseCost => {
                body.push(Line::from(""));
                body.push(Line::styled(
                    format!(" Base point cost: {}_", self.wizard.base_cost),
                    Style::default().fg(theme.fg),
                ));
                body.push(Line::from(""));
                popup::render_modal(f, area, theme, &title, &body, " Enter: Next  Esc: Back");
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
            WizardStep::Milestones => {
                body.push(Line::from(""));
                body.push(Line::styled(" Milestone descriptions:", Style::default().fg(theme.secondary)));
                for i in 0..4 {
                    let marker = if i == self.wizard.milestone_index { " > " } else { "   " };
                    let cursor = if i == self.wizard.milestone_index { "_" } else { "" };
                    body.push(Line::styled(
                        format!("{}{}% \u{2014} {}{}", marker, (i + 1) * 25, self.wizard.milestones[i], cursor),
                        Style::default().fg(theme.fg),
                    ));
                }
                body.push(Line::from(""));
                popup::render_modal(f, area, theme, &title, &body, " j/k: select  Enter: next step  Esc: Back");
            }
            WizardStep::Requirements => {
                body.push(Line::from(""));
                body.push(Line::styled(" Requirements:", Style::default().fg(theme.secondary)));
                for (i, req) in self.wizard.requirements.iter().enumerate() {
                    let desc = match req {
                        UnlockRequirement::Level { value, .. } => format!("Level >= {}", value),
                        UnlockRequirement::Skill { skill_name, .. } => format!("Skill: {}", skill_name),
                        UnlockRequirement::Achievement { name } => format!("Achievement: {}", name),
                    };
                    body.push(Line::styled(format!("   {}. {}", i + 1, desc), Style::default().fg(theme.fg)));
                }
                let types = ["Level", "Skill Name", "Achievement"];
                body.push(Line::from(""));
                body.push(Line::styled(
                    format!(" Add (h/l type): {}  Value: {}_", types[self.wizard.req_type], self.wizard.req_input),
                    Style::default().fg(theme.fg),
                ));
                body.push(Line::from(""));
                popup::render_modal(f, area, theme, &title, &body, " Enter: add | Ctrl+D: done  Esc: Back");
            }
        }
    }

    /// Handle key input. Returns true (always consumed).
    pub fn handle_input(&mut self, key: KeyEvent, app: &mut App) -> bool {
        match &self.mode.clone() {
            Mode::Browse => self.handle_browse(key, app),
            Mode::Search => self.handle_search(key),
            Mode::ViewDetail => self.handle_detail_input(key),
            Mode::Wizard(step) => {
                let step = step.clone();
                self.handle_wizard(key, app, &step);
            }
            Mode::ConfirmDelete => self.handle_delete(key, app),
        }
        true
    }

    /// Browse mode.
    fn handle_browse(&mut self, key: KeyEvent, app: &App) {
        let indices = self.filtered_indices(app);
        // Count total flat items.
        let flat_count: usize = indices.iter()
            .map(|&i| app.tree_library[i].trees.len())
            .sum();

        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                if self.cursor > 0 { self.cursor -= 1; }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.cursor < flat_count.saturating_sub(1) { self.cursor += 1; }
            }
            KeyCode::Enter => {
                if !indices.is_empty() { self.mode = Mode::ViewDetail; }
            }
            KeyCode::Char('a') => {
                self.wizard = TreeWizard::new();
                self.editing_chain_index = None;
                self.mode = Mode::Wizard(WizardStep::Name);
            }
            KeyCode::Char('c') => {
                // Add tree to selected chain.
                if let Some(&chain_idx) = indices.get(self.cursor) {
                    self.wizard = TreeWizard::new();
                    self.wizard.chain_name = app.tree_library[chain_idx].name.clone();
                    self.editing_chain_index = Some(chain_idx);
                    self.mode = Mode::Wizard(WizardStep::Name);
                }
            }
            KeyCode::Char('d') => {
                if !indices.is_empty() { self.mode = Mode::ConfirmDelete; }
            }
            KeyCode::Char('/') => {
                self.mode = Mode::Search;
            }
            KeyCode::Esc => {
                if !self.search_query.is_empty() {
                    self.search_query.clear();
                    self.cursor = 0;
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
            }
            _ => {}
        }
    }

    /// Detail view.
    fn handle_detail_input(&mut self, key: KeyEvent) {
        if key.code == KeyCode::Esc { self.mode = Mode::Browse; }
    }

    /// Wizard input.
    fn handle_wizard(&mut self, key: KeyEvent, app: &mut App, step: &WizardStep) {
        match step {
            WizardStep::Name => match key.code {
                KeyCode::Char(c) => { self.wizard.tree_name.push(c); }
                KeyCode::Backspace => { self.wizard.tree_name.pop(); }
                KeyCode::Enter if !self.wizard.tree_name.is_empty() => {
                    self.mode = Mode::Wizard(WizardStep::BaseCost);
                }
                KeyCode::Esc => { self.mode = Mode::Browse; }
                _ => {}
            },
            WizardStep::BaseCost => match key.code {
                KeyCode::Char(c) if c.is_ascii_digit() => { self.wizard.base_cost.push(c); }
                KeyCode::Backspace => { self.wizard.base_cost.pop(); }
                KeyCode::Enter if !self.wizard.base_cost.is_empty() => {
                    self.mode = Mode::Wizard(WizardStep::Description);
                }
                KeyCode::Esc => { self.mode = Mode::Wizard(WizardStep::Name); }
                _ => {}
            },
            WizardStep::Description => match key.code {
                KeyCode::Char(c) => { self.wizard.description.push(c); }
                KeyCode::Backspace => { self.wizard.description.pop(); }
                KeyCode::Enter if !self.wizard.description.is_empty() => {
                    self.mode = Mode::Wizard(WizardStep::Milestones);
                    self.wizard.milestone_index = 0;
                }
                KeyCode::Esc => { self.mode = Mode::Wizard(WizardStep::BaseCost); }
                _ => {}
            },
            WizardStep::Milestones => match key.code {
                KeyCode::Up | KeyCode::Char('k') => {
                    if self.wizard.milestone_index > 0 { self.wizard.milestone_index -= 1; }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if self.wizard.milestone_index < 3 { self.wizard.milestone_index += 1; }
                }
                KeyCode::Char(c) => {
                    self.wizard.milestones[self.wizard.milestone_index].push(c);
                }
                KeyCode::Backspace => {
                    self.wizard.milestones[self.wizard.milestone_index].pop();
                }
                KeyCode::Enter => {
                    self.mode = Mode::Wizard(WizardStep::Requirements);
                }
                KeyCode::Esc => { self.mode = Mode::Wizard(WizardStep::Description); }
                _ => {}
            },
            WizardStep::Requirements => {
                if key.code == KeyCode::Char('d') && key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) {
                    self.save_tree(app);
                    self.mode = Mode::Browse;
                    self.cursor = 0;
                } else {
                    match key.code {
                        KeyCode::Left | KeyCode::Char('h') => {
                            if self.wizard.req_type > 0 { self.wizard.req_type -= 1; }
                        }
                        KeyCode::Right | KeyCode::Char('l') => {
                            if self.wizard.req_type < 2 { self.wizard.req_type += 1; }
                        }
                        KeyCode::Char(c) => { self.wizard.req_input.push(c); }
                        KeyCode::Backspace => { self.wizard.req_input.pop(); }
                        KeyCode::Enter => {
                            if !self.wizard.req_input.is_empty() {
                                let req = match self.wizard.req_type {
                                    0 => {
                                        let val = self.wizard.req_input.parse::<u32>().unwrap_or(1);
                                        UnlockRequirement::Level {
                                            comparison: Comparison::GreaterOrEqual,
                                            value: val,
                                        }
                                    }
                                    1 => UnlockRequirement::Skill {
                                        skill_name: self.wizard.req_input.clone(),
                                        min_rank: None,
                                        min_level: None,
                                    },
                                    _ => UnlockRequirement::Achievement {
                                        name: self.wizard.req_input.clone(),
                                    },
                                };
                                self.wizard.requirements.push(req);
                                self.wizard.req_input.clear();
                            }
                        }
                        KeyCode::Esc => { self.mode = Mode::Wizard(WizardStep::Milestones); }
                        _ => {}
                    }
                }
            }
        }
    }

    /// Save the wizard's tree into the library.
    fn save_tree(&mut self, app: &mut App) {
        let tree = self.wizard.build_tree();
        let base_cost = self.wizard.base_cost.parse::<u32>().unwrap_or(40);

        if let Some(chain_idx) = self.editing_chain_index {
            app.tree_library[chain_idx].trees.push(tree);
        } else {
            let chain_name = if self.wizard.chain_name.is_empty() {
                self.wizard.tree_name.clone()
            } else {
                self.wizard.chain_name.clone()
            };
            app.tree_library.push(TreeChain {
                name: chain_name,
                base_cost,
                trees: vec![tree],
            });
        }
        let path = app.data_dir.join("trees.json");
        let _ = json_store::save_json(&path, &app.tree_library);
    }

    /// Delete confirmation.
    fn handle_delete(&mut self, key: KeyEvent, app: &mut App) {
        match key.code {
            KeyCode::Char('y') => {
                let indices = self.filtered_indices(app);
                if let Some(&idx) = indices.get(self.cursor) {
                    app.tree_library.remove(idx);
                    let path = app.data_dir.join("trees.json");
                    let _ = json_store::save_json(&path, &app.tree_library);
                    self.cursor = self.cursor.min(app.tree_library.len().saturating_sub(1));
                }
                self.mode = Mode::Browse;
            }
            KeyCode::Char('n') | KeyCode::Esc => { self.mode = Mode::Browse; }
            _ => {}
        }
    }
}
