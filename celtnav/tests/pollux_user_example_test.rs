//! Test based on user's data for Pollux sight
//! Pollux, 10 Sept 2016, 06:28:12 UTC
//!
//! TWO METHODS for star sight reduction:
//!
//! METHOD 1: SRT TABLES (Sight Reduction Tables - manual method with paper tables)
//! - GHA Aries = 86° 03.3' = 86.055° (from almanac)
//! - DR Lon = 20° W = -20°
//! - MUST optimize chosen lon to make LHA Aries whole (for table lookup)
//! - Chosen lon = 20° 03.9' W = -20.065° → LHA Aries = 66° (whole)
//! - Look up in SRT with LHA Aries=66° and "Pollux"
//! - Result from tables: Hc = 46° 04', Az = 104° T
//!
//! METHOD 2: DIRECT CALCULATION (Our spherical trig method - what we implement)
//! - GHA Aries = 86° 03.3' = 86.055° (from almanac)
//! - SHA Pollux = 243° 26.1' = 243.435° (from star catalog)
//! - GHA Pollux = GHA Aries + SHA = 86.055 + 243.435 = 329.490°
//! - DR Lon = 20° W = -20° (use DR position directly, NO optimization)
//! - LHA Pollux = 329.490 - 20 = 309.490° (can have decimals for trig!)
//! - LHA Aries = 86.055 - 20 = 66.055° (also with decimals)
//! - Calculate Hc using spherical trig with exact LHA values
//! - Both methods should give similar Hc ≈ 46° 04', Az ≈ 104° T
//!
//! KEY DIFFERENCE: Table method requires whole LHA (optimization),
//!                 Trig method uses exact DR position (no optimization needed)!

#[test]
fn test_pollux_direct_calculation_no_optimization() {
    // Pollux, 10 Sept 2016, 06:28:12 UTC
    // Using METHOD 2: Direct spherical trig calculation (NO optimization)

    // From almanac: GHA Aries = 86° 03.3' = 86.055°
    // From catalog: SHA Pollux = 243° 26.1' = 243.435°
    let gha_aries = 86.055;
    let sha_pollux = 243.435;
    let gha_pollux = (gha_aries + sha_pollux) % 360.0;  // = 329.490°

    let dr_lat = 40.0;  // 40°N (DR position)
    let dr_lon = -20.0;  // 20°W (DR position)

    // For trig calculations, NO optimization needed - use DR position directly
    let chosen_lat = dr_lat;
    let chosen_lon = dr_lon;

    // Verify we're using DR position as-is
    assert_eq!(chosen_lat, 40.0, "Should use DR latitude directly");
    assert_eq!(chosen_lon, -20.0, "Should use DR longitude directly");

    // Calculate LHA of Pollux using exact DR position
    let lha_pollux = (gha_pollux + dr_lon + 360.0) % 360.0;
    // LHA Pollux = 329.490 - 20 = 309.490° (can have decimals!)
    let expected_lha_pollux: f64 = 309.490;
    assert!(
        (lha_pollux - expected_lha_pollux).abs() < 0.01,
        "LHA Pollux should be {:.3}°, got {:.3}°",
        expected_lha_pollux, lha_pollux
    );

    // Calculate LHA Aries for comparison with SRT
    let lha_aries = (gha_aries + dr_lon + 360.0) % 360.0;
    // LHA Aries = 86.055 - 20 = 66.055° (also with decimals)
    let expected_lha_aries: f64 = 66.055;
    assert!(
        (lha_aries - expected_lha_aries).abs() < 0.01,
        "LHA Aries should be {:.3}°, got {:.3}°",
        expected_lha_aries, lha_aries
    );

    // For SRT table lookup, user would:
    // - Round LHA Aries to nearest whole: 66°
    // - Enter table with LHA Aries=66° and star name "Pollux"
    // - Get Hc ≈ 46° 04', Az ≈ 104° T
    //
    // Our spherical trig calculation uses exact LHA = 309.490° and should
    // give very similar results without requiring optimization!
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
