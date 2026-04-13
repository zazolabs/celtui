//! Sight averaging functionality
//!
//! This module provides functions to average multiple sextant observations
//! of the same celestial body, reducing observational errors.

use chrono::{NaiveTime, Timelike};

/// Represents a single sextant observation
#[derive(Debug, Clone)]
pub struct SextantObservation {
    /// Time of observation (HH:MM:SS)
    pub time: NaiveTime,
    /// Sextant altitude in degrees
    pub altitude_degrees: f64,
    /// Sextant altitude in arcminutes
    pub altitude_minutes: f64,
}

/// Averaged sight result
#[derive(Debug, Clone)]
pub struct AveragedSight {
    /// Average time of all observations
    pub avg_time: NaiveTime,
    /// Average altitude in degrees
    pub avg_altitude_degrees: f64,
    /// Average altitude in arcminutes
    pub avg_altitude_minutes: f64,
}

/// Average multiple sextant observations
///
/// Calculates the mean time and mean altitude from multiple observations.
/// Returns None if there are fewer than 2 observations.
///
/// # Arguments
/// * `observations` - Slice of sextant observations to average
///
/// # Returns
/// Some(AveragedSight) if successful, None if insufficient observations
pub fn average_sights(observations: &[SextantObservation]) -> Option<AveragedSight> {
    if observations.len() < 2 {
        return None;
    }

    // Convert times to seconds since midnight for averaging
    let total_seconds: u32 = observations
        .iter()
        .map(|obs| obs.time.num_seconds_from_midnight())
        .sum();

    let avg_seconds = total_seconds / observations.len() as u32;
    let avg_time = NaiveTime::from_num_seconds_from_midnight_opt(avg_seconds, 0).unwrap();

    // Convert altitudes to total arcminutes, average, then convert back
    let total_arcminutes: f64 = observations
        .iter()
        .map(|obs| obs.altitude_degrees * 60.0 + obs.altitude_minutes)
        .sum();

    let avg_arcminutes = total_arcminutes / observations.len() as f64;
    let avg_degrees = (avg_arcminutes / 60.0).floor();
    let avg_minutes = avg_arcminutes - (avg_degrees * 60.0);

    Some(AveragedSight {
        avg_time,
        avg_altitude_degrees: avg_degrees,
        avg_altitude_minutes: avg_minutes,
    })
}

/// Validate that altitude is within acceptable range (0-90 degrees)
///
/// # Arguments
/// * `degrees` - Degrees component of altitude
/// * `minutes` - Minutes component of altitude
///
/// # Returns
/// true if altitude is valid, false otherwise
pub fn validate_altitude(degrees: f64, minutes: f64) -> bool {
    if degrees < 0.0 || degrees > 90.0 {
        return false;
    }

    if minutes < 0.0 || minutes >= 60.0 {
        return false;
    }

    // Check if total altitude exceeds 90 degrees
    let total_degrees = degrees + (minutes / 60.0);
    total_degrees >= 0.0 && total_degrees <= 90.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_average_two_sights_simple() {
        let obs1 = SextantObservation {
            time: NaiveTime::from_hms_opt(10, 30, 0).unwrap(),
            altitude_degrees: 45.0,
            altitude_minutes: 30.0,
        };

        let obs2 = SextantObservation {
            time: NaiveTime::from_hms_opt(10, 32, 0).unwrap(),
            altitude_degrees: 45.0,
            altitude_minutes: 40.0,
        };

        let observations = vec![obs1, obs2];
        let avg = average_sights(&observations).unwrap();

        assert_eq!(avg.avg_time, NaiveTime::from_hms_opt(10, 31, 0).unwrap());
        assert_eq!(avg.avg_altitude_degrees, 45.0);
        assert!((avg.avg_altitude_minutes - 35.0).abs() < 0.01);
    }

    #[test]
    fn test_average_three_sights() {
        let obs1 = SextantObservation {
            time: NaiveTime::from_hms_opt(10, 30, 0).unwrap(),
            altitude_degrees: 45.0,
            altitude_minutes: 30.0,
        };

        let obs2 = SextantObservation {
            time: NaiveTime::from_hms_opt(10, 31, 0).unwrap(),
            altitude_degrees: 45.0,
            altitude_minutes: 33.0,
        };

        let obs3 = SextantObservation {
            time: NaiveTime::from_hms_opt(10, 32, 0).unwrap(),
            altitude_degrees: 45.0,
            altitude_minutes: 36.0,
        };

        let observations = vec![obs1, obs2, obs3];
        let avg = average_sights(&observations).unwrap();

        assert_eq!(avg.avg_time, NaiveTime::from_hms_opt(10, 31, 0).unwrap());
        assert_eq!(avg.avg_altitude_degrees, 45.0);
        assert!((avg.avg_altitude_minutes - 33.0).abs() < 0.01);
    }

    #[test]
    fn test_average_sights_with_degree_differences() {
        let obs1 = SextantObservation {
            time: NaiveTime::from_hms_opt(10, 30, 0).unwrap(),
            altitude_degrees: 44.0,
            altitude_minutes: 58.0,
        };

        let obs2 = SextantObservation {
            time: NaiveTime::from_hms_opt(10, 31, 0).unwrap(),
            altitude_degrees: 45.0,
            altitude_minutes: 2.0,
        };

        let observations = vec![obs1, obs2];
        let avg = average_sights(&observations).unwrap();

        assert_eq!(avg.avg_altitude_degrees, 45.0);
        assert!((avg.avg_altitude_minutes - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_average_insufficient_sights() {
        let obs1 = SextantObservation {
            time: NaiveTime::from_hms_opt(10, 30, 0).unwrap(),
            altitude_degrees: 45.0,
            altitude_minutes: 30.0,
        };

        let observations = vec![obs1];
        let result = average_sights(&observations);

        assert!(result.is_none());
    }

    #[test]
    fn test_average_empty_sights() {
        let observations: Vec<SextantObservation> = vec![];
        let result = average_sights(&observations);

        assert!(result.is_none());
    }

    #[test]
    fn test_average_time_with_seconds() {
        let obs1 = SextantObservation {
            time: NaiveTime::from_hms_opt(10, 30, 15).unwrap(),
            altitude_degrees: 45.0,
            altitude_minutes: 30.0,
        };

        let obs2 = SextantObservation {
            time: NaiveTime::from_hms_opt(10, 30, 45).unwrap(),
            altitude_degrees: 45.0,
            altitude_minutes: 32.0,
        };

        let observations = vec![obs1, obs2];
        let avg = average_sights(&observations).unwrap();

        assert_eq!(avg.avg_time, NaiveTime::from_hms_opt(10, 30, 30).unwrap());
    }

    #[test]
    fn test_validate_altitude_range() {
        assert!(!validate_altitude(95.0, 0.0)); // Invalid: > 90
        assert!(validate_altitude(45.0, 30.0)); // Valid
        assert!(!validate_altitude(-5.0, 0.0)); // Invalid: < 0
        assert!(!validate_altitude(45.0, 65.0)); // Invalid: minutes >= 60
        assert!(validate_altitude(0.0, 0.0)); // Valid: minimum
        assert!(validate_altitude(90.0, 0.0)); // Valid: maximum
        assert!(!validate_altitude(89.0, 61.0)); // Invalid: exceeds 90° total
    }
}
