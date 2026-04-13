//! Integration tests for sight reduction functions
//!
//! These tests verify the sight reduction calculations used in celestial navigation,
//! including computed altitude, azimuth, intercept, and altitude corrections.

use approx::assert_relative_eq;
use celtnav::sight_reduction::{
    compute_altitude, compute_azimuth, compute_intercept,
    apply_refraction_correction, apply_dip_correction,
    apply_semidiameter_correction, apply_parallax_correction,
    SightData, AltitudeCorrections,
};

/// Test computed altitude (Hc) calculation
///
/// Using known values from Sight Reduction Tables (Pub. 229)
/// Latitude: 40°N, Declination: 20°N, LHA: 30°
/// Using the formula: sin(Hc) = sin(Lat)*sin(Dec) + cos(Lat)*cos(Dec)*cos(LHA)
/// sin(Hc) = sin(40)*sin(20) + cos(40)*cos(20)*cos(30)
/// sin(Hc) = 0.6428*0.3420 + 0.7660*0.9397*0.8660 = 0.2198 + 0.6237 = 0.8435
/// Hc = arcsin(0.8435) = 57.5°
#[test]
fn test_compute_altitude_pub229() {
    let sight_data = SightData {
        latitude: 40.0,
        declination: 20.0,
        local_hour_angle: 30.0,
    };

    let hc = compute_altitude(&sight_data);

    // Computed value is approximately 57.5°
    assert_relative_eq!(hc, 57.5, epsilon = 1.0);
}

/// Test computed altitude at zenith
///
/// When a body is directly overhead (latitude = declination, LHA = 0),
/// the altitude should be 90°
#[test]
fn test_compute_altitude_zenith() {
    let sight_data = SightData {
        latitude: 20.0,
        declination: 20.0,
        local_hour_angle: 0.0,
    };

    let hc = compute_altitude(&sight_data);

    assert_relative_eq!(hc, 90.0, epsilon = 0.01);
}

/// Test computed altitude on horizon
///
/// At equator (lat=0), a body on celestial equator (dec=0)
/// at LHA=90° (east) should be on horizon (alt=0)
#[test]
fn test_compute_altitude_horizon() {
    let sight_data = SightData {
        latitude: 0.0,
        declination: 0.0,
        local_hour_angle: 90.0,
    };

    let hc = compute_altitude(&sight_data);

    assert_relative_eq!(hc, 0.0, epsilon = 0.01);
}

/// Test computed altitude with same name latitude and declination
///
/// Latitude: 35°N, Declination: 15°N, LHA: 0° (on meridian)
/// Expected: Hc = 90° - (Lat - Dec) = 90° - 20° = 70°
#[test]
fn test_compute_altitude_same_name() {
    let sight_data = SightData {
        latitude: 35.0,
        declination: 15.0,
        local_hour_angle: 0.0,
    };

    let hc = compute_altitude(&sight_data);

    assert_relative_eq!(hc, 70.0, epsilon = 0.01);
}

/// Test computed altitude with contrary name latitude and declination
///
/// Latitude: 35°N, Declination: -15°S, LHA: 0° (on meridian)
/// Expected: Hc = 90° - (Lat + |Dec|) = 90° - 50° = 40°
#[test]
fn test_compute_altitude_contrary_name() {
    let sight_data = SightData {
        latitude: 35.0,
        declination: -15.0,
        local_hour_angle: 0.0,
    };

    let hc = compute_altitude(&sight_data);

    assert_relative_eq!(hc, 40.0, epsilon = 0.01);
}

/// Test azimuth calculation
///
/// Using known values from Sight Reduction Tables
/// Latitude: 40°N, Declination: 20°N, LHA: 30°
/// Body is east of meridian (LHA < 180), so azimuth should be less than 180°
#[test]
fn test_compute_azimuth_pub229() {
    let sight_data = SightData {
        latitude: 40.0,
        declination: 20.0,
        local_hour_angle: 30.0,
    };

    let zn = compute_azimuth(&sight_data);

    // LHA = 30° means body is east of meridian
    // Azimuth should be between 90° and 180° (southeast quadrant)
    assert!(zn > 90.0 && zn < 180.0,
            "Azimuth should be in southeast quadrant, got {}", zn);
}

/// Test azimuth on meridian (south)
///
/// When LHA = 0°, body is on meridian (south for northern hemisphere)
/// Azimuth should be 180°
#[test]
fn test_compute_azimuth_meridian_south() {
    let sight_data = SightData {
        latitude: 40.0,
        declination: 20.0,
        local_hour_angle: 0.0,
    };

    let zn = compute_azimuth(&sight_data);

    assert_relative_eq!(zn, 180.0, epsilon = 0.1);
}

/// Test azimuth due east
///
/// When LHA = 90°, body should be near east (90°)
#[test]
fn test_compute_azimuth_east() {
    let sight_data = SightData {
        latitude: 0.0,
        declination: 0.0,
        local_hour_angle: 90.0,
    };

    let zn = compute_azimuth(&sight_data);

    assert_relative_eq!(zn, 90.0, epsilon = 0.1);
}

/// Test azimuth due west
///
/// When LHA = 270°, body should be near west (270°)
#[test]
fn test_compute_azimuth_west() {
    let sight_data = SightData {
        latitude: 0.0,
        declination: 0.0,
        local_hour_angle: 270.0,
    };

    let zn = compute_azimuth(&sight_data);

    assert_relative_eq!(zn, 270.0, epsilon = 0.1);
}

/// Test intercept calculation (toward)
///
/// If observed altitude (Ho) > computed altitude (Hc), intercept is positive (toward)
#[test]
fn test_compute_intercept_toward() {
    let sight_data = SightData {
        latitude: 40.0,
        declination: 20.0,
        local_hour_angle: 30.0,
    };

    // First compute Hc to know what it is
    let hc = compute_altitude(&sight_data);
    let observed_altitude = hc + 2.0; // Ho is 2° higher than Hc

    let intercept = compute_intercept(&sight_data, observed_altitude);

    // Ho > Hc by 2°, intercept should be 2° * 60 = 120 NM toward
    assert!(intercept > 0.0, "Intercept should be positive (toward)");
    assert_relative_eq!(intercept, 120.0, epsilon = 1.0);
}

/// Test intercept calculation (away)
///
/// If observed altitude (Ho) < computed altitude (Hc), intercept is negative (away)
#[test]
fn test_compute_intercept_away() {
    let sight_data = SightData {
        latitude: 40.0,
        declination: 20.0,
        local_hour_angle: 30.0,
    };

    // First compute Hc to know what it is
    let hc = compute_altitude(&sight_data);
    let observed_altitude = hc - 0.5; // Ho is 0.5° lower than Hc

    let intercept = compute_intercept(&sight_data, observed_altitude);

    // Ho < Hc by 0.5°, intercept should be -0.5° * 60 = -30 NM away
    assert!(intercept < 0.0, "Intercept should be negative (away)");
    assert_relative_eq!(intercept, -30.0, epsilon = 1.0);
}

/// Test refraction correction
///
/// Atmospheric refraction causes celestial bodies to appear higher than they are.
/// Refraction is greatest at the horizon (~34') and zero at zenith.
///
/// Formula (Bennett): R = cot(h + 7.31/(h + 4.4))
/// where h is apparent altitude in degrees, R is in arcminutes
#[test]
fn test_refraction_correction_horizon() {
    // At horizon (0°), refraction is approximately 34 arcminutes (0.57°)
    let apparent_altitude = 0.0;
    let correction = apply_refraction_correction(apparent_altitude);

    // Should be approximately -0.57° (negative because we subtract to get true altitude)
    assert_relative_eq!(correction, -0.57, epsilon = 0.05);
}

/// Test refraction correction at 45° altitude
#[test]
fn test_refraction_correction_45deg() {
    // At 45°, refraction is approximately 1 arcminute (0.017°)
    let apparent_altitude = 45.0;
    let correction = apply_refraction_correction(apparent_altitude);

    // Should be approximately -0.017°
    assert_relative_eq!(correction, -0.017, epsilon = 0.005);
}

/// Test refraction correction at zenith
#[test]
fn test_refraction_correction_zenith() {
    // At zenith (90°), refraction is effectively zero
    let apparent_altitude = 90.0;
    let correction = apply_refraction_correction(apparent_altitude);

    // Should be very close to 0
    assert!(correction.abs() < 0.001);
}

/// Test dip correction
///
/// Dip is the angle between the horizontal and the visible horizon
/// due to observer's height above sea level.
///
/// Formula: Dip (arcmin) = 1.76 * sqrt(height_meters)
/// or in degrees: Dip = 0.0293 * sqrt(height_meters)
#[test]
fn test_dip_correction_sea_level() {
    // At sea level (height = 0), dip should be 0
    let height_meters = 0.0;
    let correction = apply_dip_correction(height_meters);

    assert_eq!(correction, 0.0);
}

/// Test dip correction at 10 meters height
#[test]
fn test_dip_correction_10m() {
    // At 10m height: Dip = 1.76 * sqrt(10) ≈ 5.6' ≈ 0.093°
    let height_meters = 10.0;
    let correction = apply_dip_correction(height_meters);

    // Dip is always negative (horizon appears lower)
    assert_relative_eq!(correction, -0.093, epsilon = 0.01);
}

/// Test dip correction at bridge height (20m)
#[test]
fn test_dip_correction_20m() {
    // At 20m height: Dip = 1.76 * sqrt(20) ≈ 7.9' ≈ 0.131°
    let height_meters = 20.0;
    let correction = apply_dip_correction(height_meters);

    assert_relative_eq!(correction, -0.131, epsilon = 0.01);
}

/// Test semi-diameter correction for lower limb
///
/// When observing the lower limb of the Sun or Moon, we need to add
/// the semi-diameter to get the center's altitude.
#[test]
fn test_semidiameter_correction_sun_lower_limb() {
    // Sun's semi-diameter is approximately 16 arcminutes (0.267°)
    let semidiameter = 0.267;
    let is_lower_limb = true;

    let correction = apply_semidiameter_correction(semidiameter, is_lower_limb);

    // For lower limb, add semi-diameter
    assert_relative_eq!(correction, 0.267, epsilon = 0.001);
}

/// Test semi-diameter correction for upper limb
#[test]
fn test_semidiameter_correction_sun_upper_limb() {
    // Sun's semi-diameter is approximately 16 arcminutes (0.267°)
    let semidiameter = 0.267;
    let is_lower_limb = false;

    let correction = apply_semidiameter_correction(semidiameter, is_lower_limb);

    // For upper limb, subtract semi-diameter
    assert_relative_eq!(correction, -0.267, epsilon = 0.001);
}

/// Test parallax correction for the Moon
///
/// Parallax is the difference in apparent position due to observer's position
/// on Earth's surface. It's significant for the Moon (~1°) but negligible for stars.
///
/// Horizontal parallax for Moon ≈ 57 arcminutes (0.95°)
/// Parallax correction = HP * cos(apparent_altitude)
#[test]
fn test_parallax_correction_moon_horizon() {
    // Moon at horizon: parallax is maximum (horizontal parallax)
    let horizontal_parallax = 0.95; // degrees
    let apparent_altitude = 0.0;

    let correction = apply_parallax_correction(horizontal_parallax, apparent_altitude);

    // At horizon, parallax = HP
    assert_relative_eq!(correction, 0.95, epsilon = 0.01);
}

/// Test parallax correction for the Moon at zenith
#[test]
fn test_parallax_correction_moon_zenith() {
    // Moon at zenith: parallax is zero
    let horizontal_parallax = 0.95;
    let apparent_altitude = 90.0;

    let correction = apply_parallax_correction(horizontal_parallax, apparent_altitude);

    // At zenith, cos(90°) = 0, so parallax = 0
    assert!(correction.abs() < 0.01);
}

/// Test parallax correction for the Moon at 45° altitude
#[test]
fn test_parallax_correction_moon_45deg() {
    // Moon at 45°: parallax = HP * cos(45°) = HP * 0.707
    let horizontal_parallax = 0.95;
    let apparent_altitude = 45.0;

    let correction = apply_parallax_correction(horizontal_parallax, apparent_altitude);

    // At 45°, parallax ≈ 0.95 * 0.707 ≈ 0.67
    assert_relative_eq!(correction, 0.67, epsilon = 0.05);
}

/// Test complete altitude correction chain for Sun lower limb observation
#[test]
fn test_complete_sun_corrections() {
    let corrections = AltitudeCorrections {
        refraction: apply_refraction_correction(30.0),
        dip: apply_dip_correction(10.0),
        semidiameter: apply_semidiameter_correction(0.267, true), // lower limb
        parallax: 0.0, // negligible for Sun
    };

    let sextant_altitude = 30.0;
    let observed_altitude = sextant_altitude
        + corrections.refraction
        + corrections.dip
        + corrections.semidiameter
        + corrections.parallax;

    // Verify corrections are applied correctly
    assert!(corrections.refraction < 0.0, "Refraction should be negative");
    assert!(corrections.dip < 0.0, "Dip should be negative");
    assert!(corrections.semidiameter > 0.0, "SD should be positive for lower limb");

    // Total correction should be small
    let total_correction = observed_altitude - sextant_altitude;
    assert!(total_correction.abs() < 1.0,
            "Total correction should be less than 1°, got {}", total_correction);
}

/// Test that azimuth is always in valid range [0, 360)
#[test]
fn test_azimuth_range_validity() {
    let test_cases = vec![
        (40.0, 20.0, 0.0),
        (40.0, 20.0, 90.0),
        (40.0, 20.0, 180.0),
        (40.0, 20.0, 270.0),
        (-30.0, -15.0, 45.0),
    ];

    for (lat, dec, lha) in test_cases {
        let sight_data = SightData {
            latitude: lat,
            declination: dec,
            local_hour_angle: lha,
        };
        let zn = compute_azimuth(&sight_data);

        assert!(zn >= 0.0 && zn < 360.0,
                "Azimuth must be in range [0, 360), got {} for lat={}, dec={}, lha={}",
                zn, lat, dec, lha);
    }
}
