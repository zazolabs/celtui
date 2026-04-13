// SPDX-License-Identifier: GPL-2.0-only
// SPDX-FileCopyrightText: Alexander Atanasov <alex@zazolabs.com>
//! Verify that the calculated fix actually lies on all three LOPs
//! This checks the geometry is correct

use celtnav::fix_calculation::{fix_from_multiple_lops, LineOfPosition};
use celtnav::dms_to_decimal;

#[test]
fn test_verify_fix_on_lops() {
    println!("\n=== VERIFY FIX GEOMETRY ===\n");

    // User's LOPs
    let pollux_ap_lat = 50.0;
    let pollux_ap_lon = -dms_to_decimal(20, 3, 12.0);
    let pollux_intercept = 7.0;
    let pollux_zn = 259.0;

    let hamal_ap_lat = 50.0;
    let hamal_ap_lon = -dms_to_decimal(20, 4, 24.0);
    let hamal_intercept = 11.3;
    let hamal_zn = 121.0;

    let deneb_ap_lat = 50.0;
    let deneb_ap_lon = -dms_to_decimal(19, 25, 42.0);
    let deneb_intercept = -5.5;
    let deneb_zn = 41.0;

    let lop1 = LineOfPosition {
        azimuth: pollux_zn,
        intercept: pollux_intercept,
        dr_latitude: pollux_ap_lat,
        dr_longitude: pollux_ap_lon,
    };

    let lop2 = LineOfPosition {
        azimuth: hamal_zn,
        intercept: hamal_intercept,
        dr_latitude: hamal_ap_lat,
        dr_longitude: hamal_ap_lon,
    };

    let lop3 = LineOfPosition {
        azimuth: deneb_zn,
        intercept: deneb_intercept,
        dr_latitude: deneb_ap_lat,
        dr_longitude: deneb_ap_lon,
    };

    let lops = vec![lop1, lop2, lop3];
    let fix = fix_from_multiple_lops(&lops).expect("Failed to compute fix");

    println!("Calculated fix: {:.4}°N, {:.4}°W", fix.position.latitude, fix.position.longitude.abs());
    println!("           ({}°{:.1}'N, {}°{:.1}'W)\n",
             fix.position.latitude.floor() as i32,
             (fix.position.latitude - fix.position.latitude.floor()) * 60.0,
             fix.position.longitude.abs().floor() as i32,
             (fix.position.longitude.abs() - fix.position.longitude.abs().floor()) * 60.0);

    // For each LOP, calculate the distance from the fix to the LOP
    // Distance = perpendicular distance from fix to the line
    for (i, lop) in lops.iter().enumerate() {
        let star_name = match i {
            0 => "Pollux",
            1 => "Hamal",
            2 => "Deneb",
            _ => "Unknown",
        };

        // Point on LOP (move from AP along azimuth by intercept)
        let azimuth_rad = lop.azimuth.to_radians();
        let lop_point_lat = lop.dr_latitude + (lop.intercept * azimuth_rad.cos()) / 60.0;
        let lop_point_lon = lop.dr_longitude + (lop.intercept * azimuth_rad.sin()) / (60.0 * lop.dr_latitude.to_radians().cos());

        // Distance from fix to LOP point
        let delta_lat = (fix.position.latitude - lop_point_lat) * 60.0;  // NM
        let delta_lon = (fix.position.longitude - lop_point_lon) * 60.0 * fix.position.latitude.to_radians().cos();  // NM

        // Normal to LOP (unit vector in direction of azimuth)
        let n_lat = azimuth_rad.cos();
        let n_lon = azimuth_rad.sin();

        // Perpendicular distance from fix to LOP
        let perp_distance = delta_lat * n_lat + delta_lon * n_lon;

        println!("{} LOP:", star_name);
        println!("  AP: {:.4}°N, {:.4}°W", lop.dr_latitude, lop.dr_longitude.abs());
        println!("  Intercept: {:.1} NM {}", lop.intercept.abs(), if lop.intercept >= 0.0 { "T" } else { "A" });
        println!("  Azimuth: {:.1}°", lop.azimuth);
        println!("  Point on LOP: {:.4}°N, {:.4}°W", lop_point_lat, lop_point_lon.abs());
        println!("  Distance from fix to LOP: {:.2} NM", perp_distance.abs());
        println!();
    }

    // User's expected fix
    let user_fix_lat = dms_to_decimal(49, 33, 0.0);
    let user_fix_lon = -dms_to_decimal(19, 58, 30.0);

    println!("User's expected fix: {:.4}°N, {:.4}°W", user_fix_lat, user_fix_lon.abs());
    println!("               (49°33.0'N, 19°58.5'W)\n");

    // Check how far the user's expected fix is from each LOP
    println!("Checking user's expected fix against LOPs:");
    for (i, lop) in lops.iter().enumerate() {
        let star_name = match i {
            0 => "Pollux",
            1 => "Hamal",
            2 => "Deneb",
            _ => "Unknown",
        };

        let azimuth_rad = lop.azimuth.to_radians();
        let lop_point_lat = lop.dr_latitude + (lop.intercept * azimuth_rad.cos()) / 60.0;
        let lop_point_lon = lop.dr_longitude + (lop.intercept * azimuth_rad.sin()) / (60.0 * lop.dr_latitude.to_radians().cos());

        let delta_lat = (user_fix_lat - lop_point_lat) * 60.0;
        let delta_lon = (user_fix_lon - lop_point_lon) * 60.0 * user_fix_lat.to_radians().cos();

        let n_lat = azimuth_rad.cos();
        let n_lon = azimuth_rad.sin();

        let perp_distance = delta_lat * n_lat + delta_lon * n_lon;

        println!("  {} LOP distance: {:.2} NM", star_name, perp_distance.abs());
    }

    println!("\nIf the user's fix is correct, all three distances should be close to 0 NM.");
    println!("If our calculated fix is correct, the distances in the first section should be close to 0 NM.");
}
