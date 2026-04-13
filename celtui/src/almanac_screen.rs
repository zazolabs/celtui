//! Almanac data lookup screen
//!
//! This module provides a screen for looking up GHA, Declination, and other
//! almanac data for celestial bodies at a specific date and time.

use chrono::{DateTime, NaiveDate, NaiveTime, TimeZone, Utc};
use celtnav::almanac::{CelestialBody as AlmanacBody, Planet, get_body_position, gha_aries};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Row, Table, Wrap},
    Frame,
};

/// Celestial body selection for almanac lookup
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlmanacCelestialBody {
    Sun,
    Moon,
    Venus,
    Mars,
    Jupiter,
    Saturn,
    Aries, // For GHA Aries (used with stars)
    Star,  // For navigational stars
}

impl AlmanacCelestialBody {
    /// Returns all available celestial bodies for almanac lookup
    pub fn all() -> Vec<AlmanacCelestialBody> {
        vec![
            AlmanacCelestialBody::Sun,
            AlmanacCelestialBody::Moon,
            AlmanacCelestialBody::Venus,
            AlmanacCelestialBody::Mars,
            AlmanacCelestialBody::Jupiter,
            AlmanacCelestialBody::Saturn,
            AlmanacCelestialBody::Aries,
            AlmanacCelestialBody::Star,
        ]
    }

    /// Returns the display name of the celestial body
    pub fn name(&self) -> &str {
        match self {
            AlmanacCelestialBody::Sun => "Sun",
            AlmanacCelestialBody::Moon => "Moon",
            AlmanacCelestialBody::Venus => "Venus",
            AlmanacCelestialBody::Mars => "Mars",
            AlmanacCelestialBody::Jupiter => "Jupiter",
            AlmanacCelestialBody::Saturn => "Saturn",
            AlmanacCelestialBody::Aries => "Aries (First Point)",
            AlmanacCelestialBody::Star => "Star",
        }
    }

    /// Convert to almanac body type
    #[allow(clippy::wrong_self_convention)]
    fn to_almanac_body(&self, star_name: &str) -> Option<AlmanacBody> {
        match self {
            AlmanacCelestialBody::Sun => Some(AlmanacBody::Sun),
            AlmanacCelestialBody::Moon => Some(AlmanacBody::Moon),
            AlmanacCelestialBody::Venus => Some(AlmanacBody::Planet(Planet::Venus)),
            AlmanacCelestialBody::Mars => Some(AlmanacBody::Planet(Planet::Mars)),
            AlmanacCelestialBody::Jupiter => Some(AlmanacBody::Planet(Planet::Jupiter)),
            AlmanacCelestialBody::Saturn => Some(AlmanacBody::Planet(Planet::Saturn)),
            AlmanacCelestialBody::Aries => None, // Aries is special, uses gha_aries directly
            AlmanacCelestialBody::Star => {
                if !star_name.trim().is_empty() {
                    Some(AlmanacBody::Star(star_name.to_string()))
                } else {
                    None
                }
            }
        }
    }
}

/// Input field identifier for form navigation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlmanacInputField {
    Date,
    Time,
    CelestialBody,
    StarName,
}

impl AlmanacInputField {
    /// Returns all fields
    pub fn all() -> Vec<AlmanacInputField> {
        vec![
            AlmanacInputField::Date,
            AlmanacInputField::Time,
            AlmanacInputField::CelestialBody,
            AlmanacInputField::StarName,
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
            AlmanacInputField::Date => "Date (YYYY-MM-DD)",
            AlmanacInputField::Time => "Time UT (HH:MM:SS)",
            AlmanacInputField::CelestialBody => "Celestial Body",
            AlmanacInputField::StarName => "Star Name (type to search, +/- to browse)",
        }
    }
}

/// Almanac lookup result
#[derive(Debug, Clone)]
pub struct AlmanacResult {
    pub gha: f64,
    pub declination: f64,
    pub body_name: String,
    pub datetime: DateTime<Utc>,
}

/// Almanac screen form data
#[derive(Debug, Clone)]
pub struct AlmanacForm {
    pub date: String,
    pub time: String,
    pub celestial_body: AlmanacCelestialBody,
    pub star_name: String,
    pub star_catalog: Vec<String>, // List of all star names for cycling
    pub star_index: usize,         // Current star index in catalog
    pub star_filter_matches: Vec<String>, // Filtered star names for autocompletion
    pub star_selected_index: usize,       // Index of selected star in filtered list
    pub current_field: AlmanacInputField,
    pub result: Option<AlmanacResult>,
    pub error_message: Option<String>,
}

impl AlmanacForm {
    /// Create a new almanac form with default values
    pub fn new() -> Self {
        use celtnav::almanac::get_star_catalog;

        // Use current date and time as default
        let now = Utc::now();

        // Load star catalog
        let catalog = get_star_catalog();
        let star_names: Vec<String> = catalog.iter().map(|s| s.name.to_string()).collect();
        let default_star = star_names.first().cloned().unwrap_or_default();

        Self {
            date: now.format("%Y-%m-%d").to_string(),
            time: now.format("%H:%M:%S").to_string(),
            celestial_body: AlmanacCelestialBody::Sun,
            star_name: default_star,
            star_catalog: star_names,
            star_index: 0,
            star_filter_matches: Vec::new(),
            star_selected_index: 0,
            current_field: AlmanacInputField::Date,
            result: None,
            error_message: None,
        }
    }

    /// Move to next star in catalog
    pub fn next_star(&mut self) {
        if !self.star_catalog.is_empty() {
            self.star_index = (self.star_index + 1) % self.star_catalog.len();
            self.star_name = self.star_catalog[self.star_index].clone();
        }
    }

    /// Move to previous star in catalog
    pub fn previous_star(&mut self) {
        if !self.star_catalog.is_empty() {
            if self.star_index == 0 {
                self.star_index = self.star_catalog.len() - 1;
            } else {
                self.star_index -= 1;
            }
            self.star_name = self.star_catalog[self.star_index].clone();
        }
    }

    /// Filter star catalog based on current star_name input
    /// Returns list of matching star names
    pub fn filter_stars(&self) -> Vec<String> {
        use celtnav::almanac::get_star_catalog;

        let catalog = get_star_catalog();
        let query = self.star_name.trim().to_lowercase();

        if query.is_empty() {
            // If empty, return all stars
            catalog.iter().map(|s| s.name.to_string()).collect()
        } else {
            // Filter stars that start with the query
            catalog
                .iter()
                .filter(|star| star.name.to_lowercase().starts_with(&query))
                .map(|s| s.name.to_string())
                .collect()
        }
    }

    /// Update star filter matches based on current input
    pub fn update_star_filter(&mut self) {
        self.star_filter_matches = self.filter_stars();
        // Reset selected index if it's out of bounds
        if self.star_filter_matches.is_empty() ||
            self.star_selected_index >= self.star_filter_matches.len() {
            self.star_selected_index = 0;
        }
    }

    /// Move selection up in star list (previous star match)
    pub fn previous_star_match(&mut self) {
        if !self.star_filter_matches.is_empty() {
            if self.star_selected_index == 0 {
                self.star_selected_index = self.star_filter_matches.len() - 1;
            } else {
                self.star_selected_index -= 1;
            }
        }
    }

    /// Move selection down in star list (next star match)
    pub fn next_star_match(&mut self) {
        if !self.star_filter_matches.is_empty() {
            self.star_selected_index = (self.star_selected_index + 1) % self.star_filter_matches.len();
        }
    }

    /// Select the currently highlighted star from the filtered list
    pub fn select_current_star(&mut self) {
        if !self.star_filter_matches.is_empty() && self.star_selected_index < self.star_filter_matches.len() {
            self.star_name = self.star_filter_matches[self.star_selected_index].clone();
            self.update_star_filter();
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
    pub fn get_field_value(&self, field: AlmanacInputField) -> String {
        match field {
            AlmanacInputField::Date => self.date.clone(),
            AlmanacInputField::Time => self.time.clone(),
            AlmanacInputField::CelestialBody => self.celestial_body.name().to_string(),
            AlmanacInputField::StarName => self.star_name.clone(),
        }
    }

    /// Set the value for an input field
    pub fn set_field_value(&mut self, field: AlmanacInputField, value: String) {
        match field {
            AlmanacInputField::Date => self.date = value,
            AlmanacInputField::Time => self.time = value,
            AlmanacInputField::CelestialBody => {
                // Handled separately via next/previous celestial body
            }
            AlmanacInputField::StarName => {
                self.star_name = value;
                self.update_star_filter();
            }
        }
    }

    /// Cycle to next celestial body
    pub fn next_celestial_body(&mut self) {
        let bodies = AlmanacCelestialBody::all();
        let current_idx = bodies
            .iter()
            .position(|b| *b == self.celestial_body)
            .unwrap_or(0);
        let next_idx = (current_idx + 1) % bodies.len();
        self.celestial_body = bodies[next_idx];
    }

    /// Cycle to previous celestial body
    pub fn previous_celestial_body(&mut self) {
        let bodies = AlmanacCelestialBody::all();
        let current_idx = bodies
            .iter()
            .position(|b| *b == self.celestial_body)
            .unwrap_or(0);
        let prev_idx = if current_idx == 0 {
            bodies.len() - 1
        } else {
            current_idx - 1
        };
        self.celestial_body = bodies[prev_idx];
    }

    /// Increment time by 1 minute
    pub fn increment_time(&mut self) {
        if let Ok(datetime) = self.parse_datetime() {
            let new_time = datetime + chrono::Duration::minutes(1);
            self.date = new_time.format("%Y-%m-%d").to_string();
            self.time = new_time.format("%H:%M:%S").to_string();
        }
    }

    /// Decrement time by 1 minute
    pub fn decrement_time(&mut self) {
        if let Ok(datetime) = self.parse_datetime() {
            let new_time = datetime - chrono::Duration::minutes(1);
            self.date = new_time.format("%Y-%m-%d").to_string();
            self.time = new_time.format("%H:%M:%S").to_string();
        }
    }

    /// Check if the current field is a text input field (for disabling screen shortcuts)
    /// Returns true when user is typing in free-form text fields
    pub fn is_text_input_active(&self) -> bool {
        match self.current_field {
            // Text input fields (free-form typing)
            AlmanacInputField::Date | AlmanacInputField::Time | AlmanacInputField::StarName => true,

            // Selection fields (use +/- cycling)
            AlmanacInputField::CelestialBody => false,
        }
    }

    /// Parse date and time into DateTime<Utc>
    fn parse_datetime(&self) -> Result<DateTime<Utc>, String> {
        let date = NaiveDate::parse_from_str(&self.date, "%Y-%m-%d")
            .map_err(|_| "Invalid date format. Expected: YYYY-MM-DD".to_string())?;

        let time = NaiveTime::parse_from_str(&self.time, "%H:%M:%S")
            .or_else(|_| NaiveTime::parse_from_str(&self.time, "%H:%M"))
            .map_err(|_| "Invalid time format. Expected: HH:MM:SS or HH:MM".to_string())?;

        let naive_datetime = date.and_time(time);
        Ok(Utc.from_utc_datetime(&naive_datetime))
    }

    /// Lookup almanac data
    pub fn lookup(&mut self) {
        self.result = None;
        self.error_message = None;

        let datetime = match self.parse_datetime() {
            Ok(dt) => dt,
            Err(e) => {
                self.error_message = Some(e);
                return;
            }
        };

        match self.celestial_body {
            AlmanacCelestialBody::Aries => {
                // GHA Aries doesn't have declination
                let gha = gha_aries(datetime);
                self.result = Some(AlmanacResult {
                    gha,
                    declination: 0.0, // Aries is on celestial equator by definition
                    body_name: self.celestial_body.name().to_string(),
                    datetime,
                });
            }
            AlmanacCelestialBody::Star => {
                // Handle star lookup
                if self.star_name.trim().is_empty() {
                    self.error_message = Some("Star name is required".to_string());
                    return;
                }

                if let Some(almanac_body) = self.celestial_body.to_almanac_body(&self.star_name) {
                    match get_body_position(almanac_body, datetime) {
                        Ok(position) => {
                            self.result = Some(AlmanacResult {
                                gha: position.gha,
                                declination: position.declination,
                                body_name: self.star_name.clone(),
                                datetime,
                            });
                        }
                        Err(e) => {
                            self.error_message = Some(e);
                        }
                    }
                } else {
                    self.error_message = Some("Invalid star name".to_string());
                }
            }
            _ => {
                if let Some(almanac_body) = self.celestial_body.to_almanac_body("") {
                    match get_body_position(almanac_body, datetime) {
                        Ok(position) => {
                            self.result = Some(AlmanacResult {
                                gha: position.gha,
                                declination: position.declination,
                                body_name: self.celestial_body.name().to_string(),
                                datetime,
                            });
                        }
                        Err(e) => {
                            self.error_message = Some(e);
                        }
                    }
                }
            }
        }
    }

    /// Handle keyboard events
    pub fn handle_key_event(&mut self, key_event: KeyEvent) {
        match key_event.code {
            KeyCode::Tab => self.next_field(),
            KeyCode::BackTab => self.previous_field(),
            KeyCode::Enter => {
                // If on StarName field, select current highlighted star
                if self.current_field == AlmanacInputField::StarName {
                    self.select_current_star();
                } else {
                    self.lookup();
                }
            }
            KeyCode::Up => {
                if self.current_field == AlmanacInputField::Time {
                    self.increment_time();
                    self.lookup(); // Auto-lookup on browse
                } else if self.current_field == AlmanacInputField::StarName {
                    // In StarName field, navigate filtered star list up
                    self.previous_star_match();
                }
            }
            KeyCode::Down => {
                if self.current_field == AlmanacInputField::Time {
                    self.decrement_time();
                    self.lookup(); // Auto-lookup on browse
                } else if self.current_field == AlmanacInputField::StarName {
                    // In StarName field, navigate filtered star list down
                    self.next_star_match();
                }
            }
            KeyCode::Left => {
                // Left arrow cycles selection fields backward (same as '-')
                match self.current_field {
                    AlmanacInputField::CelestialBody => {
                        self.previous_celestial_body();
                        self.lookup(); // Auto-lookup on body change
                    }
                    AlmanacInputField::StarName => {
                        self.previous_star();
                        self.lookup(); // Auto-lookup on star change
                    }
                    // For text input fields, do nothing
                    _ => {}
                }
            }
            KeyCode::Right => {
                // Right arrow cycles selection fields forward (same as '+')
                match self.current_field {
                    AlmanacInputField::CelestialBody => {
                        self.next_celestial_body();
                        self.lookup(); // Auto-lookup on body change
                    }
                    AlmanacInputField::StarName => {
                        self.next_star();
                        self.lookup(); // Auto-lookup on star change
                    }
                    // For text input fields, do nothing
                    _ => {}
                }
            }
            KeyCode::Char(c) => {
                match self.current_field {
                    AlmanacInputField::CelestialBody => {
                        if c == '+' || c == '=' {
                            self.next_celestial_body();
                            self.lookup(); // Auto-lookup on body change
                        } else if c == '-' || c == '_' {
                            self.previous_celestial_body();
                            self.lookup(); // Auto-lookup on body change
                        }
                    }
                    AlmanacInputField::StarName => {
                        if c == '+' || c == '=' {
                            self.next_star();
                            self.lookup(); // Auto-lookup on star change
                        } else if c == '-' || c == '_' {
                            self.previous_star();
                            self.lookup(); // Auto-lookup on star change
                        } else {
                            // Add character to current field value
                            let mut value = self.get_field_value(self.current_field);
                            value.push(c);
                            self.set_field_value(self.current_field, value);
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

impl Default for AlmanacForm {
    fn default() -> Self {
        Self::new()
    }
}

/// Render the almanac screen
pub fn render_almanac_screen(frame: &mut Frame, area: Rect, form: &AlmanacForm) {
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
fn render_form(frame: &mut Frame, area: Rect, form: &AlmanacForm) {
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
    let help_text = "Tab: Next | Up/Down: Browse Time | +/- or ←→: Cycle Options | Enter: Lookup";
    let help_widget = Paragraph::new(help_text)
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Center);
    frame.render_widget(help_widget, chunks[1]);
}

/// Render the form fields
fn render_form_fields(frame: &mut Frame, area: Rect, form: &AlmanacForm) {
    let fields = AlmanacInputField::all();
    let mut lines = vec![Line::from("")];

    for field in fields {
        // Skip StarName field if Star body is not selected
        if field == AlmanacInputField::StarName && form.celestial_body != AlmanacCelestialBody::Star {
            continue;
        }

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

        // Show autocompletion suggestions for StarName field
        if field == AlmanacInputField::StarName && is_current && !form.star_filter_matches.is_empty() {
            // Show up to 5 suggestions
            let max_suggestions = 5;
            let suggestions_to_show = form.star_filter_matches.len().min(max_suggestions);

            for (i, star_name) in form.star_filter_matches.iter().take(suggestions_to_show).enumerate() {
                let is_selected = i == form.star_selected_index;
                let suggestion_style = if is_selected {
                    Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::DarkGray)
                };
                let prefix = if is_selected { "  → " } else { "    " };
                lines.push(Line::from(vec![
                    Span::styled(prefix, suggestion_style),
                    Span::styled(star_name.clone(), suggestion_style),
                ]));
            }

            if form.star_filter_matches.len() > max_suggestions {
                lines.push(Line::from(Span::styled(
                    format!("    ... {} more", form.star_filter_matches.len() - max_suggestions),
                    Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC),
                )));
            }
        }
    }

    let paragraph = Paragraph::new(lines)
        .block(
            Block::default()
                .title(" Almanac Lookup ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Green)),
        )
        .wrap(Wrap { trim: false });

    frame.render_widget(paragraph, area);
}

/// Render the results panel
fn render_results(frame: &mut Frame, area: Rect, form: &AlmanacForm) {
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
        // Create table layout
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Header
                Constraint::Min(0),    // Table
            ])
            .split(area);

        // Render header with date/time
        let header_text = format!(
            "{} - {} UT",
            result.datetime.format("%Y-%m-%d"),
            result.datetime.format("%H:%M:%S")
        );
        let header = Paragraph::new(header_text)
            .style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Blue)),
            );
        frame.render_widget(header, chunks[0]);

        // Format GHA and Declination with DM (decimal minutes)
        let gha_dms = celtnav::decimal_to_dms(result.gha);

        let dec_sign = if result.declination >= 0.0 { "N" } else { "S" };
        let dec_dms = celtnav::decimal_to_dms(result.declination.abs());

        // Create table rows
        let rows = vec![
            Row::new(vec![
                "Body".to_string(),
                result.body_name.clone(),
            ]),
            Row::new(vec![
                "GHA".to_string(),
                format!("{:03}° {:05.2}'", gha_dms.degrees, gha_dms.minutes),
            ]),
        ];

        let mut all_rows = rows;

        // Only show declination if not Aries
        if form.celestial_body != AlmanacCelestialBody::Aries {
            all_rows.push(Row::new(vec![
                "Declination".to_string(),
                format!("{} {:02}° {:05.2}'", dec_sign, dec_dms.degrees, dec_dms.minutes),
            ]));
        }

        let table = Table::new(
            all_rows,
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
                .title(" Almanac Data ")
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
                "Enter date and time, then press Enter to lookup",
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
    fn test_almanac_form_creation() {
        let form = AlmanacForm::new();
        assert_eq!(form.celestial_body, AlmanacCelestialBody::Sun);
        assert_eq!(form.current_field, AlmanacInputField::Date);
        assert!(form.result.is_none());
    }

    #[test]
    fn test_field_navigation() {
        let mut form = AlmanacForm::new();
        form.current_field = AlmanacInputField::Date;
        form.next_field();
        assert_eq!(form.current_field, AlmanacInputField::Time);
    }

    #[test]
    fn test_celestial_body_cycling() {
        let mut form = AlmanacForm::new();
        form.celestial_body = AlmanacCelestialBody::Sun;
        form.next_celestial_body();
        assert_eq!(form.celestial_body, AlmanacCelestialBody::Moon);
    }

    #[test]
    fn test_parse_datetime_valid() {
        let mut form = AlmanacForm::new();
        form.date = "2024-01-15".to_string();
        form.time = "12:30:45".to_string();
        let result = form.parse_datetime();
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_datetime_invalid_date() {
        let mut form = AlmanacForm::new();
        form.date = "invalid".to_string();
        form.time = "12:30:45".to_string();
        let result = form.parse_datetime();
        assert!(result.is_err());
    }

    #[test]
    fn test_lookup_sun() {
        let mut form = AlmanacForm::new();
        form.date = "2024-01-15".to_string();
        form.time = "12:00:00".to_string();
        form.celestial_body = AlmanacCelestialBody::Sun;
        form.lookup();
        assert!(form.result.is_some());
        assert!(form.error_message.is_none());
    }

    #[test]
    fn test_lookup_aries() {
        let mut form = AlmanacForm::new();
        form.date = "2024-01-15".to_string();
        form.time = "12:00:00".to_string();
        form.celestial_body = AlmanacCelestialBody::Aries;
        form.lookup();
        assert!(form.result.is_some());
        let result = form.result.as_ref().unwrap();
        assert_eq!(result.declination, 0.0); // Aries is on celestial equator
    }

    #[test]
    fn test_time_increment() {
        let mut form = AlmanacForm::new();
        form.date = "2024-01-15".to_string();
        form.time = "12:00:00".to_string();
        form.increment_time();
        assert_eq!(form.time, "12:01:00");
    }

    #[test]
    fn test_time_decrement() {
        let mut form = AlmanacForm::new();
        form.date = "2024-01-15".to_string();
        form.time = "12:00:00".to_string();
        form.decrement_time();
        assert_eq!(form.time, "11:59:00");
    }

    // Star integration tests

    #[test]
    fn test_almanac_form_has_star_catalog() {
        let form = AlmanacForm::new();
        assert_eq!(form.star_catalog.len(), 63);
        assert!(!form.star_name.is_empty());
    }

    #[test]
    fn test_next_star() {
        let mut form = AlmanacForm::new();
        let first_star = form.star_name.clone();
        form.next_star();
        let second_star = form.star_name.clone();
        assert_ne!(first_star, second_star);
        assert_eq!(form.star_index, 1);
    }

    #[test]
    fn test_previous_star() {
        let mut form = AlmanacForm::new();
        form.star_index = 1;
        form.star_name = form.star_catalog[1].clone();
        form.previous_star();
        assert_eq!(form.star_index, 0);
        assert_eq!(form.star_name, form.star_catalog[0]);
    }

    #[test]
    fn test_star_cycling_wraps() {
        let mut form = AlmanacForm::new();
        // Cycle to last star
        form.star_index = form.star_catalog.len() - 1;
        form.star_name = form.star_catalog[form.star_index].clone();

        // Next should wrap to first
        form.next_star();
        assert_eq!(form.star_index, 0);
        assert_eq!(form.star_name, form.star_catalog[0]);

        // Previous should wrap to last
        form.previous_star();
        assert_eq!(form.star_index, form.star_catalog.len() - 1);
    }

    #[test]
    fn test_lookup_star() {
        let mut form = AlmanacForm::new();
        form.date = "2024-01-15".to_string();
        form.time = "12:00:00".to_string();
        form.celestial_body = AlmanacCelestialBody::Star;
        form.star_name = "Sirius".to_string();
        form.lookup();

        assert!(form.result.is_some());
        assert!(form.error_message.is_none());

        let result = form.result.as_ref().unwrap();
        assert_eq!(result.body_name, "Sirius");
        assert!(result.gha >= 0.0 && result.gha < 360.0);
        // Sirius is in southern hemisphere
        assert!(result.declination < 0.0);
    }

    #[test]
    fn test_lookup_star_missing_name() {
        let mut form = AlmanacForm::new();
        form.date = "2024-01-15".to_string();
        form.time = "12:00:00".to_string();
        form.celestial_body = AlmanacCelestialBody::Star;
        form.star_name = String::new();
        form.lookup();

        assert!(form.result.is_none());
        assert!(form.error_message.is_some());
        let error = form.error_message.as_ref().unwrap();
        assert!(error.contains("Star name is required"));
    }

    #[test]
    fn test_lookup_star_invalid_name() {
        let mut form = AlmanacForm::new();
        form.date = "2024-01-15".to_string();
        form.time = "12:00:00".to_string();
        form.celestial_body = AlmanacCelestialBody::Star;
        form.star_name = "InvalidStarName".to_string();
        form.lookup();

        assert!(form.result.is_none());
        assert!(form.error_message.is_some());
    }

    #[test]
    fn test_celestial_body_all_includes_star() {
        let bodies = AlmanacCelestialBody::all();
        assert_eq!(bodies.len(), 8);
        assert!(bodies.contains(&AlmanacCelestialBody::Star));
    }

    #[test]
    fn test_star_field_in_all_fields() {
        let fields = AlmanacInputField::all();
        assert!(fields.contains(&AlmanacInputField::StarName));
    }

    // Star autocomplete tests (following TDD)

    #[test]
    fn test_filter_stars_empty_input() {
        let mut form = AlmanacForm::new();
        form.star_name = String::new();
        let matches = form.filter_stars();
        // Should return all 63 stars when input is empty
        assert_eq!(matches.len(), 63);
    }

    #[test]
    fn test_filter_stars_starts_with() {
        let mut form = AlmanacForm::new();
        form.star_name = String::from("sir");
        let matches = form.filter_stars();
        // Should match "Sirius"
        assert!(matches.contains(&"Sirius".to_string()));
        assert_eq!(matches.len(), 1);
    }

    #[test]
    fn test_filter_stars_case_insensitive() {
        let mut form = AlmanacForm::new();
        form.star_name = String::from("SIR");
        let matches = form.filter_stars();
        assert!(matches.contains(&"Sirius".to_string()));

        form.star_name = String::from("sirius");
        let matches = form.filter_stars();
        assert!(matches.contains(&"Sirius".to_string()));
    }

    #[test]
    fn test_filter_stars_multiple_matches() {
        let mut form = AlmanacForm::new();
        form.star_name = String::from("a");
        let matches = form.filter_stars();
        // Should match: Arcturus, Achernar, Acrux, Aldebaran, Altair, Antares, etc.
        assert!(matches.len() > 5);
        assert!(matches.contains(&"Arcturus".to_string()));
        assert!(matches.contains(&"Altair".to_string()));
    }

    #[test]
    fn test_filter_stars_no_matches() {
        let mut form = AlmanacForm::new();
        form.star_name = String::from("xyz");
        let matches = form.filter_stars();
        assert_eq!(matches.len(), 0);
    }

    #[test]
    fn test_update_star_filter() {
        let mut form = AlmanacForm::new();
        form.star_name = String::from("sir");
        form.update_star_filter();
        assert_eq!(form.star_filter_matches.len(), 1);
        assert_eq!(form.star_filter_matches[0], "Sirius");
        assert_eq!(form.star_selected_index, 0);
    }

    #[test]
    fn test_next_star_match() {
        let mut form = AlmanacForm::new();
        form.star_name = String::from("a");
        form.update_star_filter();

        let initial_len = form.star_filter_matches.len();
        assert!(initial_len > 1);

        // Test cycling through matches
        form.star_selected_index = 0;
        form.next_star_match();
        assert_eq!(form.star_selected_index, 1);

        // Test wrapping at end
        form.star_selected_index = initial_len - 1;
        form.next_star_match();
        assert_eq!(form.star_selected_index, 0);
    }

    #[test]
    fn test_previous_star_match() {
        let mut form = AlmanacForm::new();
        form.star_name = String::from("a");
        form.update_star_filter();

        let initial_len = form.star_filter_matches.len();
        assert!(initial_len > 1);

        // Test cycling backward
        form.star_selected_index = 1;
        form.previous_star_match();
        assert_eq!(form.star_selected_index, 0);

        // Test wrapping at beginning
        form.star_selected_index = 0;
        form.previous_star_match();
        assert_eq!(form.star_selected_index, initial_len - 1);
    }

    #[test]
    fn test_select_current_star_match() {
        let mut form = AlmanacForm::new();
        form.star_name = String::from("a");
        form.update_star_filter();

        // Select first match
        form.star_selected_index = 0;
        let first_star = form.star_filter_matches[0].clone();
        form.select_current_star();
        assert_eq!(form.star_name, first_star);

        // After selection, filter should update to exact match
        assert_eq!(form.star_filter_matches.len(), 1);
        assert_eq!(form.star_filter_matches[0], first_star);
    }

    #[test]
    fn test_set_field_value_updates_star_filter() {
        let mut form = AlmanacForm::new();
        form.set_field_value(AlmanacInputField::StarName, "sir".to_string());

        // Filter should be automatically updated
        assert_eq!(form.star_name, "sir");
        assert_eq!(form.star_filter_matches.len(), 1);
        assert_eq!(form.star_filter_matches[0], "Sirius");
    }

    // Text input active tests for Phase 2

    #[test]
    fn test_is_text_input_active_star_name() {
        let mut form = AlmanacForm::new();
        form.current_field = AlmanacInputField::StarName;
        assert!(form.is_text_input_active());
    }

    #[test]
    fn test_is_text_input_active_date() {
        let mut form = AlmanacForm::new();
        form.current_field = AlmanacInputField::Date;
        assert!(form.is_text_input_active());
    }

    #[test]
    fn test_is_text_input_active_time() {
        let mut form = AlmanacForm::new();
        form.current_field = AlmanacInputField::Time;
        assert!(form.is_text_input_active());
    }

    #[test]
    fn test_is_text_input_active_celestial_body() {
        let mut form = AlmanacForm::new();
        form.current_field = AlmanacInputField::CelestialBody;
        assert!(!form.is_text_input_active()); // CelestialBody uses +/- cycling
    }
}
