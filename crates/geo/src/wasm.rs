//! WASM bindings for the geo crate.
//!
//! These bindings allow the geo crate to be used from JavaScript/TypeScript
//! in both browser and Deno environments.

use crate::{batch::LocationItem, calculate_distances, haversine_distance, parse_postgis_point, Coordinate};
use wasm_bindgen::prelude::*;

/// Calculate distance between two coordinates.
///
/// # Arguments
/// * `lat1` - Latitude of first point
/// * `lng1` - Longitude of first point
/// * `lat2` - Latitude of second point
/// * `lng2` - Longitude of second point
///
/// # Returns
/// Distance in kilometers
#[wasm_bindgen]
pub fn distance(lat1: f64, lng1: f64, lat2: f64, lng2: f64) -> f64 {
    let from = Coordinate::new(lat1, lng1);
    let to = Coordinate::new(lat2, lng2);
    haversine_distance(&from, &to)
}

/// Calculate distances from user location to multiple products.
///
/// This is the main function that replaces the web worker.
///
/// # Arguments
/// * `user_lat` - User's latitude
/// * `user_lng` - User's longitude
/// * `products_json` - JSON string of products with id and location fields
///
/// # Returns
/// JSON string of products with added distance field
#[wasm_bindgen]
pub fn calculate_product_distances(user_lat: f64, user_lng: f64, products_json: &str) -> Result<String, JsValue> {
    // Parse input JSON
    let items: Vec<LocationItem> = serde_json::from_str(products_json)
        .map_err(|e| JsValue::from_str(&format!("JSON parse error: {}", e)))?;

    // Calculate distances
    let results = calculate_distances(user_lat, user_lng, &items);

    // Serialize results
    serde_json::to_string(&results)
        .map_err(|e| JsValue::from_str(&format!("JSON serialize error: {}", e)))
}

/// Parse a PostGIS location and return coordinates.
///
/// # Arguments
/// * `location_json` - JSON string of location (GeoJSON or WKT string)
///
/// # Returns
/// JSON string with lat/lng, or null if parsing fails
#[wasm_bindgen]
pub fn parse_location(location_json: &str) -> Result<String, JsValue> {
    let value: serde_json::Value = serde_json::from_str(location_json)
        .map_err(|e| JsValue::from_str(&format!("JSON parse error: {}", e)))?;

    match parse_postgis_point(&value) {
        Some(coord) => {
            let result = serde_json::json!({
                "latitude": coord.latitude,
                "longitude": coord.longitude
            });
            Ok(result.to_string())
        }
        None => Ok("null".to_string()),
    }
}

/// Batch distance calculation with sorting.
///
/// # Arguments
/// * `user_lat` - User's latitude
/// * `user_lng` - User's longitude
/// * `products_json` - JSON string of products
/// * `max_results` - Maximum results to return (0 for all)
///
/// # Returns
/// JSON string of sorted results
#[wasm_bindgen]
pub fn calculate_distances_sorted(
    user_lat: f64,
    user_lng: f64,
    products_json: &str,
    max_results: u32,
) -> Result<String, JsValue> {
    let items: Vec<LocationItem> = serde_json::from_str(products_json)
        .map_err(|e| JsValue::from_str(&format!("JSON parse error: {}", e)))?;

    let max = if max_results == 0 { None } else { Some(max_results as usize) };
    let results = crate::batch::calculate_distances_sorted(user_lat, user_lng, &items, max);

    serde_json::to_string(&results)
        .map_err(|e| JsValue::from_str(&format!("JSON serialize error: {}", e)))
}

/// Filter products within a radius.
///
/// # Arguments
/// * `user_lat` - User's latitude
/// * `user_lng` - User's longitude
/// * `products_json` - JSON string of products
/// * `radius_km` - Maximum distance in kilometers
///
/// # Returns
/// JSON string of filtered and sorted results
#[wasm_bindgen]
pub fn filter_within_radius(
    user_lat: f64,
    user_lng: f64,
    products_json: &str,
    radius_km: f64,
) -> Result<String, JsValue> {
    let items: Vec<LocationItem> = serde_json::from_str(products_json)
        .map_err(|e| JsValue::from_str(&format!("JSON parse error: {}", e)))?;

    let results = crate::batch::calculate_distances_within_radius(user_lat, user_lng, &items, radius_km);

    serde_json::to_string(&results)
        .map_err(|e| JsValue::from_str(&format!("JSON serialize error: {}", e)))
}
