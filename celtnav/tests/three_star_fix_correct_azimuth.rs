// SPDX-License-Identifier: GPL-2.0-only
// SPDX-FileCopyrightText: Alexander Atanasov <alex@zazolabs.com>
//! Three-star fix with CORRECT azimuths (no conversion needed for Pub.249 Vol.1)
//! Pub.249 Vol.1 gives Z as true azimuth (Zn) directly - DO NOT convert!

use celtnav::fix_calculation::{fix_from_multiple_lops, LineOfPosition};
use celtnav::dms_to_decimal;

#[test]
fn test_three_star_fix_correct_azimuths() {
    println!("\n=== THREE STAR FIX - CORRECT AZIMUTHS ===");
    println!("Pub.249 Vol.1 gives Z as Zn (true azimuth) - no conversion needed!");
    println!("Expected fix: 49°33.0'N 019°58.5'W\n");

    // User's LOPs with CORRECT azimuths (Z from Pub.249 Vol.1 IS Zn)

    // Pollux: LHA Aries = 66°, AP = 50°N 20°03.2'W
    let pollux_ap_lat = 50.0;
    let pollux_ap_lon = -dms_to_decimal(20, 3, 12.0);  // 20°03.2'W
    let pollux_intercept = 7.0;  // 7 NM Toward
    let pollux_zn = 101.0;  // Z=101° IS the true azimuth (E-SE)

    println!("POLLUX:");
    println!("  AP: 50°N, 20°03.2'W");
    println!("  Intercept: 7.0 T");
    println!("  Zn: {:.1}° (E-SE)", pollux_zn);

    let lop1 = LineOfPosition {
        azimuth: pollux_zn,
        intercept: pollux_intercept,
        dr_latitude: pollux_ap_lat,
        dr_longitude: pollux_ap_lon,
    };

    // Hamal: LHA Aries = 67°, AP = 50°N 20°04.4'W
    let hamal_ap_lat = 50.0;
    let hamal_ap_lon = -dms_to_decimal(20, 4, 24.0);  // 20°04.4'W
    let hamal_intercept = 11.3;  // 11.3 NM Toward
    let hamal_zn = 239.0;  // Z=239° IS the true azimuth (W-SW)

    println!("\nHAMAL:");
    println!("  AP: 50°N, 20°04.4'W");
    println!("  Intercept: 11.3 T");
    println!("  Zn: {:.1}° (W-SW)", hamal_zn);

    let lop2 = LineOfPosition {
        azimuth: hamal_zn,
        intercept: hamal_intercept,
        dr_latitude: hamal_ap_lat,
        dr_longitude: hamal_ap_lon,
    };

    // Deneb: LHA Aries = 68°, AP = 50°N 19°25.7'W
    let deneb_ap_lat = 50.0;
    let deneb_ap_lon = -dms_to_decimal(19, 25, 42.0);  // 19°25.7'W
    let deneb_intercept = -5.5;  // 5.5 NM Away (negative)
    let deneb_zn = 319.0;  // Z=319° IS the true azimuth (NW)

    println!("\nDENEB:");
    println!("  AP: 50°N, 19°25.7'W");
    println!("  Intercept: 5.5 A");
    println!("  Zn: {:.1}° (NW)", deneb_zn);

    let lop3 = LineOfPosition {
        azimuth: deneb_zn,
        intercept: deneb_intercept,
        dr_latitude: deneb_ap_lat,
        dr_longitude: deneb_ap_lon,
    };

    println!("\nGeometry check:");
    println!("  Pollux (Zn 101°, E-SE) - 7 NM toward");
    println!("  Hamal (Zn 239°, W-SW) - 11.3 NM toward");
    println!("  Deneb (Zn 319°, NW) - 5.5 NM away");
    println!("  Expected: LOPs intersect SW of APs (lower-left quadrant)");

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

    // Verify fix lies on LOPs
    println!("\n=== VERIFICATION: Distance from fix to each LOP ===");
    for (i, lop) in lops.iter().enumerate() {
        let star_name = match i {
            0 => "Pollux",
            1 => "Hamal",
            2 => "Deneb",
            _ => "Unknown",
        };

        // Point on LOP using plane sailing
        let azimuth_rad = lop.azimuth.to_radians();
        let cos_lat = lop.dr_latitude.to_radians().cos();

        let offset_lat = (lop.intercept * azimuth_rad.cos()) / 60.0;
        let offset_lon = (lop.intercept * azimuth_rad.sin()) / (60.0 * cos_lat);

        let lop_point_lat = lop.dr_latitude + offset_lat;
        let lop_point_lon = lop.dr_longitude + offset_lon;

        // Distance from fix to LOP point
        let delta_lat = (fix.position.latitude - lop_point_lat) * 60.0;
        let delta_lon = (fix.position.longitude - lop_point_lon) * 60.0 * fix.position.latitude.to_radians().cos();

        // Normal to LOP
        let n_lat = azimuth_rad.cos();
        let n_lon = azimuth_rad.sin();

        // Perpendicular distance
        let perp_distance = delta_lat * n_lat + delta_lon * n_lon;

        println!("  {}: {:.2} NM (should be ~0)", star_name, perp_distance.abs());
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
