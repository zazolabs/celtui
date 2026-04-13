//! Export functionality for sight logs and calculation results
//!
//! This module provides functions to export sight observations and fixes
//! to various formats (text reports, CSV, JSON) for record-keeping and
//! integration with other tools.

use crate::auto_compute_screen::Sight;
use celtnav::fix_calculation::Fix;
use chrono::Local;
use std::fmt::Write as FmtWrite;

/// Export format options
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    /// Plain text navigation log
    Text,
    /// Comma-separated values for spreadsheets
    Csv,
    /// JSON format for programmatic use
    Json,
}

/// Format a sight log as a navigation report
///
/// Creates a professional navigation log suitable for record-keeping
pub fn format_sight_log(sights: &[Sight], fix: Option<&Fix>) -> String {
    let mut output = String::new();

    // Header
    writeln!(&mut output, "═══════════════════════════════════════════════════════════").unwrap();
    writeln!(&mut output, "              CELESTIAL NAVIGATION LOG").unwrap();
    writeln!(&mut output, "═══════════════════════════════════════════════════════════").unwrap();
    writeln!(&mut output).unwrap();
    writeln!(&mut output, "Report generated: {}", Local::now().format("%Y-%m-%d %H:%M:%S %Z")).unwrap();
    writeln!(&mut output, "Number of sights: {}", sights.len()).unwrap();
    writeln!(&mut output).unwrap();

    // Individual sights
    writeln!(&mut output, "───────────────────────────────────────────────────────────").unwrap();
    writeln!(&mut output, "SIGHT OBSERVATIONS").unwrap();
    writeln!(&mut output, "───────────────────────────────────────────────────────────").unwrap();
    writeln!(&mut output).unwrap();

    for (i, sight) in sights.iter().enumerate() {
        writeln!(&mut output, "Sight #{}", i + 1).unwrap();
        writeln!(&mut output, "  Body:          {}", sight.body.name()).unwrap();
        writeln!(&mut output, "  Date:          {}", sight.date).unwrap();
        writeln!(&mut output, "  Time (UT):     {}", sight.time).unwrap();
        writeln!(&mut output, "  Sextant Alt:   {}", sight.sextant_altitude).unwrap();
        writeln!(&mut output, "  Index Error:   {}' ", sight.index_error).unwrap();
        writeln!(&mut output, "  Height of Eye: {} m", sight.height_of_eye).unwrap();
        writeln!(&mut output, "  DR Position:   {} {} {}, {} {} {}",
            sight.dr_latitude, sight.lat_direction,
            if sight.lat_direction == 'N' { "North" } else { "South" },
            sight.dr_longitude, sight.lon_direction,
            if sight.lon_direction == 'E' { "East" } else { "West" }
        ).unwrap();
        writeln!(&mut output).unwrap();
    }

    // Fix results if available
    if let Some(fix) = fix {
        writeln!(&mut output, "───────────────────────────────────────────────────────────").unwrap();
        writeln!(&mut output, "CALCULATED FIX").unwrap();
        writeln!(&mut output, "───────────────────────────────────────────────────────────").unwrap();
        writeln!(&mut output).unwrap();

        let lat_sign = if fix.position.latitude >= 0.0 { "N" } else { "S" };
        let lat_abs = fix.position.latitude.abs();
        let lat_deg = lat_abs.floor() as i32;
        let lat_min = (lat_abs - lat_deg as f64) * 60.0;

        let lon_sign = if fix.position.longitude >= 0.0 { "E" } else { "W" };
        let lon_abs = fix.position.longitude.abs();
        let lon_deg = lon_abs.floor() as i32;
        let lon_min = (lon_abs - lon_deg as f64) * 60.0;

        writeln!(&mut output, "  Latitude:  {} {:02}° {:06.3}'", lat_sign, lat_deg, lat_min).unwrap();
        writeln!(&mut output, "  Longitude: {} {:03}° {:06.3}'", lon_sign, lon_deg, lon_min).unwrap();
        writeln!(&mut output, "  Decimal:   {:.6}° {:.6}°", fix.position.latitude, fix.position.longitude).unwrap();
        writeln!(&mut output, "  LOPs used: {}", fix.num_lops).unwrap();

        if let Some(accuracy) = fix.accuracy_estimate {
            writeln!(&mut output, "  Accuracy:  {:.1} NM", accuracy).unwrap();
        }
        writeln!(&mut output).unwrap();
    }

    writeln!(&mut output, "═══════════════════════════════════════════════════════════").unwrap();
    writeln!(&mut output, "                    END OF REPORT").unwrap();
    writeln!(&mut output, "═══════════════════════════════════════════════════════════").unwrap();

    output
}

/// Export sights to CSV format
pub fn format_sight_csv(sights: &[Sight]) -> String {
    let mut output = String::new();

    // Header
    writeln!(&mut output, "Sight_Number,Body,Date,Time_UT,Sextant_Altitude,Index_Error,Height_of_Eye,DR_Lat,Lat_Dir,DR_Lon,Lon_Dir").unwrap();

    // Data rows
    for (i, sight) in sights.iter().enumerate() {
        writeln!(&mut output, "{},{},{},{},{},{},{},{},{},{},{}",
            i + 1,
            sight.body.name(),
            sight.date,
            sight.time,
            sight.sextant_altitude,
            sight.index_error,
            sight.height_of_eye,
            sight.dr_latitude,
            sight.lat_direction,
            sight.dr_longitude,
            sight.lon_direction,
        ).unwrap();
    }

    output
}

/// Export fix to CSV format (single line)
pub fn format_fix_csv(fix: &Fix) -> String {
    let mut output = String::new();

    writeln!(&mut output, "Latitude_Decimal,Longitude_Decimal,Num_LOPs,Accuracy_NM").unwrap();

    let accuracy_str = if let Some(acc) = fix.accuracy_estimate {
        format!("{:.1}", acc)
    } else {
        "N/A".to_string()
    };

    writeln!(&mut output, "{:.6},{:.6},{},{}",
        fix.position.latitude,
        fix.position.longitude,
        fix.num_lops,
        accuracy_str
    ).unwrap();

    output
}

/// Save export to file using the persistence module
pub fn save_export(content: &str, base_filename: &str) -> Result<String, String> {
    use crate::persistence::save_to_file;
    use chrono::Local;

    let timestamp = Local::now().format("%Y%m%d_%H%M%S");
    let filename = format!("{}_{}.txt", base_filename, timestamp);

    match save_to_file(&content, &filename) {
        Ok(path) => Ok(format!("Exported to {:?}", path)),
        Err(e) => Err(format!("Failed to export: {}", e)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auto_compute_screen::{Sight, SightCelestialBody};
    use celtnav::fix_calculation::{Fix, Position};

    fn create_test_sight() -> Sight {
        let mut sight = Sight::new();
        sight.body = SightCelestialBody::Sun;
        sight.date = "2024-01-15".to_string();
        sight.time = "12:30:00".to_string();
        sight.sextant_altitude = "45 30 0".to_string();
        sight.index_error = "2".to_string();
        sight.height_of_eye = "3".to_string();
        sight.dr_latitude = "40 30 0".to_string();
        sight.lat_direction = 'N';
        sight.dr_longitude = "70 15 0".to_string();
        sight.lon_direction = 'W';
        sight
    }

    fn create_test_fix() -> Fix {
        Fix {
            position: Position {
                latitude: 40.5,
                longitude: -70.25,
            },
            num_lops: 3,
            accuracy_estimate: Some(1.5),
        }
    }

    #[test]
    fn test_format_sight_log() {
        let sights = vec![create_test_sight()];
        let fix = create_test_fix();

        let log = format_sight_log(&sights, Some(&fix));

        assert!(log.contains("CELESTIAL NAVIGATION LOG"));
        assert!(log.contains("Sun"));
        assert!(log.contains("2024-01-15"));
        assert!(log.contains("12:30:00"));
        assert!(log.contains("45 30 0"));  // Now in "DD MM.M" format
        assert!(log.contains("CALCULATED FIX"));
    }

    #[test]
    fn test_format_sight_csv() {
        let sights = vec![create_test_sight()];
        let csv = format_sight_csv(&sights);

        assert!(csv.contains("Sight_Number,Body,Date"));
        assert!(csv.contains("1,Sun,2024-01-15"));
        assert!(csv.contains("45 30 0"));  // Now in "DD MM.M" format
    }

    #[test]
    fn test_format_fix_csv() {
        let fix = create_test_fix();
        let csv = format_fix_csv(&fix);

        assert!(csv.contains("Latitude_Decimal,Longitude_Decimal"));
        assert!(csv.contains("40."));
        assert!(csv.contains("-70."));
        assert!(csv.contains("3")); // num_lops
    }

    #[test]
    fn test_format_sight_log_without_fix() {
        let sights = vec![create_test_sight()];
        let log = format_sight_log(&sights, None);

        assert!(log.contains("CELESTIAL NAVIGATION LOG"));
        assert!(log.contains("Sun"));
        assert!(!log.contains("CALCULATED FIX"));
    }

    #[test]
    fn test_multiple_sights_csv() {
        let sight1 = create_test_sight();
        let mut sight2 = create_test_sight();
        sight2.body = SightCelestialBody::Moon;

        let csv = format_sight_csv(&vec![sight1, sight2]);
        let lines: Vec<&str> = csv.lines().collect();

        assert_eq!(lines.len(), 3); // header + 2 data rows
        assert!(lines[1].contains("Sun"));
        assert!(lines[2].contains("Moon"));
    }
}
