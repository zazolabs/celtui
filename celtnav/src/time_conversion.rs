//! Time conversion functions for celestial navigation
//!
//! This module provides functions for converting between different time systems
//! and calculating hour angles used in celestial navigation.

use chrono::{DateTime, TimeZone, Timelike, Utc};

/// Degrees per hour for solar motion
const DEGREES_PER_HOUR: f64 = 15.0;

/// Sidereal rate in degrees per hour
/// Sidereal day is ~23h 56m 4s, so sidereal time advances faster than solar time
const SIDEREAL_RATE_DEG_PER_HOUR: f64 = 15.04106864;

/// Seconds per day
const SECONDS_PER_DAY: f64 = 86400.0;

/// Days per Julian century
const DAYS_PER_CENTURY: f64 = 36525.0;

/// Calculates Greenwich Hour Angle (GHA) from UTC time
///
/// GHA represents the angular distance west from the Greenwich meridian
/// to the hour circle of a celestial body.
///
/// For the Sun, GHA can be approximated as:
/// GHA = 15 * (hours + minutes/60 + seconds/3600)
/// where the Sun moves 15 degrees per hour westward.
///
/// This is a simplified calculation that doesn't account for:
/// - Equation of time (variation due to Earth's elliptical orbit)
/// - Exact ephemeris data
///
/// # Arguments
/// * `datetime` - The UTC date and time
///
/// # Returns
/// GHA in degrees (0.0 to 360.0)
pub fn gha_from_utc(datetime: &DateTime<Utc>) -> f64 {
    // Simple approximation: Sun moves 15 degrees per hour
    let fractional_hours = time_to_fractional_hours(datetime);

    // Convert to degrees
    let gha = fractional_hours * DEGREES_PER_HOUR;

    // Normalize to [0, 360)
    normalize_angle(gha)
}

/// Calculates Local Hour Angle (LHA) from GHA and observer's longitude
///
/// LHA = GHA + Longitude (East positive)
///
/// # Arguments
/// * `gha` - Greenwich Hour Angle in degrees
/// * `longitude` - Observer's longitude in degrees (East positive, West negative)
///
/// # Returns
/// LHA in degrees (0.0 to 360.0)
pub fn lha_from_gha(gha: f64, longitude: f64) -> f64 {
    // LHA = GHA + longitude (East positive)
    let lha = gha + longitude;

    // Normalize to [0, 360)
    normalize_angle(lha)
}

/// Calculates Greenwich Sidereal Time (GST) from UTC
///
/// GST is the hour angle of the vernal equinox at Greenwich.
///
/// This implementation uses a simplified calculation:
/// 1. Calculate days since J2000.0 epoch (January 1, 2000, 12:00 UTC)
/// 2. Calculate GST at 0h UT using formula from astronomical almanac
/// 3. Add the time of day contribution
///
/// Sidereal time advances at approximately 15.04106864 degrees per hour
/// (366.25 sidereal days per 365.25 solar days)
///
/// # Arguments
/// * `datetime` - The UTC date and time
///
/// # Returns
/// GST in degrees (0.0 to 360.0)
pub fn gst_from_utc(datetime: &DateTime<Utc>) -> f64 {
    // Calculate Julian centuries since J2000.0
    let julian_centuries = calculate_julian_centuries(datetime);

    // GST at 0h UT (in degrees)
    // Formula from Astronomical Almanac
    let gst_0h = 100.4606184
        + 36000.77004 * julian_centuries
        + 0.000387933 * julian_centuries.powi(2)
        - julian_centuries.powi(3) / 38710000.0;

    // Time of day contribution
    let fractional_hours = time_to_fractional_hours(datetime);
    let gst_tod = fractional_hours * SIDEREAL_RATE_DEG_PER_HOUR;

    // Total GST
    let gst = gst_0h + gst_tod;

    // Normalize to [0, 360)
    normalize_angle(gst)
}

/// Converts time components to fractional hours
///
/// # Arguments
/// * `datetime` - The datetime to convert
///
/// # Returns
/// Time as fractional hours (0.0 to 24.0)
fn time_to_fractional_hours(datetime: &DateTime<Utc>) -> f64 {
    let hours = datetime.hour() as f64;
    let minutes = datetime.minute() as f64;
    let seconds = datetime.second() as f64;

    hours + minutes / 60.0 + seconds / 3600.0
}

/// Calculates Julian centuries since J2000.0 epoch
///
/// J2000.0 epoch is January 1, 2000, 12:00:00 UTC
///
/// # Arguments
/// * `datetime` - The datetime to calculate from
///
/// # Returns
/// Number of Julian centuries since J2000.0
fn calculate_julian_centuries(datetime: &DateTime<Utc>) -> f64 {
    // J2000.0 epoch: January 1, 2000, 12:00:00 UTC
    let j2000 = Utc.with_ymd_and_hms(2000, 1, 1, 12, 0, 0).unwrap();

    // Calculate days since J2000.0
    let duration = datetime.signed_duration_since(j2000);
    let days = duration.num_seconds() as f64 / SECONDS_PER_DAY;

    // Convert to Julian centuries
    days / DAYS_PER_CENTURY
}

/// Normalizes an angle to the range [0, 360)
///
/// # Arguments
/// * `angle` - An angle in degrees
///
/// # Returns
/// The equivalent angle in the range [0, 360)
fn normalize_angle(angle: f64) -> f64 {
    let mut normalized = angle % 360.0;
    if normalized < 0.0 {
        normalized += 360.0;
    }
    normalized
}
