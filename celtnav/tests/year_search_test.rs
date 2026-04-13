// Search for which year gives GHA Aries = 86° 03.3' (86.055°)
// for September 10 at 06:28:18 UTC

use chrono::{NaiveDate, NaiveTime, TimeZone, Utc};
use celtnav::almanac::{gha_aries, star_gha};

#[test]
fn search_year_for_correct_gha_aries() {
    println!("\nSearching for year with GHA Aries = 86° 03.3' (86.055°)");
    println!("Date: September 10, Time: 06:28:18 UTC\n");
    println!("{:<6} | {:<12} | {:<18} | {:<15}", "Year", "GHA Aries", "Diff from target", "GHA Pollux");
    println!("{:-<6}-+-{:-<12}-+-{:-<18}-+-{:-<15}", "", "", "", "");

    let target_gha_aries = 86.055;
    let sha_pollux = 243.435; // SHA Pollux = 243° 26.1'

    let mut best_year = 0;
    let mut best_diff = f64::MAX;

    for year in 2000..=2025 {
        let date = NaiveDate::from_ymd_opt(year, 9, 10).unwrap();
        let time = NaiveTime::from_hms_opt(6, 28, 18).unwrap();
        let datetime = Utc.from_utc_datetime(&date.and_time(time));

        let gha = gha_aries(datetime);
        let gha_pollux = star_gha("Pollux", datetime).unwrap();

        let diff = (gha - target_gha_aries).abs();

        // Format to degrees and minutes
        let gha_deg = gha.floor() as i32;
        let gha_min = (gha - gha_deg as f64) * 60.0;

        let marker = if diff < 0.1 { " *** MATCH ***" } else { "" };

        println!("{:<6} | {:03}° {:05.2}' | {:+7.3}° ({:4.1}') | {:06.3}°{}",
            year, gha_deg, gha_min, gha - target_gha_aries, diff * 60.0, gha_pollux, marker);

        if diff < best_diff {
            best_diff = diff;
            best_year = year;
        }
    }

    println!("\nBest match: Year {} with difference of {:.3}° ({:.1} arcminutes)",
        best_year, best_diff, best_diff * 60.0);
}
