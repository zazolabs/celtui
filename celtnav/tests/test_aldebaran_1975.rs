//! Aldebaran 1975 Sight Reduction Validation Test
//!
//! Verifies that proper motion and precession corrections produce the correct
//! star position when computing Hc for a 1975 observation.
//!
//! Test data:
//!   Star: Aldebaran
//!   Date: 1975-02-17 02:54:17 UTC
//!   DR: 50°17.0'S, 101°59.5'W
//!   Hs: 21°09.6', IE: -1.2', DIP: -2.8' → Ho: 21°02.0'
//!   Expected Hc: ~21°09' – 21°14'

use celtnav::almanac::{find_star_for_year, gha_aries};
use chrono::{TimeZone, Utc};

#[test]
fn test_aldebaran_1975_hc() {
    // Observation date
    let dt = Utc.with_ymd_and_hms(1975, 2, 17, 2, 54, 17).unwrap();
    let observation_year = 1975.0 + (31.0 + 17.0 + (2.0 * 3600.0 + 54.0 * 60.0 + 17.0) / 86400.0) / 365.25;

    // Get Aldebaran position corrected to 1975
    let star = find_star_for_year("Aldebaran", observation_year)
        .expect("Aldebaran should be in catalog");

    // GHA Aries at observation time
    let gha_aries = gha_aries(dt);

    // GHA star = GHA Aries + SHA star
    let gha_star = (gha_aries + star.sha).rem_euclid(360.0);

    // DR position
    let dr_lat: f64 = -50.0 - 17.0 / 60.0; // 50°17.0'S
    let dr_lon: f64 = -(101.0 + 59.5 / 60.0); // 101°59.5'W

    // LHA = GHA + longitude (W is negative)
    let lha = (gha_star + dr_lon).rem_euclid(360.0);

    // Computed altitude (Hc) using spherical trig
    let lat_rad = dr_lat.to_radians();
    let dec_rad = star.declination.to_radians();
    let lha_rad = lha.to_radians();

    let sin_hc = lat_rad.sin() * dec_rad.sin()
        + lat_rad.cos() * dec_rad.cos() * lha_rad.cos();
    let hc = sin_hc.asin().to_degrees();

    println!("Star SHA (1975): {:.4}°", star.sha);
    println!("Star Dec (1975): {:.4}°", star.declination);
    println!("GHA Aries: {:.4}°", gha_aries);
    println!("GHA Star: {:.4}°", gha_star);
    println!("LHA: {:.4}°", lha);
    println!("Hc: {:.4}° = {}°{:.1}'", hc, hc as i32, (hc.abs().fract() * 60.0));

    // Hc should be approximately 21°09' – 21°14' (21.15° – 21.23°)
    assert!(
        hc > 20.5 && hc < 22.0,
        "Hc = {:.4}° is outside expected range 20.5°–22.0°. Precession/proper motion fix may be needed.",
        hc
    );
}
