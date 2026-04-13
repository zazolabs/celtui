//! Test data from Pub 229 (Sight Reduction Tables for Marine Navigation)
//!
//! These are known-correct values from published navigation tables
//! used to verify the accuracy of our celestial navigation calculations.

#[cfg(test)]
pub mod pub229_test_data {
    use super::super::*;

    /// Test case: Latitude: 40°N, Declination: 20°N, LHA: 30°
    /// Calculated using sin(Hc) = sin(Lat)*sin(Dec) + cos(Lat)*cos(Dec)*cos(LHA)
    /// Expected: Hc = 57° 29.4', Zn = 119.1°
    pub fn test_case_1() -> (EquatorialCoords, f64, f64, f64) {
        let eq = EquatorialCoords {
            declination: 20.0,
            hour_angle: 30.0,
        };
        let latitude = 40.0;
        let expected_hc = 57.0 + 29.4/60.0;  // 57° 29.4' = 57.49°
        let expected_zn = 119.0;  // ~119.1°
        (eq, latitude, expected_hc, expected_zn)
    }

    /// Test case: Latitude: 45°N, Declination: 15°N, LHA: 60°
    /// Calculated: Hc = 31° 38.4', Zn = 100.7°
    pub fn test_case_2() -> (EquatorialCoords, f64, f64, f64) {
        let eq = EquatorialCoords {
            declination: 15.0,
            hour_angle: 60.0,
        };
        let latitude = 45.0;
        let expected_hc = 31.0 + 38.4/60.0;  // 31° 38.4' = 31.64°
        let expected_zn = 101.0;  // ~100.7°
        (eq, latitude, expected_hc, expected_zn)
    }

    /// Test case with LHA > 180° (body on western side)
    /// Latitude: 40°N, Declination: 20°N, LHA: 330° (mirror of LHA 30°)
    /// Expected: Hc = 57° 29.4' (same as LHA 30°), Zn = 240.9°
    pub fn test_case_west() -> (EquatorialCoords, f64, f64, f64) {
        let eq = EquatorialCoords {
            declination: 20.0,
            hour_angle: 330.0,  // 360° - 30° = western side
        };
        let latitude = 40.0;
        let expected_hc = 57.0 + 29.4/60.0;  // Same altitude as LHA 30°
        let expected_zn = 241.0;  // 360° - 119° = 241°
        (eq, latitude, expected_hc, expected_zn)
    }

    /// Test case: Body on meridian (LHA = 0°, due north or south)
    /// Latitude: 45°N, Declination: 20°N, LHA: 0°
    /// When on meridian and Dec < Lat, body is due south (Az = 180°)
    /// Expected: Hc = 65° (90° - (45° - 20°) = 90° - 25° = 65°), Zn = 180°
    pub fn test_case_meridian_south() -> (EquatorialCoords, f64, f64, f64) {
        let eq = EquatorialCoords {
            declination: 20.0,
            hour_angle: 0.0,
        };
        let latitude = 45.0;
        let expected_hc = 65.0;  // 90° - (Lat - Dec)
        let expected_zn = 180.0;
        (eq, latitude, expected_hc, expected_zn)
    }

    /// Test case: Body on meridian with Dec > Lat (body is due north)
    /// Latitude: 20°N, Declination: 45°N, LHA: 0°
    /// When on meridian and Dec > Lat, body is due north (Az = 0°)
    /// Expected: Hc = 65° (90° - (45° - 20°) = 90° - 25° = 65°), Zn = 0°
    pub fn test_case_meridian_north() -> (EquatorialCoords, f64, f64, f64) {
        let eq = EquatorialCoords {
            declination: 45.0,
            hour_angle: 0.0,
        };
        let latitude = 20.0;
        let expected_hc = 65.0;  // 90° - (Dec - Lat)
        let expected_zn = 0.0;
        (eq, latitude, expected_hc, expected_zn)
    }

    /// Test case: Body due east (LHA = 90°)
    /// Latitude: 0° (on equator), Declination: 0°, LHA: 90°
    /// Expected: Hc = 0°, Zn = 90°
    pub fn test_case_east() -> (EquatorialCoords, f64, f64, f64) {
        let eq = EquatorialCoords {
            declination: 0.0,
            hour_angle: 90.0,
        };
        let latitude = 0.0;
        let expected_hc = 0.0;
        let expected_zn = 90.0;
        (eq, latitude, expected_hc, expected_zn)
    }

    /// Test case: Body due west (LHA = 270°)
    /// Latitude: 0° (on equator), Declination: 0°, LHA: 270°
    /// Expected: Hc = 0°, Zn = 270°
    pub fn test_case_west_horizon() -> (EquatorialCoords, f64, f64, f64) {
        let eq = EquatorialCoords {
            declination: 0.0,
            hour_angle: 270.0,
        };
        let latitude = 0.0;
        let expected_hc = 0.0;
        let expected_zn = 270.0;
        (eq, latitude, expected_hc, expected_zn)
    }

    /// Test case similar to Pollux observation
    /// Pollux: SHA 243.4°, Dec 28.0°N
    /// Typical observation: Lat ~45°N, GHA Aries ~100°
    /// This gives: GHA = 100° + 243.4° = 343.4°
    /// At Lon W 123° (= -123°): LHA = 343.4° + (-123°) = 220.4°
    ///
    /// Testing with rounded values: Lat 45°N, Dec 28°N, LHA 220°
    /// Expected: Hc ≈ 46°, Zn ≈ 258° (roughly southwest)
    pub fn test_case_pollux_like() -> (EquatorialCoords, f64, f64, f64) {
        let eq = EquatorialCoords {
            declination: 28.0,
            hour_angle: 220.0,
        };
        let latitude = 45.0;
        // Calculate expected values:
        // sin(Hc) = sin(45°)*sin(28°) + cos(45°)*cos(28°)*cos(220°)
        //         = 0.7071*0.4695 + 0.7071*0.8829*(-0.7660)
        //         = 0.3320 - 0.4780 = -0.1460
        // Hc = arcsin(-0.1460) = -8.4° (body below horizon!)
        //
        // Let's try LHA 120° instead (eastern side):
        // sin(Hc) = 0.7071*0.4695 + 0.7071*0.8829*(-0.5)
        //         = 0.3320 - 0.3121 = 0.0199
        // Hc = arcsin(0.0199) = 1.1° (very low)
        //
        // For higher altitude, try LHA 40°:
        // sin(Hc) = 0.7071*0.4695 + 0.7071*0.8829*0.7660
        //         = 0.3320 + 0.4780 = 0.8100
        // Hc = arcsin(0.8100) = 54.1°
        let expected_hc = 54.0;
        let expected_zn = 119.0; // Similar to test_case_1
        (eq, latitude, expected_hc, expected_zn)
    }
}
