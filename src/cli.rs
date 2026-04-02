/// CLI interface for the LITRPG System.
///
/// Provides JSON-based commands for managing characters, skills, and
/// classes without launching the TUI. All output is JSON to stdout;
/// errors go to stderr with exit code 1.

use std::path::PathBuf;
use std::process;

use clap::{Parser, Subcommand};
use serde_json::json;

use crate::formulas::xp;
use crate::models::attribute::{AttributeKind, Attributes};
use crate::models::character::{Character, CharacterInnateSkill, CharacterSkill};
use crate::models::class::{CharacterClass, ClassDefinition};
use crate::models::skill::{
    MasteryRank, RankDefinition, SkillCategory, SkillDefinition, SkillEffect, SkillType,
};
use crate::storage::json_store;

// ---------------------------------------------------------------------------
// Clap definitions
// ---------------------------------------------------------------------------

/// Top-level CLI parser. When no subcommand is given, the TUI launches.
#[derive(Parser)]
#[command(name = "litrpg", about = "LITRPG System — CLI and TUI")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,
}

/// All top-level subcommands.
#[derive(Subcommand)]
pub enum Command {
    /// List all saved characters.
    List,

    /// Show full character stats as JSON.
    Show {
        /// Character name to display.
        name: String,
    },

    /// Create a new character (Grade G, Level 0).
    Create {
        /// Character name.
        name: String,
        /// Strength (1-10, default 5).
        #[arg(long = "str", default_value_t = 5)]
        str_val: u32,
        /// Agility (1-10, default 5).
        #[arg(long, default_value_t = 5)]
        agi: u32,
        /// Endurance (1-10, default 5).
        #[arg(long, default_value_t = 5)]
        end: u32,
        /// Intelligence (1-10, default 5).
        #[arg(long, default_value_t = 5)]
        int: u32,
        /// Wisdom (1-10, default 5).
        #[arg(long, default_value_t = 5)]
        wis: u32,
        /// Perception (1-10, default 5).
        #[arg(long, default_value_t = 5)]
        per: u32,
        /// Optional innate skill name from the library.
        #[arg(long)]
        innate: Option<String>,
    },

    /// Delete a saved character.
    Delete {
        /// Character name to delete.
        name: String,
    },

    /// Mutate an existing character (level-up, grade-up, add/remove skills, etc.).
    Update {
        /// Character name to update.
        name: String,
        /// Number of level-ups to apply.
        #[arg(long)]
        level_up: Option<u32>,
        /// Advance to the next grade (resets level and XP to 0).
        #[arg(long)]
        grade_up: bool,
        /// Add a skill from the library (starts at Novice 0).
        #[arg(long)]
        add_skill: Option<String>,
        /// Remove a skill by name.
        #[arg(long)]
        remove_skill: Option<String>,
        /// Add a class from the library (respects slot limit).
        #[arg(long)]
        add_class: Option<String>,
        /// Remove a class by name.
        #[arg(long)]
        remove_class: Option<String>,
        /// Distribute unspent attribute points (format: kind:points, e.g. str:3).
        #[arg(long)]
        add_attr: Option<String>,
        /// Remove attribute points (format: kind:points, minimum value 1).
        #[arg(long)]
        remove_attr: Option<String>,
        /// Apply kill XP (format: enemy_level:count).
        #[arg(long)]
        kill: Option<String>,
        /// Output full character JSON after all mutations.
        #[arg(long)]
        show: bool,
    },

    /// Manage the skill library.
    Skill {
        #[command(subcommand)]
        action: SkillAction,
    },

    /// Manage the class library.
    Class {
        #[command(subcommand)]
        action: ClassAction,
    },

    /// Clear all data for a specific tab or everything.
    Clear {
        #[command(subcommand)]
        target: ClearTarget,
    },
}

/// What to clear.
#[derive(Subcommand)]
pub enum ClearTarget {
    /// Delete all saved characters.
    Characters,
    /// Empty the skill library.
    Skills,
    /// Empty the class library.
    Classes,
    /// Clear everything (characters + all libraries).
    All,
}

/// Skill library subcommands.
#[derive(Subcommand)]
pub enum SkillAction {
    /// List all skills in the library.
    List,

    /// Show full skill definition.
    Show {
        /// Skill name.
        name: String,
    },

    /// Create a new skill definition.
    Create {
        /// Skill name.
        name: String,
        /// Category: acquired, innate, or profession (default: acquired).
        #[arg(long, default_value = "acquired")]
        category: String,
        /// Type: active or passive (default: active).
        #[arg(long = "type", default_value = "active")]
        skill_type: String,
        /// Skill description.
        #[arg(long, default_value = "")]
        description: String,
        /// Effect names (repeatable).
        #[arg(long = "effect")]
        effects: Vec<String>,
        /// Base values for effects (parallel with --effect).
        #[arg(long = "base-value")]
        base_values: Vec<f64>,
        /// Unlock levels for effects (parallel with --effect).
        #[arg(long = "unlock-level")]
        unlock_levels: Vec<u32>,
        /// Effect descriptions (parallel with --effect).
        #[arg(long = "effect-desc")]
        effect_descs: Vec<String>,
    },

    /// Delete a skill from the library.
    Delete {
        /// Skill name.
        name: String,
    },

    /// Update an existing skill definition.
    Update {
        /// Skill name.
        name: String,
        /// New description.
        #[arg(long)]
        description: Option<String>,
        /// Change category.
        #[arg(long)]
        category: Option<String>,
        /// Change type.
        #[arg(long = "type")]
        skill_type: Option<String>,
        /// Add an effect to the Novice rank.
        #[arg(long)]
        add_effect: Option<String>,
        /// Base value for the new effect.
        #[arg(long)]
        base_value: Option<f64>,
        /// Unlock level for the new effect.
        #[arg(long)]
        unlock_level: Option<u32>,
        /// Description for the new effect.
        #[arg(long)]
        effect_desc: Option<String>,
        /// Remove an effect by name from the Novice rank.
        #[arg(long)]
        remove_effect: Option<String>,
    },
}

/// Class library subcommands.
#[derive(Subcommand)]
pub enum ClassAction {
    /// List all classes.
    List,

    /// Show full class definition.
    Show {
        /// Class name.
        name: String,
    },

    /// Create a new class definition.
    Create {
        /// Class name.
        name: String,
        /// Class description.
        #[arg(long, default_value = "")]
        description: String,
        /// Passive ability name.
        #[arg(long, default_value = "")]
        passive_name: String,
        /// Passive ability description.
        #[arg(long, default_value = "")]
        passive_desc: String,
        /// Comma-separated list of granted skill names.
        #[arg(long, default_value = "")]
        skills: String,
        /// Bonus STR per level-up.
        #[arg(long, default_value_t = 0)] bonus_str: u32,
        /// Bonus AGI per level-up.
        #[arg(long, default_value_t = 0)] bonus_agi: u32,
        /// Bonus END per level-up.
        #[arg(long, default_value_t = 0)] bonus_end: u32,
        /// Bonus INT per level-up.
        #[arg(long, default_value_t = 0)] bonus_int: u32,
        /// Bonus WIS per level-up.
        #[arg(long, default_value_t = 0)] bonus_wis: u32,
        /// Bonus PER per level-up.
        #[arg(long, default_value_t = 0)] bonus_per: u32,
        /// Free unspent attribute points added per level-up.
        #[arg(long, default_value_t = 0)] bonus_free: u32,
    },

    /// Delete a class from the library.
    Delete {
        /// Class name.
        name: String,
    },

    /// Update an existing class definition.
    Update {
        /// Class name.
        name: String,
        /// New description.
        #[arg(long)]
        description: Option<String>,
        /// New passive name.
        #[arg(long)]
        passive_name: Option<String>,
        /// New passive description.
        #[arg(long)]
        passive_desc: Option<String>,
        /// Add a skill to the class's granted skills.
        #[arg(long)]
        add_skill: Option<String>,
        /// Remove a skill from the class's granted skills.
        #[arg(long)]
        remove_skill: Option<String>,
        /// Set bonus STR per level-up.
        #[arg(long)] bonus_str: Option<u32>,
        /// Set bonus AGI per level-up.
        #[arg(long)] bonus_agi: Option<u32>,
        /// Set bonus END per level-up.
        #[arg(long)] bonus_end: Option<u32>,
        /// Set bonus INT per level-up.
        #[arg(long)] bonus_int: Option<u32>,
        /// Set bonus WIS per level-up.
        #[arg(long)] bonus_wis: Option<u32>,
        /// Set bonus PER per level-up.
        #[arg(long)] bonus_per: Option<u32>,
        /// Set free unspent attribute points per level-up.
        #[arg(long)] bonus_free: Option<u32>,
    },
}

// ---------------------------------------------------------------------------
// JSON helpers
// ---------------------------------------------------------------------------

/// Build a JSON success response.
fn json_ok(msg: &str) -> String {
    serde_json::to_string_pretty(&json!({
        "status": "ok",
        "message": msg
    }))
    .expect("json_ok serialization failed")
}

/// Build a JSON error response.
fn json_err(msg: &str) -> String {
    serde_json::to_string_pretty(&json!({
        "status": "error",
        "message": msg
    }))
    .expect("json_err serialization failed")
}

// ---------------------------------------------------------------------------
// Entry point — dispatches subcommands to handlers
// ---------------------------------------------------------------------------

/// Run a CLI command. Prints JSON to stdout on success, or to stderr on
/// failure (with exit code 1).
pub fn run(cmd: Command) {
    let data_dir = crate::data_dir();
    let char_dir = data_dir.join("characters");

    let result = match cmd {
        Command::List => cmd_list(&char_dir),
        Command::Show { name } => cmd_show(&data_dir, &char_dir, &name),
        Command::Create {
            name,
            str_val,
            agi,
            end,
            int,
            wis,
            per,
            innate,
        } => cmd_create(&char_dir, &name, str_val, agi, end, int, wis, per, innate),
        Command::Delete { name } => cmd_delete(&char_dir, &name),
        Command::Update {
            name,
            level_up,
            grade_up,
            add_skill,
            remove_skill,
            add_class,
            remove_class,
            add_attr,
            remove_attr,
            kill,
            show,
        } => {
            let opts = UpdateOpts {
                level_up,
                grade_up,
                add_skill: add_skill.into_iter().collect(),
                remove_skill: remove_skill.into_iter().collect(),
                add_class: add_class.into_iter().collect(),
                remove_class: remove_class.into_iter().collect(),
                add_attr: add_attr.into_iter().collect(),
                remove_attr: remove_attr.into_iter().collect(),
                kill: kill.into_iter().collect(),
                show,
            };
            cmd_update(&data_dir, &name, &opts)
        }
        Command::Skill { action } => match action {
            SkillAction::List => cmd_skill_list(&data_dir),
            SkillAction::Show { name } => cmd_skill_show(&data_dir, &name),
            SkillAction::Create {
                name,
                category,
                skill_type,
                description,
                effects,
                base_values,
                unlock_levels,
                effect_descs,
            } => cmd_skill_create(
                &data_dir, &name, &category, &skill_type, &description,
                &effects, &base_values, &unlock_levels, &effect_descs,
            ),
            SkillAction::Delete { name } => cmd_skill_delete(&data_dir, &name),
            SkillAction::Update {
                name,
                description,
                category,
                skill_type,
                add_effect,
                base_value,
                unlock_level,
                effect_desc,
                remove_effect,
            } => cmd_skill_update(
                &data_dir, &name,
                description.as_deref(), category.as_deref(), skill_type.as_deref(),
                &add_effect.into_iter().collect::<Vec<_>>(),
                &base_value.into_iter().collect::<Vec<_>>(),
                &unlock_level.into_iter().collect::<Vec<_>>(),
                &effect_desc.into_iter().collect::<Vec<_>>(),
                &remove_effect.into_iter().collect::<Vec<_>>(),
            ),
        },
        Command::Clear { target } => cmd_clear(&data_dir, &char_dir, target),
        Command::Class { action } => match action {
            ClassAction::List => cmd_class_list(&data_dir),
            ClassAction::Show { name } => cmd_class_show(&data_dir, &name),
            ClassAction::Create {
                name, description, passive_name, passive_desc, skills,
                bonus_str, bonus_agi, bonus_end, bonus_int, bonus_wis, bonus_per, bonus_free,
            } => {
                let skill_list: Vec<String> = skills
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                cmd_class_create(
                    &data_dir, &name, &description, &passive_name, &passive_desc, &skill_list,
                    bonus_str, bonus_agi, bonus_end, bonus_int, bonus_wis, bonus_per, bonus_free,
                )
            }
            ClassAction::Delete { name } => cmd_class_delete(&data_dir, &name),

            ClassAction::Update {
                name, description, passive_name, passive_desc, add_skill, remove_skill,
                bonus_str, bonus_agi, bonus_end, bonus_int, bonus_wis, bonus_per, bonus_free,
            } => cmd_class_update(
                &data_dir, &name,
                description.as_deref(), passive_name.as_deref(), passive_desc.as_deref(),
                &add_skill.into_iter().collect::<Vec<_>>(),
                &remove_skill.into_iter().collect::<Vec<_>>(),
                bonus_str, bonus_agi, bonus_end, bonus_int, bonus_wis, bonus_per, bonus_free,
            ),
        },
    };

    match result {
        Ok(output) => println!("{}", output),
        Err(msg) => {
            eprintln!("{}", json_err(&msg));
            process::exit(1);
        }
    }
}

// ---------------------------------------------------------------------------
// Command handlers
// ---------------------------------------------------------------------------

/// List all saved character names as a JSON array.
fn cmd_list(char_dir: &PathBuf) -> Result<String, String> {
    let names = json_store::list_characters(char_dir).unwrap_or_default();
    serde_json::to_string_pretty(&names).map_err(|e| e.to_string())
}

/// Show full character state as JSON, including computed xp_percentage and
/// enriched skill data from the library.
fn cmd_show(data_dir: &PathBuf, char_dir: &PathBuf, name: &str) -> Result<String, String> {
    let character = json_store::load_character(char_dir, name)
        .map_err(|_| format!("Character '{}' not found", name))?;

    // Load skill library for enriching skill category/type.
    let skills_path = data_dir.join("skills.json");
    let skill_library: Vec<SkillDefinition> = if skills_path.exists() {
        json_store::load_json(&skills_path).unwrap_or_default()
    } else {
        Vec::new()
    };

    let json = character_to_json(&character, &skill_library);
    serde_json::to_string_pretty(&json).map_err(|e| e.to_string())
}

/// Create a new character with the given attributes and optional innate skill.
/// Checks for duplicates before saving.
fn cmd_create(
    char_dir: &PathBuf,
    name: &str,
    str_val: u32,
    agi: u32,
    end: u32,
    int: u32,
    wis: u32,
    per: u32,
    innate: Option<String>,
) -> Result<String, String> {
    // Check for duplicate.
    if json_store::load_character(char_dir, name).is_ok() {
        return Err(format!("Character '{}' already exists", name));
    }

    // Validate attribute ranges (1-10).
    for (label, val) in [("str", str_val), ("agi", agi), ("end", end), ("int", int), ("wis", wis), ("per", per)] {
        if val < 1 || val > 10 {
            return Err(format!("Invalid value for --{}: {} (must be 1-10)", label, val));
        }
    }

    let attrs = Attributes::new_clamped(str_val, agi, end, int, wis, per);
    let innate_skill = innate.map(|skill_name| CharacterInnateSkill {
        definition_name: skill_name,
        level: 0,
    });

    let character = Character::new(name.to_string(), attrs, innate_skill);
    json_store::save_character(char_dir, &character)?;

    Ok(json_ok(&format!("Created character '{}'", name)))
}

/// Delete a character's save file.
fn cmd_delete(char_dir: &PathBuf, name: &str) -> Result<String, String> {
    json_store::delete_character(char_dir, name)
        .map_err(|_| format!("Character '{}' not found", name))?;
    Ok(json_ok(&format!("Deleted character '{}'", name)))
}

/// Clear all data for the specified target.
fn cmd_clear(data_dir: &PathBuf, char_dir: &PathBuf, target: ClearTarget) -> Result<String, String> {
    let clear_characters = matches!(target, ClearTarget::Characters | ClearTarget::All);
    let clear_skills     = matches!(target, ClearTarget::Skills     | ClearTarget::All);
    let clear_classes    = matches!(target, ClearTarget::Classes    | ClearTarget::All);

    if clear_characters {
        let entries = std::fs::read_dir(char_dir).map_err(|e| e.to_string())?;
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("json") {
                std::fs::remove_file(&path).map_err(|e| e.to_string())?;
            }
        }
    }
    if clear_skills {
        json_store::save_json(&data_dir.join("skills.json"), &Vec::<SkillDefinition>::new())
            .map_err(|e| e.to_string())?;
    }
    if clear_classes {
        json_store::save_json(&data_dir.join("classes.json"), &Vec::<ClassDefinition>::new())
            .map_err(|e| e.to_string())?;
    }

    let label = match target {
        ClearTarget::Characters => "characters",
        ClearTarget::Skills     => "skill library",
        ClearTarget::Classes    => "class library",
        ClearTarget::All        => "all data",
    };
    Ok(json_ok(&format!("Cleared {}", label)))
}

// ---------------------------------------------------------------------------
// Update command
// ---------------------------------------------------------------------------

/// Options for character mutation — separated from clap for testability.
#[derive(Default)]
struct UpdateOpts {
    level_up: Option<u32>,
    grade_up: bool,
    add_skill: Vec<String>,
    remove_skill: Vec<String>,
    add_class: Vec<String>,
    remove_class: Vec<String>,
    add_attr: Vec<String>,    // "str:5" format
    remove_attr: Vec<String>, // "str:5" format
    kill: Vec<String>,        // "enemy_level:count" format
    show: bool,
}

/// Apply mutations to a character: grade-up, level-up, kill XP, skills,
/// classes, and attribute distribution.
fn cmd_update(data_dir: &std::path::Path, name: &str, opts: &UpdateOpts) -> Result<String, String> {
    let char_dir = data_dir.join("characters");
    let mut character = json_store::load_character(&char_dir, name)
        .map_err(|_| format!("Character '{}' not found", name))?;

    // Load class library up front — needed for attribute point calculations.
    let classes_path = data_dir.join("classes.json");
    let class_lib: Vec<ClassDefinition> = if classes_path.exists() {
        json_store::load_json(&classes_path).unwrap_or_default()
    } else {
        Vec::new()
    };

    // Grade up (resets level and XP).
    if opts.grade_up {
        character.grade = character.grade.next()
            .ok_or_else(|| "Already at max grade (SSS)".to_string())?;
        character.level = 0;
        character.xp = 0.0;
    }

    // Level up N times.
    if let Some(n) = opts.level_up {
        for _ in 0..n {
            if character.level >= 100 { break; }
            character.level += 1;
            character.unspent_attribute_points += character.attribute_points_per_level();
            character.apply_class_level_bonuses(&class_lib);
        }
    }

    // Kill XP — format "enemy_level:count".
    for kill_str in &opts.kill {
        let parts: Vec<&str> = kill_str.split(':').collect();
        if parts.len() != 2 {
            return Err(format!("Invalid kill format '{}', expected enemy_level:count", kill_str));
        }
        let enemy_level: u32 = parts[0].parse().map_err(|_| "Invalid enemy level".to_string())?;
        let count: u32 = parts[1].parse().map_err(|_| "Invalid kill count".to_string())?;
        for _ in 0..count {
            let xp_gain = xp::kill_xp(character.level, enemy_level);
            character.xp += xp_gain;
            // Auto-level while enough XP.
            while character.level < 100 {
                let required = xp::xp_required(character.level, character.grade.numeric());
                if character.xp >= required {
                    character.xp -= required;
                    character.level += 1;
                    character.unspent_attribute_points += character.attribute_points_per_level();
                    character.apply_class_level_bonuses(&class_lib);
                } else {
                    break;
                }
            }
        }
    }

    // Add skills from library.
    let skills_path = data_dir.join("skills.json");
    let skill_lib: Vec<SkillDefinition> = if skills_path.exists() {
        json_store::load_json(&skills_path).unwrap_or_default()
    } else {
        Vec::new()
    };
    for skill_name in &opts.add_skill {
        if character.skills.iter().any(|s| s.definition_name == *skill_name) {
            return Err(format!("Character already has skill '{}'", skill_name));
        }
        if !skill_lib.iter().any(|s| s.name == *skill_name) {
            return Err(format!("Skill '{}' not found in library", skill_name));
        }
        character.skills.push(CharacterSkill {
            definition_name: skill_name.clone(),
            rank: MasteryRank::Novice,
            level: 0,
        });
    }

    // Remove skills.
    for skill_name in &opts.remove_skill {
        let before = character.skills.len();
        character.skills.retain(|s| s.definition_name != *skill_name);
        if character.skills.len() == before {
            return Err(format!("Skill '{}' not found on character", skill_name));
        }
    }

    // Add classes from library (respects slot limit).
    for class_name in &opts.add_class {
        if character.classes.len() as u32 >= character.class_slots {
            return Err("Class slot limit reached".to_string());
        }
        if character.classes.iter().any(|p| p.definition_name == *class_name) {
            return Err(format!("Character already has class '{}'", class_name));
        }
        let class_def = class_lib.iter().find(|p| p.name == *class_name)
            .ok_or_else(|| format!("Class '{}' not found in library", class_name))?;
        // Auto-add the class's granted skills (skip duplicates).
        for skill_name in &class_def.skills {
            if !character.skills.iter().any(|s| s.definition_name == *skill_name) {
                character.skills.push(CharacterSkill {
                    definition_name: skill_name.clone(),
                    rank: MasteryRank::Novice,
                    level: 0,
                });
            }
        }
        character.classes.push(CharacterClass {
            definition_name: class_name.clone(),
            level: 0,
            passive_rank: 0,
        });
    }

    // Remove classes.
    for class_name in &opts.remove_class {
        let before = character.classes.len();
        character.classes.retain(|p| p.definition_name != *class_name);
        if character.classes.len() == before {
            return Err(format!("Class '{}' not found on character", class_name));
        }
    }

    // Add attribute points — format "kind:points".
    for attr_str in &opts.add_attr {
        let (kind, points) = parse_attr_arg(attr_str)?;
        if character.unspent_attribute_points < points {
            return Err(format!(
                "Not enough unspent points (have {}, need {})",
                character.unspent_attribute_points, points
            ));
        }
        for _ in 0..points {
            character.attributes.add(kind, 1);
        }
        character.unspent_attribute_points -= points;
    }

    // Remove attribute points — format "kind:points".
    for attr_str in &opts.remove_attr {
        let (kind, points) = parse_attr_arg(attr_str)?;
        let current = character.attributes.get(kind);
        if current < points + 1 {
            return Err(format!("Cannot reduce {} below 1", kind.name()));
        }
        subtract_attribute(&mut character.attributes, kind, points);
        character.unspent_attribute_points += points;
    }

    json_store::save_character(&char_dir, &character)?;

    if opts.show {
        let json = character_to_json(&character, &skill_lib);
        serde_json::to_string_pretty(&json).map_err(|e| e.to_string())
    } else {
        Ok(json_ok(&format!("Updated character '{}'", name)))
    }
}

/// Parse "kind:points" attribute argument (e.g. "str:3").
fn parse_attr_arg(s: &str) -> Result<(AttributeKind, u32), String> {
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() != 2 {
        return Err(format!("Invalid attr format '{}', expected kind:points", s));
    }
    let kind = match parts[0] {
        "str" => AttributeKind::Strength,
        "agi" => AttributeKind::Agility,
        "end" => AttributeKind::Endurance,
        "int" => AttributeKind::Intelligence,
        "wis" => AttributeKind::Wisdom,
        "per" => AttributeKind::Perception,
        _ => return Err(format!("Unknown attribute '{}' (use: str, agi, end, int, wis, per)", parts[0])),
    };
    let points: u32 = parts[1].parse().map_err(|_| "Invalid point value".to_string())?;
    Ok((kind, points))
}

/// Subtract attribute points by kind.
fn subtract_attribute(attrs: &mut Attributes, kind: AttributeKind, points: u32) {
    match kind {
        AttributeKind::Strength => attrs.strength -= points,
        AttributeKind::Agility => attrs.agility -= points,
        AttributeKind::Endurance => attrs.endurance -= points,
        AttributeKind::Intelligence => attrs.intelligence -= points,
        AttributeKind::Wisdom => attrs.wisdom -= points,
        AttributeKind::Perception => attrs.perception -= points,
    }
}

// ---------------------------------------------------------------------------
// Skill library commands
// ---------------------------------------------------------------------------

/// Parse skill category from string.
fn parse_category(s: &str) -> Result<SkillCategory, String> {
    match s.to_lowercase().as_str() {
        "acquired" => Ok(SkillCategory::Acquired),
        "innate" => Ok(SkillCategory::Innate),
        "class" => Ok(SkillCategory::Class),
        _ => Err(format!("Unknown category '{}' (use: acquired, innate, class)", s)),
    }
}

/// Parse skill type from string.
fn parse_skill_type(s: &str) -> Result<SkillType, String> {
    match s.to_lowercase().as_str() {
        "active" => Ok(SkillType::Active),
        "passive" => Ok(SkillType::Passive),
        _ => Err(format!("Unknown type '{}' (use: active, passive)", s)),
    }
}

/// List all skills as summary JSON array.
fn cmd_skill_list(data_dir: &std::path::Path) -> Result<String, String> {
    let path = data_dir.join("skills.json");
    let skills: Vec<SkillDefinition> = if path.exists() {
        json_store::load_json(&path).unwrap_or_default()
    } else {
        Vec::new()
    };
    let summaries: Vec<serde_json::Value> = skills.iter().map(|s| {
        let effect_count = s.ranks.first().map(|r| r.effects.len()).unwrap_or(0);
        json!({
            "name": s.name,
            "category": format!("{:?}", s.category),
            "type": format!("{:?}", s.skill_type),
            "effects": effect_count,
        })
    }).collect();
    serde_json::to_string_pretty(&summaries).map_err(|e| e.to_string())
}

/// Show full skill definition as JSON.
fn cmd_skill_show(data_dir: &std::path::Path, name: &str) -> Result<String, String> {
    let path = data_dir.join("skills.json");
    let skills: Vec<SkillDefinition> = json_store::load_json(&path)
        .map_err(|_| "No skills library found".to_string())?;
    let skill = skills.iter().find(|s| s.name == name)
        .ok_or_else(|| format!("Skill '{}' not found", name))?;
    serde_json::to_string_pretty(skill).map_err(|e| e.to_string())
}

/// Create a new skill definition with a Novice rank and effects.
fn cmd_skill_create(
    data_dir: &std::path::Path, name: &str, category: &str, skill_type: &str,
    description: &str, effect_names: &[String], base_values: &[f64],
    unlock_levels: &[u32], effect_descs: &[String],
) -> Result<String, String> {
    let path = data_dir.join("skills.json");
    let mut skills: Vec<SkillDefinition> = if path.exists() {
        json_store::load_json(&path).unwrap_or_default()
    } else {
        Vec::new()
    };
    if skills.iter().any(|s| s.name == name) {
        return Err(format!("Skill '{}' already exists", name));
    }
    let effects: Vec<SkillEffect> = effect_names.iter().enumerate().map(|(i, n)| {
        SkillEffect {
            name: Some(n.clone()),
            description: effect_descs.get(i).cloned().unwrap_or_default(),
            base_value: *base_values.get(i).unwrap_or(&0.0),
            unlock_level: *unlock_levels.get(i).unwrap_or(&0),
        }
    }).collect();
    let skill = SkillDefinition {
        name: name.to_string(),
        category: parse_category(category)?,
        skill_type: parse_skill_type(skill_type)?,
        description: description.to_string(),
        ranks: vec![RankDefinition {
            rank: MasteryRank::Novice,
            description: String::new(),
            effects,
        }],
    };
    skills.push(skill);
    json_store::save_json(&path, &skills)?;
    Ok(json_ok(&format!("Created skill '{}'", name)))
}

/// Delete a skill from the library.
fn cmd_skill_delete(data_dir: &std::path::Path, name: &str) -> Result<String, String> {
    let path = data_dir.join("skills.json");
    let mut skills: Vec<SkillDefinition> = json_store::load_json(&path)
        .map_err(|_| "No skills library found".to_string())?;
    let before = skills.len();
    skills.retain(|s| s.name != name);
    if skills.len() == before {
        return Err(format!("Skill '{}' not found", name));
    }
    json_store::save_json(&path, &skills)?;
    Ok(json_ok(&format!("Deleted skill '{}'", name)))
}

/// Update a skill definition — modify metadata and add/remove effects on Novice rank.
fn cmd_skill_update(
    data_dir: &std::path::Path, name: &str,
    description: Option<&str>, category: Option<&str>, skill_type: Option<&str>,
    add_effect_names: &[String], base_values: &[f64],
    unlock_levels: &[u32], effect_descs: &[String],
    remove_effects: &[String],
) -> Result<String, String> {
    let path = data_dir.join("skills.json");
    let mut skills: Vec<SkillDefinition> = json_store::load_json(&path)
        .map_err(|_| "No skills library found".to_string())?;
    let skill = skills.iter_mut().find(|s| s.name == name)
        .ok_or_else(|| format!("Skill '{}' not found", name))?;

    if let Some(d) = description { skill.description = d.to_string(); }
    if let Some(c) = category { skill.category = parse_category(c)?; }
    if let Some(t) = skill_type { skill.skill_type = parse_skill_type(t)?; }

    // Ensure at least a Novice rank exists.
    if skill.ranks.is_empty() {
        skill.ranks.push(RankDefinition {
            rank: MasteryRank::Novice,
            description: String::new(),
            effects: vec![],
        });
    }
    let novice = &mut skill.ranks[0];

    // Remove effects by name.
    for ename in remove_effects {
        novice.effects.retain(|e| e.name.as_deref() != Some(ename.as_str()));
    }

    // Add new effects.
    for (i, ename) in add_effect_names.iter().enumerate() {
        novice.effects.push(SkillEffect {
            name: Some(ename.clone()),
            description: effect_descs.get(i).cloned().unwrap_or_default(),
            base_value: *base_values.get(i).unwrap_or(&0.0),
            unlock_level: *unlock_levels.get(i).unwrap_or(&0),
        });
    }

    json_store::save_json(&path, &skills)?;
    Ok(json_ok(&format!("Updated skill '{}'", name)))
}

// ---------------------------------------------------------------------------
// Class library commands
// ---------------------------------------------------------------------------

/// List all classes as summary JSON array.
fn cmd_class_list(data_dir: &std::path::Path) -> Result<String, String> {
    let path = data_dir.join("classes.json");
    let classes: Vec<ClassDefinition> = if path.exists() {
        json_store::load_json(&path).unwrap_or_default()
    } else {
        Vec::new()
    };
    let summaries: Vec<serde_json::Value> = classes.iter().map(|p| {
        json!({
            "name": p.name,
            "skills": p.skills,
            "passive": p.passive_name,
        })
    }).collect();
    serde_json::to_string_pretty(&summaries).map_err(|e| e.to_string())
}

/// Show full class definition as JSON.
fn cmd_class_show(data_dir: &std::path::Path, name: &str) -> Result<String, String> {
    let path = data_dir.join("classes.json");
    let classes: Vec<ClassDefinition> = json_store::load_json(&path)
        .map_err(|_| "No class library found".to_string())?;
    let cls = classes.iter().find(|p| p.name == name)
        .ok_or_else(|| format!("Class '{}' not found", name))?;
    serde_json::to_string_pretty(cls).map_err(|e| e.to_string())
}

/// Create a new class definition.
#[allow(clippy::too_many_arguments)]
fn cmd_class_create(
    data_dir: &std::path::Path, name: &str, description: &str,
    passive_name: &str, passive_desc: &str, skills: &[String],
    bonus_str: u32, bonus_agi: u32, bonus_end: u32,
    bonus_int: u32, bonus_wis: u32, bonus_per: u32, bonus_free: u32,
) -> Result<String, String> {
    let path = data_dir.join("classes.json");
    let mut classes: Vec<ClassDefinition> = if path.exists() {
        json_store::load_json(&path).unwrap_or_default()
    } else {
        Vec::new()
    };
    if classes.iter().any(|p| p.name == name) {
        return Err(format!("Class '{}' already exists", name));
    }
    classes.push(ClassDefinition {
        name: name.to_string(),
        description: description.to_string(),
        skills: skills.to_vec(),
        passive_name: passive_name.to_string(),
        passive_description: passive_desc.to_string(),
        bonus_str, bonus_agi, bonus_end, bonus_int, bonus_wis, bonus_per,
        bonus_free_points: bonus_free,
    });
    json_store::save_json(&path, &classes)?;
    Ok(json_ok(&format!("Created class '{}'", name)))
}

/// Delete a class from the library.
fn cmd_class_delete(data_dir: &std::path::Path, name: &str) -> Result<String, String> {
    let path = data_dir.join("classes.json");
    let mut classes: Vec<ClassDefinition> = json_store::load_json(&path)
        .map_err(|_| "No class library found".to_string())?;
    let before = classes.len();
    classes.retain(|p| p.name != name);
    if classes.len() == before {
        return Err(format!("Class '{}' not found", name));
    }
    json_store::save_json(&path, &classes)?;
    Ok(json_ok(&format!("Deleted class '{}'", name)))
}

/// Update a class — modify metadata, add/remove skills, set per-stat bonuses.
#[allow(clippy::too_many_arguments)]
fn cmd_class_update(
    data_dir: &std::path::Path, name: &str,
    description: Option<&str>, passive_name: Option<&str>, passive_desc: Option<&str>,
    add_skills: &[String], remove_skills: &[String],
    bonus_str: Option<u32>, bonus_agi: Option<u32>, bonus_end: Option<u32>,
    bonus_int: Option<u32>, bonus_wis: Option<u32>, bonus_per: Option<u32>, bonus_free: Option<u32>,
) -> Result<String, String> {
    let path = data_dir.join("classes.json");
    let mut classes: Vec<ClassDefinition> = json_store::load_json(&path)
        .map_err(|_| "No class library found".to_string())?;
    let cls = classes.iter_mut().find(|p| p.name == name)
        .ok_or_else(|| format!("Class '{}' not found", name))?;

    if let Some(d) = description { cls.description = d.to_string(); }
    if let Some(n) = passive_name { cls.passive_name = n.to_string(); }
    if let Some(d) = passive_desc { cls.passive_description = d.to_string(); }
    if let Some(v) = bonus_str { cls.bonus_str = v; }
    if let Some(v) = bonus_agi { cls.bonus_agi = v; }
    if let Some(v) = bonus_end { cls.bonus_end = v; }
    if let Some(v) = bonus_int { cls.bonus_int = v; }
    if let Some(v) = bonus_wis { cls.bonus_wis = v; }
    if let Some(v) = bonus_per { cls.bonus_per = v; }
    if let Some(v) = bonus_free { cls.bonus_free_points = v; }
    for s in remove_skills { cls.skills.retain(|sk| sk != s); }
    for s in add_skills { cls.skills.push(s.clone()); }

    json_store::save_json(&path, &classes)?;
    Ok(json_ok(&format!("Updated class '{}'", name)))
}

// ---------------------------------------------------------------------------
// JSON conversion helper
// ---------------------------------------------------------------------------

/// Build a serde_json::Value representing the full character state,
/// including computed xp_percentage and enriched skill metadata.
fn character_to_json(
    character: &Character,
    skill_library: &[SkillDefinition],
) -> serde_json::Value {
    let tier = character.grade.numeric();
    let pct = xp::xp_percentage(character.xp, character.level, tier);
    // Round to 2 decimal places.
    let pct_rounded = (pct * 100.0).round() / 100.0;

    // Build enriched skill list — look up category and type from the library.
    let skills: Vec<serde_json::Value> = character
        .skills
        .iter()
        .map(|cs| {
            let (category, skill_type) = skill_library
                .iter()
                .find(|d| d.name == cs.definition_name)
                .map(|d| (format!("{:?}", d.category), format!("{:?}", d.skill_type)))
                .unwrap_or_else(|| ("Unknown".to_string(), "Unknown".to_string()));

            json!({
                "name": cs.definition_name,
                "category": category,
                "type": skill_type,
                "rank": cs.rank.name(),
                "level": cs.level
            })
        })
        .collect();

    // Build classes list.
    let classes: Vec<serde_json::Value> = character
        .classes
        .iter()
        .map(|cp| {
            json!({
                "name": cp.definition_name,
                "level": cp.level,
                "passive_rank": cp.passive_rank
            })
        })
        .collect();

    // Build innate skill (or null).
    let innate = character.innate_skill.as_ref().map(|is| {
        json!({
            "name": is.definition_name,
            "level": is.level
        })
    });

    json!({
        "name": character.name,
        "race": character.race,
        "grade": character.grade.name(),
        "level": character.level,
        "xp": character.xp,
        "xp_percentage": pct_rounded,
        "unspent_attribute_points": character.unspent_attribute_points,
        "class_slots": character.class_slots,
        "bonus_attribute_points_per_level": character.bonus_attribute_points_per_level,
        "attributes": {
            "strength": character.attributes.strength,
            "agility": character.attributes.agility,
            "endurance": character.attributes.endurance,
            "intelligence": character.attributes.intelligence,
            "wisdom": character.attributes.wisdom,
            "perception": character.attributes.perception
        },
        "innate_skill": innate,
        "classes": classes,
        "skills": skills
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::attribute::Attributes;
    use crate::models::character::Character;
    use crate::storage::json_store;
    use std::fs;

    /// Create a unique temp directory for each test, with a characters subdirectory.
    /// Uses a static counter to avoid collisions between parallel tests in the same process.
    fn test_data_dir() -> std::path::PathBuf {
        use std::sync::atomic::{AtomicU32, Ordering};
        static COUNTER: AtomicU32 = AtomicU32::new(0);
        let id = COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!(
            "litrpg_cli_test_{}_{}", std::process::id(), id
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("characters")).unwrap();
        dir
    }

    #[test]
    fn test_list_empty() {
        let dir = test_data_dir();
        let char_dir = dir.join("characters");
        let result = cmd_list(&char_dir).unwrap();
        assert_eq!(result, "[]");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_show_character() {
        let dir = test_data_dir();
        let char_dir = dir.join("characters");

        // Save a character directly via json_store.
        let attrs = Attributes::new_clamped(7, 6, 5, 4, 3, 2);
        let character = Character::new("TestHero".to_string(), attrs, None);
        json_store::save_character(&char_dir, &character).unwrap();

        // Show via CLI handler.
        let result = cmd_show(&dir, &char_dir, "TestHero").unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();

        assert_eq!(parsed["name"], "TestHero");
        assert_eq!(parsed["race"], "Human");
        assert_eq!(parsed["grade"], "G");
        assert_eq!(parsed["level"], 0);
        assert_eq!(parsed["xp"], 0.0);
        assert_eq!(parsed["attributes"]["strength"], 7);
        assert_eq!(parsed["attributes"]["agility"], 6);
        assert_eq!(parsed["attributes"]["endurance"], 5);
        assert_eq!(parsed["attributes"]["intelligence"], 4);
        assert_eq!(parsed["attributes"]["wisdom"], 3);
        assert_eq!(parsed["attributes"]["perception"], 2);
        assert!(parsed["innate_skill"].is_null());
        assert_eq!(parsed["skills"].as_array().unwrap().len(), 0);
        assert_eq!(parsed["classes"].as_array().unwrap().len(), 0);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_create_character() {
        let dir = test_data_dir();
        let char_dir = dir.join("characters");

        let result = cmd_create(&char_dir, "NewChar", 8, 7, 6, 5, 4, 3, None).unwrap();
        assert!(result.contains("Created character 'NewChar'"));

        // Verify the character was saved correctly.
        let loaded = json_store::load_character(&char_dir, "NewChar").unwrap();
        assert_eq!(loaded.name, "NewChar");
        assert_eq!(loaded.attributes.strength, 8);
        assert_eq!(loaded.attributes.agility, 7);
        assert_eq!(loaded.attributes.endurance, 6);
        assert_eq!(loaded.attributes.intelligence, 5);
        assert_eq!(loaded.attributes.wisdom, 4);
        assert_eq!(loaded.attributes.perception, 3);
        assert_eq!(loaded.level, 0);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_create_duplicate() {
        let dir = test_data_dir();
        let char_dir = dir.join("characters");

        cmd_create(&char_dir, "Dupe", 5, 5, 5, 5, 5, 5, None).unwrap();
        let result = cmd_create(&char_dir, "Dupe", 5, 5, 5, 5, 5, 5, None);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("already exists"));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_delete_character() {
        let dir = test_data_dir();
        let char_dir = dir.join("characters");

        // Create, then delete.
        cmd_create(&char_dir, "ToDelete", 5, 5, 5, 5, 5, 5, None).unwrap();
        let result = cmd_delete(&char_dir, "ToDelete").unwrap();
        assert!(result.contains("Deleted character 'ToDelete'"));

        // Verify load now fails.
        assert!(json_store::load_character(&char_dir, "ToDelete").is_err());

        let _ = fs::remove_dir_all(&dir);
    }

    // --- Update command tests ---

    #[test]
    fn test_update_level_up() {
        let dir = test_data_dir();
        let char_dir = dir.join("characters");
        cmd_create(&char_dir, "Kael", 5, 5, 5, 5, 5, 5, None).unwrap();

        let result = cmd_update(&dir, "Kael", &UpdateOpts {
            level_up: Some(3),
            ..Default::default()
        });
        assert!(result.is_ok());
        let c = json_store::load_character(&char_dir, "Kael").unwrap();
        assert_eq!(c.level, 3);
        assert!(c.unspent_attribute_points > 0);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_update_grade_up() {
        let dir = test_data_dir();
        let char_dir = dir.join("characters");
        cmd_create(&char_dir, "Kael", 5, 5, 5, 5, 5, 5, None).unwrap();

        // Level up first, then grade up — should reset level.
        cmd_update(&dir, "Kael", &UpdateOpts {
            level_up: Some(5),
            ..Default::default()
        }).unwrap();
        let result = cmd_update(&dir, "Kael", &UpdateOpts {
            grade_up: true,
            ..Default::default()
        });
        assert!(result.is_ok());
        let c = json_store::load_character(&char_dir, "Kael").unwrap();
        assert_eq!(c.grade, crate::models::grade::Grade::F);
        assert_eq!(c.level, 0);
        assert_eq!(c.xp, 0.0);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_update_add_remove_skill() {
        let dir = test_data_dir();
        let char_dir = dir.join("characters");

        // Create a skill in the library.
        cmd_skill_create(&dir, "Fireball", "acquired", "active", "A fire spell",
            &["Fire Damage".to_string()], &[20.0], &[0], &["Burns".to_string()]).unwrap();
        cmd_create(&char_dir, "Kael", 5, 5, 5, 5, 5, 5, None).unwrap();

        // Add skill.
        cmd_update(&dir, "Kael", &UpdateOpts {
            add_skill: vec!["Fireball".to_string()],
            ..Default::default()
        }).unwrap();
        let c = json_store::load_character(&char_dir, "Kael").unwrap();
        assert_eq!(c.skills.len(), 1);
        assert_eq!(c.skills[0].definition_name, "Fireball");

        // Remove skill.
        cmd_update(&dir, "Kael", &UpdateOpts {
            remove_skill: vec!["Fireball".to_string()],
            ..Default::default()
        }).unwrap();
        let c = json_store::load_character(&char_dir, "Kael").unwrap();
        assert_eq!(c.skills.len(), 0);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_update_add_attr() {
        let dir = test_data_dir();
        let char_dir = dir.join("characters");
        cmd_create(&char_dir, "Kael", 5, 5, 5, 5, 5, 5, None).unwrap();

        // Level up to get points, then distribute.
        cmd_update(&dir, "Kael", &UpdateOpts {
            level_up: Some(1),
            add_attr: vec!["str:2".to_string()],
            ..Default::default()
        }).unwrap();
        let c = json_store::load_character(&char_dir, "Kael").unwrap();
        assert_eq!(c.attributes.strength, 7); // 5 + 2

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_update_add_class() {
        let dir = test_data_dir();
        let char_dir = dir.join("characters");

        cmd_class_create(&dir, "Mage", "Casts spells", "Arcane Focus", "Mana regen",
            &["Fireball".to_string()], 0, 0, 0, 0, 0, 0, 0).unwrap();
        cmd_create(&char_dir, "Kael", 5, 5, 5, 5, 5, 5, None).unwrap();

        // Create a skill the class grants.
        cmd_skill_create(&dir, "Fireball", "acquired", "active", "Fire",
            &["Fire Damage".to_string()], &[20.0], &[0], &["Burns".to_string()]).unwrap();

        cmd_update(&dir, "Kael", &UpdateOpts {
            add_class: vec!["Mage".to_string()],
            ..Default::default()
        }).unwrap();
        let c = json_store::load_character(&char_dir, "Kael").unwrap();
        assert_eq!(c.classes.len(), 1);
        assert_eq!(c.classes[0].definition_name, "Mage");
        // Class's granted skill should be auto-added.
        assert_eq!(c.skills.len(), 1);
        assert_eq!(c.skills[0].definition_name, "Fireball");

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_update_show_flag() {
        let dir = test_data_dir();
        let char_dir = dir.join("characters");
        cmd_create(&char_dir, "Kael", 5, 5, 5, 5, 5, 5, None).unwrap();

        let result = cmd_update(&dir, "Kael", &UpdateOpts {
            level_up: Some(1),
            show: true,
            ..Default::default()
        }).unwrap();
        let json: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(json["name"], "Kael");
        assert_eq!(json["level"], 1);

        let _ = fs::remove_dir_all(&dir);
    }

    // --- Skill library tests ---

    #[test]
    fn test_skill_create_and_list() {
        let dir = test_data_dir();
        cmd_skill_create(&dir, "Fireball", "acquired", "active", "A fire spell",
            &["Fire Damage".to_string()], &[20.0], &[0], &["Burns".to_string()]).unwrap();
        let result = cmd_skill_list(&dir).unwrap();
        let json: Vec<serde_json::Value> = serde_json::from_str(&result).unwrap();
        assert_eq!(json.len(), 1);
        assert_eq!(json[0]["name"], "Fireball");

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_skill_show() {
        let dir = test_data_dir();
        cmd_skill_create(&dir, "Fireball", "acquired", "active", "A fire spell",
            &["Fire Damage".to_string()], &[20.0], &[0], &["Burns".to_string()]).unwrap();
        let result = cmd_skill_show(&dir, "Fireball").unwrap();
        let json: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(json["name"], "Fireball");
        assert_eq!(json["ranks"][0]["effects"][0]["base_value"], 20.0);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_skill_delete() {
        let dir = test_data_dir();
        cmd_skill_create(&dir, "Fireball", "acquired", "active", "",
            &[], &[], &[], &[]).unwrap();
        cmd_skill_delete(&dir, "Fireball").unwrap();
        let result = cmd_skill_list(&dir).unwrap();
        let json: Vec<serde_json::Value> = serde_json::from_str(&result).unwrap();
        assert_eq!(json.len(), 0);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_skill_update_add_remove_effect() {
        let dir = test_data_dir();
        cmd_skill_create(&dir, "Fireball", "acquired", "active", "Fire",
            &["Fire Damage".to_string()], &[20.0], &[0], &["Burns".to_string()]).unwrap();

        // Add an effect.
        cmd_skill_update(&dir, "Fireball", None, None, None,
            &["Burn DOT".to_string()], &[5.0], &[5], &["DOT".to_string()],
            &[]).unwrap();
        let result = cmd_skill_show(&dir, "Fireball").unwrap();
        let json: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(json["ranks"][0]["effects"].as_array().unwrap().len(), 2);

        // Remove an effect.
        cmd_skill_update(&dir, "Fireball", None, None, None,
            &[], &[], &[], &[],
            &["Fire Damage".to_string()]).unwrap();
        let result = cmd_skill_show(&dir, "Fireball").unwrap();
        let json: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(json["ranks"][0]["effects"].as_array().unwrap().len(), 1);

        let _ = fs::remove_dir_all(&dir);
    }

    // --- Class library tests ---

    #[test]
    fn test_class_create_list_show() {
        let dir = test_data_dir();
        cmd_class_create(&dir, "Blacksmith", "Craft weapons", "Forgeborn", "Crafting speed",
            &["Hammer Strike".to_string()], 0, 0, 0, 0, 0, 0, 0).unwrap();
        let list = cmd_class_list(&dir).unwrap();
        let json: Vec<serde_json::Value> = serde_json::from_str(&list).unwrap();
        assert_eq!(json.len(), 1);
        assert_eq!(json[0]["name"], "Blacksmith");

        let show = cmd_class_show(&dir, "Blacksmith").unwrap();
        let json: serde_json::Value = serde_json::from_str(&show).unwrap();
        assert_eq!(json["passive_name"], "Forgeborn");

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_class_delete() {
        let dir = test_data_dir();
        cmd_class_create(&dir, "Blacksmith", "", "", "", &[], 0, 0, 0, 0, 0, 0, 0).unwrap();
        cmd_class_delete(&dir, "Blacksmith").unwrap();
        let list = cmd_class_list(&dir).unwrap();
        let json: Vec<serde_json::Value> = serde_json::from_str(&list).unwrap();
        assert_eq!(json.len(), 0);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_class_update_add_remove_skill() {
        let dir = test_data_dir();
        cmd_class_create(&dir, "Blacksmith", "", "", "", &["Hammer".to_string()], 0, 0, 0, 0, 0, 0, 0).unwrap();

        cmd_class_update(&dir, "Blacksmith", None, None, None,
            &["Anvil".to_string()], &[], None, None, None, None, None, None, None).unwrap();
        let show = cmd_class_show(&dir, "Blacksmith").unwrap();
        let json: serde_json::Value = serde_json::from_str(&show).unwrap();
        assert_eq!(json["skills"].as_array().unwrap().len(), 2);

        cmd_class_update(&dir, "Blacksmith", None, None, None,
            &[], &["Hammer".to_string()], None, None, None, None, None, None, None).unwrap();
        let show = cmd_class_show(&dir, "Blacksmith").unwrap();
        let json: serde_json::Value = serde_json::from_str(&show).unwrap();
        assert_eq!(json["skills"].as_array().unwrap().len(), 1);

        let _ = fs::remove_dir_all(&dir);
    }

    // --- Clear command tests ---

    #[test]
    fn test_clear_characters() {
        let dir = test_data_dir();
        let char_dir = dir.join("characters");
        cmd_create(&char_dir, "Alpha", 5, 5, 5, 5, 5, 5, None).unwrap();
        cmd_create(&char_dir, "Beta",  5, 5, 5, 5, 5, 5, None).unwrap();
        assert_eq!(json_store::list_characters(&char_dir).unwrap().len(), 2);

        let result = cmd_clear(&dir, &char_dir, ClearTarget::Characters).unwrap();
        assert!(result.contains("Cleared characters"));
        assert_eq!(json_store::list_characters(&char_dir).unwrap().len(), 0);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_clear_skills() {
        let dir = test_data_dir();
        let char_dir = dir.join("characters");
        cmd_skill_create(&dir, "Fireball", "acquired", "active", "",
            &[], &[], &[], &[]).unwrap();
        cmd_skill_create(&dir, "Ice Shard", "acquired", "active", "",
            &[], &[], &[], &[]).unwrap();
        let list: Vec<serde_json::Value> =
            serde_json::from_str(&cmd_skill_list(&dir).unwrap()).unwrap();
        assert_eq!(list.len(), 2);

        let result = cmd_clear(&dir, &char_dir, ClearTarget::Skills).unwrap();
        assert!(result.contains("Cleared skill library"));
        let list: Vec<serde_json::Value> =
            serde_json::from_str(&cmd_skill_list(&dir).unwrap()).unwrap();
        assert_eq!(list.len(), 0);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_clear_classes() {
        let dir = test_data_dir();
        let char_dir = dir.join("characters");
        cmd_class_create(&dir, "Mage", "", "", "", &[], 0, 0, 0, 0, 0, 0, 0).unwrap();
        cmd_class_create(&dir, "Warrior", "", "", "", &[], 0, 0, 0, 0, 0, 0, 0).unwrap();
        let list: Vec<serde_json::Value> =
            serde_json::from_str(&cmd_class_list(&dir).unwrap()).unwrap();
        assert_eq!(list.len(), 2);

        let result = cmd_clear(&dir, &char_dir, ClearTarget::Classes).unwrap();
        assert!(result.contains("Cleared class library"));
        let list: Vec<serde_json::Value> =
            serde_json::from_str(&cmd_class_list(&dir).unwrap()).unwrap();
        assert_eq!(list.len(), 0);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_clear_all() {
        let dir = test_data_dir();
        let char_dir = dir.join("characters");

        cmd_create(&char_dir, "Hero", 5, 5, 5, 5, 5, 5, None).unwrap();
        cmd_skill_create(&dir, "Slash", "acquired", "active", "",
            &[], &[], &[], &[]).unwrap();
        cmd_class_create(&dir, "Fighter", "", "", "", &[], 0, 0, 0, 0, 0, 0, 0).unwrap();

        let result = cmd_clear(&dir, &char_dir, ClearTarget::All).unwrap();
        assert!(result.contains("Cleared all data"));

        assert_eq!(json_store::list_characters(&char_dir).unwrap().len(), 0);
        let skills: Vec<serde_json::Value> =
            serde_json::from_str(&cmd_skill_list(&dir).unwrap()).unwrap();
        assert_eq!(skills.len(), 0);
        let classes: Vec<serde_json::Value> =
            serde_json::from_str(&cmd_class_list(&dir).unwrap()).unwrap();
        assert_eq!(classes.len(), 0);

        let _ = fs::remove_dir_all(&dir);
    }

    /// Clears the real live data directory after all other tests have run.
    /// Run with: cargo test test_z_teardown_live_data -- --ignored
    #[test]
    #[ignore]
    fn test_z_teardown_live_data() {
        let data_dir = crate::data_dir();
        let char_dir = data_dir.join("characters");
        cmd_clear(&data_dir, &char_dir, ClearTarget::All)
            .expect("failed to clear live data");
    }

    // --- Full integration test ---

    #[test]
    fn test_full_roundtrip() {
        let dir = test_data_dir();
        let char_dir = dir.join("characters");

        // Create a skill.
        cmd_skill_create(&dir, "Fireball", "acquired", "active", "Fire",
            &["Fire Damage".to_string()], &[20.0], &[0], &["Burns".to_string()]).unwrap();

        // Create a class.
        cmd_class_create(&dir, "Mage", "Casts spells", "Arcane Focus", "Mana regen",
            &["Fireball".to_string()], 0, 0, 0, 0, 0, 0, 0).unwrap();

        // Create a character.
        cmd_create(&char_dir, "Kael", 8, 5, 6, 3, 5, 7, None).unwrap();

        // Update: level up, add skill, add class, distribute attrs, show.
        let result = cmd_update(&dir, "Kael", &UpdateOpts {
            level_up: Some(5),
            add_skill: vec!["Fireball".to_string()],
            add_class: vec!["Mage".to_string()],
            add_attr: vec!["str:3".to_string()],
            show: true,
            ..Default::default()
        }).unwrap();

        let json: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(json["name"], "Kael");
        assert_eq!(json["level"], 5);
        assert_eq!(json["skills"].as_array().unwrap().len(), 1);
        assert_eq!(json["classes"].as_array().unwrap().len(), 1);
        assert_eq!(json["attributes"]["strength"], 11); // 8 + 3

        let _ = fs::remove_dir_all(&dir);
    }
}
