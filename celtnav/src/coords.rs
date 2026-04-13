// SPDX-License-Identifier: GPL-2.0-only
// SPDX-FileCopyrightText: Alexander Atanasov <alex@zazolabs.com>
//! Coordinate transformation functions for celestial navigation
//!
//! This module provides functions for converting between different celestial
//! coordinate systems, specifically:
//! - Equatorial coordinates (Hour Angle, Declination)
//! - Horizontal coordinates (Altitude, Azimuth)

/// Equatorial coordinates
///
/// Represents a celestial body's position in the equatorial coordinate system.
#[derive(Debug, Clone, Copy)]
pub struct EquatorialCoords {
    /// Declination in degrees (-90 to +90)
    /// North is positive, South is negative
    pub declination: f64,

    /// Hour Angle in degrees (0 to 360)
    /// Measured westward from the observer's meridian
    pub hour_angle: f64,
}

/// Horizontal coordinates
///
/// Represents a celestial body's position in the observer's local coordinate system.
#[derive(Debug, Clone, Copy)]
pub struct HorizontalCoords {
    /// Altitude in degrees (-90 to +90)
    /// Angle above the horizon (positive) or below (negative)
    pub altitude: f64,

    /// Azimuth in degrees (0 to 360)
    /// Measured clockwise from North: N=0°, E=90°, S=180°, W=270°
    pub azimuth: f64,
}

/// Converts equatorial coordinates to horizontal coordinates
///
/// This uses the standard spherical trigonometry formulas for coordinate transformation.
///
/// # Altitude Formula
/// ```text
/// sin(Alt) = sin(Lat) * sin(Dec) + cos(Lat) * cos(Dec) * cos(LHA)
/// ```
/// This formula is derived from the navigational triangle formed by the celestial pole,
/// zenith, and the celestial body.
///
/// # Azimuth Formula
/// The azimuth is calculated using the atan2 function to handle all quadrants correctly:
/// ```text
/// x = cos(Dec) * sin(LHA)
/// y = cos(Lat) * sin(Dec) - sin(Lat) * cos(Dec) * cos(LHA)
/// Azimuth = atan2(x, y)
/// ```
///
/// The azimuth is measured clockwise from North:
/// - N = 0° (or 360°)
/// - E = 90°
/// - S = 180°
/// - W = 270°
///
/// # Sign Conventions
/// - **Latitude**: North positive (+), South negative (-)
/// - **Declination**: North positive (+), South negative (-)
/// - **LHA**: 0° to 360°, measured westward from observer's meridian
///
/// # Common Errors to Avoid
/// 1. **Incorrect LHA calculation**: LHA = GHA + Longitude (East positive, West negative)
/// 2. **Wrong azimuth**: Verify atan2 argument order - Rust uses atan2(y, x) not atan2(x, y)
/// 3. **Sign errors**: Ensure West longitudes are negative when calculating LHA
///
/// # Arguments
/// * `eq_coords` - Equatorial coordinates (Hour Angle, Declination)
/// * `latitude` - Observer's latitude in degrees (North positive, South negative)
///
/// # Returns
/// Horizontal coordinates (Altitude, Azimuth)
///
/// # Examples
/// ```
/// use celtnav::coords::{equatorial_to_horizontal, EquatorialCoords};
///
/// // Body on meridian (LHA = 0°)
/// let eq = EquatorialCoords {
///     declination: 20.0,
///     hour_angle: 0.0,
/// };
/// let hz = equatorial_to_horizontal(&eq, 40.0); // 40°N latitude
/// // Body will be due south (Az ≈ 180°) with altitude 70°
///
/// // Body in east (LHA ≈ 90°)
/// let eq = EquatorialCoords {
///     declination: 0.0,
///     hour_angle: 90.0,
/// };
/// let hz = equatorial_to_horizontal(&eq, 0.0); // On equator
/// // Body will be on eastern horizon (Az = 90°, Alt = 0°)
/// ```
///
/// # References
/// - Bowditch: The American Practical Navigator (Chapter 15 & 20)
/// - Pub 229: Sight Reduction Tables for Marine Navigation
/// - USNO Astronomical Applications Department formulas
pub fn equatorial_to_horizontal(eq_coords: &EquatorialCoords, latitude: f64) -> HorizontalCoords {
    // Convert to radians
    let lat_rad = latitude.to_radians();
    let dec_rad = eq_coords.declination.to_radians();
    let lha_rad = eq_coords.hour_angle.to_radians();

    // Calculate altitude using spherical trigonometry
    let sin_alt = lat_rad.sin() * dec_rad.sin()
        + lat_rad.cos() * dec_rad.cos() * lha_rad.cos();
    let altitude = sin_alt.asin().to_degrees();

    // Calculate azimuth using the proper nautical formula
    // cos(Dec) * sin(LHA) for the x-component
    // cos(Lat) * sin(Dec) - sin(Lat) * cos(Dec) * cos(LHA) for the y-component
    let x = dec_rad.cos() * lha_rad.sin();
    let y = lat_rad.cos() * dec_rad.sin() - lat_rad.sin() * dec_rad.cos() * lha_rad.cos();

    // atan2 gives angle from -180 to 180, with 0 at north
    let mut azimuth = x.atan2(y).to_degrees();

    // Convert to 0-360 range (measured from North clockwise)
    azimuth = normalize_angle(azimuth);

    HorizontalCoords { altitude, azimuth }
}

/// Converts horizontal coordinates to equatorial coordinates
///
/// This is the inverse operation of equatorial_to_horizontal.
///
/// Formula for declination:
/// sin(Dec) = sin(Lat) * sin(Alt) - cos(Lat) * cos(Alt) * cos(Az)
///
/// Formula for hour angle:
/// tan(LHA) = sin(Az) / (cos(Az) * sin(Lat) + tan(Alt) * cos(Lat))
///
/// # Arguments
/// * `hz_coords` - Horizontal coordinates (Altitude, Azimuth)
/// * `latitude` - Observer's latitude in degrees (North positive, South negative)
///
/// # Returns
/// Equatorial coordinates (Hour Angle, Declination)
pub fn horizontal_to_equatorial(hz_coords: &HorizontalCoords, latitude: f64) -> EquatorialCoords {
    // Convert to radians
    let lat_rad = latitude.to_radians();
    let alt_rad = hz_coords.altitude.to_radians();
    let az_rad = hz_coords.azimuth.to_radians();

    // Calculate declination using the inverse formula
    // sin(Dec) = sin(Lat) * sin(Alt) - cos(Lat) * cos(Alt) * cos(Az)
    let sin_dec = lat_rad.sin() * alt_rad.sin()
        - lat_rad.cos() * alt_rad.cos() * az_rad.cos();
    let declination = sin_dec.asin().to_degrees();

    // Calculate hour angle (inverse of the azimuth calculation)
    // We need to reverse: x = cos(Dec) * sin(LHA), y = cos(Lat) * sin(Dec) - sin(Lat) * cos(Dec) * cos(LHA)
    // This gives us: sin(LHA) / cos(Dec) and the y-component

    let dec_rad = declination.to_radians();
    let cos_dec = dec_rad.cos();

    // From the azimuth formula: x = cos(Dec) * sin(LHA)
    let sin_lha = az_rad.sin() * cos_dec;

    // From the azimuth formula: y = cos(Lat) * sin(Dec) - sin(Lat) * cos(Dec) * cos(LHA)
    // Rearrange to find cos(LHA):
    // cos(LHA) = (cos(Lat) * sin(Dec) - y) / (sin(Lat) * cos(Dec))
    let y = -alt_rad.cos() * az_rad.cos();  // Negative because we're inverting
    let cos_lha = (lat_rad.cos() * sin_dec - y) / (lat_rad.sin() * cos_dec);

    let mut hour_angle = sin_lha.atan2(cos_lha).to_degrees();
    hour_angle = normalize_angle(hour_angle);

    EquatorialCoords {
        declination,
        hour_angle,
    }
}

/// Normalizes an angle to the range [0, 360)
///
/// # Arguments
/// * `angle` - An angle in degrees
///
/// # Returns
/// The equivalent angle in the range [0, 360)
fn normalize_angle(angle: f64) -> f64 {
    let mut normalized = angle % 360.0;
    if normalized < 0.0 {
        normalized += 360.0;
    }
    normalized
}

#[cfg(test)]
mod coords_test_data;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_angle() {
        assert_eq!(normalize_angle(0.0), 0.0);
        assert_eq!(normalize_angle(360.0), 0.0);
        assert_eq!(normalize_angle(180.0), 180.0);
        assert_eq!(normalize_angle(-90.0), 270.0);
        assert_eq!(normalize_angle(450.0), 90.0);
        assert_eq!(normalize_angle(-180.0), 180.0);
    }

    #[test]
    fn test_equatorial_coords_creation() {
        let coords = EquatorialCoords {
            declination: 45.0,
            hour_angle: 180.0,
        };
        assert_eq!(coords.declination, 45.0);
        assert_eq!(coords.hour_angle, 180.0);
    }

    #[test]
    fn test_horizontal_coords_creation() {
        let coords = HorizontalCoords {
            altitude: 30.0,
            azimuth: 90.0,
        };
        assert_eq!(coords.altitude, 30.0);
        assert_eq!(coords.azimuth, 90.0);
    }

    #[test]
    fn test_azimuth_pub229_case1() {
        // Pub 229: Lat 40°N, Dec 20°N, LHA 30°
        // Expected: Hc = 59° 49', Zn = 122°
        use coords_test_data::pub229_test_data;
        let (eq, lat, expected_hc, expected_zn) = pub229_test_data::test_case_1();

        let hz = equatorial_to_horizontal(&eq, lat);

        // Test altitude (should be accurate to within 1 arcminute = 0.0167°)
        assert!(
            (hz.altitude - expected_hc).abs() < 0.02,
            "Hc mismatch: expected {:.2}°, got {:.2}°",
            expected_hc, hz.altitude
        );

        // Test azimuth (should be accurate to within 1°)
        assert!(
            (hz.azimuth - expected_zn).abs() < 1.0,
            "Azimuth mismatch: expected {}°, got {:.1}°",
            expected_zn, hz.azimuth
        );
    }

    #[test]
    fn test_azimuth_pub229_case2() {
        // Pub 229: Lat 45°N, Dec 15°N, LHA 60°
        // Expected: Hc = 58° 41', Zn = 130°
        use coords_test_data::pub229_test_data;
        let (eq, lat, expected_hc, expected_zn) = pub229_test_data::test_case_2();

        let hz = equatorial_to_horizontal(&eq, lat);

        assert!(
            (hz.altitude - expected_hc).abs() < 0.02,
            "Hc mismatch: expected {:.2}°, got {:.2}°",
            expected_hc, hz.altitude
        );

        assert!(
            (hz.azimuth - expected_zn).abs() < 1.0,
            "Azimuth mismatch: expected {}°, got {:.1}°",
            expected_zn, hz.azimuth
        );
    }

    #[test]
    fn test_azimuth_meridian_south() {
        // Body on meridian, Dec < Lat, should be due south (180°)
        use coords_test_data::pub229_test_data;
        let (eq, lat, expected_hc, expected_zn) = pub229_test_data::test_case_meridian_south();

        let hz = equatorial_to_horizontal(&eq, lat);

        assert!(
            (hz.altitude - expected_hc).abs() < 0.02,
            "Hc mismatch: expected {:.2}°, got {:.2}°",
            expected_hc, hz.altitude
        );

        // Azimuth should be exactly 180° (or very close)
        assert!(
            (hz.azimuth - expected_zn).abs() < 1.0,
            "Azimuth should be 180° (due south), got {:.1}°",
            hz.azimuth
        );
    }

    #[test]
    fn test_azimuth_meridian_north() {
        // Body on meridian, Dec > Lat, should be due north (0° or 360°)
        use coords_test_data::pub229_test_data;
        let (eq, lat, expected_hc, _expected_zn) = pub229_test_data::test_case_meridian_north();

        let hz = equatorial_to_horizontal(&eq, lat);

        assert!(
            (hz.altitude - expected_hc).abs() < 0.02,
            "Hc mismatch: expected {:.2}°, got {:.2}°",
            expected_hc, hz.altitude
        );

        // Azimuth should be 0° or 360° (due north)
        let az_normalized = if hz.azimuth > 180.0 { 360.0 - hz.azimuth } else { hz.azimuth };
        assert!(
            az_normalized < 1.0,
            "Azimuth should be 0° (due north), got {:.1}°",
            hz.azimuth
        );
    }

    #[test]
    fn test_azimuth_east() {
        // Body due east (LHA = 90°), should have azimuth ~90°
        use coords_test_data::pub229_test_data;
        let (eq, lat, expected_hc, expected_zn) = pub229_test_data::test_case_east();

        let hz = equatorial_to_horizontal(&eq, lat);

        assert!(
            (hz.altitude - expected_hc).abs() < 0.02,
            "Hc mismatch: expected {:.2}°, got {:.2}°",
            expected_hc, hz.altitude
        );

        assert!(
            (hz.azimuth - expected_zn).abs() < 1.0,
            "Azimuth should be 90° (due east), got {:.1}°",
            hz.azimuth
        );
    }

    #[test]
    fn test_azimuth_west() {
        // Body due west (LHA = 270°), should have azimuth ~270°
        use coords_test_data::pub229_test_data;
        let (eq, lat, expected_hc, expected_zn) = pub229_test_data::test_case_west_horizon();

        let hz = equatorial_to_horizontal(&eq, lat);

        assert!(
            (hz.altitude - expected_hc).abs() < 0.02,
            "Hc mismatch: expected {:.2}°, got {:.2}°",
            expected_hc, hz.altitude
        );

        assert!(
            (hz.azimuth - expected_zn).abs() < 1.0,
            "Azimuth should be 270° (due west), got {:.1}°",
            hz.azimuth
        );
    }

    #[test]
    fn test_azimuth_western_lha() {
        // LHA > 180° (western side)
        use coords_test_data::pub229_test_data;
        let (eq, lat, expected_hc, expected_zn) = pub229_test_data::test_case_west();

        let hz = equatorial_to_horizontal(&eq, lat);

        assert!(
            (hz.altitude - expected_hc).abs() < 0.02,
            "Hc mismatch: expected {:.2}°, got {:.2}°",
            expected_hc, hz.altitude
        );

        assert!(
            (hz.azimuth - expected_zn).abs() < 1.0,
            "Azimuth mismatch: expected {}°, got {:.1}°",
            expected_zn, hz.azimuth
        );
    }
}
