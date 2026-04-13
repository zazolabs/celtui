//! DMS (Degrees, Decimal Minutes) coordinate conversion
//!
//! This module provides functions for converting between decimal degrees
//! and DMS (Degrees, Decimal Minutes) format.
//!
//! This is essential for celestial navigation where coordinates are traditionally
//! expressed in degrees and decimal minutes rather than decimal degrees.
//! Modern navigation uses decimal minutes instead of minutes and seconds.

use std::fmt;

/// Represents a coordinate in DMS (Degrees, Decimal Minutes) format
///
/// # Fields
/// * `degrees` - Whole degrees (-180 to 180 for longitude, -90 to 90 for latitude)
/// * `minutes` - Decimal minutes component (0.0-59.999...)
/// * `seconds` - Legacy field, always 0.0 for backward compatibility
///
/// # Notes
/// For negative coordinates (South latitudes, West longitudes), the degrees
/// component is negative while minutes remain positive.
/// The seconds field is kept for backward compatibility but is always 0.0.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DMS {
    pub degrees: i32,
    pub minutes: f64,
    pub seconds: f64,
}

impl fmt::Display for DMS {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}° {:05.2}'",
            self.degrees, self.minutes
        )
    }
}

/// Converts decimal degrees to DMS (Degrees, Decimal Minutes) format
///
/// The conversion maintains high precision in decimal minutes.
/// For negative values (South latitude, West longitude), the degrees
/// component is negative while minutes are positive.
///
/// # Arguments
/// * `decimal_degrees` - Coordinate in decimal degrees
///
/// # Returns
/// A `DMS` struct containing degrees and decimal minutes (seconds is always 0.0)
///
/// # Examples
/// ```
/// use celtnav::dms::decimal_to_dms;
///
/// // 40.5° = 40° 30.0'
/// let dms = decimal_to_dms(40.5);
/// assert_eq!(dms.degrees, 40);
/// assert!((dms.minutes - 30.0).abs() < 0.01);
///
/// // 45.504167° = 45° 30.25'
/// let dms = decimal_to_dms(45.504167);
/// assert_eq!(dms.degrees, 45);
/// assert!((dms.minutes - 30.25).abs() < 0.01);
/// ```
pub fn decimal_to_dms(decimal_degrees: f64) -> DMS {
    // Handle negative values
    let sign = if decimal_degrees < 0.0 { -1.0 } else { 1.0 };
    let abs_degrees = decimal_degrees.abs();

    // Extract degrees
    let degrees = abs_degrees.floor() as i32;

    // Extract decimal minutes
    let decimal_minutes = (abs_degrees - degrees as f64) * 60.0;

    // Apply sign only to degrees
    let degrees = (degrees as f64 * sign) as i32;

    DMS {
        degrees,
        minutes: decimal_minutes,
        seconds: 0.0,
    }
}

/// Converts DMS (Degrees, Decimal Minutes) to decimal degrees
///
/// For negative coordinates (South latitude, West longitude), the degrees
/// should be negative while minutes should be positive.
///
/// # Arguments
/// * `degrees` - Degrees component (can be negative)
/// * `minutes` - Integer minutes component (0-59, always positive)
/// * `seconds` - Seconds component (0.0-59.9, always positive)
///
/// # Returns
/// Coordinate in decimal degrees
///
/// # Examples
/// ```
/// use celtnav::dms::dms_to_decimal;
///
/// // 40° 30' 0.0" = 40.5°
/// let decimal = dms_to_decimal(40, 30, 0.0);
/// assert!((decimal - 40.5).abs() < 1e-10);
/// ```
pub fn dms_to_decimal(degrees: i32, minutes: u32, seconds: f64) -> f64 {
    // Determine sign from degrees
    let sign = if degrees < 0 { -1.0 } else { 1.0 };

    // Convert to absolute degrees
    let abs_degrees = degrees.abs() as f64;

    // Convert minutes and seconds to degrees
    let decimal_minutes = minutes as f64 / 60.0;
    let decimal_seconds = seconds / 3600.0;

    // Combine and apply sign
    sign * (abs_degrees + decimal_minutes + decimal_seconds)
}

/// Converts DMS (Degrees, Decimal Minutes) to decimal degrees with f64 minutes
///
/// This is the modern version that accepts decimal minutes directly.
///
/// # Arguments
/// * `degrees` - Degrees component (can be negative)
/// * `decimal_minutes` - Decimal minutes component (0.0-59.999, always positive)
///
/// # Returns
/// Coordinate in decimal degrees
///
/// # Examples
/// ```
/// use celtnav::dms::dm_to_decimal;
///
/// // 40° 30.0' = 40.5°
/// let decimal = dm_to_decimal(40, 30.0);
/// assert!((decimal - 40.5).abs() < 1e-10);
///
/// // 45° 30.25' = 45.504167°
/// let decimal = dm_to_decimal(45, 30.25);
/// assert!((decimal - 45.504167).abs() < 1e-6);
/// ```
pub fn dm_to_decimal(degrees: i32, decimal_minutes: f64) -> f64 {
    // Determine sign from degrees
    let sign = if degrees < 0 { -1.0 } else { 1.0 };

    // Convert to absolute degrees
    let abs_degrees = degrees.abs() as f64;

    // Convert decimal minutes to degrees
    let minutes_as_degrees = decimal_minutes / 60.0;

    // Combine and apply sign
    sign * (abs_degrees + minutes_as_degrees)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dms_creation() {
        let dms = DMS {
            degrees: 45,
            minutes: 30.25,
            seconds: 0.0,
        };
        assert_eq!(dms.degrees, 45);
        assert!((dms.minutes - 30.25).abs() < 0.01);
        assert_eq!(dms.seconds, 0.0);
    }

    #[test]
    fn test_basic_conversion() {
        let dms = decimal_to_dms(40.5);
        assert_eq!(dms.degrees, 40);
        assert!((dms.minutes - 30.0).abs() < 0.01);
        assert_eq!(dms.seconds, 0.0);

        let decimal = dms_to_decimal(40, 30, 0.0);
        assert!((decimal - 40.5).abs() < 1e-10);
    }

    #[test]
    fn test_negative_conversion() {
        let dms = decimal_to_dms(-40.5);
        assert_eq!(dms.degrees, -40);
        assert!((dms.minutes - 30.0).abs() < 0.01);
        assert_eq!(dms.seconds, 0.0);

        let decimal = dms_to_decimal(-40, 30, 0.0);
        assert!((decimal - (-40.5)).abs() < 1e-10);
    }

    #[test]
    fn test_decimal_minutes() {
        // Test 45° 30.25' = 45.504167°
        let dms = decimal_to_dms(45.504167);
        assert_eq!(dms.degrees, 45);
        assert!((dms.minutes - 30.25).abs() < 0.01);
        assert_eq!(dms.seconds, 0.0);
    }

    #[test]
    fn test_round_trip() {
        let original = 45.504167;
        let dms = decimal_to_dms(original);
        let result = dm_to_decimal(dms.degrees, dms.minutes);
        assert!((result - original).abs() < 1e-5);
    }

    #[test]
    fn test_dm_to_decimal() {
        // 45° 30.25' = 45.504167°
        let decimal = dm_to_decimal(45, 30.25);
        assert!((decimal - 45.504167).abs() < 1e-6);

        // 122° 15.5' = 122.258333°
        let decimal = dm_to_decimal(122, 15.5);
        assert!((decimal - 122.258333).abs() < 1e-6);

        // Negative: -40° 30.5' = -40.508333°
        let decimal = dm_to_decimal(-40, 30.5);
        assert!((decimal - (-40.508333)).abs() < 1e-6);
    }
}
