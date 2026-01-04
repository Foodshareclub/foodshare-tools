//! Haversine distance calculation.
//!
//! The Haversine formula calculates the great-circle distance between two points
//! on a sphere given their longitudes and latitudes.

use crate::Coordinate;

/// Earth's mean radius in kilometers.
pub const EARTH_RADIUS_KM: f64 = 6371.0;

/// Earth's mean radius in meters.
pub const EARTH_RADIUS_M: f64 = 6_371_000.0;

/// Calculates the great-circle distance between two coordinates in kilometers.
///
/// Uses the Haversine formula for accurate distance calculation on a sphere.
///
/// # Arguments
/// * `from` - Starting coordinate
/// * `to` - Ending coordinate
///
/// # Returns
/// Distance in kilometers
///
/// # Example
/// ```
/// use foodshare_geo::{haversine_distance, Coordinate};
///
/// let berlin = Coordinate::new(52.5200, 13.4050);
/// let paris = Coordinate::new(48.8566, 2.3522);
///
/// let distance = haversine_distance(&berlin, &paris);
/// assert!((distance - 878.0).abs() < 10.0);
/// ```
#[inline]
pub fn haversine_distance(from: &Coordinate, to: &Coordinate) -> f64 {
    haversine_distance_with_radius(from, to, EARTH_RADIUS_KM)
}

/// Calculates the great-circle distance between two coordinates in meters.
///
/// # Arguments
/// * `from` - Starting coordinate
/// * `to` - Ending coordinate
///
/// # Returns
/// Distance in meters
#[inline]
pub fn haversine_distance_meters(from: &Coordinate, to: &Coordinate) -> f64 {
    haversine_distance_with_radius(from, to, EARTH_RADIUS_M)
}

/// Internal function that calculates distance with a custom radius.
#[inline]
fn haversine_distance_with_radius(from: &Coordinate, to: &Coordinate, radius: f64) -> f64 {
    let (lat1, lon1) = from.to_radians();
    let (lat2, lon2) = to.to_radians();

    let d_lat = lat2 - lat1;
    let d_lon = lon2 - lon1;

    let a = (d_lat / 2.0).sin().powi(2)
        + lat1.cos() * lat2.cos() * (d_lon / 2.0).sin().powi(2);

    let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());

    radius * c
}

/// Fast approximate distance for filtering (uses equirectangular projection).
///
/// This is faster than Haversine but less accurate over long distances.
/// Use for quick radius filtering before applying Haversine for exact distances.
///
/// # Arguments
/// * `from` - Starting coordinate
/// * `to` - Ending coordinate
///
/// # Returns
/// Approximate distance in kilometers
#[inline]
pub fn approximate_distance(from: &Coordinate, to: &Coordinate) -> f64 {
    let (lat1, lon1) = from.to_radians();
    let (lat2, lon2) = to.to_radians();

    let x = (lon2 - lon1) * ((lat1 + lat2) / 2.0).cos();
    let y = lat2 - lat1;

    (x * x + y * y).sqrt() * EARTH_RADIUS_KM
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test data: known distances between cities
    const BERLIN: Coordinate = Coordinate { latitude: 52.5200, longitude: 13.4050 };
    const PARIS: Coordinate = Coordinate { latitude: 48.8566, longitude: 2.3522 };
    const NEW_YORK: Coordinate = Coordinate { latitude: 40.7128, longitude: -74.0060 };
    const TOKYO: Coordinate = Coordinate { latitude: 35.6762, longitude: 139.6503 };

    #[test]
    fn test_berlin_to_paris() {
        let distance = haversine_distance(&BERLIN, &PARIS);
        // Expected: ~878 km
        assert!((distance - 878.0).abs() < 5.0, "Berlin-Paris: {}", distance);
    }

    #[test]
    fn test_new_york_to_tokyo() {
        let distance = haversine_distance(&NEW_YORK, &TOKYO);
        // Expected: ~10,838 km
        assert!((distance - 10838.0).abs() < 50.0, "NYC-Tokyo: {}", distance);
    }

    #[test]
    fn test_same_point_zero_distance() {
        let distance = haversine_distance(&BERLIN, &BERLIN);
        assert!(distance.abs() < 0.001);
    }

    #[test]
    fn test_symmetry() {
        let d1 = haversine_distance(&BERLIN, &PARIS);
        let d2 = haversine_distance(&PARIS, &BERLIN);
        assert!((d1 - d2).abs() < 0.001);
    }

    #[test]
    fn test_meters_conversion() {
        let km = haversine_distance(&BERLIN, &PARIS);
        let meters = haversine_distance_meters(&BERLIN, &PARIS);
        assert!((meters - km * 1000.0).abs() < 1.0);
    }

    #[test]
    fn test_approximate_distance_reasonable() {
        let exact = haversine_distance(&BERLIN, &PARIS);
        let approx = approximate_distance(&BERLIN, &PARIS);
        // Approximate should be within 5% for this distance
        let error = ((approx - exact) / exact).abs();
        assert!(error < 0.05, "Error: {}%", error * 100.0);
    }
}
