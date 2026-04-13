// SPDX-License-Identifier: GPL-2.0-only
// SPDX-FileCopyrightText: Alexander Atanasov <alex@zazolabs.com>
//! Dead Reckoning (DR) and Estimated Position (EP) screen
//!
//! This screen manages DR/EP positions, calculates running fixes,
//! and computes estimated positions from DR using set/drift or course/speed.

use celtnav::advance_position;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};
use serde::{Deserialize, Serialize};
use crate::persistence::{load_from_file, save_to_file};

const DR_EP_FILE: &str = "dr_ep.json";

/// Persistent DR/EP position data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DrEpData {
    /// DR latitude in decimal degrees (positive = North)
    pub dr_lat: f64,
    /// DR longitude in decimal degrees (positive = East)
    pub dr_lon: f64,
    /// EP latitude in decimal degrees (positive = North)
    pub ep_lat: f64,
    /// EP longitude in decimal degrees (positive = East)
    pub ep_lon: f64,
    /// Whether EP has been calculated
    pub ep_valid: bool,
}

impl Default for DrEpData {
    fn default() -> Self {
        Self {
            dr_lat: 0.0,
            dr_lon: 0.0,
            ep_lat: 0.0,
            ep_lon: 0.0,
            ep_valid: false,
        }
    }
}

impl DrEpData {
    pub fn load() -> Self {
        load_from_file::<DrEpData>(DR_EP_FILE).unwrap_or_default()
    }

    pub fn save(&self) {
        let _ = save_to_file(self, DR_EP_FILE);
    }

    /// Format a lat/lon pair as "DD°MM.M' N/S DDD°MM.M' E/W"
    pub fn format_position(lat: f64, lon: f64) -> String {
        let lat_dir = if lat >= 0.0 { 'N' } else { 'S' };
        let lon_dir = if lon >= 0.0 { 'E' } else { 'W' };
        let lat_abs = lat.abs();
        let lon_abs = lon.abs();
        let lat_deg = lat_abs.floor() as u32;
        let lat_min = (lat_abs - lat_deg as f64) * 60.0;
        let lon_deg = lon_abs.floor() as u32;
        let lon_min = (lon_abs - lon_deg as f64) * 60.0;
        format!(
            "{:02}°{:04.1}' {} {:03}°{:04.1}' {}",
            lat_deg, lat_min, lat_dir, lon_deg, lon_min, lon_dir
        )
    }
}

/// Which section/mode is active
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DrEpSection {
    DrPosition,
    RunningFix,
    EpCalculation,
}

/// Input fields in DR position section
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DrField {
    Latitude,
    LatDir,
    Longitude,
    LonDir,
}

/// Input fields in Running Fix section
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunFixField {
    Course,
    Speed,
    Hours,
    Minutes,
    LogDistance,
    UseTime, // toggle: use time vs log distance
}

/// Input fields in EP Calculation section
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EpField {
    SetDirection,
    DriftSpeed,
    Hours,
    Minutes,
    LogDistance,
    UseTime,
}

/// DR/EP screen state
pub struct DrEpForm {
    /// Current active section
    pub section: DrEpSection,

    // DR position fields
    pub dr_lat: String,
    pub dr_lat_dir: char,
    pub dr_lon: String,
    pub dr_lon_dir: char,
    pub dr_field: DrField,

    // Running fix fields
    pub rf_course: String,
    pub rf_speed: String,
    pub rf_hours: String,
    pub rf_minutes: String,
    pub rf_log_dist: String,
    pub rf_use_time: bool,
    pub rf_field: RunFixField,

    // EP calculation fields
    pub ep_set: String,   // set direction (degrees true)
    pub ep_drift: String, // drift speed (knots)
    pub ep_hours: String,
    pub ep_minutes: String,
    pub ep_log_dist: String,
    pub ep_use_time: bool,
    pub ep_field: EpField,

    // Results
    pub computed_dr: Option<(f64, f64)>,
    pub computed_ep: Option<(f64, f64)>,

    // Messages
    pub message: Option<String>,
    pub error: Option<String>,

    // Set to true when DR is saved/updated so App can propagate to other screens
    pub dr_updated: bool,

    // Stored persistent data
    pub stored: DrEpData,
}

impl DrEpForm {
    pub fn new() -> Self {
        let stored = DrEpData::load();

        let (dr_lat_str, dr_lat_dir, dr_lon_str, dr_lon_dir) =
            Self::decode_position(stored.dr_lat, stored.dr_lon);

        Self {
            section: DrEpSection::DrPosition,
            dr_lat: dr_lat_str,
            dr_lat_dir,
            dr_lon: dr_lon_str,
            dr_lon_dir,
            dr_field: DrField::Latitude,

            rf_course: String::new(),
            rf_speed: String::new(),
            rf_hours: String::new(),
            rf_minutes: String::new(),
            rf_log_dist: String::new(),
            rf_use_time: true,
            rf_field: RunFixField::Course,

            ep_set: String::new(),
            ep_drift: String::new(),
            ep_hours: String::new(),
            ep_minutes: String::new(),
            ep_log_dist: String::new(),
            ep_use_time: true,
            ep_field: EpField::SetDirection,

            computed_dr: None,
            computed_ep: if stored.ep_valid {
                Some((stored.ep_lat, stored.ep_lon))
            } else {
                None
            },

            message: None,
            error: None,
            dr_updated: false,
            stored,
        }
    }

    pub fn decode_position(lat: f64, lon: f64) -> (String, char, String, char) {
        let lat_dir = if lat >= 0.0 { 'N' } else { 'S' };
        let lon_dir = if lon >= 0.0 { 'E' } else { 'W' };
        let lat_abs = lat.abs();
        let lon_abs = lon.abs();
        let lat_deg = lat_abs.floor() as u32;
        let lat_min = (lat_abs - lat_deg as f64) * 60.0;
        let lon_deg = lon_abs.floor() as u32;
        let lon_min = (lon_abs - lon_deg as f64) * 60.0;
        (
            format!("{:02} {:04.1}", lat_deg, lat_min),
            lat_dir,
            format!("{:03} {:04.1}", lon_deg, lon_min),
            lon_dir,
        )
    }

    /// Get DR position as decimal degrees (returns None if fields are empty/invalid)
    pub fn get_dr_decimal(&self) -> Option<(f64, f64)> {
        let lat = parse_dm(&self.dr_lat)?;
        let lon = parse_dm(&self.dr_lon)?;
        let lat = if self.dr_lat_dir == 'S' { -lat } else { lat };
        let lon = if self.dr_lon_dir == 'W' { -lon } else { lon };
        Some((lat, lon))
    }

    /// Save current DR position to persistent store
    pub fn save_dr(&mut self) {
        if let Some((lat, lon)) = self.get_dr_decimal() {
            self.stored.dr_lat = lat;
            self.stored.dr_lon = lon;
            self.stored.save();
            self.dr_updated = true;
            self.message = Some(format!(
                "DR saved: {}",
                DrEpData::format_position(lat, lon)
            ));
            self.error = None;
        } else {
            self.error = Some("Invalid DR position format. Use DD MM.M".to_string());
            self.message = None;
        }
    }

    /// Calculate running fix: advance DR by course/speed/time or log distance
    pub fn calculate_running_fix(&mut self) {
        let (dr_lat, dr_lon) = match self.get_dr_decimal() {
            Some(pos) => pos,
            None => {
                self.error = Some("Invalid DR position".to_string());
                return;
            }
        };

        let course: f64 = match self.rf_course.trim().parse() {
            Ok(v) => v,
            Err(_) => {
                self.error = Some("Invalid course (use decimal degrees true)".to_string());
                return;
            }
        };

        let distance_nm = if self.rf_use_time {
            let speed: f64 = match self.rf_speed.trim().parse() {
                Ok(v) => v,
                Err(_) => {
                    self.error = Some("Invalid speed (knots)".to_string());
                    return;
                }
            };
            let h: f64 = self.rf_hours.trim().parse().unwrap_or(0.0);
            let m: f64 = self.rf_minutes.trim().parse().unwrap_or(0.0);
            let time_h = h + m / 60.0;
            if time_h <= 0.0 {
                self.error = Some("Time must be > 0".to_string());
                return;
            }
            speed * time_h
        } else {
            match self.rf_log_dist.trim().parse::<f64>() {
                Ok(v) => v,
                Err(_) => {
                    self.error = Some("Invalid log distance (NM)".to_string());
                    return;
                }
            }
        };

        let (new_lat, new_lon) = advance_position(dr_lat, dr_lon, course, distance_nm, 1.0);

        self.computed_dr = Some((new_lat, new_lon));
        self.stored.dr_lat = new_lat;
        self.stored.dr_lon = new_lon;
        self.stored.save();
        self.dr_updated = true;

        // Update DR fields
        let (lat_str, lat_dir, lon_str, lon_dir) =
            Self::decode_position(new_lat, new_lon);
        self.dr_lat = lat_str;
        self.dr_lat_dir = lat_dir;
        self.dr_lon = lon_str;
        self.dr_lon_dir = lon_dir;

        self.message = Some(format!(
            "DR advanced {:.1} NM → {}",
            distance_nm,
            DrEpData::format_position(new_lat, new_lon)
        ));
        self.error = None;
    }

    /// Calculate EP from DR + set/drift
    pub fn calculate_ep(&mut self) {
        let (dr_lat, dr_lon) = match self.get_dr_decimal() {
            Some(pos) => pos,
            None => {
                self.error = Some("Invalid DR position".to_string());
                return;
            }
        };

        let set: f64 = match self.ep_set.trim().parse() {
            Ok(v) => v,
            Err(_) => {
                self.error = Some("Invalid set direction (degrees true)".to_string());
                return;
            }
        };

        let drift: f64 = match self.ep_drift.trim().parse() {
            Ok(v) => v,
            Err(_) => {
                self.error = Some("Invalid drift speed (knots)".to_string());
                return;
            }
        };

        let distance_nm = if self.ep_use_time {
            let h: f64 = self.ep_hours.trim().parse().unwrap_or(0.0);
            let m: f64 = self.ep_minutes.trim().parse().unwrap_or(0.0);
            let time_h = h + m / 60.0;
            if time_h <= 0.0 {
                self.error = Some("Time must be > 0".to_string());
                return;
            }
            drift * time_h
        } else {
            match self.ep_log_dist.trim().parse::<f64>() {
                Ok(v) => v,
                Err(_) => {
                    self.error = Some("Invalid log distance (NM)".to_string());
                    return;
                }
            }
        };

        let (ep_lat, ep_lon) = advance_position(dr_lat, dr_lon, set, distance_nm, 1.0);

        self.computed_ep = Some((ep_lat, ep_lon));
        self.stored.ep_lat = ep_lat;
        self.stored.ep_lon = ep_lon;
        self.stored.ep_valid = true;
        self.stored.save();

        self.message = Some(format!(
            "EP: {}",
            DrEpData::format_position(ep_lat, ep_lon)
        ));
        self.error = None;
    }

    /// Handle keyboard input
    pub fn handle_key_event(&mut self, key: KeyEvent) {
        self.message = None;
        self.error = None;
        self.dr_updated = false;

        match key.code {
            KeyCode::Tab => self.next_field(),
            KeyCode::BackTab => self.prev_field(),
            KeyCode::Enter => self.activate_current(),
            KeyCode::F(1) => self.section = DrEpSection::DrPosition,
            KeyCode::F(2) => self.section = DrEpSection::RunningFix,
            KeyCode::F(3) => self.section = DrEpSection::EpCalculation,
            _ => self.type_into_field(key),
        }
    }

    fn next_field(&mut self) {
        match self.section {
            DrEpSection::DrPosition => {
                self.dr_field = match self.dr_field {
                    DrField::Latitude => DrField::LatDir,
                    DrField::LatDir => DrField::Longitude,
                    DrField::Longitude => DrField::LonDir,
                    DrField::LonDir => DrField::Latitude,
                };
            }
            DrEpSection::RunningFix => {
                self.rf_field = match self.rf_field {
                    RunFixField::UseTime => RunFixField::Course,
                    RunFixField::Course => RunFixField::Speed,
                    RunFixField::Speed => {
                        if self.rf_use_time {
                            RunFixField::Hours
                        } else {
                            RunFixField::LogDistance
                        }
                    }
                    RunFixField::Hours => RunFixField::Minutes,
                    RunFixField::Minutes => RunFixField::UseTime,
                    RunFixField::LogDistance => RunFixField::UseTime,
                };
            }
            DrEpSection::EpCalculation => {
                self.ep_field = match self.ep_field {
                    EpField::UseTime => EpField::SetDirection,
                    EpField::SetDirection => EpField::DriftSpeed,
                    EpField::DriftSpeed => {
                        if self.ep_use_time {
                            EpField::Hours
                        } else {
                            EpField::LogDistance
                        }
                    }
                    EpField::Hours => EpField::Minutes,
                    EpField::Minutes => EpField::UseTime,
                    EpField::LogDistance => EpField::UseTime,
                };
            }
        }
    }

    fn prev_field(&mut self) {
        match self.section {
            DrEpSection::DrPosition => {
                self.dr_field = match self.dr_field {
                    DrField::Latitude => DrField::LonDir,
                    DrField::LatDir => DrField::Latitude,
                    DrField::Longitude => DrField::LatDir,
                    DrField::LonDir => DrField::Longitude,
                };
            }
            DrEpSection::RunningFix => {
                self.rf_field = match self.rf_field {
                    RunFixField::Course => RunFixField::UseTime,
                    RunFixField::Speed => RunFixField::Course,
                    RunFixField::Hours => RunFixField::Speed,
                    RunFixField::Minutes => RunFixField::Hours,
                    RunFixField::LogDistance => RunFixField::Speed,
                    RunFixField::UseTime => {
                        if self.rf_use_time { RunFixField::Minutes } else { RunFixField::LogDistance }
                    }
                };
            }
            DrEpSection::EpCalculation => {
                self.ep_field = match self.ep_field {
                    EpField::SetDirection => EpField::UseTime,
                    EpField::DriftSpeed => EpField::SetDirection,
                    EpField::Hours => EpField::DriftSpeed,
                    EpField::Minutes => EpField::Hours,
                    EpField::LogDistance => EpField::DriftSpeed,
                    EpField::UseTime => {
                        if self.ep_use_time { EpField::Minutes } else { EpField::LogDistance }
                    }
                };
            }
        }
    }

    fn activate_current(&mut self) {
        match self.section {
            DrEpSection::DrPosition => self.save_dr(),
            DrEpSection::RunningFix => self.calculate_running_fix(),
            DrEpSection::EpCalculation => self.calculate_ep(),
        }
    }

    fn type_into_field(&mut self, key: KeyEvent) {
        let ch = match key.code {
            KeyCode::Char(c) => Some(c),
            KeyCode::Backspace => None,
            _ => return,
        };

        match self.section {
            DrEpSection::DrPosition => match self.dr_field {
                DrField::Latitude => edit_string(&mut self.dr_lat, ch),
                DrField::LatDir => {
                    if let Some(c) = ch {
                        if c == 'n' || c == 'N' { self.dr_lat_dir = 'N'; }
                        if c == 's' || c == 'S' { self.dr_lat_dir = 'S'; }
                    }
                }
                DrField::Longitude => edit_string(&mut self.dr_lon, ch),
                DrField::LonDir => {
                    if let Some(c) = ch {
                        if c == 'e' || c == 'E' { self.dr_lon_dir = 'E'; }
                        if c == 'w' || c == 'W' { self.dr_lon_dir = 'W'; }
                    }
                }
            },
            DrEpSection::RunningFix => match self.rf_field {
                RunFixField::Course => edit_string(&mut self.rf_course, ch),
                RunFixField::Speed => edit_string(&mut self.rf_speed, ch),
                RunFixField::Hours => edit_string(&mut self.rf_hours, ch),
                RunFixField::Minutes => edit_string(&mut self.rf_minutes, ch),
                RunFixField::LogDistance => edit_string(&mut self.rf_log_dist, ch),
                RunFixField::UseTime => {
                    if let Some(c) = ch {
                        if c == 't' || c == 'T' { self.rf_use_time = true; }
                        if c == 'l' || c == 'L' { self.rf_use_time = false; }
                        if c == ' ' { self.rf_use_time = !self.rf_use_time; }
                    }
                }
            },
            DrEpSection::EpCalculation => match self.ep_field {
                EpField::SetDirection => edit_string(&mut self.ep_set, ch),
                EpField::DriftSpeed => edit_string(&mut self.ep_drift, ch),
                EpField::Hours => edit_string(&mut self.ep_hours, ch),
                EpField::Minutes => edit_string(&mut self.ep_minutes, ch),
                EpField::LogDistance => edit_string(&mut self.ep_log_dist, ch),
                EpField::UseTime => {
                    if let Some(c) = ch {
                        if c == 't' || c == 'T' { self.ep_use_time = true; }
                        if c == 'l' || c == 'L' { self.ep_use_time = false; }
                        if c == ' ' { self.ep_use_time = !self.ep_use_time; }
                    }
                }
            },
        }
    }
}

fn edit_string(s: &mut String, ch: Option<char>) {
    match ch {
        Some(c) => s.push(c),
        None => { s.pop(); }
    }
}

fn parse_dm(s: &str) -> Option<f64> {
    let s = s.trim();
    if s.is_empty() {
        return Some(0.0);
    }
    // Try "DD MM.M" format
    let parts: Vec<&str> = s.splitn(2, ' ').collect();
    if parts.len() == 2 {
        let deg: f64 = parts[0].parse().ok()?;
        let min: f64 = parts[1].parse().ok()?;
        return Some(deg + min / 60.0);
    }
    // Fallback: decimal degrees
    s.parse().ok()
}

/// Render the DR/EP screen
pub fn render(frame: &mut Frame, area: Rect, form: &DrEpForm) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(8),  // DR position
            Constraint::Length(9),  // Running fix
            Constraint::Length(9),  // EP calculation
            Constraint::Min(3),     // Results / message
        ])
        .split(area);

    render_dr_section(frame, chunks[0], form);
    render_running_fix_section(frame, chunks[1], form);
    render_ep_section(frame, chunks[2], form);
    render_results_section(frame, chunks[3], form);
}

fn render_dr_section(frame: &mut Frame, area: Rect, form: &DrEpForm) {
    let active = form.section == DrEpSection::DrPosition;
    let border_style = if active {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let stored = &form.stored;
    let stored_pos = DrEpData::format_position(stored.dr_lat, stored.dr_lon);

    let lat_style = |f: DrField| field_style(active && form.dr_field == f);
    let lon_style = |f: DrField| field_style(active && form.dr_field == f);

    let lines = vec![
        Line::from(vec![
            Span::raw("  Stored DR: "),
            Span::styled(&stored_pos, Style::default().fg(Color::Yellow)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::raw("  Latitude:  "),
            Span::styled(format!("{:>10}", form.dr_lat), lat_style(DrField::Latitude)),
            Span::raw("  "),
            Span::styled(form.dr_lat_dir.to_string(), lat_style(DrField::LatDir)),
            Span::raw("   (N/S)"),
        ]),
        Line::from(vec![
            Span::raw("  Longitude: "),
            Span::styled(format!("{:>10}", form.dr_lon), lon_style(DrField::Longitude)),
            Span::raw("  "),
            Span::styled(form.dr_lon_dir.to_string(), lon_style(DrField::LonDir)),
            Span::raw("   (E/W)"),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  Press Enter to save DR position  |  F1: DR  F2: Running Fix  F3: EP",
            Style::default().fg(Color::DarkGray),
        )),
    ];

    let block = Block::default()
        .title(if active { " [ DR Position ] " } else { " DR Position " })
        .title_alignment(Alignment::Left)
        .borders(Borders::ALL)
        .border_style(border_style);

    let para = Paragraph::new(lines).block(block);
    frame.render_widget(para, area);
}

fn render_running_fix_section(frame: &mut Frame, area: Rect, form: &DrEpForm) {
    let active = form.section == DrEpSection::RunningFix;
    let border_style = if active {
        Style::default().fg(Color::Green)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let rf = form;
    let use_time = rf.rf_use_time;
    let mode_label = if use_time { "Time" } else { "Log" };

    let mut lines = vec![
        Line::from(vec![
            Span::raw("  Course (°T): "),
            Span::styled(format!("{:>7}", rf.rf_course), field_style(active && rf.rf_field == RunFixField::Course)),
            Span::raw("   Speed (kn): "),
            Span::styled(format!("{:>6}", rf.rf_speed), field_style(active && rf.rf_field == RunFixField::Speed)),
        ]),
        Line::from(vec![
            Span::raw("  Mode [T/L]:  "),
            Span::styled(format!("{:<4}", mode_label), field_style(active && rf.rf_field == RunFixField::UseTime)),
            Span::raw("  (T=Time, L=Log distance, Space=toggle)"),
        ]),
    ];

    if use_time {
        lines.push(Line::from(vec![
            Span::raw("  Hours:  "),
            Span::styled(format!("{:>4}", rf.rf_hours), field_style(active && rf.rf_field == RunFixField::Hours)),
            Span::raw("   Minutes: "),
            Span::styled(format!("{:>4}", rf.rf_minutes), field_style(active && rf.rf_field == RunFixField::Minutes)),
        ]));
    } else {
        lines.push(Line::from(vec![
            Span::raw("  Log Distance (NM): "),
            Span::styled(format!("{:>7}", rf.rf_log_dist), field_style(active && rf.rf_field == RunFixField::LogDistance)),
        ]));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  Press Enter to advance DR position",
        Style::default().fg(Color::DarkGray),
    )));

    let block = Block::default()
        .title(if active { " [ Running Fix ] " } else { " Running Fix " })
        .title_alignment(Alignment::Left)
        .borders(Borders::ALL)
        .border_style(border_style);

    let para = Paragraph::new(lines).block(block);
    frame.render_widget(para, area);
}

fn render_ep_section(frame: &mut Frame, area: Rect, form: &DrEpForm) {
    let active = form.section == DrEpSection::EpCalculation;
    let border_style = if active {
        Style::default().fg(Color::Magenta)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let ep = form;
    let use_time = ep.ep_use_time;
    let mode_label = if use_time { "Time" } else { "Log" };

    let mut lines = vec![
        Line::from(vec![
            Span::raw("  Set (°T):    "),
            Span::styled(format!("{:>7}", ep.ep_set), field_style(active && ep.ep_field == EpField::SetDirection)),
            Span::raw("   Drift (kn): "),
            Span::styled(format!("{:>6}", ep.ep_drift), field_style(active && ep.ep_field == EpField::DriftSpeed)),
        ]),
        Line::from(vec![
            Span::raw("  Mode [T/L]:  "),
            Span::styled(format!("{:<4}", mode_label), field_style(active && ep.ep_field == EpField::UseTime)),
            Span::raw("  (T=Time, L=Log distance, Space=toggle)"),
        ]),
    ];

    if use_time {
        lines.push(Line::from(vec![
            Span::raw("  Hours:  "),
            Span::styled(format!("{:>4}", ep.ep_hours), field_style(active && ep.ep_field == EpField::Hours)),
            Span::raw("   Minutes: "),
            Span::styled(format!("{:>4}", ep.ep_minutes), field_style(active && ep.ep_field == EpField::Minutes)),
        ]));
    } else {
        lines.push(Line::from(vec![
            Span::raw("  Log Distance (NM): "),
            Span::styled(format!("{:>7}", ep.ep_log_dist), field_style(active && ep.ep_field == EpField::LogDistance)),
        ]));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  Press Enter to calculate EP from DR + set/drift",
        Style::default().fg(Color::DarkGray),
    )));

    let block = Block::default()
        .title(if active { " [ EP Calculation ] " } else { " EP Calculation " })
        .title_alignment(Alignment::Left)
        .borders(Borders::ALL)
        .border_style(border_style);

    let para = Paragraph::new(lines).block(block);
    frame.render_widget(para, area);
}

fn render_results_section(frame: &mut Frame, area: Rect, form: &DrEpForm) {
    let mut lines = vec![];

    if let Some((lat, lon)) = form.computed_dr {
        lines.push(Line::from(vec![
            Span::styled("New DR: ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            Span::styled(
                DrEpData::format_position(lat, lon),
                Style::default().fg(Color::Green),
            ),
        ]));
    }

    if let Some((lat, lon)) = form.computed_ep {
        lines.push(Line::from(vec![
            Span::styled("EP:     ", Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD)),
            Span::styled(
                DrEpData::format_position(lat, lon),
                Style::default().fg(Color::Magenta),
            ),
        ]));
    }

    if let Some(msg) = &form.message {
        lines.push(Line::from(Span::styled(msg.as_str(), Style::default().fg(Color::Yellow))));
    }

    if let Some(err) = &form.error {
        lines.push(Line::from(Span::styled(err.as_str(), Style::default().fg(Color::Red))));
    }

    if lines.is_empty() {
        lines.push(Line::from(Span::styled(
            "No results yet. Fill in fields and press Enter.",
            Style::default().fg(Color::DarkGray),
        )));
    }

    let block = Block::default()
        .title(" Results ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::White));

    let para = Paragraph::new(lines).block(block).wrap(Wrap { trim: false });
    frame.render_widget(para, area);
}

fn field_style(active: bool) -> Style {
    if active {
        Style::default()
            .fg(Color::Black)
            .bg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::White)
    }
}
