// SPDX-License-Identifier: GPL-2.0-only
// SPDX-FileCopyrightText: Alexander Atanasov <alex@zazolabs.com>
//! Twilight and celestial visibility screen
//!
//! This screen calculates twilight times (morning and evening) and shows
//! which stars and planets are visible for sextant observations at those times.

use chrono::{DateTime, Datelike, Duration, Local, NaiveDate, TimeZone, Utc};
use celtnav::{calculate_twilight_times, get_all_visible_bodies_interval, gha_aries, TwilightTimes, VisibleBody, decimal_to_dms};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Row, Table, Wrap},
    Frame,
};

/// Input field for the twilight screen
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TwilightInputField {
    Date,
    Latitude,
    LatitudeDirection,
    Longitude,
    LongitudeDirection,
    TimezoneOffset,
    DstActive,
}

impl TwilightInputField {
    pub fn all() -> Vec<TwilightInputField> {
        vec![
            TwilightInputField::Date,
            TwilightInputField::Latitude,
            TwilightInputField::LatitudeDirection,
            TwilightInputField::Longitude,
            TwilightInputField::LongitudeDirection,
            TwilightInputField::TimezoneOffset,
            TwilightInputField::DstActive,
        ]
    }

#[allow(dead_code)]    pub fn label(&self) -> &str {
        match self {
            TwilightInputField::Date => "Date (YYYY-MM-DD)",
            TwilightInputField::Latitude => "DR Latitude (DD MM.M)",
            TwilightInputField::LatitudeDirection => "N/S",
            TwilightInputField::Longitude => "DR Longitude (DDD MM.M)",
            TwilightInputField::LongitudeDirection => "E/W",
            TwilightInputField::TimezoneOffset => "Timezone Offset (hours from UTC, e.g., +2)",
            TwilightInputField::DstActive => "DST Active (Y/N)",
        }
    }
}

/// Which twilight time to display bodies for
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TwilightPeriod {
    Morning,
    Evening,
}

/// Twilight screen state
pub struct TwilightForm {
    /// Current input field
    pub current_field: TwilightInputField,

    /// Input values
    pub date: String,
    pub latitude: String,
    pub lat_direction: char,
    pub longitude: String,
    pub lon_direction: char,
    pub timezone_offset: String, // Standard timezone offset in hours (e.g., "+2" for UTC+2)
    pub dst_active: bool,         // Whether DST is currently active

    /// Computed twilight times
    pub twilight_times: Option<TwilightTimes>,

    /// Observation interval (start, end)
    pub observation_interval: Option<(DateTime<Utc>, DateTime<Utc>)>,

    /// Visible bodies during observation interval
    pub visible_bodies: Vec<VisibleBody>,

    /// Which twilight period to show bodies for
    pub selected_period: TwilightPeriod,

    /// Scroll offset for visible bodies list
    pub scroll_offset: usize,

    /// Star selection mode: false = our algorithm, true = Vol.1 15° LHA band
    pub band_mode: bool,

    /// Error message
    pub error_message: Option<String>,
}

impl TwilightForm {
    pub fn new() -> Self {
        // Default to today's date
        let now = Utc::now();
        let date_str = format!("{:04}-{:02}-{:02}", now.year(), now.month(), now.day());

        // Get system timezone offset
        let local_now = Local::now();
        let offset_seconds = local_now.offset().local_minus_utc();
        let offset_hours = offset_seconds as f64 / 3600.0;

        // Detect DST using heuristics
        // Fractional offsets (like +5.5 for India) are typically standard time
        // Whole hour offsets: odd hours (±3, ±5, ±7) are often DST from even standard (±2, ±4, ±6)
        let is_fractional = (offset_hours.fract().abs() > 0.1) && (offset_hours.fract().abs() < 0.9);

        let (standard_offset_hours, dst_active) = if is_fractional {
            // Fractional offset - likely a standard timezone (e.g., India +5.5, Iran +3.5)
            (offset_hours.round() as i32, false)
        } else {
            let whole_hours = offset_hours.round() as i32;

            // Heuristic: Odd hour offsets (±1, ±3, ±5, ±7, ±9, ±11) where abs > 1
            // are more likely to be DST from an even standard timezone
            // E.g., UTC+3 is likely EEST (UTC+2 + DST), not MSK
            // Exceptions: UTC+1 (CET), UTC+5 (PKT), UTC+9 (JST) can be standard
            // But for Eastern Europe (UTC+3), this is almost certainly DST from UTC+2
            let likely_dst = whole_hours.abs() > 1 && whole_hours % 2 != 0;

            if likely_dst {
                // Assume standard timezone is one hour less
                (whole_hours - whole_hours.signum(), true)
            } else {
                (whole_hours, false)
            }
        };

        let timezone_offset = if standard_offset_hours >= 0 {
            format!("+{}", standard_offset_hours)
        } else {
            format!("{}", standard_offset_hours)
        };

        Self {
            current_field: TwilightInputField::Date,
            date: date_str,
            latitude: String::new(),
            lat_direction: 'N',
            longitude: String::new(),
            lon_direction: 'E',
            timezone_offset,
            dst_active,
            twilight_times: None,
            observation_interval: None,
            visible_bodies: Vec::new(),
            selected_period: TwilightPeriod::Morning,
            scroll_offset: 0,
            band_mode: false,
            error_message: None,
        }
    }

    /// Parse latitude from input
    fn parse_latitude(&self) -> Result<f64, String> {
        if self.latitude.trim().is_empty() {
            return Err("Latitude is required".to_string());
        }

        let parts: Vec<&str> = self.latitude.split_whitespace().collect();

        if parts.len() != 2 {
            return Err("Latitude format: DD MM.M (e.g., '40 30.0')".to_string());
        }

        let degrees: i32 = parts[0].parse()
            .map_err(|_| "Invalid degrees".to_string())?;

        let minutes: f64 = parts[1].parse()
            .map_err(|_| "Invalid minutes".to_string())?;

        if !(0..=90).contains(&degrees) {
            return Err("Latitude degrees must be 0-90".to_string());
        }

        if !(0.0..60.0).contains(&minutes) {
            return Err("Minutes must be 0-60".to_string());
        }

        let mut lat = celtnav::dms_to_decimal(degrees, minutes as u32, 0.0);

        if self.lat_direction == 'S' {
            lat = -lat;
        }

        Ok(lat)
    }

    /// Parse longitude from input
    fn parse_longitude(&self) -> Result<f64, String> {
        if self.longitude.trim().is_empty() {
            return Err("Longitude is required".to_string());
        }

        let parts: Vec<&str> = self.longitude.split_whitespace().collect();

        if parts.len() != 2 {
            return Err("Longitude format: DDD MM.M (e.g., '074 00.6')".to_string());
        }

        let degrees: i32 = parts[0].parse()
            .map_err(|_| "Invalid degrees".to_string())?;

        let minutes: f64 = parts[1].parse()
            .map_err(|_| "Invalid minutes".to_string())?;

        if !(0..=180).contains(&degrees) {
            return Err("Longitude degrees must be 0-180".to_string());
        }

        if !(0.0..60.0).contains(&minutes) {
            return Err("Minutes must be 0-60".to_string());
        }

        let mut lon = celtnav::dms_to_decimal(degrees, minutes as u32, 0.0);

        if self.lon_direction == 'W' {
            lon = -lon;
        }

        Ok(lon)
    }

    /// Parse date from input
    fn parse_date(&self) -> Result<DateTime<Utc>, String> {
        let date = NaiveDate::parse_from_str(&self.date, "%Y-%m-%d")
            .map_err(|_| "Invalid date format (use YYYY-MM-DD)".to_string())?;

        let datetime = date.and_hms_opt(12, 0, 0)
            .ok_or_else(|| "Invalid datetime".to_string())?;

        Ok(Utc.from_utc_datetime(&datetime))
    }

    /// Calculate twilight times and visible bodies
    pub fn calculate(&mut self) {
        self.error_message = None;

        // Parse inputs
        let latitude = match self.parse_latitude() {
            Ok(lat) => lat,
            Err(e) => {
                self.error_message = Some(e);
                return;
            }
        };

        let longitude = match self.parse_longitude() {
            Ok(lon) => lon,
            Err(e) => {
                self.error_message = Some(e);
                return;
            }
        };

        let date = match self.parse_date() {
            Ok(d) => d,
            Err(e) => {
                self.error_message = Some(e);
                return;
            }
        };

        // Calculate twilight times
        let twilight = calculate_twilight_times(date, latitude, longitude);

        // Observation interval uses midpoints to bracket civil twilight:
        //   Evening: midpoint(sunset → civil) to midpoint(civil → nautical)
        //   Morning: midpoint(nautical → civil) to midpoint(civil → sunrise)
        // Falls back to civil ± 20 min when the outer event (sunset/nautical) is unavailable.
        fn midpoint(a: DateTime<Utc>, b: DateTime<Utc>) -> DateTime<Utc> {
            a + Duration::minutes((b - a).num_minutes() / 2)
        }

        let civil_time = match self.selected_period {
            TwilightPeriod::Morning => twilight.morning_civil,
            TwilightPeriod::Evening => twilight.evening_civil,
        };

        if let Some(civil) = civil_time {
            let (interval_start, interval_end) = match self.selected_period {
                TwilightPeriod::Evening => {
                    let start = twilight.sunset
                        .map(|ss| midpoint(ss, civil))
                        .unwrap_or_else(|| civil - Duration::minutes(20));
                    let end = twilight.evening_nautical
                        .map(|naut| midpoint(civil, naut))
                        .unwrap_or_else(|| civil + Duration::minutes(20));
                    (start, end)
                }
                TwilightPeriod::Morning => {
                    let start = twilight.morning_nautical
                        .map(|naut| midpoint(naut, civil))
                        .unwrap_or_else(|| civil - Duration::minutes(20));
                    let end = twilight.sunrise
                        .map(|sr| midpoint(civil, sr))
                        .unwrap_or_else(|| civil + Duration::minutes(20));
                    (start, end)
                }
            };

            self.observation_interval = Some((interval_start, interval_end));
            self.visible_bodies = get_all_visible_bodies_interval(
                interval_start,
                interval_end,
                latitude,
                longitude,
                matches!(self.selected_period, TwilightPeriod::Morning),
                self.band_mode,
            );
            self.scroll_offset = 0;
        } else {
            self.visible_bodies.clear();
            self.observation_interval = None;
            self.error_message = Some(format!(
                "{} civil twilight not found (polar regions may have perpetual day/night)",
                match self.selected_period {
                    TwilightPeriod::Morning => "Morning",
                    TwilightPeriod::Evening => "Evening",
                }
            ));
        }

        self.twilight_times = Some(twilight);
    }

    /// Toggle between morning and evening twilight
    pub fn toggle_period(&mut self) {
        self.selected_period = match self.selected_period {
            TwilightPeriod::Morning => TwilightPeriod::Evening,
            TwilightPeriod::Evening => TwilightPeriod::Morning,
        };

        self.scroll_offset = 0; // Reset scroll when changing period

        // Recalculate visible bodies for new period
        if self.twilight_times.is_some() {
            self.calculate();
        }
    }

    /// Toggle between our algorithm and Vol.1 15° LHA band mode
    pub fn toggle_band_mode(&mut self) {
        self.band_mode = !self.band_mode;
        self.scroll_offset = 0;
        if self.twilight_times.is_some() {
            self.calculate();
        }
    }

    /// Toggle latitude direction
    pub fn toggle_lat_direction(&mut self) {
        self.lat_direction = if self.lat_direction == 'N' { 'S' } else { 'N' };
    }

    /// Toggle longitude direction
    pub fn toggle_lon_direction(&mut self) {
        self.lon_direction = if self.lon_direction == 'W' { 'E' } else { 'W' };
    }

    /// Handle input for current field
    pub fn handle_input(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Tab => {
                let fields = TwilightInputField::all();
                let current_idx = fields.iter().position(|&f| f == self.current_field).unwrap();
                let next_idx = (current_idx + 1) % fields.len();
                self.current_field = fields[next_idx];
            }
            KeyCode::BackTab => {
                let fields = TwilightInputField::all();
                let current_idx = fields.iter().position(|&f| f == self.current_field).unwrap();
                let prev_idx = if current_idx == 0 { fields.len() - 1 } else { current_idx - 1 };
                self.current_field = fields[prev_idx];
            }
            KeyCode::Char(c) => {
                match self.current_field {
                    TwilightInputField::LatitudeDirection => {
                        if c == 'N' || c == 'n' || c == 'S' || c == 's' {
                            self.lat_direction = c.to_uppercase().next().unwrap();
                        }
                    }
                    TwilightInputField::LongitudeDirection => {
                        if c == 'E' || c == 'e' || c == 'W' || c == 'w' {
                            self.lon_direction = c.to_uppercase().next().unwrap();
                        }
                    }
                    TwilightInputField::DstActive => {
                        if c == 'Y' || c == 'y' {
                            self.dst_active = true;
                        } else if c == 'N' || c == 'n' {
                            self.dst_active = false;
                        }
                    }
                    TwilightInputField::Date => {
                        self.date.push(c);
                    }
                    TwilightInputField::Latitude => {
                        self.latitude.push(c);
                    }
                    TwilightInputField::Longitude => {
                        self.longitude.push(c);
                    }
                    TwilightInputField::TimezoneOffset => {
                        if c.is_numeric() || c == '+' || c == '-' || c == '.' {
                            self.timezone_offset.push(c);
                        }
                    }
                }
            }
            KeyCode::Backspace => {
                match self.current_field {
                    TwilightInputField::Date => { self.date.pop(); }
                    TwilightInputField::Latitude => { self.latitude.pop(); }
                    TwilightInputField::Longitude => { self.longitude.pop(); }
                    TwilightInputField::TimezoneOffset => { self.timezone_offset.pop(); }
                    _ => {}
                }
            }
            KeyCode::Enter => {
                self.calculate();
            }
            KeyCode::Left | KeyCode::Right => {
                // Toggle direction fields with arrow keys
                match self.current_field {
                    TwilightInputField::LatitudeDirection => self.toggle_lat_direction(),
                    TwilightInputField::LongitudeDirection => self.toggle_lon_direction(),
                    TwilightInputField::DstActive => self.dst_active = !self.dst_active,
                    _ => {}
                }
            }
            KeyCode::Up => {
                // Scroll up in visible bodies list
                if self.scroll_offset > 0 {
                    self.scroll_offset -= 1;
                }
            }
            KeyCode::Down => {
                // Scroll down in visible bodies list
                if !self.visible_bodies.is_empty() && self.scroll_offset < self.visible_bodies.len() - 1 {
                    self.scroll_offset += 1;
                }
            }
            _ => {}
        }
    }
}

impl Default for TwilightForm {
    fn default() -> Self {
        Self::new()
    }
}

/// Render the twilight screen
pub fn render(frame: &mut Frame, area: Rect, form: &TwilightForm) {
    // Split the screen
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(7),  // Input area (3 input lines + borders)
            Constraint::Length(7),  // Twilight times area (compact)
            Constraint::Min(10),    // Visible bodies area
            Constraint::Length(3),  // Help
        ])
        .split(area);

    // Render input area
    render_input_area(frame, chunks[0], form);

    // Render twilight times
    render_twilight_times(frame, chunks[1], form);

    // Render visible bodies
    render_visible_bodies(frame, chunks[2], form);

    // Render help
    render_help(frame, chunks[3], form);
}

/// Render input area
fn render_input_area(frame: &mut Frame, area: Rect, form: &TwilightForm) {
    let input_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Date line
            Constraint::Length(1), // Lat/Lon line
            Constraint::Length(1), // Timezone/DST line
        ])
        .margin(1)
        .split(area);

    // Date field
    render_input_field(
        frame,
        input_chunks[0],
        "Date",
        &form.date,
        form.current_field == TwilightInputField::Date,
    );

    // Latitude and Longitude on one line
    let position_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(35), // Latitude value
            Constraint::Percentage(8),  // N/S
            Constraint::Percentage(2),  // Spacer
            Constraint::Percentage(35), // Longitude value
            Constraint::Percentage(8),  // E/W
            Constraint::Percentage(12), // Extra space
        ])
        .split(input_chunks[1]);

    render_input_field(
        frame,
        position_chunks[0],
        "Lat",
        &form.latitude,
        form.current_field == TwilightInputField::Latitude,
    );

    render_input_field(
        frame,
        position_chunks[1],
        "",
        &form.lat_direction.to_string(),
        form.current_field == TwilightInputField::LatitudeDirection,
    );

    render_input_field(
        frame,
        position_chunks[3],
        "Lon",
        &form.longitude,
        form.current_field == TwilightInputField::Longitude,
    );

    render_input_field(
        frame,
        position_chunks[4],
        "",
        &form.lon_direction.to_string(),
        form.current_field == TwilightInputField::LongitudeDirection,
    );

    // Timezone and DST on one line
    let timezone_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(25), // Timezone label + value
            Constraint::Percentage(2),  // Spacer
            Constraint::Percentage(15), // DST label + value
            Constraint::Percentage(58), // Extra space
        ])
        .split(input_chunks[2]);

    render_input_field(
        frame,
        timezone_chunks[0],
        "TZ",
        &form.timezone_offset,
        form.current_field == TwilightInputField::TimezoneOffset,
    );

    let dst_text = if form.dst_active { "Y" } else { "N" };
    render_input_field(
        frame,
        timezone_chunks[2],
        "DST",
        dst_text,
        form.current_field == TwilightInputField::DstActive,
    );

    // Draw border
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" DR Position & Date ")
        .border_style(Style::default().fg(Color::Green));
    frame.render_widget(block, area);
}

/// Render individual input field
fn render_input_field(frame: &mut Frame, area: Rect, label: &str, value: &str, is_current: bool) {
    let style = if is_current {
        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::White)
    };

    let text = format!("{}: {}", label, value);
    let paragraph = Paragraph::new(text)
        .style(style);

    frame.render_widget(paragraph, area);
}

/// Render twilight times
fn render_twilight_times(frame: &mut Frame, area: Rect, form: &TwilightForm) {
    let mut lines = Vec::new();

    if let Some(ref times) = form.twilight_times {
        // Determine which column to highlight based on selected period
        let morning_style = if form.selected_period == TwilightPeriod::Morning {
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };

        let evening_style = if form.selected_period == TwilightPeriod::Evening {
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };

        // Get longitude for LMT calculation
        let longitude = form.parse_longitude().unwrap_or(0.0);

        // Parse timezone offset if provided
        let timezone_offset: Option<f64> = form.timezone_offset.parse().ok();

        // Format time as UTC/LMT/LT (in that order)
        let format_time = |time: Option<DateTime<Utc>>| {
            if let Some(utc_time) = time {
                // Calculate LMT offset: longitude / 15 = hours offset
                let lmt_offset_hours = longitude / 15.0;
                let lmt_offset_minutes = (lmt_offset_hours * 60.0) as i64;
                let lmt_time = utc_time + Duration::minutes(lmt_offset_minutes);

                // If timezone is configured, also show local time
                if let Some(tz_offset) = timezone_offset {
                    // Calculate local standard time
                    let standard_offset_hours = tz_offset;
                    let mut total_offset_hours = standard_offset_hours;

                    // Add DST offset if active (typically +1 hour)
                    if form.dst_active {
                        total_offset_hours += 1.0;
                    }

                    let local_offset_minutes = (total_offset_hours * 60.0) as i64;
                    let local_time = utc_time + Duration::minutes(local_offset_minutes);

                    // Format: UTC/LMT/LT (reordered as requested)
                    format!("{} {} {}",
                        utc_time.format("%H:%M"),
                        lmt_time.format("%H:%M"),
                        local_time.format("%H:%M")
                    )
                } else {
                    // No timezone configured: UTC  LMT
                    format!("{}  {}",
                        utc_time.format("%H:%M"),
                        lmt_time.format("%H:%M")
                    )
                }
            } else {
                "N/A      ".to_string()
            }
        };

        // Add header line showing time format labels (aligned with row labels)
        let time_header = if timezone_offset.is_some() {
            if form.dst_active {
                "Period:   UTC   LMT   LT(DS)| UTC   LMT   LT(DS)"
            } else {
                "Period:   UTC   LMT   LT    | UTC   LMT   LT"
            }
        } else {
            "Period:   UTC  LMT           | UTC  LMT"
        };
        lines.push(Line::from(vec![
            Span::styled(time_header, Style::default().fg(Color::DarkGray)),
        ]));

        // Line 1: Nautical Twilight
        lines.push(Line::from(vec![
            Span::styled("Nautical: ", Style::default().fg(Color::Cyan)),
            Span::styled(format_time(times.morning_nautical), morning_style),
            Span::styled(" | ", Style::default().fg(Color::DarkGray)),
            Span::styled(format_time(times.evening_nautical), evening_style),
        ]));

        // Line 2: Civil Twilight
        lines.push(Line::from(vec![
            Span::styled("Civil:    ", Style::default().fg(Color::Cyan)),
            Span::styled(format_time(times.morning_civil), morning_style),
            Span::styled(" | ", Style::default().fg(Color::DarkGray)),
            Span::styled(format_time(times.evening_civil), evening_style),
        ]));

        // Line 3: Sunrise/Sunset
        lines.push(Line::from(vec![
            Span::styled("Sun:      ", Style::default().fg(Color::Cyan)),
            Span::styled(format_time(times.sunrise), morning_style),
            Span::styled(" | ", Style::default().fg(Color::DarkGray)),
            Span::styled(format_time(times.sunset), evening_style),
        ]));

        // Line 4: Observation Interval + LHA Aries
        if let Some((start, end)) = form.observation_interval {
            let mid_time = start + Duration::minutes((end - start).num_minutes() / 2);
            let longitude = form.parse_longitude().unwrap_or(0.0);
            let gha = gha_aries(mid_time);
            let lha_raw = (gha + longitude).rem_euclid(360.0);
            let lha_whole = (lha_raw.round() as u32) % 360;
            let interval_display = format!(
                "Obs. Interval: {} to {} UTC  |  LHA♈ {:.1}°  (whole {}°)",
                start.format("%H:%M"),
                end.format("%H:%M"),
                lha_raw,
                lha_whole,
            );
            lines.push(Line::from(vec![
                Span::styled(interval_display, Style::default().fg(Color::Yellow)),
            ]));
        }

        // Line 5: time format note (compact, no blank line)
        let time_format_help = if timezone_offset.is_some() {
            if form.dst_active { "UTC/LMT/LT(DST)" } else { "UTC/LMT/LT" }
        } else {
            "UTC/LMT"
        };
        lines.push(Line::from(vec![
            Span::styled(format!("T: toggle morning/evening  |  times: {}", time_format_help), Style::default().fg(Color::DarkGray)),
        ]));
    } else if let Some(ref error) = form.error_message {
        lines.push(Line::from(vec![
            Span::styled("ERROR: ", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
            Span::styled(error, Style::default().fg(Color::Red)),
        ]));
    } else {
        lines.push(Line::from(vec![
            Span::styled("Press Enter to calculate twilight times", Style::default().fg(Color::Yellow)),
        ]));
    }

    let paragraph = Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL).title(" Twilight Times (Morning | Evening) "))
        .wrap(Wrap { trim: true });

    frame.render_widget(paragraph, area);
}

/// Render visible bodies
fn render_visible_bodies(frame: &mut Frame, area: Rect, form: &TwilightForm) {
    if form.visible_bodies.is_empty() {
        let msg = if form.twilight_times.is_some() {
            "No bodies visible in sextant range (15° - 75°)"
        } else {
            "Calculate twilight times to see visible bodies"
        };

        let paragraph = Paragraph::new(msg)
            .block(Block::default().borders(Borders::ALL).title(" Visible Bodies "))
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::DarkGray));

        frame.render_widget(paragraph, area);
        return;
    }

    // Calculate how many rows we can display (area height - borders(2) - header(2))
    let available_height = area.height.saturating_sub(4) as usize;

    // Calculate visible window
    let total_bodies = form.visible_bodies.len();
    let start_idx = form.scroll_offset;
    let end_idx = (start_idx + available_height).min(total_bodies);

    // Create table header
    let header = Row::new(vec!["Name", "SHA", "Mag", "Altitude", "Azimuth"])
        .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
        .bottom_margin(1);

    // Create rows only for visible bodies
    let rows: Vec<Row> = form.visible_bodies[start_idx..end_idx].iter().map(|body| {
        let alt_dms = decimal_to_dms(body.altitude);
        let alt_str = format!("{}° {:.1}'", alt_dms.degrees, alt_dms.minutes);
        let zn_str = format!("{:.0}°", body.azimuth);

        // Format SHA (Sidereal Hour Angle)
        let sha_str = if let Some(sha) = body.sha {
            let sha_dms = decimal_to_dms(sha);
            format!("{}°{:.0}'", sha_dms.degrees, sha_dms.minutes)
        } else {
            "-".to_string()
        };

        // Format magnitude
        let mag_str = if let Some(mag) = body.magnitude {
            format!("{:.1}", mag)
        } else {
            "-".to_string()
        };

        // First-magnitude stars (mag ≤ 1.5) shown in CAPS, matching Pub 249 Vol.1 convention.
        // ◆ marker only for the best 3 recommended stars.
        let is_first_magnitude = body.magnitude.is_some_and(|m| m <= 1.5);
        let display_name = if is_first_magnitude {
            body.name.to_uppercase()
        } else {
            body.name.clone()
        };
        let name_with_marker = if body.is_recommended {
            format!("◆ {}", display_name)
        } else if body.is_second_best {
            format!("+ {}", display_name)
        } else {
            display_name
        };

        // Style recommended stars in green, second-best in cyan
        let row = Row::new(vec![
            name_with_marker,
            sha_str,
            mag_str,
            alt_str,
            zn_str,
        ]);

        if body.is_recommended {
            row.style(Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))
        } else if body.is_second_best {
            row.style(Style::default().fg(Color::Cyan))
        } else {
            row
        }
    }).collect();

    // Build title with observation period and scroll indicators
    let scroll_indicator = if total_bodies > available_height {
        format!(" [{}-{}/{}] ", start_idx + 1, end_idx, total_bodies)
    } else {
        String::new()
    };

    let period_str = match form.selected_period {
        TwilightPeriod::Morning => "Morning",
        TwilightPeriod::Evening => "Evening",
    };

    let title = if let Some((start, end)) = form.observation_interval {
        // Show observation period in local time (LT if timezone set, else LMT)
        let longitude = form.parse_longitude().unwrap_or(0.0);
        let timezone_offset: Option<f64> = form.timezone_offset.parse().ok();
        let (local_start, local_end, time_label) = if let Some(tz_offset) = timezone_offset {
            let mut total_offset_hours = tz_offset;
            if form.dst_active { total_offset_hours += 1.0; }
            let offset_mins = (total_offset_hours * 60.0) as i64;
            let label = if form.dst_active { "LT(DST)" } else { "LT" };
            (start + Duration::minutes(offset_mins), end + Duration::minutes(offset_mins), label)
        } else {
            let lmt_mins = (longitude / 15.0 * 60.0) as i64;
            (start + Duration::minutes(lmt_mins), end + Duration::minutes(lmt_mins), "LMT")
        };
        // Mode label: show band LHA when in Vol.1 band mode
        let mode_label = if form.band_mode {
            let mid = start + Duration::minutes((end - start).num_minutes() / 2);
            let gha = gha_aries(mid);
            let mid_lha = (gha + longitude).rem_euclid(360.0);
            let band_start = ((mid_lha / 15.0).floor() * 15.0).rem_euclid(360.0) as u32;
            let band_end   = (band_start + 14) % 360;
            format!(" [Band {}°-{}°]", band_start, band_end)
        } else {
            " [Optimal]".to_string()
        };
        format!(
            " {} Twilight Bodies - {} to {} {}{} ({} bodies){} ",
            period_str,
            local_start.format("%H:%M"),
            local_end.format("%H:%M"),
            time_label,
            mode_label,
            total_bodies,
            scroll_indicator
        )
    } else {
        format!(
            " Visible Bodies at {} Twilight ({} bodies){} ",
            period_str,
            total_bodies,
            scroll_indicator
        )
    };

    let table = Table::new(
        rows,
        [
            Constraint::Percentage(25), // Name
            Constraint::Percentage(15), // SHA
            Constraint::Percentage(10), // Mag
            Constraint::Percentage(25), // Altitude
            Constraint::Percentage(25), // Azimuth
        ]
    )
        .header(header)
        .block(Block::default().borders(Borders::ALL).title(title))
        .column_spacing(1);

    frame.render_widget(table, area);
}

/// Update help text to explain star markers
fn _explain_star_markers() -> &'static str {
    "◆ = Recommended for 3-star fix (good LOP crossing)"
}

/// Render help text
fn render_help(frame: &mut Frame, area: Rect, _form: &TwilightForm) {
    let help_text = "Tab: Next | Enter: Calculate | T: Toggle morning/evening | B: Band/Optimal | ↑↓: Scroll | ◆=Best 3  +=2nd | Q: Quit";

    let paragraph = Paragraph::new(help_text)
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::Cyan));

    frame.render_widget(paragraph, area);
}
