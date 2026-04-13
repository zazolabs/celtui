//! Sight Reduction Tables screen
//!
//! This module provides a screen that emulates the function of sight reduction tables
//! (e.g., Pub. 249/229), computing Hc and Zn for a given LHA, Declination, and Latitude.

use celtnav::sight_reduction::{compute_altitude, compute_azimuth, SightData};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Row, Table, Wrap},
    Frame,
};

/// Input field identifier for form navigation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SightReductionInputField {
    LHA,                  // Single field: "DD MM.M"
    Declination,          // Single field: "DD MM.M"
    DeclinationDirection, // N/S
    Latitude,             // Single field: "DD MM.M"
    LatitudeDirection,    // N/S
}

impl SightReductionInputField {
    /// Returns all fields
    pub fn all() -> Vec<SightReductionInputField> {
        vec![
            SightReductionInputField::LHA,
            SightReductionInputField::Declination,
            SightReductionInputField::DeclinationDirection,
            SightReductionInputField::Latitude,
            SightReductionInputField::LatitudeDirection,
        ]
    }

    /// Get next field in the sequence
    pub fn next(&self) -> Self {
        let fields = Self::all();
        let current_idx = fields.iter().position(|f| f == self).unwrap_or(0);
        let next_idx = (current_idx + 1) % fields.len();
        fields[next_idx]
    }

    /// Get previous field in the sequence
    pub fn previous(&self) -> Self {
        let fields = Self::all();
        let current_idx = fields.iter().position(|f| f == self).unwrap_or(0);
        let prev_idx = if current_idx == 0 {
            fields.len() - 1
        } else {
            current_idx - 1
        };
        fields[prev_idx]
    }

    /// Returns the label for this field
    pub fn label(&self) -> &str {
        match self {
            SightReductionInputField::LHA => "LHA [DD MM.M]",
            SightReductionInputField::Declination => "Declination [DD MM.M]",
            SightReductionInputField::DeclinationDirection => "Declination (N/S)",
            SightReductionInputField::Latitude => "Latitude [DD MM.M]",
            SightReductionInputField::LatitudeDirection => "Latitude (N/S)",
        }
    }
}

/// Sight reduction computation result
#[derive(Debug, Clone, Copy)]
pub struct SightReductionResult {
    pub computed_altitude: f64, // Hc in degrees
    pub azimuth: f64,            // Zn in degrees
    pub lha: f64,                // LHA in degrees
    pub declination: f64,        // Declination in degrees
    pub latitude: f64,           // Latitude in degrees
}

/// Sight reduction screen form data
#[derive(Debug, Clone)]
pub struct SightReductionForm {
    pub lha: String,                 // "DD MM.M"
    pub declination: String,         // "DD MM.M"
    pub declination_direction: char, // 'N' or 'S'
    pub latitude: String,            // "DD MM.M"
    pub latitude_direction: char,    // 'N' or 'S'
    pub current_field: SightReductionInputField,
    pub result: Option<SightReductionResult>,
    pub error_message: Option<String>,
}

impl SightReductionForm {
    /// Create a new sight reduction form with default values
    pub fn new() -> Self {
        Self {
            lha: String::new(),
            declination: String::new(),
            declination_direction: 'N',
            latitude: String::new(),
            latitude_direction: 'N',
            current_field: SightReductionInputField::LHA,
            result: None,
            error_message: None,
        }
    }

    /// Move to next input field
    pub fn next_field(&mut self) {
        self.current_field = self.current_field.next();
    }

    /// Move to previous input field
    pub fn previous_field(&mut self) {
        self.current_field = self.current_field.previous();
    }

    /// Get the current value for an input field
    pub fn get_field_value(&self, field: SightReductionInputField) -> String {
        match field {
            SightReductionInputField::LHA => self.lha.clone(),
            SightReductionInputField::Declination => self.declination.clone(),
            SightReductionInputField::DeclinationDirection => self.declination_direction.to_string(),
            SightReductionInputField::Latitude => self.latitude.clone(),
            SightReductionInputField::LatitudeDirection => self.latitude_direction.to_string(),
        }
    }

    /// Set the value for an input field
    pub fn set_field_value(&mut self, field: SightReductionInputField, value: String) {
        match field {
            SightReductionInputField::LHA => self.lha = value,
            SightReductionInputField::Declination => self.declination = value,
            SightReductionInputField::DeclinationDirection => {
                if let Some(c) = value.chars().next() {
                    if c == 'N' || c == 'S' || c == 'n' || c == 's' {
                        self.declination_direction = c.to_ascii_uppercase();
                    }
                }
            }
            SightReductionInputField::Latitude => self.latitude = value,
            SightReductionInputField::LatitudeDirection => {
                if let Some(c) = value.chars().next() {
                    if c == 'N' || c == 'S' || c == 'n' || c == 's' {
                        self.latitude_direction = c.to_ascii_uppercase();
                    }
                }
            }
        }
    }

    /// Validate and compute sight reduction
    pub fn compute(&mut self) {
        use crate::validation::parse_dms;

        self.result = None;
        self.error_message = None;

        // Parse LHA using new DMS parser
        let (lha_deg, lha_min, lha_sec) = match parse_dms(&self.lha) {
            Ok(v) => v,
            Err(e) => {
                self.error_message = Some(format!("LHA: {}", e));
                return;
            }
        };

        let lha = celtnav::dms_to_decimal(lha_deg as i32, lha_min as u32, lha_sec);

        // Parse Declination using new DMS parser
        let (dec_deg, dec_min, dec_sec) = match parse_dms(&self.declination) {
            Ok(v) => v,
            Err(e) => {
                self.error_message = Some(format!("Declination: {}", e));
                return;
            }
        };

        let declination = if self.declination_direction == 'S' {
            -celtnav::dms_to_decimal(dec_deg as i32, dec_min as u32, dec_sec)
        } else {
            celtnav::dms_to_decimal(dec_deg as i32, dec_min as u32, dec_sec)
        };

        // Parse Latitude using new DMS parser
        let (lat_deg, lat_min, lat_sec) = match parse_dms(&self.latitude) {
            Ok(v) => v,
            Err(e) => {
                self.error_message = Some(format!("Latitude: {}", e));
                return;
            }
        };

        let latitude = if self.latitude_direction == 'S' {
            -celtnav::dms_to_decimal(lat_deg as i32, lat_min as u32, lat_sec)
        } else {
            celtnav::dms_to_decimal(lat_deg as i32, lat_min as u32, lat_sec)
        };

        // Create sight data
        let sight_data = SightData {
            latitude,
            declination,
            local_hour_angle: lha,
        };

        // Compute altitude and azimuth
        let hc = compute_altitude(&sight_data);
        let zn = compute_azimuth(&sight_data);

        self.result = Some(SightReductionResult {
            computed_altitude: hc,
            azimuth: zn,
            lha,
            declination,
            latitude,
        });
    }

    /// Handle keyboard events
    pub fn handle_key_event(&mut self, key_event: KeyEvent) {
        match key_event.code {
            KeyCode::Tab => self.next_field(),
            KeyCode::BackTab => self.previous_field(),
            KeyCode::Enter => self.compute(),
            KeyCode::Char(c) => {
                match self.current_field {
                    SightReductionInputField::DeclinationDirection => {
                        if c == 'N' || c == 'n' || c == 'S' || c == 's' {
                            self.set_field_value(
                                SightReductionInputField::DeclinationDirection,
                                c.to_string(),
                            );
                        }
                    }
                    SightReductionInputField::LatitudeDirection => {
                        if c == 'N' || c == 'n' || c == 'S' || c == 's' {
                            self.set_field_value(
                                SightReductionInputField::LatitudeDirection,
                                c.to_string(),
                            );
                        }
                    }
                    _ => {
                        // Add character to current field value
                        let mut value = self.get_field_value(self.current_field);
                        value.push(c);
                        self.set_field_value(self.current_field, value);
                    }
                }
            }
            KeyCode::Backspace => {
                // Remove last character from current field
                let mut value = self.get_field_value(self.current_field);
                value.pop();
                self.set_field_value(self.current_field, value);
            }
            _ => {}
        }
    }
}

impl Default for SightReductionForm {
    fn default() -> Self {
        Self::new()
    }
}

/// Render the sight reduction screen
pub fn render_sight_reduction_screen(frame: &mut Frame, area: Rect, form: &SightReductionForm) {
    // Create main layout: form on left, results on right
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(50), // Form area
            Constraint::Percentage(50), // Results area
        ])
        .split(area);

    render_form(frame, chunks[0], form);
    render_results(frame, chunks[1], form);
}

/// Render the input form
fn render_form(frame: &mut Frame, area: Rect, form: &SightReductionForm) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),    // Form fields
            Constraint::Length(3), // Help text
        ])
        .split(area);

    // Render form fields
    render_form_fields(frame, chunks[0], form);

    // Render help text
    let help_text = "Tab: Next Field | Enter: Compute | N/S: Direction";
    let help_widget = Paragraph::new(help_text)
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Center);
    frame.render_widget(help_widget, chunks[1]);
}

/// Render the form fields
fn render_form_fields(frame: &mut Frame, area: Rect, form: &SightReductionForm) {
    let fields = SightReductionInputField::all();
    let mut lines = vec![Line::from("")];

    for field in fields {
        let value = form.get_field_value(field);
        let is_current = field == form.current_field;

        let label_style = if is_current {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };

        let value_style = if is_current {
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
        } else {
            Style::default().fg(Color::Gray)
        };

        let cursor = if is_current { "► " } else { "  " };

        lines.push(Line::from(vec![
            Span::styled(cursor, Style::default().fg(Color::Yellow)),
            Span::styled(format!("{}: ", field.label()), label_style),
            Span::styled(value, value_style),
        ]));
    }

    let paragraph = Paragraph::new(lines)
        .block(
            Block::default()
                .title(" Sight Reduction Input ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow)),
        )
        .wrap(Wrap { trim: false });

    frame.render_widget(paragraph, area);
}

/// Render the results panel
fn render_results(frame: &mut Frame, area: Rect, form: &SightReductionForm) {
    if let Some(error) = &form.error_message {
        let mut lines = vec![Line::from("")];
        lines.push(Line::from(Span::styled(
            "Error:",
            Style::default()
                .fg(Color::Red)
                .add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(error, Style::default().fg(Color::Red))));

        let paragraph = Paragraph::new(lines)
            .block(
                Block::default()
                    .title(" Results ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Blue)),
            )
            .wrap(Wrap { trim: false });

        frame.render_widget(paragraph, area);
    } else if let Some(result) = &form.result {
        // Format Hc using DM (decimal minutes)
        let hc_dms = celtnav::decimal_to_dms(result.computed_altitude);
        let hc_formatted = format!("{}° {:05.2}'", hc_dms.degrees, hc_dms.minutes);

        // Format Zn using DM (decimal minutes)
        let zn_dms = celtnav::decimal_to_dms(result.azimuth);
        let zn_formatted = format!("{}° {:05.2}'", zn_dms.degrees, zn_dms.minutes);

        // Create table rows
        let rows = vec![
            Row::new(vec!["Computed Altitude (Hc)".to_string(), hc_formatted]),
            Row::new(vec!["Azimuth (Zn)".to_string(), zn_formatted]),
        ];

        let table = Table::new(rows, [Constraint::Percentage(60), Constraint::Percentage(40)])
            .header(
                Row::new(vec!["Parameter", "Value"])
                    .style(
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    )
                    .bottom_margin(1),
            )
            .block(
                Block::default()
                    .title(" Sight Reduction Results ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Blue)),
            )
            .style(Style::default().fg(Color::White))
            .column_spacing(2);

        frame.render_widget(table, area);
    } else {
        let lines = vec![
            Line::from(""),
            Line::from(Span::styled(
                "Enter LHA, Declination, and Latitude",
                Style::default()
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::ITALIC),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "Then press Enter to compute Hc and Zn",
                Style::default()
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::ITALIC),
            )),
        ];

        let paragraph = Paragraph::new(lines)
            .block(
                Block::default()
                    .title(" Results ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Blue)),
            )
            .wrap(Wrap { trim: false });

        frame.render_widget(paragraph, area);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sight_reduction_form_creation() {
        let form = SightReductionForm::new();
        assert_eq!(form.current_field, SightReductionInputField::LHA);
        assert_eq!(form.declination_direction, 'N');
        assert_eq!(form.latitude_direction, 'N');
        assert!(form.result.is_none());
    }

    #[test]
    fn test_field_navigation() {
        let mut form = SightReductionForm::new();
        form.current_field = SightReductionInputField::LHA;
        form.next_field();
        assert_eq!(form.current_field, SightReductionInputField::Declination);
    }

    #[test]
    fn test_direction_setting() {
        let mut form = SightReductionForm::new();
        form.set_field_value(SightReductionInputField::LatitudeDirection, "s".to_string());
        assert_eq!(form.latitude_direction, 'S');
        form.set_field_value(SightReductionInputField::DeclinationDirection, "N".to_string());
        assert_eq!(form.declination_direction, 'N');
    }

    #[test]
    fn test_compute_valid_input() {
        let mut form = SightReductionForm::new();
        form.lha = "30 0".to_string();
        form.declination = "20 0".to_string();
        form.declination_direction = 'N';
        form.latitude = "40 0".to_string();
        form.latitude_direction = 'N';

        form.compute();

        assert!(form.result.is_some());
        assert!(form.error_message.is_none());
    }

    #[test]
    fn test_compute_missing_field() {
        let mut form = SightReductionForm::new();
        // Leave LHA empty
        form.compute();

        assert!(form.result.is_none());
        assert!(form.error_message.is_some());
    }

    #[test]
    fn test_compute_with_south_declination() {
        let mut form = SightReductionForm::new();
        form.lha = "30 0".to_string();
        form.declination = "20 0".to_string();
        form.declination_direction = 'S'; // Southern declination
        form.latitude = "40 0".to_string();
        form.latitude_direction = 'N';

        form.compute();

        assert!(form.result.is_some());
        let result = form.result.as_ref().unwrap();
        assert!(result.declination < 0.0); // Should be negative for south
    }
}
