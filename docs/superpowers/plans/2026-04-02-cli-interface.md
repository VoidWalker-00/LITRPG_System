# CLI Interface Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a CLI mode to the LITRPG binary so AI chatbots can manage characters, skills, and professions programmatically via JSON.

**Architecture:** A new `src/cli.rs` module defines clap subcommands and dispatches to existing model/storage/formula code. `main.rs` checks for subcommands before launching the TUI. CLI has zero dependency on `ui/`.

**Tech Stack:** Rust, clap 4 (derive API), serde_json, existing models/storage/formulas layers.

**Spec:** `docs/superpowers/specs/2026-04-01-cli-interface-design.md`

---

## File Structure

| File | Action | Responsibility |
|------|--------|---------------|
| `src/cli.rs` | Create | Clap struct definitions, JSON output helpers, all subcommand dispatch logic |
| `src/main.rs` | Modify | Add `mod cli`, wrap TUI in an else branch when no subcommand given |
| `Cargo.toml` | Modify | Already has `clap` dep added — verify it's present |

---

## Chunk 1: Foundation — Clap Setup and Character List/Show

### Task 1: Clap skeleton and `list` command

**Files:**
- Create: `src/cli.rs`
- Modify: `src/main.rs`

- [ ] **Step 1: Write test for character list command**

Create a test in `src/cli.rs` that verifies `list_characters` returns a JSON array. We test the output function directly rather than spawning a process.

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn test_data_dir() -> std::path::PathBuf {
        let dir = std::env::temp_dir().join("litrpg_cli_test");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("characters")).unwrap();
        dir
    }

    #[test]
    fn test_list_empty() {
        let dir = test_data_dir();
        let result = cmd_list(&dir);
        assert!(result.is_ok());
        // Output is "[]"
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test test_list_empty -- --nocapture`
Expected: FAIL — `cmd_list` does not exist yet.

- [ ] **Step 3: Create `src/cli.rs` with clap structs and `cmd_list`**

```rust
use clap::{Parser, Subcommand};
use std::path::{Path, PathBuf};
use crate::storage::json_store;
use crate::models::character::Character;
use crate::models::skill::SkillDefinition;
use crate::models::profession::ProfessionDefinition;

#[derive(Parser)]
#[command(name = "litrpg", about = "LITRPG System — CLI and TUI")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Subcommand)]
pub enum Command {
    /// List all saved characters.
    List,
    /// Show full character stats as JSON.
    Show { name: String },
    /// Create a new character.
    Create {
        name: String,
        #[arg(long, default_value_t = 5)] str_val: u32,  // --str conflicts with Rust keyword
        #[arg(long, default_value_t = 5)] agi: u32,
        #[arg(long, default_value_t = 5)] end: u32,
        #[arg(long, default_value_t = 5)] int: u32,
        #[arg(long, default_value_t = 5)] wis: u32,
        #[arg(long, default_value_t = 5)] per: u32,
        #[arg(long)] innate: Option<String>,
    },
    /// Delete a character.
    Delete { name: String },
    /// Update a character with mutations.
    Update {
        name: String,
        #[arg(long)] level_up: Option<u32>,
        #[arg(long)] grade_up: bool,
        #[arg(long)] add_skill: Vec<String>,
        #[arg(long)] remove_skill: Vec<String>,
        #[arg(long)] add_profession: Vec<String>,
        #[arg(long)] remove_profession: Vec<String>,
        #[arg(long)] add_attr: Vec<String>,
        #[arg(long)] remove_attr: Vec<String>,
        #[arg(long)] kill: Vec<String>,
        #[arg(long)] show: bool,
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

#[derive(Subcommand)]
pub enum SkillAction {
    List,
    Show { name: String },
    Create {
        name: String,
        #[arg(long, default_value = "acquired")] category: String,
        #[arg(long, default_value = "active")] r#type: String,
        #[arg(long, default_value = "")] description: String,
        #[arg(long)] effect: Vec<String>,
        #[arg(long)] base_value: Vec<f64>,
        #[arg(long)] unlock_level: Vec<u32>,
        #[arg(long)] effect_desc: Vec<String>,
    },
    Delete { name: String },
    Update {
        name: String,
        #[arg(long)] description: Option<String>,
        #[arg(long)] category: Option<String>,
        #[arg(long)] r#type: Option<String>,
        #[arg(long)] add_effect: Vec<String>,
        #[arg(long)] base_value: Vec<f64>,
        #[arg(long)] unlock_level: Vec<u32>,
        #[arg(long)] effect_desc: Vec<String>,
        #[arg(long)] remove_effect: Vec<String>,
    },
}

#[derive(Subcommand)]
pub enum ProfessionAction {
    List,
    Show { name: String },
    Create {
        name: String,
        #[arg(long, default_value = "")] description: String,
        #[arg(long, default_value = "")] passive_name: String,
        #[arg(long, default_value = "")] passive_desc: String,
        #[arg(long, value_delimiter = ',')] skills: Vec<String>,
    },
    Delete { name: String },
    Update {
        name: String,
        #[arg(long)] description: Option<String>,
        #[arg(long)] passive_name: Option<String>,
        #[arg(long)] passive_desc: Option<String>,
        #[arg(long)] add_skill: Vec<String>,
        #[arg(long)] remove_skill: Vec<String>,
    },
}

/// JSON helpers.
fn json_ok(msg: &str) -> String {
    serde_json::json!({"status": "ok", "message": msg}).to_string()
}
fn json_err(msg: &str) -> String {
    serde_json::json!({"status": "error", "message": msg}).to_string()
}

/// Entry point — called from main when a subcommand is present.
pub fn run(cmd: Command) {
    let data_dir = PathBuf::from("data");
    let result = match cmd {
        Command::List => cmd_list(&data_dir),
        Command::Show { name } => cmd_show(&data_dir, &name),
        Command::Create { .. } => todo!(),
        Command::Delete { .. } => todo!(),
        Command::Update { .. } => todo!(),
        Command::Skill { .. } => todo!(),
        Command::Profession { .. } => todo!(),
    };
    match result {
        Ok(json) => println!("{}", json),
        Err(msg) => {
            eprintln!("{}", json_err(&msg));
            std::process::exit(1);
        }
    }
}

fn cmd_list(data_dir: &Path) -> Result<String, String> {
    let char_dir = data_dir.join("characters");
    let names = json_store::list_characters(&char_dir).unwrap_or_default();
    serde_json::to_string_pretty(&names).map_err(|e| e.to_string())
}

fn cmd_show(data_dir: &Path, name: &str) -> Result<String, String> {
    todo!()
}
```

- [ ] **Step 4: Add `mod cli` to `main.rs` and route subcommands**

In `src/main.rs`, add the module declaration and modify `main()`:

```rust
mod cli;

// At top of main(), before TUI setup:
use clap::Parser;
let cli_args = cli::Cli::parse();
if let Some(cmd) = cli_args.command {
    cli::run(cmd);
    return Ok(());
}
// ... existing TUI code continues below
```

- [ ] **Step 5: Run test to verify it passes**

Run: `cargo test test_list_empty -- --nocapture`
Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add src/cli.rs src/main.rs
git commit -m "feat: add CLI skeleton with clap and list command"
```

---

### Task 2: `show` command with JSON character output

**Files:**
- Modify: `src/cli.rs`

- [ ] **Step 1: Write test for show command**

```rust
#[test]
fn test_show_character() {
    let dir = test_data_dir();
    let attrs = crate::models::attribute::Attributes::new_clamped(8, 5, 6, 3, 5, 7);
    let char = Character::new("Kael".to_string(), attrs, None);
    json_store::save_character(&dir.join("characters"), &char).unwrap();

    let result = cmd_show(&dir, "Kael");
    assert!(result.is_ok());
    let json: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();
    assert_eq!(json["name"], "Kael");
    assert_eq!(json["grade"], "G");
    assert_eq!(json["level"], 0);
    assert_eq!(json["attributes"]["strength"], 8);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test test_show_character -- --nocapture`
Expected: FAIL — `cmd_show` returns `todo!()`.

- [ ] **Step 3: Implement `cmd_show`**

The `show` command needs a custom JSON shape (not raw serde of `Character`), because the spec adds `xp_percentage` and flattens `professions`/`skills` differently. Build a `serde_json::Value` manually:

```rust
use crate::formulas::xp;
use crate::models::attribute::AttributeKind;

fn cmd_show(data_dir: &Path, name: &str) -> Result<String, String> {
    let char_dir = data_dir.join("characters");
    let character = json_store::load_character(&char_dir, name)?;
    let json = character_to_json(&character, data_dir);
    serde_json::to_string_pretty(&json).map_err(|e| e.to_string())
}

fn character_to_json(c: &Character, data_dir: &Path) -> serde_json::Value {
    let xp_pct = xp::xp_percentage(c.xp, c.level, c.grade.numeric());

    let skills_json: Vec<serde_json::Value> = {
        let skills_path = data_dir.join("skills.json");
        let skill_lib: Vec<SkillDefinition> = json_store::load_json(&skills_path).unwrap_or_default();
        c.skills.iter().map(|s| {
            let (cat, stype) = skill_lib.iter()
                .find(|d| d.name == s.definition_name)
                .map(|d| (format!("{:?}", d.category), format!("{:?}", d.skill_type)))
                .unwrap_or(("Unknown".to_string(), "Unknown".to_string()));
            serde_json::json!({
                "name": s.definition_name,
                "category": cat,
                "type": stype,
                "rank": s.rank.name(),
                "level": s.level,
            })
        }).collect()
    };

    let profs_json: Vec<serde_json::Value> = c.professions.iter().map(|p| {
        serde_json::json!({
            "name": p.definition_name,
            "level": p.level,
            "passive_rank": p.passive_rank,
        })
    }).collect();

    let innate_json = c.innate_skill.as_ref().map(|i| {
        serde_json::json!({ "name": i.definition_name, "level": i.level })
    });

    serde_json::json!({
        "name": c.name,
        "race": c.race,
        "grade": c.grade.name(),
        "level": c.level,
        "xp": c.xp,
        "xp_percentage": (xp_pct * 100.0).round() / 100.0,
        "unspent_attribute_points": c.unspent_attribute_points,
        "profession_slots": c.profession_slots,
        "bonus_attribute_points_per_level": c.bonus_attribute_points_per_level,
        "attributes": {
            "strength": c.attributes.get(AttributeKind::Strength),
            "agility": c.attributes.get(AttributeKind::Agility),
            "endurance": c.attributes.get(AttributeKind::Endurance),
            "intelligence": c.attributes.get(AttributeKind::Intelligence),
            "wisdom": c.attributes.get(AttributeKind::Wisdom),
            "perception": c.attributes.get(AttributeKind::Perception),
        },
        "innate_skill": innate_json,
        "professions": profs_json,
        "skills": skills_json,
    })
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test test_show_character -- --nocapture`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/cli.rs
git commit -m "feat: implement CLI show command with JSON output"
```

---

### Task 3: `create` and `delete` character commands

**Files:**
- Modify: `src/cli.rs`

- [ ] **Step 1: Write tests for create and delete**

```rust
#[test]
fn test_create_character() {
    let dir = test_data_dir();
    let result = cmd_create(&dir, "Aria", 8, 5, 6, 3, 5, 7, None);
    assert!(result.is_ok());
    // Verify saved to disk.
    let loaded = json_store::load_character(&dir.join("characters"), "Aria").unwrap();
    assert_eq!(loaded.name, "Aria");
    assert_eq!(loaded.attributes.get(AttributeKind::Strength), 8);
}

#[test]
fn test_create_duplicate() {
    let dir = test_data_dir();
    cmd_create(&dir, "Kael", 5, 5, 5, 5, 5, 5, None).unwrap();
    let result = cmd_create(&dir, "Kael", 5, 5, 5, 5, 5, 5, None);
    assert!(result.is_err());
}

#[test]
fn test_delete_character() {
    let dir = test_data_dir();
    cmd_create(&dir, "Kael", 5, 5, 5, 5, 5, 5, None).unwrap();
    let result = cmd_delete(&dir, "Kael");
    assert!(result.is_ok());
    assert!(json_store::load_character(&dir.join("characters"), "Kael").is_err());
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test test_create test_delete -- --nocapture`
Expected: FAIL

- [ ] **Step 3: Implement `cmd_create` and `cmd_delete`**

```rust
use crate::models::attribute::Attributes;
use crate::models::character::CharacterInnateSkill;

fn cmd_create(
    data_dir: &Path, name: &str,
    str_val: u32, agi: u32, end: u32, int: u32, wis: u32, per: u32,
    innate: Option<&str>,
) -> Result<String, String> {
    let char_dir = data_dir.join("characters");
    // Check for duplicate.
    let existing = json_store::list_characters(&char_dir).unwrap_or_default();
    if existing.iter().any(|n| n == name) {
        return Err(format!("Character '{}' already exists", name));
    }
    let attrs = Attributes::new_clamped(str_val, agi, end, int, wis, per);
    let innate_skill = innate.map(|n| CharacterInnateSkill {
        definition_name: n.to_string(),
        level: 0,
    });
    let character = Character::new(name.to_string(), attrs, innate_skill);
    json_store::save_character(&char_dir, &character)?;
    Ok(json_ok(&format!("Created character '{}'", name)))
}

fn cmd_delete(data_dir: &Path, name: &str) -> Result<String, String> {
    let char_dir = data_dir.join("characters");
    json_store::delete_character(&char_dir, name)?;
    Ok(json_ok(&format!("Deleted character '{}'", name)))
}
```

Wire them into the `run()` match.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test test_create test_delete -- --nocapture`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/cli.rs
git commit -m "feat: implement CLI create and delete character commands"
```

---

## Chunk 2: Character Update Mutations

### Task 4: `update` command — level-up, grade-up, kill

**Files:**
- Modify: `src/cli.rs`

- [ ] **Step 1: Write tests for level-up, grade-up, kill**

```rust
#[test]
fn test_update_level_up() {
    let dir = test_data_dir();
    cmd_create(&dir, "Kael", 5, 5, 5, 5, 5, 5, None).unwrap();
    let result = cmd_update(&dir, "Kael", &UpdateOpts {
        level_up: Some(3),
        ..Default::default()
    });
    assert!(result.is_ok());
    let c = json_store::load_character(&dir.join("characters"), "Kael").unwrap();
    assert_eq!(c.level, 3);
    assert!(c.unspent_attribute_points > 0);
}

#[test]
fn test_update_grade_up() {
    let dir = test_data_dir();
    cmd_create(&dir, "Kael", 5, 5, 5, 5, 5, 5, None).unwrap();
    let result = cmd_update(&dir, "Kael", &UpdateOpts {
        grade_up: true,
        ..Default::default()
    });
    assert!(result.is_ok());
    let c = json_store::load_character(&dir.join("characters"), "Kael").unwrap();
    assert_eq!(c.grade, crate::models::grade::Grade::F);
    assert_eq!(c.level, 0);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test test_update_level test_update_grade -- --nocapture`
Expected: FAIL

- [ ] **Step 3: Implement `cmd_update` with an `UpdateOpts` struct**

Define a plain struct to hold all mutation options (separate from clap to make it testable):

```rust
#[derive(Default)]
struct UpdateOpts {
    level_up: Option<u32>,
    grade_up: bool,
    add_skill: Vec<String>,
    remove_skill: Vec<String>,
    add_profession: Vec<String>,
    remove_profession: Vec<String>,
    add_attr: Vec<String>,    // "str:5" format
    remove_attr: Vec<String>, // "str:5" format
    kill: Vec<String>,        // "45:100" format
    show: bool,
}

fn cmd_update(data_dir: &Path, name: &str, opts: &UpdateOpts) -> Result<String, String> {
    let char_dir = data_dir.join("characters");
    let mut character = json_store::load_character(&char_dir, name)?;

    // Grade up.
    if opts.grade_up {
        character.grade = character.grade.next()
            .ok_or_else(|| "Already at max grade (SSS)".to_string())?;
        character.level = 0;
        character.xp = 0.0;
    }

    // Level up.
    if let Some(n) = opts.level_up {
        for _ in 0..n {
            if character.level >= 100 { break; }
            let required = xp::xp_required(character.level, character.grade.numeric());
            character.xp = (character.xp - required).max(0.0);
            character.level += 1;
            character.unspent_attribute_points += character.attribute_points_per_level();
        }
    }

    // Kill entry.
    for kill_str in &opts.kill {
        let parts: Vec<&str> = kill_str.split(':').collect();
        if parts.len() != 2 { return Err(format!("Invalid kill format '{}', expected level:count", kill_str)); }
        let enemy_level: u32 = parts[0].parse().map_err(|_| "Invalid enemy level".to_string())?;
        let count: u32 = parts[1].parse().map_err(|_| "Invalid kill count".to_string())?;
        for _ in 0..count {
            let xp_gain = xp::kill_xp(character.level, enemy_level);
            character.xp += xp_gain;
            while character.level < 100 {
                let required = xp::xp_required(character.level, character.grade.numeric());
                if character.xp >= required {
                    character.xp -= required;
                    character.level += 1;
                    character.unspent_attribute_points += character.attribute_points_per_level();
                } else { break; }
            }
        }
    }

    // (skill/profession/attr mutations added in next tasks)

    json_store::save_character(&char_dir, &character)?;

    if opts.show {
        let json = character_to_json(&character, data_dir);
        serde_json::to_string_pretty(&json).map_err(|e| e.to_string())
    } else {
        Ok(json_ok(&format!("Updated character '{}'", name)))
    }
}
```

Wire the `Command::Update` match arm to construct `UpdateOpts` from clap fields and call `cmd_update`.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test test_update -- --nocapture`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/cli.rs
git commit -m "feat: implement CLI update with level-up, grade-up, kill"
```

---

### Task 5: `update` command — skills, professions, attributes

**Files:**
- Modify: `src/cli.rs`

- [ ] **Step 1: Write tests for skill/profession/attribute mutations**

```rust
#[test]
fn test_update_add_remove_skill() {
    let dir = test_data_dir();
    // Create a skill in the library.
    let skill = SkillDefinition { /* Fireball, Acquired, Active, with one Novice rank */ };
    json_store::save_json(&dir.join("skills.json"), &vec![skill]).unwrap();
    cmd_create(&dir, "Kael", 5, 5, 5, 5, 5, 5, None).unwrap();

    // Add skill.
    cmd_update(&dir, "Kael", &UpdateOpts {
        add_skill: vec!["Fireball".to_string()],
        ..Default::default()
    }).unwrap();
    let c = json_store::load_character(&dir.join("characters"), "Kael").unwrap();
    assert_eq!(c.skills.len(), 1);
    assert_eq!(c.skills[0].definition_name, "Fireball");

    // Remove skill.
    cmd_update(&dir, "Kael", &UpdateOpts {
        remove_skill: vec!["Fireball".to_string()],
        ..Default::default()
    }).unwrap();
    let c = json_store::load_character(&dir.join("characters"), "Kael").unwrap();
    assert_eq!(c.skills.len(), 0);
}

#[test]
fn test_update_add_attr() {
    let dir = test_data_dir();
    cmd_create(&dir, "Kael", 5, 5, 5, 5, 5, 5, None).unwrap();
    // Level up to get points, then distribute.
    cmd_update(&dir, "Kael", &UpdateOpts {
        level_up: Some(1),
        add_attr: vec!["str:2".to_string()],
        ..Default::default()
    }).unwrap();
    let c = json_store::load_character(&dir.join("characters"), "Kael").unwrap();
    assert_eq!(c.attributes.get(AttributeKind::Strength), 7); // 5 + 2
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test test_update_add_remove test_update_add_attr -- --nocapture`
Expected: FAIL

- [ ] **Step 3: Add skill/profession/attribute mutations to `cmd_update`**

Add these blocks inside `cmd_update`, after the kill section:

```rust
    // Add skills.
    let skills_path = data_dir.join("skills.json");
    let skill_lib: Vec<SkillDefinition> = json_store::load_json(&skills_path).unwrap_or_default();
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

    // Add professions.
    let profs_path = data_dir.join("professions.json");
    let prof_lib: Vec<ProfessionDefinition> = json_store::load_json(&profs_path).unwrap_or_default();
    for prof_name in &opts.add_profession {
        if character.professions.len() as u32 >= character.profession_slots {
            return Err("Profession slot limit reached".to_string());
        }
        if !prof_lib.iter().any(|p| p.name == *prof_name) {
            return Err(format!("Profession '{}' not found in library", prof_name));
        }
        character.professions.push(CharacterProfession {
            definition_name: prof_name.clone(),
            level: 0,
            passive_rank: 0,
        });
    }

    // Remove professions.
    for prof_name in &opts.remove_profession {
        let before = character.professions.len();
        character.professions.retain(|p| p.definition_name != *prof_name);
        if character.professions.len() == before {
            return Err(format!("Profession '{}' not found on character", prof_name));
        }
    }

    // Add attribute points.
    for attr_str in &opts.add_attr {
        let (kind, points) = parse_attr_arg(attr_str)?;
        if character.unspent_attribute_points < points {
            return Err(format!("Not enough unspent points (have {}, need {})", character.unspent_attribute_points, points));
        }
        for _ in 0..points { character.attributes.add(kind, 1); }
        character.unspent_attribute_points -= points;
    }

    // Remove attribute points.
    for attr_str in &opts.remove_attr {
        let (kind, points) = parse_attr_arg(attr_str)?;
        let current = character.attributes.get(kind);
        if current - points < 1 {
            return Err(format!("Cannot reduce {} below 1", kind.name()));
        }
        // Subtract directly using match on kind.
        subtract_attribute(&mut character.attributes, kind, points);
        character.unspent_attribute_points += points;
    }
```

Add helper functions:

```rust
fn parse_attr_arg(s: &str) -> Result<(AttributeKind, u32), String> {
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() != 2 { return Err(format!("Invalid attr format '{}', expected kind:points", s)); }
    let kind = match parts[0] {
        "str" => AttributeKind::Strength,
        "agi" => AttributeKind::Agility,
        "end" => AttributeKind::Endurance,
        "int" => AttributeKind::Intelligence,
        "wis" => AttributeKind::Wisdom,
        "per" => AttributeKind::Perception,
        _ => return Err(format!("Unknown attribute '{}'", parts[0])),
    };
    let points: u32 = parts[1].parse().map_err(|_| "Invalid point value".to_string())?;
    Ok((kind, points))
}

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
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test test_update -- --nocapture`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/cli.rs
git commit -m "feat: implement CLI update skills, professions, attributes"
```

---

## Chunk 3: Skill and Profession Library Commands

### Task 6: Skill library — list, show, create, delete

**Files:**
- Modify: `src/cli.rs`

- [ ] **Step 1: Write tests for skill CRUD**

```rust
#[test]
fn test_skill_create_and_list() {
    let dir = test_data_dir();
    cmd_skill_create(&dir, "Fireball", "acquired", "active", "A fire spell",
        &["Fire Damage".to_string()], &[20.0], &[0], &["Burns".to_string()]).unwrap();
    let result = cmd_skill_list(&dir).unwrap();
    let json: Vec<serde_json::Value> = serde_json::from_str(&result).unwrap();
    assert_eq!(json.len(), 1);
    assert_eq!(json[0]["name"], "Fireball");
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
}

#[test]
fn test_skill_delete() {
    let dir = test_data_dir();
    cmd_skill_create(&dir, "Fireball", "acquired", "active", "", &[], &[], &[], &[]).unwrap();
    cmd_skill_delete(&dir, "Fireball").unwrap();
    let result = cmd_skill_list(&dir).unwrap();
    let json: Vec<serde_json::Value> = serde_json::from_str(&result).unwrap();
    assert_eq!(json.len(), 0);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test test_skill -- --nocapture`
Expected: FAIL

- [ ] **Step 3: Implement skill CRUD functions**

```rust
use crate::models::skill::{SkillCategory, SkillType, SkillEffect, RankDefinition, MasteryRank};

fn parse_category(s: &str) -> Result<SkillCategory, String> {
    match s.to_lowercase().as_str() {
        "acquired" => Ok(SkillCategory::Acquired),
        "innate" => Ok(SkillCategory::Innate),
        "profession" => Ok(SkillCategory::Profession),
        _ => Err(format!("Unknown category '{}' (use: acquired, innate, profession)", s)),
    }
}

fn parse_skill_type(s: &str) -> Result<SkillType, String> {
    match s.to_lowercase().as_str() {
        "active" => Ok(SkillType::Active),
        "passive" => Ok(SkillType::Passive),
        _ => Err(format!("Unknown type '{}' (use: active, passive)", s)),
    }
}

fn cmd_skill_list(data_dir: &Path) -> Result<String, String> {
    let path = data_dir.join("skills.json");
    let skills: Vec<SkillDefinition> = json_store::load_json(&path).unwrap_or_default();
    let summaries: Vec<serde_json::Value> = skills.iter().map(|s| {
        let effect_count = s.ranks.first().map(|r| r.effects.len()).unwrap_or(0);
        serde_json::json!({
            "name": s.name,
            "category": format!("{:?}", s.category),
            "type": format!("{:?}", s.skill_type),
            "effects": effect_count,
        })
    }).collect();
    serde_json::to_string_pretty(&summaries).map_err(|e| e.to_string())
}

fn cmd_skill_show(data_dir: &Path, name: &str) -> Result<String, String> {
    let path = data_dir.join("skills.json");
    let skills: Vec<SkillDefinition> = json_store::load_json(&path).unwrap_or_default();
    let skill = skills.iter().find(|s| s.name == name)
        .ok_or_else(|| format!("Skill '{}' not found", name))?;
    serde_json::to_string_pretty(skill).map_err(|e| e.to_string())
}

fn cmd_skill_create(
    data_dir: &Path, name: &str, category: &str, skill_type: &str, description: &str,
    effect_names: &[String], base_values: &[f64], unlock_levels: &[u32], effect_descs: &[String],
) -> Result<String, String> {
    let path = data_dir.join("skills.json");
    let mut skills: Vec<SkillDefinition> = json_store::load_json(&path).unwrap_or_default();
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

fn cmd_skill_delete(data_dir: &Path, name: &str) -> Result<String, String> {
    let path = data_dir.join("skills.json");
    let mut skills: Vec<SkillDefinition> = json_store::load_json(&path).unwrap_or_default();
    let before = skills.len();
    skills.retain(|s| s.name != name);
    if skills.len() == before { return Err(format!("Skill '{}' not found", name)); }
    json_store::save_json(&path, &skills)?;
    Ok(json_ok(&format!("Deleted skill '{}'", name)))
}
```

Wire into `run()` match under `Command::Skill { action }`.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test test_skill -- --nocapture`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/cli.rs
git commit -m "feat: implement CLI skill library list, show, create, delete"
```

---

### Task 7: Skill library — update (add/remove effects)

**Files:**
- Modify: `src/cli.rs`

- [ ] **Step 1: Write test for skill update**

```rust
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
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test test_skill_update -- --nocapture`
Expected: FAIL

- [ ] **Step 3: Implement `cmd_skill_update`**

```rust
fn cmd_skill_update(
    data_dir: &Path, name: &str,
    description: Option<&str>, category: Option<&str>, skill_type: Option<&str>,
    add_effect_names: &[String], base_values: &[f64], unlock_levels: &[u32], effect_descs: &[String],
    remove_effects: &[String],
) -> Result<String, String> {
    let path = data_dir.join("skills.json");
    let mut skills: Vec<SkillDefinition> = json_store::load_json(&path).unwrap_or_default();
    let skill = skills.iter_mut().find(|s| s.name == name)
        .ok_or_else(|| format!("Skill '{}' not found", name))?;

    if let Some(d) = description { skill.description = d.to_string(); }
    if let Some(c) = category { skill.category = parse_category(c)?; }
    if let Some(t) = skill_type { skill.skill_type = parse_skill_type(t)?; }

    // Ensure at least a Novice rank exists.
    if skill.ranks.is_empty() {
        skill.ranks.push(RankDefinition { rank: MasteryRank::Novice, description: String::new(), effects: vec![] });
    }
    let novice = &mut skill.ranks[0];

    // Remove effects.
    for ename in remove_effects {
        novice.effects.retain(|e| e.name.as_deref() != Some(ename.as_str()));
    }

    // Add effects.
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
```

Wire into the `SkillAction::Update` match arm.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test test_skill_update -- --nocapture`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/cli.rs
git commit -m "feat: implement CLI skill update with add/remove effects"
```

---

### Task 8: Profession library — full CRUD

**Files:**
- Modify: `src/cli.rs`

- [ ] **Step 1: Write tests for profession CRUD**

```rust
#[test]
fn test_profession_create_list_show() {
    let dir = test_data_dir();
    cmd_profession_create(&dir, "Blacksmith", "Craft weapons", "Forgeborn", "Crafting speed",
        &["Hammer Strike".to_string()]).unwrap();
    let list = cmd_profession_list(&dir).unwrap();
    let json: Vec<serde_json::Value> = serde_json::from_str(&list).unwrap();
    assert_eq!(json.len(), 1);
    assert_eq!(json[0]["name"], "Blacksmith");

    let show = cmd_profession_show(&dir, "Blacksmith").unwrap();
    let json: serde_json::Value = serde_json::from_str(&show).unwrap();
    assert_eq!(json["passive_name"], "Forgeborn");
}

#[test]
fn test_profession_update_add_remove_skill() {
    let dir = test_data_dir();
    cmd_profession_create(&dir, "Blacksmith", "", "", "", &["Hammer".to_string()]).unwrap();
    cmd_profession_update(&dir, "Blacksmith", None, None, None,
        &["Anvil".to_string()], &[]).unwrap();
    let show = cmd_profession_show(&dir, "Blacksmith").unwrap();
    let json: serde_json::Value = serde_json::from_str(&show).unwrap();
    assert_eq!(json["skills"].as_array().unwrap().len(), 2);

    cmd_profession_update(&dir, "Blacksmith", None, None, None,
        &[], &["Hammer".to_string()]).unwrap();
    let show = cmd_profession_show(&dir, "Blacksmith").unwrap();
    let json: serde_json::Value = serde_json::from_str(&show).unwrap();
    assert_eq!(json["skills"].as_array().unwrap().len(), 1);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test test_profession -- --nocapture`
Expected: FAIL

- [ ] **Step 3: Implement profession CRUD functions**

```rust
fn cmd_profession_list(data_dir: &Path) -> Result<String, String> {
    let path = data_dir.join("professions.json");
    let profs: Vec<ProfessionDefinition> = json_store::load_json(&path).unwrap_or_default();
    let summaries: Vec<serde_json::Value> = profs.iter().map(|p| {
        serde_json::json!({
            "name": p.name,
            "skills": p.skills,
            "passive": p.passive_name,
        })
    }).collect();
    serde_json::to_string_pretty(&summaries).map_err(|e| e.to_string())
}

fn cmd_profession_show(data_dir: &Path, name: &str) -> Result<String, String> {
    let path = data_dir.join("professions.json");
    let profs: Vec<ProfessionDefinition> = json_store::load_json(&path).unwrap_or_default();
    let prof = profs.iter().find(|p| p.name == name)
        .ok_or_else(|| format!("Profession '{}' not found", name))?;
    serde_json::to_string_pretty(prof).map_err(|e| e.to_string())
}

fn cmd_profession_create(
    data_dir: &Path, name: &str, description: &str,
    passive_name: &str, passive_desc: &str, skills: &[String],
) -> Result<String, String> {
    let path = data_dir.join("professions.json");
    let mut profs: Vec<ProfessionDefinition> = json_store::load_json(&path).unwrap_or_default();
    if profs.iter().any(|p| p.name == name) {
        return Err(format!("Profession '{}' already exists", name));
    }
    profs.push(ProfessionDefinition {
        name: name.to_string(),
        description: description.to_string(),
        skills: skills.to_vec(),
        passive_name: passive_name.to_string(),
        passive_description: passive_desc.to_string(),
    });
    json_store::save_json(&path, &profs)?;
    Ok(json_ok(&format!("Created profession '{}'", name)))
}

fn cmd_profession_delete(data_dir: &Path, name: &str) -> Result<String, String> {
    let path = data_dir.join("professions.json");
    let mut profs: Vec<ProfessionDefinition> = json_store::load_json(&path).unwrap_or_default();
    let before = profs.len();
    profs.retain(|p| p.name != name);
    if profs.len() == before { return Err(format!("Profession '{}' not found", name)); }
    json_store::save_json(&path, &profs)?;
    Ok(json_ok(&format!("Deleted profession '{}'", name)))
}

fn cmd_profession_update(
    data_dir: &Path, name: &str,
    description: Option<&str>, passive_name: Option<&str>, passive_desc: Option<&str>,
    add_skills: &[String], remove_skills: &[String],
) -> Result<String, String> {
    let path = data_dir.join("professions.json");
    let mut profs: Vec<ProfessionDefinition> = json_store::load_json(&path).unwrap_or_default();
    let prof = profs.iter_mut().find(|p| p.name == name)
        .ok_or_else(|| format!("Profession '{}' not found", name))?;

    if let Some(d) = description { prof.description = d.to_string(); }
    if let Some(n) = passive_name { prof.passive_name = n.to_string(); }
    if let Some(d) = passive_desc { prof.passive_description = d.to_string(); }
    for s in remove_skills { prof.skills.retain(|sk| sk != s); }
    for s in add_skills { prof.skills.push(s.clone()); }

    json_store::save_json(&path, &profs)?;
    Ok(json_ok(&format!("Updated profession '{}'", name)))
}
```

Wire all into the `Command::Profession { action }` match.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test test_profession -- --nocapture`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/cli.rs
git commit -m "feat: implement CLI profession library CRUD"
```

---

## Chunk 4: Integration and Final Wiring

### Task 9: Wire all commands in `run()` and verify full binary

**Files:**
- Modify: `src/cli.rs`
- Modify: `src/main.rs`

- [ ] **Step 1: Write integration test — full CLI round-trip**

```rust
#[test]
fn test_full_roundtrip() {
    let dir = test_data_dir();

    // Create a skill.
    cmd_skill_create(&dir, "Fireball", "acquired", "active", "Fire",
        &["Fire Damage".to_string()], &[20.0], &[0], &["Burns".to_string()]).unwrap();

    // Create a profession.
    cmd_profession_create(&dir, "Mage", "Casts spells", "Arcane Focus", "Mana regen",
        &["Fireball".to_string()]).unwrap();

    // Create a character.
    cmd_create(&dir, "Kael", 8, 5, 6, 3, 5, 7, None).unwrap();

    // Update: level up, add skill, add profession, distribute attrs, show.
    let result = cmd_update(&dir, "Kael", &UpdateOpts {
        level_up: Some(5),
        add_skill: vec!["Fireball".to_string()],
        add_profession: vec!["Mage".to_string()],
        add_attr: vec!["str:3".to_string()],
        show: true,
        ..Default::default()
    }).unwrap();

    let json: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(json["name"], "Kael");
    assert_eq!(json["level"], 5);
    assert_eq!(json["skills"].as_array().unwrap().len(), 1);
    assert_eq!(json["professions"].as_array().unwrap().len(), 1);
    assert_eq!(json["attributes"]["strength"], 11); // 8 + 3
}
```

- [ ] **Step 2: Run test to verify it passes**

Run: `cargo test test_full_roundtrip -- --nocapture`
Expected: PASS (all pieces already implemented)

- [ ] **Step 3: Ensure all `todo!()` calls in `run()` are replaced**

Review the `run()` function and verify every `Command` variant dispatches to its handler. Remove any remaining `todo!()` calls.

- [ ] **Step 4: Build the full binary and test manually**

Run: `cargo build`
Then test:
```bash
./target/debug/LITRPG_System skill create "Fireball" --category acquired --type active --description "Fire" --effect "Damage" --base-value 20 --unlock-level 0 --effect-desc "Burns"
./target/debug/LITRPG_System skill list
./target/debug/LITRPG_System create "Kael" --str 8 --agi 5
./target/debug/LITRPG_System update "Kael" --level-up 5 --add-skill "Fireball" --show
./target/debug/LITRPG_System list
```

- [ ] **Step 5: Run full test suite**

Run: `cargo test`
Expected: All tests pass, no warnings about unused code in `cli.rs`.

- [ ] **Step 6: Commit**

```bash
git add src/cli.rs src/main.rs
git commit -m "feat: wire all CLI commands and add integration test"
```

---

### Task 10: Verify TUI still works

**Files:** None (manual verification)

- [ ] **Step 1: Run the binary with no arguments**

Run: `cargo run`
Expected: TUI launches normally — tab navigation, character creation, skill library all work as before.

- [ ] **Step 2: Verify no regressions from clap integration**

Confirm that clap does not consume or interfere with TUI key inputs. The TUI should behave identically to before.

- [ ] **Step 3: Final commit if any cleanup needed**

```bash
git add -A
git commit -m "chore: verify TUI works alongside CLI mode"
```
