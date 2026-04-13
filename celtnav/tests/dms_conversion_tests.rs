//! Tests for DMS (Degrees, Decimal Minutes) coordinate conversion
//!
//! These tests verify the conversion between decimal degrees and DM format
//! with decimal minutes precision.

use celtnav::dms::{decimal_to_dms, dms_to_decimal, dm_to_decimal, DMS};
use approx::assert_relative_eq;

#[test]
fn test_dms_struct_creation() {
    let dms = DMS {
        degrees: 45,
        minutes: 30.25,
        seconds: 0.0,
    };
    assert_eq!(dms.degrees, 45);
    assert_relative_eq!(dms.minutes, 30.25, epsilon = 0.01);
    assert_eq!(dms.seconds, 0.0);
}

#[test]
fn test_decimal_to_dms_zero() {
    let dms = decimal_to_dms(0.0);
    assert_eq!(dms.degrees, 0);
    assert_relative_eq!(dms.minutes, 0.0, epsilon = 0.01);
    assert_eq!(dms.seconds, 0.0);
}

#[test]
fn test_decimal_to_dms_simple() {
    // 40.5° = 40° 30.0'
    let dms = decimal_to_dms(40.5);
    assert_eq!(dms.degrees, 40);
    assert_relative_eq!(dms.minutes, 30.0, epsilon = 0.01);
    assert_eq!(dms.seconds, 0.0);
}

#[test]
fn test_decimal_to_dms_with_decimal_minutes() {
    // 45.504167° = 45° 30.25'
    let dms = decimal_to_dms(45.504167);
    assert_eq!(dms.degrees, 45);
    assert_relative_eq!(dms.minutes, 30.25, epsilon = 0.01);
    assert_eq!(dms.seconds, 0.0);
}

#[test]
fn test_decimal_to_dms_with_more_decimal_minutes() {
    // 122.258333° = 122° 15.5'
    let dms = decimal_to_dms(122.258333);
    assert_eq!(dms.degrees, 122);
    assert_relative_eq!(dms.minutes, 15.5, epsilon = 0.01);
    assert_eq!(dms.seconds, 0.0);
}

#[test]
fn test_decimal_to_dms_negative() {
    // -40.508333° = -40° 30.5'
    let dms = decimal_to_dms(-40.508333);
    assert_eq!(dms.degrees, -40);
    assert_relative_eq!(dms.minutes, 30.5, epsilon = 0.01);
    assert_eq!(dms.seconds, 0.0);
}

#[test]
fn test_decimal_to_dms_ninety() {
    // 90° = 90° 0.0'
    let dms = decimal_to_dms(90.0);
    assert_eq!(dms.degrees, 90);
    assert_relative_eq!(dms.minutes, 0.0, epsilon = 0.01);
    assert_eq!(dms.seconds, 0.0);
}

#[test]
fn test_decimal_to_dms_one_eighty() {
    // 180° = 180° 0.0'
    let dms = decimal_to_dms(180.0);
    assert_eq!(dms.degrees, 180);
    assert_relative_eq!(dms.minutes, 0.0, epsilon = 0.01);
    assert_eq!(dms.seconds, 0.0);
}

#[test]
fn test_dm_to_decimal_zero() {
    let decimal = dm_to_decimal(0, 0.0);
    assert_relative_eq!(decimal, 0.0, epsilon = 1e-10);
}

#[test]
fn test_dm_to_decimal_simple() {
    // 40° 30.0' = 40.5°
    let decimal = dm_to_decimal(40, 30.0);
    assert_relative_eq!(decimal, 40.5, epsilon = 1e-10);
}

#[test]
fn test_dm_to_decimal_with_decimal_minutes() {
    // 45° 30.25' = 45.504167°
    let decimal = dm_to_decimal(45, 30.25);
    assert_relative_eq!(decimal, 45.504167, epsilon = 1e-6);
}

#[test]
fn test_dm_to_decimal_with_more_decimals() {
    // 122° 15.5' = 122.258333°
    let decimal = dm_to_decimal(122, 15.5);
    assert_relative_eq!(decimal, 122.258333, epsilon = 1e-6);
}

#[test]
fn test_dm_to_decimal_negative() {
    // -40° 30.5' = -40.508333°
    let decimal = dm_to_decimal(-40, 30.5);
    assert_relative_eq!(decimal, -40.508333, epsilon = 1e-6);
}

#[test]
fn test_dm_to_decimal_ninety() {
    // 90° 0.0' = 90.0°
    let decimal = dm_to_decimal(90, 0.0);
    assert_relative_eq!(decimal, 90.0, epsilon = 1e-10);
}

#[test]
fn test_dm_to_decimal_one_eighty() {
    // 180° 0.0' = 180.0°
    let decimal = dm_to_decimal(180, 0.0);
    assert_relative_eq!(decimal, 180.0, epsilon = 1e-10);
}

#[test]
fn test_round_trip_conversion() {
    // Test that converting back and forth maintains precision
    let original = 45.504167;
    let dms = decimal_to_dms(original);
    let converted_back = dm_to_decimal(dms.degrees, dms.minutes);
    assert_relative_eq!(converted_back, original, epsilon = 1e-5);
}

#[test]
fn test_round_trip_conversion_negative() {
    let original = -40.508333;
    let dms = decimal_to_dms(original);
    let converted_back = dm_to_decimal(dms.degrees, dms.minutes);
    assert_relative_eq!(converted_back, original, epsilon = 1e-5);
}

#[test]
fn test_round_trip_various_values() {
    let values = vec![0.0, 1.0, 45.0, 89.999, 90.0, 120.5, 179.999];
    for value in values {
        let dms = decimal_to_dms(value);
        let converted = dm_to_decimal(dms.degrees, dms.minutes);
        assert_relative_eq!(converted, value, epsilon = 1e-5);
    }
}

#[test]
fn test_precision_limits() {
    // Test decimal minutes precision
    // 45.504167° = 45° 30.25'
    let dms = decimal_to_dms(45.504167);
    assert_eq!(dms.degrees, 45);
    assert_relative_eq!(dms.minutes, 30.25, epsilon = 0.01);
    assert_eq!(dms.seconds, 0.0);
}

#[test]
fn test_edge_case_59_minutes_high() {
    // Test near degree boundary
    // 40° 59.999' = 40.99998°
    let decimal = dm_to_decimal(40, 59.999);
    assert_relative_eq!(decimal, 40.99998, epsilon = 1e-5);
}

#[test]
fn test_latitude_range_north() {
    // 40° 26.767' N (New York) - 46.0" = 0.767'
    let decimal = dm_to_decimal(40, 26.767);
    assert!(decimal >= 0.0 && decimal <= 90.0);
    assert_relative_eq!(decimal, 40.446111, epsilon = 1e-5);
}

#[test]
fn test_latitude_range_south() {
    // -33° 51.417' S (Sydney) - 25.0" = 0.417'
    let decimal = dm_to_decimal(-33, 51.417);
    assert!(decimal >= -90.0 && decimal <= 0.0);
    assert_relative_eq!(decimal, -33.856944, epsilon = 1e-5);
}

#[test]
fn test_longitude_range_east() {
    // 151° 12.433' E (Sydney) - 26.0" = 0.433'
    let decimal = dm_to_decimal(151, 12.433);
    assert!(decimal >= 0.0 && decimal <= 180.0);
    assert_relative_eq!(decimal, 151.207222, epsilon = 1e-5);
}

#[test]
fn test_longitude_range_west() {
    // -74° 0.383' W (New York) - 23.0" = 0.383'
    let decimal = dm_to_decimal(-74, 0.383);
    assert!(decimal >= -180.0 && decimal <= 0.0);
    assert_relative_eq!(decimal, -74.006389, epsilon = 1e-5);
}

#[test]
fn test_dms_display_format() {
    let dms = DMS {
        degrees: 45,
        minutes: 30.25,
        seconds: 0.0,
    };
    let formatted = format!("{}", dms);
    assert_eq!(formatted, "45° 30.25'");
}

#[test]
fn test_dms_display_format_negative() {
    let dms = DMS {
        degrees: -40,
        minutes: 30.5,
        seconds: 0.0,
    };
    let formatted = format!("{}", dms);
    assert_eq!(formatted, "-40° 30.50'");
}

#[test]
fn test_dms_display_format_zero_minutes() {
    let dms = DMS {
        degrees: 40,
        minutes: 0.0,
        seconds: 0.0,
    };
    let formatted = format!("{}", dms);
    assert_eq!(formatted, "40° 00.00'");
}
