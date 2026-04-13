// SPDX-License-Identifier: GPL-2.0-only
// SPDX-FileCopyrightText: Alexander Atanasov <alex@zazolabs.com>
//! Verify Hamal calculation against user's table values

use celtnav::almanac::{get_body_position, CelestialBody, find_star};
use celtnav::sight_reduction::{compute_altitude, compute_azimuth, SightData};
use celtnav::dms_to_decimal;
use chrono::{NaiveDate, NaiveTime, TimeZone, Utc};

#[test]
fn test_hamal_lha_aries_verification() {
    // User's data for Hamal @ 06:32:22 UTC on 2015-09-10
    let dr_lat = dms_to_decimal(50, 5, 0.0);  // 50°05'N
    let dr_lon = -dms_to_decimal(19, 45, 0.0);  // 19°45'W

    let hamal_time = Utc.from_utc_datetime(
        &NaiveDate::from_ymd_opt(2015, 9, 10).unwrap()
            .and_time(NaiveTime::from_hms_opt(6, 32, 22).unwrap())
    );

    // Get Hamal position (GHA Hamal)
    let hamal_pos = get_body_position(CelestialBody::Star("Hamal".to_string()), hamal_time)
        .expect("Failed to get Hamal position");

    // Get Hamal from star catalog
    let hamal_star = find_star("Hamal").expect("Hamal not found");

    println!("\n=== HAMAL VERIFICATION ===");
    println!("Time: 2015-09-10 06:32:22 UTC");
    println!("DR: {:.4}°N, {:.4}°W\n", dr_lat, dr_lon.abs());

    println!("From our catalog (epoch 2024):");
    println!("  SHA: {:.3}° ({:.1}')", hamal_star.sha, hamal_star.sha * 60.0);
    println!("  Dec: {:.3}° ({:.1}')", hamal_star.declination, hamal_star.declination * 60.0);

    println!("\nUser's values:");
    println!("  Dec: 23° 32.1' N");
    println!("  LHA Aries: 67°");
    println!("  Hc: 52° 04'");
    println!("  Z: 239° → Zn: 121°");

    println!("\nComputed from almanac:");
    println!("  GHA Hamal: {:.4}° ({:.1}')", hamal_pos.gha, hamal_pos.gha * 60.0);
    println!("  Dec Hamal: {:.4}° ({:.1}')", hamal_pos.declination, hamal_pos.declination * 60.0);

    // Calculate GHA Aries from GHA Hamal and SHA
    let gha_aries = (hamal_pos.gha - hamal_star.sha + 360.0) % 360.0;
    println!("\n  GHA Aries (computed): {:.4}° ({:.1}')", gha_aries, gha_aries * 60.0);

    // Calculate LHA Aries
    let lha_aries = (gha_aries + dr_lon + 360.0) % 360.0;
    println!("  LHA Aries: {:.4}° (rounds to {}°)", lha_aries, lha_aries.round() as i32);

    // Calculate LHA Hamal
    let lha_hamal = (hamal_pos.gha + dr_lon + 360.0) % 360.0;
    println!("  LHA Hamal: {:.4}°", lha_hamal);

    // Compute Hc and Zn using our spherical trig
    let sight_data = SightData {
        latitude: dr_lat,
        declination: hamal_pos.declination,
        local_hour_angle: lha_hamal,
    };

    let hc = compute_altitude(&sight_data);
    let zn = compute_azimuth(&sight_data);

    println!("\nUsing spherical trigonometry:");
    println!("  Hc: {:.4}° ({:.1}')", hc, hc * 60.0);
    println!("  Zn: {:.1}°", zn);

    // Now compute using user's declination value
    let user_dec = dms_to_decimal(23, 32, 6.0);  // 23° 32.1'
    let sight_data_user_dec = SightData {
        latitude: dr_lat,
        declination: user_dec,
        local_hour_angle: lha_hamal,
    };

    let hc_user_dec = compute_altitude(&sight_data_user_dec);

    println!("\nUsing user's declination (23° 32.1' N):");
    println!("  Hc: {:.4}° ({:.1}')", hc_user_dec, hc_user_dec * 60.0);

    println!("\nComparison:");
    println!("  Expected LHA Aries: 67°");
    println!("  Calculated LHA Aries: {:.0}°", lha_aries.round());
    println!("  Match: {}", (lha_aries - 67.0).abs() < 0.5);

    println!("\n  Expected Hc: 52° 04' (52.0667°)");
    println!("  Our catalog Hc: {:.1}' ({:.4}°)", hc * 60.0, hc);
    println!("  User dec Hc: {:.1}' ({:.4}°)", hc_user_dec * 60.0, hc_user_dec);

    let expected_hc = dms_to_decimal(52, 4, 0.0);
    println!("  Difference (our cat): {:.1}' ({:.2} NM)", (hc - expected_hc) * 60.0, (hc - expected_hc) * 60.0);
    println!("  Difference (user dec): {:.1}' ({:.2} NM)", (hc_user_dec - expected_hc) * 60.0, (hc_user_dec - expected_hc) * 60.0);

    // Check if LHA Aries matches
    assert!((lha_aries - 67.0).abs() < 0.5, "LHA Aries should be 67°, got {:.1}°", lha_aries);
}
