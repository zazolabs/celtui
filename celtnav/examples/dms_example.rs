// SPDX-License-Identifier: GPL-2.0-only
// SPDX-FileCopyrightText: Alexander Atanasov <alex@zazolabs.com>
//! Example demonstrating DMS (Degrees, Decimal Minutes) conversion
//!
//! This example shows how to use the DMS conversion functions
//! for celestial navigation coordinate precision using the modern
//! decimal minutes format.

use celtnav::{decimal_to_dms, dm_to_decimal};

fn main() {
    println!("=== DMS Conversion Examples ===\n");

    // Example 1: Convert decimal degrees to DM (Decimal Minutes)
    println!("1. Decimal to DM Conversion");
    println!("   Input: 45.504167° (decimal)");
    let dms = decimal_to_dms(45.504167);
    println!("   Output: {}", dms);
    println!("   Components: {} degrees, {:.2} minutes\n",
             dms.degrees, dms.minutes);

    // Example 2: Convert DM to decimal degrees
    println!("2. DM to Decimal Conversion");
    println!("   Input: 45° 30.25'");
    let decimal = dm_to_decimal(45, 30.25);
    println!("   Output: {:.6}°\n", decimal);

    // Example 3: Round-trip conversion
    println!("3. Round-trip Conversion (verify precision)");
    let original = 151.207222; // Sydney longitude (151° 12.433')
    println!("   Original: {:.6}°", original);
    let dms = decimal_to_dms(original);
    println!("   DM:       {}", dms);
    let converted = dm_to_decimal(dms.degrees, dms.minutes);
    println!("   Back:     {:.6}°", converted);
    println!("   Error:    {:.9}° ({})\n",
             (converted - original).abs(),
             if (converted - original).abs() < 1e-6 { "acceptable" } else { "too large" });

    // Example 4: Negative coordinates (Southern hemisphere)
    println!("4. Negative Coordinates (Sydney, Australia)");
    println!("   Latitude: 33° 51.417' S (25.0\" = 0.417')");
    let sydney_lat = dm_to_decimal(-33, 51.417);
    println!("   Decimal:  {:.6}°", sydney_lat);
    let sydney_dms = decimal_to_dms(sydney_lat);
    println!("   Back:     {}\n", sydney_dms);

    // Example 5: High precision celestial navigation
    println!("5. Precision with Decimal Minutes");
    let position: f64 = 40.5127778; // ~40° 30.767'

    // Whole minutes only (old method)
    let old_deg = position.floor() as i32;
    let old_min = ((position - old_deg as f64) * 60.0).round();
    let old_value = old_deg as f64 + old_min / 60.0;

    // Decimal minutes (modern method)
    let new_dms = decimal_to_dms(position);
    let new_value = dm_to_decimal(new_dms.degrees, new_dms.minutes);

    println!("   Original:          {:.7}°", position);
    println!("   Whole minutes:     {:.7}° (40° 31')", old_value);
    println!("   Decimal minutes:   {:.7}° ({})", new_value, new_dms);
    println!("   Whole min error:   {:.7}° (~{:.0} meters)",
             (old_value - position).abs(),
             (old_value - position).abs() * 111320.0); // meters per degree
    println!("   Decimal min error: {:.9}° (~{:.2} meters)\n",
             (new_value - position).abs(),
             (new_value - position).abs() * 111320.0);

    // Example 6: Real-world navigation positions
    println!("6. Real-world Navigation Positions (Decimal Minutes)");

    // New York City
    println!("   New York City:");
    println!("     Lat: 40° 26.767' N = {:.6}°", dm_to_decimal(40, 26.767));
    println!("     Lon: 74° 0.383' W  = {:.6}°", dm_to_decimal(-74, 0.383));

    // Tokyo
    println!("   Tokyo:");
    println!("     Lat: 35° 41.375' N = {:.6}°", dm_to_decimal(35, 41.375));
    println!("     Lon: 139° 41.502' E = {:.6}°", dm_to_decimal(139, 41.502));

    // Cape of Good Hope
    println!("   Cape of Good Hope:");
    println!("     Lat: 34° 21.483' S = {:.6}°", dm_to_decimal(-34, 21.483));
    println!("     Lon: 18° 28.567' E = {:.6}°\n", dm_to_decimal(18, 28.567));

    // Example 7: Edge cases
    println!("7. Edge Cases");

    println!("   Equator:      {}", decimal_to_dms(0.0));
    println!("   North Pole:   {}", decimal_to_dms(90.0));
    println!("   South Pole:   {}", decimal_to_dms(-90.0));
    println!("   180° E/W:     {}", decimal_to_dms(180.0));

    // Near boundary
    let boundary = dm_to_decimal(89, 59.999);
    println!("   89° 59.999': {:.6}°", boundary);
    println!("   Back to DM:  {}\n", decimal_to_dms(boundary));

    // Example 8: Celestial navigation scenario
    println!("8. Celestial Navigation Scenario");
    println!("   DR Position:");
    let dr_lat = dm_to_decimal(41, 30.258);
    let dr_lon = dm_to_decimal(-70, 15.5);
    println!("     Lat: 41° 30.258' N = {:.6}°", dr_lat);
    println!("     Lon: 70° 15.5' W = {:.6}°", dr_lon);

    println!("   Observed Altitude:");
    let altitude = dm_to_decimal(45, 23.205);
    println!("     Hs: 45° 23.205' = {:.6}°", altitude);

    println!("   LHA:");
    let lha = dm_to_decimal(30, 15.75);
    println!("     LHA: 30° 15.75' = {:.6}°\n", lha);

    println!("=== End of Examples ===");
}
