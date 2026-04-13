//! Test based on user's correct manual calculation
//! Pollux, 10 Sept 2016, 06:28:12 UTC
//!
//! User's manual calculation (CORRECT):
//! - GHA Pollux = 86° 03.3' = 86.055°
//! - DR Lon ≈ 20° W = -20°
//! - Chosen Lon = 20° 03.9' W = -20.065° (to make LHA Pollux = 66° exactly)
//! - LHA = GHA - Lon = 86.055° - 20.065° = 66° (exactly whole)
//! - Using Pub 249 Vol 1 with LHA = 66°: Hc = 46° 04', Az = 104° T
//!
//! CRITICAL: We optimize to make LHA of the STAR whole, NOT LHA Aries!

use celtnav::sight_reduction::optimize_chosen_position;

#[test]
fn test_pollux_user_example_optimization() {
    // Pollux, 10 Sept 2016, 06:28:12 UTC
    // GHA Pollux = 86° 03.3' = 86.055°
    // DR Lon ≈ 20° W = -20°
    // Expected chosen lon = 20° 03.9' W = -20.065°
    // Expected LHA = 66° exactly

    let gha_pollux = 86.055;  // 86° 03.3' (GHA of Pollux, already includes GHA Aries + SHA)
    let dr_lat = 40.0;  // Assume 40°N
    let dr_lon = -20.0;  // 20°W

    // Optimize using GHA of Pollux (NOT GHA Aries!)
    let (chosen_lat, chosen_lon) = optimize_chosen_position(dr_lat, dr_lon, gha_pollux);

    // Latitude should be rounded to 40°
    assert_eq!(chosen_lat, 40.0, "Latitude should be rounded to 40°");

    // Check chosen longitude is close to -20.065° (20° 03.9' W)
    // Using signed convention: West is negative
    let expected_lon = -20.065;
    assert!(
        (chosen_lon - expected_lon).abs() < 0.01,
        "Chosen lon should be near {:.3}°, got {:.3}°",
        expected_lon, chosen_lon
    );

    // Calculate LHA using formula: LHA = GHA + Longitude (where West is negative)
    // This is equivalent to: LHA = GHA - abs(Lon_West)
    let lha = (gha_pollux + chosen_lon + 360.0) % 360.0;

    // Should be exactly 66°
    assert!(
        (lha - 66.0).abs() < 0.01,
        "LHA should be 66°, got {:.2}°",
        lha
    );

    // Verify it's a whole number
    let lha_frac = lha - lha.round();
    assert!(
        lha_frac.abs() < 0.01,
        "LHA should be whole number, fractional part: {:.4}",
        lha_frac
    );
}

#[test]
fn test_lha_formula_west_longitude() {
    // Verify the LHA formula for West longitude
    // LHA = GHA + Longitude (where West is negative)
    // OR equivalently: LHA = GHA - abs(Longitude_West)

    let gha = 86.055;
    let lon_west_signed = -20.065;  // 20° 03.9' W (negative for West)

    // Method 1: Direct formula with signed longitude
    let lha1: f64 = (gha + lon_west_signed + 360.0) % 360.0;

    // Method 2: Subtract absolute value for West
    let lon_west_abs: f64 = 20.065;
    let lha2: f64 = (gha - lon_west_abs + 360.0) % 360.0;

    // Both methods should give same result
    assert!(
        (lha1 - lha2).abs() < 0.001,
        "Both LHA calculation methods should agree"
    );

    // Should equal 66°
    assert!((lha1 - 66.0).abs() < 0.01, "LHA should be 66°");
}

#[test]
fn test_pollux_lha_aries_vs_lha_pollux() {
    // This test demonstrates WHY the recent "fix" was wrong
    //
    // Given:
    // - GHA Aries = 78° 57.6' + correction 7° 05.7' = 86° 03.3' - 243° 24.7' SHA = -157° 21.4' = 202° 38.6'
    // - Wait, let me recalculate...
    // - Actually, user gave us GHA Pollux directly = 86° 03.3'
    // - GHA Pollux = GHA Aries + SHA Pollux
    // - If GHA Pollux = 86.055°, and SHA Pollux ≈ 243.4°
    // - Then GHA Aries = 86.055° - 243.4° = -157.345° = 202.655°

    let gha_pollux = 86.055;
    let sha_pollux = 243.4;  // Approximate SHA of Pollux
    let gha_aries = (gha_pollux - sha_pollux + 360.0) % 360.0;  // ≈ 202.655°

    let dr_lon = -20.0;  // 20° W

    // CORRECT: Optimize to make LHA Pollux whole
    let chosen_lon_correct: f64 = {
        let lha_with_dr: f64 = (gha_pollux + dr_lon + 360.0) % 360.0;
        let lha_frac = lha_with_dr - lha_with_dr.floor();
        let adjustment = if lha_frac <= 0.5 {
            -lha_frac
        } else {
            1.0 - lha_frac
        };
        dr_lon + adjustment
    };

    // WRONG: Optimize to make LHA Aries whole
    let chosen_lon_wrong: f64 = {
        let lha_aries_with_dr: f64 = (gha_aries + dr_lon + 360.0) % 360.0;
        let lha_aries_frac = lha_aries_with_dr - lha_aries_with_dr.floor();
        let adjustment = if lha_aries_frac <= 0.5 {
            -lha_aries_frac
        } else {
            1.0 - lha_aries_frac
        };
        dr_lon + adjustment
    };

    // Calculate resulting LHAs
    let lha_pollux_correct: f64 = (gha_pollux + chosen_lon_correct + 360.0) % 360.0;
    let lha_pollux_wrong: f64 = (gha_pollux + chosen_lon_wrong + 360.0) % 360.0;

    // With correct method, LHA Pollux should be whole
    assert!(
        (lha_pollux_correct - lha_pollux_correct.round()).abs() < 0.01,
        "CORRECT method: LHA Pollux should be whole, got {:.2}°",
        lha_pollux_correct
    );

    // With wrong method, LHA Pollux will NOT be whole (unless by coincidence)
    // In this case, the two methods give different results
    assert!(
        (chosen_lon_correct - chosen_lon_wrong).abs() > 0.05,
        "The two methods should give different chosen longitudes"
    );

    println!("GHA Pollux: {:.3}°", gha_pollux);
    println!("SHA Pollux: {:.3}°", sha_pollux);
    println!("GHA Aries: {:.3}°", gha_aries);
    println!();
    println!("CORRECT method (optimize LHA Pollux):");
    println!("  Chosen Lon: {:.3}° W", chosen_lon_correct.abs());
    println!("  LHA Pollux: {:.3}°", lha_pollux_correct);
    println!();
    println!("WRONG method (optimize LHA Aries):");
    println!("  Chosen Lon: {:.3}° W", chosen_lon_wrong.abs());
    println!("  LHA Pollux: {:.3}°", lha_pollux_wrong);
}
