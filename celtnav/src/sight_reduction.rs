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

/// Normalizes an angle to the range [0, 360)
///
/// # Arguments
/// * `degrees` - Angle in degrees (can be any value)
///
/// # Returns
/// Normalized angle in range [0, 360)
fn normalize_degrees(degrees: f64) -> f64 {
    let mut normalized = degrees % 360.0;
    if normalized < 0.0 {
        normalized += 360.0;
    }
    normalized
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
/// **DEPRECATED**: Use `optimize_chosen_position_celestial_body` or
/// `optimize_chosen_position_star` instead for clearer intent.
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
    optimize_chosen_position_celestial_body(dr_lat, dr_lon, gha)
}

/// Optimizes the chosen position for celestial bodies (Sun, Moon, Planets)
///
/// For non-star bodies, we optimize to make the LHA of the body itself a whole number.
/// This follows standard sight reduction practice for Pub 229 and similar tables.
///
/// # Arguments
/// * `dr_lat` - Dead reckoning latitude in decimal degrees (North positive, South negative)
/// * `dr_lon` - Dead reckoning longitude in decimal degrees (East positive, West negative)
/// * `gha` - Greenwich Hour Angle of the body in decimal degrees
///
/// # Returns
/// Tuple of (chosen_lat, chosen_lon) optimized for whole-degree LHA of the body
///
/// # Examples
/// ```
/// use celtnav::sight_reduction::optimize_chosen_position_celestial_body;
///
/// // DR position and GHA of Sun
/// let dr_lat = 40.5;
/// let dr_lon = -70.0;
/// let gha_sun = 245.7;
///
/// let (chosen_lat, chosen_lon) = optimize_chosen_position_celestial_body(dr_lat, dr_lon, gha_sun);
///
/// // LHA of Sun should be a whole number
/// let lha_sun = (gha_sun + chosen_lon + 360.0) % 360.0;
/// assert!((lha_sun - lha_sun.round()).abs() < 0.01);
/// ```
pub fn optimize_chosen_position_celestial_body(dr_lat: f64, dr_lon: f64, gha: f64) -> (f64, f64) {
    // Round latitude to nearest whole degree
    let chosen_lat = dr_lat.round();

    // Calculate LHA with DR longitude
    // LHA = GHA + Longitude (using signed convention)
    let lha_with_dr = normalize_degrees(gha + dr_lon);

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

/// Optimizes the chosen position for star sights (Pub 249 Vol 1 organization)
///
/// **CRITICAL**: For stars, we optimize to make LHA Aries a whole number, NOT the LHA of the star!
/// This is how Pub 249 Vol 1 sight reduction tables are organized.
///
/// The LHA of the star itself is calculated as: LHA_star = LHA_Aries + SHA_star
/// and will NOT be a whole number (which is correct and expected).
///
/// # Background
/// Star sight reduction tables (Pub 249 Vol 1) are indexed by:
/// - Latitude (whole degrees)
/// - LHA Aries (whole degrees)
/// - Star name
///
/// The SHA (Sidereal Hour Angle) of the star is looked up separately and is generally
/// not a whole number. This is different from Sun/Moon/Planet tables which use LHA of the body.
///
/// # Arguments
/// * `dr_lat` - Dead reckoning latitude in decimal degrees (North positive, South negative)
/// * `dr_lon` - Dead reckoning longitude in decimal degrees (East positive, West negative)
/// * `gha_aries` - Greenwich Hour Angle of Aries (♈) in decimal degrees
///
/// # Returns
/// Tuple of (chosen_lat, chosen_lon) optimized for whole-degree LHA Aries
///
/// # Examples
/// ```
/// use celtnav::sight_reduction::optimize_chosen_position_star;
///
/// // DR position and GHA Aries
/// let dr_lat = 40.5;
/// let dr_lon = -70.25;
/// let gha_aries = 145.7;
///
/// let (chosen_lat, chosen_lon) = optimize_chosen_position_star(dr_lat, dr_lon, gha_aries);
///
/// // LHA Aries should be a whole number (for Pub 249 Vol 1 lookup)
/// let lha_aries = (gha_aries + chosen_lon + 360.0) % 360.0;
/// assert!((lha_aries - lha_aries.round()).abs() < 0.01);
///
/// // LHA star = LHA Aries + SHA (will NOT be whole, and that's correct!)
/// let sha_pollux = 243.4;
/// let lha_pollux = (lha_aries + sha_pollux) % 360.0;
/// // lha_pollux may be fractional - this is expected and correct
/// ```
pub fn optimize_chosen_position_star(dr_lat: f64, dr_lon: f64, gha_aries: f64) -> (f64, f64) {
    // Round latitude to nearest whole degree
    let chosen_lat = dr_lat.round();

    // Calculate LHA Aries with DR longitude
    // LHA Aries = GHA Aries + Longitude
    let lha_aries_with_dr = normalize_degrees(gha_aries + dr_lon);

    // Find fractional part of LHA Aries
    let lha_aries_frac = lha_aries_with_dr - lha_aries_with_dr.floor();

    // Adjust longitude to make LHA ARIES whole (not LHA star!)
    let lon_adjustment = if lha_aries_frac <= 0.5 {
        -lha_aries_frac
    } else {
        1.0 - lha_aries_frac
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

    // ===== Tests for star-specific optimization (Pub 249 Vol 1) =====

    #[test]
    fn test_optimize_star_chosen_position_lha_aries_whole() {
        // For stars, we optimize to make LHA Aries a whole number (Pub 249 Vol 1 organization)
        // NOT LHA of the star itself

        // Example: Pollux with SHA = 243.4°
        let dr_lat = 40.5;
        let dr_lon = -70.25; // West
        let gha_aries = 145.7;

        let (chosen_lat, chosen_lon) = optimize_chosen_position_star(dr_lat, dr_lon, gha_aries);

        // Latitude should be rounded
        assert_eq!(chosen_lat, 41.0, "Latitude should be rounded to nearest whole degree");

        // LHA Aries MUST be whole
        let lha_aries = (gha_aries + chosen_lon + 360.0) % 360.0;
        assert!(
            (lha_aries - lha_aries.round()).abs() < 0.01,
            "LHA Aries should be whole number, got {:.2}°, fractional part: {:.4}",
            lha_aries,
            lha_aries - lha_aries.round()
        );

        // LHA star will NOT be whole (that's correct for Pub 249 Vol 1!)
        let sha_pollux = 243.4;
        let lha_pollux = (lha_aries + sha_pollux) % 360.0;
        // lha_pollux doesn't need to be whole - we accept any value here
        // The important thing is that LHA Aries is whole
        assert!(
            lha_pollux >= 0.0 && lha_pollux < 360.0,
            "LHA star should be valid angle, got {:.2}°",
            lha_pollux
        );
    }

    #[test]
    fn test_optimize_star_vs_celestial_body_different_results() {
        // For the same DR position and GHA values, star optimization and
        // celestial body optimization should give DIFFERENT results

        let dr_lat = 40.0;
        let dr_lon = -70.0;
        let gha_aries = 150.0;
        let sha_pollux = 243.4;
        let gha_pollux = (gha_aries + sha_pollux) % 360.0; // 393.4 - 360 = 33.4°

        // Star optimization: optimize based on GHA Aries
        let (star_lat, star_lon) = optimize_chosen_position_star(dr_lat, dr_lon, gha_aries);

        // Celestial body optimization: optimize based on GHA of body
        let (body_lat, body_lon) = optimize_chosen_position_celestial_body(dr_lat, dr_lon, gha_pollux);

        // Latitudes should be the same (both round to nearest whole degree)
        assert_eq!(star_lat, body_lat, "Both should round latitude the same way");

        // Longitudes should be DIFFERENT because we're optimizing different LHAs
        assert!(
            (star_lon - body_lon).abs() > 0.1,
            "Star and body optimizations should give different longitudes. Star: {:.2}°, Body: {:.2}°",
            star_lon,
            body_lon
        );

        // Verify star optimization makes LHA Aries whole
        let lha_aries = (gha_aries + star_lon + 360.0) % 360.0;
        assert!(
            (lha_aries - lha_aries.round()).abs() < 0.01,
            "Star optimization should make LHA Aries whole"
        );

        // Verify body optimization makes LHA of body whole
        let lha_body = (gha_pollux + body_lon + 360.0) % 360.0;
        assert!(
            (lha_body - lha_body.round()).abs() < 0.01,
            "Body optimization should make LHA of body whole"
        );
    }

    #[test]
    fn test_pub249_vol1_organization() {
        // Verify that for stars, we organize by LHA Aries (whole number)
        // This matches Pub 249 Vol 1 table organization

        // Test case: Observer at 40°N, 70°W
        // GHA Aries = 150°
        // SHA Sirius = 258.6°

        let (_chosen_lat, chosen_lon) = optimize_chosen_position_star(40.0, -70.0, 150.0);

        let lha_aries = (150.0 + chosen_lon + 360.0) % 360.0;
        assert!(
            (lha_aries.round() - lha_aries).abs() < 0.1,
            "LHA Aries must be whole for Pub 249 Vol 1. Got {:.2}°",
            lha_aries
        );

        // When we add SHA Sirius, LHA Sirius won't be whole - and that's OK
        let lha_sirius = (lha_aries + 258.6) % 360.0;
        // lha_sirius is allowed to be fractional - we don't test for whole number
        assert!(
            lha_sirius >= 0.0 && lha_sirius < 360.0,
            "LHA Sirius should be valid angle"
        );
    }

    #[test]
    fn test_star_optimization_multiple_scenarios() {
        // Test star optimization with various GHA Aries values

        let test_cases = vec![
            (40.0, -70.0, 150.0),    // Standard case
            (35.5, -120.5, 200.5),   // West coast US
            (50.2, 5.3, 45.8),       // English Channel
            (-33.9, 18.4, 300.1),    // Cape Town
        ];

        for (dr_lat, dr_lon, gha_aries) in test_cases {
            let (chosen_lat, chosen_lon) = optimize_chosen_position_star(dr_lat, dr_lon, gha_aries);

            // Latitude should be rounded
            assert!(
                (chosen_lat - dr_lat).abs() <= 0.5,
                "Chosen lat should be within 0.5° of DR (rounded)"
            );

            // LHA Aries must be whole
            let lha_aries = (gha_aries + chosen_lon + 360.0) % 360.0;
            assert!(
                (lha_aries - lha_aries.round()).abs() < 0.01,
                "LHA Aries must be whole. DR: ({:.1}, {:.1}), GHA♈: {:.1}, LHA♈: {:.2}",
                dr_lat, dr_lon, gha_aries, lha_aries
            );

            // Chosen position should be close to DR
            assert!(
                (chosen_lon - dr_lon).abs() <= 1.0,
                "Chosen lon should be within 1° of DR"
            );
        }
    }

    #[test]
    fn test_star_optimization_edge_cases() {
        // Test edge cases: GHA Aries near 0° and 360°

        // Near 0°
        let (lat1, lon1) = optimize_chosen_position_star(45.0, -70.0, 0.5);
        let lha_aries1 = (0.5 + lon1 + 360.0) % 360.0;
        assert!(
            (lha_aries1 - lha_aries1.round()).abs() < 0.01,
            "LHA Aries should be whole near 0°"
        );

        // Near 360°
        let (lat2, lon2) = optimize_chosen_position_star(45.0, -70.0, 359.5);
        let lha_aries2 = (359.5 + lon2 + 360.0) % 360.0;
        assert!(
            (lha_aries2 - lha_aries2.round()).abs() < 0.01,
            "LHA Aries should be whole near 360°"
        );

        // Both latitudes should be 45°
        assert_eq!(lat1, 45.0);
        assert_eq!(lat2, 45.0);
    }

    #[test]
    fn test_pollux_example_star_vs_body_optimization() {
        // Real-world example: Pollux observation
        // This demonstrates why stars need different optimization

        // Pollux data:
        // SHA Pollux = 243.4°
        // Dec Pollux = N 28° 01.6'
        // DR: 40°N, 70°W
        // GHA Aries = 145.7°

        let dr_lat = 40.0;
        let dr_lon = -70.0;
        let gha_aries = 145.7;
        let sha_pollux = 243.4;
        let gha_pollux = normalize_degrees(gha_aries + sha_pollux); // 389.1 - 360 = 29.1°

        // CORRECT (Star): Optimize based on GHA Aries
        let (star_lat, star_lon) = optimize_chosen_position_star(dr_lat, dr_lon, gha_aries);
        let lha_aries = normalize_degrees(gha_aries + star_lon);
        let lha_pollux_correct = normalize_degrees(lha_aries + sha_pollux);

        // WRONG (Body): Optimize based on GHA Pollux
        let (body_lat, body_lon) = optimize_chosen_position_celestial_body(dr_lat, dr_lon, gha_pollux);
        let lha_pollux_wrong = normalize_degrees(gha_pollux + body_lon);

        // Both round latitude the same
        assert_eq!(star_lat, 40.0);
        assert_eq!(body_lat, 40.0);

        // LHA Aries should be whole with star optimization
        assert!(
            (lha_aries - lha_aries.round()).abs() < 0.01,
            "Star optimization: LHA Aries should be whole, got {:.2}°", lha_aries
        );

        // LHA Pollux should be whole with body optimization
        assert!(
            (lha_pollux_wrong - lha_pollux_wrong.round()).abs() < 0.01,
            "Body optimization: LHA Pollux should be whole, got {:.2}°", lha_pollux_wrong
        );

        // The two LHAs for Pollux should be DIFFERENT
        // This is the bug we're fixing!
        assert!(
            (lha_pollux_correct - lha_pollux_wrong).abs() > 0.1,
            "Star and body optimizations should give different LHA for Pollux. \
             Correct (via LHA Aries): {:.2}°, Wrong (via GHA Pollux): {:.2}°",
            lha_pollux_correct, lha_pollux_wrong
        );

        // The chosen longitudes should also be different
        assert!(
            (star_lon - body_lon).abs() > 0.1,
            "Chosen longitudes should differ. Star: {:.2}°, Body: {:.2}°",
            star_lon, body_lon
        );
    }

    #[test]
    fn test_normalize_degrees() {
        // Test the normalize_degrees helper function
        assert!((normalize_degrees(0.0) - 0.0).abs() < 0.001);
        assert!((normalize_degrees(360.0) - 0.0).abs() < 0.001);
        assert!((normalize_degrees(720.0) - 0.0).abs() < 0.001);
        assert!((normalize_degrees(-90.0) - 270.0).abs() < 0.001);
        assert!((normalize_degrees(-360.0) - 0.0).abs() < 0.001);
        assert!((normalize_degrees(389.1) - 29.1).abs() < 0.001);
        assert!((normalize_degrees(180.0) - 180.0).abs() < 0.001);
    }
}
