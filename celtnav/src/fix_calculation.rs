//! Fix calculation from multiple lines of position (LOPs)
//!
//! This module provides functions for calculating a ship's position (fix) from
//! multiple celestial sight observations. It implements both 2-sight intersection
//! and least squares methods for 3+ sights.

/// A Line of Position (LOP) defined by an intercept and azimuth
///
/// An LOP is perpendicular to the azimuth at a distance equal to the intercept
/// from the assumed position (DR position).
#[derive(Debug, Clone, Copy)]
pub struct LineOfPosition {
    /// Azimuth to the celestial body in degrees (0-360)
    pub azimuth: f64,
    /// Intercept in nautical miles (positive = toward, negative = away)
    pub intercept: f64,
    /// DR Latitude in degrees (positive = North, negative = South)
    pub dr_latitude: f64,
    /// DR Longitude in degrees (positive = East, negative = West)
    pub dr_longitude: f64,
}

/// A geographical position (latitude and longitude)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Position {
    /// Latitude in degrees (positive = North, negative = South)
    pub latitude: f64,
    /// Longitude in degrees (positive = East, negative = West)
    pub longitude: f64,
}

/// A fix calculated from multiple LOPs
#[derive(Debug, Clone)]
pub struct Fix {
    /// Calculated position
    pub position: Position,
    /// Number of LOPs used in the fix
    pub num_lops: usize,
    /// Estimated accuracy in nautical miles (standard deviation for least squares)
    pub accuracy_estimate: Option<f64>,
}

/// Calculate a fix from two lines of position
///
/// This function finds the intersection point of two LOPs using geometric methods.
/// The fix is calculated by finding where the two position lines cross.
///
/// # Arguments
/// * `lop1` - First line of position
/// * `lop2` - Second line of position
///
/// # Returns
/// The calculated fix position, or None if the LOPs are parallel or nearly parallel
pub fn fix_from_two_lops(lop1: &LineOfPosition, lop2: &LineOfPosition) -> Option<Fix> {
    // Check if azimuths are too similar (lines nearly parallel)
    let azimuth_diff = (lop1.azimuth - lop2.azimuth).abs();
    let azimuth_diff_normalized = if azimuth_diff > 180.0 {
        360.0 - azimuth_diff
    } else {
        azimuth_diff
    };

    // If lines are within 10 degrees of parallel, no reliable fix
    if azimuth_diff_normalized < 10.0 || azimuth_diff_normalized > 170.0 {
        return None;
    }

    // Use average DR position as reference
    let dr_lat = (lop1.dr_latitude + lop2.dr_latitude) / 2.0;
    let dr_lon = (lop1.dr_longitude + lop2.dr_longitude) / 2.0;

    // Calculate perpendicular azimuths (LOP direction is perpendicular to body azimuth)
    let lop1_bearing = (lop1.azimuth + 90.0) % 360.0;
    let lop2_bearing = (lop2.azimuth + 90.0) % 360.0;

    // Calculate positions along each LOP at intercept distance from DR
    // Move along azimuth by intercept distance to get point on LOP
    let lop1_point = move_position(dr_lat, dr_lon, lop1.azimuth, lop1.intercept);
    let lop2_point = move_position(dr_lat, dr_lon, lop2.azimuth, lop2.intercept);

    // Find intersection of two lines using parametric form
    // Line 1: P1 + t * d1
    // Line 2: P2 + s * d2
    // where d1 and d2 are direction vectors along each LOP

    let lop1_bearing_rad = lop1_bearing.to_radians();
    let lop2_bearing_rad = lop2_bearing.to_radians();

    // Direction vectors (in lat/lon space, approximation valid for small areas)
    let d1_lat = lop1_bearing_rad.cos();
    let d1_lon = lop1_bearing_rad.sin();
    let d2_lat = lop2_bearing_rad.cos();
    let d2_lon = lop2_bearing_rad.sin();

    // Solve for intersection using parametric equations
    // (lop1_point.lat + t * d1_lat) = (lop2_point.lat + s * d2_lat)
    // (lop1_point.lon + t * d1_lon) = (lop2_point.lon + s * d2_lon)

    let det = d1_lat * d2_lon - d1_lon * d2_lat;

    if det.abs() < 1e-10 {
        // Lines are parallel
        return None;
    }

    let delta_lat = lop2_point.latitude - lop1_point.latitude;
    let delta_lon = lop2_point.longitude - lop1_point.longitude;

    let t = (delta_lat * d2_lon - delta_lon * d2_lat) / det;

    // Calculate intersection point
    let fix_lat = lop1_point.latitude + t * d1_lat / 60.0; // Convert NM to degrees
    let fix_lon = lop1_point.longitude + t * d1_lon / (60.0 * dr_lat.to_radians().cos());

    Some(Fix {
        position: Position {
            latitude: fix_lat,
            longitude: fix_lon,
        },
        num_lops: 2,
        accuracy_estimate: None,
    })
}

/// Calculate a fix from three or more lines of position using least squares method
///
/// This function uses a least squares approach to find the most likely position
/// when three or more LOPs are available. It minimizes the sum of squared distances
/// from the calculated position to each LOP.
///
/// # Arguments
/// * `lops` - Slice of three or more lines of position
///
/// # Returns
/// The calculated fix with accuracy estimate, or None if calculation fails
pub fn fix_from_multiple_lops(lops: &[LineOfPosition]) -> Option<Fix> {
    if lops.len() < 2 {
        return None;
    }

    // For exactly 2 LOPs, use the simpler intersection method
    if lops.len() == 2 {
        return fix_from_two_lops(&lops[0], &lops[1]);
    }

    // Use average DR position as initial estimate
    let dr_lat = lops.iter().map(|lop| lop.dr_latitude).sum::<f64>() / lops.len() as f64;
    let dr_lon = lops.iter().map(|lop| lop.dr_longitude).sum::<f64>() / lops.len() as f64;

    // Iterative least squares solution
    let mut current_lat = dr_lat;
    let mut current_lon = dr_lon;

    // Iterate to refine position
    for _ in 0..10 {
        let mut sum_a = 0.0;
        let mut sum_b = 0.0;
        let mut sum_c = 0.0;
        let mut sum_d = 0.0;
        let mut sum_e = 0.0;

        for lop in lops {
            // Point on this LOP from DR
            let lop_point = move_position(lop.dr_latitude, lop.dr_longitude, lop.azimuth, lop.intercept);

            // LOP bearing (perpendicular to azimuth)
            let _lop_bearing = (lop.azimuth + 90.0) % 360.0;

            // Normal to LOP (same as azimuth to body)
            let n_lat = lop.azimuth.to_radians().cos();
            let n_lon = lop.azimuth.to_radians().sin();

            // Distance from current position to LOP point
            let delta_lat = (current_lat - lop_point.latitude) * 60.0; // Convert to NM
            let delta_lon = (current_lon - lop_point.longitude) * 60.0 * current_lat.to_radians().cos();

            // Build normal equations for least squares
            sum_a += n_lat * n_lat;
            sum_b += n_lat * n_lon;
            sum_c += n_lon * n_lon;

            let residual = delta_lat * n_lat + delta_lon * n_lon;
            sum_d += residual * n_lat;
            sum_e += residual * n_lon;
        }

        // Solve normal equations
        let det = sum_a * sum_c - sum_b * sum_b;
        if det.abs() < 1e-10 {
            return None;
        }

        let delta_lat = -(sum_d * sum_c - sum_e * sum_b) / det;
        let delta_lon = -(sum_e * sum_a - sum_d * sum_b) / det;

        current_lat += delta_lat / 60.0; // Convert from NM to degrees
        current_lon += delta_lon / (60.0 * current_lat.to_radians().cos());

        // Check for convergence
        if delta_lat.abs() < 0.01 && delta_lon.abs() < 0.01 {
            break;
        }
    }

    // Calculate accuracy estimate (RMS residual)
    let mut sum_sq_residuals = 0.0;
    for lop in lops {
        let lop_point = move_position(lop.dr_latitude, lop.dr_longitude, lop.azimuth, lop.intercept);
        let delta_lat = (current_lat - lop_point.latitude) * 60.0;
        let delta_lon = (current_lon - lop_point.longitude) * 60.0 * current_lat.to_radians().cos();

        let n_lat = lop.azimuth.to_radians().cos();
        let n_lon = lop.azimuth.to_radians().sin();

        let residual = delta_lat * n_lat + delta_lon * n_lon;
        sum_sq_residuals += residual * residual;
    }

    let rms_error = (sum_sq_residuals / lops.len() as f64).sqrt();

    Some(Fix {
        position: Position {
            latitude: current_lat,
            longitude: current_lon,
        },
        num_lops: lops.len(),
        accuracy_estimate: Some(rms_error),
    })
}

/// Advance a position along a course for a given time period
///
/// This function is used for dead reckoning calculations and advancing
/// Lines of Position for running fixes.
///
/// # Arguments
/// * `lat` - Starting latitude in degrees (positive = North, negative = South)
/// * `lon` - Starting longitude in degrees (positive = East, negative = West)
/// * `course_deg` - Course in degrees (0-360, clockwise from north)
/// * `speed_knots` - Speed in knots
/// * `time_hours` - Time period in hours
///
/// # Returns
/// New position after advancing along the course
///
/// # Examples
/// ```
/// use celtnav::fix_calculation::advance_position;
///
/// // Advance 2 hours at 5 knots on course 090
/// let (new_lat, new_lon) = advance_position(40.0, -74.0, 90.0, 5.0, 2.0);
/// ```
pub fn advance_position(
    lat: f64,
    lon: f64,
    course_deg: f64,
    speed_knots: f64,
    time_hours: f64,
) -> (f64, f64) {
    let distance_nm = speed_knots * time_hours;
    let new_pos = move_position(lat, lon, course_deg, distance_nm);
    (new_pos.latitude, new_pos.longitude)
}

/// Advance a Line of Position to a new time based on vessel movement
///
/// This function is essential for running fixes, where LOPs taken at different
/// times need to be advanced to a common time before plotting a fix.
///
/// The advancement is done by moving the DR position along the course line
/// and maintaining the same azimuth and intercept relative to the new DR position.
///
/// # Arguments
/// * `lop` - The Line of Position to advance
/// * `course_deg` - Vessel course in degrees (0-360, clockwise from north)
/// * `speed_knots` - Vessel speed in knots
/// * `time_delta_hours` - Time to advance (positive value)
///
/// # Returns
/// Advanced Line of Position with new DR position
///
/// # Examples
/// ```
/// use celtnav::fix_calculation::{LineOfPosition, advance_lop};
///
/// let lop = LineOfPosition {
///     azimuth: 45.0,
///     intercept: 10.0,
///     dr_latitude: 40.0,
///     dr_longitude: -74.0,
/// };
///
/// // Advance 1 hour at 6 knots on course 000
/// let advanced = advance_lop(&lop, 0.0, 6.0, 1.0);
/// ```
pub fn advance_lop(
    lop: &LineOfPosition,
    course_deg: f64,
    speed_knots: f64,
    time_delta_hours: f64,
) -> LineOfPosition {
    let (new_lat, new_lon) = advance_position(
        lop.dr_latitude,
        lop.dr_longitude,
        course_deg,
        speed_knots,
        time_delta_hours,
    );

    LineOfPosition {
        azimuth: lop.azimuth,
        intercept: lop.intercept,
        dr_latitude: new_lat,
        dr_longitude: new_lon,
    }
}

/// Move a position along a bearing for a given distance
///
/// Uses simple spherical trigonometry for short distances.
///
/// # Arguments
/// * `lat` - Starting latitude in degrees
/// * `lon` - Starting longitude in degrees
/// * `bearing` - Bearing in degrees (0-360, clockwise from north)
/// * `distance` - Distance in nautical miles
///
/// # Returns
/// New position after the move
fn move_position(lat: f64, lon: f64, bearing: f64, distance: f64) -> Position {
    let lat_rad = lat.to_radians();
    let bearing_rad = bearing.to_radians();

    // Convert distance to angular distance (1 NM = 1 arcminute)
    let angular_distance_rad = (distance / 60.0).to_radians();

    // Calculate new latitude
    let new_lat_rad = (lat_rad.sin() * angular_distance_rad.cos()
        + lat_rad.cos() * angular_distance_rad.sin() * bearing_rad.cos())
    .asin();

    // Calculate new longitude
    let delta_lon_rad = (bearing_rad.sin() * angular_distance_rad.sin() * lat_rad.cos())
        .atan2(angular_distance_rad.cos() - lat_rad.sin() * new_lat_rad.sin());

    Position {
        latitude: new_lat_rad.to_degrees(),
        longitude: lon + delta_lon_rad.to_degrees(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_position_creation() {
        let pos = Position {
            latitude: 40.0,
            longitude: -74.0,
        };
        assert_eq!(pos.latitude, 40.0);
        assert_eq!(pos.longitude, -74.0);
    }

    #[test]
    fn test_lop_creation() {
        let lop = LineOfPosition {
            azimuth: 90.0,
            intercept: 10.0,
            dr_latitude: 40.0,
            dr_longitude: -74.0,
        };
        assert_eq!(lop.azimuth, 90.0);
        assert_eq!(lop.intercept, 10.0);
    }

    #[test]
    fn test_move_position_north() {
        let start = move_position(40.0, -74.0, 0.0, 60.0); // Move 60 NM north
        assert!((start.latitude - 41.0).abs() < 0.1); // Should be near 41°N
        assert!((start.longitude - (-74.0)).abs() < 0.1); // Longitude unchanged
    }

    #[test]
    fn test_move_position_east() {
        let start = move_position(40.0, -74.0, 90.0, 60.0); // Move 60 NM east
        assert!((start.latitude - 40.0).abs() < 0.1); // Latitude nearly unchanged
        assert!(start.longitude > -74.0); // Longitude should increase
    }

    #[test]
    fn test_fix_from_two_lops_perpendicular() {
        // Two perpendicular LOPs at DR position (should intersect at DR)
        let lop1 = LineOfPosition {
            azimuth: 0.0,   // North
            intercept: 0.0, // At DR position
            dr_latitude: 40.0,
            dr_longitude: -74.0,
        };

        let lop2 = LineOfPosition {
            azimuth: 90.0,  // East
            intercept: 0.0, // At DR position
            dr_latitude: 40.0,
            dr_longitude: -74.0,
        };

        let fix = fix_from_two_lops(&lop1, &lop2);
        assert!(fix.is_some());

        let fix = fix.unwrap();
        assert_eq!(fix.num_lops, 2);
        assert!((fix.position.latitude - 40.0).abs() < 0.1);
        assert!((fix.position.longitude - (-74.0)).abs() < 0.1);
    }

    #[test]
    fn test_fix_from_two_lops_parallel() {
        // Two nearly parallel LOPs (should return None)
        let lop1 = LineOfPosition {
            azimuth: 0.0,
            intercept: 0.0,
            dr_latitude: 40.0,
            dr_longitude: -74.0,
        };

        let lop2 = LineOfPosition {
            azimuth: 5.0, // Only 5 degrees different - nearly parallel
            intercept: 0.0,
            dr_latitude: 40.0,
            dr_longitude: -74.0,
        };

        let fix = fix_from_two_lops(&lop1, &lop2);
        assert!(fix.is_none()); // Should fail due to parallel lines
    }

    #[test]
    fn test_fix_from_two_lops_with_intercepts() {
        // Two LOPs with non-zero intercepts
        let lop1 = LineOfPosition {
            azimuth: 45.0,
            intercept: 10.0, // 10 NM toward
            dr_latitude: 40.0,
            dr_longitude: -74.0,
        };

        let lop2 = LineOfPosition {
            azimuth: 135.0,
            intercept: -5.0, // 5 NM away
            dr_latitude: 40.0,
            dr_longitude: -74.0,
        };

        let fix = fix_from_two_lops(&lop1, &lop2);
        assert!(fix.is_some());

        let fix = fix.unwrap();
        assert_eq!(fix.num_lops, 2);
        // Position should be different from DR due to intercepts
        assert!(
            (fix.position.latitude - 40.0).abs() > 0.01
                || (fix.position.longitude - (-74.0)).abs() > 0.01
        );
    }

    #[test]
    fn test_fix_from_multiple_lops_two() {
        // With exactly 2 LOPs, should use two-lop intersection
        let lops = vec![
            LineOfPosition {
                azimuth: 0.0,
                intercept: 0.0,
                dr_latitude: 40.0,
                dr_longitude: -74.0,
            },
            LineOfPosition {
                azimuth: 90.0,
                intercept: 0.0,
                dr_latitude: 40.0,
                dr_longitude: -74.0,
            },
        ];

        let fix = fix_from_multiple_lops(&lops);
        assert!(fix.is_some());

        let fix = fix.unwrap();
        assert_eq!(fix.num_lops, 2);
    }

    #[test]
    fn test_fix_from_multiple_lops_three() {
        // Three LOPs forming a triangle around a point
        let lops = vec![
            LineOfPosition {
                azimuth: 0.0,
                intercept: 5.0,
                dr_latitude: 40.0,
                dr_longitude: -74.0,
            },
            LineOfPosition {
                azimuth: 120.0,
                intercept: 5.0,
                dr_latitude: 40.0,
                dr_longitude: -74.0,
            },
            LineOfPosition {
                azimuth: 240.0,
                intercept: 5.0,
                dr_latitude: 40.0,
                dr_longitude: -74.0,
            },
        ];

        let fix = fix_from_multiple_lops(&lops);
        assert!(fix.is_some());

        let fix = fix.unwrap();
        assert_eq!(fix.num_lops, 3);
        assert!(fix.accuracy_estimate.is_some());
        // With symmetric LOPs, fix should be near DR
        assert!((fix.position.latitude - 40.0).abs() < 0.2);
        assert!((fix.position.longitude - (-74.0)).abs() < 0.2);
    }

    #[test]
    fn test_fix_from_multiple_lops_insufficient() {
        // Less than 2 LOPs should return None
        let lops = vec![LineOfPosition {
            azimuth: 0.0,
            intercept: 0.0,
            dr_latitude: 40.0,
            dr_longitude: -74.0,
        }];

        let fix = fix_from_multiple_lops(&lops);
        assert!(fix.is_none());
    }

    #[test]
    fn test_fix_from_multiple_lops_empty() {
        let lops: Vec<LineOfPosition> = vec![];
        let fix = fix_from_multiple_lops(&lops);
        assert!(fix.is_none());
    }

    #[test]
    fn test_fix_accuracy_estimate_exists_for_three_lops() {
        let lops = vec![
            LineOfPosition {
                azimuth: 30.0,
                intercept: 0.0,
                dr_latitude: 40.0,
                dr_longitude: -74.0,
            },
            LineOfPosition {
                azimuth: 120.0,
                intercept: 0.0,
                dr_latitude: 40.0,
                dr_longitude: -74.0,
            },
            LineOfPosition {
                azimuth: 210.0,
                intercept: 0.0,
                dr_latitude: 40.0,
                dr_longitude: -74.0,
            },
        ];

        let fix = fix_from_multiple_lops(&lops).unwrap();
        assert!(fix.accuracy_estimate.is_some());
        assert!(fix.accuracy_estimate.unwrap() >= 0.0);
    }

    // Tests for Running Fix (Advancing LOPs)

    #[test]
    fn test_advance_position_north() {
        // Advance 1 hour at 6 knots on course 000 (north)
        let (new_lat, new_lon) = advance_position(40.0, -74.0, 0.0, 6.0, 1.0);

        // Should move 6 NM north = 0.1 degrees north
        assert!((new_lat - 40.1).abs() < 0.01, "Expected ~40.1°N, got {}°", new_lat);
        assert!((new_lon - (-74.0)).abs() < 0.01, "Longitude should remain ~-74.0°, got {}°", new_lon);
    }

    #[test]
    fn test_advance_position_east() {
        // Advance 2 hours at 5 knots on course 090 (east)
        let (new_lat, new_lon) = advance_position(40.0, -74.0, 90.0, 5.0, 2.0);

        // Should move 10 NM east
        // At 40°N, 1° longitude ≈ 60 * cos(40°) ≈ 46 NM
        // So 10 NM ≈ 10/46 ≈ 0.217°
        assert!((new_lat - 40.0).abs() < 0.01, "Latitude should remain ~40.0°, got {}°", new_lat);
        assert!(new_lon > -74.0, "Longitude should increase from -74.0°, got {}°", new_lon);
        assert!((new_lon - (-73.783)).abs() < 0.05, "Expected ~-73.78°, got {}°", new_lon);
    }

    #[test]
    fn test_advance_position_south() {
        // Advance 30 minutes (0.5 hours) at 10 knots on course 180 (south)
        let (new_lat, new_lon) = advance_position(40.0, -74.0, 180.0, 10.0, 0.5);

        // Should move 5 NM south = 0.0833 degrees south
        assert!((new_lat - 39.917).abs() < 0.01, "Expected ~39.92°N, got {}°", new_lat);
        assert!((new_lon - (-74.0)).abs() < 0.01, "Longitude should remain ~-74.0°, got {}°", new_lon);
    }

    #[test]
    fn test_advance_position_northeast() {
        // Advance 1 hour at 6 knots on course 045 (northeast)
        let (new_lat, new_lon) = advance_position(40.0, -74.0, 45.0, 6.0, 1.0);

        // Should move 6 NM on bearing 045
        // North component: 6 * cos(45°) ≈ 4.24 NM ≈ 0.071°
        // East component: 6 * sin(45°) ≈ 4.24 NM
        assert!(new_lat > 40.0, "Latitude should increase from 40.0°, got {}°", new_lat);
        assert!(new_lon > -74.0, "Longitude should increase from -74.0°, got {}°", new_lon);
        assert!((new_lat - 40.071).abs() < 0.02, "Expected ~40.07°N, got {}°", new_lat);
    }

    #[test]
    fn test_advance_position_zero_time() {
        // Advance 0 hours - position should not change
        let (new_lat, new_lon) = advance_position(40.0, -74.0, 90.0, 6.0, 0.0);

        assert!((new_lat - 40.0).abs() < 1e-10);
        assert!((new_lon - (-74.0)).abs() < 1e-10);
    }

    #[test]
    fn test_advance_position_zero_speed() {
        // Speed 0 - position should not change
        let (new_lat, new_lon) = advance_position(40.0, -74.0, 90.0, 0.0, 2.0);

        assert!((new_lat - 40.0).abs() < 1e-10);
        assert!((new_lon - (-74.0)).abs() < 1e-10);
    }

    #[test]
    fn test_advance_lop_maintains_azimuth_and_intercept() {
        let lop = LineOfPosition {
            azimuth: 45.0,
            intercept: 10.0,
            dr_latitude: 40.0,
            dr_longitude: -74.0,
        };

        // Advance 1 hour at 6 knots on course 000
        let advanced = advance_lop(&lop, 0.0, 6.0, 1.0);

        // Azimuth and intercept should remain the same
        assert_eq!(advanced.azimuth, 45.0);
        assert_eq!(advanced.intercept, 10.0);

        // DR position should move 6 NM north
        assert!((advanced.dr_latitude - 40.1).abs() < 0.01);
        assert!((advanced.dr_longitude - (-74.0)).abs() < 0.01);
    }

    #[test]
    fn test_advance_lop_different_course() {
        let lop = LineOfPosition {
            azimuth: 90.0,
            intercept: -5.0,
            dr_latitude: 35.0,
            dr_longitude: -75.0,
        };

        // Advance 2 hours at 5 knots on course 270 (west)
        let advanced = advance_lop(&lop, 270.0, 5.0, 2.0);

        // Azimuth and intercept unchanged
        assert_eq!(advanced.azimuth, 90.0);
        assert_eq!(advanced.intercept, -5.0);

        // Should move 10 NM west
        assert!((advanced.dr_latitude - 35.0).abs() < 0.01);
        assert!(advanced.dr_longitude < -75.0);
    }

    #[test]
    fn test_advance_lop_zero_time() {
        let lop = LineOfPosition {
            azimuth: 180.0,
            intercept: 2.5,
            dr_latitude: 40.0,
            dr_longitude: -74.0,
        };

        // Advance 0 hours - LOP should not change
        let advanced = advance_lop(&lop, 90.0, 6.0, 0.0);

        assert_eq!(advanced.azimuth, lop.azimuth);
        assert_eq!(advanced.intercept, lop.intercept);
        assert!((advanced.dr_latitude - lop.dr_latitude).abs() < 1e-10);
        assert!((advanced.dr_longitude - lop.dr_longitude).abs() < 1e-10);
    }

    #[test]
    fn test_running_fix_scenario() {
        // Realistic running fix scenario:
        // First sight at 08:00, second sight at 09:30
        // Vessel on course 045° at 6 knots
        // Time difference: 1.5 hours

        let lop1 = LineOfPosition {
            azimuth: 90.0,  // Sun bearing East
            intercept: 5.0,
            dr_latitude: 40.0,
            dr_longitude: -74.0,
        };

        // Advance first LOP by 1.5 hours
        let lop1_advanced = advance_lop(&lop1, 45.0, 6.0, 1.5);

        // Second LOP taken at 09:30
        let lop2 = LineOfPosition {
            azimuth: 135.0,  // Sun bearing SE
            intercept: -3.0,
            dr_latitude: 40.106,  // Approximate DR at 09:30
            dr_longitude: -73.858,
        };

        // Both LOPs should now be at approximately the same time
        // DR positions should be close
        assert!((lop1_advanced.dr_latitude - lop2.dr_latitude).abs() < 0.01);
        assert!((lop1_advanced.dr_longitude - lop2.dr_longitude).abs() < 0.01);

        // Should be able to compute fix from both LOPs
        let fix = fix_from_two_lops(&lop1_advanced, &lop2);
        assert!(fix.is_some(), "Should be able to compute fix from advanced LOPs");
    }

    #[test]
    fn test_running_fix_three_lops_different_times() {
        // Three LOPs taken at different times, all need advancing
        // Vessel steaming on course 000° (north) at 6 knots

        // First sight at T=0
        let lop1 = LineOfPosition {
            azimuth: 45.0,
            intercept: 8.0,
            dr_latitude: 35.0,
            dr_longitude: -70.0,
        };

        // Second sight at T+1 hour
        let lop2 = LineOfPosition {
            azimuth: 135.0,
            intercept: -4.0,
            dr_latitude: 35.1,  // Moved 6 NM north = 0.1°
            dr_longitude: -70.0,
        };

        // Third sight at T+2 hours
        let lop3 = LineOfPosition {
            azimuth: 225.0,
            intercept: 6.0,
            dr_latitude: 35.2,  // Moved 12 NM north total = 0.2°
            dr_longitude: -70.0,
        };

        // Advance first two LOPs to T+2
        let lop1_advanced = advance_lop(&lop1, 0.0, 6.0, 2.0);
        let lop2_advanced = advance_lop(&lop2, 0.0, 6.0, 1.0);

        // All LOPs should now have similar DR positions (within 0.01°)
        assert!((lop1_advanced.dr_latitude - lop3.dr_latitude).abs() < 0.01,
            "Expected lat ~35.2, got {}", lop1_advanced.dr_latitude);
        assert!((lop1_advanced.dr_longitude - lop3.dr_longitude).abs() < 0.01,
            "Expected lon ~-70.0, got {}", lop1_advanced.dr_longitude);
        assert!((lop2_advanced.dr_latitude - lop3.dr_latitude).abs() < 0.01,
            "Expected lat ~35.2, got {}", lop2_advanced.dr_latitude);
        assert!((lop2_advanced.dr_longitude - lop3.dr_longitude).abs() < 0.01,
            "Expected lon ~-70.0, got {}", lop2_advanced.dr_longitude);

        // Compute fix from all three advanced LOPs
        let lops = vec![lop1_advanced, lop2_advanced, lop3];
        let fix = fix_from_multiple_lops(&lops);

        assert!(fix.is_some(), "Should be able to compute fix from 3 advanced LOPs");
        let fix = fix.unwrap();
        assert_eq!(fix.num_lops, 3);
        assert!(fix.accuracy_estimate.is_some());
    }

    #[test]
    fn test_advance_position_long_distance() {
        // Test advancing over a long distance (12 hours at 10 knots = 120 NM)
        let (new_lat, new_lon) = advance_position(0.0, 0.0, 0.0, 10.0, 12.0);

        // Should move 120 NM north = 2 degrees north
        assert!((new_lat - 2.0).abs() < 0.01, "Expected ~2.0°N, got {}°", new_lat);
        assert!((new_lon - 0.0).abs() < 0.01, "Longitude should remain ~0.0°, got {}°", new_lon);
    }

    #[test]
    fn test_advance_lop_maintains_geometry() {
        // Verify that advancing an LOP maintains its geometric relationship
        let lop = LineOfPosition {
            azimuth: 180.0,  // Body to south
            intercept: 15.0,  // 15 NM toward (closer than calculated)
            dr_latitude: 45.0,
            dr_longitude: 0.0,
        };

        // Advance 3 hours at 4 knots on course 090 (east)
        let advanced = advance_lop(&lop, 90.0, 4.0, 3.0);

        // Should move 12 NM east
        assert!((advanced.dr_latitude - 45.0).abs() < 0.01);
        assert!(advanced.dr_longitude > 0.0);

        // Azimuth and intercept remain constant
        assert_eq!(advanced.azimuth, 180.0);
        assert_eq!(advanced.intercept, 15.0);
    }
}
