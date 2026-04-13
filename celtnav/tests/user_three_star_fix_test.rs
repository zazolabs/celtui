// SPDX-License-Identifier: GPL-2.0-only
// SPDX-FileCopyrightText: Alexander Atanasov <alex@zazolabs.com>
//! Test for user's three-star fix: Pollux, Hamal, Deneb
//! Expected fix: 49°33.0'N 019°58.5'W
//! Current result: 49°33.x'N 019°25.16'W (longitude is wrong)

use celtnav::almanac::{get_body_position, CelestialBody};
use celtnav::fix_calculation::{fix_from_multiple_lops, LineOfPosition};
use celtnav::sight_reduction::{compute_altitude, compute_azimuth, compute_intercept, SightData};
use celtnav::dms_to_decimal;
use chrono::{NaiveDate, NaiveTime, TimeZone, Utc};

#[test]
fn test_three_star_fix_pollux_hamal_deneb() {
    // Common parameters
    let dr_lat = dms_to_decimal(50, 5, 0.0);  // 50°05'N
    let dr_lon = -dms_to_decimal(19, 45, 0.0);  // 19°45'W (negative)
    let index_error = -4.6 / 60.0;  // -4.6' in degrees
    let height_of_eye = 2.5;  // meters

    println!("\n=== THREE STAR FIX TEST ===");
    println!("DR Position: {:.4}°N, {:.4}°W", dr_lat, dr_lon.abs());
    println!("Date: 2015-09-10");
    println!("Index Error: {:.1}', Height of Eye: {:.1}m\n", index_error * 60.0, height_of_eye);

    // SIGHT 1: Pollux at 06:28:18 UTC
    let pollux_hs = dms_to_decimal(46, 19, 18.0);  // 46°19.3' = 46°19'18"
    let pollux_time = Utc.from_utc_datetime(
        &NaiveDate::from_ymd_opt(2015, 9, 10).unwrap()
            .and_time(NaiveTime::from_hms_opt(6, 28, 18).unwrap())
    );

    let pollux_pos = get_body_position(CelestialBody::Star("Pollux".to_string()), pollux_time)
        .expect("Failed to get Pollux position");

    // Apply corrections to get Ho
    let mut pollux_ho = pollux_hs;
    pollux_ho += index_error;  // Index error
    pollux_ho += apply_dip_correction(height_of_eye);  // Dip
    pollux_ho += apply_refraction_correction(pollux_ho);  // Refraction

    let pollux_lha = (pollux_pos.gha + dr_lon + 360.0) % 360.0;
    let pollux_sight_data = SightData {
        latitude: dr_lat,
        declination: pollux_pos.declination,
        local_hour_angle: pollux_lha,
    };

    let pollux_hc = compute_altitude(&pollux_sight_data);
    let pollux_zn = compute_azimuth(&pollux_sight_data);
    let pollux_intercept = compute_intercept(&pollux_sight_data, pollux_ho);

    println!("POLLUX @ 06:28:18 UTC");
    println!("  Hs: {:.4}° ({:.1}')", pollux_hs, pollux_hs * 60.0);
    println!("  Ho: {:.4}° ({:.1}')", pollux_ho, pollux_ho * 60.0);
    println!("  GHA: {:.4}°, Dec: {:.4}°", pollux_pos.gha, pollux_pos.declination);
    println!("  LHA: {:.4}°", pollux_lha);
    println!("  Hc: {:.4}° ({:.1}')", pollux_hc, pollux_hc * 60.0);
    println!("  Zn: {:.1}°", pollux_zn);
    println!("  Intercept: {:.2} NM", pollux_intercept);

    let lop1 = LineOfPosition {
        azimuth: pollux_zn,
        intercept: pollux_intercept,
        dr_latitude: dr_lat,
        dr_longitude: dr_lon,
    };

    // SIGHT 2: Hamal at 06:32:22 UTC
    let hamal_hs = dms_to_decimal(52, 22, 42.0);  // 52°22.7' = 52°22'42"
    let hamal_time = Utc.from_utc_datetime(
        &NaiveDate::from_ymd_opt(2015, 9, 10).unwrap()
            .and_time(NaiveTime::from_hms_opt(6, 32, 22).unwrap())
    );

    let hamal_pos = get_body_position(CelestialBody::Star("Hamal".to_string()), hamal_time)
        .expect("Failed to get Hamal position");

    let mut hamal_ho = hamal_hs;
    hamal_ho += index_error;
    hamal_ho += apply_dip_correction(height_of_eye);
    hamal_ho += apply_refraction_correction(hamal_ho);

    let hamal_lha = (hamal_pos.gha + dr_lon + 360.0) % 360.0;
    let hamal_sight_data = SightData {
        latitude: dr_lat,
        declination: hamal_pos.declination,
        local_hour_angle: hamal_lha,
    };

    let hamal_hc = compute_altitude(&hamal_sight_data);
    let hamal_zn = compute_azimuth(&hamal_sight_data);
    let hamal_intercept = compute_intercept(&hamal_sight_data, hamal_ho);

    println!("\nHAMAL @ 06:32:22 UTC");
    println!("  Hs: {:.4}° ({:.1}')", hamal_hs, hamal_hs * 60.0);
    println!("  Ho: {:.4}° ({:.1}')", hamal_ho, hamal_ho * 60.0);
    println!("  GHA: {:.4}°, Dec: {:.4}°", hamal_pos.gha, hamal_pos.declination);
    println!("  LHA: {:.4}°", hamal_lha);
    println!("  Hc: {:.4}° ({:.1}')", hamal_hc, hamal_hc * 60.0);
    println!("  Zn: {:.1}°", hamal_zn);
    println!("  Intercept: {:.2} NM", hamal_intercept);

    let lop2 = LineOfPosition {
        azimuth: hamal_zn,
        intercept: hamal_intercept,
        dr_latitude: dr_lat,
        dr_longitude: dr_lon,
    };

    // SIGHT 3: Deneb at 06:33:47 UTC
    let deneb_hs = dms_to_decimal(19, 42, 36.0);  // 19°42.6' = 19°42'36"
    let deneb_time = Utc.from_utc_datetime(
        &NaiveDate::from_ymd_opt(2015, 9, 10).unwrap()
            .and_time(NaiveTime::from_hms_opt(6, 33, 47).unwrap())
    );

    let deneb_pos = get_body_position(CelestialBody::Star("Deneb".to_string()), deneb_time)
        .expect("Failed to get Deneb position");

    let mut deneb_ho = deneb_hs;
    deneb_ho += index_error;
    deneb_ho += apply_dip_correction(height_of_eye);
    deneb_ho += apply_refraction_correction(deneb_ho);

    let deneb_lha = (deneb_pos.gha + dr_lon + 360.0) % 360.0;
    let deneb_sight_data = SightData {
        latitude: dr_lat,
        declination: deneb_pos.declination,
        local_hour_angle: deneb_lha,
    };

    let deneb_hc = compute_altitude(&deneb_sight_data);
    let deneb_zn = compute_azimuth(&deneb_sight_data);
    let deneb_intercept = compute_intercept(&deneb_sight_data, deneb_ho);

    println!("\nDENEB @ 06:33:47 UTC");
    println!("  Hs: {:.4}° ({:.1}')", deneb_hs, deneb_hs * 60.0);
    println!("  Ho: {:.4}° ({:.1}')", deneb_ho, deneb_ho * 60.0);
    println!("  GHA: {:.4}°, Dec: {:.4}°", deneb_pos.gha, deneb_pos.declination);
    println!("  LHA: {:.4}°", deneb_lha);
    println!("  Hc: {:.4}° ({:.1}')", deneb_hc, deneb_hc * 60.0);
    println!("  Zn: {:.1}°", deneb_zn);
    println!("  Intercept: {:.2} NM", deneb_intercept);

    let lop3 = LineOfPosition {
        azimuth: deneb_zn,
        intercept: deneb_intercept,
        dr_latitude: dr_lat,
        dr_longitude: dr_lon,
    };

    // Calculate fix
    let lops = vec![lop1, lop2, lop3];
    let fix = fix_from_multiple_lops(&lops).expect("Failed to compute fix");

    println!("\n=== FIX RESULT ===");
    println!("Calculated: {:.4}°N, {:.4}°W", fix.position.latitude, fix.position.longitude.abs());
    println!("           ({:.1}'N, {:.1}'W)",
             fix.position.latitude * 60.0,
             fix.position.longitude.abs() * 60.0);

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
}

// Helper functions for corrections
fn apply_dip_correction(height_of_eye_meters: f64) -> f64 {
    // Dip correction in degrees (always negative)
    // Formula: -0.0293 * sqrt(height in meters)
    -0.0293 * height_of_eye_meters.sqrt()
}

fn apply_refraction_correction(altitude_deg: f64) -> f64 {
    // Refraction correction in degrees (always negative)
    // Simplified formula for standard conditions
    let altitude_rad = altitude_deg.to_radians();
    -0.0167 / altitude_rad.tan()
}
