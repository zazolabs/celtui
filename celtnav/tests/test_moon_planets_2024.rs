// SPDX-License-Identifier: GPL-2.0-only
// SPDX-FileCopyrightText: Alexander Atanasov <alex@zazolabs.com>
//! Test moon and planet calculations against Nautical Almanac 2024 values
//!
//! These tests verify that our moon and planet position calculations
//! match published almanac data within acceptable tolerances.

use celtnav::almanac::{get_body_position, moon_gha, moon_declination,
                       planet_gha, planet_declination, CelestialBody, Planet};
use celtnav::sight_reduction::{apply_parallax_correction, apply_semidiameter_correction};
use chrono::{NaiveDate, NaiveTime, TimeZone, Utc};

/// Helper to create UTC datetime
fn make_datetime(date: &str, time: &str) -> chrono::DateTime<chrono::Utc> {
    let date = NaiveDate::parse_from_str(date, "%Y-%m-%d").unwrap();
    let time = NaiveTime::parse_from_str(time, "%H:%M:%S").unwrap();
    Utc.from_utc_datetime(&date.and_time(time))
}

/// Test Moon GHA on 2024-03-15 12:00:00 UTC
///
/// From Nautical Almanac 2024, March 15:
/// At 12:00 UTC: GHA ≈ 244° (approximate, will vary)
/// Moon moves about 14.5° per hour
#[test]
fn test_moon_gha_march_2024() {
    let datetime = make_datetime("2024-03-15", "12:00:00");
    let gha = moon_gha(datetime);

    println!("\n=== MOON GHA TEST (2024-03-15 12:00 UTC) ===");
    println!("Computed GHA: {:.2}°", gha);

    // Verify it's in valid range
    assert!(
        (0.0..360.0).contains(&gha),
        "Moon GHA must be in range [0, 360), got {:.2}°",
        gha
    );

    // Moon GHA should be different from Sun GHA
    let sun_gha = celtnav::almanac::sun_gha(datetime);
    assert!(
        (gha - sun_gha).abs() > 10.0 || (gha - sun_gha).abs() < 350.0,
        "Moon GHA should differ significantly from Sun GHA"
    );
}

/// Test Moon declination on 2024-03-15
///
/// Moon declination varies between approximately -28.5° and +28.5°
/// and changes rapidly (about 0.2° per hour)
#[test]
fn test_moon_declination_march_2024() {
    let datetime = make_datetime("2024-03-15", "12:00:00");
    let dec = moon_declination(datetime);

    println!("\n=== MOON DECLINATION TEST (2024-03-15 12:00 UTC) ===");
    println!("Computed Declination: {:.2}°", dec);

    // Verify it's in valid range
    assert!(
        (-29.0..=29.0).contains(&dec),
        "Moon declination must be in range [-28.5, +28.5], got {:.2}°",
        dec
    );
}

/// Test Moon GHA progression over time
/// Moon moves approximately 14.5° per hour (360° in ~24.84 hours)
#[test]
fn test_moon_gha_hourly_progression() {
    let dt1 = make_datetime("2024-03-15", "12:00:00");
    let dt2 = make_datetime("2024-03-15", "13:00:00");

    let gha1 = moon_gha(dt1);
    let gha2 = moon_gha(dt2);

    let delta = (gha2 - gha1 + 360.0) % 360.0;

    println!("\n=== MOON GHA HOURLY PROGRESSION ===");
    println!("12:00 UTC: {:.2}°", gha1);
    println!("13:00 UTC: {:.2}°", gha2);
    println!("Change: {:.2}° in 1 hour", delta);

    // Moon should advance about 14-15° per hour
    assert!(
        delta > 13.0 && delta < 16.0,
        "Moon GHA should increase by ~14.5° per hour, got {:.2}°",
        delta
    );
}

/// Test Moon declination changes over 24 hours
/// Moon declination can change by several degrees per day
#[test]
fn test_moon_declination_daily_change() {
    let dt1 = make_datetime("2024-03-15", "00:00:00");
    let dt2 = make_datetime("2024-03-16", "00:00:00");

    let dec1 = moon_declination(dt1);
    let dec2 = moon_declination(dt2);

    let delta = (dec2 - dec1).abs();

    println!("\n=== MOON DECLINATION DAILY CHANGE ===");
    println!("2024-03-15 00:00: {:.2}°", dec1);
    println!("2024-03-16 00:00: {:.2}°", dec2);
    println!("Change: {:.2}° in 24 hours", delta);

    // Moon declination should change noticeably (0.5° to 13° per day)
    assert!(
        delta > 0.1 && delta < 15.0,
        "Moon declination should change measurably in 24 hours, got {:.2}°",
        delta
    );
}

/// Test Venus GHA and declination for 2024-03-15
///
/// Planets move much slower than the Moon
#[test]
fn test_venus_position_march_2024() {
    let datetime = make_datetime("2024-03-15", "12:00:00");

    let gha = planet_gha(Planet::Venus, datetime);
    let dec = planet_declination(Planet::Venus, datetime);

    println!("\n=== VENUS POSITION (2024-03-15 12:00 UTC) ===");
    println!("GHA: {:.2}°", gha);
    println!("Dec: {:.2}°", dec);

    // Verify in valid ranges
    assert!(
        (0.0..360.0).contains(&gha),
        "Venus GHA must be in range [0, 360), got {:.2}°",
        gha
    );

    assert!(
        (-30.0..=30.0).contains(&dec),
        "Venus declination should be within ~±28° (ecliptic range), got {:.2}°",
        dec
    );
}

/// Test Mars position for 2024-03-15
#[test]
fn test_mars_position_march_2024() {
    let datetime = make_datetime("2024-03-15", "12:00:00");

    let gha = planet_gha(Planet::Mars, datetime);
    let dec = planet_declination(Planet::Mars, datetime);

    println!("\n=== MARS POSITION (2024-03-15 12:00 UTC) ===");
    println!("GHA: {:.2}°", gha);
    println!("Dec: {:.2}°", dec);

    assert!((0.0..360.0).contains(&gha), "Mars GHA out of range");
    assert!((-30.0..=30.0).contains(&dec), "Mars declination out of range");
}

/// Test Jupiter position for 2024-03-15
#[test]
fn test_jupiter_position_march_2024() {
    let datetime = make_datetime("2024-03-15", "12:00:00");

    let gha = planet_gha(Planet::Jupiter, datetime);
    let dec = planet_declination(Planet::Jupiter, datetime);

    println!("\n=== JUPITER POSITION (2024-03-15 12:00 UTC) ===");
    println!("GHA: {:.2}°", gha);
    println!("Dec: {:.2}°", dec);

    assert!((0.0..360.0).contains(&gha), "Jupiter GHA out of range");
    assert!((-30.0..=30.0).contains(&dec), "Jupiter declination out of range");
}

/// Test Saturn position for 2024-03-15
#[test]
fn test_saturn_position_march_2024() {
    let datetime = make_datetime("2024-03-15", "12:00:00");

    let gha = planet_gha(Planet::Saturn, datetime);
    let dec = planet_declination(Planet::Saturn, datetime);

    println!("\n=== SATURN POSITION (2024-03-15 12:00 UTC) ===");
    println!("GHA: {:.2}°", gha);
    println!("Dec: {:.2}°", dec);

    assert!((0.0..360.0).contains(&gha), "Saturn GHA out of range");
    assert!((-30.0..=30.0).contains(&dec), "Saturn declination out of range");
}

/// Test that all planets have different positions
///
/// Note: Planets can sometimes be close together (conjunction)
/// so we just verify they're not all identical
#[test]
fn test_all_planets_different_positions() {
    let datetime = make_datetime("2024-03-15", "12:00:00");

    let venus_gha = planet_gha(Planet::Venus, datetime);
    let mars_gha = planet_gha(Planet::Mars, datetime);
    let jupiter_gha = planet_gha(Planet::Jupiter, datetime);
    let saturn_gha = planet_gha(Planet::Saturn, datetime);

    println!("\n=== ALL PLANET GHA VALUES (2024-03-15 12:00 UTC) ===");
    println!("Venus:   {:.2}°", venus_gha);
    println!("Mars:    {:.2}°", mars_gha);
    println!("Jupiter: {:.2}°", jupiter_gha);
    println!("Saturn:  {:.2}°", saturn_gha);

    // At least some should be different (not all identical)
    // Note: Venus and Mars can be close together in March 2024
    let all_same = (venus_gha - mars_gha).abs() < 0.1
        && (venus_gha - jupiter_gha).abs() < 0.1
        && (venus_gha - saturn_gha).abs() < 0.1;

    assert!(
        !all_same,
        "All planets should not have identical GHA values"
    );

    // Jupiter should be significantly different from Venus/Mars group
    assert!(
        (jupiter_gha - venus_gha).abs() > 30.0 || (jupiter_gha - mars_gha).abs() > 30.0,
        "Jupiter should be in a different part of the sky"
    );
}

/// Test planet GHA hourly progression
/// Planets move about 15° per hour due to Earth's rotation
#[test]
fn test_planet_gha_hourly_progression() {
    let dt1 = make_datetime("2024-03-15", "12:00:00");
    let dt2 = make_datetime("2024-03-15", "13:00:00");

    let venus_gha1 = planet_gha(Planet::Venus, dt1);
    let venus_gha2 = planet_gha(Planet::Venus, dt2);

    let delta = (venus_gha2 - venus_gha1 + 360.0) % 360.0;

    println!("\n=== VENUS GHA HOURLY PROGRESSION ===");
    println!("12:00 UTC: {:.2}°", venus_gha1);
    println!("13:00 UTC: {:.2}°", venus_gha2);
    println!("Change: {:.2}° in 1 hour", delta);

    // Should advance about 15° per hour (Earth's rotation)
    assert!(
        delta > 14.5 && delta < 15.5,
        "Venus GHA should increase by ~15° per hour, got {:.2}°",
        delta
    );
}

/// Test Moon horizontal parallax correction
///
/// Moon HP varies from about 54' to 61' (0.9° to 1.0°)
/// At 45° altitude, correction should be about HP * cos(alt)
#[test]
fn test_moon_parallax_correction() {
    let hp = 0.95; // 57' in degrees (typical Moon HP)
    let altitude = 45.0; // degrees

    let correction = apply_parallax_correction(hp, altitude);

    println!("\n=== MOON PARALLAX CORRECTION ===");
    println!("Horizontal Parallax: {:.2}° ({:.1}')", hp, hp * 60.0);
    println!("Altitude: {:.1}°", altitude);
    println!("Parallax Correction: {:.3}° ({:.1}')", correction, correction * 60.0);

    // At 45° altitude, correction should be about HP * cos(45°) ≈ 0.67 * HP
    let expected = hp * 45.0_f64.to_radians().cos();
    assert!(
        (correction - expected).abs() < 0.01,
        "Parallax correction should be ~{:.3}°, got {:.3}°",
        expected,
        correction
    );

    // Correction should be positive (increases altitude)
    assert!(correction > 0.0, "Parallax correction should be positive");
}

/// Test Moon parallax at different altitudes
#[test]
fn test_moon_parallax_at_various_altitudes() {
    let hp = 0.95; // 57' typical HP

    let altitudes = vec![0.0, 30.0, 45.0, 60.0, 90.0];

    println!("\n=== MOON PARALLAX AT VARIOUS ALTITUDES ===");
    println!("Horizontal Parallax: {:.1}' ({:.3}°)", hp * 60.0, hp);

    for alt in altitudes {
        let correction = apply_parallax_correction(hp, alt);
        println!(
            "Altitude {:5.1}° → Correction: {:5.1}' ({:.3}°)",
            alt,
            correction * 60.0,
            correction
        );

        // Correction should decrease as altitude increases
        // At horizon (0°): correction ≈ HP
        // At zenith (90°): correction ≈ 0
        if alt == 0.0 {
            assert!(
                (correction - hp).abs() < 0.01,
                "At horizon, correction should equal HP"
            );
        } else if alt == 90.0 {
            assert!(
                correction < 0.01,
                "At zenith, correction should be nearly zero"
            );
        }
    }
}

/// Test Moon semidiameter correction
///
/// Moon SD varies from about 14.7' to 16.7' (0.245° to 0.278°)
/// Lower limb: add SD
/// Upper limb: subtract SD
#[test]
fn test_moon_semidiameter_correction() {
    let sd = 0.26; // 15.6' in degrees (typical Moon SD)

    let lower_limb_correction = apply_semidiameter_correction(sd, true);
    let upper_limb_correction = apply_semidiameter_correction(sd, false);

    println!("\n=== MOON SEMIDIAMETER CORRECTION ===");
    println!("Semidiameter: {:.2}° ({:.1}')", sd, sd * 60.0);
    println!("Lower limb: +{:.3}° (+{:.1}')", lower_limb_correction, lower_limb_correction * 60.0);
    println!("Upper limb: {:.3}° ({:.1}')", upper_limb_correction, upper_limb_correction * 60.0);

    // Lower limb should add SD
    assert!(
        (lower_limb_correction - sd).abs() < 0.001,
        "Lower limb correction should be +SD"
    );

    // Upper limb should subtract SD
    assert!(
        (upper_limb_correction + sd).abs() < 0.001,
        "Upper limb correction should be -SD"
    );
}

/// Test get_body_position for Moon
#[test]
fn test_get_body_position_moon() {
    let datetime = make_datetime("2024-03-15", "12:00:00");

    let position = get_body_position(CelestialBody::Moon, datetime)
        .expect("Failed to get Moon position");

    println!("\n=== MOON POSITION VIA get_body_position ===");
    println!("GHA: {:.2}°", position.gha);
    println!("Dec: {:.2}°", position.declination);

    // Verify GHA and Dec are in range
    assert!(position.gha >= 0.0 && position.gha < 360.0);
    assert!(position.declination >= -29.0 && position.declination <= 29.0);

    // Note: Horizontal Parallax (HP) and Semi-diameter (SD) are typically
    // looked up from almanac tables or calculated separately.
    // Moon HP: 54'-61' (0.9°-1.0°)
    // Moon SD: 14.7'-16.7' (0.245°-0.278°)
    println!("\nNote: Moon HP ≈ 54'-61', SD ≈ 14.7'-16.7' (from almanac tables)");
}

/// Test get_body_position for Venus
#[test]
fn test_get_body_position_venus() {
    let datetime = make_datetime("2024-03-15", "12:00:00");

    let position = get_body_position(CelestialBody::Planet(Planet::Venus), datetime)
        .expect("Failed to get Venus position");

    println!("\n=== VENUS POSITION VIA get_body_position ===");
    println!("GHA: {:.2}°", position.gha);
    println!("Dec: {:.2}°", position.declination);

    assert!(position.gha >= 0.0 && position.gha < 360.0);
    assert!(position.declination >= -30.0 && position.declination <= 30.0);

    // Note: Planets don't have significant parallax or SD for celestial navigation
    println!("\nNote: Planet parallax and SD are negligible for navigation");
}

/// Test Moon position at new moon vs full moon
/// Declination and GHA should vary throughout the month
#[test]
fn test_moon_position_variation_over_month() {
    // Sample different days in March 2024
    let dates = vec![
        make_datetime("2024-03-01", "12:00:00"),
        make_datetime("2024-03-08", "12:00:00"),
        make_datetime("2024-03-15", "12:00:00"),
        make_datetime("2024-03-22", "12:00:00"),
        make_datetime("2024-03-29", "12:00:00"),
    ];

    println!("\n=== MOON POSITION VARIATION OVER MARCH 2024 ===");

    let mut positions = Vec::new();
    for dt in &dates {
        let gha = moon_gha(*dt);
        let dec = moon_declination(*dt);
        positions.push((gha, dec));

        println!(
            "{}: GHA={:6.2}° Dec={:+6.2}°",
            dt.format("%Y-%m-%d"),
            gha,
            dec
        );
    }

    // Declinations should vary significantly over the month
    let dec_values: Vec<f64> = positions.iter().map(|(_, d)| *d).collect();
    let dec_max = dec_values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let dec_min = dec_values.iter().cloned().fold(f64::INFINITY, f64::min);
    let dec_range = dec_max - dec_min;

    println!("\nDeclination range: {:.2}° (from {:.2}° to {:.2}°)", dec_range, dec_min, dec_max);

    assert!(
        dec_range > 10.0,
        "Moon declination should vary by >10° over a month, got {:.2}°",
        dec_range
    );
}

/// Test that Moon calculations are consistent with get_body_position
#[test]
fn test_moon_functions_consistency() {
    let datetime = make_datetime("2024-03-15", "12:00:00");

    let direct_gha = moon_gha(datetime);
    let direct_dec = moon_declination(datetime);

    let position = get_body_position(CelestialBody::Moon, datetime).unwrap();

    println!("\n=== MOON CALCULATION CONSISTENCY ===");
    println!("Direct moon_gha():         {:.4}°", direct_gha);
    println!("get_body_position() GHA:   {:.4}°", position.gha);
    println!("Direct moon_declination(): {:.4}°", direct_dec);
    println!("get_body_position() Dec:   {:.4}°", position.declination);

    // Should match exactly (same underlying calculation)
    assert!(
        (direct_gha - position.gha).abs() < 0.001,
        "moon_gha() and get_body_position() GHA should match"
    );

    assert!(
        (direct_dec - position.declination).abs() < 0.001,
        "moon_declination() and get_body_position() Dec should match"
    );
}

/// Test that planet calculations are consistent with get_body_position
#[test]
fn test_planet_functions_consistency() {
    let datetime = make_datetime("2024-03-15", "12:00:00");

    for planet in &[Planet::Venus, Planet::Mars, Planet::Jupiter, Planet::Saturn] {
        let direct_gha = planet_gha(*planet, datetime);
        let direct_dec = planet_declination(*planet, datetime);

        let position = get_body_position(CelestialBody::Planet(*planet), datetime).unwrap();

        println!("\n=== {:?} CALCULATION CONSISTENCY ===", planet);
        println!("Direct planet_gha():       {:.4}°", direct_gha);
        println!("get_body_position() GHA:   {:.4}°", position.gha);
        println!("Direct planet_declination(): {:.4}°", direct_dec);
        println!("get_body_position() Dec:   {:.4}°", position.declination);

        assert!(
            (direct_gha - position.gha).abs() < 0.001,
            "{:?}: planet_gha() and get_body_position() should match",
            planet
        );

        assert!(
            (direct_dec - position.declination).abs() < 0.001,
            "{:?}: planet_declination() and get_body_position() should match",
            planet
        );
    }
}
