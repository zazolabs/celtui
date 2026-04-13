// SPDX-License-Identifier: GPL-2.0-only
// SPDX-FileCopyrightText: Alexander Atanasov <alex@zazolabs.com>
//! Three-star fix using correct Assumed Positions (AP) for Pub.249 Vol.1
//!
//! For Pub.249 Vol.1 star tables:
//! - Assumed Latitude: Rounded to nearest whole degree (50°N)
//! - Assumed Longitude: Optimized to make LHA Aries a whole number
//!
//! This is different from using DR position directly!

use celtnav::fix_calculation::{fix_from_multiple_lops, LineOfPosition};
use celtnav::sight_reduction::{compute_altitude, compute_azimuth, compute_intercept, SightData};
use celtnav::dms_to_decimal;
use celtnav::almanac::gha_aries;
use chrono::{NaiveDate, NaiveTime, TimeZone, Utc};

#[test]
fn test_three_star_fix_with_assumed_positions() {
    // DR position (dead reckoning)
    let dr_lat = dms_to_decimal(50, 5, 0.0);  // 50°05'N
    let dr_lon = -dms_to_decimal(19, 45, 0.0);  // 19°45'W

    // Assumed Position (AP) for Pub.249 Vol.1
    // AP Latitude: Round to nearest whole degree
    let ap_lat = 50.0;  // 50°00'N (exactly)

    let index_error = -4.6 / 60.0;  // -4.6' in degrees
    let height_of_eye = 2.5;  // meters

    println!("\n=== THREE STAR FIX - USING ASSUMED POSITIONS ===");
    println!("DR Position:  {:.4}°N, {:.4}°W (50°05'N, 19°45'W)", dr_lat, dr_lon.abs());
    println!("AP Latitude:  {:.4}°N (50°00'N exactly)", ap_lat);
    println!("Date: 2015-09-10");
    println!("Index Error: {:.1}', Height of Eye: {:.1}m\n", index_error * 60.0, height_of_eye);

    // 2015 almanac star data
    let pollux_sha_2015 = dms_to_decimal(243, 26, 6.0);   // 243°26.1'
    let pollux_dec_2015 = dms_to_decimal(27, 59, 6.0);    // 27°59.1' N

    let hamal_sha_2015 = dms_to_decimal(327, 58, 48.0);   // 327°58.8'
    let hamal_dec_2015 = dms_to_decimal(23, 32, 6.0);     // 23°32.1' N

    let deneb_sha_2015 = dms_to_decimal(49, 30, 0.0);     // 49°30.0'
    let deneb_dec_2015 = dms_to_decimal(45, 20, 36.0);    // 45°20.6' N

    println!("2015 Almanac Star Data:");
    println!("  Pollux: SHA 243°26.1', Dec 27°59.1' N");
    println!("  Hamal:  SHA 327°58.8', Dec 23°32.1' N");
    println!("  Deneb:  SHA 49°30.0',  Dec 45°20.6' N\n");

    // SIGHT 1: Pollux at 06:28:18 UTC
    let pollux_hs = dms_to_decimal(46, 19, 18.0);  // 46°19.3'
    let pollux_time = Utc.from_utc_datetime(
        &NaiveDate::from_ymd_opt(2015, 9, 10).unwrap()
            .and_time(NaiveTime::from_hms_opt(6, 28, 18).unwrap())
    );

    let gha_aries_pollux = gha_aries(pollux_time);

    // Optimize longitude to make LHA Aries a whole number
    let lha_aries_with_dr = (gha_aries_pollux + dr_lon + 360.0) % 360.0;
    let lha_aries_whole = lha_aries_with_dr.round();
    let ap_lon_pollux = (lha_aries_whole - gha_aries_pollux + 360.0) % 360.0;
    let ap_lon_pollux = if ap_lon_pollux > 180.0 { ap_lon_pollux - 360.0 } else { ap_lon_pollux };

    let gha_pollux = (gha_aries_pollux + pollux_sha_2015) % 360.0;
    let pollux_lha = (gha_pollux + ap_lon_pollux + 360.0) % 360.0;

    // Apply corrections
    let mut pollux_ho = pollux_hs;
    pollux_ho += index_error;
    pollux_ho += apply_dip_correction(height_of_eye);
    pollux_ho += apply_refraction_correction(pollux_ho);

    let pollux_sight_data = SightData {
        latitude: ap_lat,  // Use AP latitude!
        declination: pollux_dec_2015,
        local_hour_angle: pollux_lha,
    };

    let pollux_hc = compute_altitude(&pollux_sight_data);
    let pollux_zn = compute_azimuth(&pollux_sight_data);
    let pollux_intercept = compute_intercept(&pollux_sight_data, pollux_ho);

    println!("POLLUX @ 06:28:18 UTC");
    println!("  GHA Aries: {:.4}°", gha_aries_pollux);
    println!("  AP Lon: {:.4}°W ({}°{:.1}'W) [optimized for LHA Aries = {}°]",
             ap_lon_pollux.abs(), ap_lon_pollux.abs().floor() as i32,
             (ap_lon_pollux.abs() - ap_lon_pollux.abs().floor()) * 60.0,
             lha_aries_whole as i32);
    println!("  LHA Aries: {:.1}°", lha_aries_whole);
    println!("  AP: 50°N, {:.4}°W", ap_lon_pollux.abs());
    println!("  Hs: {:.1}', Ho: {:.1}'", pollux_hs * 60.0, pollux_ho * 60.0);
    println!("  Hc: {:.1}' ({:.4}°)", pollux_hc * 60.0, pollux_hc);
    println!("  Zn: {:.1}°", pollux_zn);
    println!("  Intercept: {:.2} NM", pollux_intercept);

    let lop1 = LineOfPosition {
        azimuth: pollux_zn,
        intercept: pollux_intercept,
        dr_latitude: ap_lat,       // Use AP for LOP reference!
        dr_longitude: ap_lon_pollux,
    };

    // SIGHT 2: Hamal at 06:32:22 UTC
    let hamal_hs = dms_to_decimal(52, 22, 42.0);  // 52°22.7'
    let hamal_time = Utc.from_utc_datetime(
        &NaiveDate::from_ymd_opt(2015, 9, 10).unwrap()
            .and_time(NaiveTime::from_hms_opt(6, 32, 22).unwrap())
    );

    let gha_aries_hamal = gha_aries(hamal_time);

    // User said: "Chosen/Assumed position for Hamal is 20° 04.4 to get LHA 67 for Aries"
    let ap_lon_hamal = -dms_to_decimal(20, 4, 24.0);  // 20°04.4'W

    let gha_hamal = (gha_aries_hamal + hamal_sha_2015) % 360.0;
    let hamal_lha = (gha_hamal + ap_lon_hamal + 360.0) % 360.0;
    let hamal_lha_aries = (gha_aries_hamal + ap_lon_hamal + 360.0) % 360.0;

    let mut hamal_ho = hamal_hs;
    hamal_ho += index_error;
    hamal_ho += apply_dip_correction(height_of_eye);
    hamal_ho += apply_refraction_correction(hamal_ho);

    let hamal_sight_data = SightData {
        latitude: ap_lat,  // Use AP latitude!
        declination: hamal_dec_2015,
        local_hour_angle: hamal_lha,
    };

    let hamal_hc = compute_altitude(&hamal_sight_data);
    let hamal_zn = compute_azimuth(&hamal_sight_data);
    let hamal_intercept = compute_intercept(&hamal_sight_data, hamal_ho);

    println!("\nHAMAL @ 06:32:22 UTC");
    println!("  GHA Aries: {:.4}°", gha_aries_hamal);
    println!("  AP Lon: 20°04.4'W (user specified)");
    println!("  LHA Aries: {:.1}°", hamal_lha_aries);
    println!("  AP: 50°N, 20°04.4'W");
    println!("  Hs: {:.1}', Ho: {:.1}'", hamal_hs * 60.0, hamal_ho * 60.0);
    println!("  Hc: {:.1}' ({:.4}°)", hamal_hc * 60.0, hamal_hc);
    println!("  Zn: {:.1}°", hamal_zn);
    println!("  Intercept: {:.2} NM", hamal_intercept);
    println!("  User's Hc from Pub.249: 52°04' (Zn 121°)");
    println!("  Difference: {:.1}'", (hamal_hc - dms_to_decimal(52, 4, 0.0)) * 60.0);

    let lop2 = LineOfPosition {
        azimuth: hamal_zn,
        intercept: hamal_intercept,
        dr_latitude: ap_lat,
        dr_longitude: ap_lon_hamal,
    };

    // SIGHT 3: Deneb at 06:33:47 UTC
    let deneb_hs = dms_to_decimal(19, 42, 36.0);  // 19°42.6'
    let deneb_time = Utc.from_utc_datetime(
        &NaiveDate::from_ymd_opt(2015, 9, 10).unwrap()
            .and_time(NaiveTime::from_hms_opt(6, 33, 47).unwrap())
    );

    let gha_aries_deneb = gha_aries(deneb_time);

    // Optimize longitude for Deneb
    let lha_aries_deneb_dr = (gha_aries_deneb + dr_lon + 360.0) % 360.0;
    let lha_aries_deneb_whole = lha_aries_deneb_dr.round();
    let ap_lon_deneb = (lha_aries_deneb_whole - gha_aries_deneb + 360.0) % 360.0;
    let ap_lon_deneb = if ap_lon_deneb > 180.0 { ap_lon_deneb - 360.0 } else { ap_lon_deneb };

    let gha_deneb = (gha_aries_deneb + deneb_sha_2015) % 360.0;
    let deneb_lha = (gha_deneb + ap_lon_deneb + 360.0) % 360.0;

    let mut deneb_ho = deneb_hs;
    deneb_ho += index_error;
    deneb_ho += apply_dip_correction(height_of_eye);
    deneb_ho += apply_refraction_correction(deneb_ho);

    let deneb_sight_data = SightData {
        latitude: ap_lat,  // Use AP latitude!
        declination: deneb_dec_2015,
        local_hour_angle: deneb_lha,
    };

    let deneb_hc = compute_altitude(&deneb_sight_data);
    let deneb_zn = compute_azimuth(&deneb_sight_data);
    let deneb_intercept = compute_intercept(&deneb_sight_data, deneb_ho);

    println!("\nDENEB @ 06:33:47 UTC");
    println!("  GHA Aries: {:.4}°", gha_aries_deneb);
    println!("  AP Lon: {:.4}°W ({}°{:.1}'W) [optimized for LHA Aries = {}°]",
             ap_lon_deneb.abs(), ap_lon_deneb.abs().floor() as i32,
             (ap_lon_deneb.abs() - ap_lon_deneb.abs().floor()) * 60.0,
             lha_aries_deneb_whole as i32);
    println!("  LHA Aries: {:.1}°", lha_aries_deneb_whole);
    println!("  AP: 50°N, {:.4}°W", ap_lon_deneb.abs());
    println!("  Hs: {:.1}', Ho: {:.1}'", deneb_hs * 60.0, deneb_ho * 60.0);
    println!("  Hc: {:.1}' ({:.4}°)", deneb_hc * 60.0, deneb_hc);
    println!("  Zn: {:.1}°", deneb_zn);
    println!("  Intercept: {:.2} NM", deneb_intercept);

    let lop3 = LineOfPosition {
        azimuth: deneb_zn,
        intercept: deneb_intercept,
        dr_latitude: ap_lat,
        dr_longitude: ap_lon_deneb,
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
}

fn apply_dip_correction(height_of_eye_meters: f64) -> f64 {
    -0.0293 * height_of_eye_meters.sqrt()
}

fn apply_refraction_correction(altitude_deg: f64) -> f64 {
    let altitude_rad = altitude_deg.to_radians();
    -0.0167 / altitude_rad.tan()
}
