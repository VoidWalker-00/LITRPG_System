/// JSON file persistence for characters, skills, trees, and professions.
///
/// Characters are saved as individual files in a directory (one per character).
/// Skills, trees, and professions are saved as single JSON files (arrays).
/// All functions return Result<T, String> for simple error propagation.

use std::fs;
use std::path::Path;
use serde::{Serialize, de::DeserializeOwned};
use crate::models::character::Character;
use crate::models::class::ClassDefinition;

/// Save a character to a JSON file named after the character.
/// Creates the directory if it doesn't exist.
pub fn save_character(dir: &Path, character: &Character) -> Result<(), String> {
    fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let path = dir.join(format!("{}.json", character.name));
    let json = serde_json::to_string_pretty(character).map_err(|e| e.to_string())?;
    fs::write(path, json).map_err(|e| e.to_string())
}

/// Load a character by name from the given directory.
pub fn load_character(dir: &Path, name: &str) -> Result<Character, String> {
    let path = dir.join(format!("{}.json", name));
    let json = fs::read_to_string(path).map_err(|e| e.to_string())?;
    serde_json::from_str(&json).map_err(|e| e.to_string())
}

/// List all saved character names (derived from .json filenames).
pub fn list_characters(dir: &Path) -> Result<Vec<String>, String> {
    let mut names = Vec::new();
    let entries = fs::read_dir(dir).map_err(|e| e.to_string())?;
    for entry in entries {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("json") {
            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                names.push(stem.to_string());
            }
        }
    }
    Ok(names)
}

/// Delete a character's save file by name.
pub fn delete_character(dir: &Path, name: &str) -> Result<(), String> {
    let path = dir.join(format!("{}.json", name));
    fs::remove_file(path).map_err(|e| e.to_string())
}

/// Generic save: serialize any Serialize type to a JSON file.
/// Creates parent directories if needed.
pub fn save_json<T: Serialize + ?Sized>(path: &Path, data: &T) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(data).map_err(|e| e.to_string())?;
    fs::write(path, json).map_err(|e| e.to_string())
}

/// Generic load: deserialize any DeserializeOwned type from a JSON file.
pub fn load_json<T: DeserializeOwned>(path: &Path) -> Result<T, String> {
    let json = fs::read_to_string(path).map_err(|e| e.to_string())?;
    serde_json::from_str(&json).map_err(|e| e.to_string())
}

/// Save class definitions to a JSON file.
pub fn save_classes(path: &Path, classes: &[ClassDefinition]) -> Result<(), String> {
    save_json(path, classes)
}

/// Load class definitions from a JSON file.
pub fn load_classes(path: &Path) -> Result<Vec<ClassDefinition>, String> {
    load_json(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::attribute::Attributes;
    use crate::models::character::Character;
    use std::fs;

    #[test]
    fn test_save_and_load_character() {
        let dir = std::env::temp_dir().join("litrpg_test_save_load");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        let attrs = Attributes::new_clamped(5, 5, 5, 5, 5, 5);
        let character = Character::new("TestChar".to_string(), attrs, None);

        save_character(&dir, &character).unwrap();
        let loaded = load_character(&dir, "TestChar").unwrap();

        assert_eq!(loaded.name, "TestChar");
        assert_eq!(loaded.grade, crate::models::grade::Grade::G);
        assert_eq!(loaded.level, 0);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_list_characters() {
        let dir = std::env::temp_dir().join("litrpg_test_list");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        let attrs = Attributes::new_clamped(5, 5, 5, 5, 5, 5);
        save_character(&dir, &Character::new("Alice".to_string(), attrs.clone(), None)).unwrap();
        save_character(&dir, &Character::new("Bob".to_string(), attrs, None)).unwrap();

        let names = list_characters(&dir).unwrap();
        assert_eq!(names.len(), 2);
        assert!(names.contains(&"Alice".to_string()));
        assert!(names.contains(&"Bob".to_string()));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_save_and_load_skill_library() {
        let path = std::env::temp_dir().join("litrpg_test_skills.json");
        let _ = fs::remove_file(&path);

        let skills: Vec<crate::models::skill::SkillDefinition> = vec![];
        save_json(&path, &skills).unwrap();
        let loaded: Vec<crate::models::skill::SkillDefinition> = load_json(&path).unwrap();
        assert!(loaded.is_empty());

        let _ = fs::remove_file(&path);
    }
}
