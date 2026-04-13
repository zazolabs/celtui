// SPDX-License-Identifier: GPL-2.0-only
// SPDX-FileCopyrightText: Alexander Atanasov <alex@zazolabs.com>
//! Debug azimuth calculation to understand the issue

#[test]
fn debug_hamal_azimuth() {
    // Hamal test case
    let lat: f64 = 50.0;
    let dec: f64 = 23.5295;  // With proper motion correction
    let lha: f64 = 34.9877;

    println!("\n=== HAMAL AZIMUTH DEBUG ===");
    println!("Latitude: {:.4}°", lat);
    println!("Declination: {:.4}°", dec);
    println!("LHA: {:.4}°", lha);

    // Convert to radians
    let lat_rad = lat.to_radians();
    let dec_rad = dec.to_radians();
    let lha_rad = lha.to_radians();

    // Standard azimuth formula (from coords.rs)
    let x = dec_rad.cos() * lha_rad.sin();
    let y = lat_rad.cos() * dec_rad.sin() - lat_rad.sin() * dec_rad.cos() * lha_rad.cos();

    let azimuth_current = x.atan2(y).to_degrees();
    let azimuth_normalized = if azimuth_current < 0.0 {
        azimuth_current + 360.0
    } else {
        azimuth_current
    };

    println!("\nCurrent formula:");
    println!("  x = cos(Dec) * sin(LHA) = {:.6}", x);
    println!("  y = cos(Lat)*sin(Dec) - sin(Lat)*cos(Dec)*cos(LHA) = {:.6}", y);
    println!("  atan2(x, y) = {:.2}°", azimuth_current);
    println!("  Normalized = {:.2}°", azimuth_normalized);

    // Try alternative: 360 - azimuth
    println!("\nAlternative (360 - azimuth): {:.2}°", 360.0 - azimuth_normalized);

    // Try swapping atan2 arguments
    let azimuth_swapped = y.atan2(x).to_degrees();
    let azimuth_swapped_norm = if azimuth_swapped < 0.0 {
        azimuth_swapped + 360.0
    } else {
        azimuth_swapped
    };
    println!("\nSwapped atan2(y, x): {:.2}°", azimuth_swapped_norm);

    // Try negative x
    let x_neg = -x;
    let azimuth_negx = x_neg.atan2(y).to_degrees();
    let azimuth_negx_norm = if azimuth_negx < 0.0 {
        azimuth_negx + 360.0
    } else {
        azimuth_negx
    };
    println!("With -x: atan2(-x, y) = {:.2}°", azimuth_negx_norm);

    // Try negative y
    let y_neg = -y;
    let azimuth_negy = x.atan2(y_neg).to_degrees();
    let azimuth_negy_norm = if azimuth_negy < 0.0 {
        azimuth_negy + 360.0
    } else {
        azimuth_negy
    };
    println!("With -y: atan2(x, -y) = {:.2}°", azimuth_negy_norm);

    println!("\n** Expected from Pub.249: 239° **");
    println!("** Closest match: {} **",
             if (azimuth_normalized - 239.0).abs() < 2.0 { "Current"
             } else if ((360.0 - azimuth_normalized) - 239.0).abs() < 2.0 { "360 - current"
             } else if (azimuth_swapped_norm - 239.0).abs() < 2.0 { "Swapped"
             } else if (azimuth_negx_norm - 239.0).abs() < 2.0 { "-x version"
             } else if (azimuth_negy_norm - 239.0).abs() < 2.0 { "-y version"
             } else { "NONE!" });
}
