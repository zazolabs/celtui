//! Integration tests for star sight optimization (Pub 249 Vol 1)
//!
//! These tests verify that star sights are optimized correctly by making
//! LHA Aries a whole number (not LHA of the star itself).

use celtnav::sight_reduction::{
    optimize_chosen_position_star, optimize_chosen_position_celestial_body,
    SightData, compute_altitude, compute_azimuth,
};
use celtnav::almanac::{gha_aries, get_body_position, CelestialBody};
use chrono::{TimeZone, Utc};

/// Test that star optimization makes LHA Aries whole, not LHA star
#[test]
fn test_star_optimization_makes_lha_aries_whole() {
    // Observer at 40°N, 70°W
    let dr_lat = 40.0;
    let dr_lon = -70.0;

    // Various GHA Aries values
    let test_cases = vec![
        100.0, 150.5, 200.3, 250.7, 300.2, 350.8,
    ];

    for gha_aries_val in test_cases {
        let (chosen_lat, chosen_lon) = optimize_chosen_position_star(dr_lat, dr_lon, gha_aries_val);

        // Latitude should be rounded
        assert_eq!(chosen_lat, 40.0);

        // LHA Aries MUST be whole
        let lha_aries = (gha_aries_val + chosen_lon + 360.0) % 360.0;
        let lha_aries_frac = (lha_aries - lha_aries.round()).abs();
        assert!(
            lha_aries_frac < 0.01,
            "LHA Aries should be whole for GHA Aries {:.1}°. Got LHA Aries {:.4}°, fractional part {:.4}",
            gha_aries_val, lha_aries, lha_aries_frac
        );
    }
}

/// Test real-world star sight: Pollux
#[test]
fn test_pollux_star_sight_with_real_almanac_data() {
    // Use a specific date/time to get deterministic results
    let datetime = Utc.with_ymd_and_hms(2024, 3, 15, 12, 0, 0).unwrap();

    // DR position
    let dr_lat = 40.0;
    let dr_lon = -70.0;

    // Get GHA Aries for this time
    let gha_aries_val = gha_aries(datetime);

    // Get Pollux position
    let pollux_body = CelestialBody::Star("Pollux".to_string());
    let pollux_result = get_body_position(pollux_body, datetime);

    if let Ok(pollux_pos) = pollux_result {
        // Optimize chosen position for star
        let (chosen_lat, chosen_lon) = optimize_chosen_position_star(dr_lat, dr_lon, gha_aries_val);

        // Verify LHA Aries is whole
        let lha_aries = (gha_aries_val + chosen_lon + 360.0) % 360.0;
        assert!(
            (lha_aries - lha_aries.round()).abs() < 0.01,
            "LHA Aries should be whole, got {:.2}°", lha_aries
        );

        // Calculate LHA Pollux (will NOT be whole, and that's correct!)
        let lha_pollux = (pollux_pos.gha + chosen_lon + 360.0) % 360.0;

        // Verify we can compute altitude and azimuth
        let sight_data = SightData {
            latitude: chosen_lat,
            declination: pollux_pos.declination,
            local_hour_angle: lha_pollux,
        };

        let hc = compute_altitude(&sight_data);
        let az = compute_azimuth(&sight_data);

        // Sanity checks
        assert!(hc >= -90.0 && hc <= 90.0, "Hc should be valid altitude");
        assert!(az >= 0.0 && az < 360.0, "Azimuth should be valid bearing");
    }
}

/// Test that star optimization differs from celestial body optimization
#[test]
fn test_star_vs_body_optimization_gives_different_results() {
    let dr_lat = 35.0;
    let dr_lon = -120.0;
    let gha_aries_val = 200.0;
    let sha_sirius = 258.6; // Sirius SHA
    let gha_sirius = (gha_aries_val + sha_sirius) % 360.0; // 458.6 - 360 = 98.6°

    // Star optimization: based on GHA Aries
    let (star_lat, star_lon) = optimize_chosen_position_star(dr_lat, dr_lon, gha_aries_val);

    // Body optimization: based on GHA of body
    let (body_lat, body_lon) = optimize_chosen_position_celestial_body(dr_lat, dr_lon, gha_sirius);

    // Latitudes should be the same (both round to nearest degree)
    assert_eq!(star_lat, body_lat);

    // Longitudes should be DIFFERENT
    assert!(
        (star_lon - body_lon).abs() > 0.1,
        "Star and body optimizations should give different longitudes for the same star. \
         Star optimization: {:.2}°, Body optimization: {:.2}°",
        star_lon, body_lon
    );

    // Verify star optimization makes LHA Aries whole
    let lha_aries = (gha_aries_val + star_lon + 360.0) % 360.0;
    assert!(
        (lha_aries - lha_aries.round()).abs() < 0.01,
        "Star optimization should make LHA Aries whole"
    );

    // Verify body optimization makes LHA of body whole
    let lha_body = (gha_sirius + body_lon + 360.0) % 360.0;
    assert!(
        (lha_body - lha_body.round()).abs() < 0.01,
        "Body optimization should make LHA of body whole"
    );
}

/// Test Pub 249 Vol 1 organization
/// Table is indexed by: Latitude, LHA Aries, Star Name
#[test]
fn test_pub249_vol1_table_organization() {
    // Pub 249 Vol 1 is organized by:
    // - Latitude (whole degrees)
    // - LHA Aries (whole degrees)
    // - Star name (with pre-computed SHA)

    let dr_lat = 42.3;  // Should round to 42°
    let dr_lon = -71.1; // Boston area
    let gha_aries_val = 175.8;

    let (chosen_lat, chosen_lon) = optimize_chosen_position_star(dr_lat, dr_lon, gha_aries_val);

    // Latitude: whole degree for table entry
    assert_eq!(chosen_lat, 42.0);

    // LHA Aries: whole degree for table entry
    let lha_aries = (gha_aries_val + chosen_lon + 360.0) % 360.0;
    let lha_aries_rounded = lha_aries.round();
    assert!(
        (lha_aries - lha_aries_rounded).abs() < 0.01,
        "LHA Aries should be whole for Pub 249 Vol 1 lookup"
    );

    // With these two parameters (Lat and LHA Aries), you would:
    // 1. Look up the page for Latitude 42°
    // 2. Find the row for LHA Aries (whatever whole degree it rounded to)
    // 3. Find your star in that row
    // 4. Read Hc and Zn directly from the table

    println!("Pub 249 Vol 1 lookup:");
    println!("  Latitude: {}°", chosen_lat as i32);
    println!("  LHA Aries: {}°", lha_aries_rounded as i32);
    println!("  (Then find star name in table)");
}

/// Test multiple stars to ensure consistency
#[test]
fn test_multiple_stars_same_optimization_method() {
    let dr_lat = 50.0;
    let dr_lon = -5.0;
    let gha_aries_val = 220.5;

    // All stars should use the SAME chosen position
    // because they all share the same GHA Aries
    let (_chosen_lat, chosen_lon) = optimize_chosen_position_star(dr_lat, dr_lon, gha_aries_val);

    // Different stars with different SHA values
    let stars_sha = vec![
        ("Sirius", 258.6),
        ("Arcturus", 145.9),
        ("Vega", 80.5),
        ("Pollux", 243.4),
    ];

    for (star_name, sha) in stars_sha {
        // GHA star = GHA Aries + SHA
        let gha_star = (gha_aries_val + sha) % 360.0;

        // LHA star = LHA Aries + SHA (using optimized chosen position)
        let lha_aries = (gha_aries_val + chosen_lon + 360.0) % 360.0;
        let lha_star = (lha_aries + sha) % 360.0;

        // Verify LHA Aries is whole
        assert!(
            (lha_aries - lha_aries.round()).abs() < 0.01,
            "{}: LHA Aries should be whole", star_name
        );

        // LHA star will generally NOT be whole (because SHA is not whole)
        // But that's correct for Pub 249 Vol 1!

        println!("{}: SHA={:.1}°, GHA={:.1}°, LHA={:.1}° (from LHA Aries {:.0}° + SHA)",
                 star_name, sha, gha_star, lha_star, lha_aries.round());
    }
}

/// Test edge cases near 0° and 360°
#[test]
fn test_star_optimization_near_meridian_boundaries() {
    let test_cases = vec![
        (45.0, -70.0, 0.2),    // GHA Aries near 0°
        (45.0, -70.0, 359.8),  // GHA Aries near 360°
        (45.0, 5.0, 0.5),      // East longitude, GHA Aries near 0°
        (45.0, -179.5, 180.3), // Near date line
    ];

    for (dr_lat, dr_lon, gha_aries_val) in test_cases {
        let (chosen_lat, chosen_lon) = optimize_chosen_position_star(dr_lat, dr_lon, gha_aries_val);

        // LHA Aries must be whole
        let lha_aries = (gha_aries_val + chosen_lon + 360.0) % 360.0;
        assert!(
            (lha_aries - lha_aries.round()).abs() < 0.01,
            "LHA Aries should be whole for GHA Aries {:.1}°, DR lon {:.1}°. Got LHA Aries {:.2}°",
            gha_aries_val, dr_lon, lha_aries
        );

        // Chosen position should be close to DR
        assert!((chosen_lat - dr_lat).abs() <= 0.5, "Latitude should round to nearest degree");
        assert!((chosen_lon - dr_lon).abs() <= 1.0, "Longitude adjustment should be small");
    }
}
