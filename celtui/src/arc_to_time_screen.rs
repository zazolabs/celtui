//! Arc to Time Calculator screen
//!
//! This module provides a screen for converting longitude (arc) to time offset from UTC.
//! Used for converting longitude to local time offset and meridian passage time from LMT to UTC.
//!
//! Conversion rules:
//! - 15° longitude = 1 hour
//! - 15' arc = 1 minute of time
//! - 15" arc = 1 second of time
//! - East longitude: ahead of UTC (+)
//! - West longitude: behind UTC (-)

use chrono::{NaiveTime, Timelike};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Row, Table, Wrap},
    Frame,
};

use crate::validation::{parse_dm, validate_dm_angle, validate_time};

/// Input field identifier for form navigation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArcToTimeInputField {
    Longitude,
    LongitudeDirection,
    MeridianPassageTime,
}

impl ArcToTimeInputField {
    /// Returns all fields
    pub fn all() -> Vec<ArcToTimeInputField> {
        vec![
            ArcToTimeInputField::Longitude,
            ArcToTimeInputField::LongitudeDirection,
            ArcToTimeInputField::MeridianPassageTime,
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
            ArcToTimeInputField::Longitude => "Longitude (DD MM.M)",
            ArcToTimeInputField::LongitudeDirection => "Direction (E/W)",
            ArcToTimeInputField::MeridianPassageTime => "Mer Pass LMT (HH:MM:SS)",
        }
    }
}

/// Result of arc to time conversion
#[derive(Debug, Clone)]
pub struct ArcToTimeResult {
    pub longitude_degrees: f64,
    pub longitude_minutes: f64,
    pub direction: char,
    pub offset_hours: i32,
    pub offset_minutes: i32,
    pub offset_seconds: i32,
    pub mer_pass_lmt: Option<String>,
    pub mer_pass_utc: Option<String>,
}

/// Arc to Time screen form data
#[derive(Debug, Clone)]
pub struct ArcToTimeForm {
    pub longitude: String,
    pub longitude_direction: char,
    pub meridian_passage_time: String,
    pub current_field: ArcToTimeInputField,
    pub result: Option<ArcToTimeResult>,
    pub error_message: Option<String>,
}

impl ArcToTimeForm {
    /// Create a new arc to time form with default values
    pub fn new() -> Self {
        Self {
            longitude: String::new(),
            longitude_direction: 'W',
            meridian_passage_time: String::new(),
            current_field: ArcToTimeInputField::Longitude,
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
    pub fn get_field_value(&self, field: ArcToTimeInputField) -> String {
        match field {
            ArcToTimeInputField::Longitude => self.longitude.clone(),
            ArcToTimeInputField::LongitudeDirection => self.longitude_direction.to_string(),
            ArcToTimeInputField::MeridianPassageTime => self.meridian_passage_time.clone(),
        }
    }

    /// Set the value for an input field
    pub fn set_field_value(&mut self, field: ArcToTimeInputField, value: String) {
        match field {
            ArcToTimeInputField::Longitude => self.longitude = value,
            ArcToTimeInputField::LongitudeDirection => {
                if let Some(c) = value.chars().next() {
                    let upper = c.to_ascii_uppercase();
                    if upper == 'E' || upper == 'W' {
                        self.longitude_direction = upper;
                    }
                }
            }
            ArcToTimeInputField::MeridianPassageTime => self.meridian_passage_time = value,
        }
    }

    /// Toggle longitude direction between E and W
    pub fn toggle_direction(&mut self) {
        self.longitude_direction = if self.longitude_direction == 'E' {
            'W'
        } else {
            'E'
        };
    }

    /// Clear all fields
    pub fn clear(&mut self) {
        self.longitude.clear();
        self.longitude_direction = 'W';
        self.meridian_passage_time.clear();
        self.result = None;
        self.error_message = None;
    }

    /// Convert longitude to time offset
    ///
    /// Returns (hours, minutes, seconds) with sign based on E/W direction
    /// East is positive (ahead of UTC), West is negative (behind UTC)
    pub fn longitude_to_time_offset(
        longitude_deg: f64,
        longitude_min: f64,
        direction: char,
    ) -> (i32, i32, i32) {
        // Convert longitude to decimal degrees
        let total_degrees = longitude_deg + longitude_min / 60.0;

        // Convert degrees to time: 15° = 1 hour
        let total_hours = total_degrees / 15.0;

        // Convert to total seconds for accurate calculation
        let total_seconds = (total_hours * 3600.0).round() as i32;

        // Extract hours, minutes, seconds
        let hours = total_seconds / 3600;
        let remaining_seconds = total_seconds % 3600;
        let minutes = remaining_seconds / 60;
        let seconds = remaining_seconds % 60;

        // Apply sign based on direction
        // East is ahead of UTC (+), West is behind UTC (-)
        let sign = if direction == 'E' { 1 } else { -1 };

        (sign * hours, sign * minutes, sign * seconds)
    }

    /// Apply time offset to a local time to get UTC
    ///
    /// For meridian passage: LMT - longitude_offset = UTC
    /// (subtract because we're converting FROM local TO UTC)
    pub fn apply_time_offset(
        local_time: &str,
        offset_hours: i32,
        offset_minutes: i32,
        offset_seconds: i32,
    ) -> Result<String, String> {
        // Parse the local time
        let time = NaiveTime::parse_from_str(local_time, "%H:%M:%S")
            .or_else(|_| NaiveTime::parse_from_str(local_time, "%H:%M"))
            .map_err(|_| "Invalid time format. Use HH:MM:SS or HH:MM".to_string())?;

        // Convert offset to total seconds
        let offset_total_seconds =
            offset_hours * 3600 + offset_minutes * 60 + offset_seconds;

        // Convert time to seconds since midnight
        let time_seconds = time.num_seconds_from_midnight() as i64;

        // Apply offset (subtract for LMT -> UTC conversion)
        // If we're at 12:00 LMT at 45°W (-03:00), UTC is 12:00 - (-03:00) = 15:00
        let utc_seconds = time_seconds - offset_total_seconds as i64;

        // Handle day wraparound
        let utc_seconds_normalized = if utc_seconds < 0 {
            utc_seconds + 86400
        } else if utc_seconds >= 86400 {
            utc_seconds - 86400
        } else {
            utc_seconds
        };

        // Convert back to time
        let utc_time = NaiveTime::from_num_seconds_from_midnight_opt(
            utc_seconds_normalized as u32,
            0,
        )
        .ok_or("Failed to create UTC time".to_string())?;

        Ok(utc_time.format("%H:%M:%S").to_string())
    }

    /// Calculate arc to time conversion
    pub fn calculate(&mut self) {
        self.result = None;
        self.error_message = None;

        // Validate longitude
        if let Err(e) = validate_dm_angle(&self.longitude, 0.0, 180.0, "Longitude") {
            self.error_message = Some(e);
            return;
        }

        // Parse longitude
        let (lon_deg, lon_min) = match parse_dm(&self.longitude) {
            Ok((d, m)) => (d, m),
            Err(e) => {
                self.error_message = Some(format!("Longitude: {}", e));
                return;
            }
        };

        // Validate meridian passage time if provided
        if !self.meridian_passage_time.is_empty() {
            if let Err(e) = validate_time(&self.meridian_passage_time) {
                self.error_message = Some(e);
                return;
            }
        }

        // Calculate time offset
        let (offset_hours, offset_minutes, offset_seconds) =
            Self::longitude_to_time_offset(lon_deg, lon_min, self.longitude_direction);

        // Calculate UTC meridian passage if LMT provided
        let (mer_pass_lmt, mer_pass_utc) = if !self.meridian_passage_time.is_empty() {
            match Self::apply_time_offset(
                &self.meridian_passage_time,
                offset_hours,
                offset_minutes,
                offset_seconds,
            ) {
                Ok(utc_time) => (
                    Some(self.meridian_passage_time.clone()),
                    Some(utc_time),
                ),
                Err(e) => {
                    self.error_message = Some(e);
                    return;
                }
            }
        } else {
            (None, None)
        };

        self.result = Some(ArcToTimeResult {
            longitude_degrees: lon_deg,
            longitude_minutes: lon_min,
            direction: self.longitude_direction,
            offset_hours,
            offset_minutes,
            offset_seconds,
            mer_pass_lmt,
            mer_pass_utc,
        });
    }

    /// Handle keyboard events
    pub fn handle_key_event(&mut self, key_event: KeyEvent) {
        match key_event.code {
            KeyCode::Tab => self.next_field(),
            KeyCode::BackTab => self.previous_field(),
            KeyCode::Enter => self.calculate(),
            KeyCode::Delete => self.clear(),
            KeyCode::Char(' ') if self.current_field == ArcToTimeInputField::LongitudeDirection => {
                self.toggle_direction();
            }
            KeyCode::Char('E') | KeyCode::Char('e')
                if self.current_field == ArcToTimeInputField::LongitudeDirection =>
            {
                self.longitude_direction = 'E';
            }
            KeyCode::Char('W') | KeyCode::Char('w')
                if self.current_field == ArcToTimeInputField::LongitudeDirection =>
            {
                self.longitude_direction = 'W';
            }
            KeyCode::Char(c) => {
                if self.current_field != ArcToTimeInputField::LongitudeDirection {
                    let mut value = self.get_field_value(self.current_field);
                    value.push(c);
                    self.set_field_value(self.current_field, value);
                }
            }
            KeyCode::Backspace => {
                if self.current_field != ArcToTimeInputField::LongitudeDirection {
                    let mut value = self.get_field_value(self.current_field);
                    value.pop();
                    self.set_field_value(self.current_field, value);
                }
            }
            _ => {}
        }
    }
}

impl Default for ArcToTimeForm {
    fn default() -> Self {
        Self::new()
    }
}

/// Render the arc to time screen
pub fn render_arc_to_time_screen(frame: &mut Frame, area: Rect, form: &ArcToTimeForm) {
    // Create main layout: form on left, results on right
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(40), // Form area
            Constraint::Percentage(60), // Results area
        ])
        .split(area);

    render_form(frame, chunks[0], form);
    render_results(frame, chunks[1], form);
}

/// Render the input form
fn render_form(frame: &mut Frame, area: Rect, form: &ArcToTimeForm) {
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
    let help_text = "Tab: Next | Enter: Calculate | Del: Clear | Space/E/W: Toggle Direction";
    let help_widget = Paragraph::new(help_text)
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Center);
    frame.render_widget(help_widget, chunks[1]);
}

/// Render the form fields
fn render_form_fields(frame: &mut Frame, area: Rect, form: &ArcToTimeForm) {
    let fields = ArcToTimeInputField::all();
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
                .title(" Arc to Time Calculator ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Green)),
        )
        .wrap(Wrap { trim: false });

    frame.render_widget(paragraph, area);
}

/// Render the results panel
fn render_results(frame: &mut Frame, area: Rect, form: &ArcToTimeForm) {
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
        // Create vertical layout for header and table
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(4), // Header with explanation
                Constraint::Min(0),    // Table
            ])
            .split(area);

        // Render header with explanation
        let header_text = vec![
            Line::from(Span::styled(
                "Conversion: 15° = 1 hour",
                Style::default().fg(Color::Cyan),
            )),
            Line::from(Span::styled(
                format!(
                    "Longitude: {}° {:05.2}' {}",
                    result.longitude_degrees as i32,
                    result.longitude_minutes,
                    result.direction
                ),
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            )),
        ];
        let header = Paragraph::new(header_text)
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Blue)),
            );
        frame.render_widget(header, chunks[0]);

        // Format UTC offset
        let offset_sign = if result.direction == 'E' { "+" } else { "-" };
        let offset_str = format!(
            "{}{:02}:{:02}:{:02}",
            offset_sign,
            result.offset_hours.abs(),
            result.offset_minutes.abs(),
            result.offset_seconds.abs()
        );

        // Create table rows
        let mut rows = vec![
            Row::new(vec![
                "Arc to Time".to_string(),
                format!(
                    "{:02}:{:02}:{:02}",
                    result.offset_hours.abs(),
                    result.offset_minutes.abs(),
                    result.offset_seconds.abs()
                ),
            ]),
            Row::new(vec![
                "UTC Offset".to_string(),
                format!("{} ({})", offset_str, if result.direction == 'E' { "East ahead" } else { "West behind" }),
            ]),
        ];

        // Add meridian passage times if calculated
        if let (Some(lmt), Some(utc)) = (&result.mer_pass_lmt, &result.mer_pass_utc) {
            rows.push(Row::new(vec!["".to_string(), "".to_string()])); // Blank row
            rows.push(Row::new(vec![
                "Mer Pass (LMT)".to_string(),
                lmt.clone(),
            ]));
            rows.push(Row::new(vec![
                "Mer Pass (UTC)".to_string(),
                utc.clone(),
            ]));
        }

        let table = Table::new(
            rows,
            [Constraint::Percentage(40), Constraint::Percentage(60)],
        )
        .header(
            Row::new(vec!["Item", "Value"])
                .style(
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                )
                .bottom_margin(1),
        )
        .block(
            Block::default()
                .title(" Conversion Results ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Blue)),
        )
        .style(Style::default().fg(Color::White))
        .column_spacing(2);

        frame.render_widget(table, chunks[1]);
    } else {
        let lines = vec![
            Line::from(""),
            Line::from(Span::styled(
                "Enter longitude and optionally meridian passage time (LMT),",
                Style::default()
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::ITALIC),
            )),
            Line::from(Span::styled(
                "then press Enter to calculate",
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
    fn test_arc_to_time_form_creation() {
        let form = ArcToTimeForm::new();
        assert_eq!(form.longitude, "");
        assert_eq!(form.longitude_direction, 'W');
        assert_eq!(form.meridian_passage_time, "");
        assert_eq!(form.current_field, ArcToTimeInputField::Longitude);
        assert!(form.result.is_none());
    }

    #[test]
    fn test_field_navigation() {
        let mut form = ArcToTimeForm::new();
        form.current_field = ArcToTimeInputField::Longitude;
        form.next_field();
        assert_eq!(form.current_field, ArcToTimeInputField::LongitudeDirection);
        form.next_field();
        assert_eq!(form.current_field, ArcToTimeInputField::MeridianPassageTime);
    }

    #[test]
    fn test_toggle_direction() {
        let mut form = ArcToTimeForm::new();
        assert_eq!(form.longitude_direction, 'W');
        form.toggle_direction();
        assert_eq!(form.longitude_direction, 'E');
        form.toggle_direction();
        assert_eq!(form.longitude_direction, 'W');
    }

    #[test]
    fn test_longitude_to_time_offset_west_45_degrees() {
        // 45° W should be -03:00:00 (behind UTC)
        let (h, m, s) = ArcToTimeForm::longitude_to_time_offset(45.0, 0.0, 'W');
        assert_eq!(h, -3);
        assert_eq!(m, 0);
        assert_eq!(s, 0);
    }

    #[test]
    fn test_longitude_to_time_offset_east_45_degrees() {
        // 45° E should be +03:00:00 (ahead of UTC)
        let (h, m, s) = ArcToTimeForm::longitude_to_time_offset(45.0, 0.0, 'E');
        assert_eq!(h, 3);
        assert_eq!(m, 0);
        assert_eq!(s, 0);
    }

    #[test]
    fn test_longitude_to_time_offset_west_45_30() {
        // 45° 30' W should be -03:02:00
        let (h, m, s) = ArcToTimeForm::longitude_to_time_offset(45.0, 30.0, 'W');
        assert_eq!(h, -3);
        assert_eq!(m, -2);
        assert_eq!(s, 0);
    }

    #[test]
    fn test_longitude_to_time_offset_east_122_degrees() {
        // 122° E should be +08:08:00
        let (h, m, s) = ArcToTimeForm::longitude_to_time_offset(122.0, 0.0, 'E');
        assert_eq!(h, 8);
        assert_eq!(m, 8);
        assert_eq!(s, 0);
    }

    #[test]
    fn test_longitude_to_time_offset_15_degrees_equals_1_hour() {
        // 15° = 1 hour exactly
        let (h, m, s) = ArcToTimeForm::longitude_to_time_offset(15.0, 0.0, 'E');
        assert_eq!(h, 1);
        assert_eq!(m, 0);
        assert_eq!(s, 0);
    }

    #[test]
    fn test_longitude_to_time_offset_15_minutes_arc() {
        // 0° 15' = 1 minute of time
        let (h, m, s) = ArcToTimeForm::longitude_to_time_offset(0.0, 15.0, 'E');
        assert_eq!(h, 0);
        assert_eq!(m, 1);
        assert_eq!(s, 0);
    }

    #[test]
    fn test_apply_time_offset_mer_pass_west() {
        // 12:15:30 LMT at 45°W (-03:00:00) should be 15:15:30 UTC
        let result =
            ArcToTimeForm::apply_time_offset("12:15:30", -3, 0, 0).unwrap();
        assert_eq!(result, "15:15:30");
    }

    #[test]
    fn test_apply_time_offset_mer_pass_east() {
        // 12:00:00 LMT at 45°E (+03:00:00) should be 09:00:00 UTC
        let result =
            ArcToTimeForm::apply_time_offset("12:00:00", 3, 0, 0).unwrap();
        assert_eq!(result, "09:00:00");
    }

    #[test]
    fn test_apply_time_offset_day_wraparound_forward() {
        // 23:00:00 LMT at 45°W (-03:00:00) should be 02:00:00 UTC (next day)
        let result =
            ArcToTimeForm::apply_time_offset("23:00:00", -3, 0, 0).unwrap();
        assert_eq!(result, "02:00:00");
    }

    #[test]
    fn test_apply_time_offset_day_wraparound_backward() {
        // 02:00:00 LMT at 45°E (+03:00:00) should be 23:00:00 UTC (previous day)
        let result =
            ArcToTimeForm::apply_time_offset("02:00:00", 3, 0, 0).unwrap();
        assert_eq!(result, "23:00:00");
    }

    #[test]
    fn test_apply_time_offset_with_minutes_and_seconds() {
        // 12:00:00 LMT at 45°30'W (-03:02:00) should be 15:02:00 UTC
        let result =
            ArcToTimeForm::apply_time_offset("12:00:00", -3, -2, 0).unwrap();
        assert_eq!(result, "15:02:00");
    }

    #[test]
    fn test_calculate_valid_longitude_west() {
        let mut form = ArcToTimeForm::new();
        form.longitude = "45 30".to_string();
        form.longitude_direction = 'W';
        form.calculate();

        assert!(form.result.is_some());
        assert!(form.error_message.is_none());
        let result = form.result.unwrap();
        assert_eq!(result.offset_hours, -3);
        assert_eq!(result.offset_minutes, -2);
    }

    #[test]
    fn test_calculate_with_meridian_passage() {
        let mut form = ArcToTimeForm::new();
        form.longitude = "45 0".to_string();
        form.longitude_direction = 'W';
        form.meridian_passage_time = "12:15:30".to_string();
        form.calculate();

        assert!(form.result.is_some());
        let result = form.result.unwrap();
        assert_eq!(result.mer_pass_lmt, Some("12:15:30".to_string()));
        assert_eq!(result.mer_pass_utc, Some("15:15:30".to_string()));
    }

    #[test]
    fn test_calculate_invalid_longitude() {
        let mut form = ArcToTimeForm::new();
        form.longitude = "200 0".to_string(); // Invalid, > 180
        form.calculate();

        assert!(form.result.is_none());
        assert!(form.error_message.is_some());
    }

    #[test]
    fn test_calculate_invalid_time() {
        let mut form = ArcToTimeForm::new();
        form.longitude = "45 0".to_string();
        form.meridian_passage_time = "25:00:00".to_string(); // Invalid hour
        form.calculate();

        assert!(form.result.is_none());
        assert!(form.error_message.is_some());
    }

    #[test]
    fn test_clear() {
        let mut form = ArcToTimeForm::new();
        form.longitude = "45 30".to_string();
        form.meridian_passage_time = "12:00:00".to_string();
        form.calculate();

        form.clear();
        assert_eq!(form.longitude, "");
        assert_eq!(form.meridian_passage_time, "");
        assert!(form.result.is_none());
        assert!(form.error_message.is_none());
    }

    #[test]
    fn test_real_world_example_san_francisco() {
        // San Francisco: 122°W 25'
        let (h, m, s) = ArcToTimeForm::longitude_to_time_offset(122.0, 25.0, 'W');
        // 122.416667° / 15 = 8.161111 hours = 8h 9m 40s
        assert_eq!(h, -8);
        assert_eq!(m, -9);
        assert_eq!(s, -40);
    }

    #[test]
    fn test_real_world_example_london() {
        // London (near Prime Meridian): 0° 7.5' W
        let (h, m, s) = ArcToTimeForm::longitude_to_time_offset(0.0, 7.5, 'W');
        // 0.125° / 15 = 0.00833 hours = 0h 0m 30s
        assert_eq!(h, 0);
        assert_eq!(m, 0);
        assert_eq!(s, -30);
    }
}
