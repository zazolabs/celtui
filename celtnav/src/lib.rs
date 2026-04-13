//! Celestial Navigation Library
//!
//! This library provides functions for celestial navigation calculations,
//! including time conversions, sight reductions, and almanac data management.

pub mod time_conversion;
pub mod coords;
pub mod dms;
pub mod sight_reduction;
pub mod almanac;
pub mod fix_calculation;
pub mod sight_averaging;
pub mod twilight;

pub use time_conversion::{gha_from_utc, lha_from_gha, gst_from_utc};
pub use coords::{equatorial_to_horizontal, horizontal_to_equatorial, EquatorialCoords, HorizontalCoords};
pub use sight_reduction::{
    compute_altitude, compute_azimuth, compute_intercept,
    apply_refraction_correction, apply_dip_correction,
    apply_semidiameter_correction, apply_parallax_correction,
    SightData, AltitudeCorrections,
};
pub use almanac::{
    sun_gha, sun_declination, moon_gha, moon_declination,
    gha_aries, star_gha, star_declination, find_star, get_star_catalog, Star,
    planet_gha, planet_declination, Planet,
    CelestialBody, BodyPosition, get_body_position,
};
pub use fix_calculation::{
    LineOfPosition, Position, Fix,
    fix_from_two_lops, fix_from_multiple_lops,
    advance_position, advance_lop,
};
pub use sight_averaging::{
    SextantObservation, AveragedSight,
    average_sights, validate_altitude,
};
pub use dms::{decimal_to_dms, dms_to_decimal, dm_to_decimal, DMS};
pub use twilight::{
    calculate_twilight_times, get_visible_stars, get_visible_planets,
    get_all_visible_bodies, get_all_visible_bodies_interval,
    TwilightTimes, VisibleBody,
    NAUTICAL_TWILIGHT_ANGLE, CIVIL_TWILIGHT_ANGLE,
    MIN_SEXTANT_ALTITUDE, MAX_SEXTANT_ALTITUDE,
    OPTIMAL_MIN_ALTITUDE, OPTIMAL_MAX_ALTITUDE,
};
