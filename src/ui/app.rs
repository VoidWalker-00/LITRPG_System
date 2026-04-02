/// Core application state and tab management.
///
/// The App struct holds all runtime state: active tab, loaded data,
/// the active character, and global UI flags. Tabs replace the old
/// Screen/MainMenu pattern — there's no main menu, just 4 tabs.

use std::fs;
use std::path::PathBuf;
use std::time::{Duration, Instant, SystemTime};
use crate::models::character::Character;
use crate::models::skill::SkillDefinition;
use crate::models::tree::TreeChain;
use crate::models::profession::ProfessionDefinition;
use crate::storage::json_store;

/// The four navigable tabs, cycled with the Tab key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Character,
    SystemPanel,
    SkillLibrary,
    ProfessionLibrary,
}

impl Tab {
    /// All tabs in display order.
    pub const ALL: [Tab; 4] = [
        Tab::Character,
        Tab::SystemPanel,
        Tab::SkillLibrary,
        Tab::ProfessionLibrary,
    ];

    /// Display label for the tab bar.
    pub fn label(&self) -> &'static str {
        match self {
            Tab::Character => "Character",
            Tab::SystemPanel => "System Panel",
            Tab::SkillLibrary => "Skill Library",
            Tab::ProfessionLibrary => "Profession Library",
        }
    }

    /// Cycle to the next tab (wraps around).
    pub fn next(self) -> Tab {
        match self {
            Tab::Character => Tab::SystemPanel,
            Tab::SystemPanel => Tab::SkillLibrary,
            Tab::SkillLibrary => Tab::ProfessionLibrary,
            Tab::ProfessionLibrary => Tab::Character,
        }
    }
}

/// Central application state shared across all tabs.
pub struct App {
    /// Which tab is currently active.
    pub active_tab: Tab,
    /// Whether the app is still running (false = quit).
    pub running: bool,
    /// Root directory for all saved data.
    pub data_dir: PathBuf,
    /// The currently loaded character (None if no character is active).
    pub current_character: Option<Character>,
    /// All skill definitions loaded from the library.
    pub skill_library: Vec<SkillDefinition>,
    /// All tree chains loaded from the library (kept for tree_library.rs compatibility).
    pub tree_library: Vec<TreeChain>,
    /// All profession definitions loaded from the library.
    pub profession_library: Vec<ProfessionDefinition>,
    /// Whether the help bar is visible (toggled with ?).
    pub show_help: bool,
    /// Whether the quit confirmation popup is showing.
    pub show_quit_confirm: bool,
}

impl App {
    /// Create a new App with default state pointing to the given data directory.
    pub fn new(data_dir: PathBuf) -> Self {
        Self {
            active_tab: Tab::Character,
            running: true,
            data_dir,
            current_character: None,
            skill_library: Vec::new(),
            tree_library: Vec::new(),
            profession_library: Vec::new(),
            show_help: false,
            show_quit_confirm: false,
        }
    }
}

// ---------------------------------------------------------------------------
// Live data reloader — mtime-based, checks every 2 seconds
// ---------------------------------------------------------------------------

/// Interval between filesystem mtime checks.
const RELOAD_INTERVAL: Duration = Duration::from_secs(2);

/// Tracks file modification times and reloads data into App when files change.
pub struct DataReloader {
    last_check: Instant,
    mtime_skills: Option<SystemTime>,
    mtime_trees: Option<SystemTime>,
    mtime_professions: Option<SystemTime>,
    mtime_characters: Option<SystemTime>,
    mtime_active_char: Option<SystemTime>,
}

impl DataReloader {
    /// Create a new reloader. Call `force_reload` right after to do the initial load.
    pub fn new() -> Self {
        Self {
            last_check: Instant::now(),
            mtime_skills: None,
            mtime_trees: None,
            mtime_professions: None,
            mtime_characters: None,
            mtime_active_char: None,
        }
    }

    /// Check if any data files changed and reload if needed. Called every frame;
    /// only stats the filesystem every RELOAD_INTERVAL.
    pub fn check(&mut self, app: &mut App, char_state: &mut super::character_creation::CharacterCreationState) {
        if self.last_check.elapsed() < RELOAD_INTERVAL {
            return;
        }
        self.last_check = Instant::now();
        self.reload_if_changed(app, char_state);
    }

    /// Force an immediate reload of all data files regardless of mtime.
    pub fn force_reload(&mut self, app: &mut App, char_state: &mut super::character_creation::CharacterCreationState) {
        self.mtime_skills = None;
        self.mtime_trees = None;
        self.mtime_professions = None;
        self.mtime_characters = None;
        self.mtime_active_char = None;
        self.reload_if_changed(app, char_state);
    }

    /// Compare mtimes and reload files that changed.
    fn reload_if_changed(&mut self, app: &mut App, char_state: &mut super::character_creation::CharacterCreationState) {
        let skills_path = app.data_dir.join("skills.json");
        let trees_path = app.data_dir.join("trees.json");
        let profs_path = app.data_dir.join("professions.json");
        let char_dir = app.data_dir.join("characters");

        // Skills library.
        if let Some(new_mtime) = file_mtime(&skills_path) {
            if self.mtime_skills != Some(new_mtime) {
                self.mtime_skills = Some(new_mtime);
                if let Ok(skills) = json_store::load_json::<Vec<SkillDefinition>>(&skills_path) {
                    app.skill_library = skills;
                }
            }
        }

        // Tree library.
        if let Some(new_mtime) = file_mtime(&trees_path) {
            if self.mtime_trees != Some(new_mtime) {
                self.mtime_trees = Some(new_mtime);
                if let Ok(trees) = json_store::load_json::<Vec<TreeChain>>(&trees_path) {
                    app.tree_library = trees;
                }
            }
        }

        // Profession library.
        if let Some(new_mtime) = file_mtime(&profs_path) {
            if self.mtime_professions != Some(new_mtime) {
                self.mtime_professions = Some(new_mtime);
                if let Ok(profs) = json_store::load_json::<Vec<ProfessionDefinition>>(&profs_path) {
                    app.profession_library = profs;
                }
            }
        }

        // Characters directory — use dir mtime as a proxy for any file change.
        if let Some(new_mtime) = file_mtime(&char_dir) {
            if self.mtime_characters != Some(new_mtime) {
                self.mtime_characters = Some(new_mtime);
                char_state.refresh_characters(app);
            }
        }

        // Active character — reload if its file changed on disk.
        if let Some(character) = &app.current_character {
            let char_path = char_dir.join(format!("{}.json", character.name));
            if let Some(new_mtime) = file_mtime(&char_path) {
                if self.mtime_active_char != Some(new_mtime) {
                    self.mtime_active_char = Some(new_mtime);
                    if let Ok(reloaded) = json_store::load_character(&char_dir, &character.name) {
                        app.current_character = Some(reloaded);
                    }
                }
            }
        } else {
            self.mtime_active_char = None;
        }
    }
}

/// Get the modification time of a path, or None if it doesn't exist.
fn file_mtime(path: &std::path::Path) -> Option<SystemTime> {
    fs::metadata(path).ok().and_then(|m| m.modified().ok())
}
