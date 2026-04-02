# CLI Interface Design

> Programmatic CLI for the LITRPG System, enabling AI chatbots and scripts to manage characters, skills, and professions without launching the TUI.

---

## Purpose

Add a CLI mode to the LITRPG System binary so that AI chatbots can invoke it as a tool when assisting with book writing. All output is JSON to stdout. The TUI launches only when no CLI subcommand is provided.

---

## Command Tree

```
litrpg                            # No args → launch TUI
litrpg list                       # List all saved characters
litrpg show <name>                # Show full character stats
litrpg create <name> [options]    # Create a new character
litrpg delete <name>              # Delete a character
litrpg update <name> [mutations] [--show]  # Mutate character, optionally show result

litrpg skill list                 # List all skills in library
litrpg skill show <name>          # Show full skill definition
litrpg skill create <name> [opts] # Create a new skill
litrpg skill delete <name>        # Delete a skill
litrpg skill update <name> [opts] # Edit skill / add-remove effects

litrpg profession list            # List all professions
litrpg profession show <name>     # Show full profession definition
litrpg profession create <name> [opts]  # Create a new profession
litrpg profession delete <name>   # Delete a profession
litrpg profession update <name> [opts]  # Edit profession / add-remove skills
```

---

## Character Commands

### `litrpg list`

Output: JSON array of character names.

```json
["Kael", "Aria", "Draven"]
```

### `litrpg show <name>`

Output: Full character state as JSON. The `xp_percentage` field is computed at display time.

```json
{
  "name": "Kael",
  "race": "Human",
  "grade": "F",
  "level": 42,
  "xp": 1234.5,
  "xp_percentage": 67.33,
  "unspent_attribute_points": 12,
  "profession_slots": 1,
  "bonus_attribute_points_per_level": 0,
  "attributes": {
    "strength": 78,
    "agility": 65,
    "endurance": 54,
    "intelligence": 41,
    "wisdom": 39,
    "perception": 47
  },
  "innate_skill": {
    "name": "Sword Affinity",
    "level": 8
  },
  "professions": [
    {
      "name": "Blacksmith",
      "level": 0,
      "passive_rank": 0
    }
  ],
  "skills": [
    {
      "name": "Sword Mastery",
      "category": "Acquired",
      "type": "Active",
      "rank": "Apprentice",
      "level": 18
    }
  ]
}
```

### `litrpg create <name> [options]`

Options:
- `--str <n>`, `--agi <n>`, `--end <n>`, `--int <n>`, `--wis <n>`, `--per <n>` — attribute values (1-10, default 5)
- `--innate <skill_name>` — optional innate skill from library

Creates a Grade G, Level 0 character. Race is always "Human" (hardcoded in `Character::new()`). Saves to disk.

Output:
```json
{"status": "ok", "message": "Created character 'Kael'"}
```

### `litrpg delete <name>`

Output:
```json
{"status": "ok", "message": "Deleted character 'Kael'"}
```

### `litrpg update <name> [mutations] [--show]`

Mutation flags (chainable, executed in order):

| Flag | Description |
|------|-------------|
| `--level-up <n>` | Level up n times, granting attribute points per level |
| `--grade-up` | Advance to next grade (resets level to 0, resets XP to 0) |
| `--add-skill <name>` | Add a skill from the library (starts at Novice 0) |
| `--remove-skill <name>` | Remove a skill by name |
| `--add-profession <name>` | Add a profession from the library (respects slot limit) |
| `--remove-profession <name>` | Remove a profession by name |
| `--add-attr <kind>:<points>` | Distribute unspent attribute points (kind: str/agi/end/int/wis/per) |
| `--remove-attr <kind>:<points>` | Remove attribute points (minimum value 1) |
| `--kill <enemy_level>:<count>` | Apply kill XP and auto-level (see Kill Semantics below) |
| `--show` | Output full character JSON after all mutations |

Without `--show`, outputs:
```json
{"status": "ok", "message": "Updated character 'Kael'"}
```

With `--show`, outputs the full character JSON (same format as `litrpg show`).

#### Kill Semantics

`--kill <enemy_level>:<count>` applies kills one at a time in a loop:
1. Calculate `kill_xp(character.level, enemy_level)` for each kill
2. Add XP to character
3. After each kill, check if XP exceeds `xp_required(level, grade)` — if so, auto-level (increment level, grant attribute points, subtract required XP)
4. Level cap is 100 per grade. Grade-up does NOT happen automatically — use `--grade-up` explicitly.

This means the character's level may change between kills, affecting XP calculations for subsequent kills.

---

## Skill Library Commands

### Data Model Note

The `SkillDefinition` struct stores effects inside `ranks: Vec<RankDefinition>`, where each rank has its own description and effects list. The CLI simplifies this: all effects specified via CLI are placed under a single Novice rank definition. This matches the TUI's current behavior. The full rank hierarchy can be managed via the TUI for advanced use cases.

Skills in the library are all stored in `data/skills.json` as `Vec<SkillDefinition>`, regardless of category (Acquired, Innate, Profession). The `SkillCategory` field on `SkillDefinition` determines the category. Innate skill *definitions* in the library use the same `SkillDefinition` struct — the separate `InnateSkillDefinition` struct is only used for future evolution features and is not managed via CLI.

### `litrpg skill list`

Output: JSON array of skill summaries. The `effects` count is the number of effects in the first (Novice) rank.

```json
[
  {"name": "Fireball", "category": "Acquired", "type": "Active", "effects": 2},
  {"name": "Iron Skin", "category": "Acquired", "type": "Passive", "effects": 1},
  {"name": "Sword Affinity", "category": "Innate", "type": "Passive", "effects": 1}
]
```

### `litrpg skill show <name>`

Output: Full skill definition as JSON. Uses serde serialization of `SkillDefinition`.

```json
{
  "name": "Fireball",
  "category": "Acquired",
  "skill_type": "Active",
  "description": "Launches a ball of fire",
  "ranks": [
    {
      "rank": "Novice",
      "description": "",
      "effects": [
        {
          "name": "Fire Damage",
          "description": "Burns the target",
          "base_value": 20.0,
          "unlock_level": 0
        }
      ]
    }
  ]
}
```

### `litrpg skill create <name> [options]`

Options:
- `--category <acquired|innate|profession>` — default: acquired
- `--type <active|passive>` — default: active
- `--description <text>`
- `--effect <name> --base-value <n> --unlock-level <n> --effect-desc <text>` — repeatable group for each effect. All effects are placed under a single Novice rank definition.

Example:
```
litrpg skill create "Fireball" \
  --category acquired --type active \
  --description "Launches a ball of fire" \
  --effect "Fire Damage" --base-value 20 --unlock-level 0 --effect-desc "Burns the target" \
  --effect "Burn DOT" --base-value 5 --unlock-level 5 --effect-desc "Damage over time"
```

Output:
```json
{"status": "ok", "message": "Created skill 'Fireball'"}
```

### `litrpg skill delete <name>`

Removes skill from library. Does not remove from characters that already have it.

### `litrpg skill update <name> [options]`

Options:
- `--description <text>` — update description
- `--category <acquired|innate|profession>` — change category
- `--type <active|passive>` — change type
- `--add-effect <name> --base-value <n> --unlock-level <n> --effect-desc <text>` — add an effect to the Novice rank
- `--remove-effect <name>` — remove an effect by name from the Novice rank

---

## Profession Library Commands

### `litrpg profession list`

Output: JSON array of profession summaries.

```json
[
  {"name": "Blacksmith", "skills": ["Hammer Strike", "Metal Shaping"], "passive": "Forgeborn"}
]
```

### `litrpg profession show <name>`

Output: Full profession definition as JSON. Uses serde serialization of `ProfessionDefinition`.

```json
{
  "name": "Blacksmith",
  "description": "Craft weapons and armor",
  "skills": ["Hammer Strike", "Metal Shaping"],
  "passive_name": "Forgeborn",
  "passive_description": "Increases crafting speed"
}
```

### `litrpg profession create <name> [options]`

Options:
- `--description <text>`
- `--passive-name <text>`
- `--passive-desc <text>`
- `--skills <comma-separated list>`

Example:
```
litrpg profession create "Blacksmith" \
  --description "Craft weapons and armor" \
  --passive-name "Forgeborn" \
  --passive-desc "Increases crafting speed" \
  --skills "Hammer Strike,Metal Shaping"
```

### `litrpg profession delete <name>`

### `litrpg profession update <name> [options]`

Options:
- `--description <text>`
- `--passive-name <text>`
- `--passive-desc <text>`
- `--add-skill <name>` — add a skill to the profession's granted skills list
- `--remove-skill <name>` — remove a skill from the list

---

## Error Handling

All errors output JSON to stderr and exit with code 1:

```json
{"status": "error", "message": "Character 'Kael' not found"}
```

Error scenarios:
- Character / skill / profession not found
- Duplicate name on create
- Invalid attribute values (outside 1-10 on create)
- No unspent attribute points for `--add-attr`
- `--remove-attr` would reduce attribute below 1
- Skill / profession not in library for `--add-skill` / `--add-profession`
- Character already has the skill being added
- Already at max grade (SSS) for `--grade-up`
- Profession slot limit reached for `--add-profession`
- Profession not found on character for `--remove-profession`
- File system errors (permissions, disk full)

---

## Architecture

### File Structure

```
src/
  cli.rs          # NEW — clap definitions + dispatch logic
  main.rs         # MODIFIED — check for subcommands before TUI
```

### How It Works

1. `main.rs` uses `clap` with an optional subcommand enum. If a subcommand is present, call `cli::run()`. Otherwise, launch the TUI.
2. `cli.rs` defines the clap structs (derive API), loads data from disk using `storage::json_store`, executes operations using `models/` and `formulas/`, saves back to disk, and prints JSON to stdout.
3. `cli.rs` has no dependency on `ui/`. It only imports from `models/`, `formulas/`, and `storage/`.
4. Skill library uses the generic `json_store::load_json` / `save_json` functions with `data/skills.json` (same as the TUI already does in `main.rs`).

### Data Directory

Uses the same `data/` directory as the TUI:
- Characters: `data/characters/<name>.json`
- Skills: `data/skills.json` (all categories in one file)
- Professions: `data/professions.json`

### Dependencies

Add `clap = { version = "4", features = ["derive"] }` to Cargo.toml (already added).

---

## Scope Exclusions

- No interactive prompts in CLI mode — all input via flags.
- No TUI features in CLI (no pywal theming, no card grid).
- No `--watch` or streaming modes.
- Tree library management is not included (trees are managed via TUI only).
- `InnateSkillDefinition` (evolutions) is not managed via CLI — innate skills use `SkillDefinition` with `category: Innate`.
- Race is always "Human" — no `--race` flag.
