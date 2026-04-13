//! Sight reduction calculation screen
//!
//! This module provides a comprehensive form for celestial navigation sight reduction
//! with support for both automatic and manual modes.

use chrono::{NaiveDate, NaiveTime, TimeZone, Utc};
use celtnav::sight_reduction::{
    compute_altitude, compute_azimuth, compute_intercept, SightData,
    apply_refraction_correction, apply_dip_correction,
    apply_semidiameter_correction, apply_parallax_correction,
};
use celtnav::almanac::{
    get_body_position, CelestialBody as AlmanacBody, Planet,
};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

/// Celestial body selection
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CelestialBody {
    Sun,
    Moon,
    Venus,
    Mars,
    Jupiter,
    Saturn,
    Aries, // First Point of Aries (used for star calculations)
    Star,  // Navigate stars from catalog
}

impl CelestialBody {
    /// Returns all available celestial bodies
    pub fn all() -> Vec<CelestialBody> {
        vec![
            CelestialBody::Sun,
            CelestialBody::Moon,
            CelestialBody::Venus,
            CelestialBody::Mars,
            CelestialBody::Jupiter,
            CelestialBody::Saturn,
            CelestialBody::Aries,
            CelestialBody::Star,
        ]
    }

    /// Returns the display name of the celestial body
    pub fn name(&self) -> &str {
        match self {
            CelestialBody::Sun => "Sun",
            CelestialBody::Moon => "Moon",
            CelestialBody::Venus => "Venus",
            CelestialBody::Mars => "Mars",
            CelestialBody::Jupiter => "Jupiter",
            CelestialBody::Saturn => "Saturn",
            CelestialBody::Aries => "Aries (First Point)",
            CelestialBody::Star => "Star",
        }
    }
}

/// Operating mode for the calculation screen
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CalculationMode {
    /// Automatic mode - system looks up all values
    Automatic,
    /// Manual mode - user provides almanac values
    Manual,
}

impl CalculationMode {
    /// Toggle between automatic and manual mode
    pub fn toggle(&self) -> Self {
        match self {
            CalculationMode::Automatic => CalculationMode::Manual,
            CalculationMode::Manual => CalculationMode::Automatic,
        }
    }

    /// Returns the display name of the mode
    pub fn name(&self) -> &str {
        match self {
            CalculationMode::Automatic => "Automatic",
            CalculationMode::Manual => "Manual",
        }
    }
}

/// Input field identifier for form navigation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputField {
    SextantAltitude,   // Single field: "DD MM.M"
    Date,
    Time,
    Latitude,          // Single field: "DD MM.M"
    LatitudeDirection, // N/S - immediately follows Latitude
    Longitude,         // Single field: "DD MM.M"
    LongitudeDirection, // E/W - immediately follows Longitude
    CelestialBody,
    StarName,          // Star name input (when CelestialBody::Star is selected)
    IndexError,
    HeightOfEye,
    // Manual mode only fields
    GHA,               // Single field: "DD MM.M"
    Declination,       // Single field: "DD MM.M"
    DeclinationDirection, // N/S - immediately follows Declination
}

impl InputField {
    /// Returns all fields for automatic mode
    /// Note: StarName is conditionally included during navigation based on selected body
    pub fn automatic_fields() -> Vec<InputField> {
        vec![
            InputField::SextantAltitude,
            InputField::Date,
            InputField::Time,
            InputField::Latitude,
            InputField::LatitudeDirection,
            InputField::Longitude,
            InputField::LongitudeDirection,
            InputField::CelestialBody,
            InputField::StarName,  // Always in list, visibility controlled by selected body
            InputField::IndexError,
            InputField::HeightOfEye,
        ]
    }

    /// Returns all fields for manual mode
    pub fn manual_fields() -> Vec<InputField> {
        vec![
            InputField::SextantAltitude,
            InputField::Date,
            InputField::Time,
            InputField::Latitude,
            InputField::LatitudeDirection,
            InputField::Longitude,
            InputField::LongitudeDirection,
            InputField::CelestialBody,
            InputField::StarName,
            InputField::IndexError,
            InputField::HeightOfEye,
            InputField::GHA,
            InputField::Declination,
            InputField::DeclinationDirection,  // N/S immediately follows Declination
        ]
    }

    /// Get next field in the sequence
    pub fn next(&self, mode: CalculationMode) -> Self {
        let fields = match mode {
            CalculationMode::Automatic => Self::automatic_fields(),
            CalculationMode::Manual => Self::manual_fields(),
        };

        let current_idx = fields.iter().position(|f| f == self).unwrap_or(0);
        let next_idx = (current_idx + 1) % fields.len();
        fields[next_idx]
    }

    /// Get previous field in the sequence
    pub fn previous(&self, mode: CalculationMode) -> Self {
        let fields = match mode {
            CalculationMode::Automatic => Self::automatic_fields(),
            CalculationMode::Manual => Self::manual_fields(),
        };

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
            InputField::SextantAltitude => "Sextant Altitude (Hs) [DD MM.M]",
            InputField::Date => "Date (YYYY-MM-DD)",
            InputField::Time => "Time (HH:MM:SS)",
            InputField::Latitude => "Latitude [DD MM.M]",
            InputField::LatitudeDirection => "Latitude Direction (N/S)",
            InputField::Longitude => "Longitude [DD MM.M]",
            InputField::LongitudeDirection => "Longitude Direction (E/W)",
            InputField::CelestialBody => "Celestial Body",
            InputField::StarName => "Star Name (type to search)",
            InputField::IndexError => "Index Error (arcminutes)",
            InputField::HeightOfEye => "Height of Eye (meters)",
            InputField::GHA => "GHA [DD MM.M]",
            InputField::Declination => "Declination [DD MM.M]",
            InputField::DeclinationDirection => "Declination Direction (N/S)",
        }
    }
}

/// Calculation results
#[derive(Debug, Clone)]
pub struct CalculationResults {
    /// Computed altitude (Hc) in degrees
    pub computed_altitude: f64,
    /// Azimuth (Zn) in degrees
    pub azimuth: f64,
    /// Intercept in nautical miles (positive = toward, negative = away)
    pub intercept: f64,
    /// Observed altitude (Ho) after corrections
    pub observed_altitude: f64,
    /// Applied corrections (for display)
    pub corrections: AppliedCorrections,
}

/// Applied corrections for display purposes
#[derive(Debug, Clone)]
pub struct AppliedCorrections {
    pub refraction: f64,
    pub dip: f64,
    pub semidiameter: f64,
    pub parallax: f64,
    pub index_error: f64,
    pub total: f64,
}

/// Form data for the calculation screen
#[derive(Debug, Clone)]
pub struct CalculationForm {
    // Common fields - now using single DMS fields
    pub sextant_altitude: String,      // "DD MM.M"
    pub date: String,
    pub time: String,
    pub latitude: String,              // "DD MM.M"
    pub latitude_direction: char,      // 'N' or 'S'
    pub longitude: String,             // "DD MM.M"
    pub longitude_direction: char,     // 'E' or 'W'
    pub celestial_body: CelestialBody,
    pub star_name: String,             // Star name input (when CelestialBody::Star is selected)
    pub star_filter_matches: Vec<String>, // Filtered star names for autocompletion
    pub star_selected_index: usize,    // Index of selected star in filtered list
    pub index_error: String,
    pub height_of_eye: String,

    // Manual mode fields
    pub gha: String,                   // "DD MM.M"
    pub declination: String,           // "DD MM.M"
    pub declination_direction: char,   // 'N' or 'S'

    // State
    pub mode: CalculationMode,
    pub current_field: InputField,
    pub results: Option<CalculationResults>,
    pub error_message: Option<String>,
}

impl CalculationForm {
    /// Create a new calculation form with default values
    pub fn new() -> Self {
        Self {
            sextant_altitude: String::new(),
            date: String::new(),
            time: String::new(),
            latitude: String::new(),
            latitude_direction: 'N', // Default: North (positive)
            longitude: String::new(),
            longitude_direction: 'E', // Default: East (positive)
            celestial_body: CelestialBody::Sun,
            star_name: String::new(),
            star_filter_matches: Vec::new(),
            star_selected_index: 0,
            index_error: String::from("0"),
            height_of_eye: String::from("0"),
            gha: String::new(),
            declination: String::new(),
            declination_direction: 'N',
            mode: CalculationMode::Automatic,
            current_field: InputField::SextantAltitude,
            results: None,
            error_message: None,
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
            // Filter stars that start with or contain the query
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
        if self.star_filter_matches.is_empty() {
            self.star_selected_index = 0;
        } else if self.star_selected_index >= self.star_filter_matches.len() {
            self.star_selected_index = 0;
        }
    }

    /// Move selection up in star list (previous star)
    pub fn previous_star_match(&mut self) {
        if !self.star_filter_matches.is_empty() {
            if self.star_selected_index == 0 {
                self.star_selected_index = self.star_filter_matches.len() - 1;
            } else {
                self.star_selected_index -= 1;
            }
        }
    }

    /// Move selection down in star list (next star)
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

    /// Toggle calculation mode
    pub fn toggle_mode(&mut self) {
        self.mode = self.mode.toggle();
        self.results = None;
        self.error_message = None;
    }

    /// Move to next input field
    pub fn next_field(&mut self) {
        self.current_field = self.current_field.next(self.mode);
    }

    /// Move to previous input field
    pub fn previous_field(&mut self) {
        self.current_field = self.current_field.previous(self.mode);
    }

    /// Get the current value for an input field
    pub fn get_field_value(&self, field: InputField) -> String {
        match field {
            InputField::SextantAltitude => self.sextant_altitude.clone(),
            InputField::Date => self.date.clone(),
            InputField::Time => self.time.clone(),
            InputField::Latitude => self.latitude.clone(),
            InputField::LatitudeDirection => self.latitude_direction.to_string(),
            InputField::Longitude => self.longitude.clone(),
            InputField::LongitudeDirection => self.longitude_direction.to_string(),
            InputField::CelestialBody => self.celestial_body.name().to_string(),
            InputField::StarName => self.star_name.clone(),
            InputField::IndexError => self.index_error.clone(),
            InputField::HeightOfEye => self.height_of_eye.clone(),
            InputField::GHA => self.gha.clone(),
            InputField::Declination => self.declination.clone(),
            InputField::DeclinationDirection => self.declination_direction.to_string(),
        }
    }

    /// Set the value for an input field
    pub fn set_field_value(&mut self, field: InputField, value: String) {
        match field {
            InputField::SextantAltitude => self.sextant_altitude = value,
            InputField::Date => self.date = value,
            InputField::Time => self.time = value,
            InputField::Latitude => self.latitude = value,
            InputField::LatitudeDirection => {
                if let Some(c) = value.chars().next() {
                    if c == 'N' || c == 'S' || c == 'n' || c == 's' {
                        self.latitude_direction = c.to_ascii_uppercase();
                    }
                }
            }
            InputField::Longitude => self.longitude = value,
            InputField::LongitudeDirection => {
                if let Some(c) = value.chars().next() {
                    if c == 'E' || c == 'W' || c == 'e' || c == 'w' {
                        self.longitude_direction = c.to_ascii_uppercase();
                    }
                }
            }
            InputField::CelestialBody => {
                // For now, we'll handle body selection separately
            }
            InputField::StarName => {
                self.star_name = value;
                self.update_star_filter();
            }
            InputField::IndexError => self.index_error = value,
            InputField::HeightOfEye => self.height_of_eye = value,
            InputField::GHA => self.gha = value,
            InputField::Declination => self.declination = value,
            InputField::DeclinationDirection => {
                if let Some(c) = value.chars().next() {
                    if c == 'N' || c == 'S' || c == 'n' || c == 's' {
                        self.declination_direction = c.to_ascii_uppercase();
                    }
                }
            }
        }
    }

    /// Cycle to next celestial body
    pub fn next_celestial_body(&mut self) {
        let bodies = CelestialBody::all();
        let current_idx = bodies.iter().position(|b| *b == self.celestial_body).unwrap_or(0);
        let next_idx = (current_idx + 1) % bodies.len();
        self.celestial_body = bodies[next_idx];
    }

    /// Cycle to previous celestial body
    pub fn previous_celestial_body(&mut self) {
        let bodies = CelestialBody::all();
        let current_idx = bodies.iter().position(|b| *b == self.celestial_body).unwrap_or(0);
        let prev_idx = if current_idx == 0 {
            bodies.len() - 1
        } else {
            current_idx - 1
        };
        self.celestial_body = bodies[prev_idx];
    }

    /// Toggle latitude direction between N and S
    pub fn toggle_latitude_direction(&mut self) {
        self.latitude_direction = if self.latitude_direction == 'N' {
            'S'
        } else {
            'N'
        };
    }

    /// Toggle longitude direction between E and W
    pub fn toggle_longitude_direction(&mut self) {
        self.longitude_direction = if self.longitude_direction == 'E' {
            'W'
        } else {
            'E'
        };
    }

    /// Toggle declination direction between N and S
    pub fn toggle_declination_direction(&mut self) {
        self.declination_direction = if self.declination_direction == 'N' {
            'S'
        } else {
            'N'
        };
    }

    /// Check if the current field is a text input field (for disabling screen shortcuts)
    /// Returns true when user is typing in free-form text fields
    pub fn is_text_input_active(&self) -> bool {
        match self.current_field {
            // Text input fields (free-form typing)
            InputField::SextantAltitude
            | InputField::Date
            | InputField::Time
            | InputField::Latitude
            | InputField::Longitude
            | InputField::StarName
            | InputField::IndexError
            | InputField::HeightOfEye
            | InputField::GHA
            | InputField::Declination => true,

            // Selection fields (use +/- or specific keys, not free-form text)
            InputField::CelestialBody
            | InputField::LatitudeDirection
            | InputField::LongitudeDirection
            | InputField::DeclinationDirection => false,
        }
    }

    /// Handle keyboard events
    pub fn handle_key_event(&mut self, key_event: KeyEvent) {
        match key_event.code {
            KeyCode::Tab => self.next_field(),
            KeyCode::BackTab => self.previous_field(),
            KeyCode::Enter => {
                // If on StarName field, select current highlighted star
                if self.current_field == InputField::StarName {
                    self.select_current_star();
                } else {
                    self.calculate();
                }
            }
            KeyCode::Char('m') | KeyCode::Char('M') => self.toggle_mode(),
            KeyCode::Up => {
                // In StarName field, navigate filtered star list up
                if self.current_field == InputField::StarName {
                    self.previous_star_match();
                }
            }
            KeyCode::Down => {
                // In StarName field, navigate filtered star list down
                if self.current_field == InputField::StarName {
                    self.next_star_match();
                }
            }
            KeyCode::Left => {
                // Left arrow cycles selection fields backward (same as '-')
                match self.current_field {
                    InputField::CelestialBody => {
                        self.previous_celestial_body();
                    }
                    InputField::LatitudeDirection => {
                        self.toggle_latitude_direction();
                    }
                    InputField::LongitudeDirection => {
                        self.toggle_longitude_direction();
                    }
                    InputField::DeclinationDirection => {
                        self.toggle_declination_direction();
                    }
                    // For text input fields, do nothing
                    _ => {}
                }
            }
            KeyCode::Right => {
                // Right arrow cycles selection fields forward (same as '+')
                match self.current_field {
                    InputField::CelestialBody => {
                        self.next_celestial_body();
                    }
                    InputField::LatitudeDirection => {
                        self.toggle_latitude_direction();
                    }
                    InputField::LongitudeDirection => {
                        self.toggle_longitude_direction();
                    }
                    InputField::DeclinationDirection => {
                        self.toggle_declination_direction();
                    }
                    // For text input fields, do nothing
                    _ => {}
                }
            }
            KeyCode::Char(c) => {
                // Handle text input for current field
                match self.current_field {
                    InputField::CelestialBody => {
                        if c == '+' || c == '=' {
                            self.next_celestial_body();
                        } else if c == '-' || c == '_' {
                            self.previous_celestial_body();
                        }
                    }
                    InputField::LatitudeDirection => {
                        if c == 'N' || c == 'n' || c == 'S' || c == 's' {
                            self.set_field_value(InputField::LatitudeDirection, c.to_string());
                        }
                    }
                    InputField::LongitudeDirection => {
                        if c == 'E' || c == 'e' || c == 'W' || c == 'w' {
                            self.set_field_value(InputField::LongitudeDirection, c.to_string());
                        }
                    }
                    InputField::DeclinationDirection => {
                        if c == 'N' || c == 'n' || c == 'S' || c == 's' {
                            self.set_field_value(InputField::DeclinationDirection, c.to_string());
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

/// Render the calculation screen
pub fn render_calculation_screen(frame: &mut Frame, area: Rect, form: &CalculationForm) {
    // Create main layout: form on left, results on right
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(60), // Form area
            Constraint::Percentage(40), // Results area
        ])
        .split(area);

    render_form(frame, chunks[0], form);
    render_results(frame, chunks[1], form);
}

/// Render the input form
fn render_form(frame: &mut Frame, area: Rect, form: &CalculationForm) {
    // Create layout for mode toggle and form fields
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Mode toggle
            Constraint::Min(0),    // Form fields
            Constraint::Length(3), // Help text
        ])
        .split(area);

    // Render mode toggle
    let mode_text = format!("Mode: {} (Press M to toggle)", form.mode.name());
    let mode_widget = Paragraph::new(mode_text)
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(mode_widget, chunks[0]);

    // Render form fields
    render_form_fields(frame, chunks[1], form);

    // Render help text
    let help_text = "Tab/Shift+Tab: Navigate | Enter: Calculate | +/- or ←→: Cycle Options";
    let help_widget = Paragraph::new(help_text)
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Center);
    frame.render_widget(help_widget, chunks[2]);
}

/// Render the form fields
fn render_form_fields(frame: &mut Frame, area: Rect, form: &CalculationForm) {
    let fields = match form.mode {
        CalculationMode::Automatic => InputField::automatic_fields(),
        CalculationMode::Manual => InputField::manual_fields(),
    };

    let mut lines = vec![Line::from("")];

    for field in fields {
        // Skip StarName field if Star body is not selected
        if field == InputField::StarName && form.celestial_body != CelestialBody::Star {
            continue;
        }

        let value = form.get_field_value(field);
        let is_current = field == form.current_field;

        let label_style = if is_current {
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };

        let value_style = if is_current {
            Style::default().fg(Color::Green).add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
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
        if field == InputField::StarName && is_current && !form.star_filter_matches.is_empty() {
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
                .title(" Input Form ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Green)),
        )
        .wrap(Wrap { trim: false });

    frame.render_widget(paragraph, area);
}

/// Render the results panel
fn render_results(frame: &mut Frame, area: Rect, form: &CalculationForm) {
    let mut lines = vec![Line::from("")];

    if let Some(error) = &form.error_message {
        lines.push(Line::from(Span::styled(
            "Error:",
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            error,
            Style::default().fg(Color::Red),
        )));
    } else if let Some(results) = &form.results {
        lines.push(Line::from(Span::styled(
            "Results:",
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(""));

        // Computed altitude with DM (decimal minutes) formatting
        let hc_dms = celtnav::decimal_to_dms(results.computed_altitude);
        let hc_formatted = format!("{}° {:05.2}'", hc_dms.degrees, hc_dms.minutes);
        lines.push(Line::from(vec![
            Span::styled("Computed Altitude (Hc): ", Style::default().fg(Color::White)),
            Span::styled(
                hc_formatted,
                Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
            ),
        ]));

        // Azimuth with DM (decimal minutes) formatting
        let zn_dms = celtnav::decimal_to_dms(results.azimuth);
        let zn_formatted = format!("{}° {:05.2}'", zn_dms.degrees, zn_dms.minutes);
        lines.push(Line::from(vec![
            Span::styled("Azimuth (Zn): ", Style::default().fg(Color::White)),
            Span::styled(
                zn_formatted,
                Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
            ),
        ]));

        // Intercept
        let intercept_color = if results.intercept > 0.0 {
            Color::Green
        } else {
            Color::Red
        };
        let intercept_direction = if results.intercept > 0.0 {
            "TOWARD"
        } else {
            "AWAY"
        };
        lines.push(Line::from(vec![
            Span::styled("Intercept: ", Style::default().fg(Color::White)),
            Span::styled(
                format!("{:.1} NM {}", results.intercept.abs(), intercept_direction),
                Style::default().fg(intercept_color).add_modifier(Modifier::BOLD),
            ),
        ]));

        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Corrections Applied:",
            Style::default().fg(Color::Cyan).add_modifier(Modifier::UNDERLINED),
        )));

        // Show corrections (all values in arcminutes)
        lines.push(Line::from(format!(
            "  Index Error: {:+.1}'",
            results.corrections.index_error * 60.0
        )));
        lines.push(Line::from(format!(
            "  Dip: {:+.1}'",
            results.corrections.dip * 60.0
        )));
        lines.push(Line::from(format!(
            "  Refraction: {:+.1}'",
            results.corrections.refraction * 60.0
        )));
        if results.corrections.semidiameter != 0.0 {
            lines.push(Line::from(format!(
                "  Semi-diameter: {:+.1}'",
                results.corrections.semidiameter * 60.0
            )));
        }
        if results.corrections.parallax != 0.0 {
            lines.push(Line::from(format!(
                "  Parallax: {:+.1}'",
                results.corrections.parallax * 60.0
            )));
        }
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("  Total Correction: ", Style::default().fg(Color::White)),
            Span::styled(
                format!("{:+.1}'", results.corrections.total * 60.0),
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            ),
        ]));

        lines.push(Line::from(""));
        // Observed altitude with DM (decimal minutes) formatting
        let ho_dms = celtnav::decimal_to_dms(results.observed_altitude);
        let ho_formatted = format!("{}° {:05.2}'", ho_dms.degrees, ho_dms.minutes);
        lines.push(Line::from(vec![
            Span::styled("Observed Altitude (Ho): ", Style::default().fg(Color::White)),
            Span::styled(
                ho_formatted,
                Style::default().fg(Color::Cyan),
            ),
        ]));
    } else {
        lines.push(Line::from(Span::styled(
            "Enter values and press Enter to calculate",
            Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC),
        )));
    }

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

impl Default for CalculationForm {
    fn default() -> Self {
        Self::new()
    }
}

/// Validation errors
#[derive(Debug, Clone, PartialEq)]
pub enum ValidationError {
    EmptyField(String),
    InvalidNumber(String),
    OutOfRange { field: String, min: f64, max: f64, value: f64 },
    InvalidDate(String),
    InvalidTime(String),
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ValidationError::EmptyField(field) => write!(f, "Field '{}' is required", field),
            ValidationError::InvalidNumber(field) => write!(f, "Field '{}' must be a valid number", field),
            ValidationError::OutOfRange { field, min, max, value } => {
                write!(f, "Field '{}' value {} is out of range [{}, {}]", field, value, min, max)
            }
            ValidationError::InvalidDate(msg) => write!(f, "Invalid date: {}", msg),
            ValidationError::InvalidTime(msg) => write!(f, "Invalid time: {}", msg),
        }
    }
}

/// Validated input data
#[derive(Debug, Clone)]
pub struct ValidatedInput {
    pub sextant_altitude: f64, // in degrees
    pub date: NaiveDate,
    pub time: NaiveTime,
    pub latitude: f64, // in degrees (positive = North)
    pub longitude: f64, // in degrees (positive = East)
    pub celestial_body: CelestialBody,
    pub index_error: f64, // in arcminutes
    pub height_of_eye: f64, // in meters
    pub gha: Option<f64>, // in degrees (manual mode only)
    pub declination: Option<f64>, // in degrees (manual mode only)
}

impl CalculationForm {
    /// Validate all form inputs
    pub fn validate(&self) -> Result<ValidatedInput, ValidationError> {
        use crate::validation::parse_dms;

        // Validate sextant altitude using new DMS parser
        let (sext_deg, sext_min, sext_sec) = parse_dms(&self.sextant_altitude)
            .map_err(|e| ValidationError::InvalidNumber(format!("Sextant altitude: {}", e)))?;

        self.validate_range(sext_deg, "Sextant altitude degrees", 0.0, 90.0)?;

        let sextant_altitude = celtnav::dms_to_decimal(sext_deg as i32, sext_min as u32, sext_sec);

        // Validate date
        let date = NaiveDate::parse_from_str(&self.date, "%Y-%m-%d")
            .map_err(|_| ValidationError::InvalidDate("Expected format: YYYY-MM-DD".to_string()))?;

        // Validate time
        let time = NaiveTime::parse_from_str(&self.time, "%H:%M:%S")
            .or_else(|_| NaiveTime::parse_from_str(&self.time, "%H:%M"))
            .map_err(|_| ValidationError::InvalidTime("Expected format: HH:MM:SS or HH:MM".to_string()))?;

        // Validate latitude using new DMS parser
        let (lat_deg, lat_min, lat_sec) = parse_dms(&self.latitude)
            .map_err(|e| ValidationError::InvalidNumber(format!("Latitude: {}", e)))?;

        self.validate_range(lat_deg, "Latitude degrees", 0.0, 90.0)?;

        let latitude = if self.latitude_direction == 'S' {
            -celtnav::dms_to_decimal(lat_deg as i32, lat_min as u32, lat_sec)
        } else {
            celtnav::dms_to_decimal(lat_deg as i32, lat_min as u32, lat_sec)
        };

        // Validate longitude using new DMS parser
        let (lon_deg, lon_min, lon_sec) = parse_dms(&self.longitude)
            .map_err(|e| ValidationError::InvalidNumber(format!("Longitude: {}", e)))?;

        self.validate_range(lon_deg, "Longitude degrees", 0.0, 180.0)?;

        let longitude = if self.longitude_direction == 'W' {
            -celtnav::dms_to_decimal(lon_deg as i32, lon_min as u32, lon_sec)
        } else {
            celtnav::dms_to_decimal(lon_deg as i32, lon_min as u32, lon_sec)
        };

        // Validate index error and height of eye
        let index_error = self.parse_number(&self.index_error, "Index Error")?;
        let height_of_eye = self.parse_number(&self.height_of_eye, "Height of Eye")?;

        self.validate_range(height_of_eye, "Height of Eye", 0.0, 100.0)?;

        // Validate manual mode fields if needed
        let gha = if self.mode == CalculationMode::Manual {
            let (gha_deg, gha_min, gha_sec) = parse_dms(&self.gha)
                .map_err(|e| ValidationError::InvalidNumber(format!("GHA: {}", e)))?;

            self.validate_range(gha_deg, "GHA degrees", 0.0, 360.0)?;

            Some(celtnav::dms_to_decimal(gha_deg as i32, gha_min as u32, gha_sec))
        } else {
            None
        };

        let declination = if self.mode == CalculationMode::Manual {
            let (dec_deg, dec_min, dec_sec) = parse_dms(&self.declination)
                .map_err(|e| ValidationError::InvalidNumber(format!("Declination: {}", e)))?;

            self.validate_range(dec_deg, "Declination degrees", 0.0, 90.0)?;

            let dec_value = if self.declination_direction == 'S' {
                -celtnav::dms_to_decimal(dec_deg as i32, dec_min as u32, dec_sec)
            } else {
                celtnav::dms_to_decimal(dec_deg as i32, dec_min as u32, dec_sec)
            };
            Some(dec_value)
        } else {
            None
        };

        Ok(ValidatedInput {
            sextant_altitude,
            date,
            time,
            latitude,
            longitude,
            celestial_body: self.celestial_body,
            index_error,
            height_of_eye,
            gha,
            declination,
        })
    }

    /// Parse a string as a number
    fn parse_number(&self, value: &str, field_name: &str) -> Result<f64, ValidationError> {
        if value.trim().is_empty() {
            return Err(ValidationError::EmptyField(field_name.to_string()));
        }

        value.trim().parse::<f64>()
            .map_err(|_| ValidationError::InvalidNumber(field_name.to_string()))
    }

    /// Validate that a value is within a range
    fn validate_range(&self, value: f64, field_name: &str, min: f64, max: f64) -> Result<(), ValidationError> {
        if value < min || value > max {
            Err(ValidationError::OutOfRange {
                field: field_name.to_string(),
                min,
                max,
                value,
            })
        } else {
            Ok(())
        }
    }

    /// Perform the sight reduction calculation
    pub fn calculate(&mut self) {
        // Clear previous results
        self.results = None;
        self.error_message = None;

        // Validate inputs
        let validated = match self.validate() {
            Ok(v) => v,
            Err(e) => {
                self.error_message = Some(e.to_string());
                return;
            }
        };

        // Perform calculation based on mode
        let result = match self.mode {
            CalculationMode::Automatic => self.calculate_automatic(&validated),
            CalculationMode::Manual => self.calculate_manual(&validated),
        };

        match result {
            Ok(results) => {
                self.results = Some(results);
            }
            Err(e) => {
                self.error_message = Some(e);
            }
        }
    }

    /// Calculate in automatic mode (look up almanac data automatically)
    fn calculate_automatic(&self, input: &ValidatedInput) -> Result<CalculationResults, String> {
        // For now, we'll use placeholder GHA and Declination values
        // In a real implementation, these would be looked up from almanac data
        // based on the date, time, and celestial body

        // Placeholder almanac values (these should be looked up)
        let gha = self.lookup_gha(input)?;
        let declination = self.lookup_declination(input)?;

        self.perform_sight_reduction(input, gha, declination)
    }

    /// Calculate in manual mode (use user-provided almanac data)
    fn calculate_manual(&self, input: &ValidatedInput) -> Result<CalculationResults, String> {
        let gha = input.gha.ok_or("GHA is required in manual mode")?;
        let declination = input.declination.ok_or("Declination is required in manual mode")?;

        self.perform_sight_reduction(input, gha, declination)
    }

    /// Perform the actual sight reduction calculations
    fn perform_sight_reduction(
        &self,
        input: &ValidatedInput,
        gha: f64,
        declination: f64,
    ) -> Result<CalculationResults, String> {
        // Apply altitude corrections to sextant altitude to get observed altitude (Ho)
        let mut ho = input.sextant_altitude;

        // Apply index error (convert from arcminutes to degrees)
        let index_error_deg = input.index_error / 60.0;
        ho += index_error_deg;

        // Apply dip correction
        let dip = apply_dip_correction(input.height_of_eye);
        ho += dip;

        // Apply refraction correction
        let refraction = apply_refraction_correction(ho);
        ho += refraction;

        // Apply semi-diameter correction (only for Sun and Moon)
        let semidiameter = match input.celestial_body {
            CelestialBody::Sun => {
                // Sun's semi-diameter is approximately 16 arcminutes (0.267°)
                // Assuming lower limb observation
                apply_semidiameter_correction(0.267, true)
            }
            CelestialBody::Moon => {
                // Moon's semi-diameter varies, using average of 15 arcminutes (0.25°)
                // Assuming lower limb observation
                apply_semidiameter_correction(0.25, true)
            }
            // No semi-diameter correction for planets, Aries, or stars (point sources)
            _ => 0.0,
        };
        ho += semidiameter;

        // Apply parallax correction (only significant for Moon)
        let parallax = match input.celestial_body {
            CelestialBody::Moon => {
                // Moon's horizontal parallax is approximately 57 arcminutes (0.95°)
                apply_parallax_correction(0.95, ho)
            }
            // Parallax negligible for Sun, planets, Aries, and stars
            _ => 0.0,
        };
        ho += parallax;

        // Calculate Local Hour Angle (LHA) = GHA + Longitude (East positive)
        let lha = (gha + input.longitude + 360.0) % 360.0;

        // Create sight data for computation
        let sight_data = SightData {
            latitude: input.latitude,
            declination,
            local_hour_angle: lha,
        };

        // Compute altitude (Hc) and azimuth (Zn)
        let hc = compute_altitude(&sight_data);
        let zn = compute_azimuth(&sight_data);

        // Compute intercept
        let intercept = compute_intercept(&sight_data, ho);

        Ok(CalculationResults {
            computed_altitude: hc,
            azimuth: zn,
            intercept,
            observed_altitude: ho,
            corrections: AppliedCorrections {
                refraction,
                dip,
                semidiameter,
                parallax,
                index_error: index_error_deg,
                total: refraction + dip + semidiameter + parallax + index_error_deg,
            },
        })
    }

    /// Convert UI CelestialBody to almanac CelestialBody
    /// Returns None for Aries since it's handled specially (GHA Aries is calculated directly)
    fn to_almanac_body(&self, ui_body: CelestialBody) -> Option<AlmanacBody> {
        match ui_body {
            CelestialBody::Sun => Some(AlmanacBody::Sun),
            CelestialBody::Moon => Some(AlmanacBody::Moon),
            CelestialBody::Venus => Some(AlmanacBody::Planet(Planet::Venus)),
            CelestialBody::Mars => Some(AlmanacBody::Planet(Planet::Mars)),
            CelestialBody::Jupiter => Some(AlmanacBody::Planet(Planet::Jupiter)),
            CelestialBody::Saturn => Some(AlmanacBody::Planet(Planet::Saturn)),
            // Aries is special: only GHA is calculated directly via gha_aries()
            // GHA_Aries = GMST (Greenwich Mean Sidereal Time)
            CelestialBody::Aries => None,
            // Star uses the star_name field to look up from catalog
            // GHA_star = GHA_Aries + SHA_star (from star catalog)
            // Declination_star = from star catalog (essentially constant)
            CelestialBody::Star => {
                if !self.star_name.is_empty() {
                    Some(AlmanacBody::Star(self.star_name.clone()))
                } else {
                    None
                }
            }
        }
    }

    /// Look up GHA from almanac data
    fn lookup_gha(&self, input: &ValidatedInput) -> Result<f64, String> {
        use celtnav::almanac::gha_aries;

        // Create DateTime<Utc> from date and time
        let naive_datetime = input.date.and_time(input.time);
        let datetime = Utc.from_utc_datetime(&naive_datetime);

        // Handle Aries specially - it's calculated directly, not via get_body_position
        if input.celestial_body == CelestialBody::Aries {
            // GHA Aries is the Greenwich Hour Angle of the First Point of Aries
            // This is used as the reference for star positions:
            // GHA_star = GHA_Aries + SHA_star
            return Ok(gha_aries(datetime));
        }

        // Handle Star - validate star name is provided
        if input.celestial_body == CelestialBody::Star {
            if self.star_name.trim().is_empty() {
                return Err("Star name is required when 'Star' is selected".to_string());
            }
        }

        // Convert to almanac body type
        let almanac_body = self.to_almanac_body(input.celestial_body)
            .ok_or_else(|| "Invalid celestial body for almanac lookup".to_string())?;

        // Get position from almanac
        let position = get_body_position(almanac_body, datetime)?;

        Ok(position.gha)
    }

    /// Look up Declination from almanac data
    fn lookup_declination(&self, input: &ValidatedInput) -> Result<f64, String> {
        // Create DateTime<Utc> from date and time
        let naive_datetime = input.date.and_time(input.time);
        let datetime = Utc.from_utc_datetime(&naive_datetime);

        // Handle Aries specially - declination is always 0 (on celestial equator by definition)
        if input.celestial_body == CelestialBody::Aries {
            // The First Point of Aries is on the celestial equator by definition
            // (intersection of celestial equator and ecliptic at vernal equinox)
            return Ok(0.0);
        }

        // Handle Star - validate star name is provided
        if input.celestial_body == CelestialBody::Star {
            if self.star_name.trim().is_empty() {
                return Err("Star name is required when 'Star' is selected".to_string());
            }
        }

        // Convert to almanac body type
        let almanac_body = self.to_almanac_body(input.celestial_body)
            .ok_or_else(|| "Invalid celestial body for almanac lookup".to_string())?;

        // Get position from almanac
        let position = get_body_position(almanac_body, datetime)?;

        Ok(position.declination)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculation_form_creation() {
        let form = CalculationForm::new();
        assert_eq!(form.mode, CalculationMode::Automatic);
        assert_eq!(form.current_field, InputField::SextantAltitude);
        assert!(form.results.is_none());
    }

    #[test]
    fn test_mode_toggle() {
        let mut form = CalculationForm::new();
        assert_eq!(form.mode, CalculationMode::Automatic);
        form.toggle_mode();
        assert_eq!(form.mode, CalculationMode::Manual);
        form.toggle_mode();
        assert_eq!(form.mode, CalculationMode::Automatic);
    }

    #[test]
    fn test_field_navigation() {
        let mut form = CalculationForm::new();
        form.current_field = InputField::SextantAltitude;
        form.next_field();
        assert_eq!(form.current_field, InputField::Date);
        form.next_field();
        assert_eq!(form.current_field, InputField::Time);
    }

    #[test]
    fn test_celestial_body_cycling() {
        let mut form = CalculationForm::new();
        form.celestial_body = CelestialBody::Sun;
        form.next_celestial_body();
        assert_eq!(form.celestial_body, CelestialBody::Moon);
    }

    #[test]
    fn test_latitude_direction_setting() {
        let mut form = CalculationForm::new();
        form.set_field_value(InputField::LatitudeDirection, "s".to_string());
        assert_eq!(form.latitude_direction, 'S');
        form.set_field_value(InputField::LatitudeDirection, "N".to_string());
        assert_eq!(form.latitude_direction, 'N');
    }

    #[test]
    fn test_longitude_direction_setting() {
        let mut form = CalculationForm::new();
        // Default should be E
        assert_eq!(form.longitude_direction, 'E');
        form.set_field_value(InputField::LongitudeDirection, "w".to_string());
        assert_eq!(form.longitude_direction, 'W');
        form.set_field_value(InputField::LongitudeDirection, "E".to_string());
        assert_eq!(form.longitude_direction, 'E');
    }

    // Validation tests
    #[test]
    fn test_validation_empty_field() {
        let form = CalculationForm::new();
        let result = form.validate();
        assert!(result.is_err());
        // Should get an error about sextant altitude being required or invalid
        assert!(matches!(result, Err(ValidationError::InvalidNumber(_))));
    }

    #[test]
    fn test_validation_invalid_number() {
        let mut form = CalculationForm::new();
        form.sextant_altitude = "abc".to_string();
        let result = form.validate();
        assert!(result.is_err());
        assert!(matches!(result, Err(ValidationError::InvalidNumber(_))));
    }

    #[test]
    fn test_validation_altitude_out_of_range() {
        let mut form = CalculationForm::new();
        form.sextant_altitude = "95 30".to_string(); // > 90
        let result = form.validate();
        assert!(result.is_err());
        if let Err(ValidationError::OutOfRange { field, min, max, value }) = result {
            assert_eq!(field, "Sextant altitude degrees");
            assert_eq!(min, 0.0);
            assert_eq!(max, 90.0);
            assert_eq!(value, 95.0);
        } else {
            panic!("Expected OutOfRange error");
        }
    }

    #[test]
    fn test_validation_latitude_out_of_range() {
        let mut form = CalculationForm::new();
        form.sextant_altitude = "45 30".to_string();
        form.date = "2024-01-15".to_string();
        form.time = "12:30:45".to_string();
        form.latitude = "95 0".to_string(); // > 90
        let result = form.validate();
        assert!(result.is_err());
        if let Err(ValidationError::OutOfRange { field, .. }) = result {
            assert_eq!(field, "Latitude degrees");
        } else {
            panic!("Expected OutOfRange error for latitude");
        }
    }

    #[test]
    fn test_validation_longitude_out_of_range() {
        let mut form = CalculationForm::new();
        form.sextant_altitude = "45 30".to_string();
        form.date = "2024-01-15".to_string();
        form.time = "12:30:45".to_string();
        form.latitude = "40 0".to_string();
        form.longitude = "185 0".to_string(); // > 180
        let result = form.validate();
        assert!(result.is_err());
        if let Err(ValidationError::OutOfRange { field, .. }) = result {
            assert_eq!(field, "Longitude degrees");
        } else {
            panic!("Expected OutOfRange error for longitude");
        }
    }

    #[test]
    fn test_validation_invalid_date() {
        let mut form = CalculationForm::new();
        form.sextant_altitude = "45 30".to_string();
        form.date = "2024-13-45".to_string(); // Invalid date
        form.time = "12:30:45".to_string();
        let result = form.validate();
        assert!(result.is_err());
        if let Err(ValidationError::InvalidDate(_)) = result {
            // Expected
        } else {
            panic!("Expected InvalidDate error");
        }
    }

    #[test]
    fn test_validation_invalid_time() {
        let mut form = CalculationForm::new();
        form.sextant_altitude = "45 30".to_string();
        form.date = "2024-01-15".to_string();
        form.time = "25:30:45".to_string(); // Invalid time
        let result = form.validate();
        assert!(result.is_err());
        if let Err(ValidationError::InvalidTime(_)) = result {
            // Expected
        } else {
            panic!("Expected InvalidTime error");
        }
    }

    #[test]
    fn test_validation_valid_automatic_mode() {
        let mut form = CalculationForm::new();
        form.sextant_altitude = "45 30".to_string();
        form.date = "2024-01-15".to_string();
        form.time = "12:30:45".to_string();
        form.latitude = "40 15".to_string();
        form.latitude_direction = 'N';
        form.longitude = "74 0".to_string();
        form.longitude_direction = 'E'; // Changed to E to match new default
        form.index_error = "0".to_string();
        form.height_of_eye = "10".to_string();
        form.mode = CalculationMode::Automatic;

        let result = form.validate();
        assert!(result.is_ok());
        let validated = result.unwrap();
        assert_eq!(validated.sextant_altitude, 45.5);
        assert_eq!(validated.latitude, 40.25);
        assert_eq!(validated.longitude, 74.0); // Now positive (East)
        assert!(validated.gha.is_none());
        assert!(validated.declination.is_none());
    }

    #[test]
    fn test_validation_valid_manual_mode() {
        let mut form = CalculationForm::new();
        form.sextant_altitude = "45 30".to_string();
        form.date = "2024-01-15".to_string();
        form.time = "12:30:45".to_string();
        form.latitude = "40 15".to_string();
        form.latitude_direction = 'S';
        form.longitude = "74 30".to_string();
        form.longitude_direction = 'E';
        form.index_error = "2.5".to_string();
        form.height_of_eye = "15".to_string();
        form.gha = "120 30".to_string();
        form.declination = "15 15".to_string();
        form.declination_direction = 'N';
        form.mode = CalculationMode::Manual;

        let result = form.validate();
        assert!(result.is_ok());
        let validated = result.unwrap();
        // Sextant altitude is now in DMS, should be 45° 30' 0" = 45.5°
        assert!((validated.sextant_altitude - 45.5).abs() < 0.001);
        // Latitude is 40° 15' 0" S = -40.25°
        assert!((validated.latitude - (-40.25)).abs() < 0.001);
        // Longitude is 74° 30' 0" E = 74.5°
        assert!((validated.longitude - 74.5).abs() < 0.001);
        // GHA is 120° 30' 0" = 120.5°
        assert_eq!(validated.gha, Some(120.5));
        // Declination is 15° 15' 0" N = 15.25°
        assert_eq!(validated.declination, Some(15.25));
    }

    #[test]
    fn test_validation_manual_mode_missing_gha() {
        let mut form = CalculationForm::new();
        form.sextant_altitude = "45 30".to_string();
        form.date = "2024-01-15".to_string();
        form.time = "12:30:45".to_string();
        form.latitude = "40 0".to_string();
        form.longitude = "74 0".to_string();
        form.index_error = "0".to_string();
        form.height_of_eye = "10".to_string();
        form.mode = CalculationMode::Manual;
        // gha is empty (default)

        let result = form.validate();
        assert!(result.is_err());
        // Should get an invalid number or empty field error for GHA
        assert!(matches!(result, Err(ValidationError::InvalidNumber(_))));
    }

    // Calculation tests
    #[test]
    fn test_calculate_automatic_mode_success() {
        let mut form = CalculationForm::new();
        form.sextant_altitude = "45 30".to_string();
        form.date = "2024-01-15".to_string();
        form.time = "12:30:45".to_string();
        form.latitude = "40 15".to_string();
        form.latitude_direction = 'N';
        form.longitude = "74 0".to_string();
        form.longitude_direction = 'W';
        form.index_error = "0".to_string();
        form.height_of_eye = "10".to_string();
        form.mode = CalculationMode::Automatic;

        form.calculate();

        assert!(form.results.is_some());
        assert!(form.error_message.is_none());

        let results = form.results.as_ref().unwrap();
        // Check that results are within reasonable ranges
        assert!(results.computed_altitude >= -90.0 && results.computed_altitude <= 90.0);
        assert!(results.azimuth >= 0.0 && results.azimuth <= 360.0);
    }

    #[test]
    fn test_calculate_manual_mode_success() {
        let mut form = CalculationForm::new();
        form.sextant_altitude = "45 30".to_string();
        form.date = "2024-01-15".to_string();
        form.time = "12:30:45".to_string();
        form.latitude = "40 15".to_string();
        form.latitude_direction = 'N';
        form.longitude = "74 0".to_string();
        form.longitude_direction = 'W';
        form.index_error = "0".to_string();
        form.height_of_eye = "10".to_string();
        form.gha = "180 30".to_string();
        form.declination = "20 15".to_string();
        form.declination_direction = 'N';
        form.mode = CalculationMode::Manual;

        form.calculate();

        assert!(form.results.is_some());
        assert!(form.error_message.is_none());

        let results = form.results.as_ref().unwrap();
        assert!(results.computed_altitude >= -90.0 && results.computed_altitude <= 90.0);
        assert!(results.azimuth >= 0.0 && results.azimuth <= 360.0);
    }

    #[test]
    fn test_calculate_with_validation_error() {
        let mut form = CalculationForm::new();
        // Missing required fields
        form.mode = CalculationMode::Automatic;

        form.calculate();

        assert!(form.results.is_none());
        assert!(form.error_message.is_some());
    }

    #[test]
    fn test_calculate_applies_corrections() {
        let mut form = CalculationForm::new();
        form.sextant_altitude = "45 30".to_string();
        form.date = "2024-01-15".to_string();
        form.time = "12:30:45".to_string();
        form.latitude = "40 0".to_string();
        form.latitude_direction = 'N';
        form.longitude = "74 0".to_string();
        form.longitude_direction = 'W';
        form.index_error = "2.5".to_string(); // 2.5 arcminutes
        form.height_of_eye = "10".to_string(); // 10 meters
        form.mode = CalculationMode::Automatic;

        form.calculate();

        assert!(form.results.is_some());
        let results = form.results.as_ref().unwrap();

        // Check that corrections were applied
        assert!(results.corrections.dip < 0.0); // Dip is always negative
        assert!(results.corrections.refraction < 0.0); // Refraction is always negative
        assert!(results.corrections.index_error > 0.0); // We set positive index error
    }

    #[test]
    fn test_intercept_calculation() {
        let mut form = CalculationForm::new();
        form.sextant_altitude = "45 30".to_string();
        form.date = "2024-01-15".to_string();
        form.time = "12:30:45".to_string();
        form.latitude = "40 0".to_string();
        form.latitude_direction = 'N';
        form.longitude = "74 0".to_string();
        form.longitude_direction = 'W';
        form.index_error = "0".to_string();
        form.height_of_eye = "10".to_string();
        form.gha = "180 0".to_string();
        form.declination = "20 0".to_string();
        form.declination_direction = 'N';
        form.mode = CalculationMode::Manual;

        form.calculate();

        assert!(form.results.is_some());
        let results = form.results.as_ref().unwrap();

        // Intercept should be in nautical miles
        // The value itself depends on the specific inputs, but it should be calculable
        // Just verify it's a reasonable number (not NaN or infinite)
        assert!(!results.intercept.is_nan());
        assert!(!results.intercept.is_infinite());
    }

    #[test]
    fn test_aries_automatic_mode() {
        let mut form = CalculationForm::new();
        form.sextant_altitude = "45 30".to_string();
        form.date = "2024-01-15".to_string();
        form.time = "12:00:00".to_string();
        form.latitude = "40 15".to_string();
        form.latitude_direction = 'N';
        form.longitude = "74 0".to_string();
        form.longitude_direction = 'E';
        form.index_error = "0".to_string();
        form.height_of_eye = "10".to_string();
        form.celestial_body = CelestialBody::Aries;
        form.mode = CalculationMode::Automatic;

        form.calculate();

        // Should succeed - Aries GHA and declination can be looked up
        assert!(form.results.is_some());
        assert!(form.error_message.is_none());

        let results = form.results.as_ref().unwrap();
        // Results should be valid
        assert!(results.computed_altitude >= -90.0 && results.computed_altitude <= 90.0);
        assert!(results.azimuth >= 0.0 && results.azimuth <= 360.0);
        assert!(!results.intercept.is_nan());
        assert!(!results.intercept.is_infinite());
    }

    #[test]
    fn test_aries_declination_is_zero() {
        let mut form = CalculationForm::new();
        form.date = "2024-01-15".to_string();
        form.time = "12:00:00".to_string();
        form.celestial_body = CelestialBody::Aries;

        let input = ValidatedInput {
            sextant_altitude: 45.0,
            date: chrono::NaiveDate::parse_from_str("2024-01-15", "%Y-%m-%d").unwrap(),
            time: chrono::NaiveTime::parse_from_str("12:00:00", "%H:%M:%S").unwrap(),
            latitude: 40.0,
            longitude: 74.0,
            celestial_body: CelestialBody::Aries,
            index_error: 0.0,
            height_of_eye: 10.0,
            gha: None,
            declination: None,
        };

        // Aries declination should always be 0
        let dec = form.lookup_declination(&input);
        assert!(dec.is_ok());
        assert_eq!(dec.unwrap(), 0.0);
    }

    #[test]
    fn test_default_directions() {
        let form = CalculationForm::new();
        assert_eq!(form.latitude_direction, 'N', "Default latitude should be North");
        assert_eq!(form.longitude_direction, 'E', "Default longitude should be East");
    }

    #[test]
    fn test_celestial_body_names() {
        assert_eq!(CelestialBody::Sun.name(), "Sun");
        assert_eq!(CelestialBody::Moon.name(), "Moon");
        assert_eq!(CelestialBody::Venus.name(), "Venus");
        assert_eq!(CelestialBody::Mars.name(), "Mars");
        assert_eq!(CelestialBody::Jupiter.name(), "Jupiter");
        assert_eq!(CelestialBody::Saturn.name(), "Saturn");
        assert_eq!(CelestialBody::Aries.name(), "Aries (First Point)");
        assert_eq!(CelestialBody::Star.name(), "Star");
    }

    // Star catalog integration tests

    #[test]
    fn test_star_filter_empty_input() {
        let mut form = CalculationForm::new();
        form.star_name = String::new();
        let matches = form.filter_stars();
        // Should return all 63 stars when input is empty
        assert_eq!(matches.len(), 63);
    }

    #[test]
    fn test_star_filter_starts_with() {
        let mut form = CalculationForm::new();
        form.star_name = String::from("sir");
        let matches = form.filter_stars();
        // Should match "Sirius"
        assert!(matches.contains(&"Sirius".to_string()));
        assert_eq!(matches.len(), 1);
    }

    #[test]
    fn test_star_filter_case_insensitive() {
        let mut form = CalculationForm::new();
        form.star_name = String::from("SIR");
        let matches = form.filter_stars();
        assert!(matches.contains(&"Sirius".to_string()));

        form.star_name = String::from("sirius");
        let matches = form.filter_stars();
        assert!(matches.contains(&"Sirius".to_string()));
    }

    #[test]
    fn test_star_filter_multiple_matches() {
        let mut form = CalculationForm::new();
        form.star_name = String::from("a");
        let matches = form.filter_stars();
        // Should match: Arcturus, Achernar, Acrux, Aldebaran, Altair, Antares, etc.
        assert!(matches.len() > 5);
        assert!(matches.contains(&"Arcturus".to_string()));
        assert!(matches.contains(&"Altair".to_string()));
    }

    #[test]
    fn test_star_filter_no_matches() {
        let mut form = CalculationForm::new();
        form.star_name = String::from("xyz");
        let matches = form.filter_stars();
        assert_eq!(matches.len(), 0);
    }

    #[test]
    fn test_update_star_filter() {
        let mut form = CalculationForm::new();
        form.star_name = String::from("sir");
        form.update_star_filter();
        assert_eq!(form.star_filter_matches.len(), 1);
        assert_eq!(form.star_filter_matches[0], "Sirius");
        assert_eq!(form.star_selected_index, 0);
    }

    #[test]
    fn test_next_star_match() {
        let mut form = CalculationForm::new();
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
        let mut form = CalculationForm::new();
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
    fn test_select_current_star() {
        let mut form = CalculationForm::new();
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
    fn test_star_name_field_in_automatic_fields() {
        let fields = InputField::automatic_fields();
        assert!(fields.contains(&InputField::StarName));
    }

    #[test]
    fn test_star_lookup_automatic_mode() {
        let mut form = CalculationForm::new();
        form.sextant_altitude = "45 30".to_string();
        form.date = "2024-01-15".to_string();
        form.time = "12:00:00".to_string();
        form.latitude = "40 15".to_string();
        form.latitude_direction = 'N';
        form.longitude = "74 0".to_string();
        form.longitude_direction = 'E';
        form.index_error = "0".to_string();
        form.height_of_eye = "10".to_string();
        form.celestial_body = CelestialBody::Star;
        form.star_name = "Sirius".to_string();
        form.mode = CalculationMode::Automatic;

        form.calculate();

        // Should succeed with valid star name
        assert!(form.results.is_some());
        assert!(form.error_message.is_none());

        let results = form.results.as_ref().unwrap();
        assert!(results.computed_altitude >= -90.0 && results.computed_altitude <= 90.0);
        assert!(results.azimuth >= 0.0 && results.azimuth <= 360.0);
    }

    #[test]
    fn test_star_lookup_missing_name() {
        let mut form = CalculationForm::new();
        form.sextant_altitude = "45 30".to_string();
        form.date = "2024-01-15".to_string();
        form.time = "12:00:00".to_string();
        form.latitude = "40 15".to_string();
        form.latitude_direction = 'N';
        form.longitude = "74 0".to_string();
        form.longitude_direction = 'E';
        form.index_error = "0".to_string();
        form.height_of_eye = "10".to_string();
        form.celestial_body = CelestialBody::Star;
        form.star_name = String::new(); // Empty star name
        form.mode = CalculationMode::Automatic;

        form.calculate();

        // Should fail with error message
        assert!(form.results.is_none());
        assert!(form.error_message.is_some());
        let error = form.error_message.as_ref().unwrap();
        assert!(error.contains("Star name is required"));
    }

    #[test]
    fn test_star_lookup_invalid_name() {
        let mut form = CalculationForm::new();
        form.sextant_altitude = "45 30".to_string();
        form.date = "2024-01-15".to_string();
        form.time = "12:00:00".to_string();
        form.latitude = "40 15".to_string();
        form.latitude_direction = 'N';
        form.longitude = "74 0".to_string();
        form.longitude_direction = 'E';
        form.index_error = "0".to_string();
        form.height_of_eye = "10".to_string();
        form.celestial_body = CelestialBody::Star;
        form.star_name = "InvalidStar".to_string();
        form.mode = CalculationMode::Automatic;

        form.calculate();

        // Should fail with error about star not found
        assert!(form.results.is_none());
        assert!(form.error_message.is_some());
        let error = form.error_message.as_ref().unwrap();
        assert!(error.contains("not found"));
    }

    #[test]
    fn test_set_field_value_updates_star_filter() {
        let mut form = CalculationForm::new();
        form.set_field_value(InputField::StarName, "sir".to_string());

        // Filter should be automatically updated
        assert_eq!(form.star_name, "sir");
        assert_eq!(form.star_filter_matches.len(), 1);
        assert_eq!(form.star_filter_matches[0], "Sirius");
    }

    #[test]
    fn test_celestial_body_cycling_includes_star() {
        let mut form = CalculationForm::new();
        let bodies = CelestialBody::all();
        assert_eq!(bodies.len(), 8); // Sun, Moon, 4 planets, Aries, Star
        assert!(bodies.contains(&CelestialBody::Star));

        // Cycle through to find Star
        form.celestial_body = CelestialBody::Aries;
        form.next_celestial_body();
        assert_eq!(form.celestial_body, CelestialBody::Star);
    }

    // Text input active tests for Phase 2

    #[test]
    fn test_is_text_input_active_star_name() {
        let mut form = CalculationForm::new();
        form.celestial_body = CelestialBody::Star;
        form.current_field = InputField::StarName;
        assert!(form.is_text_input_active());
    }

    #[test]
    fn test_is_text_input_active_date() {
        let mut form = CalculationForm::new();
        form.current_field = InputField::Date;
        assert!(form.is_text_input_active());
    }

    #[test]
    fn test_is_text_input_active_time() {
        let mut form = CalculationForm::new();
        form.current_field = InputField::Time;
        assert!(form.is_text_input_active());
    }

    #[test]
    fn test_is_text_input_active_latitude() {
        let mut form = CalculationForm::new();
        form.current_field = InputField::Latitude;
        assert!(form.is_text_input_active());
    }

    #[test]
    fn test_is_text_input_active_celestial_body() {
        let mut form = CalculationForm::new();
        form.current_field = InputField::CelestialBody;
        assert!(!form.is_text_input_active()); // CelestialBody uses +/- cycling, not text input
    }

    #[test]
    fn test_is_text_input_active_latitude_direction() {
        let mut form = CalculationForm::new();
        form.current_field = InputField::LatitudeDirection;
        assert!(!form.is_text_input_active()); // Direction fields use N/S/E/W, not free-form text
    }

    // TDD tests for field order - directions should follow their values

    #[test]
    fn test_field_order_latitude_direction_follows_latitude() {
        let mut form = CalculationForm::new();
        form.current_field = InputField::Latitude;

        // Navigating forward from Latitude should go to LatitudeDirection
        form.next_field();
        assert_eq!(form.current_field, InputField::LatitudeDirection,
            "LatitudeDirection should immediately follow Latitude");
    }

    #[test]
    fn test_field_order_longitude_direction_follows_longitude() {
        let mut form = CalculationForm::new();
        form.current_field = InputField::Longitude;

        // Navigating forward from Longitude should go to LongitudeDirection
        form.next_field();
        assert_eq!(form.current_field, InputField::LongitudeDirection,
            "LongitudeDirection should immediately follow Longitude");
    }

    #[test]
    fn test_field_order_declination_direction_follows_declination() {
        let mut form = CalculationForm::new();
        form.mode = CalculationMode::Manual;
        form.current_field = InputField::Declination;

        // Navigating forward from Declination should go to DeclinationDirection
        form.next_field();
        assert_eq!(form.current_field, InputField::DeclinationDirection,
            "DeclinationDirection should immediately follow Declination");
    }

    #[test]
    fn test_field_order_backwards_direction_precedes_value() {
        let mut form = CalculationForm::new();
        form.current_field = InputField::LatitudeDirection;

        // Navigating backward from LatitudeDirection should go to Latitude
        form.previous_field();
        assert_eq!(form.current_field, InputField::Latitude,
            "Latitude should immediately precede LatitudeDirection");
    }
}
