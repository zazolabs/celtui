//! Celestial Navigation TUI Application
//!
//! A terminal user interface for celestial navigation calculations,
//! built with ratatui and the celtnav library.

mod app;
mod ui;
mod calculation_screen;
mod almanac_screen;
mod sight_reduction_screen;
mod auto_compute_screen;
mod averaging_screen;
mod arc_to_time_screen;
mod twilight_screen;
mod dr_ep_screen;
mod persistence;
mod export;
mod validation;

use anyhow::Result;
use app::App;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;
use std::time::Duration;

fn main() -> Result<()> {
    // Set up terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create application state
    let mut app = App::new();

    // Run the application
    let result = run_app(&mut terminal, &mut app);

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    // Print any errors that occurred
    if let Err(err) = result {
        eprintln!("Error: {:?}", err);
    }

    Ok(())
}

/// Main application loop
///
/// Continuously handles events and renders the UI until the user quits.
fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
) -> Result<()> {
    loop {
        // Render UI
        terminal.draw(|frame| {
            ui::render(frame, app);
        })?;

        // Handle events with a timeout
        app.handle_events(Duration::from_millis(100))?;

        // Check if we should quit
        if app.should_quit {
            break;
        }
    }

    Ok(())
}
