// SPDX-License-Identifier: GPL-2.0-only
// SPDX-FileCopyrightText: Alexander Atanasov <alex@zazolabs.com>
//! Tests for fix calculation from lines of position (LOPs)
//!
//! These tests verify that the fix calculation correctly accounts for
//! latitude scaling when computing LOP intersections.

use celtnav::fix_calculation::{fix_from_two_lops, fix_from_multiple_lops, LineOfPosition};

#[test]
fn test_two_lop_fix_with_latitude_scaling() {
    // This test demonstrates the latitude scaling bug
    //
    // At 50°N, cos(50°) ≈ 0.643, so 1° longitude ≈ 38.6 NM instead of 60 NM
    //
    // We'll create two LOPs at 50°N, 20°W:
    // 1. LOP1: Running E-W (perpendicular to azimuth 0° = North)
    //    - Azimuth to body: 0° (North)
    //    - Intercept: +5 NM (toward the body)
    //    - Point on LOP: 50° 05'N, 20°W (5 NM north of DR)
    //    - LOP runs E-W through this point
    //
    // 2. LOP2: Running N-S (perpendicular to azimuth 90° = East)
    //    - Azimuth to body: 90° (East)
    //    - Intercept: +5 NM (toward the body)
    //    - Point on LOP: 50°N, 19° 52.3'W (5 NM east of DR at 50°N)
    //    - LOP runs N-S through this point
    //
    // At 50°N, 5 NM east means:
    //   5 NM / (60 NM/deg × cos(50°)) = 5 / (60 × 0.643) = 5 / 38.58 ≈ 0.1296° ≈ 7.7'
    //   So longitude = -20° + 0.1296° = -19.8704° = 19° 52.2'W
    //
    // The two LOPs should intersect at: 50° 05'N, 19° 52.2'W

    let lop1 = LineOfPosition {
        azimuth: 0.0,    // North
        intercept: 5.0,  // 5 NM toward
        dr_latitude: 50.0,
        dr_longitude: -20.0,
    };

    let lop2 = LineOfPosition {
        azimuth: 90.0,   // East
        intercept: 5.0,  // 5 NM toward
        dr_latitude: 50.0,
        dr_longitude: -20.0,
    };

    let fix = fix_from_two_lops(&lop1, &lop2).expect("Should calculate fix");

    // Expected fix position
    let expected_lat = 50.0 + 5.0 / 60.0;  // 50° 05'N = 50.0833°N
    let expected_lon = -20.0 + 5.0 / (60.0 * 50.0_f64.to_radians().cos());  // ≈ -19.8704°

    println!("Expected fix: {:.4}°N, {:.4}°W", expected_lat, expected_lon.abs());
    println!("Calculated fix: {:.4}°N, {:.4}°W",
             fix.position.latitude, fix.position.longitude.abs());

    // The fix should be within 0.1 NM (0.0017°) of the expected position
    let lat_error = (fix.position.latitude - expected_lat).abs();
    let lon_error = (fix.position.longitude - expected_lon).abs();

    // Convert errors to nautical miles for better readability
    let lat_error_nm = lat_error * 60.0;
    let lon_error_nm = lon_error * 60.0 * fix.position.latitude.to_radians().cos();

    println!("Latitude error: {:.3} NM", lat_error_nm);
    println!("Longitude error: {:.3} NM", lon_error_nm);

    assert!(
        lat_error < 0.0017,
        "Latitude error too large: {:.3} NM (expected < 0.1 NM)",
        lat_error_nm
    );

    assert!(
        lon_error < 0.0017,
        "Longitude error too large: {:.3} NM (expected < 0.1 NM)",
        lon_error_nm
    );
}

#[test]
fn test_three_lop_fix_with_latitude_scaling() {
    // Test the least squares method with 3 LOPs at high latitude
    // Same setup as above, plus a third LOP from NE

    let lop1 = LineOfPosition {
        azimuth: 0.0,    // North
        intercept: 5.0,
        dr_latitude: 50.0,
        dr_longitude: -20.0,
    };

    let lop2 = LineOfPosition {
        azimuth: 90.0,   // East
        intercept: 5.0,
        dr_latitude: 50.0,
        dr_longitude: -20.0,
    };

    let lop3 = LineOfPosition {
        azimuth: 45.0,   // NE
        intercept: 7.07, // √(5² + 5²) ≈ 7.07 NM toward NE
        dr_latitude: 50.0,
        dr_longitude: -20.0,
    };

    let fix = fix_from_multiple_lops(&[lop1, lop2, lop3]).expect("Should calculate fix");

    // Expected fix position (same as 2-LOP case)
    let expected_lat = 50.0 + 5.0 / 60.0;
    let expected_lon = -20.0 + 5.0 / (60.0 * 50.0_f64.to_radians().cos());

    println!("Expected fix: {:.4}°N, {:.4}°W", expected_lat, expected_lon.abs());
    println!("Calculated fix: {:.4}°N, {:.4}°W",
             fix.position.latitude, fix.position.longitude.abs());

    let lat_error = (fix.position.latitude - expected_lat).abs();
    let lon_error = (fix.position.longitude - expected_lon).abs();

    let lat_error_nm = lat_error * 60.0;
    let lon_error_nm = lon_error * 60.0 * fix.position.latitude.to_radians().cos();

    println!("Latitude error: {:.3} NM", lat_error_nm);
    println!("Longitude error: {:.3} NM", lon_error_nm);

    // Least squares should be even more accurate
    assert!(
        lat_error < 0.0017,
        "Latitude error too large: {:.3} NM (expected < 0.1 NM)",
        lat_error_nm
    );

    assert!(
        lon_error < 0.0017,
        "Longitude error too large: {:.3} NM (expected < 0.1 NM)",
        lon_error_nm
    );
}
