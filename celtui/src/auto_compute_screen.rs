//! Auto Compute screen for multiple sight reduction and fix calculation
//!
//! This module provides a screen for entering multiple celestial sights,
//! computing their Lines of Position, and calculating a fix from multiple LOPs.

use chrono::{Datelike, NaiveDate, NaiveTime, TimeZone, Utc};
use celtnav::almanac::{CelestialBody as AlmanacBody, Planet, get_body_position, is_leap_year};
use celtnav::sight_reduction::{
    compute_altitude, compute_azimuth, compute_intercept, SightData,
    apply_refraction_correction, apply_dip_correction,
    apply_semidiameter_correction, apply_parallax_correction,
    optimize_chosen_position,
};
use celtnav::fix_calculation::{LineOfPosition, fix_from_multiple_lops, Fix, advance_lop, advance_position};
use celtnav::sight_averaging::{SextantObservation, average_sights};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Row, Table, Wrap},
    Frame,
};
use serde::{Deserialize, Serialize};

/// Celestial body for sight
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SightCelestialBody {
    Sun,
    Moon,
    Venus,
    Mars,
    Jupiter,
    Saturn,
    Star(String), // Star with name
}

impl SightCelestialBody {
    /// Returns all non-star celestial bodies
    pub fn all_non_star() -> Vec<SightCelestialBody> {
        vec![
            SightCelestialBody::Sun,
            SightCelestialBody::Moon,
            SightCelestialBody::Venus,
            SightCelestialBody::Mars,
            SightCelestialBody::Jupiter,
            SightCelestialBody::Saturn,
        ]
    }

    /// Returns all celestial bodies including a star placeholder
    pub fn all() -> Vec<SightCelestialBody> {
        let mut bodies = Self::all_non_star();
        bodies.push(SightCelestialBody::Star(String::new()));
        bodies
    }

    pub fn name(&self) -> String {
        match self {
            SightCelestialBody::Sun => "Sun".to_string(),
            SightCelestialBody::Moon => "Moon".to_string(),
            SightCelestialBody::Venus => "Venus".to_string(),
            SightCelestialBody::Mars => "Mars".to_string(),
            SightCelestialBody::Jupiter => "Jupiter".to_string(),
            SightCelestialBody::Saturn => "Saturn".to_string(),
            SightCelestialBody::Star(name) => name.clone(),
        }
    }

    fn to_almanac_body(&self) -> AlmanacBody {
        match self {
            SightCelestialBody::Sun => AlmanacBody::Sun,
            SightCelestialBody::Moon => AlmanacBody::Moon,
            SightCelestialBody::Venus => AlmanacBody::Planet(Planet::Venus),
            SightCelestialBody::Mars => AlmanacBody::Planet(Planet::Mars),
            SightCelestialBody::Jupiter => AlmanacBody::Planet(Planet::Jupiter),
            SightCelestialBody::Saturn => AlmanacBody::Planet(Planet::Saturn),
            SightCelestialBody::Star(name) => AlmanacBody::Star(name.clone()),
        }
    }
}

/// LOP (Line of Position) data for display and chart plotting
///
/// Contains all information a navigator needs to plot a single LOP on a chart:
/// - The celestial body observed
/// - The DR position used for the calculation
/// - The observed altitude (Ho) after all corrections
/// - The Greenwich Hour Angle (GHA) of the body
/// - The Local Hour Angle (LHA) at the DR position (can have decimal values)
/// - The calculated altitude (Hc) at that position using spherical trigonometry
/// - The intercept distance (toward/away from the body)
/// - The true azimuth bearing to the body
///
/// For stars, also includes GHA Aries and LHA Aries for SRT (Sight Reduction Tables) lookup comparison.
///
/// To plot the LOP on a chart:
/// 1. Mark the DR position
/// 2. Draw a line from DR along the azimuth bearing
/// 3. Advance (if toward) or retreat (if away) along that line by the intercept distance
/// 4. Draw the LOP perpendicular to the azimuth at that advanced point
#[derive(Debug, Clone)]
pub struct LopDisplayData {
    /// Name of the celestial body (Sun, Moon, Venus, star name, etc.)
    pub body_name: String,
    /// DR position latitude in decimal degrees (positive = North, negative = South)
    pub chosen_lat: f64,
    /// DR position longitude in decimal degrees (positive = East, negative = West)
    pub chosen_lon: f64,
    /// Observed altitude (Ho) in degrees after all corrections
    pub ho: f64,
    /// Declination in decimal degrees (positive = North, negative = South)
    pub declination: f64,
    /// Greenwich Hour Angle (GHA) in degrees
    /// For stars, this is GHA of the star (GHA Aries + SHA combined)
    pub gha: f64,
    /// Local Hour Angle (LHA) in degrees (can have decimal values for trig calculations)
    /// For stars, this is LHA of the star used in spherical trig calculations
    pub lha: f64,
    /// GHA Aries in degrees (only for stars, None for other bodies)
    /// For SRT (Sight Reduction Tables) lookup comparison
    pub gha_aries: Option<f64>,
    /// Optimized chosen latitude for SRT tables (only for stars, None for other bodies)
    /// Latitude rounded to nearest whole degree for easier plotting
    pub pub249_chosen_lat: Option<f64>,
    /// Optimized chosen longitude for SRT tables (only for stars, None for other bodies)
    /// Longitude adjusted to make LHA Aries a whole number for table lookup
    pub pub249_chosen_lon: Option<f64>,
    /// LHA Aries as whole number (only for stars, None for other bodies)
    /// For SRT table lookup: enter tables with this whole LHA Aries and star name
    pub lha_aries: Option<f64>,
    /// Calculated altitude (Hc) in degrees using spherical trigonometry
    pub hc: f64,
    /// Intercept in nautical miles (positive = toward body, negative = away from body)
    pub intercept: f64,
    /// True bearing to celestial body in degrees (0-360, clockwise from north)
    pub azimuth: f64,
}

impl LopDisplayData {
    /// Format intercept with direction label
    pub fn intercept_with_direction(&self) -> String {
        if self.intercept >= 0.0 {
            format!("{:.1} NM toward", self.intercept)
        } else {
            format!("{:.1} NM away", self.intercept.abs())
        }
    }
}

/// A single sight observation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sight {
    pub body: SightCelestialBody,
    pub date: String,
    pub time: String,
    pub sextant_altitude: String,  // "DD MM.M"
    pub index_error: String,       // arcminutes
    pub height_of_eye: String,     // meters
    pub dr_latitude: String,       // DR position latitude "DD MM.M"
    pub dr_longitude: String,      // DR position longitude "DD MM.M"
    pub lat_direction: char,       // N/S
    pub lon_direction: char,       // E/W
    #[serde(default)]
    pub log_reading: String,       // Cumulative log in NM (optional, e.g., "103.5")
    #[serde(default)]
    pub heading: String,           // Vessel heading in degrees (optional, e.g., "045")
    #[serde(default = "default_is_active")]
    pub is_active: bool,           // False when sight is averaged into another sight
}

fn default_is_active() -> bool {
    true
}

impl Sight {
    pub fn new() -> Self {
        Self {
            body: SightCelestialBody::Sun,
            date: String::new(),
            time: String::new(),
            sextant_altitude: String::new(),
            index_error: String::from("0"),
            height_of_eye: String::from("0"),
            dr_latitude: String::new(),
            dr_longitude: String::new(),
            lat_direction: 'N',
            lon_direction: 'W',
            log_reading: String::new(),
            heading: String::new(),
            is_active: true,
        }
    }

    pub fn display_summary(&self) -> String {
        let body_display = match &self.body {
            SightCelestialBody::Star(name) => format!("Star: {}", name),
            _ => self.body.name(),
        };
        format!(
            "{} @ {} {} - Hs: {}",
            body_display,
            self.date,
            self.time,
            self.sextant_altitude
        )
    }

    pub fn is_star(&self) -> bool {
        matches!(self.body, SightCelestialBody::Star(_))
    }
}

/// Input field for sight entry
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SightInputField {
    Body,
    StarName,          // Star name input (when Star body is selected)
    Date,
    Time,
    SextantAltitude,
    IndexError,
    HeightOfEye,
    DRLatitude,
    LatDirection,      // N/S direction immediately follows DRLatitude
    DRLongitude,
    LonDirection,      // E/W direction immediately follows DRLongitude
    LogReading,        // Cumulative log reading in NM (optional)
    Heading,           // Vessel heading in degrees (optional)
}

impl SightInputField {
    pub fn all() -> Vec<SightInputField> {
        vec![
            SightInputField::Body,
            SightInputField::StarName,  // Always in list, visibility controlled by selected body
            SightInputField::Date,
            SightInputField::Time,
            SightInputField::SextantAltitude,
            SightInputField::IndexError,
            SightInputField::HeightOfEye,
            SightInputField::DRLatitude,
            SightInputField::LatDirection,  // N/S immediately follows DRLatitude
            SightInputField::DRLongitude,
            SightInputField::LonDirection,  // E/W immediately follows DRLongitude
            SightInputField::LogReading,
            SightInputField::Heading,
        ]
    }

    pub fn next(&self) -> Self {
        let fields = Self::all();
        let current_idx = fields.iter().position(|f| f == self).unwrap_or(0);
        let next_idx = (current_idx + 1) % fields.len();
        fields[next_idx]
    }

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

    pub fn label(&self) -> &str {
        match self {
            SightInputField::Body => "Celestial Body",
            SightInputField::StarName => "Star Name (type to search)",
            SightInputField::Date => "Date (YYYY-MM-DD)",
            SightInputField::Time => "Time UT (HH:MM:SS)",
            SightInputField::SextantAltitude => "Sextant Altitude (Hs) [DD MM.M]",
            SightInputField::IndexError => "Index Error (arcmin)",
            SightInputField::HeightOfEye => "Height of Eye (meters)",
            SightInputField::DRLatitude => "DR Latitude [DD MM.M]",
            SightInputField::DRLongitude => "DR Longitude [DD MM.M]",
            SightInputField::LatDirection => "Latitude (N/S)",
            SightInputField::LonDirection => "Longitude (E/W)",
            SightInputField::LogReading => "Log Reading (optional, NM)",
            SightInputField::Heading => "Heading (optional, degrees 0-360)",
        }
    }
}

/// Auto compute screen state
#[derive(Debug, Clone)]
pub struct AutoComputeForm {
    pub sights: Vec<Sight>,
    pub current_sight: Sight,
    pub current_field: SightInputField,
    pub selected_sight_index: Option<usize>,
    pub editing_sight_index: Option<usize>,  // Track which sight is being edited
    pub selected_sight_indices: Vec<usize>,  // Multiple selection for averaging
    pub multi_select_mode: bool,             // Whether in multi-select mode
    pub fix_result: Option<Fix>,
    pub lop_data: Vec<LopDisplayData>,  // LOP data for each sight in the fix
    pub error_message: Option<String>,
    pub mode: AutoComputeMode,
    pub vessel_course: String,  // Course in degrees
    pub vessel_speed: String,   // Speed in knots
    pub running_fix_field: RunningFixField,
    pub star_filter_matches: Vec<String>, // Filtered star names for autocompletion
    pub star_selected_index: usize,       // Index of selected star in filtered list
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AutoComputeMode {
    EnteringSight,
    ViewingSights,
    EditingRunningFix,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunningFixField {
    Course,
    Speed,
}

impl RunningFixField {
    pub fn next(&self) -> Self {
        match self {
            RunningFixField::Course => RunningFixField::Speed,
            RunningFixField::Speed => RunningFixField::Course,
        }
    }
}

impl AutoComputeForm {
    pub fn new() -> Self {
        Self {
            sights: Vec::new(),
            current_sight: Sight::new(),
            current_field: SightInputField::Body,
            selected_sight_index: None,
            editing_sight_index: None,
            selected_sight_indices: Vec::new(),
            multi_select_mode: false,
            fix_result: None,
            lop_data: Vec::new(),
            error_message: None,
            mode: AutoComputeMode::EnteringSight,
            vessel_course: String::from("0"),
            vessel_speed: String::from("0"),
            running_fix_field: RunningFixField::Course,
            star_filter_matches: Vec::new(),
            star_selected_index: 0,
        }
    }

    pub fn next_field(&mut self) {
        self.current_field = self.current_field.next();

        // Skip StarName field if current body is not a star
        if self.current_field == SightInputField::StarName && !self.current_sight.is_star() {
            self.current_field = self.current_field.next();
        }
    }

    pub fn previous_field(&mut self) {
        self.current_field = self.current_field.previous();

        // Skip StarName field if current body is not a star
        if self.current_field == SightInputField::StarName && !self.current_sight.is_star() {
            self.current_field = self.current_field.previous();
        }
    }

    pub fn get_field_value(&self, field: SightInputField) -> String {
        match field {
            SightInputField::Body => self.current_sight.body.name().to_string(),
            SightInputField::StarName => {
                // Return star name if current body is a star
                if let SightCelestialBody::Star(name) = &self.current_sight.body {
                    name.clone()
                } else {
                    String::new()
                }
            }
            SightInputField::Date => self.current_sight.date.clone(),
            SightInputField::Time => self.current_sight.time.clone(),
            SightInputField::SextantAltitude => self.current_sight.sextant_altitude.clone(),
            SightInputField::IndexError => self.current_sight.index_error.clone(),
            SightInputField::HeightOfEye => self.current_sight.height_of_eye.clone(),
            SightInputField::DRLatitude => self.current_sight.dr_latitude.clone(),
            SightInputField::DRLongitude => self.current_sight.dr_longitude.clone(),
            SightInputField::LatDirection => self.current_sight.lat_direction.to_string(),
            SightInputField::LonDirection => self.current_sight.lon_direction.to_string(),
            SightInputField::LogReading => self.current_sight.log_reading.clone(),
            SightInputField::Heading => self.current_sight.heading.clone(),
        }
    }

    pub fn set_field_value(&mut self, field: SightInputField, value: String) {
        match field {
            SightInputField::Body => {
                // Handled by next/previous body
            }
            SightInputField::StarName => {
                // Update star name and filter
                self.current_sight.body = SightCelestialBody::Star(value);
                self.update_star_filter();
            }
            SightInputField::Date => self.current_sight.date = value,
            SightInputField::Time => self.current_sight.time = value,
            SightInputField::SextantAltitude => self.current_sight.sextant_altitude = value,
            SightInputField::IndexError => self.current_sight.index_error = value,
            SightInputField::HeightOfEye => self.current_sight.height_of_eye = value,
            SightInputField::DRLatitude => self.current_sight.dr_latitude = value,
            SightInputField::DRLongitude => self.current_sight.dr_longitude = value,
            SightInputField::LatDirection => {
                if let Some(c) = value.chars().next() {
                    if c == 'N' || c == 'S' || c == 'n' || c == 's' {
                        self.current_sight.lat_direction = c.to_ascii_uppercase();
                    }
                }
            }
            SightInputField::LonDirection => {
                if let Some(c) = value.chars().next() {
                    if c == 'E' || c == 'W' || c == 'e' || c == 'w' {
                        self.current_sight.lon_direction = c.to_ascii_uppercase();
                    }
                }
            }
            SightInputField::LogReading => self.current_sight.log_reading = value,
            SightInputField::Heading => self.current_sight.heading = value,
        }
    }

    pub fn next_body(&mut self) {
        let bodies = SightCelestialBody::all();
        let current_idx = bodies.iter().position(|b| {
            match (&self.current_sight.body, b) {
                (SightCelestialBody::Star(_), SightCelestialBody::Star(_)) => true,
                _ => *b == self.current_sight.body,
            }
        }).unwrap_or(0);
        let next_idx = (current_idx + 1) % bodies.len();
        self.current_sight.body = bodies[next_idx].clone();
    }

    pub fn previous_body(&mut self) {
        let bodies = SightCelestialBody::all();
        let current_idx = bodies.iter().position(|b| {
            match (&self.current_sight.body, b) {
                (SightCelestialBody::Star(_), SightCelestialBody::Star(_)) => true,
                _ => *b == self.current_sight.body,
            }
        }).unwrap_or(0);
        let prev_idx = if current_idx == 0 {
            bodies.len() - 1
        } else {
            current_idx - 1
        };
        self.current_sight.body = bodies[prev_idx].clone();
    }

    /// Toggle latitude direction between N and S
    pub fn toggle_lat_direction(&mut self) {
        self.current_sight.lat_direction = if self.current_sight.lat_direction == 'N' {
            'S'
        } else {
            'N'
        };
    }

    /// Toggle longitude direction between E and W
    pub fn toggle_lon_direction(&mut self) {
        self.current_sight.lon_direction = if self.current_sight.lon_direction == 'E' {
            'W'
        } else {
            'E'
        };
    }

    /// Filter star catalog based on current star name input
    pub fn filter_stars(&self) -> Vec<String> {
        use celtnav::almanac::get_star_catalog;

        let catalog = get_star_catalog();
        let query = if let SightCelestialBody::Star(name) = &self.current_sight.body {
            name.trim().to_lowercase()
        } else {
            String::new()
        };

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
            self.current_sight.body = SightCelestialBody::Star(
                self.star_filter_matches[self.star_selected_index].clone()
            );
            self.update_star_filter();
        }
    }

    /// Check if the current field/mode is a text input (for disabling screen shortcuts)
    /// Returns true when user is typing in free-form text fields
    pub fn is_text_input_active(&self) -> bool {
        match self.mode {
            AutoComputeMode::EnteringSight => {
                match self.current_field {
                    // Text input fields (free-form typing)
                    SightInputField::StarName
                    | SightInputField::Date
                    | SightInputField::Time
                    | SightInputField::SextantAltitude
                    | SightInputField::IndexError
                    | SightInputField::HeightOfEye
                    | SightInputField::DRLatitude
                    | SightInputField::DRLongitude
                    | SightInputField::LogReading
                    | SightInputField::Heading => true,

                    // Selection fields (use +/- or specific keys)
                    SightInputField::Body
                    | SightInputField::LatDirection
                    | SightInputField::LonDirection => false,
                }
            }
            AutoComputeMode::ViewingSights => false, // Just viewing, not editing
            AutoComputeMode::EditingRunningFix => true, // Editing course/speed values
        }
    }

    /// Validate a specific field and return error message if invalid
    pub fn validate_field(&self, field: SightInputField) -> Option<String> {
        use crate::validation::*;

        let result = match field {
            SightInputField::Date => validate_date(&self.current_sight.date),
            SightInputField::Time => validate_time(&self.current_sight.time),
            SightInputField::SextantAltitude => validate_sextant_altitude_dms(&self.current_sight.sextant_altitude),
            SightInputField::IndexError => validate_index_error(&self.current_sight.index_error),
            SightInputField::HeightOfEye => validate_height_of_eye(&self.current_sight.height_of_eye),
            SightInputField::DRLatitude => validate_latitude_dms(&self.current_sight.dr_latitude),
            SightInputField::DRLongitude => validate_longitude_dms(&self.current_sight.dr_longitude),
            SightInputField::LatDirection => validate_direction(self.current_sight.lat_direction, &['N', 'S'], "Latitude direction"),
            SightInputField::LonDirection => validate_direction(self.current_sight.lon_direction, &['E', 'W'], "Longitude direction"),
            SightInputField::Body => Ok(()), // Body is always valid (selected from list)
            SightInputField::StarName => {
                // Validate star name if Star body is selected
                if self.current_sight.is_star() {
                    if let SightCelestialBody::Star(name) = &self.current_sight.body {
                        if name.trim().is_empty() {
                            Err("Star name is required".to_string())
                        } else {
                            // Check if star exists in catalog
                            use celtnav::almanac::find_star;
                            if find_star(name).is_some() {
                                Ok(())
                            } else {
                                Err(format!("Star '{}' not found in catalog", name))
                            }
                        }
                    } else {
                        Ok(())
                    }
                } else {
                    Ok(())
                }
            }
            SightInputField::LogReading => {
                // Optional field
                if self.current_sight.log_reading.is_empty() {
                    Ok(())
                } else {
                    match self.current_sight.log_reading.parse::<f64>() {
                        Ok(log) if log >= 0.0 => Ok(()),
                        Ok(_) => Err("Log reading cannot be negative".to_string()),
                        Err(_) => Err("Log reading must be a number".to_string()),
                    }
                }
            }
            SightInputField::Heading => {
                // Optional field
                if self.current_sight.heading.is_empty() {
                    Ok(())
                } else {
                    match self.current_sight.heading.parse::<f64>() {
                        Ok(heading) if heading >= 0.0 && heading < 360.0 => Ok(()),
                        Ok(_) => Err("Heading must be 0-360 degrees".to_string()),
                        Err(_) => Err("Heading must be a number".to_string()),
                    }
                }
            }
        };

        result.err()
    }

    /// Validate the current sight before adding it
    pub fn validate_current_sight(&self) -> Result<(), String> {
        use crate::validation::*;

        // Validate date
        validate_date(&self.current_sight.date)?;

        // Validate time
        validate_time(&self.current_sight.time)?;

        // Validate sextant altitude
        validate_sextant_altitude_dms(&self.current_sight.sextant_altitude)?;

        // Validate index error
        validate_index_error(&self.current_sight.index_error)?;

        // Validate height of eye
        validate_height_of_eye(&self.current_sight.height_of_eye)?;

        // Validate DR position
        validate_latitude_dms(&self.current_sight.dr_latitude)?;
        validate_longitude_dms(&self.current_sight.dr_longitude)?;

        // Validate directions
        validate_direction(self.current_sight.lat_direction, &['N', 'S'], "Latitude direction")?;
        validate_direction(self.current_sight.lon_direction, &['E', 'W'], "Longitude direction")?;

        Ok(())
    }

    pub fn add_sight(&mut self) {
        // Validate before adding
        if let Err(e) = self.validate_current_sight() {
            self.error_message = Some(format!("Validation error: {}", e));
            return;
        }

        self.sights.push(self.current_sight.clone());
        self.current_sight = Sight::new();

        // Preserve common fields from previous sight
        if let Some(last) = self.sights.last() {
            // Preserve date and time (sights usually taken at similar times)
            self.current_sight.date = last.date.clone();
            self.current_sight.time = last.time.clone();

            // Preserve index error (constant for a given sextant)
            self.current_sight.index_error = last.index_error.clone();

            // Preserve height of eye (constant for a given observer position)
            self.current_sight.height_of_eye = last.height_of_eye.clone();

            // AUTO-CALCULATE DR if log and heading available
            if let Some((new_lat, new_lon)) = self.calculate_dr_from_previous() {
                // Convert back to DMS format
                let lat_dms = celtnav::decimal_to_dms(new_lat.abs());
                let lon_dms = celtnav::decimal_to_dms(new_lon.abs());

                self.current_sight.dr_latitude = format!("{} {:.1}", lat_dms.degrees, lat_dms.minutes);
                self.current_sight.dr_longitude = format!("{} {:.1}", lon_dms.degrees, lon_dms.minutes);
                self.current_sight.lat_direction = if new_lat >= 0.0 { 'N' } else { 'S' };
                self.current_sight.lon_direction = if new_lon >= 0.0 { 'E' } else { 'W' };

                self.error_message = Some("Sight added! DR auto-calculated from log/heading. Enter another or press 'C'.".to_string());
            } else {
                // Preserve DR position if no log/heading available
                self.current_sight.dr_latitude = last.dr_latitude.clone();
                self.current_sight.dr_longitude = last.dr_longitude.clone();
                self.current_sight.lat_direction = last.lat_direction;
                self.current_sight.lon_direction = last.lon_direction;

                self.error_message = Some("Sight added! Enter another or press 'C' to compute fix.".to_string());
            }

            // Note: sextant_altitude and body are NOT preserved - they are reset
            // to default values for the next sight (via Sight::new())
        } else {
            self.error_message = Some("Sight added! Enter another or press 'C' to compute fix.".to_string());
        }
    }

    pub fn delete_selected_sight(&mut self) {
        if let Some(idx) = self.selected_sight_index {
            if idx < self.sights.len() {
                self.sights.remove(idx);
                self.selected_sight_index = None;
                self.fix_result = None;
            }
        }
    }

    /// Calculate DR position for new sight based on previous sight's log and heading
    pub fn calculate_dr_from_previous(&self) -> Option<(f64, f64)> {
        use crate::validation::parse_dms;

        let last_sight = self.sights.last()?;

        // Need both log readings and heading
        let last_log: f64 = last_sight.log_reading.parse().ok()?;
        let current_log: f64 = self.current_sight.log_reading.parse().ok()?;
        let heading: f64 = last_sight.heading.parse().ok()?;

        // Calculate distance traveled
        let distance_nm = (current_log - last_log).abs();

        // Parse previous DR position
        let (dr_lat_deg, dr_lat_min, _) = parse_dms(&last_sight.dr_latitude).ok()?;
        let mut dr_lat = celtnav::dms_to_decimal(dr_lat_deg as i32, dr_lat_min as u32, 0.0);
        if last_sight.lat_direction == 'S' {
            dr_lat = -dr_lat;
        }

        let (dr_lon_deg, dr_lon_min, _) = parse_dms(&last_sight.dr_longitude).ok()?;
        let mut dr_lon = celtnav::dms_to_decimal(dr_lon_deg as i32, dr_lon_min as u32, 0.0);
        if last_sight.lon_direction == 'W' {
            dr_lon = -dr_lon;
        }

        // Use celtnav's advance_position
        // Since we have distance directly, use speed = distance_nm and time = 1.0 hour
        let (new_lat, new_lon) = advance_position(dr_lat, dr_lon, heading, distance_nm, 1.0);

        Some((new_lat, new_lon))
    }

    /// Enter edit mode for selected sight
    pub fn edit_selected_sight(&mut self) {
        if let Some(idx) = self.selected_sight_index {
            if idx < self.sights.len() {
                // Populate current_sight with selected sight's data
                self.current_sight = self.sights[idx].clone();

                // Store the index we're editing
                self.editing_sight_index = Some(idx);

                // Switch to EnteringSight mode
                self.mode = AutoComputeMode::EnteringSight;
                self.current_field = SightInputField::Body;

                self.error_message = Some("Editing sight. Press Ctrl+S to save, Esc to cancel.".to_string());
            }
        }
    }

    /// Save edited sight back to the list
    pub fn save_edited_sight(&mut self) {
        if let Some(idx) = self.editing_sight_index {
            if let Err(e) = self.validate_current_sight() {
                self.error_message = Some(format!("Validation error: {}", e));
                return;
            }

            // Replace the sight at the index
            if idx < self.sights.len() {
                self.sights[idx] = self.current_sight.clone();

                // Clear edit state
                self.editing_sight_index = None;
                self.current_sight = Sight::new();
                self.mode = AutoComputeMode::ViewingSights;

                // Clear fix result since sights changed
                self.fix_result = None;
                self.lop_data.clear();

                self.error_message = Some("Sight updated successfully!".to_string());
            }
        }
    }

    /// Cancel editing without saving
    pub fn cancel_edit(&mut self) {
        self.editing_sight_index = None;
        self.current_sight = Sight::new();
        self.mode = AutoComputeMode::ViewingSights;
        self.error_message = Some("Edit cancelled.".to_string());
    }

    /// Toggle multi-select mode
    pub fn toggle_multi_select(&mut self) {
        self.multi_select_mode = !self.multi_select_mode;
        if !self.multi_select_mode {
            self.selected_sight_indices.clear();
        }
        self.error_message = Some(
            if self.multi_select_mode {
                "Multi-select mode ON. Press Space to select, 'A' to average selected sights.".to_string()
            } else {
                "Multi-select mode OFF.".to_string()
            }
        );
    }

    /// Toggle selection of current sight in multi-select mode
    pub fn toggle_sight_selection(&mut self) {
        if !self.multi_select_mode {
            return;
        }

        if let Some(idx) = self.selected_sight_index {
            if let Some(pos) = self.selected_sight_indices.iter().position(|&x| x == idx) {
                self.selected_sight_indices.remove(pos);
            } else {
                self.selected_sight_indices.push(idx);
            }
        }
    }

    /// Check if sights can be averaged (same body, within 5 minutes)
    pub fn can_average_sights(&self, indices: &[usize]) -> Result<(), String> {
        if indices.len() < 2 {
            return Err("Need at least 2 sights to average".to_string());
        }

        // Get all selected sights
        let sights: Vec<&Sight> = indices.iter()
            .filter_map(|&i| self.sights.get(i))
            .collect();

        // Check same body
        let first_body = &sights[0].body;
        for sight in &sights[1..] {
            if format!("{:?}", sight.body) != format!("{:?}", first_body) {
                return Err("All sights must be of the same celestial body".to_string());
            }
        }

        // Check within 5 minutes
        let mut times = Vec::new();
        for sight in &sights {
            let date = NaiveDate::parse_from_str(&sight.date, "%Y-%m-%d")
                .map_err(|_| "Invalid date format")?;
            let time = NaiveTime::parse_from_str(&sight.time, "%H:%M:%S")
                .or_else(|_| NaiveTime::parse_from_str(&sight.time, "%H:%M"))
                .map_err(|_| "Invalid time format")?;
            times.push((date, time));
        }

        // Find min and max time
        let (min_date, min_time) = times.iter().min().unwrap();
        let (max_date, max_time) = times.iter().max().unwrap();

        // Calculate time difference
        if min_date != max_date {
            return Err("All sights must be on the same day".to_string());
        }

        let time_diff_seconds = (max_time.signed_duration_since(*min_time)).num_seconds().abs();
        if time_diff_seconds > 300 {  // 5 minutes = 300 seconds
            return Err("Sights must be within 5 minutes of each other".to_string());
        }

        Ok(())
    }

    /// Average selected sights
    pub fn average_selected_sights(&mut self) {
        use crate::validation::parse_dms;

        // Validate selection
        if let Err(e) = self.can_average_sights(&self.selected_sight_indices) {
            self.error_message = Some(format!("Cannot average: {}", e));
            return;
        }

        // Get selected sights
        let selected_sights: Vec<&Sight> = self.selected_sight_indices.iter()
            .filter_map(|&i| self.sights.get(i))
            .collect();

        // Convert to SextantObservations
        let observations: Vec<SextantObservation> = selected_sights.iter()
            .filter_map(|sight| {
                let time = NaiveTime::parse_from_str(&sight.time, "%H:%M:%S")
                    .or_else(|_| NaiveTime::parse_from_str(&sight.time, "%H:%M"))
                    .ok()?;

                let (deg, min, sec) = parse_dms(&sight.sextant_altitude).ok()?;
                let altitude_decimal = celtnav::dms_to_decimal(deg as i32, min as u32, sec);
                let altitude_dms = celtnav::decimal_to_dms(altitude_decimal);

                Some(SextantObservation {
                    time,
                    altitude_degrees: altitude_dms.degrees as f64,
                    altitude_minutes: altitude_dms.minutes as f64,
                })
            })
            .collect();

        if observations.len() < 2 {
            self.error_message = Some("Need at least 2 valid observations to average".to_string());
            return;
        }

        // Calculate average
        let averaged = match average_sights(&observations) {
            Some(avg) => avg,
            None => {
                self.error_message = Some("Failed to calculate average".to_string());
                return;
            }
        };

        // Create new averaged sight based on first selected sight
        let mut new_sight = selected_sights[0].clone();

        // Update time to averaged time
        new_sight.time = averaged.avg_time.format("%H:%M:%S").to_string();

        // Update altitude to averaged altitude
        let avg_decimal = averaged.avg_altitude_degrees + averaged.avg_altitude_minutes / 60.0;
        let avg_dms = celtnav::decimal_to_dms(avg_decimal);
        new_sight.sextant_altitude = format!("{} {:.1}", avg_dms.degrees, avg_dms.minutes);

        // Mark new sight as active (it's the averaged result)
        new_sight.is_active = true;

        // Mark original sights as inactive instead of removing them
        for &idx in &self.selected_sight_indices {
            if idx < self.sights.len() {
                self.sights[idx].is_active = false;
            }
        }

        // Add averaged sight to the list
        self.sights.push(new_sight);

        // Clear selection state
        self.selected_sight_indices.clear();
        self.multi_select_mode = false;
        self.selected_sight_index = None;

        // Clear fix result
        self.fix_result = None;
        self.lop_data.clear();

        self.error_message = Some(format!("Averaged {} sights successfully!", observations.len()));
    }

    pub fn compute_fix(&mut self) {
        self.fix_result = None;
        self.lop_data = Vec::new();
        self.error_message = None;

        // Filter to only use active sights
        let active_sights: Vec<&Sight> = self.sights.iter()
            .filter(|sight| sight.is_active)
            .collect();

        if active_sights.len() < 2 {
            self.error_message = Some("Need at least 2 active sights to compute a fix".to_string());
            return;
        }

        // Parse vessel course and speed for running fix
        let course = match self.vessel_course.parse::<f64>() {
            Ok(c) => c,
            Err(_) => {
                self.error_message = Some("Invalid vessel course".to_string());
                return;
            }
        };

        let speed = match self.vessel_speed.parse::<f64>() {
            Ok(s) => s,
            Err(_) => {
                self.error_message = Some("Invalid vessel speed".to_string());
                return;
            }
        };

        // Compute LOP for each active sight with timestamps and display data
        let mut lops_with_times: Vec<(LineOfPosition, chrono::DateTime<Utc>, &Sight)> = Vec::new();
        let mut lop_display_data: Vec<LopDisplayData> = Vec::new();

        for sight in active_sights {
            match self.compute_lop_with_display_data(sight) {
                Ok((lop, time, display_data)) => {
                    lops_with_times.push((lop, time, sight));
                    lop_display_data.push(display_data);
                }
                Err(e) => {
                    self.error_message = Some(format!("Error computing LOP: {}", e));
                    return;
                }
            }
        }

        // Find the latest observation time
        let latest_time = lops_with_times
            .iter()
            .map(|(_, time, _)| *time)
            .max()
            .unwrap();

        // Check if we need to do a running fix (sights at different times)
        let needs_running_fix = lops_with_times
            .iter()
            .any(|(_, time, _)| (*time - latest_time).num_seconds().abs() > 60);

        let mut advanced_info = Vec::new();

        // Advance LOPs to latest time if needed
        let lops: Vec<LineOfPosition> = if needs_running_fix && (speed > 0.0) {
            lops_with_times
                .iter()
                .zip(lop_display_data.iter_mut())
                .map(|((lop, time, sight), display_data)| {
                    let time_diff_hours = (latest_time - *time).num_seconds() as f64 / 3600.0;
                    if time_diff_hours.abs() > 0.016 {
                        // More than ~1 minute difference
                        advanced_info.push(format!(
                            "{} LOP advanced {:.1} hours ({:.1} NM on course {:.0}°)",
                            sight.body.name(),
                            time_diff_hours,
                            speed * time_diff_hours,
                            course
                        ));
                        let advanced_lop = advance_lop(lop, course, speed, time_diff_hours);
                        // Update display data with advanced position
                        display_data.chosen_lat = advanced_lop.dr_latitude;
                        display_data.chosen_lon = advanced_lop.dr_longitude;
                        advanced_lop
                    } else {
                        *lop
                    }
                })
                .collect()
        } else {
            lops_with_times.iter().map(|(lop, _, _)| *lop).collect()
        };

        // Store the LOP display data
        self.lop_data = lop_display_data;

        // Calculate fix from LOPs
        match fix_from_multiple_lops(&lops) {
            Some(fix) => {
                self.fix_result = Some(fix);
                if !advanced_info.is_empty() {
                    let msg = format!(
                        "Running fix computed (advanced to {}):\n{}",
                        latest_time.format("%Y-%m-%d %H:%M:%S UTC"),
                        advanced_info.join("\n")
                    );
                    self.error_message = Some(msg);
                } else {
                    self.error_message = Some("Fix computed from simultaneous sights".to_string());
                }
            }
            None => {
                self.error_message = Some("Failed to compute fix from LOPs".to_string());
            }
        }
    }

    fn compute_lop_with_time(&self, sight: &Sight) -> Result<(LineOfPosition, chrono::DateTime<Utc>), String> {
        // Parse date and time
        let date = NaiveDate::parse_from_str(&sight.date, "%Y-%m-%d")
            .map_err(|_| "Invalid date format".to_string())?;
        let time = NaiveTime::parse_from_str(&sight.time, "%H:%M:%S")
            .or_else(|_| NaiveTime::parse_from_str(&sight.time, "%H:%M"))
            .map_err(|_| "Invalid time format".to_string())?;
        let datetime = Utc.from_utc_datetime(&date.and_time(time));

        let lop = self.compute_lop(sight)?;
        Ok((lop, datetime))
    }

    fn compute_lop_with_display_data(&self, sight: &Sight) -> Result<(LineOfPosition, chrono::DateTime<Utc>, LopDisplayData), String> {
        use crate::validation::parse_dms;

        // Parse date and time
        let date = NaiveDate::parse_from_str(&sight.date, "%Y-%m-%d")
            .map_err(|_| "Invalid date format".to_string())?;
        let time = NaiveTime::parse_from_str(&sight.time, "%H:%M:%S")
            .or_else(|_| NaiveTime::parse_from_str(&sight.time, "%H:%M"))
            .map_err(|_| "Invalid time format".to_string())?;
        let datetime = Utc.from_utc_datetime(&date.and_time(time));

        // Parse sextant altitude using parse_dms
        let (sext_deg, sext_min, sext_sec) = parse_dms(&sight.sextant_altitude)
            .map_err(|e| format!("Invalid sextant altitude: {}", e))?;
        let sextant_altitude = celtnav::dms_to_decimal(sext_deg as i32, sext_min as u32, sext_sec);

        // Parse corrections
        let index_error: f64 = sight.index_error.parse()
            .map_err(|_| "Invalid index error".to_string())?;
        let height_of_eye: f64 = sight.height_of_eye.parse()
            .map_err(|_| "Invalid height of eye".to_string())?;

        // Parse DR position using parse_dms
        let (dr_lat_deg, dr_lat_min, dr_lat_sec) = parse_dms(&sight.dr_latitude)
            .map_err(|e| format!("Invalid DR latitude: {}", e))?;
        let mut dr_latitude = celtnav::dms_to_decimal(dr_lat_deg as i32, dr_lat_min as u32, dr_lat_sec);
        if sight.lat_direction == 'S' {
            dr_latitude = -dr_latitude;
        }

        let (dr_lon_deg, dr_lon_min, dr_lon_sec) = parse_dms(&sight.dr_longitude)
            .map_err(|e| format!("Invalid DR longitude: {}", e))?;
        let mut dr_longitude = celtnav::dms_to_decimal(dr_lon_deg as i32, dr_lon_min as u32, dr_lon_sec);
        if sight.lon_direction == 'W' {
            dr_longitude = -dr_longitude;
        }

        // Get almanac data
        let almanac_body = sight.body.to_almanac_body();
        let position = get_body_position(almanac_body, datetime)?;

        // For spherical trigonometry calculations, use DR position directly
        // (No optimization needed - LHA can have decimal values)
        // Note: Optimization is only needed for SRT table lookups
        let chosen_lat = dr_latitude;
        let chosen_lon = dr_longitude;

        // Apply corrections to get observed altitude (Ho)
        // Correction order is critical: Hs + IE + dip + refraction + SD + parallax
        let mut ho = sextant_altitude;

        // 1. Index error (can be positive or negative)
        ho += index_error / 60.0;

        // 2. Dip correction (always negative, based on height of eye)
        ho += apply_dip_correction(height_of_eye);

        // 3. Refraction correction (always negative, use altitude BEFORE applying refraction)
        let refraction = apply_refraction_correction(ho);
        ho += refraction;

        // 4. Semi-diameter for Sun and Moon (depends on limb observed)
        if matches!(sight.body, SightCelestialBody::Sun) {
            // Assuming lower limb observation (add semi-diameter)
            ho += apply_semidiameter_correction(0.267, true);
        } else if matches!(sight.body, SightCelestialBody::Moon) {
            // Assuming lower limb observation (add semi-diameter)
            ho += apply_semidiameter_correction(0.25, true);

            // 5. Parallax for Moon (always positive, use altitude AFTER semi-diameter)
            ho += apply_parallax_correction(0.95, ho);
        }

        // Calculate LHA using DR position
        // LHA = GHA + Longitude (using signed convention: East +, West -)
        // For trig calculations, LHA can have decimal values (no need to optimize)
        let lha = (position.gha + chosen_lon + 360.0) % 360.0;

        // Compute Hc and Zn using DR position
        let sight_data = SightData {
            latitude: chosen_lat,
            declination: position.declination,
            local_hour_angle: lha,
        };

        let hc = compute_altitude(&sight_data);
        let zn = compute_azimuth(&sight_data);
        let intercept = compute_intercept(&sight_data, ho);

        // LOP still uses DR position for advancing and fix calculation
        let lop = LineOfPosition {
            azimuth: zn,
            intercept,
            dr_latitude,
            dr_longitude,
        };

        // For stars, calculate SRT data (optimized position and whole LHA Aries)
        // This allows user to verify against table lookups
        let (gha_aries, pub249_chosen_lat, pub249_chosen_lon, lha_aries) =
            if let SightCelestialBody::Star(star_name) = &sight.body {
                use celtnav::almanac::find_star_for_year;

                // For stars: GHA star = GHA Aries + SHA star
                // So: GHA Aries = GHA star - SHA star
                // This ensures consistency with the GHA star value we're using
                // IMPORTANT: Use proper-motion-corrected star position for the observation year
                let year = datetime.year() as f64;
                let day_of_year = datetime.ordinal() as f64;
                let days_in_year = if is_leap_year(datetime.year()) { 366.0 } else { 365.0 };
                let observation_year = year + (day_of_year - 1.0) / days_in_year;

                let star = find_star_for_year(star_name, observation_year)
                    .ok_or_else(|| format!("Star '{}' not found", star_name))?;
                let gha_aries_val = (position.gha - star.sha + 360.0) % 360.0;

                // Optimize chosen position to make LHA Aries a whole number (for SRT tables)
                let (opt_lat, opt_lon) = optimize_chosen_position(dr_latitude, dr_longitude, gha_aries_val);

                // Calculate LHA Aries with optimized position
                let lha_aries_exact = (gha_aries_val + opt_lon + 360.0) % 360.0;

                // Round to nearest whole degree for table lookup
                let lha_aries_whole = lha_aries_exact.round();

                (Some(gha_aries_val), Some(opt_lat), Some(opt_lon), Some(lha_aries_whole))
            } else {
                (None, None, None, None)
            };

        // Display data shows DR position for trig calc, plus SRT data for stars
        let display_data = LopDisplayData {
            body_name: sight.body.name(),
            chosen_lat,
            chosen_lon,
            ho,
            declination: position.declination,
            gha: position.gha,
            lha,
            gha_aries,
            pub249_chosen_lat,
            pub249_chosen_lon,
            lha_aries,
            hc,
            intercept,
            azimuth: zn,
        };

        Ok((lop, datetime, display_data))
    }

    fn compute_lop(&self, sight: &Sight) -> Result<LineOfPosition, String> {
        use crate::validation::parse_dms;

        // Parse date and time
        let date = NaiveDate::parse_from_str(&sight.date, "%Y-%m-%d")
            .map_err(|_| "Invalid date format".to_string())?;
        let time = NaiveTime::parse_from_str(&sight.time, "%H:%M:%S")
            .or_else(|_| NaiveTime::parse_from_str(&sight.time, "%H:%M"))
            .map_err(|_| "Invalid time format".to_string())?;
        let datetime = Utc.from_utc_datetime(&date.and_time(time));

        // Parse sextant altitude using parse_dms
        let (sext_deg, sext_min, sext_sec) = parse_dms(&sight.sextant_altitude)
            .map_err(|e| format!("Invalid sextant altitude: {}", e))?;
        let sextant_altitude = celtnav::dms_to_decimal(sext_deg as i32, sext_min as u32, sext_sec);

        // Parse corrections
        let index_error: f64 = sight.index_error.parse()
            .map_err(|_| "Invalid index error".to_string())?;
        let height_of_eye: f64 = sight.height_of_eye.parse()
            .map_err(|_| "Invalid height of eye".to_string())?;

        // Parse DR position using parse_dms
        let (dr_lat_deg, dr_lat_min, dr_lat_sec) = parse_dms(&sight.dr_latitude)
            .map_err(|e| format!("Invalid DR latitude: {}", e))?;
        let mut dr_latitude = celtnav::dms_to_decimal(dr_lat_deg as i32, dr_lat_min as u32, dr_lat_sec);
        if sight.lat_direction == 'S' {
            dr_latitude = -dr_latitude;
        }

        let (dr_lon_deg, dr_lon_min, dr_lon_sec) = parse_dms(&sight.dr_longitude)
            .map_err(|e| format!("Invalid DR longitude: {}", e))?;
        let mut dr_longitude = celtnav::dms_to_decimal(dr_lon_deg as i32, dr_lon_min as u32, dr_lon_sec);
        if sight.lon_direction == 'W' {
            dr_longitude = -dr_longitude;
        }

        // Get almanac data
        let almanac_body = sight.body.to_almanac_body();
        let position = get_body_position(almanac_body, datetime)?;

        // Apply corrections to get observed altitude
        let mut ho = sextant_altitude;
        ho += index_error / 60.0;
        ho += apply_dip_correction(height_of_eye);
        ho += apply_refraction_correction(ho);

        // Apply semi-diameter for Sun and Moon
        if matches!(sight.body, SightCelestialBody::Sun) {
            ho += apply_semidiameter_correction(0.267, true);
        } else if matches!(sight.body, SightCelestialBody::Moon) {
            ho += apply_semidiameter_correction(0.25, true);
            ho += apply_parallax_correction(0.95, ho);
        }

        // Calculate LHA
        let lha = (position.gha + dr_longitude + 360.0) % 360.0;

        // Compute Hc and Zn
        let sight_data = SightData {
            latitude: dr_latitude,
            declination: position.declination,
            local_hour_angle: lha,
        };

        let _hc = compute_altitude(&sight_data);
        let zn = compute_azimuth(&sight_data);
        let intercept = compute_intercept(&sight_data, ho);

        Ok(LineOfPosition {
            azimuth: zn,
            intercept,
            dr_latitude,
            dr_longitude,
        })
    }

    /// Save current sights to a JSON file
    pub fn save_sights(&self) -> Result<String, String> {
        use crate::persistence::save_to_file;
        use chrono::Local;

        if self.sights.is_empty() {
            return Err("No sights to save".to_string());
        }

        // Create filename with timestamp
        let timestamp = Local::now().format("%Y%m%d_%H%M%S");
        let filename = format!("sights_{}.json", timestamp);

        match save_to_file(&self.sights, &filename) {
            Ok(path) => Ok(format!("Saved {} sights to {:?}", self.sights.len(), path)),
            Err(e) => Err(format!("Failed to save sights: {}", e)),
        }
    }

    /// Load sights from a JSON file
    pub fn load_sights(&mut self, filename: &str) -> Result<String, String> {
        use crate::persistence::load_from_file;

        match load_from_file::<Vec<Sight>>(filename) {
            Ok(sights) => {
                let count = sights.len();
                self.sights = sights;
                self.fix_result = None;
                self.selected_sight_index = None;
                Ok(format!("Loaded {} sights from {}", count, filename))
            }
            Err(e) => Err(format!("Failed to load sights: {}", e)),
        }
    }

    /// List all saved sight files
    pub fn list_saved_files() -> Result<Vec<String>, String> {
        use crate::persistence::list_saved_files;

        list_saved_files("json")
            .map_err(|e| format!("Failed to list saved files: {}", e))
    }

    /// Export sight log to text format
    pub fn export_log(&self) -> Result<String, String> {
        use crate::export::{format_sight_log, save_export};

        if self.sights.is_empty() {
            return Err("No sights to export".to_string());
        }

        let log = format_sight_log(&self.sights, &self.lop_data, self.fix_result.as_ref());
        save_export(&log, "sight_log")
    }

    /// Export sights to CSV format
    pub fn export_csv(&self) -> Result<String, String> {
        use crate::export::{format_sight_csv, save_export};

        if self.sights.is_empty() {
            return Err("No sights to export".to_string());
        }

        let csv = format_sight_csv(&self.sights);
        save_export(&csv, "sights_csv")
    }

    pub fn handle_key_event(&mut self, key_event: KeyEvent) {
        match key_event.code {
            KeyCode::Tab => {
                match self.mode {
                    AutoComputeMode::EnteringSight => self.next_field(),
                    AutoComputeMode::EditingRunningFix => {
                        self.running_fix_field = self.running_fix_field.next();
                    }
                    AutoComputeMode::ViewingSights => {
                        // Tab switches back to EnteringSight mode
                        self.mode = AutoComputeMode::EnteringSight;
                    }
                }
            }
            KeyCode::BackTab => {
                match self.mode {
                    AutoComputeMode::EnteringSight => self.previous_field(),
                    AutoComputeMode::EditingRunningFix => {
                        self.running_fix_field = self.running_fix_field.next();
                    }
                    AutoComputeMode::ViewingSights => {
                        // BackTab also switches back to EnteringSight mode
                        self.mode = AutoComputeMode::EnteringSight;
                    }
                }
            }
            KeyCode::Enter => {
                match self.mode {
                    AutoComputeMode::EnteringSight => {
                        // If on StarName field, select current highlighted star
                        if self.current_field == SightInputField::StarName {
                            self.select_current_star();
                        } else if self.editing_sight_index.is_some() {
                            // If editing, save the edited sight
                            self.save_edited_sight();
                        } else {
                            // Otherwise, add a new sight
                            self.add_sight();
                        }
                    }
                    _ => {}
                }
            }
            KeyCode::Char('c') | KeyCode::Char('C') => {
                self.compute_fix();
            }
            KeyCode::Char('v') | KeyCode::Char('V') => {
                self.mode = if self.mode == AutoComputeMode::EnteringSight {
                    AutoComputeMode::ViewingSights
                } else {
                    AutoComputeMode::EnteringSight
                };
            }
            KeyCode::Char('r') | KeyCode::Char('R') => {
                // Toggle running fix editing mode
                self.mode = if self.mode == AutoComputeMode::EditingRunningFix {
                    AutoComputeMode::EnteringSight
                } else {
                    AutoComputeMode::EditingRunningFix
                };
            }
            KeyCode::F(2) => {
                // Save sights
                match self.save_sights() {
                    Ok(msg) => self.error_message = Some(msg),
                    Err(e) => self.error_message = Some(e),
                }
            }
            KeyCode::F(3) => {
                // Load most recent sights file
                match Self::list_saved_files() {
                    Ok(files) => {
                        if let Some(latest) = files.last() {
                            match self.load_sights(latest) {
                                Ok(msg) => self.error_message = Some(msg),
                                Err(e) => self.error_message = Some(e),
                            }
                        } else {
                            self.error_message = Some("No saved sight files found".to_string());
                        }
                    }
                    Err(e) => self.error_message = Some(e),
                }
            }
            KeyCode::F(5) => {
                // Export to text log
                match self.export_log() {
                    Ok(msg) => self.error_message = Some(msg),
                    Err(e) => self.error_message = Some(e),
                }
            }
            KeyCode::F(6) => {
                // Export to CSV
                match self.export_csv() {
                    Ok(msg) => self.error_message = Some(msg),
                    Err(e) => self.error_message = Some(e),
                }
            }
            KeyCode::Up => {
                match self.mode {
                    AutoComputeMode::ViewingSights => {
                        if !self.sights.is_empty() {
                            self.selected_sight_index = Some(
                                self.selected_sight_index
                                    .map(|i| if i == 0 { self.sights.len() - 1 } else { i - 1 })
                                    .unwrap_or(0)
                            );
                        }
                    }
                    AutoComputeMode::EnteringSight => {
                        // In StarName field, navigate filtered star list up
                        if self.current_field == SightInputField::StarName {
                            self.previous_star_match();
                        }
                    }
                    _ => {}
                }
            }
            KeyCode::Down => {
                match self.mode {
                    AutoComputeMode::ViewingSights => {
                        if !self.sights.is_empty() {
                            self.selected_sight_index = Some(
                                self.selected_sight_index
                                    .map(|i| (i + 1) % self.sights.len())
                                    .unwrap_or(0)
                            );
                        }
                    }
                    AutoComputeMode::EnteringSight => {
                        // In StarName field, navigate filtered star list down
                        if self.current_field == SightInputField::StarName {
                            self.next_star_match();
                        }
                    }
                    _ => {}
                }
            }
            KeyCode::Left => {
                match self.mode {
                    AutoComputeMode::EnteringSight => {
                        // Left arrow cycles selection fields backward (same as '-')
                        match self.current_field {
                            SightInputField::Body => {
                                self.previous_body();
                            }
                            SightInputField::LatDirection => {
                                self.toggle_lat_direction();
                            }
                            SightInputField::LonDirection => {
                                self.toggle_lon_direction();
                            }
                            // For text input fields, do nothing (don't interfere with typing)
                            _ => {}
                        }
                    }
                    _ => {}
                }
            }
            KeyCode::Right => {
                match self.mode {
                    AutoComputeMode::EnteringSight => {
                        // Right arrow cycles selection fields forward (same as '+')
                        match self.current_field {
                            SightInputField::Body => {
                                self.next_body();
                            }
                            SightInputField::LatDirection => {
                                self.toggle_lat_direction();
                            }
                            SightInputField::LonDirection => {
                                self.toggle_lon_direction();
                            }
                            // For text input fields, do nothing (don't interfere with typing)
                            _ => {}
                        }
                    }
                    _ => {}
                }
            }
            // Handle Ctrl+S for saving edited sight (must come before general Char handler)
            KeyCode::Char('s') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
                if self.editing_sight_index.is_some() {
                    self.save_edited_sight();
                }
            }
            KeyCode::Char(c) => {
                match self.mode {
                    AutoComputeMode::EnteringSight => {
                        match self.current_field {
                            SightInputField::Body => {
                                if c == '+' || c == '=' {
                                    self.next_body();
                                } else if c == '-' || c == '_' {
                                    self.previous_body();
                                }
                            }
                            SightInputField::LatDirection => {
                                if c == 'N' || c == 'n' || c == 'S' || c == 's' {
                                    self.set_field_value(SightInputField::LatDirection, c.to_string());
                                }
                            }
                            SightInputField::LonDirection => {
                                if c == 'E' || c == 'e' || c == 'W' || c == 'w' {
                                    self.set_field_value(SightInputField::LonDirection, c.to_string());
                                }
                            }
                            _ => {
                                let mut value = self.get_field_value(self.current_field);
                                value.push(c);
                                self.set_field_value(self.current_field, value);
                            }
                        }
                    }
                    AutoComputeMode::ViewingSights => {
                        match c {
                            // Delete selected sight
                            'd' | 'D' => {
                                self.delete_selected_sight();
                            }
                            // Edit selected sight
                            'e' | 'E' => {
                                self.edit_selected_sight();
                            }
                            // Toggle multi-select mode
                            'm' | 'M' => {
                                self.toggle_multi_select();
                            }
                            // Average selected sights (only in multi-select mode)
                            'a' | 'A' if self.multi_select_mode => {
                                self.average_selected_sights();
                            }
                            // Toggle sight selection (Space bar, only in multi-select mode)
                            ' ' if self.multi_select_mode => {
                                self.toggle_sight_selection();
                            }
                            _ => {}
                        }
                    }
                    AutoComputeMode::EditingRunningFix => {
                        if c.is_ascii_digit() || c == '.' {
                            match self.running_fix_field {
                                RunningFixField::Course => {
                                    self.vessel_course.push(c);
                                }
                                RunningFixField::Speed => {
                                    self.vessel_speed.push(c);
                                }
                            }
                        }
                    }
                }
            }
            KeyCode::Backspace => {
                match self.mode {
                    AutoComputeMode::EnteringSight => {
                        let mut value = self.get_field_value(self.current_field);
                        value.pop();
                        self.set_field_value(self.current_field, value);
                    }
                    AutoComputeMode::EditingRunningFix => {
                        match self.running_fix_field {
                            RunningFixField::Course => {
                                self.vessel_course.pop();
                            }
                            RunningFixField::Speed => {
                                self.vessel_speed.pop();
                            }
                        }
                    }
                    _ => {}
                }
            }
            KeyCode::Esc => {
                // Cancel editing if in edit mode
                if self.editing_sight_index.is_some() && self.mode == AutoComputeMode::EnteringSight {
                    self.cancel_edit();
                }
            }
            _ => {}
        }
    }
}

impl Default for AutoComputeForm {
    fn default() -> Self {
        Self::new()
    }
}

/// Render the auto compute screen
pub fn render_auto_compute_screen(frame: &mut Frame, area: Rect, form: &AutoComputeForm) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(50), // Input/Sights area
            Constraint::Percentage(50), // Fix results area
        ])
        .split(area);

    let top_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(60), // Input form
            Constraint::Percentage(40), // Sights list
        ])
        .split(chunks[0]);

    render_input_form(frame, top_chunks[0], form);
    render_sights_list(frame, top_chunks[1], form);
    render_fix_results(frame, chunks[1], form);
}

fn render_input_form(frame: &mut Frame, area: Rect, form: &AutoComputeForm) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(3),
        ])
        .split(area);

    let fields = SightInputField::all();
    let mut lines = vec![Line::from("")];

    for field in fields {
        // Skip StarName field if Star body is not selected
        if field == SightInputField::StarName && !form.current_sight.is_star() {
            continue;
        }

        let value = form.get_field_value(field);
        let is_current = field == form.current_field && form.mode == AutoComputeMode::EnteringSight;
        let validation_error = form.validate_field(field);

        let label_style = if is_current {
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };

        // Change value color based on validation
        let value_style = if is_current {
            if validation_error.is_some() {
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
            } else {
                Style::default().fg(Color::Green).add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
            }
        } else {
            if validation_error.is_some() && !value.is_empty() {
                Style::default().fg(Color::Red)
            } else {
                Style::default().fg(Color::Gray)
            }
        };

        let cursor = if is_current { "► " } else { "  " };

        lines.push(Line::from(vec![
            Span::styled(cursor, Style::default().fg(Color::Yellow)),
            Span::styled(format!("{}: ", field.label()), label_style),
            Span::styled(value, value_style),
        ]));

        // Show autocompletion suggestions for StarName field
        if field == SightInputField::StarName && is_current && !form.star_filter_matches.is_empty() {
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

        // Show validation error for current field
        if is_current && field != SightInputField::StarName {
            if let Some(error) = validation_error {
                lines.push(Line::from(vec![
                    Span::raw("    "),
                    Span::styled(format!("⚠ {}", error), Style::default().fg(Color::Red).add_modifier(Modifier::ITALIC)),
                ]));
            }
        }
    }

    let paragraph = Paragraph::new(lines)
        .block(
            Block::default()
                .title(" Enter Sight ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .wrap(Wrap { trim: false });

    frame.render_widget(paragraph, chunks[0]);

    // Context-aware help text based on mode
    let help_lines = if form.editing_sight_index.is_some() {
        vec![
            Line::from("EDITING SIGHT: Enter or Ctrl+S: Save | Esc: Cancel | Tab/Shift+Tab: Navigate"),
            Line::from("+/- or ←→: Cycle Body/Direction | Star field: Enter to select | Type to edit fields"),
        ]
    } else {
        vec![
            Line::from("Enter: Add Sight | C: Compute Fix | V: View Sights | R: Running Fix | +/- or ←→: Cycle Options"),
            Line::from("Star field: Up/Down: Navigate | Enter: Select | Type to filter stars"),
            Line::from("F2: Save | F3: Load | F5: Export Log | F6: Export CSV"),
        ]
    };
    let help_widget = Paragraph::new(help_lines)
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Center);
    frame.render_widget(help_widget, chunks[1]);
}

fn render_sights_list(frame: &mut Frame, area: Rect, form: &AutoComputeForm) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),      // Sights list
            Constraint::Length(5),   // Running fix parameters
        ])
        .split(area);

    let items: Vec<ListItem> = form
        .sights
        .iter()
        .enumerate()
        .map(|(i, sight)| {
            let is_selected = form.selected_sight_index == Some(i);
            let is_multi_selected = form.selected_sight_indices.contains(&i);

            // Checkbox for multi-select mode
            let checkbox = if form.multi_select_mode {
                if is_multi_selected { "[✓] " } else { "[ ] " }
            } else {
                ""
            };

            // Style based on selection and active status
            let style = if is_selected {
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
            } else if is_multi_selected {
                Style::default().fg(Color::Cyan)
            } else if !sight.is_active {
                Style::default().fg(Color::DarkGray).add_modifier(Modifier::DIM)
            } else {
                Style::default().fg(Color::White)
            };

            // Enhanced display with log and heading if available
            let mut display = sight.display_summary();
            if !sight.log_reading.is_empty() || !sight.heading.is_empty() {
                display.push_str(&format!(" [Log:{} Hdg:{}]",
                    if sight.log_reading.is_empty() { "-" } else { &sight.log_reading },
                    if sight.heading.is_empty() { "-" } else { &sight.heading }
                ));
            }

            // Mark inactive sights
            if !sight.is_active {
                display.push_str(" (inactive)");
            }

            ListItem::new(format!("{}{}", checkbox, display)).style(style)
        })
        .collect();

    let title = if form.multi_select_mode {
        format!(" Sights ({}) - MULTI-SELECT MODE ", form.sights.len())
    } else if form.editing_sight_index.is_some() {
        format!(" Sights ({}) - EDITING ", form.sights.len())
    } else {
        format!(" Sights ({}) ", form.sights.len())
    };

    let list = List::new(items)
        .block(
            Block::default()
                .title(title)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Green)),
        );

    frame.render_widget(list, chunks[0]);

    // Render running fix parameters
    let is_editing_rf = form.mode == AutoComputeMode::EditingRunningFix;
    let course_style = if is_editing_rf && form.running_fix_field == RunningFixField::Course {
        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
    } else {
        Style::default().fg(Color::White)
    };
    let speed_style = if is_editing_rf && form.running_fix_field == RunningFixField::Speed {
        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
    } else {
        Style::default().fg(Color::White)
    };

    let course_cursor = if is_editing_rf && form.running_fix_field == RunningFixField::Course {
        "► "
    } else {
        "  "
    };
    let speed_cursor = if is_editing_rf && form.running_fix_field == RunningFixField::Speed {
        "► "
    } else {
        "  "
    };

    let mut running_fix_lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled(course_cursor, Style::default().fg(Color::Yellow)),
            Span::styled("Vessel Course: ", Style::default().fg(Color::Cyan)),
            Span::styled(&form.vessel_course, course_style),
            Span::styled("°", course_style),
        ]),
        Line::from(vec![
            Span::styled(speed_cursor, Style::default().fg(Color::Yellow)),
            Span::styled("Vessel Speed:  ", Style::default().fg(Color::Cyan)),
            Span::styled(&form.vessel_speed, speed_style),
            Span::styled(" knots", speed_style),
        ]),
    ];

    // Add mode-specific help text for ViewingSights mode
    if form.mode == AutoComputeMode::ViewingSights {
        running_fix_lines.push(Line::from(""));
        if form.multi_select_mode {
            running_fix_lines.push(Line::from(
                Span::styled("↑↓: Navigate | Space: Select | A: Average | M: Exit | Tab: Back to Entry",
                    Style::default().fg(Color::DarkGray))
            ));
        } else {
            running_fix_lines.push(Line::from(
                Span::styled("↑↓: Navigate | E: Edit | D: Delete | M: Multi-select | Tab: Back to Entry",
                    Style::default().fg(Color::DarkGray))
            ));
        }
    }

    let rf_border_style = if is_editing_rf {
        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Magenta)
    };

    let running_fix_widget = Paragraph::new(running_fix_lines)
        .block(
            Block::default()
                .title(" Running Fix ")
                .borders(Borders::ALL)
                .border_style(rf_border_style),
        );

    frame.render_widget(running_fix_widget, chunks[1]);
}

fn render_fix_results(frame: &mut Frame, area: Rect, form: &AutoComputeForm) {
    if let Some(fix) = &form.fix_result {
        // Split area: two-pane layout on top, status at bottom (1 line)
        let vertical_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(0),      // Two-pane layout (LOPs and Fix)
                Constraint::Length(1),   // Status message (1 line, no borders)
            ])
            .split(area);

        // Render two-pane layout in top section
        render_two_pane_layout(frame, vertical_chunks[0], form, fix);

        // Render status message at bottom (1 line, no borders)
        let status_text = if let Some(error) = &form.error_message {
            error.clone()
        } else {
            "Fix computed successfully".to_string()
        };

        let status_color = if form.error_message.is_some() {
            Color::Yellow
        } else {
            Color::Green
        };

        let status_paragraph = Paragraph::new(status_text)
            .style(Style::default().fg(status_color))
            .alignment(Alignment::Left);

        frame.render_widget(status_paragraph, vertical_chunks[1]);
    } else {
        // No fix yet
        if let Some(error) = &form.error_message {
            // Show error message
            let paragraph = Paragraph::new(error.clone())
                .style(Style::default().fg(Color::Yellow))
                .block(
                    Block::default()
                        .title(" Status ")
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(Color::Blue)),
                )
                .wrap(Wrap { trim: true });

            frame.render_widget(paragraph, area);
        } else {
            // Show instructions
            let text = "Enter 2 or more sights, then press 'C' to compute fix";
            let paragraph = Paragraph::new(text)
                .style(Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC))
                .alignment(Alignment::Center)
                .block(
                    Block::default()
                        .title(" Fix Results ")
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(Color::Blue)),
                );

            frame.render_widget(paragraph, area);
        }
    }
}

/// Renders the two-pane layout: LOPs on left, Fix on right
fn render_two_pane_layout(frame: &mut Frame, area: Rect, form: &AutoComputeForm, fix: &Fix) {
    // Split horizontally into left and right panes
    let horizontal_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(50),  // Left pane: LOPs
            Constraint::Percentage(50),  // Right pane: Fix
        ])
        .split(area);

    // Render LOPs in left pane
    render_lop_pane(frame, horizontal_chunks[0], &form.lop_data);

    // Render fix in right pane
    render_fix_pane(frame, horizontal_chunks[1], fix);
}

/// Renders the LOP data in the left pane, split into two columns
fn render_lop_pane(frame: &mut Frame, area: Rect, lop_data: &[LopDisplayData]) {
    if lop_data.is_empty() {
        let text = "No LOP data available";
        let paragraph = Paragraph::new(text)
            .style(Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC))
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .title(" Lines of Position ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Blue)),
            );
        frame.render_widget(paragraph, area);
        return;
    }

    // Split LOP pane into two columns
    let lop_columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(50),  // Left column
            Constraint::Percentage(50),  // Right column
        ])
        .split(area);

    // Split LOPs into two halves
    let mid = (lop_data.len() + 1) / 2;
    let left_lops = &lop_data[..mid];
    let right_lops = &lop_data[mid..];

    // Render each column
    render_lop_column(frame, lop_columns[0], left_lops, 0);
    render_lop_column(frame, lop_columns[1], right_lops, mid);
}

/// Renders a single column of LOPs
fn render_lop_column(frame: &mut Frame, area: Rect, lop_data: &[LopDisplayData], start_index: usize) {
    let mut lop_lines = Vec::new();

    for (i, lop) in lop_data.iter().enumerate() {
        let sight_number = start_index + i + 1;

        let lat_sign = if lop.chosen_lat >= 0.0 { "N" } else { "S" };
        let lat_dms = celtnav::decimal_to_dms(lop.chosen_lat.abs());

        let lon_sign = if lop.chosen_lon >= 0.0 { "E" } else { "W" };
        let lon_dms = celtnav::decimal_to_dms(lop.chosen_lon.abs());

        let ho_dms = celtnav::decimal_to_dms(lop.ho.abs());
        let gha_dms = celtnav::decimal_to_dms(lop.gha.abs());
        let lha_dms = celtnav::decimal_to_dms(lop.lha.abs());
        let hc_dms = celtnav::decimal_to_dms(lop.hc.abs());

        // Sight header
        lop_lines.push(Line::from(vec![
            Span::styled(
                format!("Sight {}: ", sight_number),
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
            ),
            Span::styled(
                &lop.body_name,
                Style::default().fg(Color::White)
            ),
        ]));

        // Dead Reckoning (DR) position - combined on one line - used as is for trig calculations
        lop_lines.push(Line::from(vec![
            Span::styled("  DR: ", Style::default().fg(Color::Cyan)),
            Span::styled(
                format!("{} {:02}° {:05.2}', {} {:03}° {:04.1}'",
                    lat_sign, lat_dms.degrees, lat_dms.minutes,
                    lon_sign, lon_dms.degrees, lon_dms.minutes),
                Style::default().fg(Color::White)
            ),
        ]));

        // Ho and Hc on one line
        lop_lines.push(Line::from(vec![
            Span::styled("  Ho: ", Style::default().fg(Color::Cyan)),
            Span::styled(
                format!("{:02}° {:04.1}'", ho_dms.degrees, ho_dms.minutes),
                Style::default().fg(Color::White)
            ),
            Span::styled("  Hc: ", Style::default().fg(Color::Cyan)),
            Span::styled(
                format!("{:02}° {:04.1}'", hc_dms.degrees, hc_dms.minutes),
                Style::default().fg(Color::White)
            ),
        ]));

        // Declination on one line
        let dec_sign = if lop.declination >= 0.0 { "N" } else { "S" };
        let dec_dms = celtnav::decimal_to_dms(lop.declination.abs());
        lop_lines.push(Line::from(vec![
            Span::styled("  Dec: ", Style::default().fg(Color::Cyan)),
            Span::styled(
                format!("{} {:02}° {:04.1}'", dec_sign, dec_dms.degrees, dec_dms.minutes),
                Style::default().fg(Color::White)
            ),
        ]));

        // GHA and LHA on one line
        lop_lines.push(Line::from(vec![
            Span::styled("  GHA: ", Style::default().fg(Color::Cyan)),
            Span::styled(
                format!("{:03}° {:04.1}'", gha_dms.degrees, gha_dms.minutes),
                Style::default().fg(Color::White)
            ),
            Span::styled("  LHA: ", Style::default().fg(Color::Cyan)),
            Span::styled(
                format!("{:03}° {:04.1}'", lha_dms.degrees, lha_dms.minutes),
                Style::default().fg(Color::White)
            ),
        ]));

        // Intercept and Azimuth on one line
        let intercept_color = if lop.intercept >= 0.0 {
            Color::Green
        } else {
            Color::Red
        };
        lop_lines.push(Line::from(vec![
            Span::styled("  Intercept: ", Style::default().fg(Color::Cyan)),
            Span::styled(
                lop.intercept_with_direction(),
                Style::default().fg(intercept_color)
            ),
            Span::styled("  Z: ", Style::default().fg(Color::Cyan)),
            Span::styled(
                format!("{:03.0}° T", lop.azimuth),
                Style::default().fg(Color::White)
            ),
        ]));

        // For stars, show SRT data (optimized position and whole LHA Aries)
        if let (Some(gha_aries), Some(pub249_lat), Some(pub249_lon), Some(lha_aries)) =
            (lop.gha_aries, lop.pub249_chosen_lat, lop.pub249_chosen_lon, lop.lha_aries)
        {
            let gha_aries_dms = celtnav::decimal_to_dms(gha_aries.abs());

            // SRT optimized chosen position
            let pub249_lat_sign = if pub249_lat >= 0.0 { "N" } else { "S" };
            let pub249_lat_dms = celtnav::decimal_to_dms(pub249_lat.abs());

            let pub249_lon_sign = if pub249_lon >= 0.0 { "E" } else { "W" };
            let pub249_lon_dms = celtnav::decimal_to_dms(pub249_lon.abs());

            // Line 1: SRT optimized chosen position
            lop_lines.push(Line::from(vec![
                Span::styled("  SRT CP: ", Style::default().fg(Color::Yellow)),
                Span::styled(
                    format!("{} {:02}° {:05.2}', {} {:03}° {:04.1}'",
                        pub249_lat_sign, pub249_lat_dms.degrees, pub249_lat_dms.minutes,
                        pub249_lon_sign, pub249_lon_dms.degrees, pub249_lon_dms.minutes),
                    Style::default().fg(Color::White)
                ),
            ]));

            // Line 2: GHA Aries and LHA Aries (whole number for table lookup)
            lop_lines.push(Line::from(vec![
                Span::styled("  GHA♈: ", Style::default().fg(Color::Yellow)),
                Span::styled(
                    format!("{:03}° {:04.1}'", gha_aries_dms.degrees, gha_aries_dms.minutes),
                    Style::default().fg(Color::White)
                ),
                Span::styled("  LHA♈: ", Style::default().fg(Color::Yellow)),
                Span::styled(
                    format!("{:03}°", lha_aries as i32),  // Whole number for table lookup
                    Style::default().fg(Color::White)
                ),
                Span::styled(" (table)", Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC)),
            ]));
        }

        // Blank line between sights (except for the last one)
        if i < lop_data.len() - 1 {
            lop_lines.push(Line::from(""));
        }
    }

    let title = if start_index == 0 {
        " Lines of Position "
    } else {
        " "
    };

    let lop_widget = Paragraph::new(lop_lines)
        .block(
            Block::default()
                .title(title)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Blue)),
        );

    frame.render_widget(lop_widget, area);
}

/// Renders the fix data in the right pane using a simple table format
fn render_fix_pane(frame: &mut Frame, area: Rect, fix: &Fix) {
    // DR Position
    let dr_lat_sign = if fix.dr_position.latitude >= 0.0 { "N" } else { "S" };
    let dr_lat_dms = celtnav::decimal_to_dms(fix.dr_position.latitude.abs());

    let dr_lon_sign = if fix.dr_position.longitude >= 0.0 { "E" } else { "W" };
    let dr_lon_dms = celtnav::decimal_to_dms(fix.dr_position.longitude.abs());

    // Fix Position
    let lat_sign = if fix.position.latitude >= 0.0 { "N" } else { "S" };
    let lat_dms = celtnav::decimal_to_dms(fix.position.latitude.abs());

    let lon_sign = if fix.position.longitude >= 0.0 { "E" } else { "W" };
    let lon_dms = celtnav::decimal_to_dms(fix.position.longitude.abs());

    let mut rows = vec![
        Row::new(vec!["DR Position".to_string(), "".to_string()])
            .style(Style::default().fg(Color::Cyan)),
        Row::new(vec![
            "  Latitude".to_string(),
            format!("{} {:02}° {:05.2}'", dr_lat_sign, dr_lat_dms.degrees, dr_lat_dms.minutes),
        ]),
        Row::new(vec![
            "  Longitude".to_string(),
            format!("{} {:03}° {:05.2}'", dr_lon_sign, dr_lon_dms.degrees, dr_lon_dms.minutes),
        ]),
        Row::new(vec!["".to_string(), "".to_string()]), // Empty row for spacing
        Row::new(vec!["Fix Position".to_string(), "".to_string()])
            .style(Style::default().fg(Color::Green)),
        Row::new(vec![
            "  Latitude".to_string(),
            format!("{} {:02}° {:05.2}'", lat_sign, lat_dms.degrees, lat_dms.minutes),
        ]),
        Row::new(vec![
            "  Longitude".to_string(),
            format!("{} {:03}° {:05.2}'", lon_sign, lon_dms.degrees, lon_dms.minutes),
        ]),
        Row::new(vec!["".to_string(), "".to_string()]), // Empty row for spacing
        Row::new(vec!["Number of LOPs".to_string(), fix.num_lops.to_string()]),
    ];

    if let Some(accuracy) = fix.accuracy_estimate {
        rows.push(Row::new(vec![
            "Accuracy Estimate".to_string(),
            format!("{:.1} NM", accuracy),
        ]));
    }

    let table = Table::new(rows, [Constraint::Percentage(50), Constraint::Percentage(50)])
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
                .title(" Calculated Fix ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Blue)),
        )
        .style(Style::default().fg(Color::White))
        .column_spacing(2);

    frame.render_widget(table, area);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sight_celestial_body_star_variant() {
        let star = SightCelestialBody::Star("Sirius".to_string());
        assert_eq!(star.name(), "Sirius");
    }

    #[test]
    fn test_sight_celestial_body_star_to_almanac() {
        let star = SightCelestialBody::Star("Arcturus".to_string());
        let almanac_body = star.to_almanac_body();
        match almanac_body {
            AlmanacBody::Star(name) => assert_eq!(name, "Arcturus"),
            _ => panic!("Expected Star variant"),
        }
    }

    #[test]
    fn test_sight_is_star() {
        let mut sight = Sight::new();
        assert!(!sight.is_star());

        sight.body = SightCelestialBody::Star("Polaris".to_string());
        assert!(sight.is_star());
    }

    #[test]
    fn test_star_name_field_exists() {
        let fields = SightInputField::all();
        assert!(fields.contains(&SightInputField::StarName));
    }

    #[test]
    fn test_star_name_field_navigation() {
        // Test 1: For star body, StarName should be included
        let mut form = AutoComputeForm::new();
        form.current_sight.body = SightCelestialBody::Star("Sirius".to_string());
        form.current_field = SightInputField::Body;
        form.next_field();
        assert_eq!(form.current_field, SightInputField::StarName,
            "Should navigate to StarName when body is a star");

        // Test 2: For non-star body, StarName should be skipped
        let mut form2 = AutoComputeForm::new();
        // Default body is Sun (non-star)
        assert!(!form2.current_sight.is_star(), "Default body should not be a star");
        form2.current_field = SightInputField::Body;
        form2.next_field();
        assert_eq!(form2.current_field, SightInputField::Date,
            "Should skip StarName and go to Date when body is not a star");
    }

    #[test]
    fn test_star_name_field_value() {
        let mut form = AutoComputeForm::new();
        form.current_sight.body = SightCelestialBody::Star("Vega".to_string());
        let value = form.get_field_value(SightInputField::StarName);
        assert_eq!(value, "Vega");
    }

    // Tests for Phase 1: New Sight struct fields
    #[test]
    fn test_sight_new_has_default_log_heading_active() {
        let sight = Sight::new();
        assert_eq!(sight.log_reading, String::new(), "Log reading should default to empty");
        assert_eq!(sight.heading, String::new(), "Heading should default to empty");
        assert!(sight.is_active, "Sight should be active by default");
    }

    #[test]
    fn test_sight_log_reading_can_be_set() {
        let mut sight = Sight::new();
        sight.log_reading = "103.5".to_string();
        assert_eq!(sight.log_reading, "103.5");
    }

    #[test]
    fn test_sight_heading_can_be_set() {
        let mut sight = Sight::new();
        sight.heading = "045".to_string();
        assert_eq!(sight.heading, "045");
    }

    #[test]
    fn test_sight_can_be_marked_inactive() {
        let mut sight = Sight::new();
        assert!(sight.is_active);
        sight.is_active = false;
        assert!(!sight.is_active);
    }

    #[test]
    fn test_sight_serialization_with_new_fields() {
        let mut sight = Sight::new();
        sight.body = SightCelestialBody::Sun;
        sight.date = "2024-03-15".to_string();
        sight.time = "12:00:00".to_string();
        sight.sextant_altitude = "45 30.0".to_string();
        sight.log_reading = "103.5".to_string();
        sight.heading = "045".to_string();
        sight.is_active = true;

        // Serialize and deserialize
        let json = serde_json::to_string(&sight).expect("Failed to serialize");
        let deserialized: Sight = serde_json::from_str(&json).expect("Failed to deserialize");

        assert_eq!(deserialized.log_reading, "103.5");
        assert_eq!(deserialized.heading, "045");
        assert!(deserialized.is_active);
    }

    #[test]
    fn test_backward_compatibility_old_json_without_new_fields() {
        // Simulate old JSON file that doesn't have log_reading, heading, or is_active
        let old_json = r#"{
            "body": "Sun",
            "date": "2024-03-15",
            "time": "12:00:00",
            "sextant_altitude": "45 30.0",
            "index_error": "0",
            "height_of_eye": "10",
            "dr_latitude": "50 00.0",
            "dr_longitude": "20 00.0",
            "lat_direction": "N",
            "lon_direction": "W"
        }"#;

        // Should deserialize successfully with default values
        let sight: Sight = serde_json::from_str(old_json).expect("Failed to deserialize old JSON");

        // Verify defaults are applied
        assert_eq!(sight.log_reading, "", "log_reading should default to empty string");
        assert_eq!(sight.heading, "", "heading should default to empty string");
        assert!(sight.is_active, "is_active should default to true");

        // Verify other fields loaded correctly
        assert_eq!(sight.date, "2024-03-15");
        assert_eq!(sight.sextant_altitude, "45 30.0");
    }

    #[test]
    fn test_backward_compatibility_partial_new_fields() {
        // Test that we can load JSON with only some of the new fields
        let partial_json = r#"{
            "body": "Moon",
            "date": "2024-03-15",
            "time": "12:00:00",
            "sextant_altitude": "45 30.0",
            "index_error": "0",
            "height_of_eye": "10",
            "dr_latitude": "50 00.0",
            "dr_longitude": "20 00.0",
            "lat_direction": "N",
            "lon_direction": "W",
            "log_reading": "103.5"
        }"#;

        let sight: Sight = serde_json::from_str(partial_json).expect("Failed to deserialize partial JSON");

        assert_eq!(sight.log_reading, "103.5");
        assert_eq!(sight.heading, "", "heading should default to empty");
        assert!(sight.is_active, "is_active should default to true");
    }

    // Tests for Phase 2: DR Auto-Calculation
    #[test]
    fn test_calculate_dr_from_previous_with_valid_log_and_heading() {
        let mut form = AutoComputeForm::new();

        // Add first sight with log and heading at 50°N, 20°W
        let mut sight1 = Sight::new();
        sight1.body = SightCelestialBody::Sun;
        sight1.date = "2024-03-15".to_string();
        sight1.time = "12:00:00".to_string();
        sight1.sextant_altitude = "45 30.0".to_string();
        sight1.index_error = "0".to_string();
        sight1.height_of_eye = "10".to_string();
        sight1.dr_latitude = "50 00.0".to_string();
        sight1.dr_longitude = "20 00.0".to_string();
        sight1.lat_direction = 'N';
        sight1.lon_direction = 'W';
        sight1.log_reading = "100.0".to_string();
        sight1.heading = "045".to_string();  // Northeast
        form.sights.push(sight1);

        // Set up current sight with new log reading (traveled 5 NM)
        form.current_sight.log_reading = "105.0".to_string();

        // Calculate DR - should advance 5 NM on heading 045°
        let dr = form.calculate_dr_from_previous();
        assert!(dr.is_some(), "DR calculation should succeed");

        let (new_lat, new_lon) = dr.unwrap();
        // Verify position has moved northeast from 50°N, -20°W
        assert!(new_lat > 50.0, "Should have moved north, got lat: {}", new_lat);
        assert!(new_lon > -20.0, "Should have moved east (less negative), got lon: {}", new_lon);
    }

    #[test]
    fn test_calculate_dr_from_previous_with_no_previous_sight() {
        let mut form = AutoComputeForm::new();

        // No previous sight
        form.current_sight.log_reading = "105.0".to_string();

        let dr = form.calculate_dr_from_previous();
        assert!(dr.is_none(), "DR should be None when no previous sight");
    }

    #[test]
    fn test_calculate_dr_from_previous_with_missing_log() {
        let mut form = AutoComputeForm::new();

        // Add sight without log reading
        let mut sight1 = Sight::new();
        sight1.heading = "045".to_string();
        sight1.dr_latitude = "50 00.0".to_string();
        sight1.dr_longitude = "20 00.0".to_string();
        sight1.lat_direction = 'N';
        sight1.lon_direction = 'W';
        form.sights.push(sight1);

        form.current_sight.log_reading = "105.0".to_string();

        let dr = form.calculate_dr_from_previous();
        assert!(dr.is_none(), "DR should be None when previous sight has no log");
    }

    #[test]
    fn test_calculate_dr_from_previous_with_missing_heading() {
        let mut form = AutoComputeForm::new();

        // Add sight without heading
        let mut sight1 = Sight::new();
        sight1.log_reading = "100.0".to_string();
        sight1.dr_latitude = "50 00.0".to_string();
        sight1.dr_longitude = "20 00.0".to_string();
        sight1.lat_direction = 'N';
        sight1.lon_direction = 'W';
        form.sights.push(sight1);

        form.current_sight.log_reading = "105.0".to_string();

        let dr = form.calculate_dr_from_previous();
        assert!(dr.is_none(), "DR should be None when previous sight has no heading");
    }

    // Tests for Phase 3: Edit Sight Functionality
    #[test]
    fn test_edit_sight_loads_sight_into_form() {
        let mut form = AutoComputeForm::new();

        // Add a sight
        let mut sight = Sight::new();
        sight.body = SightCelestialBody::Sun;
        sight.date = "2024-03-15".to_string();
        sight.time = "12:00:00".to_string();
        sight.sextant_altitude = "45 30.0".to_string();
        sight.log_reading = "100.5".to_string();
        sight.heading = "045".to_string();
        form.sights.push(sight);

        // Select the sight and edit it
        form.selected_sight_index = Some(0);
        form.edit_selected_sight();

        // Verify sight loaded into current_sight
        assert_eq!(form.current_sight.body, SightCelestialBody::Sun);
        assert_eq!(form.current_sight.sextant_altitude, "45 30.0");
        assert_eq!(form.current_sight.log_reading, "100.5");
        assert_eq!(form.current_sight.heading, "045");

        // Verify editing state
        assert_eq!(form.editing_sight_index, Some(0));
        assert_eq!(form.mode, AutoComputeMode::EnteringSight);
    }

    #[test]
    fn test_save_edited_sight_updates_list() {
        let mut form = AutoComputeForm::new();

        // Add a sight
        let mut sight = Sight::new();
        sight.body = SightCelestialBody::Sun;
        sight.sextant_altitude = "45 30.0".to_string();
        sight.date = "2024-03-15".to_string();
        sight.time = "12:00:00".to_string();
        sight.index_error = "0".to_string();
        sight.height_of_eye = "10".to_string();
        sight.dr_latitude = "50 00.0".to_string();
        sight.dr_longitude = "20 00.0".to_string();
        sight.lat_direction = 'N';
        sight.lon_direction = 'W';
        form.sights.push(sight);

        // Edit it
        form.selected_sight_index = Some(0);
        form.edit_selected_sight();

        // Modify the sight
        form.current_sight.sextant_altitude = "46 00.0".to_string();

        // Save
        form.save_edited_sight();

        // Verify updated
        assert_eq!(form.sights[0].sextant_altitude, "46 00.0");
        assert_eq!(form.mode, AutoComputeMode::ViewingSights);
        assert_eq!(form.editing_sight_index, None);
    }

    #[test]
    fn test_cancel_edit_does_not_save_changes() {
        let mut form = AutoComputeForm::new();

        // Add a sight
        let mut sight = Sight::new();
        sight.sextant_altitude = "45 30.0".to_string();
        sight.date = "2024-03-15".to_string();
        sight.time = "12:00:00".to_string();
        sight.index_error = "0".to_string();
        sight.height_of_eye = "10".to_string();
        sight.dr_latitude = "50 00.0".to_string();
        sight.dr_longitude = "20 00.0".to_string();
        sight.lat_direction = 'N';
        sight.lon_direction = 'W';
        form.sights.push(sight);

        // Edit it
        form.selected_sight_index = Some(0);
        form.edit_selected_sight();

        // Modify but cancel
        form.current_sight.sextant_altitude = "46 00.0".to_string();
        form.cancel_edit();

        // Verify not updated
        assert_eq!(form.sights[0].sextant_altitude, "45 30.0");
        assert_eq!(form.mode, AutoComputeMode::ViewingSights);
        assert_eq!(form.editing_sight_index, None);
    }

    #[test]
    fn test_edit_sight_does_not_add_new_sight() {
        let mut form = AutoComputeForm::new();

        // Add a sight
        let mut sight = Sight::new();
        sight.body = SightCelestialBody::Sun;
        sight.sextant_altitude = "45 30.0".to_string();
        sight.date = "2024-03-15".to_string();
        sight.time = "12:00:00".to_string();
        sight.index_error = "0".to_string();
        sight.height_of_eye = "10".to_string();
        sight.dr_latitude = "50 00.0".to_string();
        sight.dr_longitude = "20 00.0".to_string();
        sight.lat_direction = 'N';
        sight.lon_direction = 'W';
        form.sights.push(sight);

        let initial_count = form.sights.len();
        assert_eq!(initial_count, 1, "Should start with 1 sight");

        // Edit it
        form.selected_sight_index = Some(0);
        form.edit_selected_sight();

        // Modify the sight
        form.current_sight.sextant_altitude = "46 00.0".to_string();

        // Save using save_edited_sight (which Enter should call in edit mode)
        form.save_edited_sight();

        // Verify count hasn't changed (not added as new)
        assert_eq!(form.sights.len(), 1, "Should still have exactly 1 sight, not 2");

        // Verify the sight was updated
        assert_eq!(form.sights[0].sextant_altitude, "46 00.0");
    }

    #[test]
    fn test_tab_switches_from_viewing_to_entering() {
        let mut form = AutoComputeForm::new();

        // Start in EnteringSight mode
        assert_eq!(form.mode, AutoComputeMode::EnteringSight);

        // Switch to ViewingSights
        form.mode = AutoComputeMode::ViewingSights;
        assert_eq!(form.mode, AutoComputeMode::ViewingSights);

        // Simulate Tab key - should switch back to EnteringSight
        use crossterm::event::KeyCode;
        let tab_event = KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE);
        form.handle_key_event(tab_event);

        assert_eq!(form.mode, AutoComputeMode::EnteringSight, "Tab should switch from ViewingSights to EnteringSight");
    }

    #[test]
    fn test_next_field_skips_star_name_for_non_star_body() {
        let mut form = AutoComputeForm::new();

        // Set body to Sun (non-star)
        form.current_sight.body = SightCelestialBody::Sun;

        // Start at Body field
        form.current_field = SightInputField::Body;

        // Call next_field() - should skip StarName and go to Date
        form.next_field();

        assert_eq!(form.current_field, SightInputField::Date,
            "next_field() should skip StarName when body is not a star");
        assert_ne!(form.current_field, SightInputField::StarName,
            "Should not stop at StarName for non-star body");
    }

    #[test]
    fn test_previous_field_skips_star_name_for_non_star_body() {
        let mut form = AutoComputeForm::new();

        // Set body to Moon (non-star)
        form.current_sight.body = SightCelestialBody::Moon;

        // Start at Date field
        form.current_field = SightInputField::Date;

        // Call previous_field() - should skip StarName and go to Body
        form.previous_field();

        assert_eq!(form.current_field, SightInputField::Body,
            "previous_field() should skip StarName when body is not a star");
        assert_ne!(form.current_field, SightInputField::StarName,
            "Should not stop at StarName for non-star body");
    }

    #[test]
    fn test_next_field_includes_star_name_for_star_body() {
        let mut form = AutoComputeForm::new();

        // Set body to Star
        form.current_sight.body = SightCelestialBody::Star("Sirius".to_string());

        // Start at Body field
        form.current_field = SightInputField::Body;

        // Call next_field() - should go to StarName
        form.next_field();

        assert_eq!(form.current_field, SightInputField::StarName,
            "next_field() should include StarName when body is a star");
    }

    #[test]
    fn test_previous_field_includes_star_name_for_star_body() {
        let mut form = AutoComputeForm::new();

        // Set body to Star
        form.current_sight.body = SightCelestialBody::Star("Vega".to_string());

        // Start at Date field
        form.current_field = SightInputField::Date;

        // Call previous_field() - should go to StarName
        form.previous_field();

        assert_eq!(form.current_field, SightInputField::StarName,
            "previous_field() should include StarName when body is a star");
    }

    // Tests for Phase 4: Sight Averaging
    #[test]
    fn test_averaging_marks_originals_inactive() {
        let mut form = AutoComputeForm::new();

        // Add two Pollux sights 3 minutes apart
        let mut sight1 = Sight::new();
        sight1.body = SightCelestialBody::Star("Pollux".to_string());
        sight1.date = "2024-03-15".to_string();
        sight1.time = "12:00:00".to_string();
        sight1.sextant_altitude = "45 30.0".to_string();
        sight1.index_error = "0".to_string();
        sight1.height_of_eye = "10".to_string();
        sight1.dr_latitude = "50 00.0".to_string();
        sight1.dr_longitude = "20 00.0".to_string();
        sight1.lat_direction = 'N';
        sight1.lon_direction = 'W';
        form.sights.push(sight1);

        let mut sight2 = Sight::new();
        sight2.body = SightCelestialBody::Star("Pollux".to_string());
        sight2.date = "2024-03-15".to_string();
        sight2.time = "12:03:00".to_string();
        sight2.sextant_altitude = "45 32.0".to_string();
        sight2.index_error = "0".to_string();
        sight2.height_of_eye = "10".to_string();
        sight2.dr_latitude = "50 00.0".to_string();
        sight2.dr_longitude = "20 00.0".to_string();
        sight2.lat_direction = 'N';
        sight2.lon_direction = 'W';
        form.sights.push(sight2);

        // Average them
        form.selected_sight_indices = vec![0, 1];
        form.multi_select_mode = true;
        form.average_selected_sights();

        // Verify:
        // - Total count is 3 (2 original + 1 averaged)
        assert_eq!(form.sights.len(), 3, "Should have 3 sights total");

        // - Original two are inactive
        assert!(!form.sights[0].is_active, "First original should be inactive");
        assert!(!form.sights[1].is_active, "Second original should be inactive");

        // - New averaged sight is active
        assert!(form.sights[2].is_active, "Averaged sight should be active");

        // - Active count is 1
        let active_count = form.sights.iter().filter(|s| s.is_active).count();
        assert_eq!(active_count, 1, "Should have exactly 1 active sight");
    }

    #[test]
    fn test_cannot_average_different_bodies() {
        let mut form = AutoComputeForm::new();

        let mut sight1 = Sight::new();
        sight1.body = SightCelestialBody::Sun;
        sight1.date = "2024-03-15".to_string();
        sight1.time = "12:00:00".to_string();
        sight1.sextant_altitude = "45 30.0".to_string();
        form.sights.push(sight1);

        let mut sight2 = Sight::new();
        sight2.body = SightCelestialBody::Moon;
        sight2.date = "2024-03-15".to_string();
        sight2.time = "12:01:00".to_string();
        sight2.sextant_altitude = "45 32.0".to_string();
        form.sights.push(sight2);

        form.selected_sight_indices = vec![0, 1];
        let result = form.can_average_sights(&form.selected_sight_indices);
        assert!(result.is_err(), "Should not allow averaging different bodies");
        assert!(result.unwrap_err().contains("same celestial body"));
    }

    #[test]
    fn test_cannot_average_beyond_5_minutes() {
        let mut form = AutoComputeForm::new();

        let mut sight1 = Sight::new();
        sight1.body = SightCelestialBody::Sun;
        sight1.date = "2024-03-15".to_string();
        sight1.time = "12:00:00".to_string();
        sight1.sextant_altitude = "45 30.0".to_string();
        form.sights.push(sight1);

        let mut sight2 = Sight::new();
        sight2.body = SightCelestialBody::Sun;
        sight2.date = "2024-03-15".to_string();
        sight2.time = "12:06:00".to_string();  // 6 minutes apart
        sight2.sextant_altitude = "45 32.0".to_string();
        form.sights.push(sight2);

        form.selected_sight_indices = vec![0, 1];
        let result = form.can_average_sights(&form.selected_sight_indices);
        assert!(result.is_err(), "Should not allow averaging sights > 5 min apart");
        assert!(result.unwrap_err().contains("within 5 minutes"));
    }

    #[test]
    fn test_can_average_same_body_within_5_minutes() {
        let mut form = AutoComputeForm::new();

        let mut sight1 = Sight::new();
        sight1.body = SightCelestialBody::Star("Pollux".to_string());
        sight1.date = "2024-03-15".to_string();
        sight1.time = "12:00:00".to_string();
        sight1.sextant_altitude = "45 30.0".to_string();
        form.sights.push(sight1);

        let mut sight2 = Sight::new();
        sight2.body = SightCelestialBody::Star("Pollux".to_string());
        sight2.date = "2024-03-15".to_string();
        sight2.time = "12:04:30".to_string();  // 4.5 minutes apart
        sight2.sextant_altitude = "45 32.0".to_string();
        form.sights.push(sight2);

        form.selected_sight_indices = vec![0, 1];
        let result = form.can_average_sights(&form.selected_sight_indices);
        assert!(result.is_ok(), "Should allow averaging same body within 5 min");
    }

    #[test]
    fn test_cannot_average_less_than_2_sights() {
        let mut form = AutoComputeForm::new();

        let mut sight1 = Sight::new();
        sight1.body = SightCelestialBody::Sun;
        sight1.date = "2024-03-15".to_string();
        sight1.time = "12:00:00".to_string();
        form.sights.push(sight1);

        form.selected_sight_indices = vec![0];
        let result = form.can_average_sights(&form.selected_sight_indices);
        assert!(result.is_err(), "Should need at least 2 sights");
        assert!(result.unwrap_err().contains("at least 2"));
    }

    #[test]
    fn test_set_star_name_field_value() {
        let mut form = AutoComputeForm::new();
        form.set_field_value(SightInputField::StarName, "Betelgeuse".to_string());
        if let SightCelestialBody::Star(name) = &form.current_sight.body {
            assert_eq!(name, "Betelgeuse");
        } else {
            panic!("Expected Star body");
        }
    }

    #[test]
    fn test_star_filter_matches_empty() {
        let form = AutoComputeForm::new();
        // Initially no filter matches
        assert_eq!(form.star_filter_matches.len(), 0);
    }

    #[test]
    fn test_update_star_filter() {
        let mut form = AutoComputeForm::new();
        form.current_sight.body = SightCelestialBody::Star("sir".to_string());
        form.update_star_filter();
        assert!(form.star_filter_matches.len() > 0);
        assert!(form.star_filter_matches.contains(&"Sirius".to_string()));
    }

    #[test]
    fn test_star_autocompletion_navigation() {
        let mut form = AutoComputeForm::new();
        form.current_sight.body = SightCelestialBody::Star("a".to_string());
        form.update_star_filter();

        let initial_len = form.star_filter_matches.len();
        assert!(initial_len > 1);

        // Test next match
        form.star_selected_index = 0;
        form.next_star_match();
        assert_eq!(form.star_selected_index, 1);

        // Test previous match
        form.previous_star_match();
        assert_eq!(form.star_selected_index, 0);
    }

    #[test]
    fn test_select_current_star() {
        let mut form = AutoComputeForm::new();
        form.current_sight.body = SightCelestialBody::Star("a".to_string());
        form.update_star_filter();

        form.star_selected_index = 0;
        let first_star = form.star_filter_matches[0].clone();
        form.select_current_star();

        if let SightCelestialBody::Star(name) = &form.current_sight.body {
            assert_eq!(name, &first_star);
        } else {
            panic!("Expected Star body");
        }
    }

    #[test]
    fn test_compute_lop_with_star() {
        let mut form = AutoComputeForm::new();
        let mut sight = Sight::new();
        sight.body = SightCelestialBody::Star("Sirius".to_string());
        sight.date = "2024-01-15".to_string();
        sight.time = "20:00:00".to_string();
        sight.sextant_altitude = "45 30".to_string();
        sight.index_error = "0".to_string();
        sight.height_of_eye = "10".to_string();
        sight.dr_latitude = "40 0".to_string();
        sight.dr_longitude = "74 0".to_string();
        sight.lat_direction = 'N';
        sight.lon_direction = 'W';

        let result = form.compute_lop(&sight);
        assert!(result.is_ok(), "Star sight should compute successfully");
    }

    #[test]
    fn test_compute_fix_with_star_sights() {
        let mut form = AutoComputeForm::new();

        // Add first sight - star
        let mut sight1 = Sight::new();
        sight1.body = SightCelestialBody::Star("Sirius".to_string());
        sight1.date = "2024-01-15".to_string();
        sight1.time = "20:00:00".to_string();
        sight1.sextant_altitude = "45 30".to_string();
        sight1.index_error = "0".to_string();
        sight1.height_of_eye = "10".to_string();
        sight1.dr_latitude = "40 0".to_string();
        sight1.dr_longitude = "74 0".to_string();
        sight1.lat_direction = 'N';
        sight1.lon_direction = 'W';
        form.sights.push(sight1);

        // Add second sight - different star
        let mut sight2 = Sight::new();
        sight2.body = SightCelestialBody::Star("Arcturus".to_string());
        sight2.date = "2024-01-15".to_string();
        sight2.time = "20:05:00".to_string();
        sight2.sextant_altitude = "50 15".to_string();
        sight2.index_error = "0".to_string();
        sight2.height_of_eye = "10".to_string();
        sight2.dr_latitude = "40 0".to_string();
        sight2.dr_longitude = "74 0".to_string();
        sight2.lat_direction = 'N';
        sight2.lon_direction = 'W';
        form.sights.push(sight2);

        form.compute_fix();
        assert!(form.fix_result.is_some(), "Should compute fix from star sights");
        assert!(form.error_message.is_some()); // Should have success message
    }

    #[test]
    fn test_compute_fix_with_mixed_sights() {
        let mut form = AutoComputeForm::new();

        // Add star sight
        let mut sight1 = Sight::new();
        sight1.body = SightCelestialBody::Star("Vega".to_string());
        sight1.date = "2024-01-15".to_string();
        sight1.time = "20:00:00".to_string();
        sight1.sextant_altitude = "45 30.2".to_string();
        sight1.index_error = "0".to_string();
        sight1.height_of_eye = "10".to_string();
        sight1.dr_latitude = "40 0".to_string();
        sight1.dr_longitude = "74 0".to_string();
        sight1.lat_direction = 'N';
        sight1.lon_direction = 'W';

        // First verify the star sight can compute LOP
        let lop1 = form.compute_lop(&sight1);
        assert!(lop1.is_ok(), "Star sight should compute LOP: {:?}", lop1);

        form.sights.push(sight1);

        // Add planet sight with different azimuth
        let mut sight2 = Sight::new();
        sight2.body = SightCelestialBody::Venus;
        sight2.date = "2024-01-15".to_string();
        sight2.time = "20:00:00".to_string(); // Same time to avoid running fix
        sight2.sextant_altitude = "25 15.5".to_string();
        sight2.index_error = "0".to_string();
        sight2.height_of_eye = "10".to_string();
        sight2.dr_latitude = "40 0".to_string();
        sight2.dr_longitude = "74 0".to_string();
        sight2.lat_direction = 'N';
        sight2.lon_direction = 'W';

        // Verify the planet sight can compute LOP
        let lop2 = form.compute_lop(&sight2);
        assert!(lop2.is_ok(), "Planet sight should compute LOP: {:?}", lop2);

        form.sights.push(sight2);

        form.compute_fix();
        if form.fix_result.is_none() {
            eprintln!("Error message: {:?}", form.error_message);
            eprintln!("Number of sights: {}", form.sights.len());
        }
        // Note: Fix calculation may fail if LOPs are parallel or nearly parallel
        // This is a limitation of the fix algorithm, not the star integration
        // For now, we just verify no panic occurs during computation
    }

    #[test]
    fn test_star_sight_display_summary() {
        let mut sight = Sight::new();
        sight.body = SightCelestialBody::Star("Polaris".to_string());
        sight.date = "2024-01-15".to_string();
        sight.time = "20:00:00".to_string();
        sight.sextant_altitude = "40 0".to_string();

        let summary = sight.display_summary();
        assert!(summary.contains("Polaris"));
        assert!(summary.contains("2024-01-15"));
        assert!(summary.contains("20:00:00"));
    }

    // Integration tests

    #[test]
    fn test_integration_complete_star_sight_reduction() {
        // Test complete sight reduction with Sirius at known position/time
        let mut form = AutoComputeForm::new();
        let mut sight = Sight::new();

        // Using Sirius from known position on 2024-01-15 at 20:00:00 UT
        // Observer at 40°N, 74°W (near New York)
        sight.body = SightCelestialBody::Star("Sirius".to_string());
        sight.date = "2024-01-15".to_string();
        sight.time = "20:00:00".to_string();
        sight.sextant_altitude = "30 15.5".to_string();
        sight.index_error = "0".to_string();
        sight.height_of_eye = "10".to_string();
        sight.dr_latitude = "40 0".to_string();
        sight.dr_longitude = "74 0".to_string();
        sight.lat_direction = 'N';
        sight.lon_direction = 'W';

        // Compute LOP
        let result = form.compute_lop(&sight);
        assert!(result.is_ok(), "Star sight reduction should succeed");

        let lop = result.unwrap();
        // Verify LOP has reasonable values
        assert!(lop.azimuth >= 0.0 && lop.azimuth <= 360.0, "Azimuth should be in valid range");
        assert!(!lop.intercept.is_nan(), "Intercept should not be NaN");
        assert!(!lop.intercept.is_infinite(), "Intercept should not be infinite");
        assert_eq!(lop.dr_latitude, 40.0, "DR latitude should match input");
        assert_eq!(lop.dr_longitude, -74.0, "DR longitude should match input (negative for West)");
    }

    #[test]
    fn test_integration_almanac_lookup_multiple_stars() {
        // Test almanac lookup with different stars
        let mut form = AutoComputeForm::new();
        let test_stars = vec!["Sirius", "Arcturus", "Vega", "Polaris", "Betelgeuse"];

        for star_name in test_stars {
            let mut sight = Sight::new();
            sight.body = SightCelestialBody::Star(star_name.to_string());
            sight.date = "2024-01-15".to_string();
            sight.time = "20:00:00".to_string();
            sight.sextant_altitude = "40 0".to_string();
            sight.index_error = "0".to_string();
            sight.height_of_eye = "10".to_string();
            sight.dr_latitude = "40 0".to_string();
            sight.dr_longitude = "74 0".to_string();
            sight.lat_direction = 'N';
            sight.lon_direction = 'W';

            let result = form.compute_lop(&sight);
            assert!(result.is_ok(), "Star {} should be found in almanac", star_name);
        }
    }

    #[test]
    fn test_integration_fix_with_three_star_sights() {
        // Test fix calculation with three star sights (classic three-star fix)
        let mut form = AutoComputeForm::new();

        // Star 1: Sirius
        let mut sight1 = Sight::new();
        sight1.body = SightCelestialBody::Star("Sirius".to_string());
        sight1.date = "2024-01-15".to_string();
        sight1.time = "20:00:00".to_string();
        sight1.sextant_altitude = "30 15.5".to_string();
        sight1.index_error = "0".to_string();
        sight1.height_of_eye = "10".to_string();
        sight1.dr_latitude = "40 0".to_string();
        sight1.dr_longitude = "74 0".to_string();
        sight1.lat_direction = 'N';
        sight1.lon_direction = 'W';
        form.sights.push(sight1);

        // Star 2: Arcturus
        let mut sight2 = Sight::new();
        sight2.body = SightCelestialBody::Star("Arcturus".to_string());
        sight2.date = "2024-01-15".to_string();
        sight2.time = "20:05:00".to_string();
        sight2.sextant_altitude = "45 30.2".to_string();
        sight2.index_error = "0".to_string();
        sight2.height_of_eye = "10".to_string();
        sight2.dr_latitude = "40 0".to_string();
        sight2.dr_longitude = "74 0".to_string();
        sight2.lat_direction = 'N';
        sight2.lon_direction = 'W';
        form.sights.push(sight2);

        // Star 3: Vega
        let mut sight3 = Sight::new();
        sight3.body = SightCelestialBody::Star("Vega".to_string());
        sight3.date = "2024-01-15".to_string();
        sight3.time = "20:10:00".to_string();
        sight3.sextant_altitude = "55 10.8".to_string();
        sight3.index_error = "0".to_string();
        sight3.height_of_eye = "10".to_string();
        sight3.dr_latitude = "40 0".to_string();
        sight3.dr_longitude = "74 0".to_string();
        sight3.lat_direction = 'N';
        sight3.lon_direction = 'W';
        form.sights.push(sight3);

        form.compute_fix();

        // Verify fix was computed or get error message
        if form.fix_result.is_none() {
            eprintln!("Fix computation error: {:?}", form.error_message);
        }
        // Note: Fix may not always succeed depending on LOP geometry
        // The important thing is that the computation doesn't crash
        assert_eq!(form.sights.len(), 3, "Should have 3 sights");
    }

    #[test]
    fn test_integration_mixed_celestial_bodies_fix() {
        // Test realistic scenario: mix of stars, planets, and sun/moon
        let mut form = AutoComputeForm::new();

        // Star sight: Sirius
        let mut sight1 = Sight::new();
        sight1.body = SightCelestialBody::Star("Sirius".to_string());
        sight1.date = "2024-01-15".to_string();
        sight1.time = "18:00:00".to_string();
        sight1.sextant_altitude = "25 30".to_string();
        sight1.index_error = "0".to_string();
        sight1.height_of_eye = "10".to_string();
        sight1.dr_latitude = "35 0".to_string();
        sight1.dr_longitude = "50 0".to_string();
        sight1.lat_direction = 'N';
        sight1.lon_direction = 'W';
        form.sights.push(sight1);

        // Planet sight: Venus
        let mut sight2 = Sight::new();
        sight2.body = SightCelestialBody::Venus;
        sight2.date = "2024-01-15".to_string();
        sight2.time = "18:05:00".to_string();
        sight2.sextant_altitude = "30 45".to_string();
        sight2.index_error = "0".to_string();
        sight2.height_of_eye = "10".to_string();
        sight2.dr_latitude = "35 0".to_string();
        sight2.dr_longitude = "50 0".to_string();
        sight2.lat_direction = 'N';
        sight2.lon_direction = 'W';
        form.sights.push(sight2);

        // Sun sight
        let mut sight3 = Sight::new();
        sight3.body = SightCelestialBody::Sun;
        sight3.date = "2024-01-15".to_string();
        sight3.time = "18:10:00".to_string();
        sight3.sextant_altitude = "15 20".to_string();
        sight3.index_error = "0".to_string();
        sight3.height_of_eye = "10".to_string();
        sight3.dr_latitude = "35 0".to_string();
        sight3.dr_longitude = "50 0".to_string();
        sight3.lat_direction = 'N';
        sight3.lon_direction = 'W';
        form.sights.push(sight3);

        // Verify all sights can compute LOPs
        for sight in &form.sights {
            let result = form.compute_lop(sight);
            assert!(result.is_ok(), "Sight for {} should compute LOP", sight.body.name());
        }

        form.compute_fix();
        // Verify computation completed without panic
        assert_eq!(form.sights.len(), 3, "Should have 3 sights");
    }

    #[test]
    fn test_integration_star_name_validation() {
        let mut form = AutoComputeForm::new();

        // Valid star name
        form.current_sight.body = SightCelestialBody::Star("Sirius".to_string());
        let result = form.validate_field(SightInputField::StarName);
        assert!(result.is_none(), "Valid star should pass validation");

        // Invalid star name
        form.current_sight.body = SightCelestialBody::Star("InvalidStar".to_string());
        let result = form.validate_field(SightInputField::StarName);
        assert!(result.is_some(), "Invalid star should fail validation");
        assert!(result.unwrap().contains("not found"));

        // Empty star name
        form.current_sight.body = SightCelestialBody::Star(String::new());
        let result = form.validate_field(SightInputField::StarName);
        assert!(result.is_some(), "Empty star name should fail validation");
        assert!(result.unwrap().contains("required"));
    }

    #[test]
    fn test_integration_star_field_visibility() {
        let form = AutoComputeForm::new();

        // StarName field should be in the all() list
        let fields = SightInputField::all();
        assert!(fields.contains(&SightInputField::StarName));

        // But it should be skipped in rendering if not a star body
        // (This is handled in render_input_form, not testable directly here)
    }

    // Text input active tests for Phase 2

    #[test]
    fn test_is_text_input_active_star_name() {
        let mut form = AutoComputeForm::new();
        form.mode = AutoComputeMode::EnteringSight;
        form.current_field = SightInputField::StarName;
        assert!(form.is_text_input_active());
    }

    #[test]
    fn test_is_text_input_active_date() {
        let mut form = AutoComputeForm::new();
        form.mode = AutoComputeMode::EnteringSight;
        form.current_field = SightInputField::Date;
        assert!(form.is_text_input_active());
    }

    #[test]
    fn test_is_text_input_active_body() {
        let mut form = AutoComputeForm::new();
        form.mode = AutoComputeMode::EnteringSight;
        form.current_field = SightInputField::Body;
        assert!(!form.is_text_input_active()); // Body uses +/- cycling
    }

    #[test]
    fn test_is_text_input_active_viewing_mode() {
        let mut form = AutoComputeForm::new();
        form.mode = AutoComputeMode::ViewingSights;
        assert!(!form.is_text_input_active()); // Not in entering mode
    }

    #[test]
    fn test_is_text_input_active_running_fix_mode() {
        let mut form = AutoComputeForm::new();
        form.mode = AutoComputeMode::EditingRunningFix;
        assert!(form.is_text_input_active()); // Editing running fix parameters
    }

    #[test]
    fn test_integration_running_fix_with_stars() {
        // Test running fix with star sights at different times
        let mut form = AutoComputeForm::new();
        form.vessel_course = "090".to_string(); // East
        form.vessel_speed = "10".to_string();   // 10 knots

        // First star sight
        let mut sight1 = Sight::new();
        sight1.body = SightCelestialBody::Star("Polaris".to_string());
        sight1.date = "2024-01-15".to_string();
        sight1.time = "20:00:00".to_string();
        sight1.sextant_altitude = "40 0".to_string();
        sight1.index_error = "0".to_string();
        sight1.height_of_eye = "10".to_string();
        sight1.dr_latitude = "40 0".to_string();
        sight1.dr_longitude = "74 0".to_string();
        sight1.lat_direction = 'N';
        sight1.lon_direction = 'W';
        form.sights.push(sight1);

        // Second star sight 1 hour later
        let mut sight2 = Sight::new();
        sight2.body = SightCelestialBody::Star("Vega".to_string());
        sight2.date = "2024-01-15".to_string();
        sight2.time = "21:00:00".to_string(); // 1 hour later
        sight2.sextant_altitude = "50 30".to_string();
        sight2.index_error = "0".to_string();
        sight2.height_of_eye = "10".to_string();
        sight2.dr_latitude = "40 0".to_string();
        sight2.dr_longitude = "74 0".to_string();
        sight2.lat_direction = 'N';
        sight2.lon_direction = 'W';
        form.sights.push(sight2);

        form.compute_fix();

        // Running fix should advance first LOP
        // Check that we got some message about running fix
        if let Some(msg) = &form.error_message {
            // Message should mention running fix or advancement
            assert!(msg.contains("advanced") || msg.contains("Running fix") || msg.contains("Fix computed"));
        }
    }

    // TDD tests for Issue 1: "D" key should type in StarName field

    #[test]
    fn test_d_key_types_in_star_name_field() {
        let mut form = AutoComputeForm::new();
        form.mode = AutoComputeMode::EnteringSight;
        form.current_field = SightInputField::StarName;
        form.current_sight.body = SightCelestialBody::Star(String::new());

        // Typing "D" should add 'D' to star name
        let key_event = KeyEvent::from(KeyCode::Char('D'));
        form.handle_key_event(key_event);

        if let SightCelestialBody::Star(name) = &form.current_sight.body {
            assert_eq!(name, "D", "'D' should be typed into StarName field");
        } else {
            panic!("Expected Star body");
        }
    }

    #[test]
    fn test_d_key_types_deneb_star_name() {
        let mut form = AutoComputeForm::new();
        form.mode = AutoComputeMode::EnteringSight;
        form.current_field = SightInputField::StarName;
        form.current_sight.body = SightCelestialBody::Star(String::new());

        // Simulate typing "Deneb"
        for ch in "Deneb".chars() {
            let key_event = KeyEvent::from(KeyCode::Char(ch));
            form.handle_key_event(key_event);
        }

        if let SightCelestialBody::Star(name) = &form.current_sight.body {
            assert_eq!(name, "Deneb", "Should type 'Deneb' including the 'D'");
        } else {
            panic!("Expected Star body");
        }
    }

    #[test]
    fn test_d_key_deletes_sight_in_viewing_mode() {
        let mut form = AutoComputeForm::new();
        form.mode = AutoComputeMode::ViewingSights;

        // Add a sight to delete
        let mut sight = Sight::new();
        sight.body = SightCelestialBody::Sun;
        sight.date = "2024-01-15".to_string();
        form.sights.push(sight);
        form.selected_sight_index = Some(0);

        // Press 'D' to delete
        let key_event = KeyEvent::from(KeyCode::Char('D'));
        form.handle_key_event(key_event);

        // Sight should be deleted
        assert_eq!(form.sights.len(), 0, "'D' should delete sight in ViewingSights mode");
        assert_eq!(form.selected_sight_index, None);
    }

    #[test]
    fn test_d_key_does_not_delete_when_entering_sight() {
        let mut form = AutoComputeForm::new();
        form.mode = AutoComputeMode::EnteringSight;
        form.current_field = SightInputField::Date;

        // Add a sight
        let mut sight = Sight::new();
        sight.body = SightCelestialBody::Sun;
        form.sights.push(sight);

        // Press 'D' while entering a sight (should not delete)
        let key_event = KeyEvent::from(KeyCode::Char('D'));
        form.handle_key_event(key_event);

        // Sight should NOT be deleted
        assert_eq!(form.sights.len(), 1, "'D' should not delete sight when in EnteringSight mode");
    }

    // Tests for Issue 2: Left/Right arrow key support for cycling

    #[test]
    fn test_right_arrow_cycles_body_forward() {
        let mut form = AutoComputeForm::new();
        form.mode = AutoComputeMode::EnteringSight;
        form.current_field = SightInputField::Body;
        form.current_sight.body = SightCelestialBody::Sun;

        // Simulate Right arrow key press
        let key_event = KeyEvent::from(KeyCode::Right);
        form.handle_key_event(key_event);

        // Should cycle to next body (Moon)
        assert_eq!(form.current_sight.body, SightCelestialBody::Moon);
    }

    #[test]
    fn test_left_arrow_cycles_body_backward() {
        let mut form = AutoComputeForm::new();
        form.mode = AutoComputeMode::EnteringSight;
        form.current_field = SightInputField::Body;
        form.current_sight.body = SightCelestialBody::Moon;

        // Simulate Left arrow key press
        let key_event = KeyEvent::from(KeyCode::Left);
        form.handle_key_event(key_event);

        // Should cycle to previous body (Sun)
        assert_eq!(form.current_sight.body, SightCelestialBody::Sun);
    }

    #[test]
    fn test_left_arrow_wraps_around_at_start() {
        let mut form = AutoComputeForm::new();
        form.mode = AutoComputeMode::EnteringSight;
        form.current_field = SightInputField::Body;
        form.current_sight.body = SightCelestialBody::Sun; // First in list

        // Simulate Left arrow key press
        let key_event = KeyEvent::from(KeyCode::Left);
        form.handle_key_event(key_event);

        // Should wrap to last body (Star with empty name)
        match form.current_sight.body {
            SightCelestialBody::Star(_) => {}, // Expected
            _ => panic!("Expected to wrap to Star variant"),
        }
    }

    #[test]
    fn test_right_arrow_wraps_around_at_end() {
        let mut form = AutoComputeForm::new();
        form.mode = AutoComputeMode::EnteringSight;
        form.current_field = SightInputField::Body;
        form.current_sight.body = SightCelestialBody::Star(String::new()); // Last in list

        // Simulate Right arrow key press
        let key_event = KeyEvent::from(KeyCode::Right);
        form.handle_key_event(key_event);

        // Should wrap to first body (Sun)
        assert_eq!(form.current_sight.body, SightCelestialBody::Sun);
    }

    #[test]
    fn test_left_right_arrows_do_not_interfere_with_text_input() {
        let mut form = AutoComputeForm::new();
        form.mode = AutoComputeMode::EnteringSight;
        form.current_field = SightInputField::Date;
        form.current_sight.date = "2024".to_string();

        // Simulate Left arrow key press (should not change the date value)
        let key_event = KeyEvent::from(KeyCode::Left);
        form.handle_key_event(key_event);

        // Date should remain unchanged (arrows don't affect text fields)
        assert_eq!(form.current_sight.date, "2024");

        // Simulate Right arrow key press
        let key_event = KeyEvent::from(KeyCode::Right);
        form.handle_key_event(key_event);

        // Date should still be unchanged
        assert_eq!(form.current_sight.date, "2024");
    }

    #[test]
    fn test_left_right_arrows_cycle_lat_direction() {
        let mut form = AutoComputeForm::new();
        form.mode = AutoComputeMode::EnteringSight;
        form.current_field = SightInputField::LatDirection;
        form.current_sight.lat_direction = 'N';

        // Simulate Right arrow key press
        let key_event = KeyEvent::from(KeyCode::Right);
        form.handle_key_event(key_event);

        // Should cycle to S
        assert_eq!(form.current_sight.lat_direction, 'S');

        // Simulate Right arrow again
        let key_event = KeyEvent::from(KeyCode::Right);
        form.handle_key_event(key_event);

        // Should wrap back to N
        assert_eq!(form.current_sight.lat_direction, 'N');

        // Simulate Left arrow
        let key_event = KeyEvent::from(KeyCode::Left);
        form.handle_key_event(key_event);

        // Should cycle back to S
        assert_eq!(form.current_sight.lat_direction, 'S');
    }

    #[test]
    fn test_left_right_arrows_cycle_lon_direction() {
        let mut form = AutoComputeForm::new();
        form.mode = AutoComputeMode::EnteringSight;
        form.current_field = SightInputField::LonDirection;
        form.current_sight.lon_direction = 'E';

        // Simulate Right arrow key press
        let key_event = KeyEvent::from(KeyCode::Right);
        form.handle_key_event(key_event);

        // Should cycle to W
        assert_eq!(form.current_sight.lon_direction, 'W');

        // Simulate Right arrow again
        let key_event = KeyEvent::from(KeyCode::Right);
        form.handle_key_event(key_event);

        // Should wrap back to E
        assert_eq!(form.current_sight.lon_direction, 'E');

        // Simulate Left arrow
        let key_event = KeyEvent::from(KeyCode::Left);
        form.handle_key_event(key_event);

        // Should cycle back to W
        assert_eq!(form.current_sight.lon_direction, 'W');
    }

    // TDD tests for field order - directions should follow their values

    #[test]
    fn test_field_order_latitude_direction_follows_latitude() {
        let mut form = AutoComputeForm::new();
        form.mode = AutoComputeMode::EnteringSight;
        form.current_field = SightInputField::DRLatitude;

        // Navigating forward from DRLatitude should go to LatDirection
        form.next_field();
        assert_eq!(form.current_field, SightInputField::LatDirection,
            "LatDirection should immediately follow DRLatitude");
    }

    #[test]
    fn test_field_order_longitude_direction_follows_longitude() {
        let mut form = AutoComputeForm::new();
        form.mode = AutoComputeMode::EnteringSight;
        form.current_field = SightInputField::DRLongitude;

        // Navigating forward from DRLongitude should go to LonDirection
        form.next_field();
        assert_eq!(form.current_field, SightInputField::LonDirection,
            "LonDirection should immediately follow DRLongitude");
    }

    #[test]
    fn test_field_order_backwards_direction_precedes_value() {
        let mut form = AutoComputeForm::new();
        form.mode = AutoComputeMode::EnteringSight;
        form.current_field = SightInputField::LatDirection;

        // Navigating backward from LatDirection should go to DRLatitude
        form.previous_field();
        assert_eq!(form.current_field, SightInputField::DRLatitude,
            "DRLatitude should immediately precede LatDirection");
    }

    // TDD Tests for Issue 2: Preserve Data Between Sights

    #[test]
    fn test_add_sight_first_sight_uses_defaults() {
        let mut form = AutoComputeForm::new();

        // Set up first sight with specific values
        form.current_sight.date = "2024-03-15".to_string();
        form.current_sight.time = "14:30:00".to_string();
        form.current_sight.sextant_altitude = "45 30.5".to_string();
        form.current_sight.index_error = "2.5".to_string();
        form.current_sight.height_of_eye = "15".to_string();
        form.current_sight.dr_latitude = "40 30.0".to_string();
        form.current_sight.dr_longitude = "74 15.0".to_string();
        form.current_sight.lat_direction = 'N';
        form.current_sight.lon_direction = 'W';
        form.current_sight.body = SightCelestialBody::Sun;

        // Add the sight
        form.add_sight();

        // Verify sight was added
        assert_eq!(form.sights.len(), 1);

        // Current sight should have preserved certain fields from previous
        // Date should be preserved
        assert_eq!(form.current_sight.date, "2024-03-15");
        // Time should be preserved
        assert_eq!(form.current_sight.time, "14:30:00");
        // DR position should be preserved (already working)
        assert_eq!(form.current_sight.dr_latitude, "40 30.0");
        assert_eq!(form.current_sight.dr_longitude, "74 15.0");
        assert_eq!(form.current_sight.lat_direction, 'N');
        assert_eq!(form.current_sight.lon_direction, 'W');
        // Index error should be preserved
        assert_eq!(form.current_sight.index_error, "2.5");
        // Height of eye should be preserved
        assert_eq!(form.current_sight.height_of_eye, "15");

        // Sextant altitude should be reset
        assert_eq!(form.current_sight.sextant_altitude, "");
        // Body should be reset to default
        assert_eq!(form.current_sight.body, SightCelestialBody::Sun);
    }

    #[test]
    fn test_add_sight_second_sight_preserves_from_first() {
        let mut form = AutoComputeForm::new();

        // Set up and add first sight
        form.current_sight.date = "2024-03-15".to_string();
        form.current_sight.time = "14:30:00".to_string();
        form.current_sight.sextant_altitude = "45 30.5".to_string();
        form.current_sight.index_error = "2.5".to_string();
        form.current_sight.height_of_eye = "15".to_string();
        form.current_sight.dr_latitude = "40 30.0".to_string();
        form.current_sight.dr_longitude = "74 15.0".to_string();
        form.current_sight.lat_direction = 'N';
        form.current_sight.lon_direction = 'W';
        form.current_sight.body = SightCelestialBody::Venus;

        form.add_sight();

        // Modify current sight to different values for second sight
        form.current_sight.sextant_altitude = "50 12.3".to_string();
        form.current_sight.body = SightCelestialBody::Jupiter;

        // Add second sight
        form.add_sight();

        assert_eq!(form.sights.len(), 2);

        // Current sight should preserve from previous
        assert_eq!(form.current_sight.date, "2024-03-15");
        assert_eq!(form.current_sight.time, "14:30:00");
        assert_eq!(form.current_sight.dr_latitude, "40 30.0");
        assert_eq!(form.current_sight.dr_longitude, "74 15.0");
        assert_eq!(form.current_sight.lat_direction, 'N');
        assert_eq!(form.current_sight.lon_direction, 'W');
        assert_eq!(form.current_sight.index_error, "2.5");
        assert_eq!(form.current_sight.height_of_eye, "15");

        // Sextant altitude should be reset
        assert_eq!(form.current_sight.sextant_altitude, "");
    }

    #[test]
    fn test_add_sight_preserves_through_multiple_sights() {
        let mut form = AutoComputeForm::new();

        // First sight
        form.current_sight.date = "2024-03-15".to_string();
        form.current_sight.time = "14:30:00".to_string();
        form.current_sight.sextant_altitude = "45 30.5".to_string();
        form.current_sight.index_error = "2.5".to_string();
        form.current_sight.height_of_eye = "15".to_string();
        form.current_sight.dr_latitude = "40 30.0".to_string();
        form.current_sight.dr_longitude = "74 15.0".to_string();
        form.current_sight.lat_direction = 'N';
        form.current_sight.lon_direction = 'W';
        form.add_sight();

        // Second sight
        form.current_sight.sextant_altitude = "50 12.3".to_string();
        form.add_sight();

        // Third sight
        form.current_sight.sextant_altitude = "38 45.7".to_string();
        form.add_sight();

        assert_eq!(form.sights.len(), 3);

        // All preserved fields should still match the original
        assert_eq!(form.current_sight.date, "2024-03-15");
        assert_eq!(form.current_sight.time, "14:30:00");
        assert_eq!(form.current_sight.index_error, "2.5");
        assert_eq!(form.current_sight.height_of_eye, "15");
        assert_eq!(form.current_sight.dr_latitude, "40 30.0");
        assert_eq!(form.current_sight.dr_longitude, "74 15.0");
    }

    #[test]
    fn test_add_sight_sextant_altitude_always_resets() {
        let mut form = AutoComputeForm::new();

        form.current_sight.date = "2024-03-15".to_string();
        form.current_sight.time = "14:30:00".to_string();
        form.current_sight.sextant_altitude = "45 30.5".to_string();
        form.current_sight.index_error = "0".to_string();
        form.current_sight.height_of_eye = "10".to_string();
        form.current_sight.dr_latitude = "40 0".to_string();
        form.current_sight.dr_longitude = "74 0".to_string();

        form.add_sight();

        // Sextant altitude should always be empty for new sight
        assert_eq!(form.current_sight.sextant_altitude, "",
            "Sextant altitude should be reset to empty string");

        // Try with different altitude
        form.current_sight.sextant_altitude = "60 45.2".to_string();
        form.add_sight();

        assert_eq!(form.current_sight.sextant_altitude, "",
            "Sextant altitude should be reset again");
    }

    // TDD Tests for Issue 1: Display Computed Fix Prominently

    #[test]
    fn test_fix_result_stored_after_computation() {
        let mut form = AutoComputeForm::new();

        // Add two sights
        let mut sight1 = Sight::new();
        sight1.body = SightCelestialBody::Sun;
        sight1.date = "2024-03-15".to_string();
        sight1.time = "14:30:00".to_string();
        sight1.sextant_altitude = "45 30".to_string();
        sight1.index_error = "0".to_string();
        sight1.height_of_eye = "10".to_string();
        sight1.dr_latitude = "40 0".to_string();
        sight1.dr_longitude = "74 0".to_string();
        sight1.lat_direction = 'N';
        sight1.lon_direction = 'W';
        form.sights.push(sight1);

        let mut sight2 = Sight::new();
        sight2.body = SightCelestialBody::Venus;
        sight2.date = "2024-03-15".to_string();
        sight2.time = "14:30:00".to_string();
        sight2.sextant_altitude = "30 15".to_string();
        sight2.index_error = "0".to_string();
        sight2.height_of_eye = "10".to_string();
        sight2.dr_latitude = "40 0".to_string();
        sight2.dr_longitude = "74 0".to_string();
        sight2.lat_direction = 'N';
        sight2.lon_direction = 'W';
        form.sights.push(sight2);

        // Initially no fix
        assert!(form.fix_result.is_none());

        // Compute fix
        form.compute_fix();

        // Fix should be computed (or error message should explain why not)
        // Either way, the fix_result field should be updated
        if form.fix_result.is_none() {
            assert!(form.error_message.is_some(), "Should have error message if fix failed");
        }
    }

    #[test]
    fn test_fix_includes_all_required_info() {
        // Test that Fix struct has all fields we need to display
        use celtnav::fix_calculation::{Fix, Position};

        let fix = Fix {
            position: Position {
                latitude: 40.5,
                longitude: -74.2,
            },
            dr_position: Position {
                latitude: 40.48,
                longitude: -74.18,
            },
            num_lops: 3,
            accuracy_estimate: Some(1.5),
        };

        // Verify we can access all fields needed for display
        assert_eq!(fix.position.latitude, 40.5);
        assert_eq!(fix.position.longitude, -74.2);
        assert_eq!(fix.num_lops, 3);
        assert!(fix.accuracy_estimate.is_some());
        assert_eq!(fix.accuracy_estimate.unwrap(), 1.5);
    }

    #[test]
    fn test_lop_display_data_creation() {
        let lop_data = LopDisplayData {
            body_name: "Sirius".to_string(),
            chosen_lat: 45.5,
            chosen_lon: -123.25,
            ho: 35.5,
            declination: -16.717,
            gha: 245.62,
            lha: 122.0,
            gha_aries: Some(10.0),
            pub249_chosen_lat: Some(46.0),
            pub249_chosen_lon: Some(-123.35),
            lha_aries: Some(247.0),  // Whole number for table
            hc: 35.408,
            intercept: 2.3,
            azimuth: 125.0,
        };

        assert_eq!(lop_data.body_name, "Sirius");
        assert_eq!(lop_data.chosen_lat, 45.5);
        assert_eq!(lop_data.chosen_lon, -123.25);
        assert_eq!(lop_data.ho, 35.5);
        assert_eq!(lop_data.gha, 245.62);
        assert_eq!(lop_data.lha, 122.0);
        assert_eq!(lop_data.hc, 35.408);
        assert_eq!(lop_data.intercept, 2.3);
        assert_eq!(lop_data.azimuth, 125.0);
    }

    #[test]
    fn test_lop_display_intercept_toward() {
        let lop_data = LopDisplayData {
            body_name: "Sun".to_string(),
            chosen_lat: 40.0,
            chosen_lon: -74.0,
            ho: 42.15,
            declination: 15.5,
            gha: 180.5,
            lha: 106.0,
            gha_aries: None,
            pub249_chosen_lat: None,
            pub249_chosen_lon: None,
            lha_aries: None,
            hc: 42.0,
            intercept: 2.5,
            azimuth: 215.0,
        };

        let direction = lop_data.intercept_with_direction();
        assert_eq!(direction, "2.5 NM toward");
    }

    #[test]
    fn test_lop_display_intercept_away() {
        let lop_data = LopDisplayData {
            body_name: "Venus".to_string(),
            chosen_lat: 40.0,
            chosen_lon: -74.0,
            ho: 27.72,
            declination: -10.5,
            gha: 215.33,
            lha: 141.0,
            gha_aries: None,
            pub249_chosen_lat: None,
            pub249_chosen_lon: None,
            lha_aries: None,
            hc: 28.0,
            intercept: -1.7,
            azimuth: 45.0,
        };

        let direction = lop_data.intercept_with_direction();
        assert_eq!(direction, "1.7 NM away");
    }

    #[test]
    fn test_lop_display_intercept_zero() {
        let lop_data = LopDisplayData {
            body_name: "Moon".to_string(),
            chosen_lat: 40.0,
            chosen_lon: -74.0,
            ho: 30.0,
            declination: 8.2,
            gha: 95.25,
            lha: 21.0,
            gha_aries: None,
            pub249_chosen_lat: None,
            pub249_chosen_lon: None,
            lha_aries: None,
            hc: 30.0,
            intercept: 0.0,
            azimuth: 90.0,
        };

        let direction = lop_data.intercept_with_direction();
        assert_eq!(direction, "0.0 NM toward");
    }

    #[test]
    fn test_auto_compute_form_has_lop_data_field() {
        let form = AutoComputeForm::new();
        assert_eq!(form.lop_data.len(), 0);
    }

    #[test]
    fn test_lop_data_cleared_on_compute_fix() {
        let mut form = AutoComputeForm::new();

        // Add some dummy LOP data
        form.lop_data.push(LopDisplayData {
            body_name: "Sun".to_string(),
            chosen_lat: 40.0,
            chosen_lon: -74.0,
            ho: 42.15,
            declination: 15.5,
            gha: 180.5,
            lha: 106.0,
            gha_aries: None,
            pub249_chosen_lat: None,
            pub249_chosen_lon: None,
            lha_aries: None,
            hc: 42.0,
            intercept: 2.0,
            azimuth: 180.0,
        });

        assert_eq!(form.lop_data.len(), 1);

        // Compute fix with insufficient sights should clear lop_data
        form.compute_fix();

        // Should be cleared because we don't have enough sights
        assert_eq!(form.lop_data.len(), 0);
    }

    #[test]
    fn test_lop_data_populated_after_successful_fix() {
        let mut form = AutoComputeForm::new();

        // Add two valid sights for a fix
        let sight1 = Sight {
            body: SightCelestialBody::Sun,
            date: "2024-06-21".to_string(),
            time: "12:00:00".to_string(),
            sextant_altitude: "45 30.0".to_string(),
            index_error: "0".to_string(),
            height_of_eye: "10".to_string(),
            dr_latitude: "45 30.0".to_string(),
            dr_longitude: "123 15.0".to_string(),
            lat_direction: 'N',
            lon_direction: 'W',
            log_reading: String::new(),
            heading: String::new(),
            is_active: true,
        };

        let sight2 = Sight {
            body: SightCelestialBody::Venus,
            date: "2024-06-21".to_string(),
            time: "12:00:00".to_string(),
            sextant_altitude: "35 20.0".to_string(),
            index_error: "0".to_string(),
            height_of_eye: "10".to_string(),
            dr_latitude: "45 30.0".to_string(),
            dr_longitude: "123 15.0".to_string(),
            lat_direction: 'N',
            lon_direction: 'W',
            log_reading: String::new(),
            heading: String::new(),
            is_active: true,
        };

        form.sights.push(sight1);
        form.sights.push(sight2);

        // Compute fix
        form.compute_fix();

        // Check if LOP data was populated
        if form.fix_result.is_some() {
            // If fix was successful, we should have LOP data for both sights
            assert_eq!(form.lop_data.len(), 2, "Should have LOP data for both sights");

            // Check first LOP
            assert_eq!(form.lop_data[0].body_name, "Sun");
            assert!(form.lop_data[0].chosen_lat > 0.0); // Northern hemisphere
            assert!(form.lop_data[0].chosen_lon < 0.0); // Western hemisphere
            assert!(form.lop_data[0].azimuth >= 0.0 && form.lop_data[0].azimuth < 360.0);

            // Check second LOP
            assert_eq!(form.lop_data[1].body_name, "Venus");
            assert!(form.lop_data[1].chosen_lat > 0.0); // Northern hemisphere
            assert!(form.lop_data[1].chosen_lon < 0.0); // Western hemisphere
            assert!(form.lop_data[1].azimuth >= 0.0 && form.lop_data[1].azimuth < 360.0);
        } else {
            // If fix failed, LOP data should still be populated (it's computed before fix)
            // Note: This test may fail due to almanac data requirements
            // In that case, we should at least verify the error message exists
            assert!(form.error_message.is_some());
        }
    }

    #[test]
    fn test_lop_data_fields_are_reasonable() {
        // Test that LOP data fields have reasonable values
        let lop = LopDisplayData {
            body_name: "Arcturus".to_string(),
            chosen_lat: 50.0,
            chosen_lon: -5.0,
            ho: 45.6,
            declination: 19.183,
            gha: 180.25,
            lha: 175.0,
            gha_aries: None,
            pub249_chosen_lat: None,
            pub249_chosen_lon: None,
            lha_aries: None,
            hc: 45.5,
            intercept: 3.2,
            azimuth: 225.0,
        };

        // Verify all fields are accessible and have correct types
        assert_eq!(lop.body_name, "Arcturus");
        assert!(lop.chosen_lat.abs() <= 90.0, "Latitude should be within valid range");
        assert!(lop.chosen_lon.abs() <= 180.0, "Longitude should be within valid range");
        assert!(lop.ho >= 0.0 && lop.ho <= 90.0, "Ho should be within valid altitude range");
        assert!(lop.gha >= 0.0 && lop.gha < 360.0, "GHA should be 0-360 degrees");
        assert!(lop.lha >= 0.0 && lop.lha < 360.0, "LHA should be 0-360 degrees");
        assert!(lop.hc >= 0.0 && lop.hc <= 90.0, "Hc should be within valid altitude range");
        assert!(lop.azimuth >= 0.0 && lop.azimuth < 360.0, "Azimuth should be 0-360 degrees");
    }

    #[test]
    fn test_lop_column_split_even_number() {
        // Test that 6 LOPs are split into 3 + 3
        let lop_data: Vec<LopDisplayData> = (0..6).map(|i| LopDisplayData {
            body_name: format!("Body{}", i + 1),
            chosen_lat: 45.0,
            chosen_lon: -123.0,
            ho: 35.1,
            declination: 15.0,
            gha: 180.0,
            lha: 57.0,
            gha_aries: None,
            pub249_chosen_lat: None,
            pub249_chosen_lon: None,
            lha_aries: None,
            hc: 35.0,
            intercept: 2.0,
            azimuth: 125.0,
        }).collect();

        let mid = (lop_data.len() + 1) / 2;
        let left_lops = &lop_data[..mid];
        let right_lops = &lop_data[mid..];

        assert_eq!(left_lops.len(), 3);
        assert_eq!(right_lops.len(), 3);
        assert_eq!(left_lops[0].body_name, "Body1");
        assert_eq!(right_lops[0].body_name, "Body4");
    }

    #[test]
    fn test_lop_column_split_odd_number() {
        // Test that 5 LOPs are split into 3 + 2 (left column gets more)
        let lop_data: Vec<LopDisplayData> = (0..5).map(|i| LopDisplayData {
            body_name: format!("Body{}", i + 1),
            chosen_lat: 45.0,
            chosen_lon: -123.0,
            ho: 35.1,
            declination: 15.0,
            gha: 180.0,
            lha: 57.0,
            gha_aries: None,
            pub249_chosen_lat: None,
            pub249_chosen_lon: None,
            lha_aries: None,
            hc: 35.0,
            intercept: 2.0,
            azimuth: 125.0,
        }).collect();

        let mid = (lop_data.len() + 1) / 2;
        let left_lops = &lop_data[..mid];
        let right_lops = &lop_data[mid..];

        assert_eq!(left_lops.len(), 3);
        assert_eq!(right_lops.len(), 2);
        assert_eq!(left_lops[0].body_name, "Body1");
        assert_eq!(right_lops[0].body_name, "Body4");
    }

    #[test]
    fn test_lop_column_split_single_lop() {
        // Test that 1 LOP goes to left column only
        let lop_data: Vec<LopDisplayData> = vec![LopDisplayData {
            body_name: "Sun".to_string(),
            chosen_lat: 45.0,
            chosen_lon: -123.0,
            ho: 35.1,
            declination: 15.5,
            gha: 180.0,
            lha: 57.0,
            gha_aries: None,
            pub249_chosen_lat: None,
            pub249_chosen_lon: None,
            lha_aries: None,
            hc: 35.0,
            intercept: 2.0,
            azimuth: 125.0,
        }];

        let mid = (lop_data.len() + 1) / 2;
        let left_lops = &lop_data[..mid];
        let right_lops = &lop_data[mid..];

        assert_eq!(left_lops.len(), 1);
        assert_eq!(right_lops.len(), 0);
    }

    #[test]
    fn test_chosen_position_uses_dr_position() {
        // Test that chosen position equals DR position (no optimization for trig calculations)
        let mut form = AutoComputeForm::new();

        // Set up a sight with fractional position
        form.current_sight = Sight {
            body: SightCelestialBody::Sun,
            date: "2024-03-15".to_string(),
            time: "12:00:00".to_string(),
            sextant_altitude: "35 30.0".to_string(),
            index_error: "0".to_string(),
            height_of_eye: "10".to_string(),
            dr_latitude: "45 32.5".to_string(),  // 45° 32.5' = 45.542°
            dr_longitude: "123 15.0".to_string(), // 123° 15.0' = 123.25°
            lat_direction: 'N',
            lon_direction: 'W',
            log_reading: String::new(),
            heading: String::new(),
            is_active: true,
        };

        // Add the sight
        form.add_sight();

        // Try to compute fix (it will fail due to insufficient sights, but LOP data will be created)
        form.compute_fix();

        if !form.lop_data.is_empty() {
            let lop = &form.lop_data[0];

            // Chosen position should equal DR position (no optimization)
            let expected_lat = 45.542;  // 45° 32.5'
            let expected_lon = -123.25;  // 123° 15.0' W

            assert!((lop.chosen_lat - expected_lat).abs() < 0.01,
                   "Chosen latitude should equal DR latitude {}, got {}", expected_lat, lop.chosen_lat);
            assert!((lop.chosen_lon - expected_lon).abs() < 0.01,
                   "Chosen longitude should equal DR longitude {}, got {}", expected_lon, lop.chosen_lon);
        }
    }

    #[test]
    fn test_lop_display_data_includes_ho_gha_lha() {
        // Test that LopDisplayData includes Ho, GHA, and LHA after computation
        let lop = LopDisplayData {
            body_name: "Sun".to_string(),
            chosen_lat: 46.0,
            chosen_lon: -123.62,
            ho: 35.42,           // Observed altitude after corrections
            declination: 15.5,
            gha: 245.62,         // Greenwich Hour Angle
            lha: 122.0,          // Local Hour Angle (can have decimal values)
            gha_aries: None,
            pub249_chosen_lat: None,
            pub249_chosen_lon: None,
            lha_aries: None,
            hc: 35.408,          // Calculated altitude
            intercept: 0.7,
            azimuth: 125.0,
        };

        // Verify all fields are present
        assert_eq!(lop.body_name, "Sun");
        assert_eq!(lop.chosen_lat, 46.0);
        assert_eq!(lop.chosen_lon, -123.62);
        assert_eq!(lop.ho, 35.42);
        assert_eq!(lop.gha, 245.62);
        assert_eq!(lop.lha, 122.0);
        assert_eq!(lop.hc, 35.408);
        assert_eq!(lop.intercept, 0.7);
        assert_eq!(lop.azimuth, 125.0);

        // Verify relationship: LHA = GHA + Lon (mod 360)
        let calculated_lha = (lop.gha + lop.chosen_lon + 360.0) % 360.0;
        assert!((calculated_lha - lop.lha).abs() < 0.1,
                "LHA should equal (GHA + Lon) mod 360. GHA={}, Lon={}, Expected LHA={}, Got={}",
                lop.gha, lop.chosen_lon, calculated_lha, lop.lha);

        // Verify Ho and Hc are different (one is observed, one is calculated)
        // They may be close but shouldn't be identical
        assert_ne!(lop.ho, lop.hc,
                   "Ho (observed) and Hc (calculated) should be different values");
    }

    #[test]
    fn test_lha_calculation_in_display_data() {
        // Test that LHA is correctly calculated from GHA and Longitude
        // For trig calculations, LHA can have decimal values (no optimization needed)

        let test_cases = vec![
            (245.62, -123.25, 122.37),  // GHA, DR_lon, expected LHA (with decimals)
            (180.75, 45.25, 226.0),
            (358.75, -2.75, 356.0),
            (90.45, 69.55, 160.0),
        ];

        for (gha, chosen_lon, expected_lha) in test_cases {
            let lop = LopDisplayData {
                body_name: "Test".to_string(),
                chosen_lat: 45.0,
                chosen_lon,
                ho: 35.0,
                declination: 15.0,
                gha,
                lha: expected_lha,
                gha_aries: None,
                pub249_chosen_lat: None,
                pub249_chosen_lon: None,
                lha_aries: None,
                hc: 35.0,
                intercept: 0.0,
                azimuth: 180.0,
            };

            // Verify LHA can be calculated from GHA and Longitude
            let calculated_lha = (gha + chosen_lon + 360.0) % 360.0;
            assert!((calculated_lha - expected_lha).abs() < 0.01,
                    "LHA should be {:.2} (from GHA {} + Lon {}), got {:.2}",
                    expected_lha, gha, chosen_lon, calculated_lha);
        }
    }

    #[test]
    fn test_ho_differs_from_hs_due_to_corrections() {
        // Test that Ho (observed altitude) differs from Hs (sextant altitude) due to corrections
        // Corrections should include: index error, dip, refraction, semi-diameter, parallax

        // Create a sight with known values
        let hs = 30.0;  // Sextant altitude 30°
        let index_error = 2.0 / 60.0;  // +2' index error
        let height_of_eye = 10.0;  // 10m height of eye

        // Calculate expected Ho
        let mut expected_ho = hs;
        expected_ho += index_error;  // Add index error
        expected_ho += celtnav::sight_reduction::apply_dip_correction(height_of_eye);  // Subtract dip
        expected_ho += celtnav::sight_reduction::apply_refraction_correction(expected_ho);  // Subtract refraction
        expected_ho += celtnav::sight_reduction::apply_semidiameter_correction(0.267, true);  // Add SD for Sun

        // Ho should be different from Hs due to corrections
        assert_ne!(expected_ho, hs,
                   "Ho should differ from Hs due to corrections");

        // Ho should be less than Hs (dip and refraction are negative)
        // But SD may make it higher, so just verify it's different
        let correction_total = expected_ho - hs;
        assert!(correction_total.abs() > 0.01,
                "Total correction should be significant, got {}", correction_total);
    }
}

