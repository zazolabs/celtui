//! Input validation helpers for celestial navigation forms
//!
//! Provides validation functions for coordinates, angles, dates, times,
//! and other navigation-related inputs with detailed error messages.

use chrono::{Datelike, NaiveDate, NaiveTime};

/// Result type for validation operations
pub type ValidationResult = Result<(), String>;

/// Validate a date string in YYYY-MM-DD format
pub fn validate_date(date: &str) -> ValidationResult {
    if date.is_empty() {
        return Err("Date is required".to_string());
    }

    match NaiveDate::parse_from_str(date, "%Y-%m-%d") {
        Ok(parsed_date) => {
            // Check if date is reasonable (between 1900 and 2100)
            if parsed_date.year() < 1900 || parsed_date.year() > 2100 {
                Err(format!("Date year {} is out of range (1900-2100)", parsed_date.year()))
            } else {
                Ok(())
            }
        }
        Err(_) => Err("Invalid date format. Use YYYY-MM-DD (e.g., 2024-03-15)".to_string()),
    }
}

/// Validate a time string in HH:MM:SS or HH:MM format
pub fn validate_time(time: &str) -> ValidationResult {
    if time.is_empty() {
        return Err("Time is required".to_string());
    }

    // Try both formats
    let result = NaiveTime::parse_from_str(time, "%H:%M:%S")
        .or_else(|_| NaiveTime::parse_from_str(time, "%H:%M"));

    match result {
        Ok(_) => Ok(()),
        Err(_) => Err("Invalid time format. Use HH:MM:SS or HH:MM (e.g., 14:30:00)".to_string()),
    }
}

/// Validate degrees component of an angle (0-90 for latitude, 0-180 for longitude, 0-360 for general)
pub fn validate_degrees(degrees_str: &str, min_val: f64, max_val: f64, name: &str) -> ValidationResult {
    if degrees_str.is_empty() {
        return Err(format!("{} degrees is required", name));
    }

    match degrees_str.parse::<f64>() {
        Ok(deg) => {
            if deg < min_val || deg > max_val {
                Err(format!("{} degrees must be between {} and {}", name, min_val, max_val))
            } else {
                Ok(())
            }
        }
        Err(_) => Err(format!("{} degrees must be a valid number", name)),
    }
}

/// Validate decimal minutes component of an angle (0-60)
pub fn validate_minutes(minutes_str: &str, name: &str) -> ValidationResult {
    if minutes_str.is_empty() {
        return Err(format!("{} minutes is required", name));
    }

    match minutes_str.parse::<f64>() {
        Ok(min) => {
            if min < 0.0 || min >= 60.0 {
                Err(format!("{} minutes must be between 0.0 and 59.999", name))
            } else {
                Ok(())
            }
        }
        Err(_) => Err(format!("{} minutes must be a valid number", name)),
    }
}

/// Validate seconds component of an angle (0-60 with one decimal place)
pub fn validate_seconds(seconds_str: &str, name: &str) -> ValidationResult {
    if seconds_str.is_empty() {
        return Ok(()); // Seconds can be optional, defaults to 0
    }

    match seconds_str.parse::<f64>() {
        Ok(sec) => {
            if sec < 0.0 || sec >= 60.0 {
                Err(format!("{} seconds must be between 0.0 and 59.9", name))
            } else {
                Ok(())
            }
        }
        Err(_) => Err(format!("{} seconds must be a valid number", name)),
    }
}

/// Validate DM angle string with range checking
///
/// # Arguments
/// * `dm_str` - String in "DD MM.M" format
/// * `min_deg` - Minimum degrees value
/// * `max_deg` - Maximum degrees value
/// * `name` - Field name for error messages
///
/// # Returns
/// ValidationResult
pub fn validate_dm_angle(dm_str: &str, min_deg: f64, max_deg: f64, name: &str) -> ValidationResult {
    if dm_str.is_empty() {
        return Err(format!("{} is required", name));
    }

    let (degrees, _minutes) = parse_dm(dm_str)
        .map_err(|e| format!("{}: {}", name, e))?;

    // Validate degrees range
    if degrees < min_deg || degrees > max_deg {
        return Err(format!("{} degrees must be between {} and {}", name, min_deg, max_deg));
    }

    // Minutes range is already validated by parse_dm
    Ok(())
}

/// Validate DMS angle string with range checking (legacy, for backward compatibility)
///
/// # Arguments
/// * `dms_str` - String in "DD MM.M" or "DD MM.M" format
/// * `min_deg` - Minimum degrees value
/// * `max_deg` - Maximum degrees value
/// * `name` - Field name for error messages
///
/// # Returns
/// ValidationResult
pub fn validate_dms_angle(dms_str: &str, min_deg: f64, max_deg: f64, name: &str) -> ValidationResult {
    validate_dm_angle(dms_str, min_deg, max_deg, name)
}

/// Validate latitude in "DD MM.M" format
pub fn validate_latitude_dms(lat_str: &str) -> ValidationResult {
    validate_dms_angle(lat_str, 0.0, 90.0, "Latitude")
}

/// Validate longitude in "DD MM.M" format
pub fn validate_longitude_dms(lon_str: &str) -> ValidationResult {
    validate_dms_angle(lon_str, 0.0, 180.0, "Longitude")
}

/// Validate sextant altitude in "DD MM.M" format
pub fn validate_sextant_altitude_dms(hs_str: &str) -> ValidationResult {
    validate_dms_angle(hs_str, 0.0, 90.0, "Sextant altitude")
}

/// Validate GHA in "DD MM.M" format
pub fn validate_gha_dms(gha_str: &str) -> ValidationResult {
    validate_dms_angle(gha_str, 0.0, 360.0, "GHA")
}

/// Validate declination in "DD MM.M" format
pub fn validate_declination_dms(dec_str: &str) -> ValidationResult {
    validate_dms_angle(dec_str, 0.0, 90.0, "Declination")
}

/// Validate LHA in "DD MM.M" format
pub fn validate_lha_dms(lha_str: &str) -> ValidationResult {
    validate_dms_angle(lha_str, 0.0, 360.0, "LHA")
}

/// Validate latitude in "DD MM" format (old format, kept for backward compatibility)
pub fn validate_latitude(lat_str: &str) -> ValidationResult {
    if lat_str.is_empty() {
        return Err("Latitude is required".to_string());
    }

    let parts: Vec<&str> = lat_str.trim().split_whitespace().collect();
    if parts.len() != 2 {
        return Err("Latitude must be in format: DD MM (e.g., 40 30)".to_string());
    }

    validate_degrees(parts[0], 0.0, 90.0, "Latitude")?;
    validate_minutes(parts[1], "Latitude")?;

    Ok(())
}

/// Validate longitude in "DD MM" or "DDD MM" format (old format, kept for backward compatibility)
pub fn validate_longitude(lon_str: &str) -> ValidationResult {
    if lon_str.is_empty() {
        return Err("Longitude is required".to_string());
    }

    let parts: Vec<&str> = lon_str.trim().split_whitespace().collect();
    if parts.len() != 2 {
        return Err("Longitude must be in format: DDD MM (e.g., 070 15)".to_string());
    }

    validate_degrees(parts[0], 0.0, 180.0, "Longitude")?;
    validate_minutes(parts[1], "Longitude")?;

    Ok(())
}

/// Validate sextant altitude degrees (typically 0-90)
pub fn validate_sextant_degrees(deg_str: &str) -> ValidationResult {
    validate_degrees(deg_str, 0.0, 90.0, "Sextant altitude")
}

/// Validate sextant altitude minutes
pub fn validate_sextant_minutes(min_str: &str) -> ValidationResult {
    validate_minutes(min_str, "Sextant altitude")
}

/// Validate index error in arc minutes (typically -10 to +10)
pub fn validate_index_error(ie_str: &str) -> ValidationResult {
    if ie_str.is_empty() {
        return Ok(()); // Index error is optional, defaults to 0
    }

    match ie_str.parse::<f64>() {
        Ok(ie) => {
            if ie < -30.0 || ie > 30.0 {
                Err("Index error must be between -30 and +30 arcminutes".to_string())
            } else {
                Ok(())
            }
        }
        Err(_) => Err("Index error must be a valid number".to_string()),
    }
}

/// Validate height of eye in meters (typically 0-100)
pub fn validate_height_of_eye(hoe_str: &str) -> ValidationResult {
    if hoe_str.is_empty() {
        return Ok(()); // Height of eye is optional, defaults to 0
    }

    match hoe_str.parse::<f64>() {
        Ok(hoe) => {
            if hoe < 0.0 || hoe > 500.0 {
                Err("Height of eye must be between 0 and 500 meters".to_string())
            } else {
                Ok(())
            }
        }
        Err(_) => Err("Height of eye must be a valid number".to_string()),
    }
}

/// Validate hemisphere direction (N/S for latitude, E/W for longitude)
pub fn validate_direction(dir: char, valid_chars: &[char], name: &str) -> ValidationResult {
    if valid_chars.contains(&dir.to_ascii_uppercase()) {
        Ok(())
    } else {
        let valid_str: String = valid_chars.iter().collect();
        Err(format!("{} must be one of: {}", name, valid_str))
    }
}

/// Parse coordinate string "DD MM" to decimal degrees
pub fn parse_coordinate(coord_str: &str) -> Result<f64, String> {
    let parts: Vec<&str> = coord_str.trim().split_whitespace().collect();
    if parts.len() != 2 {
        return Err("Coordinate must be in format: DD MM".to_string());
    }

    let degrees: f64 = parts[0].parse()
        .map_err(|_| "Invalid degrees value".to_string())?;
    let minutes: f64 = parts[1].parse()
        .map_err(|_| "Invalid minutes value".to_string())?;

    Ok(degrees + minutes / 60.0)
}

/// Parse DM format string "DD MM.M" into components
///
/// Accepts formats:
/// - "DD MM.M" - full format with decimal minutes (e.g., "45 30.25")
/// - "DD MM" - whole minutes (e.g., "45 30")
/// - "DD" - degrees only, minutes default to 0.0 (e.g., "45")
///
/// # Arguments
/// * `input` - String in "DD MM.M" format
///
/// # Returns
/// Result with (degrees, decimal_minutes) or error message
///
/// # Examples
/// ```
/// use celtui::validation::parse_dm;
///
/// let (d, m) = parse_dm("45 30.25").unwrap();
/// assert_eq!(d, 45.0);
/// assert!((m - 30.25).abs() < 0.01);
///
/// let (d, m) = parse_dm("45 30").unwrap();
/// assert_eq!((d, m), (45.0, 30.0));
///
/// let (d, m) = parse_dm("45").unwrap();
/// assert_eq!((d, m), (45.0, 0.0));
/// ```
pub fn parse_dm(input: &str) -> Result<(f64, f64), String> {
    let input = input.trim();
    if input.is_empty() {
        return Err("Input is required".to_string());
    }

    let parts: Vec<&str> = input.split_whitespace().collect();

    match parts.len() {
        1 => {
            // Just degrees
            let degrees = parts[0].parse::<f64>()
                .map_err(|_| "Invalid degrees value".to_string())?;
            Ok((degrees, 0.0))
        }
        2 => {
            // Degrees and decimal minutes
            let degrees = parts[0].parse::<f64>()
                .map_err(|_| "Invalid degrees value".to_string())?;
            let minutes = parts[1].parse::<f64>()
                .map_err(|_| "Invalid minutes value".to_string())?;

            if minutes < 0.0 || minutes >= 60.0 {
                return Err("Minutes must be between 0.0 and 59.999".to_string());
            }

            Ok((degrees, minutes))
        }
        _ => {
            Err("Format must be 'DD MM.M', 'DD MM', or 'DD'".to_string())
        }
    }
}

/// Parse DMS format string "DD MM.M" into components (legacy, for backward compatibility)
///
/// This function is kept for backward compatibility. New code should use parse_dm.
///
/// Accepts formats:
/// - "DD MM.M" - full format (e.g., "45 30 15.5")
/// - "DD MM" - seconds default to 0 (e.g., "45 30")
/// - "DD" - minutes and seconds default to 0 (e.g., "45")
///
/// # Arguments
/// * `input` - String in "DD MM.M" format
///
/// # Returns
/// Result with (degrees, minutes, seconds) or error message
pub fn parse_dms(input: &str) -> Result<(f64, f64, f64), String> {
    let input = input.trim();
    if input.is_empty() {
        return Err("Input is required".to_string());
    }

    let parts: Vec<&str> = input.split_whitespace().collect();

    match parts.len() {
        1 => {
            // Just degrees
            let degrees = parts[0].parse::<f64>()
                .map_err(|_| "Invalid degrees value".to_string())?;
            Ok((degrees, 0.0, 0.0))
        }
        2 => {
            // Degrees and minutes - treat as decimal minutes, convert to DMS
            let degrees = parts[0].parse::<f64>()
                .map_err(|_| "Invalid degrees value".to_string())?;
            let minutes = parts[1].parse::<f64>()
                .map_err(|_| "Invalid minutes value".to_string())?;

            if minutes < 0.0 || minutes >= 60.0 {
                return Err("Minutes must be between 0.0 and 59.999".to_string());
            }

            // Return as (degrees, integer_minutes, fractional_seconds)
            // For "45 30.25", return (45, 30, 15.0) where 0.25 minutes = 15 seconds
            let int_minutes = minutes.floor();
            let frac_minutes = minutes - int_minutes;
            let seconds = frac_minutes * 60.0;

            Ok((degrees, int_minutes, seconds))
        }
        3 => {
            // Degrees, minutes, and seconds
            let degrees = parts[0].parse::<f64>()
                .map_err(|_| "Invalid degrees value".to_string())?;
            let minutes = parts[1].parse::<f64>()
                .map_err(|_| "Invalid minutes value".to_string())?;
            let seconds = parts[2].parse::<f64>()
                .map_err(|_| "Invalid seconds value".to_string())?;

            if minutes < 0.0 || minutes >= 60.0 {
                return Err("Minutes must be between 0 and 59.99".to_string());
            }
            if seconds < 0.0 || seconds >= 60.0 {
                return Err("Seconds must be between 0.0 and 59.9".to_string());
            }

            Ok((degrees, minutes, seconds))
        }
        _ => {
            Err("Format must be 'DD MM.M', 'DD MM', or 'DD'".to_string())
        }
    }
}

/// Format DM components as input string "DD MM.MM"
///
/// # Arguments
/// * `degrees` - Degrees component
/// * `decimal_minutes` - Decimal minutes component
///
/// # Returns
/// String formatted as "DD MM.MM" with zero-padded minutes and 2 decimal places
///
/// # Examples
/// ```
/// use celtui::validation::format_dm_input;
///
/// assert_eq!(format_dm_input(45.0, 30.25), "45 30.25");
/// assert_eq!(format_dm_input(45.0, 30.0), "45 30.00");
/// assert_eq!(format_dm_input(45.0, 0.0), "45 00.00");
/// assert_eq!(format_dm_input(45.0, 5.5), "45 05.50");
/// ```
pub fn format_dm_input(degrees: f64, decimal_minutes: f64) -> String {
    format!("{} {:05.2}", degrees as i32, decimal_minutes)
}

/// Format DMS components as input string "DD MM.M" (legacy)
///
/// This function is kept for backward compatibility. New code should use format_dm_input.
///
/// # Arguments
/// * `degrees` - Degrees component
/// * `minutes` - Minutes component
/// * `seconds` - Seconds component
///
/// # Returns
/// String formatted as "DD MM.M" (converts to decimal minutes format)
pub fn format_dms_input(degrees: f64, minutes: f64, seconds: f64) -> String {
    // Convert to decimal minutes
    let decimal_minutes = minutes + (seconds / 60.0);
    format_dm_input(degrees, decimal_minutes)
}

/// Convert DMS components to decimal degrees
///
/// # Arguments
/// * `degrees` - Degrees component (can be negative for S/W)
/// * `minutes` - Minutes component (0-59)
/// * `seconds` - Seconds component (0.0-59.9)
///
/// # Returns
/// Decimal degrees
pub fn dms_to_decimal_degrees(degrees: i32, minutes: u32, seconds: f64) -> f64 {
    celtnav::dms_to_decimal(degrees, minutes, seconds)
}

/// Format decimal degrees as DM string
///
/// # Arguments
/// * `decimal_degrees` - Coordinate in decimal degrees
///
/// # Returns
/// String formatted as "DD° MM.MM'"
pub fn format_dm(decimal_degrees: f64) -> String {
    let dms = celtnav::decimal_to_dms(decimal_degrees);
    // Format with zero-padded minutes and 2 decimal places
    format!("{}° {:05.2}'", dms.degrees, dms.minutes)
}

/// Format decimal degrees as DMS string (legacy)
///
/// This function is kept for backward compatibility. New code should use format_dm.
///
/// # Returns
/// String formatted as "DD° MM.M'"
pub fn format_dms(decimal_degrees: f64) -> String {
    format_dm(decimal_degrees)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_date() {
        assert!(validate_date("2024-03-15").is_ok());
        assert!(validate_date("2024-12-31").is_ok());
        assert!(validate_date("").is_err());
        assert!(validate_date("2024/03/15").is_err());
        assert!(validate_date("15-03-2024").is_err());
        assert!(validate_date("2024-13-01").is_err());
        assert!(validate_date("1850-01-01").is_err()); // Too old
    }

    #[test]
    fn test_validate_time() {
        assert!(validate_time("14:30:00").is_ok());
        assert!(validate_time("14:30").is_ok());
        assert!(validate_time("00:00:00").is_ok());
        assert!(validate_time("23:59:59").is_ok());
        assert!(validate_time("").is_err());
        assert!(validate_time("25:00:00").is_err());
        assert!(validate_time("14:60:00").is_err());
    }

    #[test]
    fn test_validate_degrees() {
        assert!(validate_degrees("45", 0.0, 90.0, "Latitude").is_ok());
        assert!(validate_degrees("0", 0.0, 90.0, "Latitude").is_ok());
        assert!(validate_degrees("90", 0.0, 90.0, "Latitude").is_ok());
        assert!(validate_degrees("91", 0.0, 90.0, "Latitude").is_err());
        assert!(validate_degrees("-1", 0.0, 90.0, "Latitude").is_err());
        assert!(validate_degrees("abc", 0.0, 90.0, "Latitude").is_err());
    }

    #[test]
    fn test_validate_minutes() {
        assert!(validate_minutes("30", "Test").is_ok());
        assert!(validate_minutes("0", "Test").is_ok());
        assert!(validate_minutes("59.9", "Test").is_ok());
        assert!(validate_minutes("60", "Test").is_err());
        assert!(validate_minutes("-1", "Test").is_err());
        assert!(validate_minutes("abc", "Test").is_err());
    }

    #[test]
    fn test_validate_seconds() {
        assert!(validate_seconds("30.5", "Test").is_ok());
        assert!(validate_seconds("0", "Test").is_ok());
        assert!(validate_seconds("0.0", "Test").is_ok());
        assert!(validate_seconds("59.9", "Test").is_ok());
        assert!(validate_seconds("", "Test").is_ok()); // Optional
        assert!(validate_seconds("60", "Test").is_err());
        assert!(validate_seconds("60.0", "Test").is_err());
        assert!(validate_seconds("-1", "Test").is_err());
        assert!(validate_seconds("abc", "Test").is_err());
    }

    #[test]
    fn test_validate_latitude() {
        assert!(validate_latitude("40 30").is_ok());
        assert!(validate_latitude("0 0").is_ok());
        assert!(validate_latitude("90 0").is_ok());
        assert!(validate_latitude("40").is_err()); // Missing minutes
        assert!(validate_latitude("91 0").is_err()); // Out of range
        assert!(validate_latitude("40 60").is_err()); // Invalid minutes
    }

    #[test]
    fn test_validate_longitude() {
        assert!(validate_longitude("70 15").is_ok());
        assert!(validate_longitude("0 0").is_ok());
        assert!(validate_longitude("180 0").is_ok());
        assert!(validate_longitude("70").is_err()); // Missing minutes
        assert!(validate_longitude("181 0").is_err()); // Out of range
        assert!(validate_longitude("70 60").is_err()); // Invalid minutes
    }

    #[test]
    fn test_validate_index_error() {
        assert!(validate_index_error("0").is_ok());
        assert!(validate_index_error("5").is_ok());
        assert!(validate_index_error("-5").is_ok());
        assert!(validate_index_error("").is_ok()); // Optional
        assert!(validate_index_error("31").is_err()); // Out of range
        assert!(validate_index_error("abc").is_err());
    }

    #[test]
    fn test_validate_height_of_eye() {
        assert!(validate_height_of_eye("3").is_ok());
        assert!(validate_height_of_eye("0").is_ok());
        assert!(validate_height_of_eye("100").is_ok());
        assert!(validate_height_of_eye("").is_ok()); // Optional
        assert!(validate_height_of_eye("-1").is_err()); // Negative
        assert!(validate_height_of_eye("501").is_err()); // Too high
    }

    #[test]
    fn test_validate_direction() {
        assert!(validate_direction('N', &['N', 'S'], "Latitude").is_ok());
        assert!(validate_direction('S', &['N', 'S'], "Latitude").is_ok());
        assert!(validate_direction('n', &['N', 'S'], "Latitude").is_ok()); // Case insensitive
        assert!(validate_direction('E', &['N', 'S'], "Latitude").is_err());
        assert!(validate_direction('X', &['N', 'S'], "Latitude").is_err());
    }

    #[test]
    fn test_parse_coordinate() {
        assert_eq!(parse_coordinate("40 30").unwrap(), 40.5);
        assert_eq!(parse_coordinate("0 0").unwrap(), 0.0);
        assert_eq!(parse_coordinate("90 0").unwrap(), 90.0);
        assert!(parse_coordinate("40").is_err());
        assert!(parse_coordinate("abc def").is_err());
    }

    #[test]
    fn test_dms_to_decimal_degrees() {
        let result = dms_to_decimal_degrees(40, 30, 0.0);
        assert!((result - 40.5).abs() < 1e-10);

        let result = dms_to_decimal_degrees(40, 30, 45.5);
        assert!((result - 40.512639).abs() < 1e-6);

        let result = dms_to_decimal_degrees(-40, 30, 45.5);
        assert!((result - (-40.512639)).abs() < 1e-6);
    }

    #[test]
    fn test_format_dm() {
        assert_eq!(format_dm(40.5), "40° 30.00'");
        assert_eq!(format_dm(-40.5), "-40° 30.00'");
        // 45.504167° = 45° 30.25002' which rounds to 30.25' at 2 decimal places
        assert_eq!(format_dm(45.504167), "45° 30.25'");
        // Test with exact value that gives 30.20': 45 + 30.2/60 = 45.503333...
        assert_eq!(format_dm(45.503333), "45° 30.20'");
        // Test small minutes with zero padding
        assert_eq!(format_dm(45.015833), "45° 00.95'"); // 45° 0.95'
        assert_eq!(format_dm(23.015), "23° 00.90'"); // 23° 0.9'
    }

    #[test]
    fn test_format_dms() {
        // Legacy function should now format as decimal minutes with zero-padded 2 decimals
        assert_eq!(format_dms(40.5), "40° 30.00'");
        assert_eq!(format_dms(-40.5), "-40° 30.00'");
        assert_eq!(format_dms(45.504167), "45° 30.25'");
    }

    // Tests for parse_dm function
    #[test]
    fn test_parse_dm_full_format() {
        let (d, m) = parse_dm("45 30.25").unwrap();
        assert_eq!(d, 45.0);
        assert!((m - 30.25).abs() < 0.01);
    }

    #[test]
    fn test_parse_dm_whole_minutes() {
        let (d, m) = parse_dm("45 30").unwrap();
        assert_eq!(d, 45.0);
        assert_eq!(m, 30.0);
    }

    #[test]
    fn test_parse_dm_degrees_only() {
        let (d, m) = parse_dm("45").unwrap();
        assert_eq!(d, 45.0);
        assert_eq!(m, 0.0);
    }

    #[test]
    fn test_parse_dm_with_spaces() {
        let (d, m) = parse_dm("  45   30.25  ").unwrap();
        assert_eq!(d, 45.0);
        assert!((m - 30.25).abs() < 0.01);
    }

    #[test]
    fn test_parse_dm_negative_degrees() {
        let (d, m) = parse_dm("-40 30.5").unwrap();
        assert_eq!(d, -40.0);
        assert!((m - 30.5).abs() < 0.01);
    }

    #[test]
    fn test_parse_dm_empty() {
        assert!(parse_dm("").is_err());
        assert!(parse_dm("   ").is_err());
    }

    #[test]
    fn test_parse_dm_invalid() {
        assert!(parse_dm("abc 30").is_err());
        assert!(parse_dm("45 abc").is_err());
        assert!(parse_dm("45 60").is_err());
        assert!(parse_dm("45 -1").is_err());
    }

    // Tests for parse_dms function (legacy - now treats 2-part as decimal minutes)
    #[test]
    fn test_parse_dms_full_format() {
        let (d, m, s) = parse_dms("45 30 15.5").unwrap();
        assert_eq!(d, 45.0);
        assert_eq!(m, 30.0);
        assert_eq!(s, 15.5);
    }

    #[test]
    fn test_parse_dms_degrees_minutes_as_decimal() {
        // Now "45 30.25" is treated as decimal minutes and converted
        let (d, m, s) = parse_dms("45 30.25").unwrap();
        assert_eq!(d, 45.0);
        assert_eq!(m, 30.0);
        assert!((s - 15.0).abs() < 0.1); // 0.25 minutes = 15 seconds
    }

    #[test]
    fn test_parse_dms_degrees_only() {
        let (d, m, s) = parse_dms("45").unwrap();
        assert_eq!(d, 45.0);
        assert_eq!(m, 0.0);
        assert_eq!(s, 0.0);
    }

    #[test]
    fn test_parse_dms_with_spaces() {
        let (d, m, s) = parse_dms("  45   30   15.5  ").unwrap();
        assert_eq!(d, 45.0);
        assert_eq!(m, 30.0);
        assert_eq!(s, 15.5);
    }

    #[test]
    fn test_parse_dms_negative_degrees() {
        let (d, m, s) = parse_dms("-40 30 15").unwrap();
        assert_eq!(d, -40.0);
        assert_eq!(m, 30.0);
        assert_eq!(s, 15.0);
    }

    #[test]
    fn test_parse_dms_empty() {
        assert!(parse_dms("").is_err());
        assert!(parse_dms("   ").is_err());
    }

    #[test]
    fn test_parse_dms_invalid_degrees() {
        assert!(parse_dms("abc 30 15").is_err());
        assert!(parse_dms("abc").is_err());
    }

    #[test]
    fn test_parse_dms_invalid_minutes() {
        assert!(parse_dms("45 abc 15").is_err());
        assert!(parse_dms("45 60 0").is_err());
        assert!(parse_dms("45 -1 0").is_err());
    }

    #[test]
    fn test_parse_dms_invalid_seconds() {
        assert!(parse_dms("45 30 abc").is_err());
        assert!(parse_dms("45 30 60").is_err());
        assert!(parse_dms("45 30 -1").is_err());
    }

    #[test]
    fn test_parse_dms_too_many_parts() {
        assert!(parse_dms("45 30 15 10").is_err());
    }

    #[test]
    fn test_parse_dms_decimal_seconds() {
        let (d, m, s) = parse_dms("122 15 30.2").unwrap();
        assert_eq!(d, 122.0);
        assert_eq!(m, 15.0);
        assert!((s - 30.2).abs() < 0.01);
    }

    // Tests for format_dm_input function
    #[test]
    fn test_format_dm_input_full() {
        assert_eq!(format_dm_input(45.0, 30.25), "45 30.25");
    }

    #[test]
    fn test_format_dm_input_whole_minutes() {
        assert_eq!(format_dm_input(45.0, 30.0), "45 30.00");
    }

    #[test]
    fn test_format_dm_input_degrees_only() {
        assert_eq!(format_dm_input(45.0, 0.0), "45 00.00");
    }

    #[test]
    fn test_format_dm_input_negative_degrees() {
        assert_eq!(format_dm_input(-40.0, 30.5), "-40 30.50");
    }

    #[test]
    fn test_format_dm_input_small_minutes() {
        assert_eq!(format_dm_input(45.0, 5.5), "45 05.50");
        assert_eq!(format_dm_input(23.0, 0.9), "23 00.90");
    }

    // Tests for format_dms_input function (legacy - now converts to decimal minutes)
    #[test]
    fn test_format_dms_input_legacy() {
        // 30 minutes + 15.0 seconds = 30.25 minutes
        assert_eq!(format_dms_input(45.0, 30.0, 15.0), "45 30.25");
    }

    #[test]
    fn test_format_dms_input_no_seconds() {
        assert_eq!(format_dms_input(45.0, 30.0, 0.0), "45 30.00");
    }

    #[test]
    fn test_format_dms_input_degrees_only() {
        assert_eq!(format_dms_input(45.0, 0.0, 0.0), "45 00.00");
    }

    // Round-trip tests for decimal minutes
    #[test]
    fn test_parse_format_dm_roundtrip() {
        let inputs = vec![
            "45 30.25",
            "45 30",
            "45",
            "122 15.5",
            "-40 30.5",
        ];

        for input in inputs {
            let (d, m) = parse_dm(input).unwrap();
            let formatted = format_dm_input(d, m);
            let (d2, m2) = parse_dm(&formatted).unwrap();
            assert_eq!(d, d2);
            assert!((m - m2).abs() < 0.01);
        }
    }

    // Tests for DM validation functions (now using decimal minutes)
    #[test]
    fn test_validate_latitude_dms() {
        assert!(validate_latitude_dms("40 30.25").is_ok());
        assert!(validate_latitude_dms("0 0").is_ok());
        assert!(validate_latitude_dms("90 0").is_ok());
        assert!(validate_latitude_dms("45 30").is_ok());
        assert!(validate_latitude_dms("45").is_ok());
        assert!(validate_latitude_dms("91 0").is_err()); // Out of range
        assert!(validate_latitude_dms("").is_err()); // Empty
        assert!(validate_latitude_dms("abc").is_err()); // Invalid
    }

    #[test]
    fn test_validate_longitude_dms() {
        assert!(validate_longitude_dms("122 15.5").is_ok());
        assert!(validate_longitude_dms("0 0").is_ok());
        assert!(validate_longitude_dms("180 0").is_ok());
        assert!(validate_longitude_dms("122 15").is_ok());
        assert!(validate_longitude_dms("122").is_ok());
        assert!(validate_longitude_dms("181 0").is_err()); // Out of range
        assert!(validate_longitude_dms("").is_err()); // Empty
    }

    #[test]
    fn test_validate_sextant_altitude_dms() {
        assert!(validate_sextant_altitude_dms("45 30.25").is_ok());
        assert!(validate_sextant_altitude_dms("0 0").is_ok());
        assert!(validate_sextant_altitude_dms("90 0").is_ok());
        assert!(validate_sextant_altitude_dms("91 0").is_err()); // Out of range
    }

    #[test]
    fn test_validate_gha_dms() {
        assert!(validate_gha_dms("180 30.25").is_ok());
        assert!(validate_gha_dms("0 0").is_ok());
        assert!(validate_gha_dms("360 0").is_ok());
        assert!(validate_gha_dms("361 0").is_err()); // Out of range
    }

    #[test]
    fn test_validate_declination_dms() {
        assert!(validate_declination_dms("23 26.357").is_ok());
        assert!(validate_declination_dms("0 0").is_ok());
        assert!(validate_declination_dms("90 0").is_ok());
        assert!(validate_declination_dms("91 0").is_err()); // Out of range
    }

    #[test]
    fn test_validate_lha_dms() {
        assert!(validate_lha_dms("180 30.25").is_ok());
        assert!(validate_lha_dms("0 0").is_ok());
        assert!(validate_lha_dms("360 0").is_ok());
        assert!(validate_lha_dms("361 0").is_err()); // Out of range
    }
}
