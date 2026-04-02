/// LITRPG System — terminal-based progression system.
///
/// Entry point: sets up the terminal, loads persisted data, runs the
/// main event loop with tab-based navigation, saves on quit, and
/// restores terminal state.

mod models;
mod formulas;
mod storage;
mod ui;
mod cli;

use std::io;
use std::path::PathBuf;
use crossterm::{
    event::{self, Event, KeyCode},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    execute,
};
use ratatui::{
    backend::CrosstermBackend,
    Terminal,
    widgets::{Block, Borders, Paragraph},
    layout::{Layout, Constraint, Direction, Rect, Alignment},
    style::{Style, Modifier},
    text::{Line, Span},
};
use ui::app::{App, DataReloader, Tab};
use ui::theme::Theme;
use ui::character_creation::CharacterCreationState;
use ui::system_panel::SystemPanelState;
use ui::skill_library::SkillLibraryState;
use ui::tree_library::TreeLibraryState;
use ui::profession_library::ProfessionLibraryState;
use ui::popup;
use storage::json_store;

/// Returns the absolute path for all persisted data.
/// Uses XDG standard: ~/.local/share/litrpg (Linux), AppData/Roaming/litrpg (Windows).
pub fn data_dir() -> std::path::PathBuf {
    let dir = dirs::data_dir()
        .expect("Could not determine data directory")
        .join("litrpg");
    std::fs::create_dir_all(&dir).expect("Could not create data directory");
    dir
}

/// ASCII art title displayed at the top of every screen.
const TITLE_ART: &[&str] = &[
    "╦  ╦╔╦╗╦═╗╔═╗╔═╗",
    "║  ║ ║ ╠╦╝╠═╝║ ╦",
    "╩═╝╩ ╩ ╩╚═╩  ╚═╝",
];

fn main() -> io::Result<()> {
    // Check for CLI subcommands before launching the TUI.
    use clap::Parser;
    let cli_args = cli::Cli::parse();
    if let Some(cmd) = cli_args.command {
        cli::run(cmd);
        return Ok(());
    }

    // Enter raw mode and alternate screen for TUI rendering.
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Load pywal color theme.
    let theme = Theme::load();

    let data_dir = data_dir();
    let mut app = App::new(data_dir);

    // Per-tab state — created once, reused throughout.
    let mut char_state = CharacterCreationState::new();
    let mut panel_state = SystemPanelState::new();
    let mut skill_state = SkillLibraryState::new();
    let mut _tree_state = TreeLibraryState::new();
    let mut profession_state = ProfessionLibraryState::new();

    // Live data reloader — initial load + periodic mtime checks.
    let mut reloader = DataReloader::new();
    reloader.force_reload(&mut app, &mut char_state);

    // Main event loop.
    while app.running {
        // Check for external data changes (mtime-gated, every 2s).
        reloader.check(&mut app, &mut char_state);

        terminal.draw(|f| {
            let full = f.area();

            // Outer border frame around the entire application.
            let outer_block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.border))
                .style(Style::default().bg(theme.bg));
            let inner = outer_block.inner(full);
            f.render_widget(outer_block, full);

            // Split inner area: title + tabs | separator | content | separator | help.
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(4),  // ASCII art (3) + tab bar (1)
                    Constraint::Length(1),  // horizontal separator
                    Constraint::Min(1),    // content area
                    Constraint::Length(1),  // horizontal separator
                    Constraint::Length(1),  // help bar
                ])
                .split(inner);

            // Render centered title + tab bar.
            render_title_and_tabs(f, chunks[0], &app, &theme);

            // Top horizontal separator.
            let sep = make_separator(chunks[1].width as usize, &theme);
            f.render_widget(sep, chunks[1]);

            // Render active tab content.
            match app.active_tab {
                Tab::Character => char_state.render(f, chunks[2], &app, &theme),
                Tab::SystemPanel => panel_state.render(f, chunks[2], &app, &theme),
                Tab::SkillLibrary => skill_state.render(f, chunks[2], &app, &theme),
                Tab::ProfessionLibrary => profession_state.render(f, chunks[2], &app, &theme),
            }

            // Bottom horizontal separator.
            let sep = make_separator(chunks[3].width as usize, &theme);
            f.render_widget(sep, chunks[3]);

            // Help bar at the bottom.
            if app.show_help {
                let help_text = match app.active_tab {
                    Tab::Character => " Tab: Next tab  j/k: Navigate  Enter: Load  a: New  d: Delete  r: Reload  ?: Hide help  q: Quit",
                    Tab::SystemPanel => " Tab: Next tab  j/k: Navigate  +/-: Adjust  Enter: Expand  d: Delete  r: Reload  ?: Hide help  q: Quit",
                    Tab::SkillLibrary => " Tab: Next tab  j/k: Navigate  /: Search  Enter: View  a: Add  d: Delete  r: Reload  ?: Hide help  q: Quit",
                    Tab::ProfessionLibrary => " Tab: Next tab  j/k: Navigate  /: Search  Enter: View  a: Add  d: Delete  r: Reload  ?: Hide help",
                };
                let help = Paragraph::new(help_text)
                    .style(Style::default().fg(theme.dimmed));
                f.render_widget(help, chunks[4]);
            } else {
                // Compact help hint with right-aligned quit.
                let left = " ?: Help";
                let right = "q: Quit ";
                let gap = (chunks[4].width as usize).saturating_sub(left.len() + right.len());
                let hint_text = format!("{}{}{}", left, " ".repeat(gap), right);
                let hint = Paragraph::new(hint_text)
                    .style(Style::default().fg(theme.dimmed));
                f.render_widget(hint, chunks[4]);
            }

            // Quit confirmation popup (renders on top of everything).
            if app.show_quit_confirm {
                popup::render_popup(
                    f,
                    full,
                    &theme,
                    "Quit LITRPG System?",
                    &[
                        Line::from(""),
                        Line::from("All data will be saved."),
                        Line::from(""),
                    ],
                    "[y] Yes     [n] No",
                );
            }
        })?;

        // Input handling.
        if let Event::Key(key) = event::read()? {
            // Quit confirmation takes priority.
            if app.show_quit_confirm {
                match key.code {
                    KeyCode::Char('y') => app.running = false,
                    KeyCode::Char('n') | KeyCode::Esc => app.show_quit_confirm = false,
                    _ => {}
                }
                continue;
            }

            // Global keys: Tab to cycle, ? to toggle help, q to quit.
            // Skip global Tab when skill wizard needs it for field cycling.
            let tab_consumed = app.active_tab == Tab::SkillLibrary && skill_state.wants_tab();
            match key.code {
                KeyCode::Tab if !tab_consumed => {
                    app.active_tab = app.active_tab.next();
                    continue;
                }
                KeyCode::Char('?') => {
                    app.show_help = !app.show_help;
                    continue;
                }
                KeyCode::Char('q') => {
                    app.show_quit_confirm = true;
                    continue;
                }
                KeyCode::Char('r') => {
                    reloader.force_reload(&mut app, &mut char_state);
                    continue;
                }
                _ => {}
            }

            // Dispatch to the active tab's input handler.
            match app.active_tab {
                Tab::Character => { char_state.handle_input(key, &mut app); }
                Tab::SystemPanel => { panel_state.handle_input(key, &mut app); }
                Tab::SkillLibrary => { skill_state.handle_input(key, &mut app); }
                Tab::ProfessionLibrary => { profession_state.handle_input(key, &mut app); }
            }
        }
    }

    // Save all data before exiting.
    let char_dir = app.data_dir.join("characters");
    if let Some(character) = &app.current_character {
        let _ = json_store::save_character(&char_dir, character);
    }
    let _ = json_store::save_json(&app.data_dir.join("skills.json"), &app.skill_library);
    let _ = json_store::save_json(&app.data_dir.join("trees.json"), &app.tree_library);
    let _ = json_store::save_json(&app.data_dir.join("professions.json"), &app.profession_library);

    // Restore terminal state.
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    Ok(())
}

/// Render the centered ASCII art title and centered tab bar.
fn render_title_and_tabs(f: &mut ratatui::Frame, area: Rect, app: &App, theme: &Theme) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // ASCII art
            Constraint::Length(1), // tab bar
        ])
        .split(area);

    // ASCII art title — centered using Alignment.
    let art_lines: Vec<Line> = TITLE_ART.iter()
        .map(|line| Line::styled(*line, Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)))
        .collect();
    let art = Paragraph::new(art_lines).alignment(Alignment::Center);
    f.render_widget(art, chunks[0]);

    // Tab bar — centered with spacing.
    let mut spans: Vec<Span> = Vec::new();
    for (i, tab) in Tab::ALL.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled("   ", Style::default().fg(theme.dimmed)));
        }
        let style = if *tab == app.active_tab {
            Style::default().fg(theme.accent).add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
        } else {
            Style::default().fg(theme.dimmed)
        };
        spans.push(Span::styled(tab.label(), style));
    }
    let tab_line = Line::from(spans);
    let tab_bar = Paragraph::new(vec![tab_line]).alignment(Alignment::Center);
    f.render_widget(tab_bar, chunks[1]);
}

/// Create a horizontal separator line that spans the given width.
fn make_separator(width: usize, theme: &Theme) -> Paragraph<'static> {
    let line = "─".repeat(width);
    Paragraph::new(line).style(Style::default().fg(theme.border))
}
