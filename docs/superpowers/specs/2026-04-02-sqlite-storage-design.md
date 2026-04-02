# SQLite Storage Migration Design

## Goal

Replace JSON file storage with a single SQLite database. Both TUI and CLI share one database file at a fixed XDG-standard path, eliminating working-directory issues and enabling live data sharing without polling.

Rename "profession" to "class" throughout, and add attribute bonuses per class level.

## Database Location

Uses the `dirs` crate for cross-platform path resolution:

| OS | Path |
|---|---|
| Linux | `~/.local/share/litrpg/data.db` |
| macOS | `~/Library/Application Support/litrpg/data.db` |
| Windows | `C:\Users\<name>\AppData\Roaming\litrpg\data.db` |

Directory created automatically on first run. WAL mode enabled for concurrent read/write.

## Schema

```sql
CREATE TABLE characters (
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

CREATE TABLE character_skills (
    character_name TEXT REFERENCES characters(name) ON DELETE CASCADE,
    skill_name TEXT NOT NULL,
    rank TEXT NOT NULL DEFAULT 'Novice',
    level INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (character_name, skill_name)
);

CREATE TABLE character_innate_skills (
    character_name TEXT PRIMARY KEY REFERENCES characters(name) ON DELETE CASCADE,
    skill_name TEXT NOT NULL,
    level INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE character_classes (
    character_name TEXT REFERENCES characters(name) ON DELETE CASCADE,
    class_name TEXT NOT NULL,
    level INTEGER NOT NULL DEFAULT 0,
    passive_rank INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (character_name, class_name)
);

CREATE TABLE skill_definitions (
    name TEXT PRIMARY KEY,
    category TEXT NOT NULL,
    skill_type TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT ''
);

CREATE TABLE skill_effects (
    skill_name TEXT REFERENCES skill_definitions(name) ON DELETE CASCADE,
    rank TEXT NOT NULL,
    effect_name TEXT,
    description TEXT NOT NULL DEFAULT '',
    base_value REAL NOT NULL DEFAULT 0.0,
    unlock_level INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE class_definitions (
    name TEXT PRIMARY KEY,
    description TEXT NOT NULL DEFAULT '',
    passive_name TEXT NOT NULL DEFAULT '',
    passive_description TEXT NOT NULL DEFAULT ''
);

CREATE TABLE class_skills (
    class_name TEXT REFERENCES class_definitions(name) ON DELETE CASCADE,
    skill_name TEXT NOT NULL,
    PRIMARY KEY (class_name, skill_name)
);

CREATE TABLE class_attribute_bonuses (
    class_name TEXT REFERENCES class_definitions(name) ON DELETE CASCADE,
    attribute TEXT NOT NULL,
    points_per_level INTEGER NOT NULL,
    PRIMARY KEY (class_name, attribute)
);
```

Grade stored as integer (0=G through 9=SSS). Rank stored as text. `ON DELETE CASCADE` on all foreign keys.

## Database API

Single `Database` struct in `src/storage/db.rs`:

```rust
pub struct Database {
    conn: rusqlite::Connection,
}

impl Database {
    pub fn open() -> Result<Self, String>

    // Characters
    pub fn list_characters(&self) -> Result<Vec<String>, String>
    pub fn load_character(&self, name: &str) -> Result<Character, String>
    pub fn save_character(&self, character: &Character) -> Result<(), String>  // upsert
    pub fn delete_character(&self, name: &str) -> Result<(), String>

    // Skill library
    pub fn list_skills(&self) -> Result<Vec<SkillDefinition>, String>
    pub fn load_skill(&self, name: &str) -> Result<SkillDefinition, String>
    pub fn save_skill(&self, skill: &SkillDefinition) -> Result<(), String>
    pub fn delete_skill(&self, name: &str) -> Result<(), String>

    // Class library
    pub fn list_classes(&self) -> Result<Vec<ClassDefinition>, String>
    pub fn load_class(&self, name: &str) -> Result<ClassDefinition, String>
    pub fn save_class(&self, class: &ClassDefinition) -> Result<(), String>
    pub fn delete_class(&self, name: &str) -> Result<(), String>
}
```

`save_character` does a full upsert in a single transaction: replaces the character row plus all related rows (skills, classes, innate).

## Model Changes

- `ProfessionDefinition` renamed to `ClassDefinition`, gains `attribute_bonuses: Vec<(AttributeKind, u32)>` field
- `CharacterProfession` renamed to `CharacterClass`
- `Character.profession_slots` renamed to `Character.class_slots`
- Tree-related models removed

## Data Flow Change

**Before:** TUI loads all data into memory at startup, saves on exit. DataReloader polls file mtimes every 2s.

**After:** TUI queries the database on each render. Saves immediately on mutation. CLI does the same. Both always see current data. No reloader needed.

## What Gets Removed

- `src/storage/json_store.rs`
- `DataReloader` in `src/ui/app.rs`
- `data/` directory and all JSON files
- `src/ui/tree_library.rs`
- Tree-related models (`TreeChain`, `TreeDefinition`, `TreeMilestone`, `UnlockRequirement`, `Comparison`)
- `DATA_DIR` constant

## What Gets Added

- `src/storage/db.rs` — `Database` struct with all CRUD
- `src/models/class.rs` — `ClassDefinition` with attribute bonuses
- `rusqlite` and `dirs` crate dependencies

## What Gets Modified

- `src/storage/mod.rs` — `pub mod db` replaces `pub mod json_store`
- `src/models/mod.rs` — `pub mod class` replaces `pub mod profession`
- `src/models/character.rs` — profession -> class rename
- `src/ui/app.rs` — `App` holds `&Database`, no reloader, class_library replaces profession_library
- `src/main.rs` — `Database::open()`, passes db to TUI/CLI
- `src/cli.rs` — uses `&Database`, profession -> class rename
- `src/ui/system_panel.rs` — queries DB, profession -> class
- `src/ui/skill_library.rs` — queries DB
- `src/ui/profession_library.rs` — renamed to `src/ui/class_library.rs`

## Dependencies

```toml
rusqlite = { version = "0.31", features = ["bundled"] }
dirs = "5"
```

`bundled` feature compiles SQLite from source so no system library dependency.

## Implementation Approach

Incremental steps, each one compiling and testable before moving to the next.
