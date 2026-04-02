/// Pywal color theme — loads colors from ~/.cache/wal/colors.json at runtime.
///
/// Maps pywal color slots to UI roles:
///   background  → special.background
///   foreground  → special.foreground
///   accent      → color1 (active tab, cursor, warnings)
///   dimmed      → color8 (inactive tabs)
///   secondary   → color2 (section headers)
///   border      → color4 (borders, dividers)
///
/// Falls back to sensible defaults if pywal isn't available.

use ratatui::style::Color;
use std::fs;
use std::path::PathBuf;

/// Holds resolved RGB colors for each UI role.
#[derive(Debug, Clone)]
pub struct Theme {
    pub bg: Color,
    pub fg: Color,
    pub accent: Color,
    pub dimmed: Color,
    pub secondary: Color,
    pub border: Color,
}

impl Theme {
    /// Try to load pywal colors from the standard cache path.
    /// Returns default theme if the file is missing or malformed.
    pub fn load() -> Self {
        let path = dirs_path().join("colors.json");
        match fs::read_to_string(&path) {
            Ok(json) => Self::from_json(&json).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    /// Parse the pywal JSON format into a Theme.
    fn from_json(json: &str) -> Option<Self> {
        let root: serde_json::Value = serde_json::from_str(json).ok()?;

        let special = root.get("special")?;
        let colors = root.get("colors")?;

        Some(Self {
            bg: parse_hex(special.get("background")?.as_str()?)?,
            fg: parse_hex(special.get("foreground")?.as_str()?)?,
            accent: parse_hex(colors.get("color1")?.as_str()?)?,
            dimmed: parse_hex(colors.get("color8")?.as_str()?)?,
            secondary: parse_hex(colors.get("color2")?.as_str()?)?,
            border: parse_hex(colors.get("color4")?.as_str()?)?,
        })
    }
}

impl Default for Theme {
    /// Fallback colors when pywal isn't available.
    fn default() -> Self {
        Self {
            bg: Color::Black,
            fg: Color::White,
            accent: Color::Red,
            dimmed: Color::DarkGray,
            secondary: Color::Green,
            border: Color::Blue,
        }
    }
}

/// Standard pywal cache directory.
fn dirs_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".cache").join("wal")
}

/// Convert a hex color string (#RRGGBB) to a ratatui Color.
fn parse_hex(hex: &str) -> Option<Color> {
    let hex = hex.strip_prefix('#')?;
    if hex.len() != 6 { return None; }
    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
    Some(Color::Rgb(r, g, b))
}
