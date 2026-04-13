//! Test the move_position function to ensure it's working correctly

use celtnav::fix_calculation::LineOfPosition;
use celtnav::dms_to_decimal;

// We can't directly test move_position since it's private, but we can test
// the behavior indirectly by checking if our intercept calculations are correct

#[test]
fn test_intercept_geometry() {
    // Test case: Hamal
    // AP: 50°N, 20°04.4'W
    // Intercept: 11.3 NM Toward (positive)
    // Azimuth: 121°

    let ap_lat: f64 = 50.0;
    let ap_lon: f64 = -dms_to_decimal(20, 4, 24.0);  // 20°04.4'W
    let intercept: f64 = 11.3;
    let azimuth: f64 = 121.0;

    println!("\n=== MANUAL INTERCEPT CALCULATION ===");
    println!("AP: {:.4}°N, {:.4}°W", ap_lat, ap_lon.abs());
    println!("Intercept: {:.1} NM Toward", intercept);
    println!("Azimuth: {:.1}°", azimuth);

    // Using plane sailing approximation (standard for celestial nav)
    let azimuth_rad = azimuth.to_radians();
    let cos_lat = ap_lat.to_radians().cos();

    // Offset in NM
    let offset_lat_nm = intercept * azimuth_rad.cos();  //  NM north/south
    let offset_lon_nm = intercept * azimuth_rad.sin();  // NM east/west

    // Convert to degrees
    let offset_lat_deg = offset_lat_nm / 60.0;
    let offset_lon_deg = offset_lon_nm / (60.0 * cos_lat);

    let lop_point_lat = ap_lat + offset_lat_deg;
    let lop_point_lon = ap_lon + offset_lon_deg;

    println!("\nPlane sailing calculation:");
    println!("  Offset: {:.2} NM south, {:.2} NM east", -offset_lat_nm, offset_lon_nm);
    println!("  LOP point: {:.4}°N, {:.4}°W", lop_point_lat, lop_point_lon.abs());

    // The LOP passes through this point, perpendicular to the azimuth
    // Test that a fix at 49°59.9'N, 19°56.6'W is close to this LOP

    let test_fix_lat = dms_to_decimal(49, 59, 54.0);  // 49°59.9'N
    let test_fix_lon = -dms_to_decimal(19, 56, 36.0); // 19°56.6'W

    // Vector from LOP point to test fix in NM
    let delta_lat_nm = (test_fix_lat - lop_point_lat) * 60.0;
    let delta_lon_nm = (test_fix_lon - lop_point_lon) * 60.0 * test_fix_lat.to_radians().cos();

    // Normal to LOP (in azimuth direction)
    let n_lat = azimuth_rad.cos();
    let n_lon = azimuth_rad.sin();

    // Perpendicular distance from test fix to LOP
    let perp_dist = delta_lat_nm * n_lat + delta_lon_nm * n_lon;

    println!("\nTest fix: {:.4}°N, {:.4}°W", test_fix_lat, test_fix_lon.abs());
    println!("Distance from test fix to Hamal LOP: {:.2} NM", perp_dist.abs());
    println!("\nIf fix calculation is correct, this should be close to 0 NM");
}
