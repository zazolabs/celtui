//! Twilight calculations for celestial navigation
//!
//! This module provides functions to calculate twilight times and
//! determine which celestial bodies are visible for sextant observations.

use crate::almanac::{sun_gha, sun_declination, get_star_catalog, planet_gha, planet_declination, Planet, find_star_for_year, is_leap_year};
use crate::sight_reduction::{compute_altitude, SightData};
use chrono::{DateTime, Utc, Duration, Timelike, Datelike};

/// Nautical twilight: Sun is 12° below horizon
pub const NAUTICAL_TWILIGHT_ANGLE: f64 = -12.0;

/// Civil twilight: Sun is 6° below horizon
pub const CIVIL_TWILIGHT_ANGLE: f64 = -6.0;

/// Astronomical twilight: Sun is 18° below horizon
pub const ASTRONOMICAL_TWILIGHT_ANGLE: f64 = -18.0;

/// Sunrise/Sunset: Sun center at horizon with refraction correction
pub const SUNRISE_SUNSET_ANGLE: f64 = -0.833;

/// Best altitude range for sextant observations (degrees)
pub const MIN_SEXTANT_ALTITUDE: f64 = 15.0;
pub const MAX_SEXTANT_ALTITUDE: f64 = 75.0;

/// Altitude range for navigation (degrees) - matches Pub 249 Vol. 1 selection range.
/// Pub 249 Vol. 1 uses 15°–75°; below 15° refraction errors dominate; above 75° sextant
/// observation becomes awkward.
pub const OPTIMAL_MIN_ALTITUDE: f64 = 15.0;
pub const OPTIMAL_MAX_ALTITUDE: f64 = 60.0;

/// Non-standard stars: in our catalog but NOT in the Pub 249 57-star selection list.
/// These are shown in the twilight screen but excluded from the LOP selection algorithm.
/// Scheat (β Peg) and Alnilam (ε Ori) are common bright stars but not in the 57.
const NON_STANDARD_STARS: &[&str] = &["Mirach", "Alnitak", "Naos", "Saiph", "Polaris", "Scheat", "Alnilam"];

/// Big Dipper stars: finding the asterism gives you these stars for free.
const ASTERISM_BIG_DIPPER: &[&str] = &["Dubhe", "Alioth", "Alkaid"];

/// Southern Cross stars: fixed beacon of the southern sky.
const ASTERISM_SOUTHERN_CROSS: &[&str] = &["Acrux", "Gacrux"];

/// Stars that are distinctively identifiable beyond raw brightness:
/// Rigel/Betelgeuse — navigate from Orion's belt; Antares/Aldebaran — unmistakable colour.
const DISTINCTIVE_STARS: &[&str] = &["Rigel", "Betelgeuse", "Antares", "Aldebaran"];

/// Minimum azimuth separation (degrees) between any two stars in a selected trio.
/// Pub 249 Vol. 1 requires at least 40-45° azimuth difference to ensure LOPs cross
/// at useful angles. Combinations where any pair is within this angle are rejected.
const MIN_AZIMUTH_SEPARATION: f64 = 45.0;

/// Geometry threshold for evening: geometry is essentially always decisive.
const GEOMETRY_THRESHOLD_EVENING: f64 = 0.07;

/// Geometry threshold for morning: allow wider search so period (western sky) preference
/// can override a marginally better-geometry combo that favours the eastern sky.
const GEOMETRY_THRESHOLD_MORNING: f64 = 0.08;

/// Geometry threshold for Vol.1 band mode: same as evening, keeping geometry decisive
/// while allowing ease/brightness to break near-ties.
const GEOMETRY_THRESHOLD_BAND: f64 = 0.07;

/// Twilight times for a given date and location
#[derive(Debug, Clone)]
pub struct TwilightTimes {
    /// Morning nautical twilight (Sun at -12°)
    pub morning_nautical: Option<DateTime<Utc>>,
    /// Evening nautical twilight (Sun at -12°)
    pub evening_nautical: Option<DateTime<Utc>>,
    /// Morning civil twilight (Sun at -6°)
    pub morning_civil: Option<DateTime<Utc>>,
    /// Evening civil twilight (Sun at -6°)
    pub evening_civil: Option<DateTime<Utc>>,
    /// Sunrise (Sun center at -0.833°, accounting for refraction)
    pub sunrise: Option<DateTime<Utc>>,
    /// Sunset (Sun center at -0.833°, accounting for refraction)
    pub sunset: Option<DateTime<Utc>>,
}

/// Information about a visible celestial body
#[derive(Debug, Clone)]
pub struct VisibleBody {
    pub name: String,
    pub altitude: f64,  // degrees
    pub azimuth: f64,   // degrees (0-360, from North clockwise)
    pub gha: f64,       // degrees
    pub sha: Option<f64>, // Sidereal Hour Angle (stars only, None for planets)
    pub declination: f64, // degrees
    pub magnitude: Option<f64>, // visual magnitude (lower = brighter), None for planets
    pub is_recommended: bool,  // true if this is one of the best 3 stars for LOP
    pub is_second_best: bool,  // true if this is one of the next 4 backup stars
}

/// Calculate Sun altitude at a given time and position
fn sun_altitude_at_time(datetime: DateTime<Utc>, latitude: f64, longitude: f64) -> f64 {
    let gha = sun_gha(datetime);
    let dec = sun_declination(datetime);
    let lha = (gha + longitude + 360.0) % 360.0;

    let sight_data = SightData {
        latitude,
        declination: dec,
        local_hour_angle: lha,
    };

    compute_altitude(&sight_data)
}

/// Find time when Sun reaches a specific altitude (e.g., -12° for nautical twilight)
///
/// Uses iterative search to find when Sun altitude crosses the target.
/// Search range is from start_time to start_time + 24 hours.
fn find_sun_altitude_time(
    start_time: DateTime<Utc>,
    latitude: f64,
    longitude: f64,
    target_altitude: f64,
    is_rising: bool,
) -> Option<DateTime<Utc>> {
    let search_duration_hours = 24;
    let time_step_minutes = 5;

    let mut current_time = start_time;
    let end_time = start_time + Duration::hours(search_duration_hours);

    let mut prev_alt = sun_altitude_at_time(current_time, latitude, longitude);

    while current_time < end_time {
        current_time += Duration::minutes(time_step_minutes);
        let current_alt = sun_altitude_at_time(current_time, latitude, longitude);

        // Check if we crossed the target altitude
        let crossed = if is_rising {
            prev_alt < target_altitude && current_alt >= target_altitude
        } else {
            prev_alt > target_altitude && current_alt <= target_altitude
        };

        if crossed {
            // Refine with binary search for better accuracy
            return Some(refine_twilight_time(
                current_time - Duration::minutes(time_step_minutes),
                current_time,
                latitude,
                longitude,
                target_altitude,
            ));
        }

        prev_alt = current_alt;
    }

    None
}

/// Refine twilight time using binary search
fn refine_twilight_time(
    start: DateTime<Utc>,
    end: DateTime<Utc>,
    latitude: f64,
    longitude: f64,
    target_altitude: f64,
) -> DateTime<Utc> {
    let mut low = start;
    let mut high = end;

    // Binary search to 1-second precision
    while (high - low).num_seconds() > 1 {
        let mid = low + (high - low) / 2;
        let alt = sun_altitude_at_time(mid, latitude, longitude);

        if alt < target_altitude {
            low = mid;
        } else {
            high = mid;
        }
    }

    low
}

/// Calculate twilight times for a given date and DR position
///
/// # Arguments
/// * `date` - Date to calculate twilight for (time component is ignored)
/// * `latitude` - DR latitude in degrees (North positive)
/// * `longitude` - DR longitude in degrees (East positive, West negative)
///
/// # Returns
/// TwilightTimes struct with morning and evening twilight times
pub fn calculate_twilight_times(
    date: DateTime<Utc>,
    latitude: f64,
    longitude: f64,
) -> TwilightTimes {
    // Start search from midnight UTC
    let start_of_day = date
        .with_hour(0).unwrap()
        .with_minute(0).unwrap()
        .with_second(0).unwrap();

    // Search for morning twilight starting from midnight
    // (Sun rising through -12° in the morning)
    let morning_nautical = find_sun_altitude_time(
        start_of_day,
        latitude,
        longitude,
        NAUTICAL_TWILIGHT_ANGLE,
        true, // rising
    );

    // Search for evening twilight starting from noon
    // (Sun setting through -12° in the evening)
    let noon = start_of_day.with_hour(12).unwrap();
    let evening_nautical = find_sun_altitude_time(
        noon,
        latitude,
        longitude,
        NAUTICAL_TWILIGHT_ANGLE,
        false, // setting
    );

    // Find civil twilight times
    let morning_civil = find_sun_altitude_time(
        start_of_day,
        latitude,
        longitude,
        CIVIL_TWILIGHT_ANGLE,
        true,
    );

    let evening_civil = find_sun_altitude_time(
        noon,
        latitude,
        longitude,
        CIVIL_TWILIGHT_ANGLE,
        false,
    );

    // Find sunrise and sunset times
    let sunrise = find_sun_altitude_time(
        start_of_day,
        latitude,
        longitude,
        SUNRISE_SUNSET_ANGLE,
        true,
    );

    let sunset = find_sun_altitude_time(
        noon,
        latitude,
        longitude,
        SUNRISE_SUNSET_ANGLE,
        false,
    );

    TwilightTimes {
        morning_nautical,
        evening_nautical,
        morning_civil,
        evening_civil,
        sunrise,
        sunset,
    }
}

/// Get visible stars at a given time and position suitable for sextant observation
///
/// Returns stars with altitude between MIN_SEXTANT_ALTITUDE and MAX_SEXTANT_ALTITUDE,
/// sorted by SHA in descending order (360° → 0°).
pub fn get_visible_stars(
    datetime: DateTime<Utc>,
    latitude: f64,
    longitude: f64,
) -> Vec<VisibleBody> {
    let catalog = get_star_catalog();
    let mut visible_stars = Vec::new();

    // Calculate observation year for proper motion correction
    let year = datetime.year() as f64;
    let day_of_year = datetime.ordinal() as f64;
    let days_in_year = if is_leap_year(datetime.year()) { 366.0 } else { 365.0 };
    let observation_year = year + (day_of_year - 1.0) / days_in_year;

    // Get GHA Aries for star positions
    let gha_aries = crate::almanac::gha_aries(datetime);

    for star_name in catalog.iter().map(|s| s.name) {
        // Get star with proper motion correction
        if let Some(star) = find_star_for_year(star_name, observation_year) {
            let gha = (gha_aries + star.sha) % 360.0;
            let dec = star.declination;
            let lha = (gha + longitude + 360.0) % 360.0;

            let sight_data = SightData {
                latitude,
                declination: dec,
                local_hour_angle: lha,
            };

            let altitude_geometric = compute_altitude(&sight_data);
            let azimuth = crate::sight_reduction::compute_azimuth(&sight_data);

            // Atmospheric refraction lifts stars above their geometric position.
            // apply_refraction_correction returns a negative value (used to reduce observed Ho
            // toward Hc). To convert geometric altitude → apparent (observed) altitude we
            // subtract it (double-negation = addition of the magnitude).
            let refraction = crate::sight_reduction::apply_refraction_correction(altitude_geometric);
            let altitude_apparent = altitude_geometric - refraction; // refraction < 0, so this adds it

            // Include only stars in good sextant range (using apparent altitude)
            if (MIN_SEXTANT_ALTITUDE..=MAX_SEXTANT_ALTITUDE).contains(&altitude_apparent) {
                visible_stars.push(VisibleBody {
                    name: star_name.to_string(),
                    altitude: altitude_apparent, // Store apparent altitude for display
                    azimuth,
                    gha,
                    sha: Some(star.sha),
                    declination: dec,
                    magnitude: Some(star.magnitude),
                    is_recommended: false, // Will be set later
                    is_second_best: false,
                });
            }
        }
    }

    // Sort by SHA (decreasing order - highest SHA first)
    visible_stars.sort_by(|a, b| {
        match (a.sha, b.sha) {
            (Some(sha_a), Some(sha_b)) => sha_b.partial_cmp(&sha_a).unwrap(), // Reversed for descending
            _ => std::cmp::Ordering::Equal,
        }
    });

    visible_stars
}

/// Get visible planets at a given time and position suitable for sextant observation
pub fn get_visible_planets(
    datetime: DateTime<Utc>,
    latitude: f64,
    longitude: f64,
) -> Vec<VisibleBody> {
    let planets = vec![Planet::Venus, Planet::Mars, Planet::Jupiter, Planet::Saturn];
    let mut visible_planets = Vec::new();

    for planet in planets {
        let gha = planet_gha(planet, datetime);
        let dec = planet_declination(planet, datetime);
        let lha = (gha + longitude + 360.0) % 360.0;

        let sight_data = SightData {
            latitude,
            declination: dec,
            local_hour_angle: lha,
        };

        let altitude_geometric = compute_altitude(&sight_data);
        let azimuth = crate::sight_reduction::compute_azimuth(&sight_data);

        // Atmospheric refraction lifts planets above their geometric position (same fix as stars).
        let refraction = crate::sight_reduction::apply_refraction_correction(altitude_geometric);
        let altitude_apparent = altitude_geometric - refraction; // refraction < 0, so this adds it

        // Include only planets in good sextant range (using apparent altitude)
        if (MIN_SEXTANT_ALTITUDE..=MAX_SEXTANT_ALTITUDE).contains(&altitude_apparent) {
            let name = match planet {
                Planet::Venus => "Venus",
                Planet::Mars => "Mars",
                Planet::Jupiter => "Jupiter",
                Planet::Saturn => "Saturn",
            };

            visible_planets.push(VisibleBody {
                name: name.to_string(),
                altitude: altitude_apparent, // Store apparent altitude for display
                azimuth,
                gha,
                sha: None, // Planets don't have fixed SHA
                declination: dec,
                magnitude: None, // Planets don't have fixed magnitudes
                is_recommended: false,
                is_second_best: false,
            });
        }
    }

    // Sort by altitude (highest first)
    visible_planets.sort_by(|a, b| b.altitude.partial_cmp(&a.altitude).unwrap());

    visible_planets
}

/// Select the best 3 stars for LOP crossings
///
/// Chooses stars that are:
/// 1. PRIORITY: Stars in optimal altitude range (15-65°)
/// 2. If not enough optimal stars, supplement with stars in extended range (65-75°)
/// 3. PRIORITY: Well-distributed in azimuth (ideally ~120° apart for good LOP geometry)
/// 4. Bright (low magnitude for easier observation) - secondary priority
/// 5. Excludes Polaris (used separately for latitude)
///
/// Azimuth distribution is weighted 6x higher than brightness for optimal fix geometry.
///
/// Returns indices of the best 3 stars
fn select_best_stars_for_lop(stars: &[VisibleBody], is_morning: bool, band_mode: bool) -> Vec<usize> {
    if stars.len() < 3 {
        return (0..stars.len()).collect();
    }

    // Separate stars into optimal and extended altitude ranges
    let mut optimal_stars: Vec<usize> = Vec::new();
    let mut extended_stars: Vec<usize> = Vec::new();

    for (idx, star) in stars.iter().enumerate() {
        if star.name == "Polaris" || star.magnitude.is_none() {
            continue;
        }

        if star.altitude >= OPTIMAL_MIN_ALTITUDE && star.altitude <= OPTIMAL_MAX_ALTITUDE {
            optimal_stars.push(idx);
        } else if star.altitude > OPTIMAL_MAX_ALTITUDE && star.altitude <= MAX_SEXTANT_ALTITUDE {
            extended_stars.push(idx);
        }
    }

    // Try to select 3 stars from optimal range first
    if optimal_stars.len() >= 3 {
        select_three_stars_from_pool(stars, &optimal_stars, is_morning, band_mode)
    } else if optimal_stars.is_empty() && extended_stars.len() >= 3 {
        select_three_stars_from_pool(stars, &extended_stars, is_morning, band_mode)
    } else {
        let mut selected = select_stars_from_pool(stars, &optimal_stars, 3, is_morning, band_mode);
        if selected.len() < 3 {
            let needed = 3 - selected.len();
            let mut extended_selected = select_stars_from_pool(stars, &extended_stars, needed, is_morning, band_mode);
            selected.append(&mut extended_selected);
        }
        selected
    }
}

/// Select up to N stars from a pool, optimizing for azimuth distribution and brightness
fn select_stars_from_pool(stars: &[VisibleBody], pool: &[usize], count: usize, is_morning: bool, band_mode: bool) -> Vec<usize> {
    if pool.is_empty() {
        return Vec::new();
    }
    if pool.len() <= count {
        return pool.to_vec();
    }

    select_three_stars_from_pool(stars, pool, is_morning, band_mode)
}

/// Evaluate the altitude quality for observation
/// Returns 0.0-1.0, with 1.0 being the "sweet spot" for sextant observation
fn evaluate_altitude_quality(altitude: f64) -> f64 {
    // Sweet spot is 40-50° - ideal mid-range for accurate sextant observation
    // Gradual falloff outside this range
    if (40.0..=50.0).contains(&altitude) {
        1.0 // Perfect mid-range
    } else if (35.0..40.0).contains(&altitude) {
        0.96 + (altitude - 35.0) / 5.0 * 0.04 // 35°=0.96, 40°=1.0
    } else if altitude > 50.0 && altitude <= 55.0 {
        0.96 + (55.0 - altitude) / 5.0 * 0.04 // 50°=1.0, 55°=0.96
    } else if (30.0..35.0).contains(&altitude) {
        0.90 + (altitude - 30.0) / 5.0 * 0.06 // 30°=0.90, 35°=0.96
    } else if altitude > 55.0 && altitude <= 58.0 {
        0.90 + (58.0 - altitude) / 3.0 * 0.06 // 55°=0.96, 58°=0.90
    } else if (25.0..30.0).contains(&altitude) {
        0.83 + (altitude - 25.0) / 5.0 * 0.07 // 25°=0.83, 30°=0.90
    } else if (20.0..25.0).contains(&altitude) {
        0.75 + (altitude - 20.0) / 5.0 * 0.08 // 20°=0.75, 25°=0.83
    } else if altitude > 58.0 && altitude <= 60.0 {
        0.80 - (altitude - 58.0) / 2.0 * 0.10 // 58°=0.90, 60°=0.70
    } else if (18.0..20.0).contains(&altitude) {
        0.68 + (altitude - 18.0) / 2.0 * 0.07 // 18°=0.68, 20°=0.75
    } else {
        0.50 // Poor
    }
}

/// Evaluate the geometry quality of a 3-star combination
fn evaluate_three_star_geometry(stars: &[VisibleBody], indices: &[usize; 3]) -> f64 {
    let mut az = [
        stars[indices[0]].azimuth,
        stars[indices[1]].azimuth,
        stars[indices[2]].azimuth,
    ];

    // Sort azimuths to get the three consecutive arcs around the full circle.
    // Using pairwise minimum arcs is wrong: 3 stars at 0°/10°/20° would score
    // ~0.89 (high) with minimum arcs but the 340° gap behind them makes it
    // a terrible triad. Consecutive arcs always sum to 360° and correctly
    // penalise clustered configurations.
    az.sort_by(|a, b| a.partial_cmp(b).unwrap());

    let gap1 = az[1] - az[0];
    let gap2 = az[2] - az[1];
    let gap3 = 360.0 - az[2] + az[0]; // wrap-around gap

    // Score each gap: ideal is 120°
    let score1 = 1.0 - ((gap1 - 120.0).abs() / 120.0).min(1.0);
    let score2 = 1.0 - ((gap2 - 120.0).abs() / 120.0).min(1.0);
    let score3 = 1.0 - ((gap3 - 120.0).abs() / 120.0).min(1.0);

    // Average geometry score (0.0 = worst, 1.0 = perfect 120° separation)
    (score1 + score2 + score3) / 3.0
}

/// Score how easily a star can be found and identified during twilight.
///
/// Returns [0.0, 1.0] where 1.0 = easiest to identify. Combines two independent factors:
///
/// 1. **Raw brightness** (base score): `(2.5 - magnitude) / 4.0` — a brighter star is visible
///    through thin cloud, haze, or before the eye fully dark-adapts.
/// 2. **Pattern / distinctiveness bonus**: stars in well-known asterisms or with unmistakable
///    appearance get an extra bonus because the navigator locates them by recognition, not by
///    counting faint stars.
///
/// Both factors matter in navigation: brightness helps in poor visibility; pattern recognition
/// reduces misidentification risk when time is short.
pub fn navigational_ease_score(name: &str, magnitude: f64) -> f64 {
    let base = (2.5 - magnitude) / 4.0;
    let bonus = if ASTERISM_BIG_DIPPER.contains(&name) {
        0.20 // find the Dipper shape; Dubhe/Alioth/Alkaid are immediately located
    } else if ASTERISM_SOUTHERN_CROSS.contains(&name) {
        0.15 // fixed beacon of the southern sky, unique shape
    } else if DISTINCTIVE_STARS.contains(&name) {
        0.10 // Rigel/Betelgeuse found via Orion's belt; Antares/Aldebaran confirmed by colour
    } else {
        0.0
    };
    // Extra proportional boost for negative-magnitude stars: they are so much
    // brighter than the rest of the sky that identification is essentially
    // instantaneous even in marginal conditions.  Sirius (−1.46) ≫ Procyon (0.38).
    let neg_mag_extra = (-magnitude).max(0.0) * 0.20;
    (base + bonus + neg_mag_extra).clamp(0.0, 1.5)
}

/// Altitude preference score: peaks at the midpoint of the optimal range (37.5°),
/// falls linearly to 0 at the edges (15° and 60°).
///
/// Stars near 15° suffer large and uncertain atmospheric refraction; stars near 60°
/// are difficult to bring down to the horizon with a sextant.  Middle altitudes
/// (≈30–45°) are easiest to sight accurately.
fn altitude_preference_score(altitude: f64) -> f64 {
    let mid        = (OPTIMAL_MIN_ALTITUDE + OPTIMAL_MAX_ALTITUDE) / 2.0; // 37.5°
    let half_range = (OPTIMAL_MAX_ALTITUDE - OPTIMAL_MIN_ALTITUDE) / 2.0; // 22.5°
    (1.0 - (altitude - mid).abs() / half_range).max(0.0)
}

/// Select the best 3 stars for a celestial fix, matching Pub 249 Vol. 1 rules:
///
/// 1. Filter to 57-star catalog stars (exclude non-standard extras), mag ≤ 2.5,
///    altitude in the practical observation range (OPTIMAL_MIN/MAX_ALTITUDE = 15°–60°).
/// 2. Exclude any combination where the minimum azimuth separation between any two
///    stars is less than MIN_AZIMUTH_SEPARATION (45°). This hard limit ensures LOPs
///    cross at useful angles, matching Pub 249 Vol. 1 practice.
/// 3. Two-pass geometry/brightness selection:
///    Pass 1: find the global best azimuth geometry score across all valid combinations.
///    Pass 2: among all combinations within GEOMETRY_THRESHOLD of the best geometry,
///    choose the one with the lowest average magnitude (brightest).
fn select_three_stars_from_pool(stars: &[VisibleBody], pool: &[usize], is_morning: bool, band_mode: bool) -> Vec<usize> {
    if pool.len() < 3 {
        return pool.to_vec();
    }

    if pool.len() == 3 {
        return pool.to_vec();
    }

    // Filter to Pub 249 navigational stars: 57-star catalog only, altitude in practical range,
    // magnitude bright enough to observe (faintest Pub 249 star is ~2.43).
    let acceptable_stars: Vec<usize> = pool.iter()
        .filter(|&&idx| {
            let mag_ok = stars[idx].magnitude.is_none_or(|mag| mag <= 2.5);
            let alt = stars[idx].altitude;
            let alt_ok = (OPTIMAL_MIN_ALTITUDE..=OPTIMAL_MAX_ALTITUDE).contains(&alt);
            let catalog_ok = !NON_STANDARD_STARS.contains(&stars[idx].name.as_str());
            mag_ok && alt_ok && catalog_ok
        })
        .copied()
        .collect();

    // Use acceptable stars if we have enough, otherwise fall back to full pool.
    // Sort brightest-first so that within equal geometry scores, brighter combinations
    // are enumerated first — ensures the brightness tiebreaker is deterministic.
    let mut filtered_pool = if acceptable_stars.len() >= 3 {
        acceptable_stars
    } else {
        pool.to_vec()
    };
    filtered_pool.sort_by(|&a, &b| {
        let mag_a = stars[a].magnitude.unwrap_or(3.0);
        let mag_b = stars[b].magnitude.unwrap_or(3.0);
        mag_a.partial_cmp(&mag_b).unwrap() // ascending: lower magnitude = brighter
    });

    // Helper: check that all three stars satisfy the minimum azimuth separation.
    // The minimum azimuth gap (sorted consecutive arcs) must be >= MIN_AZIMUTH_SEPARATION.
    // This matches Pub 249 Vol. 1 which rejects combinations where two stars are too
    // close in azimuth (LOPs would be nearly parallel).
    let az_sep_ok = |i: usize, j: usize, k: usize| -> bool {
        let mut az = [
            stars[filtered_pool[i]].azimuth,
            stars[filtered_pool[j]].azimuth,
            stars[filtered_pool[k]].azimuth,
        ];
        az.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let gap1 = az[1] - az[0];
        let gap2 = az[2] - az[1];
        let gap3 = 360.0 - az[2] + az[0];
        gap1 >= MIN_AZIMUTH_SEPARATION
            && gap2 >= MIN_AZIMUTH_SEPARATION
            && gap3 >= MIN_AZIMUTH_SEPARATION
    };

    // Two-pass selection matching Pub 249 Vol. 1 rules:
    // Pass 1: find the best geometry score across all valid combinations.
    // Pass 2: among all combinations within GEOMETRY_THRESHOLD of that best,
    //         choose the one with the lowest average magnitude (brightest stars).
    //
    // Two-pass is required for correctness — a single-pass greedy comparator is
    // order-dependent (the running "current best" changes the tiebreaker outcome),
    // so it can produce different results depending on evaluation order.
    let mut global_best_geometry = f64::MIN;

    for i in 0..filtered_pool.len() {
        for j in (i + 1)..filtered_pool.len() {
            for k in (j + 1)..filtered_pool.len() {
                if !az_sep_ok(i, j, k) { continue; }
                let combination = [filtered_pool[i], filtered_pool[j], filtered_pool[k]];
                let geometry_score = evaluate_three_star_geometry(stars, &combination);
                if geometry_score > global_best_geometry {
                    global_best_geometry = geometry_score;
                }
            }
        }
    }

    let geometry_threshold = if band_mode {
        GEOMETRY_THRESHOLD_BAND
    } else if is_morning {
        GEOMETRY_THRESHOLD_MORNING
    } else {
        GEOMETRY_THRESHOLD_EVENING
    };
    let geometry_cutoff = global_best_geometry - geometry_threshold;
    let mut best_combination = None;
    let mut best_tiebreak = f64::MIN;

    for i in 0..filtered_pool.len() {
        for j in (i + 1)..filtered_pool.len() {
            for k in (j + 1)..filtered_pool.len() {
                if !az_sep_ok(i, j, k) { continue; }
                let combination = [filtered_pool[i], filtered_pool[j], filtered_pool[k]];
                let geometry_score = evaluate_three_star_geometry(stars, &combination);
                if geometry_score < geometry_cutoff {
                    continue;
                }
                let mag1 = stars[combination[0]].magnitude.unwrap_or(2.5);
                let mag2 = stars[combination[1]].magnitude.unwrap_or(2.5);
                let mag3 = stars[combination[2]].magnitude.unwrap_or(2.5);
                // brightness_score: raw magnitude advantage (cloud penetration, haze).
                let brightness_score = -(mag1 + mag2 + mag3); // higher = brighter (lower sum)
                // ease_score: navigational ease including asterism/colour recognition bonuses.
                // Both factors are kept: brightness helps in poor visibility; ease reduces
                // misidentification risk when time is short.
                let ease_score =
                    navigational_ease_score(&stars[combination[0]].name, mag1) +
                    navigational_ease_score(&stars[combination[1]].name, mag2) +
                    navigational_ease_score(&stars[combination[2]].name, mag3);
                // altitude_score: prefer stars in the 30-45° sweet spot where
                // refraction is small and sighting is straightforward.
                let altitude_score =
                    altitude_preference_score(stars[combination[0]].altitude) +
                    altitude_preference_score(stars[combination[1]].altitude) +
                    altitude_preference_score(stars[combination[2]].altitude);

                // For morning: prefer western-sky stars (longer visibility after dawn),
                // for evening: prefer eastern-sky stars (earlier to appear/longer visible).
                // period_score averages max(0, ±sin(az)) over the 3 stars; peaks at 270° (W)
                // for morning, 90° (E) for evening.
                let period_score = if is_morning {
                    combination.iter().map(|&ci| {
                        let az_rad = stars[ci].azimuth.to_radians();
                        (-az_rad.sin()).max(0.0)
                    }).sum::<f64>() / 3.0
                } else {
                    combination.iter().map(|&ci| {
                        let az_rad = stars[ci].azimuth.to_radians();
                        az_rad.sin().max(0.0)
                    }).sum::<f64>() / 3.0
                };

                let tiebreak = if band_mode {
                    // Vol.1 band mode: brightness + ease but NO period_score.
                    // Vol.1 has no morning/evening sky preference — same stars at a
                    // given LHA regardless of time of day.
                    ease_score + brightness_score + 0.3 * altitude_score
                } else if is_morning {
                    period_score * 10.0 + ease_score + brightness_score + 0.3 * altitude_score
                } else {
                    ease_score + brightness_score + 0.3 * altitude_score
                };

                if tiebreak > best_tiebreak {
                    best_tiebreak = tiebreak;
                    best_combination = Some(combination);
                }
            }
        }
    }

    best_combination.map(|c| c.to_vec()).unwrap_or_else(|| pool.to_vec())
}

/// Select up to 4 backup stars that best complement the already-chosen best 3.
///
/// Greedy selection: each new star is scored against ALL currently selected stars
/// (best 3 + any already-chosen backups) to ensure diverse azimuth coverage.
///
/// Scoring for each candidate: maximum geometry score achievable with any 2 stars
/// from the current selected set, plus a navigational ease bonus.  Only combinations
/// that pass the MIN_AZIMUTH_SEPARATION check are considered.  Stars that cannot form
/// a valid triple with any pair score zero and are skipped.
///
/// Returns indices into the same `stars` slice as `best_indices`.
fn select_second_best_stars(stars: &[VisibleBody], best_indices: &[usize]) -> Vec<usize> {
    // Pool: acceptable stars not already in the best set
    let mut available: Vec<usize> = (0..stars.len())
        .filter(|&idx| {
            let mag_ok = stars[idx].magnitude.is_some_and(|m| m <= 2.5);
            let alt_ok = stars[idx].altitude >= OPTIMAL_MIN_ALTITUDE
                && stars[idx].altitude <= OPTIMAL_MAX_ALTITUDE;
            let catalog_ok = !NON_STANDARD_STARS.contains(&stars[idx].name.as_str());
            let not_best = !best_indices.contains(&idx);
            mag_ok && alt_ok && catalog_ok && not_best
        })
        .collect();

    let mut selected = best_indices.to_vec(); // grows as we pick each backup
    let mut second_best: Vec<usize> = Vec::new();

    for _ in 0..4 {
        if available.is_empty() {
            break;
        }

        // Score every remaining candidate
        let best_candidate = available.iter().enumerate()
            .filter_map(|(ai, &cidx)| {
                let ease = navigational_ease_score(
                    &stars[cidx].name,
                    stars[cidx].magnitude.unwrap_or(2.5),
                );

                // Find the best geometry this star achieves with any pair from selected
                let mut max_geo = 0.0_f64;
                let mut valid = false;
                for i in 0..selected.len() {
                    for j in (i + 1)..selected.len() {
                        let mut az = [
                            stars[selected[i]].azimuth,
                            stars[selected[j]].azimuth,
                            stars[cidx].azimuth,
                        ];
                        az.sort_by(|a, b| a.partial_cmp(b).unwrap());
                        let g1 = az[1] - az[0];
                        let g2 = az[2] - az[1];
                        let g3 = 360.0 - az[2] + az[0];
                        if g1 >= MIN_AZIMUTH_SEPARATION
                            && g2 >= MIN_AZIMUTH_SEPARATION
                            && g3 >= MIN_AZIMUTH_SEPARATION
                        {
                            let combo = [selected[i], selected[j], cidx];
                            let geo = evaluate_three_star_geometry(stars, &combo);
                            if geo > max_geo {
                                max_geo = geo;
                                valid = true;
                            }
                        }
                    }
                }

                if valid {
                    // Second-best stars are backup alternatives: weight ease/brightness
                    // more heavily than geometry.  A very bright star like Arcturus or
                    // Altair is more useful as a backup than a geometrically ideal dim
                    // star, because you need to *find* it quickly through thin cloud.
                    let alt_pref = altitude_preference_score(stars[cidx].altitude);
                    Some((ai, max_geo * 0.4 + ease + 0.3 * alt_pref))
                } else {
                    None
                }
            })
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

        match best_candidate {
            Some((ai, _)) => {
                let idx = available.remove(ai);
                selected.push(idx);
                second_best.push(idx);
            }
            None => break, // no valid candidates remain
        }
    }

    second_best
}

/// OLD SCORING ALGORITHM - kept for reference but not used
#[allow(dead_code)]
fn select_three_stars_scoring_algorithm(stars: &[VisibleBody], pool: &[usize]) -> Vec<usize> {
    if pool.len() < 3 {
        return pool.to_vec();
    }

    if pool.len() == 3 {
        return pool.to_vec();
    }

    // Try all possible combinations of 3 stars and pick the best
    let mut best_combination = None;
    let mut best_score = f64::MIN;

    for i in 0..pool.len() {
        for j in (i + 1)..pool.len() {
            for k in (j + 1)..pool.len() {
                let combination = [pool[i], pool[j], pool[k]];

                // Geometry score (0.0 to 1.0, higher is better)
                let geometry_score = evaluate_three_star_geometry(stars, &combination);

                // Altitude quality score - average quality of observation altitudes
                let alt_quality1 = evaluate_altitude_quality(stars[combination[0]].altitude);
                let alt_quality2 = evaluate_altitude_quality(stars[combination[1]].altitude);
                let alt_quality3 = evaluate_altitude_quality(stars[combination[2]].altitude);
                let avg_altitude_quality = (alt_quality1 + alt_quality2 + alt_quality3) / 3.0;

                // Brightness score - average brightness of the 3 stars
                let mag1 = stars[combination[0]].magnitude.unwrap_or(3.0);
                let mag2 = stars[combination[1]].magnitude.unwrap_or(3.0);
                let mag3 = stars[combination[2]].magnitude.unwrap_or(3.0);
                let avg_magnitude = (mag1 + mag2 + mag3) / 3.0;
                let brightness_score = (3.0 - avg_magnitude).max(0.0) / 3.0; // Normalize to ~0-1

                // Combined score: geometry (18x) + brightness (4x) + altitude quality (3x)
                // Optimize for LOP geometry while favoring bright, well-positioned stars
                let total_score = geometry_score * 18.0 + brightness_score * 4.0 + avg_altitude_quality * 3.0;

                if total_score > best_score {
                    best_score = total_score;
                    best_combination = Some(combination);
                }
            }
        }
    }

    best_combination.map(|c| c.to_vec()).unwrap_or_else(|| pool.to_vec())
}

/// OLD ALGORITHM - kept for reference but not used
#[allow(dead_code)]
fn select_three_stars_sequential(stars: &[VisibleBody], pool: &[usize]) -> Vec<usize> {
    if pool.len() < 3 {
        return pool.to_vec();
    }

    // Find brightest star in pool as first selection
    let mut first_idx = None;
    let mut best_brightness = f64::MAX; // Lower magnitude = brighter

    for &idx in pool {
        let magnitude = stars[idx].magnitude.unwrap_or(3.0);
        if magnitude < best_brightness {
            best_brightness = magnitude;
            first_idx = Some(idx);
        }
    }

    let first_idx = match first_idx {
        Some(idx) => idx,
        None => return Vec::new(),
    };

    let mut selected = vec![first_idx];

    // Find second star - bright and well-separated in azimuth from first
    let first_azimuth = stars[first_idx].azimuth;
    let mut best_second = None;
    let mut best_second_score = f64::MIN;

    for &idx in pool {
        if idx == first_idx {
            continue;
        }

        let azimuth_diff = (stars[idx].azimuth - first_azimuth).abs();
        let azimuth_diff = azimuth_diff.min(360.0 - azimuth_diff);

        // Azimuth score - prefer 90-150° separation, ideal ~120°
        let azimuth_score = if (90.0..=150.0).contains(&azimuth_diff) {
            azimuth_diff / 120.0
        } else if azimuth_diff < 90.0 {
            azimuth_diff / 90.0 * 0.5
        } else {
            (180.0 - azimuth_diff) / 30.0 * 0.7
        };

        // Brightness score (lower magnitude = brighter = higher score)
        let magnitude = stars[idx].magnitude.unwrap_or(3.0);
        let brightness_score = (3.0 - magnitude).max(0.0);

        // Weighted: PRIORITIZE azimuth distribution (3.0x) over brightness (0.5x)
        let total_score = azimuth_score * 3.0 + brightness_score * 0.5;

        if total_score > best_second_score {
            best_second_score = total_score;
            best_second = Some(idx);
        }
    }

    if let Some(second_idx) = best_second {
        selected.push(second_idx);

        // Find third star - bright and well-separated from both first and second
        let second_azimuth = stars[second_idx].azimuth;
        let mut best_third = None;
        let mut best_third_score = f64::MIN;

        for &idx in pool {
            if selected.contains(&idx) {
                continue;
            }

            let diff1 = (stars[idx].azimuth - first_azimuth).abs();
            let diff1 = diff1.min(360.0 - diff1);

            let diff2 = (stars[idx].azimuth - second_azimuth).abs();
            let diff2 = diff2.min(360.0 - diff2);

            // Azimuth score - should be ~120° from both
            let azimuth_score = if diff1 >= 90.0 && diff2 >= 90.0 {
                ((diff1 / 120.0) + (diff2 / 120.0)) / 2.0
            } else {
                diff1.min(diff2) / 90.0 * 0.3
            };

            // Brightness score
            let magnitude = stars[idx].magnitude.unwrap_or(3.0);
            let brightness_score = (3.0 - magnitude).max(0.0);

            // Weighted: PRIORITIZE azimuth distribution (3.0x) over brightness (0.5x)
            let total_score = azimuth_score * 3.0 + brightness_score * 0.5;

            if total_score > best_third_score {
                best_third_score = total_score;
                best_third = Some(idx);
            }
        }

        if let Some(third_idx) = best_third {
            selected.push(third_idx);
        }
    }

    selected
}

/// Get all visible bodies during an observation interval
///
/// Returns bodies sorted by SHA, considering visibility throughout the interval.
/// The best 3 stars for LOP crossings are marked with is_recommended = true.
///
/// Stars must be visible for the entire interval (or at least partially as fallback).
///
/// When `band_mode` is true the selection mimics Pub 249 Vol. 1: the midpoint
/// LHA is quantised to the nearest 15° multiple and star positions are evaluated
/// at the single UTC moment corresponding to that fixed LHA.  This gives the
/// same stable star set for the whole 15°-wide band, just like the printed table.
pub fn get_all_visible_bodies_interval(
    start_time: DateTime<Utc>,
    end_time: DateTime<Utc>,
    latitude: f64,
    longitude: f64,
    is_morning: bool,
    band_mode: bool,
) -> Vec<VisibleBody> {
    // Sample times across the interval (start, middle, end)
    let mid_time = start_time + Duration::minutes((end_time - start_time).num_minutes() / 2);
    let sample_times = vec![start_time, mid_time, end_time];

    // Get bodies at each sample time
    let mut body_visibility: std::collections::HashMap<String, Vec<VisibleBody>> = std::collections::HashMap::new();

    for time in &sample_times {
        let bodies = get_all_visible_bodies_single(*time, latitude, longitude);
        for body in bodies {
            body_visibility.entry(body.name.clone())
                .or_default()
                .push(body);
        }
    }

    // Collect in sorted key order so downstream processing is deterministic
    // (HashMap iteration order is arbitrary in Rust).
    let mut sorted_keys: Vec<String> = body_visibility.keys().cloned().collect();
    sorted_keys.sort();

    let mut all_bodies: Vec<VisibleBody> = Vec::new();

    for name in &sorted_keys {
        let observations = &body_visibility[name];
        if observations.len() == sample_times.len() {
            // Visible throughout - use middle observation for display
            all_bodies.push(observations[1].clone());
        } else if observations.len() >= 2 {
            // Visible for 2 of 3 sample times (rising or setting star) — usable for
            // most of the window.  Use the observation closest to midpoint for display.
            all_bodies.push(observations[observations.len() / 2].clone());
        }
        // Stars visible at only 1 sample are excluded — too brief to be reliable.
    }

    // Select best 3 stars from available bodies
    let stars: Vec<&VisibleBody> = all_bodies.iter()
        .filter(|b| b.sha.is_some())
        .collect();

    let star_indices: Vec<usize> = (0..all_bodies.len())
        .filter(|&i| all_bodies[i].sha.is_some())
        .collect();

    // For selection, enforce altitude bounds:
    //   upper bound: use max altitude — excludes stars that climb above 60° at any point
    //   lower bound: use the displayed (midpoint) altitude — a star visible at midpoint
    //     is selectable even if it rises/sets before or after one sample point.
    //     Using min altitude was too strict and excluded stars like Antares that are
    //     valid for most of the window but set just below 15° at the far end.
    // Display altitude (in all_bodies) stays at midpoint; only the selection copy changes.
    let stars_vec: Vec<VisibleBody> = stars.iter().map(|&s| {
        let mut sel = s.clone();
        if let Some(obs) = body_visibility.get(&s.name) {
            let max_alt = obs.iter().map(|o| o.altitude).fold(f64::NEG_INFINITY, f64::max);
            // If the star ever exceeds the ceiling use max_alt (upper-bound rejection).
            // Otherwise leave sel.altitude as the midpoint value (lower-bound check).
            if max_alt > OPTIMAL_MAX_ALTITUDE {
                sel.altitude = max_alt;
            }
        }
        sel
    }).collect();
    let best_star_indices = select_best_stars_for_lop(&stars_vec, is_morning, band_mode);

    for &idx in &best_star_indices {
        if idx < star_indices.len() {
            let actual_idx = star_indices[idx];
            if actual_idx < all_bodies.len() {
                all_bodies[actual_idx].is_recommended = true;
            }
        }
    }

    let second_best_indices = select_second_best_stars(&stars_vec, &best_star_indices);
    for &idx in &second_best_indices {
        if idx < star_indices.len() {
            let actual_idx = star_indices[idx];
            if actual_idx < all_bodies.len() {
                all_bodies[actual_idx].is_second_best = true;
            }
        }
    }

    // Sort by SHA (descending)
    all_bodies.sort_by(|a, b| {
        match (a.sha, b.sha) {
            (Some(sha_a), Some(sha_b)) => sha_b.partial_cmp(&sha_a).unwrap(),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => a.name.cmp(&b.name),
        }
    });

    all_bodies
}

/// Get all visible bodies at a single time point (helper function)
fn get_all_visible_bodies_single(
    datetime: DateTime<Utc>,
    latitude: f64,
    longitude: f64,
) -> Vec<VisibleBody> {
    let stars = get_visible_stars(datetime, latitude, longitude);
    let planets = get_visible_planets(datetime, latitude, longitude);

    let mut all_bodies = stars;
    all_bodies.extend(planets);

    all_bodies
}

/// Get all visible bodies (stars and planets) suitable for observation
///
/// Returns bodies sorted by SHA (descending: 360° → 0°), with planets at the end.
/// The best 3 stars for LOP crossings are marked with is_recommended = true.
pub fn get_all_visible_bodies(
    datetime: DateTime<Utc>,
    latitude: f64,
    longitude: f64,
    is_morning: bool,
    band_mode: bool,
) -> Vec<VisibleBody> {
    // Get stars already sorted by SHA descending
    let mut stars = get_visible_stars(datetime, latitude, longitude);
    let planets = get_visible_planets(datetime, latitude, longitude);

    // Select best 3 stars for LOP crossings from the SHA-sorted list
    let best_star_indices = select_best_stars_for_lop(&stars, is_morning, band_mode);
    for &idx in &best_star_indices {
        if idx < stars.len() {
            stars[idx].is_recommended = true;
        }
    }

    // Select 4 second-best backup stars that complement the best 3
    let second_best_indices = select_second_best_stars(&stars, &best_star_indices);
    for &idx in &second_best_indices {
        if idx < stars.len() {
            stars[idx].is_second_best = true;
        }
    }

    // Combine stars and planets
    let mut all_bodies = stars;
    all_bodies.extend(planets);

    // Final sort by SHA (descending) to maintain consistent order
    // Stars are already SHA-sorted, this ensures planets appear after all stars
    all_bodies.sort_by(|a, b| {
        match (a.sha, b.sha) {
            (Some(sha_a), Some(sha_b)) => sha_b.partial_cmp(&sha_a).unwrap(), // Descending SHA
            (Some(_), None) => std::cmp::Ordering::Less, // Stars before planets
            (None, Some(_)) => std::cmp::Ordering::Greater, // Planets after stars
            (None, None) => a.name.cmp(&b.name), // Planets sorted by name
        }
    });

    all_bodies
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{NaiveDate, TimeZone};

    fn make_datetime(date: &str, hour: u32, minute: u32) -> DateTime<Utc> {
        let date = NaiveDate::parse_from_str(date, "%Y-%m-%d").unwrap();
        Utc.from_utc_datetime(&date.and_hms_opt(hour, minute, 0).unwrap())
    }

    #[test]
    fn test_twilight_calculation() {
        let date = make_datetime("2024-03-15", 0, 0);
        let latitude = 40.0; // 40°N
        let longitude = -74.0; // 74°W (New York area)

        let twilight = calculate_twilight_times(date, latitude, longitude);

        println!("Morning nautical: {:?}", twilight.morning_nautical);
        println!("Evening nautical: {:?}", twilight.evening_nautical);

        // Should have both morning and evening twilight
        assert!(twilight.morning_nautical.is_some(), "Should have morning nautical twilight");
        assert!(twilight.evening_nautical.is_some(), "Should have evening nautical twilight");

        // Morning should be before evening
        if let (Some(morning), Some(evening)) = (twilight.morning_nautical, twilight.evening_nautical) {
            println!("Morning: {}", morning);
            println!("Evening: {}", evening);
            assert!(morning < evening, "Morning twilight should be before evening: morning={}, evening={}", morning, evening);
        }
    }

    #[test]
    fn test_visible_stars() {
        let datetime = make_datetime("2024-03-15", 20, 0); // 8 PM UTC
        let latitude = 40.0;
        let longitude = -74.0;

        let stars = get_visible_stars(datetime, latitude, longitude);

        // Should have some visible stars
        assert!(!stars.is_empty(), "Should have visible stars");

        // Verify stars are sorted by SHA in descending order
        for i in 0..stars.len().saturating_sub(1) {
            if let (Some(sha1), Some(sha2)) = (stars[i].sha, stars[i + 1].sha) {
                assert!(sha1 >= sha2,
                    "Stars should be sorted by SHA descending: {} (SHA {}) should come before {} (SHA {})",
                    stars[i].name, sha1, stars[i + 1].name, sha2);
            }
        }

        // All should be in sextant range
        for star in &stars {
            assert!(star.altitude >= MIN_SEXTANT_ALTITUDE);
            assert!(star.altitude <= MAX_SEXTANT_ALTITUDE);
        }
    }

    #[test]
    fn test_visible_planets() {
        let datetime = make_datetime("2024-03-15", 20, 0);
        let latitude = 40.0;
        let longitude = -74.0;

        let planets = get_visible_planets(datetime, latitude, longitude);

        // All should be in sextant range
        for planet in &planets {
            assert!(planet.altitude >= MIN_SEXTANT_ALTITUDE);
            assert!(planet.altitude <= MAX_SEXTANT_ALTITUDE);
        }
    }

    #[test]
    fn test_2026_03_29_morning_twilight() {
        // Test case from user: 2026-03-29 morning civil twilight
        // Position: 43°12.53'N, 27°56.36'E
        let latitude = 43.2088;  // 43°12.53'N
        let longitude = 27.9393; // 27°56.36'E
        let date = make_datetime("2026-03-29", 0, 0);

        // Calculate twilight times
        let twilight = calculate_twilight_times(date, latitude, longitude);

        // Get morning civil twilight time
        if let Some(civil_time) = twilight.morning_civil {
            println!("\nMorning Civil Twilight: {}", civil_time);

            // Get all visible bodies
            let bodies = get_all_visible_bodies(civil_time, latitude, longitude, true, false);

            println!("\nAll visible stars in sextant range (15-75°):");
            println!("{:<15} {:>8} {:>8} {:>10} {:>10} {:>5}", "Name", "Altitude", "Azimuth", "SHA", "Mag", "Rec");

            for body in &bodies {
                if body.sha.is_some() { // Only stars
                    let alt_deg = body.altitude as i32;
                    let alt_min = ((body.altitude - alt_deg as f64) * 60.0) as i32;
                    let az_deg = body.azimuth as i32;

                    let sha_str = if let Some(sha) = body.sha {
                        format!("{:.1}°", sha)
                    } else {
                        "-".to_string()
                    };

                    let mag_str = if let Some(mag) = body.magnitude {
                        format!("{:.1}", mag)
                    } else {
                        "-".to_string()
                    };

                    let rec = if body.is_recommended { "★" } else { "" };

                    println!("{:<15} {:>3}°{:02}' {:>6}° {:>10} {:>10} {:>5}",
                        body.name, alt_deg, alt_min, az_deg, sha_str, mag_str, rec);
                }
            }

            println!("\nRecommended stars for LOP:");
            for body in &bodies {
                if body.is_recommended {
                    println!("  ★ {} (Az: {:.0}°, Alt: {:.1}°, Mag: {:.1})",
                        body.name, body.azimuth, body.altitude, body.magnitude.unwrap_or(0.0));
                }
            }

            // Check expected combination: Deneb, Antares, Alkaid
            println!("\nExpected combination (Deneb, Antares, Alkaid):");
            for name in &["Deneb", "Antares", "Alkaid"] {
                if let Some(star) = bodies.iter().find(|b| &b.name == name) {
                    println!("  {} - Az: {:.1}°, Alt: {:.2}°, Mag: {:.1}",
                        star.name, star.azimuth, star.altitude, star.magnitude.unwrap_or(0.0));
                }
            }

            // Calculate geometry for expected combination
            if let (Some(deneb), Some(antares), Some(alkaid)) = (
                bodies.iter().find(|b| b.name == "Deneb"),
                bodies.iter().find(|b| b.name == "Antares"),
                bodies.iter().find(|b| b.name == "Alkaid"),
            ) {
                let sep_da = (antares.azimuth - deneb.azimuth).abs().min(360.0 - (antares.azimuth - deneb.azimuth).abs());
                let sep_aa = (alkaid.azimuth - antares.azimuth).abs().min(360.0 - (alkaid.azimuth - antares.azimuth).abs());
                let sep_ad = (deneb.azimuth - alkaid.azimuth).abs().min(360.0 - (deneb.azimuth - alkaid.azimuth).abs());

                println!("\nExpected geometry:");
                println!("  Deneb-Antares: {:.1}°", sep_da);
                println!("  Antares-Alkaid: {:.1}°", sep_aa);
                println!("  Alkaid-Deneb: {:.1}°", sep_ad);
                println!("  Average deviation from 120°: {:.1}°",
                    ((sep_da - 120.0).abs() + (sep_aa - 120.0).abs() + (sep_ad - 120.0).abs()) / 3.0);
            }

            // Also show actual selected combination geometry
            println!("\nActual selected combination:");
            let selected: Vec<&VisibleBody> = bodies.iter().filter(|b| b.is_recommended).collect();
            if selected.len() == 3 {
                let sep_12 = (selected[1].azimuth - selected[0].azimuth).abs().min(360.0 - (selected[1].azimuth - selected[0].azimuth).abs());
                let sep_23 = (selected[2].azimuth - selected[1].azimuth).abs().min(360.0 - (selected[2].azimuth - selected[1].azimuth).abs());
                let sep_31 = (selected[0].azimuth - selected[2].azimuth).abs().min(360.0 - (selected[0].azimuth - selected[2].azimuth).abs());

                println!("  {}-{}: {:.1}°", selected[0].name, selected[1].name, sep_12);
                println!("  {}-{}: {:.1}°", selected[1].name, selected[2].name, sep_23);
                println!("  {}-{}: {:.1}°", selected[2].name, selected[0].name, sep_31);
                println!("  Average deviation from 120°: {:.1}°",
                    ((sep_12 - 120.0).abs() + (sep_23 - 120.0).abs() + (sep_31 - 120.0).abs()) / 3.0);
            }
        }
    }

    #[test]
    fn test_2026_03_29_evening_twilight() {
        // Test case: 2026-03-29 evening civil twilight
        // Position: 43°12.53'N, 27°56.36'E
        let latitude = 43.2088;  // 43°12.53'N
        let longitude = 27.9393; // 27°56.36'E
        let date = make_datetime("2026-03-29", 0, 0);

        // Calculate twilight times
        let twilight = calculate_twilight_times(date, latitude, longitude);

        // Get evening civil twilight time
        if let Some(civil_time) = twilight.evening_civil {
            println!("\nEvening Civil Twilight: {}", civil_time);

            // Get all visible bodies
            let bodies = get_all_visible_bodies(civil_time, latitude, longitude, false, false);

            println!("\nAll visible stars in sextant range (15-75°):");
            println!("{:<15} {:>8} {:>8} {:>10} {:>10} {:>5}", "Name", "Altitude", "Azimuth", "SHA", "Mag", "Rec");

            for body in &bodies {
                if body.sha.is_some() { // Only stars
                    let alt_deg = body.altitude as i32;
                    let alt_min = ((body.altitude - alt_deg as f64) * 60.0) as i32;
                    let az_deg = body.azimuth as i32;

                    let sha_str = if let Some(sha) = body.sha {
                        format!("{:.1}°", sha)
                    } else {
                        "-".to_string()
                    };

                    let mag_str = if let Some(mag) = body.magnitude {
                        format!("{:.1}", mag)
                    } else {
                        "-".to_string()
                    };

                    let rec = if body.is_recommended { "★" } else { "" };

                    println!("{:<15} {:>3}°{:02}' {:>6}° {:>10} {:>10} {:>5}",
                        body.name, alt_deg, alt_min, az_deg, sha_str, mag_str, rec);
                }
            }

            println!("\nRecommended stars for LOP:");
            for body in &bodies {
                if body.is_recommended {
                    println!("  ★ {} (Az: {:.0}°, Alt: {:.1}°, Mag: {:.1})",
                        body.name, body.azimuth, body.altitude, body.magnitude.unwrap_or(0.0));
                }
            }

            // Check if Sirius is visible and recommended
            let sirius = bodies.iter().find(|b| b.name == "Sirius");
            if let Some(star) = sirius {
                println!("\nSirius status:");
                println!("  Visible: YES");
                println!("  Az: {:.1}°, Alt: {:.1}°, Mag: {:.1}",
                    star.azimuth, star.altitude, star.magnitude.unwrap_or(0.0));
                println!("  Recommended: {}", if star.is_recommended { "YES ✓" } else { "NO - should be preferred as brightest star!" });
            } else {
                println!("\nSirius: Not visible in sextant range");
            }
        }
    }
}
