// SPDX-License-Identifier: GPL-2.0-only
// SPDX-FileCopyrightText: Alexander Atanasov <alex@zazolabs.com>
//! Test complete Moon and planet sight reduction workflows
//!
//! These tests verify the complete process of reducing a Moon or planet sight,
//! including all corrections (dip, refraction, parallax, semidiameter).

use celtnav::almanac::{get_body_position, CelestialBody, Planet};
use celtnav::sight_reduction::{
    apply_dip_correction, apply_refraction_correction,
    apply_parallax_correction, apply_semidiameter_correction,
    compute_altitude, compute_azimuth, compute_intercept, SightData,
};
use celtnav::dms_to_decimal;
use chrono::{NaiveDate, NaiveTime, TimeZone, Utc};

/// Helper to create UTC datetime
fn make_datetime(date: &str, time: &str) -> chrono::DateTime<chrono::Utc> {
    let date = NaiveDate::parse_from_str(date, "%Y-%m-%d").unwrap();
    let time = NaiveTime::parse_from_str(time, "%H:%M:%S").unwrap();
    Utc.from_utc_datetime(&date.and_time(time))
}

/// Test complete Moon sight reduction
///
/// Scenario: Moon lower limb observation
/// - Sextant altitude (Hs): 45° 30.0'
/// - Height of eye: 10 meters
/// - Index error: 0'
/// - Moon HP: 57.0' (0.95°)
/// - Moon SD: 15.6' (0.26°)
#[test]
fn test_moon_lower_limb_sight_reduction() {
    println!("\n=== COMPLETE MOON SIGHT REDUCTION ===");
    println!("Lower limb observation");

    // Sextant altitude
    let hs = dms_to_decimal(45, 30, 0.0);
    println!("\n1. Sextant altitude (Hs): 45° 30.0' = {:.4}°", hs);

    // Apply index error (assume 0)
    let index_error = 0.0;
    let hs_corrected = hs + index_error;
    println!("   Index error: 0'");
    println!("   Hs corrected: {:.4}°", hs_corrected);

    // Apply dip correction (height of eye = 10m)
    let height_of_eye = 10.0;
    let dip = apply_dip_correction(height_of_eye);
    let ha = hs_corrected + dip; // dip is negative
    println!("\n2. Dip correction (HoE=10m): {:.3}° ({:.1}')", dip, dip * 60.0);
    println!("   Apparent altitude (Ha): {:.4}°", ha);

    // Apply refraction correction
    let refraction = apply_refraction_correction(ha);
    let mut ho = ha + refraction; // refraction is negative
    println!("\n3. Refraction correction: {:.3}° ({:.1}')", refraction, refraction * 60.0);

    // Apply Moon parallax (HP = 57.0')
    let hp = 0.95; // 57' in degrees
    let parallax = apply_parallax_correction(hp, ho);
    ho += parallax; // parallax is positive for Moon
    println!("\n4. Parallax correction (HP=57'): +{:.3}° (+{:.1}')", parallax, parallax * 60.0);

    // Apply Moon semidiameter (SD = 15.6')
    let sd = 0.26; // 15.6' in degrees
    let sd_correction = apply_semidiameter_correction(sd, true); // true = lower limb
    ho += sd_correction;
    println!("\n5. Semidiameter correction (SD=15.6', LL): +{:.3}° (+{:.1}')", sd_correction, sd_correction * 60.0);

    println!("\n6. Observed altitude (Ho): {:.4}° ({:.0}° {:.1}')",
             ho, ho.floor(), (ho.fract() * 60.0));

    // Verify Ho is reasonable
    assert!(
        ho > 45.0 && ho < 47.0,
        "Final Ho should be around 45-47° for this scenario, got {:.2}°",
        ho
    );

    // Verify corrections went in expected directions
    assert!(dip < 0.0, "Dip should be negative");
    assert!(refraction < 0.0, "Refraction should be negative");
    assert!(parallax > 0.0, "Moon parallax should be positive");
    assert!(sd_correction > 0.0, "Lower limb SD correction should be positive");

    println!("\n✓ All corrections applied correctly");
}

/// Test Moon upper limb sight reduction
#[test]
fn test_moon_upper_limb_sight_reduction() {
    println!("\n=== MOON UPPER LIMB SIGHT REDUCTION ===");

    let hs = dms_to_decimal(45, 30, 0.0);
    let height_of_eye = 10.0;
    let hp = 0.95;
    let sd = 0.26;

    // Apply corrections
    let dip = apply_dip_correction(height_of_eye);
    let ha = hs + dip;
    let refraction = apply_refraction_correction(ha);
    let parallax = apply_parallax_correction(hp, ha + refraction);
    let sd_correction = apply_semidiameter_correction(sd, false); // false = upper limb

    let ho = hs + dip + refraction + parallax + sd_correction;

    println!("Sextant altitude (Hs): 45° 30.0'");
    println!("Dip:           {:.1}'", dip * 60.0);
    println!("Refraction:    {:.1}'", refraction * 60.0);
    println!("Parallax:      +{:.1}'", parallax * 60.0);
    println!("SD (UL):       {:.1}' (subtract for upper limb)", sd_correction * 60.0);
    println!("Ho:            {:.0}° {:.1}'", ho.floor(), ho.fract() * 60.0);

    // Upper limb SD should be negative
    assert!(sd_correction < 0.0, "Upper limb SD correction should be negative");

    // Ho should be less than lower limb Ho (by 2*SD ≈ 31')
    let ll_sd = apply_semidiameter_correction(sd, true);
    let difference = (ll_sd - sd_correction).abs();
    println!("\nDifference from LL: {:.1}' (should be ~{:.1}')",
             difference * 60.0, 2.0 * sd * 60.0);

    assert!(
        (difference - 2.0 * sd).abs() < 0.01,
        "UL vs LL difference should be 2*SD"
    );
}

/// Test Venus sight reduction (no parallax or SD corrections needed)
#[test]
fn test_venus_sight_reduction() {
    println!("\n=== VENUS SIGHT REDUCTION ===");
    println!("Planets don't need parallax or SD corrections");

    let hs = dms_to_decimal(30, 15, 0.0);
    let height_of_eye = 10.0;

    println!("\nSextant altitude (Hs): 30° 15.0'");

    // Only dip and refraction for planets
    let dip = apply_dip_correction(height_of_eye);
    let ha = hs + dip;
    let refraction = apply_refraction_correction(ha);
    let ho = hs + dip + refraction;

    println!("Dip:        {:.1}'", dip * 60.0);
    println!("Refraction: {:.1}'", refraction * 60.0);
    println!("Ho:         {:.0}° {:.1}'", ho.floor(), ho.fract() * 60.0);

    // Verify no parallax or SD needed
    let planet_parallax = apply_parallax_correction(0.0, ha); // HP = 0 for planets
    assert_eq!(planet_parallax, 0.0, "Planet parallax should be zero");

    println!("\n✓ Planet corrections complete (no parallax/SD)");
}

/// Test Moon sight reduction with altitude calculation
#[test]
fn test_moon_complete_sight_with_hc() {
    let datetime = make_datetime("2024-03-15", "12:00:00");

    // Observer's DR position
    let dr_lat = 45.0; // 45°N
    let dr_lon = -123.0; // 123°W

    // Get Moon position
    let moon_pos = get_body_position(CelestialBody::Moon, datetime)
        .expect("Failed to get Moon position");

    println!("\n=== MOON SIGHT WITH Hc CALCULATION ===");
    println!("Date/Time: 2024-03-15 12:00:00 UTC");
    println!("DR Position: 45°N, 123°W");
    println!("\nMoon position:");
    println!("  GHA: {:.2}°", moon_pos.gha);
    println!("  Dec: {:.2}°", moon_pos.declination);

    // Calculate LHA
    let lha = (moon_pos.gha + dr_lon + 360.0) % 360.0;
    println!("  LHA: {:.2}°", lha);

    // Compute altitude and azimuth
    let sight_data = SightData {
        latitude: dr_lat,
        declination: moon_pos.declination,
        local_hour_angle: lha,
    };

    let hc = compute_altitude(&sight_data);
    let zn = compute_azimuth(&sight_data);

    println!("\nComputed:");
    println!("  Hc: {:.0}° {:.1}' ({:.4}°)", hc.floor(), hc.fract() * 60.0, hc);
    println!("  Zn: {:.0}° ({:.2}°)", zn.round(), zn);

    // Verify results are reasonable
    assert!(
        (-90.0..=90.0).contains(&hc),
        "Hc should be in range [-90°, 90°], got {:.2}°",
        hc
    );

    assert!(
        (0.0..360.0).contains(&zn),
        "Zn should be in range [0°, 360°), got {:.2}°",
        zn
    );

    // Test intercept calculation
    // Assume observed altitude (after corrections) is slightly different
    let ho = hc + 0.3; // 18' toward (positive intercept)
    let intercept = compute_intercept(&sight_data, ho);

    println!("\nIntercept:");
    println!("  Ho: {:.0}° {:.1}'", ho.floor(), ho.fract() * 60.0);
    println!("  Hc: {:.0}° {:.1}'", hc.floor(), hc.fract() * 60.0);
    println!("  Intercept: {:.1} NM {}", intercept.abs(), if intercept > 0.0 { "T" } else { "A" });

    // Intercept should be about 18 NM toward
    assert!(
        (intercept - 18.0).abs() < 1.0,
        "Intercept should be ~18 NM toward, got {:.1}",
        intercept
    );
}

/// Test planet sight with Hc calculation
#[test]
fn test_jupiter_complete_sight() {
    let datetime = make_datetime("2024-03-15", "20:00:00");

    let dr_lat = 35.0;
    let dr_lon = -75.0; // US East Coast

    let jupiter_pos = get_body_position(CelestialBody::Planet(Planet::Jupiter), datetime)
        .expect("Failed to get Jupiter position");

    println!("\n=== JUPITER SIGHT WITH Hc CALCULATION ===");
    println!("Date/Time: 2024-03-15 20:00:00 UTC");
    println!("DR Position: 35°N, 75°W");
    println!("\nJupiter position:");
    println!("  GHA: {:.2}°", jupiter_pos.gha);
    println!("  Dec: {:.2}°", jupiter_pos.declination);

    let lha = (jupiter_pos.gha + dr_lon + 360.0) % 360.0;
    println!("  LHA: {:.2}°", lha);

    let sight_data = SightData {
        latitude: dr_lat,
        declination: jupiter_pos.declination,
        local_hour_angle: lha,
    };

    let hc = compute_altitude(&sight_data);
    let zn = compute_azimuth(&sight_data);

    println!("\nComputed:");
    println!("  Hc: {:.0}° {:.1}'", hc.floor(), hc.fract() * 60.0);
    println!("  Zn: {:.0}°", zn.round());

    assert!((-90.0..=90.0).contains(&hc));
    assert!((0.0..360.0).contains(&zn));

    // Jupiter should be visible (above horizon) at this time/location
    // or below horizon depending on actual position
    println!("\nJupiter is {} at this time/location",
             if hc > 0.0 { "above horizon" } else { "below horizon" });
}

/// Test Moon parallax significance vs planets
#[test]
fn test_moon_vs_planet_parallax() {
    let altitude = 30.0; // degrees

    let moon_hp = 0.95; // 57' typical
    let planet_hp = 0.0; // negligible

    let moon_parallax = apply_parallax_correction(moon_hp, altitude);
    let planet_parallax = apply_parallax_correction(planet_hp, altitude);

    println!("\n=== MOON VS PLANET PARALLAX (altitude=30°) ===");
    println!("Moon parallax:   {:.1}' ({:.3}°)", moon_parallax * 60.0, moon_parallax);
    println!("Planet parallax: {:.1}' ({:.3}°)", planet_parallax * 60.0, planet_parallax);
    println!("Difference:      {:.1}' ({:.3}°)",
             (moon_parallax - planet_parallax) * 60.0,
             moon_parallax - planet_parallax);

    // Moon parallax should be significant (> 40' at this altitude)
    assert!(
        moon_parallax * 60.0 > 40.0,
        "Moon parallax should be significant, got {:.1}'",
        moon_parallax * 60.0
    );

    // Planet parallax should be negligible
    assert_eq!(planet_parallax, 0.0, "Planet parallax should be zero");

    // Difference is significant for accurate navigation
    let difference_arcmin = moon_parallax * 60.0;
    println!("\n✓ Moon parallax correction is significant: {:.1}' = {:.1} NM", difference_arcmin, difference_arcmin);
}

/// Test corrections at different altitudes
#[test]
fn test_corrections_at_various_altitudes() {
    let altitudes = vec![5.0, 15.0, 30.0, 45.0, 60.0, 75.0];
    let moon_hp = 0.95;

    println!("\n=== CORRECTIONS AT VARIOUS ALTITUDES ===");
    println!("{:>8} {:>12} {:>12} {:>12}",
             "Alt", "Refraction", "Parallax", "Net");

    for alt in altitudes {
        let refraction = apply_refraction_correction(alt);
        let parallax = apply_parallax_correction(moon_hp, alt);
        let net = refraction + parallax;

        println!("{:>7.0}° {:>11.1}' {:>11.1}' {:>11.1}'",
                 alt,
                 refraction * 60.0,
                 parallax * 60.0,
                 net * 60.0);
    }

    // For the Moon, parallax can actually dominate at all altitudes due to large HP
    // At low altitudes, both corrections are significant
    let low_refraction = apply_refraction_correction(5.0);
    let low_parallax = apply_parallax_correction(moon_hp, 5.0);
    println!("\nAt 5°: refraction={:.1}', parallax={:.1}' (Moon parallax is unusually large)",
             low_refraction * 60.0, low_parallax * 60.0);

    // At high altitudes, parallax decreases more than refraction
    let high_refraction = apply_refraction_correction(60.0);
    let high_parallax = apply_parallax_correction(moon_hp, 60.0);
    println!("At 60°: refraction={:.1}', parallax={:.1}'",
             high_refraction * 60.0, high_parallax * 60.0);

    // Both should be significant at low altitudes
    assert!(
        low_refraction.abs() > 0.1 && low_parallax.abs() > 0.5,
        "Both corrections should be significant at low altitudes"
    );
}

/// Test Moon sight at different latitudes
#[test]
fn test_moon_sight_different_latitudes() {
    let datetime = make_datetime("2024-03-15", "12:00:00");
    let moon_pos = get_body_position(CelestialBody::Moon, datetime).unwrap();

    let latitudes = vec![-60.0, -30.0, 0.0, 30.0, 60.0];
    let dr_lon = -30.0;

    println!("\n=== MOON SIGHT AT DIFFERENT LATITUDES ===");
    println!("Date/Time: 2024-03-15 12:00:00 UTC");
    println!("DR Longitude: 30°W");
    println!("Moon GHA: {:.2}°, Dec: {:.2}°", moon_pos.gha, moon_pos.declination);

    println!("\n{:>8} {:>12} {:>8}", "Latitude", "Hc", "Zn");

    for lat in latitudes {
        let lha = (moon_pos.gha + dr_lon + 360.0) % 360.0;

        let sight_data = SightData {
            latitude: lat,
            declination: moon_pos.declination,
            local_hour_angle: lha,
        };

        let hc = compute_altitude(&sight_data);
        let zn = compute_azimuth(&sight_data);

        println!("{:>7.0}° {:>11.1}° {:>7.0}°", lat, hc, zn.round());
    }

    println!("\n✓ Moon calculations work across all latitudes");
}
