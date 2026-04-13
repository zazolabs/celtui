// SPDX-License-Identifier: GPL-2.0-only
// SPDX-FileCopyrightText: Alexander Atanasov <alex@zazolabs.com>
//! Three-star fix using user's exact intercept and azimuth values
//! This verifies that the fix calculation itself is working correctly

use celtnav::fix_calculation::{fix_from_multiple_lops, LineOfPosition};
use celtnav::dms_to_decimal;

#[test]
fn test_three_star_fix_with_user_intercepts() {
    println!("\n=== THREE STAR FIX - USER'S INTERCEPT VALUES ===");
    println!("Using exact values from user's manual calculation");
    println!("Expected fix: 49°33.0'N 019°58.5'W\n");

    // User's assumed positions for each star
    // All use AP Lat = 50°N, but different AP Lon to make LHA Aries whole

    // Pollux: LHA Aries = 66°, AP Lon = 20°03.2'W
    let pollux_ap_lat = 50.0;
    let pollux_ap_lon = -dms_to_decimal(20, 3, 12.0);  // 20°03.2'W
    let pollux_intercept = 7.0;  // 7 NM Toward
    let pollux_zn = 101.0;  // Zn from Pub.249 Vol.1 (already true azimuth)

    println!("POLLUX:");
    println!("  AP: 50°N, 20°03.2'W");
    println!("  Intercept: 7.0 T");
    println!("  Zn: 101°");

    let lop1 = LineOfPosition {
        azimuth: pollux_zn,
        intercept: pollux_intercept,
        dr_latitude: pollux_ap_lat,
        dr_longitude: pollux_ap_lon,
    };

    // Hamal: LHA Aries = 67°, AP Lon = 20°04.4'W
    let hamal_ap_lat = 50.0;
    let hamal_ap_lon = -dms_to_decimal(20, 4, 24.0);  // 20°04.4'W
    let hamal_intercept = 11.3;  // 11.3 NM Toward
    let hamal_zn = 239.0;  // Zn from Pub.249 Vol.1 (already true azimuth)

    println!("\nHAMAL:");
    println!("  AP: 50°N, 20°04.4'W");
    println!("  Intercept: 11.3 T");
    println!("  Zn: 239°");

    let lop2 = LineOfPosition {
        azimuth: hamal_zn,
        intercept: hamal_intercept,
        dr_latitude: hamal_ap_lat,
        dr_longitude: hamal_ap_lon,
    };

    // Deneb: LHA Aries = 68°, AP Lon = 19°25.7'W
    let deneb_ap_lat = 50.0;
    let deneb_ap_lon = -dms_to_decimal(19, 25, 42.0);  // 19°25.7'W
    let deneb_intercept = -5.5;  // 5.5 NM Away (negative)
    let deneb_zn = 319.0;  // Zn from Pub.249 Vol.1 (already true azimuth)

    println!("\nDENEB:");
    println!("  AP: 50°N, 19°25.7'W");
    println!("  Intercept: 5.5 A");
    println!("  Zn: 319°");

    let lop3 = LineOfPosition {
        azimuth: deneb_zn,
        intercept: deneb_intercept,
        dr_latitude: deneb_ap_lat,
        dr_longitude: deneb_ap_lon,
    };

    // Calculate fix
    let lops = vec![lop1, lop2, lop3];
    let fix = fix_from_multiple_lops(&lops).expect("Failed to compute fix");

    println!("\n=== FIX RESULT ===");
    println!("Calculated: {:.4}°N, {:.4}°W", fix.position.latitude, fix.position.longitude.abs());

    let calc_lat_deg = fix.position.latitude.floor() as i32;
    let calc_lat_min = (fix.position.latitude - calc_lat_deg as f64) * 60.0;
    let calc_lon_deg = fix.position.longitude.abs().floor() as i32;
    let calc_lon_min = (fix.position.longitude.abs() - calc_lon_deg as f64) * 60.0;

    println!("           {}°{:.1}'N, {}°{:.1}'W",
             calc_lat_deg, calc_lat_min,
             calc_lon_deg, calc_lon_min);

    // Expected fix: 49°33.0'N 019°58.5'W
    let expected_lat = dms_to_decimal(49, 33, 0.0);
    let expected_lon = -dms_to_decimal(19, 58, 30.0);

    println!("\nExpected:   {:.4}°N, {:.4}°W", expected_lat, expected_lon.abs());
    println!("           49°33.0'N, 19°58.5'W");

    let lat_error_nm = (fix.position.latitude - expected_lat).abs() * 60.0;
    let lon_error_nm = (fix.position.longitude - expected_lon).abs() * 60.0 * fix.position.latitude.to_radians().cos();

    println!("\nErrors:");
    println!("  Latitude: {:.2} NM", lat_error_nm);
    println!("  Longitude: {:.2} NM", lon_error_nm);
    println!("  Total: {:.2} NM", (lat_error_nm * lat_error_nm + lon_error_nm * lon_error_nm).sqrt());

    if let Some(accuracy) = fix.accuracy_estimate {
        println!("  RMS residual: {:.2} NM", accuracy);
    }

    // Assert within 2.5 NM as user specified
    assert!(
        lat_error_nm < 2.5,
        "Latitude error too large: {:.2} NM (expected < 2.5 NM)",
        lat_error_nm
    );

    assert!(
        lon_error_nm < 2.5,
        "Longitude error too large: {:.2} NM (expected < 2.5 NM)",
        lon_error_nm
    );

    println!("\n✓ Fix calculation is accurate within 2.5 NM!");
}
