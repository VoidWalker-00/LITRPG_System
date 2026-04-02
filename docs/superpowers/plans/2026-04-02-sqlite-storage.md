# SQLite Storage Migration Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace JSON file storage with SQLite, rename "profession" to "class" with attribute bonuses, remove tree system, and enable live data sharing between TUI and CLI.

**Architecture:** A `Database` struct wraps a `rusqlite::Connection` to a single SQLite file at the XDG data directory. Both TUI and CLI open the same database. TUI queries on each render instead of caching in memory, eliminating the DataReloader.

**Tech Stack:** Rust, rusqlite (bundled), dirs crate, existing serde for CLI JSON output.

**Spec:** `docs/superpowers/specs/2026-04-02-sqlite-storage-design.md`

---

## File Structure

| File | Action | Responsibility |
|------|--------|---------------|
| `Cargo.toml` | Modify | Add `rusqlite` and `dirs` dependencies |
| `src/storage/mod.rs` | Modify | Replace `json_store` with `db` module |
| `src/storage/db.rs` | Create | `Database` struct with all CRUD methods |
| `src/storage/json_store.rs` | Delete | Replaced by db.rs |
| `src/models/mod.rs` | Modify | Replace `profession` with `class`, remove `tree` |
| `src/models/class.rs` | Create | `ClassDefinition` with attribute bonuses |
| `src/models/profession.rs` | Delete | Replaced by class.rs |
| `src/models/tree.rs` | Delete | Tree system removed |
| `src/models/character.rs` | Modify | Rename profession fields to class, remove tree |
| `src/ui/mod.rs` | Modify | Replace `profession_library`/`tree_library` with `class_library` |
| `src/ui/app.rs` | Modify | Remove DataReloader, remove in-memory caches, remove tree_library |
| `src/ui/class_library.rs` | Create | Renamed/rewritten from profession_library.rs |
| `src/ui/profession_library.rs` | Delete | Replaced by class_library.rs |
| `src/ui/tree_library.rs` | Delete | Tree system removed |
| `src/ui/system_panel.rs` | Modify | profession -> class rename, query DB |
| `src/ui/skill_library.rs` | Modify | Use DB instead of json_store |
| `src/ui/character_creation.rs` | Modify | Use DB instead of json_store |
| `src/main.rs` | Modify | Open Database, remove reloader, remove tree/json loading |
| `src/cli.rs` | Modify | Use `&Database`, profession -> class rename |

---

## Chunk 1: Foundation — Dependencies, Models, and Database Schema

### Task 1: Add dependencies to Cargo.toml

**Files:**
- Modify: `Cargo.toml`

- [ ] **Step 1: Add rusqlite and dirs dependencies**

Add to `[dependencies]`:

```toml
rusqlite = { version = "0.31", features = ["bundled"] }
dirs = "5"
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo check`
Expected: PASS (new deps downloaded and compiled)

- [ ] **Step 3: Commit**

```bash
git add Cargo.toml Cargo.lock
git commit -m "chore: add rusqlite and dirs dependencies"
```

---

### Task 2: Create ClassDefinition model

**Files:**
- Create: `src/models/class.rs`
- Modify: `src/models/mod.rs`

- [ ] **Step 1: Write test for ClassDefinition**

Create `src/models/class.rs` with a test:

```rust
use serde::{Deserialize, Serialize};
use crate::models::attribute::AttributeKind;

/// A class definition from the library (e.g. Warrior, Mage).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassDefinition {
    pub name: String,
    pub description: String,
    pub skills: Vec<String>,
    pub passive_name: String,
    pub passive_description: String,
    /// Attribute points granted per class level, e.g. [(Strength, 3), (Endurance, 2)].
    pub attribute_bonuses: Vec<(AttributeKind, u32)>,
}

/// A class held by a character.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterClass {
    pub definition_name: String,
    pub level: u32,
    pub passive_rank: u32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::attribute::AttributeKind;

    #[test]
    fn test_class_definition_creation() {
        let class = ClassDefinition {
            name: "Warrior".to_string(),
            description: "Melee combatant".to_string(),
            skills: vec!["Sword Mastery".to_string()],
            passive_name: "Battle Hardened".to_string(),
            passive_description: "Phys resist".to_string(),
            attribute_bonuses: vec![
                (AttributeKind::Strength, 3),
                (AttributeKind::Endurance, 2),
            ],
        };
        assert_eq!(class.name, "Warrior");
        assert_eq!(class.attribute_bonuses.len(), 2);
        assert_eq!(class.attribute_bonuses[0].1, 3);
    }

    #[test]
    fn test_character_class_creation() {
        let cc = CharacterClass {
            definition_name: "Warrior".to_string(),
            level: 5,
            passive_rank: 2,
        };
        assert_eq!(cc.definition_name, "Warrior");
        assert_eq!(cc.level, 5);
    }
}
```

- [ ] **Step 2: Check that AttributeKind supports Serialize/Deserialize**

`AttributeKind` in `src/models/attribute.rs` needs `#[derive(Serialize, Deserialize)]` added to its derive list if not already present. Check and add if needed.

- [ ] **Step 3: Add class module to models/mod.rs**

In `src/models/mod.rs`, add `pub mod class;` and remove `pub mod tree;` and `pub mod profession;`:

```rust
pub mod attribute;
pub mod skill;
pub mod innate_skill;
pub mod grade;
pub mod class;
pub mod character;
```

- [ ] **Step 4: Run test to verify**

Run: `cargo test test_class_definition_creation test_character_class -- --nocapture`
Expected: PASS

Note: This will cause compile errors in files still referencing `profession` and `tree` modules. Those are fixed in subsequent tasks.

- [ ] **Step 5: Commit**

```bash
git add src/models/class.rs src/models/mod.rs src/models/attribute.rs
git commit -m "feat: add ClassDefinition model with attribute bonuses"
```

---

### Task 3: Update Character model — profession to class, remove tree

**Files:**
- Modify: `src/models/character.rs`

- [ ] **Step 1: Replace profession with class in Character struct**

Replace the imports and struct. Remove `CharacterTree`. Change `professions` to `classes`, `profession_slots` to `class_slots`:

```rust
use serde::{Deserialize, Serialize};
use crate::models::attribute::Attributes;
use crate::models::class::CharacterClass;
use crate::models::grade::Grade;
use crate::models::skill::MasteryRank;

/// Base attribute points each level grants before bonuses.
const BASE_ATTRIBUTE_POINTS_PER_LEVEL: u32 = 3;

/// A skill slot on a character.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterSkill {
    pub definition_name: String,
    pub rank: MasteryRank,
    pub level: u32,
}

/// An innate skill bound to a character at creation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterInnateSkill {
    pub definition_name: String,
    pub level: u32,
}

/// Full character state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Character {
    pub name: String,
    pub grade: Grade,
    pub race: String,
    pub level: u32,
    pub xp: f64,
    pub attributes: Attributes,
    pub unspent_attribute_points: u32,
    pub skills: Vec<CharacterSkill>,
    pub innate_skill: Option<CharacterInnateSkill>,
    pub classes: Vec<CharacterClass>,
    pub class_slots: u32,
    pub bonus_attribute_points_per_level: u32,
}

impl Character {
    /// Create a new Grade-G, Level-0 character.
    pub fn new(
        name: String,
        attributes: Attributes,
        innate_skill: Option<CharacterInnateSkill>,
    ) -> Self {
        Self {
            name,
            grade: Grade::G,
            race: "Human".to_string(),
            level: 0,
            xp: 0.0,
            attributes,
            unspent_attribute_points: 0,
            skills: Vec::new(),
            innate_skill,
            classes: Vec::new(),
            class_slots: 1,
            bonus_attribute_points_per_level: 0,
        }
    }

    /// Attribute points earned per level-up: (base + bonus) * 2^grade.
    pub fn attribute_points_per_level(&self) -> u32 {
        (BASE_ATTRIBUTE_POINTS_PER_LEVEL + self.bonus_attribute_points_per_level)
            * 2u32.pow(self.grade.numeric())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::attribute::Attributes;

    #[test]
    fn test_new_character() {
        let attrs = Attributes::new_clamped(5, 5, 5, 5, 5, 5);
        let c = Character::new("Test".to_string(), attrs, None);
        assert_eq!(c.name, "Test");
        assert_eq!(c.level, 0);
        assert_eq!(c.class_slots, 1);
        assert!(c.classes.is_empty());
        assert_eq!(c.attribute_points_per_level(), 3);
    }
}
```

- [ ] **Step 2: Run test to verify**

Run: `cargo test test_new_character -- --nocapture`
Expected: PASS

- [ ] **Step 3: Commit**

```bash
git add src/models/character.rs
git commit -m "feat: rename profession to class in Character, remove tree"
```

---

### Task 4: Create Database struct with schema initialization

**Files:**
- Create: `src/storage/db.rs`
- Modify: `src/storage/mod.rs`

- [ ] **Step 1: Write test for database creation**

Create `src/storage/db.rs`:

```rust
use std::path::PathBuf;
use rusqlite::Connection;

/// Wraps a SQLite connection to the LITRPG data store.
pub struct Database {
    pub conn: Connection,
}

impl Database {
    /// Open (or create) the database at the XDG data directory.
    pub fn open() -> Result<Self, String> {
        let data_dir = dirs::data_dir()
            .ok_or("Could not determine data directory")?
            .join("litrpg");
        std::fs::create_dir_all(&data_dir)
            .map_err(|e| format!("Failed to create data dir: {}", e))?;
        let db_path = data_dir.join("data.db");
        Self::open_at(db_path)
    }

    /// Open a database at a specific path (used for testing).
    pub fn open_at(path: PathBuf) -> Result<Self, String> {
        let conn = Connection::open(&path)
            .map_err(|e| format!("Failed to open database: {}", e))?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")
            .map_err(|e| format!("Failed to set pragmas: {}", e))?;
        let db = Self { conn };
        db.create_tables()?;
        Ok(db)
    }

    /// Open an in-memory database (for tests).
    pub fn open_memory() -> Result<Self, String> {
        let conn = Connection::open_in_memory()
            .map_err(|e| format!("Failed to open in-memory database: {}", e))?;
        conn.execute_batch("PRAGMA foreign_keys=ON;")
            .map_err(|e| format!("Failed to set pragmas: {}", e))?;
        let db = Self { conn };
        db.create_tables()?;
        Ok(db)
    }

    /// Create all tables if they don't exist.
    fn create_tables(&self) -> Result<(), String> {
        self.conn.execute_batch("
            CREATE TABLE IF NOT EXISTS characters (
                name TEXT PRIMARY KEY,
                race TEXT NOT NULL DEFAULT 'Human',
                grade INTEGER NOT NULL DEFAULT 0,
                level INTEGER NOT NULL DEFAULT 0,
                xp REAL NOT NULL DEFAULT 0.0,
                strength INTEGER NOT NULL,
                agility INTEGER NOT NULL,
                endurance INTEGER NOT NULL,
                intelligence INTEGER NOT NULL,
                wisdom INTEGER NOT NULL,
                perception INTEGER NOT NULL,
                unspent_attribute_points INTEGER NOT NULL DEFAULT 0,
                class_slots INTEGER NOT NULL DEFAULT 1,
                bonus_attribute_points_per_level INTEGER NOT NULL DEFAULT 0
            );

            CREATE TABLE IF NOT EXISTS character_skills (
                character_name TEXT REFERENCES characters(name) ON DELETE CASCADE,
                skill_name TEXT NOT NULL,
                rank TEXT NOT NULL DEFAULT 'Novice',
                level INTEGER NOT NULL DEFAULT 0,
                PRIMARY KEY (character_name, skill_name)
            );

            CREATE TABLE IF NOT EXISTS character_innate_skills (
                character_name TEXT PRIMARY KEY REFERENCES characters(name) ON DELETE CASCADE,
                skill_name TEXT NOT NULL,
                level INTEGER NOT NULL DEFAULT 0
            );

            CREATE TABLE IF NOT EXISTS character_classes (
                character_name TEXT REFERENCES characters(name) ON DELETE CASCADE,
                class_name TEXT NOT NULL,
                level INTEGER NOT NULL DEFAULT 0,
                passive_rank INTEGER NOT NULL DEFAULT 0,
                PRIMARY KEY (character_name, class_name)
            );

            CREATE TABLE IF NOT EXISTS skill_definitions (
                name TEXT PRIMARY KEY,
                category TEXT NOT NULL,
                skill_type TEXT NOT NULL,
                description TEXT NOT NULL DEFAULT ''
            );

            CREATE TABLE IF NOT EXISTS skill_effects (
                skill_name TEXT REFERENCES skill_definitions(name) ON DELETE CASCADE,
                rank TEXT NOT NULL,
                effect_name TEXT,
                description TEXT NOT NULL DEFAULT '',
                base_value REAL NOT NULL DEFAULT 0.0,
                unlock_level INTEGER NOT NULL DEFAULT 0
            );

            CREATE TABLE IF NOT EXISTS class_definitions (
                name TEXT PRIMARY KEY,
                description TEXT NOT NULL DEFAULT '',
                passive_name TEXT NOT NULL DEFAULT '',
                passive_description TEXT NOT NULL DEFAULT ''
            );

            CREATE TABLE IF NOT EXISTS class_skills (
                class_name TEXT REFERENCES class_definitions(name) ON DELETE CASCADE,
                skill_name TEXT NOT NULL,
                PRIMARY KEY (class_name, skill_name)
            );

            CREATE TABLE IF NOT EXISTS class_attribute_bonuses (
                class_name TEXT REFERENCES class_definitions(name) ON DELETE CASCADE,
                attribute TEXT NOT NULL,
                points_per_level INTEGER NOT NULL,
                PRIMARY KEY (class_name, attribute)
            );
        ").map_err(|e| format!("Failed to create tables: {}", e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_open_memory() {
        let db = Database::open_memory().unwrap();
        // Verify tables exist by querying them.
        let count: i64 = db.conn
            .query_row("SELECT COUNT(*) FROM characters", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 0);
    }
}
```

- [ ] **Step 2: Update storage/mod.rs**

Replace contents of `src/storage/mod.rs`:

```rust
pub mod db;
```

Note: Remove `pub mod json_store;`. This will cause compile errors in files still referencing `json_store`. Those are fixed in later tasks.

- [ ] **Step 3: Run test to verify**

Run: `cargo test test_open_memory -- --nocapture`
Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add src/storage/db.rs src/storage/mod.rs
git commit -m "feat: add Database struct with SQLite schema initialization"
```

---

## Chunk 2: Database CRUD — Characters

### Task 5: Character CRUD methods on Database

**Files:**
- Modify: `src/storage/db.rs`

- [ ] **Step 1: Write tests for character CRUD**

Add to the `tests` module in `db.rs`:

```rust
    use crate::models::attribute::Attributes;
    use crate::models::character::Character;

    #[test]
    fn test_save_and_load_character() {
        let db = Database::open_memory().unwrap();
        let attrs = Attributes::new_clamped(8, 6, 5, 4, 3, 2);
        let c = Character::new("Kael".to_string(), attrs, None);
        db.save_character(&c).unwrap();

        let loaded = db.load_character("Kael").unwrap();
        assert_eq!(loaded.name, "Kael");
        assert_eq!(loaded.attributes.strength, 8);
        assert_eq!(loaded.attributes.perception, 2);
        assert_eq!(loaded.level, 0);
        assert!(loaded.classes.is_empty());
    }

    #[test]
    fn test_list_characters() {
        let db = Database::open_memory().unwrap();
        let attrs = Attributes::new_clamped(5, 5, 5, 5, 5, 5);
        db.save_character(&Character::new("Alpha".to_string(), attrs.clone(), None)).unwrap();
        db.save_character(&Character::new("Beta".to_string(), attrs, None)).unwrap();

        let names = db.list_characters().unwrap();
        assert_eq!(names.len(), 2);
        assert!(names.contains(&"Alpha".to_string()));
        assert!(names.contains(&"Beta".to_string()));
    }

    #[test]
    fn test_delete_character() {
        let db = Database::open_memory().unwrap();
        let attrs = Attributes::new_clamped(5, 5, 5, 5, 5, 5);
        db.save_character(&Character::new("Kael".to_string(), attrs, None)).unwrap();
        db.delete_character("Kael").unwrap();
        assert!(db.load_character("Kael").is_err());
    }

    #[test]
    fn test_save_character_with_skills_and_classes() {
        let db = Database::open_memory().unwrap();
        let attrs = Attributes::new_clamped(5, 5, 5, 5, 5, 5);
        let mut c = Character::new("Kael".to_string(), attrs, None);
        c.skills.push(crate::models::character::CharacterSkill {
            definition_name: "Fireball".to_string(),
            rank: crate::models::skill::MasteryRank::Novice,
            level: 3,
        });
        c.classes.push(crate::models::class::CharacterClass {
            definition_name: "Mage".to_string(),
            level: 2,
            passive_rank: 1,
        });
        db.save_character(&c).unwrap();

        let loaded = db.load_character("Kael").unwrap();
        assert_eq!(loaded.skills.len(), 1);
        assert_eq!(loaded.skills[0].definition_name, "Fireball");
        assert_eq!(loaded.skills[0].level, 3);
        assert_eq!(loaded.classes.len(), 1);
        assert_eq!(loaded.classes[0].definition_name, "Mage");
        assert_eq!(loaded.classes[0].level, 2);
    }

    #[test]
    fn test_save_character_upsert() {
        let db = Database::open_memory().unwrap();
        let attrs = Attributes::new_clamped(5, 5, 5, 5, 5, 5);
        let mut c = Character::new("Kael".to_string(), attrs, None);
        db.save_character(&c).unwrap();

        c.level = 10;
        c.attributes.strength = 20;
        db.save_character(&c).unwrap();

        let loaded = db.load_character("Kael").unwrap();
        assert_eq!(loaded.level, 10);
        assert_eq!(loaded.attributes.strength, 20);
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test test_save_and_load_character test_list_characters test_delete_character test_save_character_with test_save_character_upsert -- --nocapture`
Expected: FAIL — methods don't exist yet.

- [ ] **Step 3: Implement character CRUD methods**

Add these imports and methods to `Database` in `db.rs`:

```rust
use crate::models::attribute::Attributes;
use crate::models::character::{Character, CharacterInnateSkill, CharacterSkill};
use crate::models::class::CharacterClass;
use crate::models::grade::Grade;
use crate::models::skill::MasteryRank;

impl Database {
    // ... existing open/create_tables methods ...

    /// List all character names.
    pub fn list_characters(&self) -> Result<Vec<String>, String> {
        let mut stmt = self.conn.prepare("SELECT name FROM characters ORDER BY name")
            .map_err(|e| e.to_string())?;
        let names = stmt.query_map([], |row| row.get(0))
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<String>, _>>()
            .map_err(|e| e.to_string())?;
        Ok(names)
    }

    /// Load a full character with skills, innate skill, and classes.
    pub fn load_character(&self, name: &str) -> Result<Character, String> {
        let row = self.conn.query_row(
            "SELECT name, race, grade, level, xp, strength, agility, endurance,
                    intelligence, wisdom, perception, unspent_attribute_points,
                    class_slots, bonus_attribute_points_per_level
             FROM characters WHERE name = ?1",
            [name],
            |row| {
                Ok(Character {
                    name: row.get(0)?,
                    race: row.get(1)?,
                    grade: Grade::from_numeric(row.get::<_, u32>(2)?),
                    level: row.get(3)?,
                    xp: row.get(4)?,
                    attributes: Attributes {
                        strength: row.get(5)?,
                        agility: row.get(6)?,
                        endurance: row.get(7)?,
                        intelligence: row.get(8)?,
                        wisdom: row.get(9)?,
                        perception: row.get(10)?,
                    },
                    unspent_attribute_points: row.get(11)?,
                    skills: Vec::new(),
                    innate_skill: None,
                    classes: Vec::new(),
                    class_slots: row.get(12)?,
                    bonus_attribute_points_per_level: row.get(13)?,
                })
            },
        ).map_err(|_| format!("Character '{}' not found", name))?;

        let mut character = row;

        // Load skills.
        let mut stmt = self.conn.prepare(
            "SELECT skill_name, rank, level FROM character_skills WHERE character_name = ?1"
        ).map_err(|e| e.to_string())?;
        character.skills = stmt.query_map([name], |row| {
            Ok(CharacterSkill {
                definition_name: row.get(0)?,
                rank: parse_mastery_rank(&row.get::<_, String>(1)?),
                level: row.get(2)?,
            })
        }).map_err(|e| e.to_string())?
          .collect::<Result<Vec<_>, _>>()
          .map_err(|e| e.to_string())?;

        // Load innate skill.
        character.innate_skill = self.conn.query_row(
            "SELECT skill_name, level FROM character_innate_skills WHERE character_name = ?1",
            [name],
            |row| Ok(CharacterInnateSkill {
                definition_name: row.get(0)?,
                level: row.get(1)?,
            }),
        ).ok();

        // Load classes.
        let mut stmt = self.conn.prepare(
            "SELECT class_name, level, passive_rank FROM character_classes WHERE character_name = ?1"
        ).map_err(|e| e.to_string())?;
        character.classes = stmt.query_map([name], |row| {
            Ok(CharacterClass {
                definition_name: row.get(0)?,
                level: row.get(1)?,
                passive_rank: row.get(2)?,
            })
        }).map_err(|e| e.to_string())?
          .collect::<Result<Vec<_>, _>>()
          .map_err(|e| e.to_string())?;

        Ok(character)
    }

    /// Save (upsert) a character and all related data in a single transaction.
    pub fn save_character(&self, c: &Character) -> Result<(), String> {
        self.conn.execute_batch("BEGIN").map_err(|e| e.to_string())?;

        self.conn.execute(
            "INSERT OR REPLACE INTO characters
             (name, race, grade, level, xp, strength, agility, endurance,
              intelligence, wisdom, perception, unspent_attribute_points,
              class_slots, bonus_attribute_points_per_level)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14)",
            rusqlite::params![
                c.name, c.race, c.grade.numeric(), c.level, c.xp,
                c.attributes.strength, c.attributes.agility, c.attributes.endurance,
                c.attributes.intelligence, c.attributes.wisdom, c.attributes.perception,
                c.unspent_attribute_points, c.class_slots, c.bonus_attribute_points_per_level,
            ],
        ).map_err(|e| { let _ = self.conn.execute_batch("ROLLBACK"); e.to_string() })?;

        // Replace skills.
        self.conn.execute("DELETE FROM character_skills WHERE character_name = ?1", [&c.name])
            .map_err(|e| { let _ = self.conn.execute_batch("ROLLBACK"); e.to_string() })?;
        for s in &c.skills {
            self.conn.execute(
                "INSERT INTO character_skills (character_name, skill_name, rank, level) VALUES (?1,?2,?3,?4)",
                rusqlite::params![c.name, s.definition_name, s.rank.name(), s.level],
            ).map_err(|e| { let _ = self.conn.execute_batch("ROLLBACK"); e.to_string() })?;
        }

        // Replace innate skill.
        self.conn.execute("DELETE FROM character_innate_skills WHERE character_name = ?1", [&c.name])
            .map_err(|e| { let _ = self.conn.execute_batch("ROLLBACK"); e.to_string() })?;
        if let Some(is) = &c.innate_skill {
            self.conn.execute(
                "INSERT INTO character_innate_skills (character_name, skill_name, level) VALUES (?1,?2,?3)",
                rusqlite::params![c.name, is.definition_name, is.level],
            ).map_err(|e| { let _ = self.conn.execute_batch("ROLLBACK"); e.to_string() })?;
        }

        // Replace classes.
        self.conn.execute("DELETE FROM character_classes WHERE character_name = ?1", [&c.name])
            .map_err(|e| { let _ = self.conn.execute_batch("ROLLBACK"); e.to_string() })?;
        for cl in &c.classes {
            self.conn.execute(
                "INSERT INTO character_classes (character_name, class_name, level, passive_rank) VALUES (?1,?2,?3,?4)",
                rusqlite::params![c.name, cl.definition_name, cl.level, cl.passive_rank],
            ).map_err(|e| { let _ = self.conn.execute_batch("ROLLBACK"); e.to_string() })?;
        }

        self.conn.execute_batch("COMMIT").map_err(|e| e.to_string())
    }

    /// Delete a character and all related data (CASCADE handles children).
    pub fn delete_character(&self, name: &str) -> Result<(), String> {
        let affected = self.conn.execute("DELETE FROM characters WHERE name = ?1", [name])
            .map_err(|e| e.to_string())?;
        if affected == 0 {
            return Err(format!("Character '{}' not found", name));
        }
        Ok(())
    }
}

/// Parse a MasteryRank from its string name.
fn parse_mastery_rank(s: &str) -> MasteryRank {
    match s {
        "Novice" => MasteryRank::Novice,
        "Apprentice" => MasteryRank::Apprentice,
        "Journeyman" => MasteryRank::Journeyman,
        "Advanced" => MasteryRank::Advanced,
        "Expert" => MasteryRank::Expert,
        "Master" => MasteryRank::Master,
        "Grandmaster" => MasteryRank::Grandmaster,
        _ => MasteryRank::Novice,
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test test_save_and_load test_list_characters test_delete_character test_save_character -- --nocapture`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/storage/db.rs
git commit -m "feat: implement character CRUD on Database"
```

---

## Chunk 3: Database CRUD — Skills and Classes

### Task 6: Skill library CRUD methods on Database

**Files:**
- Modify: `src/storage/db.rs`

- [ ] **Step 1: Write tests for skill CRUD**

Add to the `tests` module:

```rust
    use crate::models::skill::{SkillDefinition, SkillCategory, SkillType, RankDefinition, SkillEffect, MasteryRank};

    #[test]
    fn test_skill_save_and_load() {
        let db = Database::open_memory().unwrap();
        let skill = SkillDefinition {
            name: "Fireball".to_string(),
            category: SkillCategory::Acquired,
            skill_type: SkillType::Active,
            description: "Fire spell".to_string(),
            ranks: vec![RankDefinition {
                rank: MasteryRank::Novice,
                description: String::new(),
                effects: vec![SkillEffect {
                    name: Some("Fire Damage".to_string()),
                    description: "Burns".to_string(),
                    base_value: 20.0,
                    unlock_level: 0,
                }],
            }],
        };
        db.save_skill(&skill).unwrap();

        let loaded = db.load_skill("Fireball").unwrap();
        assert_eq!(loaded.name, "Fireball");
        assert_eq!(loaded.ranks[0].effects[0].base_value, 20.0);
    }

    #[test]
    fn test_skill_list_and_delete() {
        let db = Database::open_memory().unwrap();
        let skill = SkillDefinition {
            name: "Fireball".to_string(),
            category: SkillCategory::Acquired,
            skill_type: SkillType::Active,
            description: String::new(),
            ranks: vec![],
        };
        db.save_skill(&skill).unwrap();
        assert_eq!(db.list_skills().unwrap().len(), 1);

        db.delete_skill("Fireball").unwrap();
        assert_eq!(db.list_skills().unwrap().len(), 0);
    }
```

- [ ] **Step 2: Implement skill CRUD methods**

Add imports and methods to `Database`:

```rust
use crate::models::skill::{SkillDefinition, SkillCategory, SkillType, RankDefinition, SkillEffect};

impl Database {
    /// List all skill definitions.
    pub fn list_skills(&self) -> Result<Vec<SkillDefinition>, String> {
        let mut stmt = self.conn.prepare("SELECT name FROM skill_definitions ORDER BY name")
            .map_err(|e| e.to_string())?;
        let names: Vec<String> = stmt.query_map([], |row| row.get(0))
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;
        names.iter().map(|n| self.load_skill(n)).collect()
    }

    /// Load a full skill definition with all ranks and effects.
    pub fn load_skill(&self, name: &str) -> Result<SkillDefinition, String> {
        let (category_str, type_str, description) = self.conn.query_row(
            "SELECT category, skill_type, description FROM skill_definitions WHERE name = ?1",
            [name],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?)),
        ).map_err(|_| format!("Skill '{}' not found", name))?;

        // Load effects grouped by rank.
        let mut stmt = self.conn.prepare(
            "SELECT rank, effect_name, description, base_value, unlock_level
             FROM skill_effects WHERE skill_name = ?1 ORDER BY rank"
        ).map_err(|e| e.to_string())?;
        let effects: Vec<(String, SkillEffect)> = stmt.query_map([name], |row| {
            Ok((row.get::<_, String>(0)?, SkillEffect {
                name: row.get(1)?,
                description: row.get(2)?,
                base_value: row.get(3)?,
                unlock_level: row.get(4)?,
            }))
        }).map_err(|e| e.to_string())?
          .collect::<Result<Vec<_>, _>>()
          .map_err(|e| e.to_string())?;

        // Group effects by rank.
        let mut ranks: Vec<RankDefinition> = Vec::new();
        for (rank_str, effect) in effects {
            let rank = parse_mastery_rank(&rank_str);
            if let Some(rd) = ranks.iter_mut().find(|r| r.rank == rank) {
                rd.effects.push(effect);
            } else {
                ranks.push(RankDefinition {
                    rank,
                    description: String::new(),
                    effects: vec![effect],
                });
            }
        }

        Ok(SkillDefinition {
            name: name.to_string(),
            category: parse_skill_category(&category_str),
            skill_type: parse_skill_type(&type_str),
            description,
            ranks,
        })
    }

    /// Save (upsert) a skill definition with all ranks and effects.
    pub fn save_skill(&self, skill: &SkillDefinition) -> Result<(), String> {
        self.conn.execute_batch("BEGIN").map_err(|e| e.to_string())?;

        self.conn.execute(
            "INSERT OR REPLACE INTO skill_definitions (name, category, skill_type, description)
             VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![
                skill.name, format!("{:?}", skill.category),
                format!("{:?}", skill.skill_type), skill.description
            ],
        ).map_err(|e| { let _ = self.conn.execute_batch("ROLLBACK"); e.to_string() })?;

        self.conn.execute("DELETE FROM skill_effects WHERE skill_name = ?1", [&skill.name])
            .map_err(|e| { let _ = self.conn.execute_batch("ROLLBACK"); e.to_string() })?;

        for rank_def in &skill.ranks {
            for effect in &rank_def.effects {
                self.conn.execute(
                    "INSERT INTO skill_effects (skill_name, rank, effect_name, description, base_value, unlock_level)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    rusqlite::params![
                        skill.name, rank_def.rank.name(), effect.name,
                        effect.description, effect.base_value, effect.unlock_level
                    ],
                ).map_err(|e| { let _ = self.conn.execute_batch("ROLLBACK"); e.to_string() })?;
            }
        }

        self.conn.execute_batch("COMMIT").map_err(|e| e.to_string())
    }

    /// Delete a skill definition (CASCADE removes effects).
    pub fn delete_skill(&self, name: &str) -> Result<(), String> {
        let affected = self.conn.execute("DELETE FROM skill_definitions WHERE name = ?1", [name])
            .map_err(|e| e.to_string())?;
        if affected == 0 {
            return Err(format!("Skill '{}' not found", name));
        }
        Ok(())
    }
}

fn parse_skill_category(s: &str) -> SkillCategory {
    match s {
        "Acquired" => SkillCategory::Acquired,
        "Innate" => SkillCategory::Innate,
        "Profession" | "Class" => SkillCategory::Profession,
        _ => SkillCategory::Acquired,
    }
}

fn parse_skill_type(s: &str) -> SkillType {
    match s {
        "Active" => SkillType::Active,
        "Passive" => SkillType::Passive,
        _ => SkillType::Active,
    }
}
```

Note: `MasteryRank` needs `PartialEq` derived — check `src/models/skill.rs` and add if missing.

- [ ] **Step 3: Run tests to verify**

Run: `cargo test test_skill_save test_skill_list -- --nocapture`
Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add src/storage/db.rs
git commit -m "feat: implement skill library CRUD on Database"
```

---

### Task 7: Class library CRUD methods on Database

**Files:**
- Modify: `src/storage/db.rs`

- [ ] **Step 1: Write tests for class CRUD**

Add to the `tests` module:

```rust
    use crate::models::class::ClassDefinition;
    use crate::models::attribute::AttributeKind;

    #[test]
    fn test_class_save_and_load() {
        let db = Database::open_memory().unwrap();
        let class = ClassDefinition {
            name: "Warrior".to_string(),
            description: "Melee fighter".to_string(),
            skills: vec!["Sword Mastery".to_string(), "Axe Mastery".to_string()],
            passive_name: "Battle Hardened".to_string(),
            passive_description: "Phys resist".to_string(),
            attribute_bonuses: vec![
                (AttributeKind::Strength, 3),
                (AttributeKind::Endurance, 2),
            ],
        };
        db.save_class(&class).unwrap();

        let loaded = db.load_class("Warrior").unwrap();
        assert_eq!(loaded.name, "Warrior");
        assert_eq!(loaded.skills.len(), 2);
        assert_eq!(loaded.attribute_bonuses.len(), 2);
        assert_eq!(loaded.attribute_bonuses[0], (AttributeKind::Strength, 3));
    }

    #[test]
    fn test_class_list_and_delete() {
        let db = Database::open_memory().unwrap();
        let class = ClassDefinition {
            name: "Mage".to_string(),
            description: String::new(),
            skills: vec![],
            passive_name: String::new(),
            passive_description: String::new(),
            attribute_bonuses: vec![],
        };
        db.save_class(&class).unwrap();
        assert_eq!(db.list_classes().unwrap().len(), 1);

        db.delete_class("Mage").unwrap();
        assert_eq!(db.list_classes().unwrap().len(), 0);
    }
```

- [ ] **Step 2: Implement class CRUD methods**

Add methods to `Database`:

```rust
use crate::models::class::ClassDefinition;
use crate::models::attribute::AttributeKind;

impl Database {
    /// List all class definitions.
    pub fn list_classes(&self) -> Result<Vec<ClassDefinition>, String> {
        let mut stmt = self.conn.prepare("SELECT name FROM class_definitions ORDER BY name")
            .map_err(|e| e.to_string())?;
        let names: Vec<String> = stmt.query_map([], |row| row.get(0))
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;
        names.iter().map(|n| self.load_class(n)).collect()
    }

    /// Load a full class definition with skills and attribute bonuses.
    pub fn load_class(&self, name: &str) -> Result<ClassDefinition, String> {
        let (description, passive_name, passive_description) = self.conn.query_row(
            "SELECT description, passive_name, passive_description FROM class_definitions WHERE name = ?1",
            [name],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?)),
        ).map_err(|_| format!("Class '{}' not found", name))?;

        // Load granted skills.
        let mut stmt = self.conn.prepare(
            "SELECT skill_name FROM class_skills WHERE class_name = ?1 ORDER BY skill_name"
        ).map_err(|e| e.to_string())?;
        let skills: Vec<String> = stmt.query_map([name], |row| row.get(0))
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;

        // Load attribute bonuses.
        let mut stmt = self.conn.prepare(
            "SELECT attribute, points_per_level FROM class_attribute_bonuses WHERE class_name = ?1"
        ).map_err(|e| e.to_string())?;
        let attribute_bonuses: Vec<(AttributeKind, u32)> = stmt.query_map([name], |row| {
            let attr_str: String = row.get(0)?;
            let points: u32 = row.get(1)?;
            Ok((parse_attribute_kind(&attr_str), points))
        }).map_err(|e| e.to_string())?
          .collect::<Result<Vec<_>, _>>()
          .map_err(|e| e.to_string())?;

        Ok(ClassDefinition {
            name: name.to_string(),
            description,
            skills,
            passive_name,
            passive_description,
            attribute_bonuses,
        })
    }

    /// Save (upsert) a class definition.
    pub fn save_class(&self, class: &ClassDefinition) -> Result<(), String> {
        self.conn.execute_batch("BEGIN").map_err(|e| e.to_string())?;

        self.conn.execute(
            "INSERT OR REPLACE INTO class_definitions (name, description, passive_name, passive_description)
             VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![class.name, class.description, class.passive_name, class.passive_description],
        ).map_err(|e| { let _ = self.conn.execute_batch("ROLLBACK"); e.to_string() })?;

        // Replace skills.
        self.conn.execute("DELETE FROM class_skills WHERE class_name = ?1", [&class.name])
            .map_err(|e| { let _ = self.conn.execute_batch("ROLLBACK"); e.to_string() })?;
        for skill in &class.skills {
            self.conn.execute(
                "INSERT INTO class_skills (class_name, skill_name) VALUES (?1, ?2)",
                rusqlite::params![class.name, skill],
            ).map_err(|e| { let _ = self.conn.execute_batch("ROLLBACK"); e.to_string() })?;
        }

        // Replace attribute bonuses.
        self.conn.execute("DELETE FROM class_attribute_bonuses WHERE class_name = ?1", [&class.name])
            .map_err(|e| { let _ = self.conn.execute_batch("ROLLBACK"); e.to_string() })?;
        for (kind, points) in &class.attribute_bonuses {
            self.conn.execute(
                "INSERT INTO class_attribute_bonuses (class_name, attribute, points_per_level) VALUES (?1, ?2, ?3)",
                rusqlite::params![class.name, kind.name(), points],
            ).map_err(|e| { let _ = self.conn.execute_batch("ROLLBACK"); e.to_string() })?;
        }

        self.conn.execute_batch("COMMIT").map_err(|e| e.to_string())
    }

    /// Delete a class definition (CASCADE removes children).
    pub fn delete_class(&self, name: &str) -> Result<(), String> {
        let affected = self.conn.execute("DELETE FROM class_definitions WHERE name = ?1", [name])
            .map_err(|e| e.to_string())?;
        if affected == 0 {
            return Err(format!("Class '{}' not found", name));
        }
        Ok(())
    }
}

fn parse_attribute_kind(s: &str) -> AttributeKind {
    match s {
        "Strength" => AttributeKind::Strength,
        "Agility" => AttributeKind::Agility,
        "Endurance" => AttributeKind::Endurance,
        "Intelligence" => AttributeKind::Intelligence,
        "Wisdom" => AttributeKind::Wisdom,
        "Perception" => AttributeKind::Perception,
        _ => AttributeKind::Strength,
    }
}
```

Note: `AttributeKind` needs `PartialEq` derived — check and add if missing.

- [ ] **Step 3: Run tests to verify**

Run: `cargo test test_class_save test_class_list -- --nocapture`
Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add src/storage/db.rs
git commit -m "feat: implement class library CRUD on Database"
```

---

## Chunk 4: Rewire CLI

### Task 8: Rewrite CLI to use Database and rename profession to class

**Files:**
- Modify: `src/cli.rs`

- [ ] **Step 1: Rewrite CLI imports and run() to accept &Database**

Replace the top of `src/cli.rs`. The `run` function takes `&Database` instead of constructing paths. All `Command::Profession` variants become `Command::Class`. All `ProfessionAction` becomes `ClassAction`.

Update clap structs:
- `Command::Profession` -> `Command::Class`
- `ProfessionAction` -> `ClassAction`
- All field names and help text: "profession" -> "class"

Update `run()` signature:
```rust
pub fn run(cmd: Command, db: &Database) {
```

Replace all `json_store` calls with `db.*` calls. Replace `UpdateOpts` fields: `add_profession/remove_profession` -> `add_class/remove_class`.

The full CLI rewrite is extensive — update every command handler to use `db` methods instead of file-based operations.

- [ ] **Step 2: Update CLI tests to use Database::open_memory()**

Replace `test_data_dir()` with `Database::open_memory()`. Update all tests to use `db.*` methods. Rename profession tests to class tests.

- [ ] **Step 3: Run tests to verify**

Run: `cargo test cli::tests -- --nocapture`
Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add src/cli.rs
git commit -m "feat: rewrite CLI to use Database, rename profession to class"
```

---

## Chunk 5: Rewire TUI — App, Main, and UI Modules

### Task 9: Update App struct and main.rs

**Files:**
- Modify: `src/ui/app.rs`
- Modify: `src/main.rs`

- [ ] **Step 1: Simplify App struct**

Remove from `App`:
- `data_dir` field (no longer needed — Database knows its path)
- `skill_library`, `tree_library`, `profession_library` (query DB directly)
- `current_character` stays (loaded character in the TUI session)

Remove `DataReloader` entirely.

Add `Tab::ClassLibrary` replacing `Tab::ProfessionLibrary`.

- [ ] **Step 2: Update main.rs**

- Open `Database::open()` at startup
- Pass `&db` to `cli::run(cmd, &db)`
- Pass `&db` to all UI state handlers
- Remove all `json_store` loading/saving code
- Remove reloader
- On quit: only save current character via `db.save_character()`
- Remove tree_library state

- [ ] **Step 3: Verify it compiles**

Run: `cargo check`
Expected: May have errors in UI modules — those are fixed in the next tasks.

- [ ] **Step 4: Commit**

```bash
git add src/ui/app.rs src/main.rs
git commit -m "feat: simplify App struct, wire Database into main"
```

---

### Task 10: Update character_creation.rs to use Database

**Files:**
- Modify: `src/ui/character_creation.rs`

- [ ] **Step 1: Replace json_store calls with Database calls**

- `refresh_characters` → `db.list_characters()`
- `load_character` → `db.load_character()`
- `save_character` → `db.save_character()`
- `delete_character` → `db.delete_character()`

All methods that currently take `&App` will also need `&Database`.

- [ ] **Step 2: Verify it compiles**

Run: `cargo check`
Expected: PASS for this module

- [ ] **Step 3: Commit**

```bash
git add src/ui/character_creation.rs
git commit -m "feat: update character_creation to use Database"
```

---

### Task 11: Update system_panel.rs — profession to class, use Database

**Files:**
- Modify: `src/ui/system_panel.rs`

- [ ] **Step 1: Rename all profession references to class**

- `Section::Profession` -> `Section::Class`
- `PopupMode::AddProfession` -> `PopupMode::AddClass`
- `profession_picker_*` -> `class_picker_*`
- `open_add_profession` -> `open_add_class`
- `handle_add_profession` -> `handle_add_class`
- All display text: "Profession" -> "Class"
- `CharacterProfession` -> `CharacterClass`
- `character.professions` -> `character.classes`
- `character.profession_slots` -> `character.class_slots`
- `app.profession_library` -> query `db.list_classes()`

- [ ] **Step 2: Verify it compiles**

Run: `cargo check`
Expected: PASS for this module

- [ ] **Step 3: Commit**

```bash
git add src/ui/system_panel.rs
git commit -m "feat: rename profession to class in system_panel, use Database"
```

---

### Task 12: Update skill_library.rs to use Database

**Files:**
- Modify: `src/ui/skill_library.rs`

- [ ] **Step 1: Replace json_store calls with Database calls**

- Save skill: `db.save_skill()` instead of `json_store::save_json()`
- Delete skill: `db.delete_skill()` instead of modifying and saving the whole array
- List/load skills from DB instead of `app.skill_library`

- [ ] **Step 2: Verify it compiles**

Run: `cargo check`
Expected: PASS

- [ ] **Step 3: Commit**

```bash
git add src/ui/skill_library.rs
git commit -m "feat: update skill_library to use Database"
```

---

### Task 13: Create class_library.rs (replace profession_library.rs)

**Files:**
- Create: `src/ui/class_library.rs`
- Delete: `src/ui/profession_library.rs`
- Modify: `src/ui/mod.rs`

- [ ] **Step 1: Copy profession_library.rs to class_library.rs and rename**

- Rename all "profession" to "class" throughout
- Use `db.list_classes()`, `db.save_class()`, `db.delete_class()`
- Update `ClassDefinition` usage — add `attribute_bonuses` to the creation wizard
- Update display to show attribute bonuses

- [ ] **Step 2: Update ui/mod.rs**

```rust
pub mod app;
pub mod theme;
pub mod popup;
pub mod character_creation;
pub mod system_panel;
pub mod skill_library;
pub mod class_library;
pub mod card_grid;
```

Remove `pub mod tree_library;` and `pub mod profession_library;`.

- [ ] **Step 3: Delete old files**

```bash
rm src/ui/profession_library.rs
rm src/ui/tree_library.rs
rm src/models/profession.rs
rm src/models/tree.rs
rm src/storage/json_store.rs
```

- [ ] **Step 4: Verify it compiles**

Run: `cargo check`
Expected: PASS — all references to old modules should be gone

- [ ] **Step 5: Commit**

```bash
git add src/ui/class_library.rs src/ui/mod.rs
git add -u  # stages deletions
git commit -m "feat: replace profession_library with class_library, remove tree and json_store"
```

---

## Chunk 6: Final Verification

### Task 14: Run full test suite and manual verification

**Files:** None (verification only)

- [ ] **Step 1: Run full test suite**

Run: `cargo test`
Expected: All tests pass.

- [ ] **Step 2: Build release binary**

Run: `cargo build`
Expected: Clean build, no warnings from our code.

- [ ] **Step 3: Test CLI commands**

```bash
./target/debug/LITRPG_System skill create "Fireball" --category acquired --type active --description "Fire"
./target/debug/LITRPG_System skill list
./target/debug/LITRPG_System class create "Warrior" --description "Melee" --passive-name "Battle Hardened" --passive-desc "Phys resist" --skills "Fireball" --attr-bonus "str:3,end:2"
./target/debug/LITRPG_System class list
./target/debug/LITRPG_System create "Kael" --str 8 --agi 5
./target/debug/LITRPG_System update "Kael" --add-class "Warrior" --level-up 3 --show
./target/debug/LITRPG_System list
```

- [ ] **Step 4: Verify database location**

```bash
ls -la ~/.local/share/litrpg/data.db
```

Expected: File exists at XDG path.

- [ ] **Step 5: Test live TUI/CLI sharing**

1. Launch TUI: `cargo run`
2. In another terminal, add a skill via CLI
3. TUI should show the new skill immediately (no restart, no `r` key)

- [ ] **Step 6: Commit if any cleanup needed**

```bash
git add -A
git commit -m "chore: final verification and cleanup"
```

- [ ] **Step 7: Delete old data directory**

```bash
rm -rf data/
```

```bash
git add -u
git commit -m "chore: remove old JSON data directory"
```
