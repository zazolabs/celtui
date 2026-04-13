// SPDX-License-Identifier: GPL-2.0-only
// SPDX-FileCopyrightText: Alexander Atanasov <alex@zazolabs.com>
//! Sight averaging screen
//!
//! This module provides a screen for entering multiple observations of the same
//! celestial body and calculating the average time and average sextant altitude.

use celtnav::sight_averaging::{SextantObservation, AveragedSight, average_sights, validate_altitude};
use chrono::NaiveTime;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame,
};
use serde::{Deserialize, Serialize};

/// Input field identifier for form navigation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputField {
    Time,
    SextantAltitude,
}

impl InputField {
    /// Get next field in the sequence
    pub fn next(&self) -> Self {
        match self {
            InputField::Time => InputField::SextantAltitude,
            InputField::SextantAltitude => InputField::Time,
        }
    }

    /// Get previous field in the sequence
    pub fn previous(&self) -> Self {
        match self {
            InputField::Time => InputField::SextantAltitude,
            InputField::SextantAltitude => InputField::Time,
        }
    }

    /// Returns the label for this field
    pub fn label(&self) -> &str {
        match self {
            InputField::Time => "Time (HH:MM:SS)",
            InputField::SextantAltitude => "Sextant Altitude [DD MM.M]",
        }
    }
}

/// A single observation entry (before conversion to library type)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservationEntry {
    pub time: String,
    pub altitude: String,  // "DD MM.M"
}

impl ObservationEntry {
    pub fn new() -> Self {
        Self {
            time: String::new(),
            altitude: String::new(),
        }
    }

    /// Try to convert to a SextantObservation for averaging
    pub fn to_sextant_observation(&self) -> Option<SextantObservation> {
        use crate::validation::parse_dms;

        let time = NaiveTime::parse_from_str(&self.time, "%H:%M:%S").ok()?;

        // Parse altitude using parse_dms
        let (degrees, minutes, seconds) = parse_dms(&self.altitude).ok()?;

        // Validate altitude range
        if !validate_altitude(degrees, minutes) {
            return None;
        }

        // Convert DMS to decimal for storage
        let altitude_decimal = celtnav::dms_to_decimal(degrees as i32, minutes as u32, seconds);
        let altitude_dms = celtnav::decimal_to_dms(altitude_decimal);

        Some(SextantObservation {
            time,
            altitude_degrees: altitude_dms.degrees as f64,
            altitude_minutes: altitude_dms.minutes as f64,
        })
    }

    /// Check if entry is complete and valid
    pub fn is_valid(&self) -> bool {
        self.to_sextant_observation().is_some()
    }
}

/// Averaging screen form state
pub struct AveragingForm {
    /// Current input field
    pub current_field: InputField,
    /// Current observation being entered
    pub current_observation: ObservationEntry,
    /// List of completed observations
    pub observations: Vec<ObservationEntry>,
    /// Currently selected observation index (for deletion)
    pub selected_index: Option<usize>,
    /// Calculated average (if available)
    pub averaged_sight: Option<AveragedSight>,
    /// Error message to display
    pub error_message: Option<String>,
    /// Success message to display
    pub message: Option<String>,
}

impl AveragingForm {
    pub fn new() -> Self {
        Self {
            current_field: InputField::Time,
            current_observation: ObservationEntry::new(),
            observations: Vec::new(),
            selected_index: None,
            averaged_sight: None,
            error_message: None,
            message: None,
        }
    }

    /// Add current observation to the list
    pub fn add_observation(&mut self) {
        if !self.current_observation.is_valid() {
            self.error_message = Some("Invalid observation. Check time format (HH:MM:SS) and altitude format (DD MM.M)".to_string());
            return;
        }

        self.observations.push(self.current_observation.clone());
        // Don't clear the form - keep data for easy entry of similar observations
        self.error_message = None;
        self.message = Some(format!("Observation {} added", self.observations.len()));

        // Recalculate average
        self.calculate_average();
    }

    /// Delete observation at index
    pub fn delete_observation(&mut self, index: usize) {
        if index < self.observations.len() {
            self.observations.remove(index);
            self.message = Some(format!("Observation {} deleted", index + 1));

            // Adjust selection after deletion
            if let Some(selected) = self.selected_index {
                if selected >= self.observations.len() && !self.observations.is_empty() {
                    self.selected_index = Some(self.observations.len() - 1);
                } else if self.observations.is_empty() {
                    self.selected_index = None;
                }
            }

            self.calculate_average();
        }
    }

    /// Delete selected observation (or last if none selected)
    pub fn delete_selected_observation(&mut self) {
        if self.observations.is_empty() {
            self.error_message = Some("No observations to delete".to_string());
            return;
        }

        let index = self.selected_index.unwrap_or(self.observations.len() - 1);
        self.delete_observation(index);
    }

    /// Select next observation (move down in list)
    pub fn select_next(&mut self) {
        if self.observations.is_empty() {
            self.selected_index = None;
            return;
        }

        self.selected_index = Some(match self.selected_index {
            None => 0,
            Some(i) if i >= self.observations.len() - 1 => self.observations.len() - 1,
            Some(i) => i + 1,
        });
    }

    /// Select previous observation (move up in list)
    pub fn select_previous(&mut self) {
        if self.observations.is_empty() {
            self.selected_index = None;
            return;
        }

        self.selected_index = Some(match self.selected_index {
            None => self.observations.len() - 1,
            Some(0) => 0,
            Some(i) => i - 1,
        });
    }

    /// Clear all observations
    pub fn clear_all(&mut self) {
        self.observations.clear();
        self.selected_index = None;
        self.averaged_sight = None;
        self.error_message = None;
        self.message = Some("All observations cleared".to_string());
    }

    /// Calculate average from all observations
    fn calculate_average(&mut self) {
        if self.observations.len() < 2 {
            self.averaged_sight = None;
            return;
        }

        let sextant_obs: Vec<SextantObservation> = self.observations
            .iter()
            .filter_map(|e| e.to_sextant_observation())
            .collect();

        if sextant_obs.len() < 2 {
            self.averaged_sight = None;
            self.error_message = Some("Need at least 2 valid observations to calculate average".to_string());
            return;
        }

        self.averaged_sight = average_sights(&sextant_obs);
    }

    /// Handle keyboard events
    pub fn handle_key_event(&mut self, key_event: KeyEvent) {
        // Clear messages on new key press
        self.message = None;

        match key_event.code {
            // Tab to move to next field
            KeyCode::Tab => {
                self.current_field = self.current_field.next();
            }
            // Shift+Tab to move to previous field
            KeyCode::BackTab => {
                self.current_field = self.current_field.previous();
            }
            // Arrow Up to select previous observation
            KeyCode::Up => {
                self.select_previous();
            }
            // Arrow Down to select next observation
            KeyCode::Down => {
                self.select_next();
            }
            // Enter to add observation
            KeyCode::Enter => {
                self.add_observation();
            }
            // 'd' or Delete to delete selected observation (or last if none selected)
            KeyCode::Char('d') | KeyCode::Char('D') | KeyCode::Delete => {
                self.delete_selected_observation();
            }
            // 'x' to clear all
            KeyCode::Char('x') | KeyCode::Char('X') => {
                self.clear_all();
            }
            // Backspace to delete character
            KeyCode::Backspace => {
                let field = match self.current_field {
                    InputField::Time => &mut self.current_observation.time,
                    InputField::SextantAltitude => &mut self.current_observation.altitude,
                };
                field.pop();
                self.error_message = None;
            }
            // Character input
            KeyCode::Char(c) => {
                let field = match self.current_field {
                    InputField::Time => &mut self.current_observation.time,
                    InputField::SextantAltitude => &mut self.current_observation.altitude,
                };

                // Allow numeric input and specific formatting characters
                match self.current_field {
                    InputField::Time => {
                        if c.is_ascii_digit() || c == ':' {
                            field.push(c);
                        }
                    }
                    InputField::SextantAltitude => {
                        if c.is_ascii_digit() || c == '.' || c == ' ' {
                            field.push(c);
                        }
                    }
                }
                self.error_message = None;
            }
            _ => {}
        }
    }

    /// Get the averaged sight (if available and exportable)
#[allow(dead_code)]    pub fn get_averaged_sight(&self) -> Option<&AveragedSight> {
        self.averaged_sight.as_ref()
    }
}

/// Render the averaging screen
pub fn render_averaging_screen(frame: &mut Frame, area: Rect, form: &AveragingForm) {
    // Split screen into sections
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(10), // Input form
            Constraint::Min(10),    // Observations list
            Constraint::Length(6),  // Average results
            Constraint::Length(3),  // Messages/help
        ])
        .split(area);

    // Render input form
    render_input_form(frame, chunks[0], form);

    // Render observations list
    render_observations_list(frame, chunks[1], form);

    // Render average results
    render_average_results(frame, chunks[2], form);

    // Render messages and help
    render_messages(frame, chunks[3], form);
}

fn render_input_form(frame: &mut Frame, area: Rect, form: &AveragingForm) {
    let block = Block::default()
        .title("Enter Observation (Press Enter to Add)")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let inner_area = block.inner(area);
    frame.render_widget(block, area);

    let form_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Length(2),
        ])
        .split(inner_area);

    // Render each input field
    render_field(
        frame,
        form_chunks[0],
        &form.current_observation.time,
        InputField::Time,
        form.current_field == InputField::Time,
    );

    render_field(
        frame,
        form_chunks[1],
        &form.current_observation.altitude,
        InputField::SextantAltitude,
        form.current_field == InputField::SextantAltitude,
    );
}

fn render_field(frame: &mut Frame, area: Rect, value: &str, field: InputField, is_active: bool) {
    let style = if is_active {
        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::White)
    };

    let text = format!("{}: {}", field.label(), value);
    let para = Paragraph::new(text).style(style);
    frame.render_widget(para, area);
}

fn render_observations_list(frame: &mut Frame, area: Rect, form: &AveragingForm) {
    let block = Block::default()
        .title(format!("Observations ({}) - Use ↑↓ to select, D to delete", form.observations.len()))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Green));

    if form.observations.is_empty() {
        let para = Paragraph::new("No observations yet. Enter observation data and press Enter to add.")
            .block(block)
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true });
        frame.render_widget(para, area);
        return;
    }

    let items: Vec<ListItem> = form.observations
        .iter()
        .enumerate()
        .map(|(i, obs)| {
            let is_selected = form.selected_index == Some(i);
            let content = format!(
                "{}. Time: {}  Altitude: {}",
                i + 1,
                obs.time,
                obs.altitude
            );

            let style = if is_selected {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
                    .add_modifier(Modifier::REVERSED)
            } else {
                Style::default().fg(Color::White)
            };

            ListItem::new(content).style(style)
        })
        .collect();

    let list = List::new(items).block(block);
    frame.render_widget(list, area);
}

fn render_average_results(frame: &mut Frame, area: Rect, form: &AveragingForm) {
    let block = Block::default()
        .title("Averaged Sight")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Magenta));

    if form.observations.len() < 2 {
        let para = Paragraph::new("Need at least 2 observations to calculate average")
            .block(block)
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::DarkGray));
        frame.render_widget(para, area);
        return;
    }

    let text = if let Some(avg) = &form.averaged_sight {
        // Convert averaged altitude to DM (decimal minutes)
        let avg_decimal = avg.avg_altitude_degrees + avg.avg_altitude_minutes / 60.0;
        let avg_dms = celtnav::decimal_to_dms(avg_decimal);

        format!(
            "Average Time: {}\nAverage Altitude: {}° {:05.2}'",
            avg.avg_time.format("%H:%M:%S"),
            avg_dms.degrees,
            avg_dms.minutes
        )
    } else {
        "Error calculating average (check that all observations are valid)".to_string()
    };

    let para = Paragraph::new(text)
        .block(block)
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD));

    frame.render_widget(para, area);
}

fn render_messages(frame: &mut Frame, area: Rect, form: &AveragingForm) {
    let mut lines = vec![
        Line::from(vec![
            Span::styled("Tab", Style::default().fg(Color::Yellow)),
            Span::raw(": Next Field | "),
            Span::styled("↑↓", Style::default().fg(Color::Yellow)),
            Span::raw(": Select | "),
            Span::styled("Enter", Style::default().fg(Color::Yellow)),
            Span::raw(": Add | "),
            Span::styled("D", Style::default().fg(Color::Yellow)),
            Span::raw(": Delete | "),
            Span::styled("X", Style::default().fg(Color::Yellow)),
            Span::raw(": Clear All"),
        ]),
    ];

    // Add error or success message
    if let Some(err) = &form.error_message {
        lines.push(Line::from(Span::styled(
            err,
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        )));
    } else if let Some(msg) = &form.message {
        lines.push(Line::from(Span::styled(
            msg,
            Style::default().fg(Color::Green),
        )));
    }

    let para = Paragraph::new(lines)
        .alignment(Alignment::Center);

    frame.render_widget(para, area);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_form_creation() {
        let form = AveragingForm::new();
        assert_eq!(form.current_field, InputField::Time);
        assert!(form.observations.is_empty());
        assert!(form.averaged_sight.is_none());
    }

    #[test]
    fn test_observation_entry_validation() {
        let mut entry = ObservationEntry::new();
        entry.time = "10:30:00".to_string();
        entry.altitude = "45 30 15.5".to_string();

        assert!(entry.is_valid());

        // Invalid time
        entry.time = "invalid".to_string();
        assert!(!entry.is_valid());
    }

    #[test]
    fn test_add_observation() {
        let mut form = AveragingForm::new();
        form.current_observation.time = "10:30:00".to_string();
        form.current_observation.altitude = "45 30 15.5".to_string();

        form.add_observation();
        assert_eq!(form.observations.len(), 1);
    }

    #[test]
    fn test_averaging_calculation() {
        let mut form = AveragingForm::new();

        // Add first observation
        form.current_observation.time = "10:30:00".to_string();
        form.current_observation.altitude = "45 30 0".to_string();
        form.add_observation();

        // Should not have average yet (need 2)
        assert!(form.averaged_sight.is_none());

        // Add second observation
        form.current_observation.time = "10:32:00".to_string();
        form.current_observation.altitude = "45 40 0".to_string();
        form.add_observation();

        // Now should have average
        assert!(form.averaged_sight.is_some());
        let avg = form.averaged_sight.as_ref().unwrap();
        assert_eq!(avg.avg_altitude_degrees, 45.0);
        assert!((avg.avg_altitude_minutes - 35.0).abs() < 0.01);
    }

    #[test]
    fn test_default_altitude_is_empty() {
        let entry = ObservationEntry::new();
        assert_eq!(entry.altitude, "");
    }

    #[test]
    fn test_preserve_data_after_adding() {
        let mut form = AveragingForm::new();

        // Enter observation data
        form.current_observation.time = "10:30:00".to_string();
        form.current_observation.altitude = "45 30 15.5".to_string();

        // Add observation
        form.add_observation();

        // Verify observation was added
        assert_eq!(form.observations.len(), 1);

        // Verify form data was NOT cleared
        assert_eq!(form.current_observation.time, "10:30:00");
        assert_eq!(form.current_observation.altitude, "45 30 15.5");
    }

    #[test]
    fn test_select_next_observation() {
        let mut form = AveragingForm::new();

        // Initially no selection
        assert_eq!(form.selected_index, None);

        // Add three observations
        for i in 0..3 {
            form.current_observation.time = format!("10:3{}:00", i);
            form.current_observation.altitude = "45 30 0".to_string();
            form.add_observation();
        }

        // Select next should select first
        form.select_next();
        assert_eq!(form.selected_index, Some(0));

        // Select next should move to second
        form.select_next();
        assert_eq!(form.selected_index, Some(1));

        // Select next should move to third
        form.select_next();
        assert_eq!(form.selected_index, Some(2));

        // Select next should stay at last
        form.select_next();
        assert_eq!(form.selected_index, Some(2));
    }

    #[test]
    fn test_select_previous_observation() {
        let mut form = AveragingForm::new();

        // Add three observations
        for i in 0..3 {
            form.current_observation.time = format!("10:3{}:00", i);
            form.current_observation.altitude = "45 30 0".to_string();
            form.add_observation();
        }

        // Select previous from none should select last
        form.select_previous();
        assert_eq!(form.selected_index, Some(2));

        // Select previous should move to second
        form.select_previous();
        assert_eq!(form.selected_index, Some(1));

        // Select previous should move to first
        form.select_previous();
        assert_eq!(form.selected_index, Some(0));

        // Select previous should stay at first
        form.select_previous();
        assert_eq!(form.selected_index, Some(0));
    }

    #[test]
    fn test_delete_selected_observation() {
        let mut form = AveragingForm::new();

        // Add three observations
        for i in 0..3 {
            form.current_observation.time = format!("10:3{}:00", i);
            form.current_observation.altitude = format!("45 {} 0", 30 + i);
            form.add_observation();
        }

        assert_eq!(form.observations.len(), 3);

        // Select second observation (index 1)
        form.selected_index = Some(1);

        // Delete it
        form.delete_selected_observation();

        // Should have 2 observations left
        assert_eq!(form.observations.len(), 2);

        // Verify the middle one was deleted (check altitude)
        assert_eq!(form.observations[0].altitude, "45 30 0");
        assert_eq!(form.observations[1].altitude, "45 32 0");
    }

    #[test]
    fn test_delete_last_when_none_selected() {
        let mut form = AveragingForm::new();

        // Add two observations
        for i in 0..2 {
            form.current_observation.time = format!("10:3{}:00", i);
            form.current_observation.altitude = format!("45 {} 0", 30 + i);
            form.add_observation();
        }

        assert_eq!(form.observations.len(), 2);

        // No selection
        form.selected_index = None;

        // Delete should delete last
        form.delete_selected_observation();

        // Should have 1 observation left
        assert_eq!(form.observations.len(), 1);
        assert_eq!(form.observations[0].altitude, "45 30 0");
    }

    #[test]
    fn test_selection_adjustment_after_deletion() {
        let mut form = AveragingForm::new();

        // Add three observations
        for i in 0..3 {
            form.current_observation.time = format!("10:3{}:00", i);
            form.current_observation.altitude = "45 30 0".to_string();
            form.add_observation();
        }

        // Select last observation (index 2)
        form.selected_index = Some(2);

        // Delete it
        form.delete_selected_observation();

        // Selection should adjust to new last (index 1)
        assert_eq!(form.selected_index, Some(1));
    }

    #[test]
    fn test_selection_cleared_when_all_deleted() {
        let mut form = AveragingForm::new();

        // Add one observation
        form.current_observation.time = "10:30:00".to_string();
        form.current_observation.altitude = "45 30 0".to_string();
        form.add_observation();

        // Select it
        form.selected_index = Some(0);

        // Delete it
        form.delete_selected_observation();

        // Selection should be cleared
        assert_eq!(form.selected_index, None);
    }

    #[test]
    fn test_arrow_key_handling() {
        let mut form = AveragingForm::new();

        // Add observations
        for i in 0..2 {
            form.current_observation.time = format!("10:3{}:00", i);
            form.current_observation.altitude = "45 30 0".to_string();
            form.add_observation();
        }

        // Test down arrow
        form.handle_key_event(KeyEvent::from(KeyCode::Down));
        assert_eq!(form.selected_index, Some(0));

        form.handle_key_event(KeyEvent::from(KeyCode::Down));
        assert_eq!(form.selected_index, Some(1));

        // Test up arrow
        form.handle_key_event(KeyEvent::from(KeyCode::Up));
        assert_eq!(form.selected_index, Some(0));
    }
}
