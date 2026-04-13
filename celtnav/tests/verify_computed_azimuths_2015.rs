//! Verify that computed azimuths match Pub.249 Vol.1 table values
//! for the 2015 three-star fix

use celtnav::almanac::{get_body_position, CelestialBody};
use celtnav::sight_reduction::{compute_azimuth, SightData};
use celtnav::dms_to_decimal;
use chrono::{NaiveDate, NaiveTime, TimeZone, Utc};

#[test]
fn verify_hamal_azimuth_2015() {
    // User's Pub.249 data for Hamal:
    // AP: 50°N, 20°04.4'W
    // LHA Aries: 67°
    // Z: 239° (which IS Zn - no conversion needed)
    // Hc: 52°04'

    let ap_lat = 50.0;
    let ap_lon = -dms_to_decimal(20, 4, 24.0);  // 20°04.4'W

    let hamal_time = Utc.from_utc_datetime(
        &NaiveDate::from_ymd_opt(2015, 9, 10).unwrap()
            .and_time(NaiveTime::from_hms_opt(6, 32, 22).unwrap())
    );

    // Get Hamal position with proper motion correction
    let hamal_pos = get_body_position(CelestialBody::Star("Hamal".to_string()), hamal_time)
        .expect("Failed to get Hamal position");

    // Calculate LHA Hamal
    let lha_hamal = (hamal_pos.gha + ap_lon + 360.0) % 360.0;

    // Compute azimuth using spherical trigonometry
    let sight_data = SightData {
        latitude: ap_lat,
        declination: hamal_pos.declination,
        local_hour_angle: lha_hamal,
    };

    let computed_zn = compute_azimuth(&sight_data);

    println!("\n=== HAMAL AZIMUTH VERIFICATION (2015-09-10) ===");
    println!("AP: 50°N, 20°04.4'W");
    println!("GHA Hamal: {:.4}°", hamal_pos.gha);
    println!("LHA Hamal: {:.4}°", lha_hamal);
    println!("Dec Hamal: {:.4}°", hamal_pos.declination);
    println!("\nPub.249 Vol.1 gives:");
    println!("  Z (Zn): 239°");
    println!("\nComputed with spherical trig:");
    println!("  Zn: {:.1}°", computed_zn);
    println!("\nDifference: {:.1}°", (computed_zn - 239.0).abs());

    let azimuth_error = (computed_zn - 239.0).abs();

    // Azimuth should be within 2° of table value
    assert!(
        azimuth_error < 2.0,
        "Azimuth error too large: {:.1}° (expected < 2°)",
        azimuth_error
    );
}

#[test]
fn verify_pollux_azimuth_2015() {
    // User's data: Z=101° (Zn)
    let ap_lat = 50.0;
    let ap_lon = -dms_to_decimal(20, 3, 12.0);  // 20°03.2'W (for LHA Aries = 66°)

    let pollux_time = Utc.from_utc_datetime(
        &NaiveDate::from_ymd_opt(2015, 9, 10).unwrap()
            .and_time(NaiveTime::from_hms_opt(6, 28, 18).unwrap())
    );

    let pollux_pos = get_body_position(CelestialBody::Star("Pollux".to_string()), pollux_time)
        .expect("Failed to get Pollux position");

    let lha_pollux = (pollux_pos.gha + ap_lon + 360.0) % 360.0;

    let sight_data = SightData {
        latitude: ap_lat,
        declination: pollux_pos.declination,
        local_hour_angle: lha_pollux,
    };

    let computed_zn = compute_azimuth(&sight_data);

    println!("\n=== POLLUX AZIMUTH VERIFICATION (2015-09-10) ===");
    println!("Pub.249 Vol.1: Z (Zn) = 101°");
    println!("Computed:      Zn = {:.1}°", computed_zn);
    println!("Difference: {:.1}°", (computed_zn - 101.0).abs());

    let azimuth_error = (computed_zn - 101.0).abs();
    assert!(
        azimuth_error < 2.0,
        "Azimuth error too large: {:.1}°",
        azimuth_error
    );
}

#[test]
fn verify_deneb_azimuth_2015() {
    // User's data: Z=319° (Zn)
    let ap_lat = 50.0;
    let ap_lon = -dms_to_decimal(19, 25, 42.0);  // 19°25.7'W (for LHA Aries = 68°)

    let deneb_time = Utc.from_utc_datetime(
        &NaiveDate::from_ymd_opt(2015, 9, 10).unwrap()
            .and_time(NaiveTime::from_hms_opt(6, 33, 47).unwrap())
    );

    let deneb_pos = get_body_position(CelestialBody::Star("Deneb".to_string()), deneb_time)
        .expect("Failed to get Deneb position");

    let lha_deneb = (deneb_pos.gha + ap_lon + 360.0) % 360.0;

    let sight_data = SightData {
        latitude: ap_lat,
        declination: deneb_pos.declination,
        local_hour_angle: lha_deneb,
    };

    let computed_zn = compute_azimuth(&sight_data);

    println!("\n=== DENEB AZIMUTH VERIFICATION (2015-09-10) ===");
    println!("Pub.249 Vol.1: Z (Zn) = 319°");
    println!("Computed:      Zn = {:.1}°", computed_zn);
    println!("Difference: {:.1}°", (computed_zn - 319.0).abs());

    let azimuth_error = (computed_zn - 319.0).abs();
    assert!(
        azimuth_error < 2.0,
        "Azimuth error too large: {:.1}°",
        azimuth_error
    );
}
