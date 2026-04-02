# LITRPG System

A terminal-based tool for building and tracking progression systems in LITRPG-style worlds.
Designed for writers and world-builders who want fine-grained control over stats, levels, skills, and character growth.

---

## Overview

LITRPG System separates **system design** (the rules of your world) from **character tracking** (individual progress).

- Design progression systems — stats, levels, grades, skills, classes
- Create and track characters that live within those systems
- View and manage everything through a terminal UI or script against it with the CLI

---

## TUI (Terminal User Interface)

Launch by running `litrpg` with no arguments.

```
litrpg
```

The TUI is a full-screen terminal application with four tabs:

| Tab | What it does |
|-----|-------------|
| **Character** | Create, load, and view characters. Shows stats, grade, level, XP, skills, and class. |
| **System Panel** | Manage the progression system — adjust stat formulas, level thresholds, and grade rules. |
| **Skill Library** | Browse, add, search, and delete skills available in your world. |
| **Profession Library** | Manage classes/professions that characters can hold. |

**Navigation:**
- `Tab` — cycle between tabs
- `j` / `k` — move up/down in lists
- `Enter` — select / expand
- `a` — add new entry
- `d` — delete selected
- `/` — search (Skill/Profession tabs)
- `r` — reload data from disk
- `?` — toggle help bar
- `q` — quit

---

## CLI (Command Line Interface)

The CLI lets you manage characters and data from scripts or other tools without opening the TUI.
All output is JSON.

```
litrpg <command> [options]
```

### Commands

**List all characters:**
```bash
litrpg list
```

**Show a character's full stats:**
```bash
litrpg show "Kael"
```

**Create a new character:**
```bash
litrpg create "Kael" --str 7 --agi 6 --end 5 --int 4 --wis 4 --per 4
```

Default value for all stats is `5` if not specified.

**Delete a character:**
```bash
litrpg delete "Kael"
```

**Update a character** (level up, grade up, add/remove skills or professions):
```bash
litrpg update "Kael" --level-up 3
litrpg update "Kael" --grade-up
litrpg update "Kael" --add-skill "Shadow Step"
litrpg update "Kael" --remove-skill "Shadow Step"
litrpg update "Kael" --add-profession "Rogue"
```

---

## Installation

### From AUR (Arch Linux) — recommended

Using `yay`:
```bash
yay -S litrpg-system-git
```

This clones and builds from source automatically.

### From Source

**Requirements:** Rust (stable), Cargo

```bash
git clone https://github.com/VoidWalker-00/LITRPG_System.git
cd LITRPG_System
cargo build --release
sudo install -Dm755 target/release/LITRPG_System /usr/bin/litrpg
```

---

## Data Storage

Character and system data is stored at:
- **Linux:** `~/.local/share/litrpg/`
- **Windows:** `%APPDATA%\litrpg\`

---

## License

MIT
