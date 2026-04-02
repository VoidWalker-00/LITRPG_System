pub mod app;
pub mod theme;
pub mod popup;
pub mod character_creation;
pub mod system_panel;
pub mod skill_library;
pub mod profession_library;
pub mod card_grid;

/// Build a full-width divider: "  ─────...──" padded to fill the area.
pub fn divider(width: u16) -> String {
    let w = (width as usize).saturating_sub(4); // 2 indent + 2 margin
    format!("  {}", "─".repeat(w))
}

