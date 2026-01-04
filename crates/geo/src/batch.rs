//! Batch distance calculations with optional parallelism.
//!
//! This module provides high-performance batch processing of distance calculations,
//! which is the primary use case for replacing the web worker.

use crate::{haversine_distance, parse_postgis_point, Coordinate};
use serde::{Deserialize, Serialize};

/// Result of a distance calculation for a single item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistanceResult {
    /// The item ID
    pub id: i64,
    /// Calculated distance in kilometers (Infinity if location is invalid)
    pub distance: f64,
}

/// Input item for batch distance calculation.
#[derive(Debug, Clone, Deserialize)]
pub struct LocationItem {
    /// Item ID
    pub id: i64,
    /// Location in PostGIS format (GeoJSON or WKT)
    pub location: serde_json::Value,
}

/// Calculate distances from a user location to multiple items.
///
/// This is the main batch processing function that replaces the web worker.
///
/// # Arguments
/// * `user_lat` - User's latitude
/// * `user_lng` - User's longitude
/// * `items` - Slice of items with location data
///
/// # Returns
/// Vector of distance results, one for each input item.
///
/// # Example
/// ```
/// use foodshare_geo::{calculate_distances, batch::LocationItem};
/// use serde_json::json;
///
/// let items = vec![
///     LocationItem { id: 1, location: json!({"coordinates": [13.4050, 52.5200]}) },
///     LocationItem { id: 2, location: json!("POINT(2.3522 48.8566)") },
/// ];
///
/// let results = calculate_distances(50.0, 10.0, &items);
/// assert_eq!(results.len(), 2);
/// ```
pub fn calculate_distances(user_lat: f64, user_lng: f64, items: &[LocationItem]) -> Vec<DistanceResult> {
    let user_coord = Coordinate::new(user_lat, user_lng);

    #[cfg(feature = "parallel")]
    {
        use rayon::prelude::*;
        items
            .par_iter()
            .map(|item| calculate_single_distance(&user_coord, item))
            .collect()
    }

    #[cfg(not(feature = "parallel"))]
    {
        items
            .iter()
            .map(|item| calculate_single_distance(&user_coord, item))
            .collect()
    }
}

/// Calculate distances and return items sorted by distance.
///
/// # Arguments
/// * `user_lat` - User's latitude
/// * `user_lng` - User's longitude
/// * `items` - Slice of items with location data
/// * `max_results` - Maximum number of results to return (None for all)
///
/// # Returns
/// Vector of distance results sorted by distance (closest first).
pub fn calculate_distances_sorted(
    user_lat: f64,
    user_lng: f64,
    items: &[LocationItem],
    max_results: Option<usize>,
) -> Vec<DistanceResult> {
    let mut results = calculate_distances(user_lat, user_lng, items);

    // Sort by distance, putting Infinity at the end
    results.sort_by(|a, b| {
        a.distance.partial_cmp(&b.distance).unwrap_or(std::cmp::Ordering::Equal)
    });

    if let Some(max) = max_results {
        results.truncate(max);
    }

    results
}

/// Calculate distances within a radius.
///
/// # Arguments
/// * `user_lat` - User's latitude
/// * `user_lng` - User's longitude
/// * `items` - Slice of items with location data
/// * `radius_km` - Maximum distance in kilometers
///
/// # Returns
/// Vector of distance results for items within the radius, sorted by distance.
pub fn calculate_distances_within_radius(
    user_lat: f64,
    user_lng: f64,
    items: &[LocationItem],
    radius_km: f64,
) -> Vec<DistanceResult> {
    let mut results = calculate_distances(user_lat, user_lng, items);

    // Filter by radius
    results.retain(|r| r.distance <= radius_km);

    // Sort by distance
    results.sort_by(|a, b| {
        a.distance.partial_cmp(&b.distance).unwrap_or(std::cmp::Ordering::Equal)
    });

    results
}

/// Calculate distance for a single item.
#[inline]
fn calculate_single_distance(user_coord: &Coordinate, item: &LocationItem) -> DistanceResult {
    let distance = parse_postgis_point(&item.location)
        .map(|coord| haversine_distance(user_coord, &coord))
        .unwrap_or(f64::INFINITY);

    DistanceResult {
        id: item.id,
        distance,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn create_test_items() -> Vec<LocationItem> {
        vec![
            // Berlin
            LocationItem {
                id: 1,
                location: json!({"type": "Point", "coordinates": [13.4050, 52.5200]}),
            },
            // Paris
            LocationItem {
                id: 2,
                location: json!("POINT(2.3522 48.8566)"),
            },
            // London
            LocationItem {
                id: 3,
                location: json!({"coordinates": [-0.1276, 51.5074]}),
            },
            // Invalid location
            LocationItem {
                id: 4,
                location: json!(null),
            },
        ]
    }

    #[test]
    fn test_batch_distances() {
        let items = create_test_items();
        // User in Frankfurt (roughly between Berlin and Paris)
        let results = calculate_distances(50.1109, 8.6821, &items);

        assert_eq!(results.len(), 4);

        // Check Berlin result
        let berlin = results.iter().find(|r| r.id == 1).unwrap();
        assert!(berlin.distance > 0.0 && berlin.distance < 500.0);

        // Check invalid location has Infinity
        let invalid = results.iter().find(|r| r.id == 4).unwrap();
        assert!(invalid.distance.is_infinite());
    }

    #[test]
    fn test_sorted_distances() {
        let items = create_test_items();
        let results = calculate_distances_sorted(50.1109, 8.6821, &items, None);

        // Should be sorted by distance (excluding Infinity at end)
        for window in results.windows(2) {
            if !window[0].distance.is_infinite() && !window[1].distance.is_infinite() {
                assert!(window[0].distance <= window[1].distance);
            }
        }
    }

    #[test]
    fn test_radius_filter() {
        let items = create_test_items();
        // User in Frankfurt, radius 400km (should include Berlin but not London)
        let results = calculate_distances_within_radius(50.1109, 8.6821, &items, 400.0);

        // Berlin (~400km) should be included, Paris (~450km) should be close
        // London (~650km) should be excluded
        assert!(results.iter().all(|r| r.distance <= 400.0));
    }

    #[test]
    fn test_max_results() {
        let items = create_test_items();
        let results = calculate_distances_sorted(50.1109, 8.6821, &items, Some(2));

        assert_eq!(results.len(), 2);
    }
}
