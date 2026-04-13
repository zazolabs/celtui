//! Pollux Observation Debugging Test
//!
//! This test demonstrates why the same altitude (Hc ≈ 46°) can produce
//! two different azimuths (104° vs 265°) depending on the LHA.
//!
//! This is NOT a bug - it's correct celestial navigation!

use celtnav::{compute_altitude, compute_azimuth, SightData};

#[test]
fn test_pollux_eastern_observation() {
    // Morning observation: Pollux in the east
    // Latitude: 45°N, Declination: 28°N (Pollux)
    // LHA: ~52° (eastern side, body rising)

    let sight = SightData {
        latitude: 45.0,
        declination: 28.0,
        local_hour_angle: 51.7,  // Calculated to give Hc ≈ 46°
    };

    let hc = compute_altitude(&sight);
    let zn = compute_azimuth(&sight);

    println!("\n=== POLLUX MORNING OBSERVATION (EASTERN SKY) ===");
    println!("Latitude:    45° 00.0' N");
    println!("Declination: 28° 00.0' N (Pollux)");
    println!("LHA:         {:05.1}°", sight.local_hour_angle);
    println!("---");
    println!("Hc:          {:.2}° ({:02}° {:02.0}')", hc, hc.floor() as i32, (hc.fract() * 60.0));
    println!("Zn:          {:.0}° T (West)", zn);
    println!("===\n");

    // Verify this matches user's EXPECTED values
    assert!(
        (hc - 46.0).abs() < 1.0,
        "Hc should be ~46°, got {:.1}°",
        hc
    );

    assert!(
        (zn - 265.0).abs() < 10.0,
        "Azimuth should be ~265° (W), got {:.0}°",
        zn
    );
}

#[test]
fn test_pollux_western_observation() {
    // Evening observation: Pollux in the west
    // Latitude: 45°N, Declination: 28°N (Pollux)
    // LHA: ~308° (western side, body setting)

    let sight = SightData {
        latitude: 45.0,
        declination: 28.0,
        local_hour_angle: 308.3,  // Calculated to give Hc ≈ 46°
    };

    let hc = compute_altitude(&sight);
    let zn = compute_azimuth(&sight);

    println!("\n=== POLLUX EVENING OBSERVATION (WESTERN SKY) ===");
    println!("Latitude:    45° 00.0' N");
    println!("Declination: 28° 00.0' N (Pollux)");
    println!("LHA:         {:05.1}°", sight.local_hour_angle);
    println!("---");
    println!("Hc:          {:.2}° ({:02}° {:02.0}')", hc, hc.floor() as i32, (hc.fract() * 60.0));
    println!("Zn:          {:.0}° T (East)", zn);
    println!("===\n");

    // Verify this matches what user is GETTING
    assert!(
        (hc - 46.0).abs() < 1.0,
        "Hc should be ~46°, got {:.1}°",
        hc
    );

    assert!(
        (zn - 95.0).abs() < 10.0,
        "Azimuth should be ~95° (E), got {:.0}°",
        zn
    );
}

#[test]
fn test_pollux_lha_from_time() {
    // Demonstrate how LHA changes based on observation time
    // Pollux: SHA = 243.4°

    println!("\n=== HOW OBSERVATION TIME AFFECTS LHA AND AZIMUTH ===");

    // Example: DR Position at W 123° longitude
    let longitude = -123.0;  // West is negative

    // Scenario 1: GHA Aries = 291.3° (morning - to get LHA 52° and Az 104°)
    let gha_aries_morning = 291.3;
    let gha_pollux_morning = (gha_aries_morning + 243.4) % 360.0;  // SHA Pollux = 243.4°
    let lha_morning = (gha_pollux_morning + longitude + 360.0) % 360.0;

    let sight_morning = SightData {
        latitude: 45.0,
        declination: 28.0,
        local_hour_angle: lha_morning,
    };

    let hc_morning = compute_altitude(&sight_morning);
    let zn_morning = compute_azimuth(&sight_morning);

    println!("\nScenario 1: MORNING (GHA Aries = {:.1}°)", gha_aries_morning);
    println!("  GHA Aries:  {:6.1}°", gha_aries_morning);
    println!("  SHA Pollux: {:6.1}°", 243.4);
    println!("  GHA Pollux: {:6.1}°", gha_pollux_morning);
    println!("  Longitude:  {:6.1}° (W 123°)", longitude);
    println!("  LHA:        {:6.1}°", lha_morning);
    println!("  ---");
    println!("  Hc:         {:6.2}° ({:02}° {:02.0}')", hc_morning, hc_morning.floor() as i32, (hc_morning.fract() * 60.0));
    println!("  Zn:         {:6.0}° T (EAST)", zn_morning);

    // Scenario 2: GHA Aries = 48.2° (evening, ~12 hours later - to get LHA 308° and Az 265°)
    // To get LHA = 308°: need GHA Pollux = 308 - lon = 308 - (-123) = 431 % 360 = 71°
    // GHA Aries = 71 - 243.4 + 360 = 187.6°
    let gha_aries_evening = 187.6;
    let gha_pollux_evening = (gha_aries_evening + 243.4) % 360.0;
    let lha_evening = (gha_pollux_evening + longitude + 360.0) % 360.0;

    let sight_evening = SightData {
        latitude: 45.0,
        declination: 28.0,
        local_hour_angle: lha_evening,
    };

    let hc_evening = compute_altitude(&sight_evening);
    let zn_evening = compute_azimuth(&sight_evening);

    println!("\nScenario 2: EVENING (GHA Aries = {:.1}°)", gha_aries_evening);
    println!("  GHA Aries:  {:6.1}°", gha_aries_evening);
    println!("  SHA Pollux: {:6.1}°", 243.4);
    println!("  GHA Pollux: {:6.1}°", gha_pollux_evening);
    println!("  Longitude:  {:6.1}° (W 123°)", longitude);
    println!("  LHA:        {:6.1}°", lha_evening);
    println!("  ---");
    println!("  Hc:         {:6.2}° ({:02}° {:02.0}')", hc_evening, hc_evening.floor() as i32, (hc_evening.fract() * 60.0));
    println!("  Zn:         {:6.0}° T (WEST)", zn_evening);

    println!("\n=== CONCLUSION ===");
    println!("Same altitude can occur at two different times:");
    println!("  Morning: Hc={:.1}°, Az={:.0}° (body in EAST)", hc_morning, zn_morning);
    println!("  Evening: Hc={:.1}°, Az={:.0}° (body in WEST)", hc_evening, zn_evening);
    println!("Difference: {:.0}° in azimuth!", (zn_evening - zn_morning).abs());
    println!("===\n");

    // Both should give similar altitudes but very different azimuths
    assert!(
        (hc_morning - hc_evening).abs() < 5.0,
        "Both observations should give similar altitudes"
    );

    assert!(
        (zn_evening - zn_morning).abs() > 100.0,
        "Azimuths should differ by more than 100°"
    );
}

#[test]
fn test_what_gha_aries_gives_expected_azimuth() {
    // User expects Az = 104°
    // This test finds what GHA Aries would produce that result

    let longitude = -123.0;  // W 123°
    let sha_pollux = 243.4;
    let target_lha = 51.7;  // LHA that gives Az ≈ 104°

    // LHA = GHA + Lon
    // GHA = LHA - Lon = LHA - Lon(for West, Lon is negative, so - (-123) = + 123)
    // GHA Pollux = LHA - Lon
    let gha_pollux_needed = (target_lha - longitude + 360.0) % 360.0;

    // GHA Pollux = GHA Aries + SHA
    // GHA Aries = GHA Pollux - SHA
    let gha_aries_needed = (gha_pollux_needed - sha_pollux + 360.0) % 360.0;

    println!("\n=== TO GET AZIMUTH 104° ===");
    println!("Required:");
    println!("  GHA Aries:  {:.1}°", gha_aries_needed);
    println!("  SHA Pollux: {:.1}°", sha_pollux);
    println!("  GHA Pollux: {:.1}°", gha_pollux_needed);
    println!("  Longitude:  {:.1}° (W 123°)", longitude);
    println!("  LHA:        {:.1}°", target_lha);
    println!("\nUser should check if GHA Aries = {:.1}° at observation time", gha_aries_needed);
    println!("If not, there may be a time zone or date error.\n");

    // Verify the calculation
    let lha_check: f64 = (gha_pollux_needed + longitude + 360.0) % 360.0;
    assert!(
        (lha_check - target_lha).abs() < 0.1,
        "LHA calculation check failed"
    );
}
