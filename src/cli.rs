/// CLI interface for the LITRPG System.
///
/// Provides JSON-based commands for managing characters, skills, and
/// professions without launching the TUI. All output is JSON to stdout;
/// errors go to stderr with exit code 1.

use std::path::PathBuf;
use std::process;

use clap::{Parser, Subcommand};
use serde_json::json;

use crate::formulas::xp;
use crate::models::attribute::Attributes;
use crate::models::character::{Character, CharacterInnateSkill};
use crate::models::skill::SkillDefinition;
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
        /// Add a profession from the library (respects slot limit).
        #[arg(long)]
        add_profession: Option<String>,
        /// Remove a profession by name.
        #[arg(long)]
        remove_profession: Option<String>,
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

    /// Manage the profession library.
    Profession {
        #[command(subcommand)]
        action: ProfessionAction,
    },
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

/// Profession library subcommands.
#[derive(Subcommand)]
pub enum ProfessionAction {
    /// List all professions.
    List,

    /// Show full profession definition.
    Show {
        /// Profession name.
        name: String,
    },

    /// Create a new profession definition.
    Create {
        /// Profession name.
        name: String,
        /// Profession description.
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
    },

    /// Delete a profession from the library.
    Delete {
        /// Profession name.
        name: String,
    },

    /// Update an existing profession definition.
    Update {
        /// Profession name.
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
        /// Add a skill to the profession's granted skills.
        #[arg(long)]
        add_skill: Option<String>,
        /// Remove a skill from the profession's granted skills.
        #[arg(long)]
        remove_skill: Option<String>,
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
    let data_dir = PathBuf::from("data");
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
        Command::Update { .. } => {
            todo!("Update command will be implemented in Tasks 4-5")
        }
        Command::Skill { .. } => {
            todo!("Skill commands will be implemented in Tasks 6-8")
        }
        Command::Profession { .. } => {
            todo!("Profession commands will be implemented in Tasks 6-8")
        }
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

    // Build professions list.
    let professions: Vec<serde_json::Value> = character
        .professions
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
        "profession_slots": character.profession_slots,
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
        "professions": professions,
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
        assert_eq!(parsed["professions"].as_array().unwrap().len(), 0);

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
}
