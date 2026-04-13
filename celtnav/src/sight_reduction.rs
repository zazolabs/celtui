//! Sight reduction functions for celestial navigation
//!
//! This module provides functions for reducing celestial sights, including:
//! - Computing altitude (Hc) and azimuth (Zn) using spherical trigonometry
//! - Calculating intercept (difference between observed and computed altitude)
//! - Applying altitude corrections (refraction, dip, semi-diameter, parallax)

use crate::coords::{equatorial_to_horizontal, EquatorialCoords};

/// Sight data required for sight reduction calculations
#[derive(Debug, Clone, Copy)]
pub struct SightData {
    /// Observer's latitude in degrees (North positive, South negative)
    pub latitude: f64,

    /// Celestial body's declination in degrees (North positive, South negative)
    pub declination: f64,

    /// Local Hour Angle (LHA) in degrees
    /// LHA = GHA + Longitude (East positive)
    pub local_hour_angle: f64,
}

/// Altitude corrections structure
#[derive(Debug, Clone, Copy)]
pub struct AltitudeCorrections {
    /// Refraction correction in degrees (always negative)
    pub refraction: f64,

    /// Dip correction in degrees (always negative)
    pub dip: f64,

    /// Semi-diameter correction in degrees (positive for lower limb, negative for upper)
    pub semidiameter: f64,

    /// Parallax correction in degrees (always positive, significant only for Moon)
    pub parallax: f64,
}

/// Computes the altitude (Hc) of a celestial body
///
/// Uses the fundamental equation of spherical astronomy:
/// sin(Hc) = sin(Lat) * sin(Dec) + cos(Lat) * cos(Dec) * cos(LHA)
///
/// This is equivalent to converting equatorial coordinates to horizontal coordinates.
///
/// # Arguments
/// * `sight_data` - Sight data containing latitude, declination, and LHA
///
/// # Returns
/// Computed altitude (Hc) in degrees
///
/// # Examples
/// ```
/// use celtnav::sight_reduction::{compute_altitude, SightData};
///
/// let sight = SightData {
///     latitude: 40.0,
///     declination: 20.0,
///     local_hour_angle: 30.0,
/// };
/// let hc = compute_altitude(&sight);
/// ```
pub fn compute_altitude(sight_data: &SightData) -> f64 {
    // Convert to equatorial coordinates
    let eq_coords = EquatorialCoords {
        declination: sight_data.declination,
        hour_angle: sight_data.local_hour_angle,
    };

    // Convert to horizontal coordinates and return altitude
    let hz_coords = equatorial_to_horizontal(&eq_coords, sight_data.latitude);
    hz_coords.altitude
}

/// Computes the azimuth (Zn) of a celestial body
///
/// Uses spherical trigonometry to calculate the true azimuth.
/// Azimuth is measured clockwise from North: N=0°, E=90°, S=180°, W=270°
///
/// # Arguments
/// * `sight_data` - Sight data containing latitude, declination, and LHA
///
/// # Returns
/// Azimuth (Zn) in degrees (0 to 360)
///
/// # Examples
/// ```
/// use celtnav::sight_reduction::{compute_azimuth, SightData};
///
/// let sight = SightData {
///     latitude: 40.0,
///     declination: 20.0,
///     local_hour_angle: 30.0,
/// };
/// let zn = compute_azimuth(&sight);
/// ```
pub fn compute_azimuth(sight_data: &SightData) -> f64 {
    // Convert to equatorial coordinates
    let eq_coords = EquatorialCoords {
        declination: sight_data.declination,
        hour_angle: sight_data.local_hour_angle,
    };

    // Convert to horizontal coordinates and return azimuth
    let hz_coords = equatorial_to_horizontal(&eq_coords, sight_data.latitude);
    hz_coords.azimuth
}

/// Computes the intercept
///
/// The intercept is the difference between observed altitude (Ho) and computed altitude (Hc).
/// - Positive intercept: Ho > Hc, plot TOWARD the body
/// - Negative intercept: Ho < Hc, plot AWAY from the body
///
/// Result is in nautical miles (1 arcminute = 1 nautical mile)
///
/// # Arguments
/// * `sight_data` - Sight data for computing Hc
/// * `observed_altitude` - Observed altitude (Ho) in degrees after corrections
///
/// # Returns
/// Intercept in nautical miles (positive = toward, negative = away)
///
/// # Examples
/// ```
/// use celtnav::sight_reduction::{compute_intercept, SightData};
///
/// let sight = SightData {
///     latitude: 40.0,
///     declination: 20.0,
///     local_hour_angle: 30.0,
/// };
/// let intercept = compute_intercept(&sight, 60.0);
/// ```
pub fn compute_intercept(sight_data: &SightData, observed_altitude: f64) -> f64 {
    let computed_altitude = compute_altitude(sight_data);

    // Intercept = Ho - Hc
    let intercept_degrees = observed_altitude - computed_altitude;

    // Convert to nautical miles (60 arcminutes = 1 degree, 1 arcminute = 1 NM)
    intercept_degrees * 60.0
}

/// Applies refraction correction
///
/// Atmospheric refraction causes celestial bodies to appear higher than they actually are.
/// The correction is greatest at the horizon (~34 arcminutes) and zero at the zenith.
///
/// Uses Bennett's formula (accurate to 0.07' for altitudes > 15°):
/// R = cot(h + 7.31/(h + 4.4))
/// where h is apparent altitude in degrees, R is refraction in arcminutes
///
/// # Arguments
/// * `apparent_altitude` - Apparent altitude in degrees (sextant altitude after index correction)
///
/// # Returns
/// Refraction correction in degrees (always negative, to be added to apparent altitude)
///
/// # Examples
/// ```
/// use celtnav::sight_reduction::apply_refraction_correction;
///
/// let correction = apply_refraction_correction(30.0);
/// assert!(correction < 0.0); // Refraction is always negative
/// ```
pub fn apply_refraction_correction(apparent_altitude: f64) -> f64 {
    // Handle edge cases
    if apparent_altitude >= 90.0 {
        return 0.0; // No refraction at zenith
    }
    if apparent_altitude < 0.0 {
        // Use horizon value for negative altitudes
        return apply_refraction_correction(0.0);
    }

    // Bennett's formula for refraction in arcminutes
    let h = apparent_altitude;
    let refraction_arcmin = 1.0 / ((h + 7.31 / (h + 4.4)).to_radians().tan());

    // Convert to degrees and make negative (we subtract refraction from apparent altitude)
    -(refraction_arcmin / 60.0)
}

/// Applies dip correction
///
/// Dip is the angle between the true horizon and the visible horizon,
/// caused by the observer's height above sea level.
///
/// Formula: Dip (arcminutes) = 1.76 * sqrt(height_meters)
/// or in degrees: Dip = 0.0293 * sqrt(height_meters)
///
/// The visible horizon appears lower than the true horizon, so dip is always negative.
///
/// # Arguments
/// * `height_meters` - Observer's height above sea level in meters
///
/// # Returns
/// Dip correction in degrees (always negative or zero)
///
/// # Examples
/// ```
/// use celtnav::sight_reduction::apply_dip_correction;
///
/// let correction = apply_dip_correction(10.0);
/// assert!(correction < 0.0); // Dip is always negative
/// ```
pub fn apply_dip_correction(height_meters: f64) -> f64 {
    if height_meters <= 0.0 {
        return 0.0;
    }

    // Dip in arcminutes = 1.76 * sqrt(height)
    let dip_arcmin = 1.76 * height_meters.sqrt();

    // Convert to degrees and make negative
    -(dip_arcmin / 60.0)
}

/// Applies semi-diameter correction
///
/// When observing the Sun or Moon, we typically observe the lower or upper limb
/// rather than the center. This correction adjusts to give the altitude of the center.
///
/// - Lower limb: Add semi-diameter (observed limb is below center)
/// - Upper limb: Subtract semi-diameter (observed limb is above center)
///
/// # Arguments
/// * `semidiameter` - Semi-diameter of the body in degrees (from almanac)
/// * `is_lower_limb` - True if lower limb was observed, false for upper limb
///
/// # Returns
/// Semi-diameter correction in degrees
///
/// # Examples
/// ```
/// use celtnav::sight_reduction::apply_semidiameter_correction;
///
/// // Sun's semi-diameter is approximately 16 arcminutes (0.267°)
/// let correction = apply_semidiameter_correction(0.267, true); // lower limb
/// assert!(correction > 0.0); // Add for lower limb
/// ```
pub fn apply_semidiameter_correction(semidiameter: f64, is_lower_limb: bool) -> f64 {
    if is_lower_limb {
        semidiameter // Add for lower limb
    } else {
        -semidiameter // Subtract for upper limb
    }
}

/// Applies parallax correction
///
/// Parallax is the difference in apparent position of a celestial body
/// as viewed from the observer's position versus the center of Earth.
///
/// It's only significant for the Moon (~1°) and negligible for other bodies.
///
/// Formula: Parallax = HP * cos(apparent_altitude)
/// where HP is horizontal parallax (from almanac)
///
/// Parallax is always positive (makes the body appear higher).
///
/// # Arguments
/// * `horizontal_parallax` - Horizontal parallax in degrees (from almanac)
/// * `apparent_altitude` - Apparent altitude in degrees
///
/// # Returns
/// Parallax correction in degrees (always positive or zero)
///
/// # Examples
/// ```
/// use celtnav::sight_reduction::apply_parallax_correction;
///
/// // Moon's horizontal parallax is approximately 57 arcminutes (0.95°)
/// let correction = apply_parallax_correction(0.95, 30.0);
/// assert!(correction > 0.0); // Parallax is always positive
/// ```
pub fn apply_parallax_correction(horizontal_parallax: f64, apparent_altitude: f64) -> f64 {
    if horizontal_parallax == 0.0 {
        return 0.0;
    }

    // Parallax = HP * cos(altitude)
    horizontal_parallax * apparent_altitude.to_radians().cos()
}

/// Optimizes the chosen position (AP - Assumed Position) for easier sight reduction
///
/// Following standard celestial navigation practice, this function:
/// 1. Rounds latitude to the nearest whole degree
/// 2. Adjusts longitude so that LHA (Local Hour Angle) is a whole number
///
/// This optimization makes sight reduction table lookups much easier since
/// tables are indexed by whole degrees.
///
/// # Arguments
/// * `dr_lat` - Dead reckoning latitude in decimal degrees (North positive, South negative)
/// * `dr_lon` - Dead reckoning longitude in decimal degrees (East positive, West negative)
/// * `gha` - Greenwich Hour Angle in decimal degrees
///
/// # Returns
/// Tuple of (chosen_lat, chosen_lon) optimized for whole-degree LHA
///
/// # Formula
/// LHA = GHA + Longitude (for both East and West, using signed convention)
/// We adjust longitude so LHA is exactly X° 00.0'
///
/// # Examples
/// ```
/// use celtnav::sight_reduction::optimize_chosen_position;
///
/// // DR position: 45° 32.5' N, 123° 15.0' W
/// // GHA: 245° 37.2'
/// let dr_lat = 45.542; // 45° 32.5' N
/// let dr_lon = -123.25; // 123° 15.0' W (negative for West)
/// let gha = 245.62; // 245° 37.2'
///
/// let (chosen_lat, chosen_lon) = optimize_chosen_position(dr_lat, dr_lon, gha);
///
/// // Latitude rounded to nearest degree: 46° N
/// assert!((chosen_lat - 46.0).abs() < 0.01);
///
/// // Longitude adjusted so LHA is whole number
/// let lha = (gha + chosen_lon + 360.0) % 360.0;
/// assert!((lha - lha.round()).abs() < 0.01);
/// ```
pub fn optimize_chosen_position(dr_lat: f64, dr_lon: f64, gha: f64) -> (f64, f64) {
    // Round latitude to nearest whole degree
    let chosen_lat = dr_lat.round();

    // Calculate LHA with DR longitude
    // LHA = GHA + Longitude (using signed convention)
    let lha_with_dr = (gha + dr_lon + 360.0) % 360.0;

    // Find fractional part of LHA
    let lha_frac = lha_with_dr - lha_with_dr.floor();

    // Adjust longitude to make LHA whole
    // If fractional part <= 0.5, round down (subtract fraction)
    // If fractional part > 0.5, round up (add to reach next whole degree)
    let lon_adjustment = if lha_frac <= 0.5 {
        -lha_frac
    } else {
        1.0 - lha_frac
    };

    let chosen_lon = dr_lon + lon_adjustment;

    (chosen_lat, chosen_lon)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sight_data_creation() {
        let sight = SightData {
            latitude: 40.0,
            declination: 20.0,
            local_hour_angle: 30.0,
        };
        assert_eq!(sight.latitude, 40.0);
        assert_eq!(sight.declination, 20.0);
        assert_eq!(sight.local_hour_angle, 30.0);
    }

    #[test]
    fn test_altitude_corrections_creation() {
        let corrections = AltitudeCorrections {
            refraction: -0.1,
            dip: -0.05,
            semidiameter: 0.267,
            parallax: 0.5,
        };
        assert_eq!(corrections.refraction, -0.1);
        assert_eq!(corrections.dip, -0.05);
    }

    #[test]
    fn test_refraction_always_negative() {
        let altitudes = vec![0.0, 15.0, 30.0, 45.0, 60.0, 75.0];
        for alt in altitudes {
            let correction = apply_refraction_correction(alt);
            assert!(correction < 0.0,
                    "Refraction should always be negative, got {} for altitude {}",
                    correction, alt);
        }
    }

    #[test]
    fn test_dip_always_negative_or_zero() {
        let heights = vec![0.0, 5.0, 10.0, 20.0, 50.0];
        for height in heights {
            let correction = apply_dip_correction(height);
            assert!(correction <= 0.0,
                    "Dip should always be negative or zero, got {} for height {}",
                    correction, height);
        }
    }

    #[test]
    fn test_parallax_always_positive_or_zero() {
        let hp = 0.95; // Moon's HP
        let altitudes = vec![0.0, 30.0, 60.0, 90.0];
        for alt in altitudes {
            let correction = apply_parallax_correction(hp, alt);
            assert!(correction >= 0.0,
                    "Parallax should always be positive or zero, got {} for altitude {}",
                    correction, alt);
        }
    }

    #[test]
    fn test_optimize_chosen_position_west_longitude() {
        // Test case from user: DR W 123° 15.0', GHA 245° 37.2'
        let dr_lat = 45.542; // 45° 32.5' N
        let dr_lon = -123.25; // 123° 15.0' W (negative for West)
        let gha = 245.62; // 245° 37.2'

        let (chosen_lat, chosen_lon) = optimize_chosen_position(dr_lat, dr_lon, gha);

        // Latitude should be rounded to 46° N
        assert!((chosen_lat - 46.0).abs() < 0.01, "Chosen latitude should be 46°, got {}", chosen_lat);

        // Calculate resulting LHA
        let lha = (gha + chosen_lon + 360.0) % 360.0;

        // LHA should be a whole number (within 0.01°)
        let lha_frac = lha - lha.round();
        assert!(lha_frac.abs() < 0.01,
                "LHA should be whole number, got LHA={}, fraction={}", lha, lha_frac);

        // Chosen longitude should be closest to DR that makes LHA whole
        // Expected: LHA = GHA + Lon = 245.62 + (-123.25) = 122.37
        // Want LHA = 122°, so adjust lon by -0.37°
        // Chosen lon = -123.25 - 0.37 = -123.62 = W 123° 37.2'
        let expected_lon = -123.62;
        assert!((chosen_lon - expected_lon).abs() < 0.05,
                "Chosen longitude should be near {}, got {}", expected_lon, chosen_lon);
    }

    #[test]
    fn test_optimize_chosen_position_east_longitude() {
        // Test with East longitude: DR E 45° 30.0', GHA 180° 45.0'
        let dr_lat = 35.25; // 35° 15.0' N
        let dr_lon = 45.5; // 45° 30.0' E (positive for East)
        let gha = 180.75; // 180° 45.0'

        let (chosen_lat, chosen_lon) = optimize_chosen_position(dr_lat, dr_lon, gha);

        // Latitude should be rounded to 35° N
        assert!((chosen_lat - 35.0).abs() < 0.01, "Chosen latitude should be 35°, got {}", chosen_lat);

        // Calculate resulting LHA
        let lha = (gha + chosen_lon + 360.0) % 360.0;

        // LHA should be a whole number
        let lha_frac = lha - lha.round();
        assert!(lha_frac.abs() < 0.01,
                "LHA should be whole number, got LHA={}, fraction={}", lha, lha_frac);

        // Verify chosen longitude is nearest to DR
        // LHA = 180.75 + 45.5 = 226.25
        // Want LHA = 226° or 227°
        // For LHA = 226°: need Lon = 226 - 180.75 = 45.25° E
        // For LHA = 227°: need Lon = 227 - 180.75 = 46.25° E
        // DR is 45.5°, so 45.25° is closest (0.25° away vs 0.75° away)
        let expected_lon = 45.25;
        assert!((chosen_lon - expected_lon).abs() < 0.05,
                "Chosen longitude should be near {}, got {}", expected_lon, chosen_lon);
    }

    #[test]
    fn test_optimize_chosen_position_crossing_zero_meridian() {
        // Test crossing 0° meridian: DR W 2° 30.0', GHA 358° 45.0'
        let dr_lat = 50.0; // 50° 0.0' N
        let dr_lon = -2.5; // 2° 30.0' W (negative for West)
        let gha = 358.75; // 358° 45.0'

        let (chosen_lat, chosen_lon) = optimize_chosen_position(dr_lat, dr_lon, gha);

        // Latitude should be 50° N
        assert!((chosen_lat - 50.0).abs() < 0.01, "Chosen latitude should be 50°, got {}", chosen_lat);

        // Calculate resulting LHA
        let lha = (gha + chosen_lon + 360.0) % 360.0;

        // LHA should be a whole number
        let lha_frac = lha - lha.round();
        assert!(lha_frac.abs() < 0.01,
                "LHA should be whole number, got LHA={}, fraction={}", lha, lha_frac);

        // LHA = 358.75 + (-2.5) = 356.25
        // Want LHA = 356° or 357°
        // For LHA = 356°: need Lon = 356 - 358.75 = -2.75° W
        // For LHA = 357°: need Lon = 357 - 358.75 = -1.75° W
        // DR is -2.5°, so -2.75° is closest (0.25° away vs 0.75° away)
        let expected_lon = -2.75;
        assert!((chosen_lon - expected_lon).abs() < 0.05,
                "Chosen longitude should be near {}, got {}", expected_lon, chosen_lon);
    }

    #[test]
    fn test_optimize_chosen_position_lha_near_round() {
        // Test when LHA is already nearly a whole number
        let dr_lat = 40.0;
        let dr_lon = -70.0; // 70° W
        let gha = 290.05; // 290° 03.0' - results in LHA very close to 220°

        let (chosen_lat, chosen_lon) = optimize_chosen_position(dr_lat, dr_lon, gha);

        // Latitude should be 40° N
        assert!((chosen_lat - 40.0).abs() < 0.01);

        // Calculate resulting LHA
        let lha = (gha + chosen_lon + 360.0) % 360.0;

        // LHA should be a whole number
        let lha_frac = lha - lha.round();
        assert!(lha_frac.abs() < 0.01,
                "LHA should be whole number, got LHA={}, fraction={}", lha, lha_frac);

        // Chosen longitude should be very close to DR since only small adjustment needed
        assert!((chosen_lon - dr_lon).abs() < 0.1,
                "Chosen longitude should be close to DR={}, got {}", dr_lon, chosen_lon);
    }

    #[test]
    fn test_ho_calculation_correction_order() {
        // Test that Ho corrections are applied in correct order
        // Order should be: Hs + IE + dip + refraction + SD + parallax

        let hs = 30.0;  // Sextant altitude: 30°
        let index_error = 2.0 / 60.0;  // 2 arcminutes on scale
        let height_of_eye = 10.0;  // 10 meters

        // Apply corrections in correct order
        let mut ho = hs;
        ho += index_error;  // Add index error (can be positive or negative)

        // Store altitude before dip for later comparison
        let after_ie = ho;

        ho += apply_dip_correction(height_of_eye);  // Subtract dip (always negative)
        let dip = apply_dip_correction(height_of_eye);
        assert!(dip < 0.0, "Dip should be negative");
        assert!(ho < after_ie, "Altitude after dip should be less than before");

        // Store altitude after dip for refraction calculation
        let after_dip = ho;

        ho += apply_refraction_correction(after_dip);  // Subtract refraction (always negative)
        let refraction = apply_refraction_correction(after_dip);
        assert!(refraction < 0.0, "Refraction should be negative");
        assert!(ho < after_dip, "Altitude after refraction should be less than before");

        // For Sun: add semi-diameter (lower limb)
        let sd_sun = 0.267;  // Sun's semi-diameter in degrees (~16')
        ho += apply_semidiameter_correction(sd_sun, true);  // Lower limb: add SD
        let sd = apply_semidiameter_correction(sd_sun, true);
        assert!(sd > 0.0, "SD correction for lower limb should be positive");

        // Verify Ho is reasonable
        assert!(ho > 0.0 && ho < 90.0, "Ho should be between 0° and 90°");

        // Verify correction magnitudes are reasonable
        assert!(dip.abs() < 0.1, "Dip should be less than 0.1° (6') for height of 10m");
        assert!(refraction.abs() < 0.1, "Refraction should be less than 0.1° (6') at 30° altitude");
    }

    #[test]
    fn test_lha_is_whole_number_after_optimization() {
        // After optimizing chosen position, LHA must be exactly a whole number
        // This is critical for sight reduction table lookups

        let test_cases = vec![
            (45.542, -123.25, 245.62),  // 45°32.5'N, 123°15.0'W, GHA 245°37.2'
            (35.25, 45.5, 180.75),      // 35°15.0'N, 45°30.0'E, GHA 180°45.0'
            (50.0, -2.5, 358.75),       // 50°00.0'N, 2°30.0'W, GHA 358°45.0'
            (-20.33, 150.67, 90.45),    // 20°20'S, 150°40'E, GHA 90°27'
        ];

        for (dr_lat, dr_lon, gha) in test_cases {
            let (chosen_lat, chosen_lon) = optimize_chosen_position(dr_lat, dr_lon, gha);

            // Calculate LHA
            let lha = (gha + chosen_lon + 360.0) % 360.0;

            // LHA must be whole number (within floating point precision)
            let lha_frac = lha - lha.round();
            assert!(lha_frac.abs() < 0.001,
                    "LHA must be whole number. DR: ({}, {}), GHA: {}, LHA: {}, fraction: {}",
                    dr_lat, dr_lon, gha, lha, lha_frac);

            // Chosen position should be within 1° of DR
            assert!((chosen_lat - dr_lat).abs() <= 1.0,
                    "Chosen lat should be within 1° of DR");
            assert!((chosen_lon - dr_lon).abs() <= 1.0,
                    "Chosen lon should be within 1° of DR");
        }
    }

    #[test]
    fn test_semidiameter_sign_for_limbs() {
        let sd = 0.267;  // Sun's semi-diameter

        let sd_lower = apply_semidiameter_correction(sd, true);
        assert!(sd_lower > 0.0, "Lower limb: SD correction should be positive (add SD)");
        assert!((sd_lower - sd).abs() < 0.001, "Lower limb: SD correction should equal SD");

        let sd_upper = apply_semidiameter_correction(sd, false);
        assert!(sd_upper < 0.0, "Upper limb: SD correction should be negative (subtract SD)");
        assert!((sd_upper + sd).abs() < 0.001, "Upper limb: SD correction should equal -SD");
    }

    #[test]
    fn test_pollux_scenario_east() {
        // Pollux scenario where body is in the east (morning observation)
        // This should give Az ~ 104° (east-southeast)
        let sight = SightData {
            latitude: 45.0,
            declination: 28.0,  // Pollux declination
            local_hour_angle: 52.0,  // Approximate LHA for Hc ≈ 46°, Az ≈ 104°
        };

        let hc = compute_altitude(&sight);
        let zn = compute_azimuth(&sight);

        // Should be roughly 46° altitude
        assert!(
            (hc - 46.0).abs() < 1.0,
            "Hc should be ~46° for this LHA, got {:.1}°",
            hc
        );

        // Should be east-southeast (around 95-110°)
        assert!(
            zn > 90.0 && zn < 120.0,
            "Azimuth should be east-southeast (~104°), got {:.0}°",
            zn
        );
    }

    #[test]
    fn test_pollux_scenario_west() {
        // Pollux scenario where body is in the west (evening observation)
        // This gives Az ~ 265° (west-southwest)
        let sight = SightData {
            latitude: 45.0,
            declination: 28.0,  // Pollux declination
            local_hour_angle: 308.0,  // Approximate LHA for Hc ≈ 46°, Az ≈ 265°
        };

        let hc = compute_altitude(&sight);
        let zn = compute_azimuth(&sight);

        // Should be roughly 46° altitude
        assert!(
            (hc - 46.0).abs() < 1.0,
            "Hc should be ~46° for this LHA, got {:.1}°",
            hc
        );

        // Should be west-southwest (around 255-275°)
        assert!(
            zn > 250.0 && zn < 280.0,
            "Azimuth should be west-southwest (~265°), got {:.0}°",
            zn
        );
    }

    #[test]
    fn test_lha_calculation_east_vs_west_longitude() {
        // Test that LHA is calculated correctly for East and West longitudes
        // Formula: LHA = GHA + Longitude (East positive, West negative)

        // Example: GHA = 343.4° (Pollux with GHA Aries ≈ 100°)

        // Case 1: West longitude (should be negative in calculation)
        let gha: f64 = 343.4;
        let lon_west: f64 = -123.0;  // W 123° = -123°
        let lha_west = (gha + lon_west + 360.0) % 360.0;

        assert!(
            (lha_west - 220.4_f64).abs() < 0.1,
            "LHA for W 123° should be ~220.4°, got {:.1}°",
            lha_west
        );

        // Case 2: East longitude (should be positive)
        let lon_east: f64 = 123.0;  // E 123° = +123°
        let lha_east = (gha + lon_east + 360.0) % 360.0;

        assert!(
            (lha_east - 106.4_f64).abs() < 0.1,
            "LHA for E 123° should be ~106.4°, got {:.1}°",
            lha_east
        );

        // The two LHAs should be complementary in terms of results
        // Same altitude, but azimuth on opposite sides
        let sight_west = SightData {
            latitude: 45.0,
            declination: 28.0,
            local_hour_angle: lha_west,
        };

        let sight_east = SightData {
            latitude: 45.0,
            declination: 28.0,
            local_hour_angle: lha_east,
        };

        let hc_west = compute_altitude(&sight_west);
        let hc_east = compute_altitude(&sight_east);

        // Altitudes are independent of azimuth direction, but will differ
        // due to different LHA values - this is just demonstrating the calculation works
        assert!(hc_west < 0.0, "Body below horizon for LHA 220.4° at this position");
        assert!(hc_east > 0.0, "Body above horizon for LHA 106.4° at this position");
    }
}
