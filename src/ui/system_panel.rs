/// System Panel tab — displays and interacts with the active character.
///
/// Layout:
///   - Name
///   - Class: <name> (navigable, Enter to pick, d to remove)
///   - Race [Grade]
///   - Level [XP%]
///   - Attributes
///   - Skills grouped by category: Active, Passive, Innate
///   - Total effects (collapsible)
///
/// Interaction: j/k navigate, +/- adjust values, Enter expand/kill entry,
/// a to add skill, d to delete.

use crate::models::attribute::AttributeKind;
use crate::models::character::{Character, CharacterSkill};
use crate::models::class::CharacterClass;
use crate::models::skill::{MasteryRank, SkillCategory, SkillType};
use crate::formulas::xp;
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

/// Sections the cursor can be on.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Section {
    Class,
    Grade,
    Level,
    Attribute(usize),
    Skill(usize),
    TotalEffects,
}

/// Popup modes for data entry.
#[derive(Debug, Clone, PartialEq)]
enum PopupMode {
    None,
    KillEntry,
    AddSkill,
    AddClass,
}

/// All state for the System Panel tab.
pub struct SystemPanelState {
    cursor: usize,
    expanded_skills: Vec<bool>,
    expanded_effects: bool,
    popup: PopupMode,
    kill_enemy_level: String,
    kill_count: String,
    kill_field: usize,
    skill_picker_cursor: usize,
    skill_picker_list: Vec<String>,
    class_picker_cursor: usize,
    class_picker_list: Vec<String>,
}

impl SystemPanelState {
    pub fn new() -> Self {
        Self {
            cursor: 0,
            expanded_skills: Vec::new(),
            expanded_effects: false,
            popup: PopupMode::None,
            kill_enemy_level: String::new(),
            kill_count: String::new(),
            kill_field: 0,
            skill_picker_cursor: 0,
            skill_picker_list: Vec::new(),
            class_picker_cursor: 0,
            class_picker_list: Vec::new(),
        }
    }

    /// Build the ordered list of navigable sections from the current character.
    fn build_nav(&self, app: &App) -> Vec<Section> {
        let mut nav = Vec::new();
        let Some(character) = &app.current_character else { return nav; };

        nav.push(Section::Class);
        nav.push(Section::Grade);
        nav.push(Section::Level);
        for i in 0..6 { nav.push(Section::Attribute(i)); }
        for i in 0..character.skills.len() { nav.push(Section::Skill(i)); }
        nav.push(Section::TotalEffects);
        nav
    }

    /// Look up the SkillType for a character skill from the library.
    fn skill_type_for<'a>(&self, skill: &CharacterSkill, app: &'a App) -> Option<SkillType> {
        app.skill_library.iter()
            .find(|s| s.name == skill.definition_name)
            .map(|s| s.skill_type)
    }

    /// Look up the SkillCategory for a character skill from the library.
    fn skill_category_for<'a>(&self, skill: &CharacterSkill, app: &'a App) -> Option<SkillCategory> {
        app.skill_library.iter()
            .find(|s| s.name == skill.definition_name)
            .map(|s| s.category)
    }

    /// Render the system panel into the given area.
    pub fn render(&mut self, f: &mut Frame, area: Rect, app: &App, theme: &Theme) {
        let Some(character) = &app.current_character else {
            let msg = Paragraph::new("  No character loaded. Load one from the Character tab.")
                .style(Style::default().fg(theme.dimmed));
            f.render_widget(msg, area);
            return;
        };

        // Sync expanded_skills length.
        if self.expanded_skills.len() != character.skills.len() {
            self.expanded_skills.resize(character.skills.len(), false);
        }

        let nav = self.build_nav(app);
        let mut lines: Vec<Line> = Vec::new();

        // Character name.
        lines.push(Line::styled(
            format!("  {}", character.name),
            Style::default().fg(theme.fg).add_modifier(Modifier::BOLD),
        ));

        // Class row — navigable, Enter to pick, d to remove.
        let prof_prefix = self.cursor_prefix(&nav, Section::Class);
        let prof_style = self.cursor_style(theme, &nav, Section::Class);
        let prof_name = character.classes.first()
            .map(|p| p.definition_name.as_str())
            .unwrap_or("None");
        lines.push(Line::styled(
            format!("{}Class: {}", prof_prefix, prof_name),
            prof_style,
        ));

        // Grade — navigable, upgradeable with '+'.
        let grade_prefix = self.cursor_prefix(&nav, Section::Grade);
        let grade_style = self.cursor_style(theme, &nav, Section::Grade);
        let grade_hint = if character.grade.next().is_some() { "" } else { " [MAX]" };
        lines.push(Line::styled(
            format!("{}Race: {} [Grade {}]{}", grade_prefix, character.race, character.grade.name(), grade_hint),
            grade_style,
        ));

        // Level on its own navigable row.
        let xp_pct = xp::xp_percentage(character.xp, character.level, character.grade.numeric());
        let level_style = self.cursor_style(theme, &nav, Section::Level);
        let level_prefix = self.cursor_prefix(&nav, Section::Level);
        lines.push(Line::styled(
            format!("{}Level {} [{:.2}%]", level_prefix, character.level, xp_pct),
            level_style,
        ));
        lines.push(Line::styled(divider(area.width), Style::default().fg(theme.border)));

        // Attributes section.
        lines.push(Line::styled(
            format!("  Attributes (Unspent: {})", character.unspent_attribute_points),
            Style::default().fg(theme.secondary).add_modifier(Modifier::BOLD),
        ));
        for (i, kind) in AttributeKind::ALL.iter().enumerate() {
            let prefix = self.cursor_prefix(&nav, Section::Attribute(i));
            let style = self.cursor_style(theme, &nav, Section::Attribute(i));
            let val = character.attributes.get(*kind);
            lines.push(Line::styled(
                format!("{}{:<14}: {}", prefix, kind.name(), val),
                style,
            ));
        }
        lines.push(Line::styled(divider(area.width), Style::default().fg(theme.border)));

        // Skills grouped by category: Active, Passive, Innate.
        // Build index lists for each category.
        let mut active_indices: Vec<usize> = Vec::new();
        let mut passive_indices: Vec<usize> = Vec::new();
        let mut innate_indices: Vec<usize> = Vec::new();
        let mut class_indices: Vec<usize> = Vec::new();

        for (i, skill) in character.skills.iter().enumerate() {
            let cat = self.skill_category_for(skill, app);
            let stype = self.skill_type_for(skill, app);
            match cat {
                Some(SkillCategory::Innate) => innate_indices.push(i),
                Some(SkillCategory::Class) => class_indices.push(i),
                _ => match stype {
                    Some(SkillType::Active) => active_indices.push(i),
                    Some(SkillType::Passive) => passive_indices.push(i),
                    None => active_indices.push(i), // fallback
                },
            }
        }

        // Render each skill category.
        self.render_skill_group(f, "Active Skills", &active_indices, character, app, &nav, &mut lines, theme);
        self.render_skill_group(f, "Passive Skills", &passive_indices, character, app, &nav, &mut lines, theme);
        self.render_skill_group(f, "Innate Skills", &innate_indices, character, app, &nav, &mut lines, theme);
        self.render_skill_group(f, "Class Skills", &class_indices, character, app, &nav, &mut lines, theme);

        lines.push(Line::styled(divider(area.width), Style::default().fg(theme.border)));

        // Total effects section.
        let effects_prefix = self.cursor_prefix(&nav, Section::TotalEffects);
        let effects_style = self.cursor_style(theme, &nav, Section::TotalEffects);
        let arrow = if self.expanded_effects { "\u{25be}" } else { "\u{25b8}" };
        lines.push(Line::styled(
            format!("{}{} Total Effects [Enter to expand]", effects_prefix, arrow),
            effects_style,
        ));

        if self.expanded_effects {
            self.render_total_effects(character, app, &mut lines, theme);
        }

        let content = Paragraph::new(lines).block(Block::default().borders(Borders::NONE));
        f.render_widget(content, area);

        // Render popups.
        if self.popup == PopupMode::AddSkill {
            let body: Vec<Line> = self.skill_picker_list.iter().enumerate().map(|(i, name)| {
                let prefix = if i == self.skill_picker_cursor { ">> " } else { "   " };
                Line::from(format!("{}{}", prefix, name))
            }).collect();
            popup::render_modal(f, area, theme, "Add Skill", &body, "j/k: Navigate  Enter: Add  Esc: Cancel");
        }
        if self.popup == PopupMode::AddClass {
            let body: Vec<Line> = self.class_picker_list.iter().enumerate().map(|(i, name)| {
                let prefix = if i == self.class_picker_cursor { ">> " } else { "   " };
                Line::from(format!("{}{}", prefix, name))
            }).collect();
            popup::render_modal(f, area, theme, "Add Class", &body, "j/k: Navigate  Enter: Add  Esc: Cancel");
        }
        if self.popup == PopupMode::KillEntry {
            let field_marker = |f_idx: usize| if self.kill_field == f_idx { "> " } else { "  " };
            popup::render_popup(
                f, area, theme, "Kill Entry",
                &[
                    Line::from(format!("{}Enemy Level : {}_ ", field_marker(0), self.kill_enemy_level)),
                    Line::from(format!("{}Kill Count  : {}_ ", field_marker(1), self.kill_count)),
                ],
                "Tab: Switch  Enter: Calculate  Esc: Back",
            );
        }
    }

    /// Render a group of skills under a category heading.
    fn render_skill_group(
        &self, _f: &mut Frame, label: &str,
        indices: &[usize], character: &Character, app: &App,
        nav: &[Section], lines: &mut Vec<Line>, theme: &Theme,
    ) {
        lines.push(Line::styled(
            format!("  {}", label),
            Style::default().fg(theme.secondary).add_modifier(Modifier::BOLD),
        ));
        if indices.is_empty() {
            lines.push(Line::styled("     (none)", Style::default().fg(theme.dimmed)));
            return;
        }
        for &i in indices {
            let skill = &character.skills[i];
            let prefix = self.cursor_prefix(nav, Section::Skill(i));
            let style = self.cursor_style(theme, nav, Section::Skill(i));
            let max_tag = if skill.level >= skill.rank.max_level() { " [MAX]" } else { "" };
            lines.push(Line::styled(
                format!("{}{:<20}: {} {}{}", prefix, skill.definition_name, skill.rank.name(), skill.level, max_tag),
                style,
            ));

            // Expanded skill details.
            if i < self.expanded_skills.len() && self.expanded_skills[i] {
                if let Some(def) = app.skill_library.iter().find(|s| s.name == skill.definition_name) {
                    lines.push(Line::styled(
                        format!("     \u{251c}\u{2500} Type: {:?} | {:?}", def.category, def.skill_type),
                        Style::default().fg(theme.dimmed),
                    ));
                    if let Some(rank_def) = def.ranks.iter().find(|r| r.rank == skill.rank) {
                        for (ei, effect) in rank_def.effects.iter().enumerate() {
                            if effect.is_unlocked(skill.level) {
                                let val = effect.value_at_level(skill.level, skill.rank, def.skill_type);
                                let connector = if ei == rank_def.effects.len() - 1 { "\u{2514}\u{2500}" } else { "\u{251c}\u{2500}" };
                                let effect_label = effect.name.as_deref().unwrap_or("Effect");
                                lines.push(Line::styled(
                                    format!("     {} {:<14}: base {} \u{2192} current {:.1}", connector, effect_label, effect.base_value, val),
                                    Style::default().fg(theme.dimmed),
                                ));
                                if !effect.description.is_empty() {
                                    let indent = if ei == rank_def.effects.len() - 1 { "      " } else { "     \u{2502}" };
                                    lines.push(Line::styled(
                                        format!("{}  {}", indent, effect.description),
                                        Style::default().fg(theme.dimmed),
                                    ));
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    /// Aggregate and render total effects from all skills.
    fn render_total_effects<'a>(
        &self, character: &Character, app: &App, lines: &mut Vec<Line<'a>>, theme: &Theme,
    ) {
        for skill in &character.skills {
            if let Some(def) = app.skill_library.iter().find(|s| s.name == skill.definition_name) {
                if let Some(rank_def) = def.ranks.iter().find(|r| r.rank == skill.rank) {
                    for effect in &rank_def.effects {
                        if effect.is_unlocked(skill.level) {
                            let val = effect.value_at_level(skill.level, skill.rank, def.skill_type);
                            let effect_label = effect.name.as_deref().unwrap_or("Effect");
                            lines.push(Line::styled(
                                format!("     {:<20}: +{:.1}%", effect_label, val),
                                Style::default().fg(theme.fg),
                            ));
                        }
                    }
                }
            }
        }
    }

    /// Get cursor prefix ("  >> " or "     ") for a given section.
    fn cursor_prefix(&self, nav: &[Section], section: Section) -> &'static str {
        if nav.get(self.cursor) == Some(&section) { "  >> " } else { "     " }
    }

    /// Get style for a section (highlighted if cursor is there).
    fn cursor_style(&self, theme: &Theme, nav: &[Section], section: Section) -> Style {
        if nav.get(self.cursor) == Some(&section) {
            Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.fg)
        }
    }

    /// Handle key input.
    pub fn handle_input(&mut self, key: KeyEvent, app: &mut App) -> bool {
        if self.popup == PopupMode::KillEntry {
            return self.handle_kill_entry(key, app);
        }
        if self.popup == PopupMode::AddSkill {
            return self.handle_add_skill(key, app);
        }
        if self.popup == PopupMode::AddClass {
            return self.handle_add_class(key, app);
        }

        let nav = self.build_nav(app);
        let max = nav.len().saturating_sub(1);

        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                if self.cursor > 0 { self.cursor -= 1; }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.cursor < max { self.cursor += 1; }
            }
            KeyCode::Enter => {
                match nav.get(self.cursor) {
                    Some(Section::Class) => {
                        // Open class picker.
                        self.open_add_class(app);
                    }
                    Some(Section::Level) => {
                        self.popup = PopupMode::KillEntry;
                        self.kill_enemy_level.clear();
                        self.kill_count.clear();
                        self.kill_field = 0;
                    }
                    Some(Section::Skill(i)) => {
                        let i = *i;
                        if i < self.expanded_skills.len() {
                            self.expanded_skills[i] = !self.expanded_skills[i];
                        }
                    }
                    Some(Section::TotalEffects) => {
                        self.expanded_effects = !self.expanded_effects;
                    }
                    _ => {}
                }
            }
            KeyCode::Char('+') | KeyCode::Char('=') => {
                self.handle_increment(app, &nav);
            }
            KeyCode::Char('-') => {
                self.handle_decrement(app, &nav);
            }
            KeyCode::Char('a') => {
                self.open_add_skill(app);
            }
            KeyCode::Char('d') => {
                self.handle_delete(app, &nav);
            }
            _ => {}
        }
        true
    }

    /// Increment: grade up, level up, add attr point, level/rank skill.
    fn handle_increment(&mut self, app: &mut App, nav: &[Section]) {
        let Some(character) = &mut app.current_character else { return; };
        match nav.get(self.cursor) {
            Some(Section::Grade) => {
                if let Some(next) = character.grade.next() {
                    character.grade = next;
                    character.level = 0;
                    character.xp = 0.0;
                }
            }
            Some(Section::Level) => {
                if character.level < 100 {
                    let required = xp::xp_required(character.level, character.grade.numeric());
                    character.xp = (character.xp - required).max(0.0);
                    character.level += 1;
                    character.unspent_attribute_points += character.attribute_points_per_level();
                    character.apply_class_level_bonuses(&app.class_library);
                }
            }
            Some(Section::Attribute(i)) => {
                if character.unspent_attribute_points > 0 {
                    let kind = AttributeKind::ALL[*i];
                    character.attributes.add(kind, 1);
                    character.unspent_attribute_points -= 1;
                }
            }
            Some(Section::Skill(i)) => {
                if let Some(skill) = character.skills.get_mut(*i) {
                    if skill.level < skill.rank.max_level() {
                        skill.level += 1;
                    } else if let Some(next_rank) = skill.rank.next() {
                        skill.rank = next_rank;
                        skill.level = 0;
                    }
                }
            }
            _ => {}
        }
    }

    /// Decrement: remove attr point.
    fn handle_decrement(&mut self, app: &mut App, nav: &[Section]) {
        let Some(character) = &mut app.current_character else { return; };
        match nav.get(self.cursor) {
            Some(Section::Attribute(i)) => {
                let kind = AttributeKind::ALL[*i];
                let current = character.attributes.get(kind);
                if current > 1 {
                    match kind {
                        AttributeKind::Strength => character.attributes.strength -= 1,
                        AttributeKind::Agility => character.attributes.agility -= 1,
                        AttributeKind::Endurance => character.attributes.endurance -= 1,
                        AttributeKind::Intelligence => character.attributes.intelligence -= 1,
                        AttributeKind::Wisdom => character.attributes.wisdom -= 1,
                        AttributeKind::Perception => character.attributes.perception -= 1,
                    }
                    character.unspent_attribute_points += 1;
                }
            }
            _ => {}
        }
    }

    /// Delete: remove skill at cursor, or remove class on Class row.
    fn handle_delete(&mut self, app: &mut App, nav: &[Section]) {
        let Some(character) = &mut app.current_character else { return; };
        match nav.get(self.cursor) {
            Some(Section::Class) => {
                if !character.classes.is_empty() {
                    character.classes.remove(0);
                }
            }
            Some(Section::Skill(i)) => {
                let i = *i;
                if i < character.skills.len() {
                    character.skills.remove(i);
                    if i < self.expanded_skills.len() {
                        self.expanded_skills.remove(i);
                    }
                }
            }
            _ => {}
        }
    }

    /// Build the list of skills from the library not already on the character.
    fn open_add_skill(&mut self, app: &App) {
        let Some(character) = &app.current_character else { return; };
        let owned: Vec<&str> = character.skills.iter().map(|s| s.definition_name.as_str()).collect();
        self.skill_picker_list = app.skill_library.iter()
            .filter(|s| !owned.contains(&s.name.as_str()))
            .map(|s| s.name.clone())
            .collect();
        if self.skill_picker_list.is_empty() { return; }
        self.skill_picker_cursor = 0;
        self.popup = PopupMode::AddSkill;
    }

    /// Handle input in the add-skill picker popup.
    fn handle_add_skill(&mut self, key: KeyEvent, app: &mut App) -> bool {
        let max = self.skill_picker_list.len().saturating_sub(1);
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                if self.skill_picker_cursor > 0 { self.skill_picker_cursor -= 1; }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.skill_picker_cursor < max { self.skill_picker_cursor += 1; }
            }
            KeyCode::Enter => {
                if let Some(name) = self.skill_picker_list.get(self.skill_picker_cursor) {
                    if let Some(character) = &mut app.current_character {
                        character.skills.push(CharacterSkill {
                            definition_name: name.clone(),
                            rank: MasteryRank::Novice,
                            level: 0,
                        });
                    }
                }
                self.popup = PopupMode::None;
            }
            KeyCode::Esc => { self.popup = PopupMode::None; }
            _ => {}
        }
        true
    }

    /// Build the list of classs not already on the character.
    fn open_add_class(&mut self, app: &App) {
        let Some(character) = &app.current_character else { return; };
        if character.classes.len() as u32 >= character.class_slots { return; }
        let owned: Vec<&str> = character.classes.iter().map(|p| p.definition_name.as_str()).collect();
        self.class_picker_list = app.class_library.iter()
            .filter(|p| !owned.contains(&p.name.as_str()))
            .map(|p| p.name.clone())
            .collect();
        if self.class_picker_list.is_empty() { return; }
        self.class_picker_cursor = 0;
        self.popup = PopupMode::AddClass;
    }

    /// Handle input in the add-class picker popup.
    fn handle_add_class(&mut self, key: KeyEvent, app: &mut App) -> bool {
        let max = self.class_picker_list.len().saturating_sub(1);
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                if self.class_picker_cursor > 0 { self.class_picker_cursor -= 1; }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.class_picker_cursor < max { self.class_picker_cursor += 1; }
            }
            KeyCode::Enter => {
                if let Some(name) = self.class_picker_list.get(self.class_picker_cursor) {
                    if let Some(character) = &mut app.current_character {
                        // Auto-add the class's granted skills.
                        if let Some(cls_def) = app.class_library.iter().find(|p| &p.name == name) {
                            for skill_name in &cls_def.skills {
                                if !character.skills.iter().any(|s| s.definition_name == *skill_name) {
                                    character.skills.push(CharacterSkill {
                                        definition_name: skill_name.clone(),
                                        rank: MasteryRank::Novice,
                                        level: 0,
                                    });
                                }
                            }
                        }
                        character.classes.push(CharacterClass {
                            definition_name: name.clone(),
                            level: 0,
                            passive_rank: 0,
                        });
                    }
                }
                self.popup = PopupMode::None;
            }
            KeyCode::Esc => { self.popup = PopupMode::None; }
            _ => {}
        }
        true
    }

    /// Handle input in the kill entry popup.
    fn handle_kill_entry(&mut self, key: KeyEvent, app: &mut App) -> bool {
        match key.code {
            KeyCode::Char(c) if c.is_ascii_digit() => {
                if self.kill_field == 0 {
                    self.kill_enemy_level.push(c);
                } else {
                    self.kill_count.push(c);
                }
            }
            KeyCode::Backspace => {
                if self.kill_field == 0 { self.kill_enemy_level.pop(); }
                else { self.kill_count.pop(); }
            }
            KeyCode::Tab => { self.kill_field = 1 - self.kill_field; }
            KeyCode::Enter => {
                let class_library = app.class_library.clone();
                if let Some(character) = &mut app.current_character {
                    let enemy_level = self.kill_enemy_level.parse::<u32>().unwrap_or(character.level);
                    let kills = self.kill_count.parse::<u32>().unwrap_or(0);
                    let xp_per = xp::kill_xp(character.level, enemy_level);
                    character.xp += xp_per * kills as f64;
                    // Auto-level.
                    while character.level < 100 {
                        let required = xp::xp_required(character.level, character.grade.numeric());
                        if character.xp >= required {
                            character.xp -= required;
                            character.level += 1;
                            character.unspent_attribute_points += character.attribute_points_per_level();
                            character.apply_class_level_bonuses(&class_library);
                        } else {
                            break;
                        }
                    }
                }
                self.popup = PopupMode::None;
            }
            KeyCode::Esc => { self.popup = PopupMode::None; }
            _ => {}
        }
        true
    }
}
