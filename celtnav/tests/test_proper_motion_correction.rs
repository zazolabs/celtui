//! Test that proper motion correction is being applied correctly

use celtnav::almanac::{get_body_position, CelestialBody};
use celtnav::dms_to_decimal;
use chrono::{NaiveDate, NaiveTime, TimeZone, Utc};

#[test]
fn test_hamal_proper_motion_2015() {
    // Hamal observation on 2015-09-10 06:32:22 UTC
    let datetime = Utc.from_utc_datetime(
        &NaiveDate::from_ymd_opt(2015, 9, 10).unwrap()
            .and_time(NaiveTime::from_hms_opt(6, 32, 22).unwrap())
    );

    let hamal_pos = get_body_position(CelestialBody::Star("Hamal".to_string()), datetime)
        .expect("Failed to get Hamal position");

    // Expected 2015 values from almanac:
    // SHA: 327°58.8' = 327.98°
    // Dec: 23°32.1' N = 23.535°

    // GHA Aries at this time
    let gha_aries = celtnav::almanac::gha_aries(datetime);
    println!("\n=== HAMAL PROPER MOTION TEST (2015-09-10) ===");
    println!("GHA Aries: {:.4}°", gha_aries);

    // Calculate expected GHA Hamal
    let expected_sha_2015 = dms_to_decimal(327, 58, 48.0);  // 327°58.8'
    let expected_gha_hamal = (gha_aries + expected_sha_2015) % 360.0;

    println!("\nExpected (2015 almanac):");
    println!("  SHA: 327°58.8' ({:.4}°)", expected_sha_2015);
    println!("  GHA: {:.4}°", expected_gha_hamal);
    println!("  Dec: 23°32.1' N ({:.4}°)", dms_to_decimal(23, 32, 6.0));

    println!("\nComputed with proper motion:");
    println!("  GHA: {:.4}°", hamal_pos.gha);
    println!("  Dec: {:.4}°", hamal_pos.declination);

    // Calculate the implied SHA
    let computed_sha = (hamal_pos.gha - gha_aries + 360.0) % 360.0;
    println!("  SHA (implied): {:.4}°", computed_sha);

    // Check if proper motion correction was applied
    let gha_diff_arcmin = (hamal_pos.gha - expected_gha_hamal).abs() * 60.0;
    let dec_diff_arcmin = (hamal_pos.declination - dms_to_decimal(23, 32, 6.0)).abs() * 60.0;
    let sha_diff_arcmin = (computed_sha - expected_sha_2015).abs() * 60.0;

    println!("\nDifferences from 2015 almanac:");
    println!("  GHA: {:.2}' ({:.1}\" per year over 9 years)", gha_diff_arcmin, gha_diff_arcmin * 60.0 / 9.0);
    println!("  Dec: {:.2}' ({:.1}\" per year over 9 years)", dec_diff_arcmin, dec_diff_arcmin * 60.0 / 9.0);
    println!("  SHA: {:.2}' ({:.1}\" per year over 9 years)", sha_diff_arcmin, sha_diff_arcmin * 60.0 / 9.0);

    // Assert SHA is within 1 arcminute of expected (proper motion should bring it close)
    assert!(
        sha_diff_arcmin < 1.0,
        "SHA difference too large: {:.2}' (expected < 1')",
        sha_diff_arcmin
    );

    // Assert Dec is within 1 arcminute of expected
    assert!(
        dec_diff_arcmin < 1.0,
        "Declination difference too large: {:.2}' (expected < 1')",
        dec_diff_arcmin
    );

    println!("\n✓ Proper motion correction working!");
}

#[test]
fn test_deneb_proper_motion_2015() {
    let datetime = Utc.from_utc_datetime(
        &NaiveDate::from_ymd_opt(2015, 9, 10).unwrap()
            .and_time(NaiveTime::from_hms_opt(6, 33, 47).unwrap())
    );

    let deneb_pos = get_body_position(CelestialBody::Star("Deneb".to_string()), datetime)
        .expect("Failed to get Deneb position");

    // Expected 2015 values: SHA 49°30.0', Dec 45°20.6' N
    let expected_sha_2015 = dms_to_decimal(49, 30, 0.0);
    let expected_dec_2015 = dms_to_decimal(45, 20, 36.0);

    let gha_aries = celtnav::almanac::gha_aries(datetime);
    let computed_sha = (deneb_pos.gha - gha_aries + 360.0) % 360.0;

    let sha_diff_arcmin = (computed_sha - expected_sha_2015).abs() * 60.0;
    let dec_diff_arcmin = (deneb_pos.declination - expected_dec_2015).abs() * 60.0;

    println!("\n=== DENEB PROPER MOTION TEST (2015-09-10) ===");
    println!("Expected SHA: 49°30.0'  ({:.4}°)", expected_sha_2015);
    println!("Computed SHA: {:.4}°", computed_sha);
    println!("Difference: {:.2}'", sha_diff_arcmin);

    println!("\nExpected Dec: 45°20.6' N ({:.4}°)", expected_dec_2015);
    println!("Computed Dec: {:.4}°", deneb_pos.declination);
    println!("Difference: {:.2}'", dec_diff_arcmin);

    assert!(
        sha_diff_arcmin < 1.0,
        "SHA difference too large: {:.2}'",
        sha_diff_arcmin
    );

    assert!(
        dec_diff_arcmin < 1.0,
        "Dec difference too large: {:.2}'",
        dec_diff_arcmin
    );

    println!("\n✓ Proper motion correction working!");
}
