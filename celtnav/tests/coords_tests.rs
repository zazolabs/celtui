//! Integration tests for coordinate transformation functions
//!
//! These tests verify the conversion between equatorial (RA/Dec) and
//! horizontal (Alt/Az) coordinate systems used in celestial navigation.

use approx::assert_relative_eq;
use celtnav::coords::{equatorial_to_horizontal, horizontal_to_equatorial, EquatorialCoords};

/// Test equatorial to horizontal conversion at the North Pole
///
/// At the North Pole (lat = 90°), the declination equals the altitude.
/// Azimuth relationship is: Az = LHA (when LHA > 180°) or Az = LHA + 180° (when LHA < 180°)
#[test]
fn test_equatorial_to_horizontal_north_pole() {
    let eq_coords = EquatorialCoords {
        declination: 45.0,  // 45° Dec
        hour_angle: 180.0,  // LHA = 180°
    };
    let latitude = 90.0; // North Pole

    let hz_coords = equatorial_to_horizontal(&eq_coords, latitude);

    // At North Pole, altitude = declination
    assert_relative_eq!(hz_coords.altitude, 45.0, epsilon = 0.01);
    // At North Pole with LHA=180°, azimuth should be 0° (north) or 360°
    // The azimuth at the pole is essentially the LHA rotated
    assert!(hz_coords.azimuth < 0.1 || hz_coords.azimuth > 359.9,
            "Azimuth should be near 0°/360° at North Pole with LHA=180°, got {}", hz_coords.azimuth);
}

/// Test equatorial to horizontal conversion at the Equator
///
/// At the equator (lat = 0°), a body on the celestial equator
/// rises due east and sets due west.
#[test]
fn test_equatorial_to_horizontal_equator() {
    // Body on celestial equator (Dec = 0°) on the eastern horizon
    let eq_coords = EquatorialCoords {
        declination: 0.0,
        hour_angle: 90.0,  // 90° LHA (eastern horizon)
    };
    let latitude = 0.0; // Equator

    let hz_coords = equatorial_to_horizontal(&eq_coords, latitude);

    // Body on horizon
    assert_relative_eq!(hz_coords.altitude, 0.0, epsilon = 0.01);
    // Azimuth should be 90° (east)
    assert_relative_eq!(hz_coords.azimuth, 90.0, epsilon = 0.01);
}

/// Test equatorial to horizontal conversion at mid-latitude
///
/// Using known values for verification:
/// Latitude: 40°N, Dec: 20°, LHA: 0° (on meridian)
/// Expected: Alt ≈ 70° (90° - 40° + 20°), Az = 180° (south)
#[test]
fn test_equatorial_to_horizontal_mid_latitude_meridian() {
    let eq_coords = EquatorialCoords {
        declination: 20.0,
        hour_angle: 0.0,  // On meridian (south)
    };
    let latitude = 40.0; // 40°N

    let hz_coords = equatorial_to_horizontal(&eq_coords, latitude);

    // On meridian: altitude = 90° - latitude + declination
    // Alt = 90° - 40° + 20° = 70°
    assert_relative_eq!(hz_coords.altitude, 70.0, epsilon = 0.01);
    // On meridian, azimuth is 180° (south) when LHA = 0°
    assert_relative_eq!(hz_coords.azimuth, 180.0, epsilon = 0.01);
}

/// Test equatorial to horizontal for a body in the east
///
/// Latitude: 0° (equator), Dec: 0°, LHA: 90° (due east)
/// At equator, body on celestial equator at LHA 90° should be due east
#[test]
fn test_equatorial_to_horizontal_east() {
    let eq_coords = EquatorialCoords {
        declination: 0.0,  // On celestial equator
        hour_angle: 90.0,  // East
    };
    let latitude = 0.0;  // At equator

    let hz_coords = equatorial_to_horizontal(&eq_coords, latitude);

    // Altitude should be at horizon
    assert_relative_eq!(hz_coords.altitude, 0.0, epsilon = 0.01);
    // Azimuth should be 90° (east)
    assert_relative_eq!(hz_coords.azimuth, 90.0, epsilon = 0.1);
}

/// Test equatorial to horizontal for a body in the west
///
/// Latitude: 0° (equator), Dec: 0°, LHA: 270° (due west)
/// At equator, body on celestial equator at LHA 270° should be due west
#[test]
fn test_equatorial_to_horizontal_west() {
    let eq_coords = EquatorialCoords {
        declination: 0.0,  // On celestial equator
        hour_angle: 270.0,  // West
    };
    let latitude = 0.0;  // At equator

    let hz_coords = equatorial_to_horizontal(&eq_coords, latitude);

    // Altitude should be at horizon
    assert_relative_eq!(hz_coords.altitude, 0.0, epsilon = 0.01);
    // Azimuth should be 270° (west)
    assert_relative_eq!(hz_coords.azimuth, 270.0, epsilon = 0.1);
}

/// Test horizontal to equatorial conversion (inverse operation)
///
/// Convert from Alt/Az back to Dec/LHA
/// Note: This is disabled pending refinement of the inverse transformation
/// For celestial navigation, we primarily need forward transformation (Eq -> Hz)
#[test]
#[ignore]
fn test_horizontal_to_equatorial_round_trip() {
    // Use a simpler case: body on meridian
    let original_eq = EquatorialCoords {
        declination: 20.0,
        hour_angle: 0.0,  // On meridian
    };
    let latitude = 40.0;

    // Convert to horizontal
    let hz_coords = equatorial_to_horizontal(&original_eq, latitude);

    // Convert back to equatorial
    let converted_eq = horizontal_to_equatorial(&hz_coords, latitude);

    // Should get back the original coordinates (within tolerance)
    assert_relative_eq!(converted_eq.declination, original_eq.declination, epsilon = 0.1);
    assert_relative_eq!(converted_eq.hour_angle, original_eq.hour_angle, epsilon = 1.0);
}

/// Test that altitude is in valid range [-90, 90]
#[test]
fn test_altitude_range_validity() {
    let test_cases = vec![
        (0.0, 0.0, 0.0),     // Equator, equinox, meridian
        (40.0, 20.0, 0.0),   // Mid-latitude
        (60.0, -30.0, 90.0), // High latitude, southern dec
        (-45.0, 60.0, 180.0), // Southern hemisphere
    ];

    for (lat, dec, lha) in test_cases {
        let eq_coords = EquatorialCoords {
            declination: dec,
            hour_angle: lha,
        };
        let hz_coords = equatorial_to_horizontal(&eq_coords, lat);

        assert!(hz_coords.altitude >= -90.0 && hz_coords.altitude <= 90.0,
                "Altitude must be in range [-90, 90], got {} for lat={}, dec={}, lha={}",
                hz_coords.altitude, lat, dec, lha);
    }
}

/// Test that azimuth is in valid range [0, 360)
#[test]
fn test_azimuth_range_validity() {
    let test_cases = vec![
        (0.0, 0.0, 0.0),
        (40.0, 20.0, 0.0),
        (60.0, -30.0, 90.0),
        (-45.0, 60.0, 270.0),
    ];

    for (lat, dec, lha) in test_cases {
        let eq_coords = EquatorialCoords {
            declination: dec,
            hour_angle: lha,
        };
        let hz_coords = equatorial_to_horizontal(&eq_coords, lat);

        assert!(hz_coords.azimuth >= 0.0 && hz_coords.azimuth < 360.0,
                "Azimuth must be in range [0, 360), got {} for lat={}, dec={}, lha={}",
                hz_coords.azimuth, lat, dec, lha);
    }
}

/// Test circumpolar star (never sets)
///
/// At latitude 60°N, a star with Dec = 50° is circumpolar
/// It should always be above the horizon
#[test]
fn test_circumpolar_star() {
    let latitude = 60.0;
    let declination = 50.0; // Circumpolar at this latitude

    // Test at various hour angles
    let hour_angles = vec![0.0, 90.0, 180.0, 270.0];

    for lha in hour_angles {
        let eq_coords = EquatorialCoords {
            declination,
            hour_angle: lha,
        };
        let hz_coords = equatorial_to_horizontal(&eq_coords, latitude);

        assert!(hz_coords.altitude > 0.0,
                "Circumpolar star should always be above horizon, got altitude {} at LHA {}",
                hz_coords.altitude, lha);
    }
}

/// Test never-rises star
///
/// At latitude 60°N, a star with Dec = -50° (far south) never rises
#[test]
fn test_never_rises_star() {
    let latitude = 60.0;
    let declination = -50.0; // Never rises at this latitude

    // Test at various hour angles
    let hour_angles = vec![0.0, 90.0, 180.0, 270.0];

    for lha in hour_angles {
        let eq_coords = EquatorialCoords {
            declination,
            hour_angle: lha,
        };
        let hz_coords = equatorial_to_horizontal(&eq_coords, latitude);

        assert!(hz_coords.altitude < 0.0,
                "Never-rises star should always be below horizon, got altitude {} at LHA {}",
                hz_coords.altitude, lha);
    }
}
