//! User interface rendering
//!
//! This module contains functions for rendering different screens
//! and UI components using ratatui.

use crate::app::{App, Screen};
use crate::calculation_screen::render_calculation_screen;
use crate::almanac_screen::render_almanac_screen;
use crate::sight_reduction_screen::render_sight_reduction_screen;
use crate::auto_compute_screen::render_auto_compute_screen;
use crate::averaging_screen::render_averaging_screen;
use crate::arc_to_time_screen::render_arc_to_time_screen;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame,
};

/// Renders the entire UI based on current application state
pub fn render(frame: &mut Frame, app: &App) {
    // Create main layout with title bar and content area
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Title bar
            Constraint::Min(0),    // Content area
            Constraint::Length(2), // Status/help bar
        ])
        .split(frame.area());

    // Render title bar
    render_title_bar(frame, chunks[0], app);

    // Render current screen
    match app.current_screen {
        Screen::Home => render_home_screen(frame, chunks[1]),
        Screen::Almanac => render_almanac_screen(frame, chunks[1], &app.almanac_form),
        Screen::SightReduction => render_sight_reduction_screen(frame, chunks[1], &app.sight_reduction_form),
        Screen::AutoCompute => render_auto_compute_screen(frame, chunks[1], &app.auto_compute_form),
        Screen::Calculation => render_calculation_screen(frame, chunks[1], &app.calculation_form),
        Screen::Averaging => render_averaging_screen(frame, chunks[1], &app.averaging_form),
        Screen::ArcToTime => render_arc_to_time_screen(frame, chunks[1], &app.arc_to_time_form),
        Screen::Help => render_help_screen(frame, chunks[1]),
    }

    // Render status bar
    render_status_bar(frame, chunks[2], app);
}

/// Renders the title bar at the top of the screen
fn render_title_bar(frame: &mut Frame, area: Rect, app: &App) {
    let title = Paragraph::new(app.current_screen_title())
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::White)),
        );

    frame.render_widget(title, area);
}

/// Renders the status bar at the bottom of the screen
fn render_status_bar(frame: &mut Frame, area: Rect, app: &App) {
    let help_text = match app.current_screen {
        Screen::Home => "1-6: Select | ?: Help | Tab: Next | Q: Quit",
        _ => "H: Home | ?: Help | Tab: Next | Q: Back | Esc: Home",
    };

    let status = Paragraph::new(help_text)
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Center);

    frame.render_widget(status, area);
}

/// Renders the home/menu screen
fn render_home_screen(frame: &mut Frame, area: Rect) {
    // Create centered layout
    let vertical_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(20),
            Constraint::Percentage(60),
            Constraint::Percentage(20),
        ])
        .split(area);

    let horizontal_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(25),
            Constraint::Percentage(50),
            Constraint::Percentage(25),
        ])
        .split(vertical_chunks[1]);

    let center_area = horizontal_chunks[1];

    // Create menu items
    let menu_items = vec![
        ListItem::new("1. Almanac Data Lookup").style(Style::default().fg(Color::Green)),
        ListItem::new("2. Sight Reduction Tables").style(Style::default().fg(Color::Yellow)),
        ListItem::new("3. Sight Reduction Calculator").style(Style::default().fg(Color::Cyan)),
        ListItem::new("4. Automatic Fix Computation").style(Style::default().fg(Color::Blue)),
        ListItem::new("5. Sight Averaging").style(Style::default().fg(Color::LightBlue)),
        ListItem::new("6. Arc to Time Calculator").style(Style::default().fg(Color::LightGreen)),
        ListItem::new(""),
        ListItem::new("?. Help & Instructions").style(Style::default().fg(Color::Magenta)),
        ListItem::new("Q. Quit Application").style(Style::default().fg(Color::Red)),
    ];

    let menu = List::new(menu_items)
        .block(
            Block::default()
                .title(" Main Menu ")
                .title_alignment(Alignment::Center)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .style(Style::default().fg(Color::White));

    frame.render_widget(menu, center_area);
}




/// Renders the help screen
fn render_help_screen(frame: &mut Frame, area: Rect) {
    let text = vec![
        Line::from(""),
        Line::from(Span::styled(
            "Keyboard Shortcuts",
            Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled("Navigation:", Style::default().add_modifier(Modifier::UNDERLINED))),
        Line::from("  H       - Home screen"),
        Line::from("  A       - Almanac data lookup"),
        Line::from("  S       - Sight reduction tables"),
        Line::from("  C       - Sight reduction calculator"),
        Line::from("  V       - Sight averaging"),
        Line::from("  T       - Arc to time calculator"),
        Line::from("  ?       - This help screen"),
        Line::from("  Tab     - Next screen"),
        Line::from("  Shift+Tab - Previous screen"),
        Line::from("  1-6     - Quick select (from home)"),
        Line::from(""),
        Line::from(Span::styled("Almanac Screen:", Style::default().add_modifier(Modifier::UNDERLINED))),
        Line::from("  Tab     - Next field"),
        Line::from("  Enter   - Lookup almanac data"),
        Line::from("  Up/Down - Browse time (increment/decrement)"),
        Line::from("  +/-     - Change celestial body"),
        Line::from(""),
        Line::from(Span::styled("Sight Reduction Tables:", Style::default().add_modifier(Modifier::UNDERLINED))),
        Line::from("  Tab     - Next field"),
        Line::from("  Enter   - Compute Hc and Zn"),
        Line::from("  N/S     - Set latitude/declination direction"),
        Line::from(""),
        Line::from(Span::styled("Auto Compute (Multiple Sights):", Style::default().add_modifier(Modifier::UNDERLINED))),
        Line::from("  Tab     - Next field"),
        Line::from("  Enter   - Add sight to list"),
        Line::from("  C       - Compute fix from all sights"),
        Line::from("  V       - Toggle view/enter mode"),
        Line::from("  D       - Delete selected sight (in view mode)"),
        Line::from("  Up/Down - Select sight (in view mode)"),
        Line::from(""),
        Line::from(Span::styled("Calculator Screen:", Style::default().add_modifier(Modifier::UNDERLINED))),
        Line::from("  M       - Toggle Auto/Manual mode"),
        Line::from("  Tab     - Next field"),
        Line::from("  Enter   - Calculate"),
        Line::from("  +/-     - Change celestial body"),
        Line::from(""),
        Line::from(Span::styled("Sight Averaging:", Style::default().add_modifier(Modifier::UNDERLINED))),
        Line::from("  Tab     - Next field"),
        Line::from("  Enter   - Add observation"),
        Line::from("  D       - Delete last observation"),
        Line::from("  X       - Clear all observations"),
        Line::from(""),
        Line::from(Span::styled("Arc to Time Calculator:", Style::default().add_modifier(Modifier::UNDERLINED))),
        Line::from("  Tab     - Next field"),
        Line::from("  Enter   - Calculate conversion"),
        Line::from("  Del     - Clear all fields"),
        Line::from("  E/W     - Set longitude direction"),
        Line::from("  Space   - Toggle E/W direction"),
        Line::from(""),
        Line::from(Span::styled("General:", Style::default().add_modifier(Modifier::UNDERLINED))),
        Line::from("  Q       - Quit (from home) or Back"),
        Line::from("  Esc     - Return to home"),
        Line::from(""),
        Line::from(Span::styled(
            "About:",
            Style::default().add_modifier(Modifier::UNDERLINED),
        )),
        Line::from("  Celestial Navigation TUI v0.1.0"),
        Line::from("  A terminal-based celestial navigation calculator"),
        Line::from("  Built with Rust and ratatui"),
    ];

    let paragraph = Paragraph::new(text)
        .block(
            Block::default()
                .title(" Help & Instructions ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Magenta)),
        )
        .wrap(Wrap { trim: true });

    frame.render_widget(paragraph, area);
}
