//! WASM bindings for the geo crate.
//!
//! These bindings allow the geo crate to be used from JavaScript/TypeScript
//! in both browser and Deno environments.
//!
//! Structured data crosses the boundary via `serde-wasm-bindgen` (no JSON
//! string roundtrip): callers pass JS objects/arrays directly and receive JS
//! objects/arrays back.

use crate::{
    Coordinate, batch::LocationItem, calculate_distances, haversine_distance, parse_postgis_point,
};
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
/// * `products` - JS array of products with id and location fields
///
/// # Returns
/// JS array of products with added distance field
#[wasm_bindgen]
pub fn calculate_product_distances(
    user_lat: f64,
    user_lng: f64,
    products: JsValue,
) -> Result<JsValue, JsValue> {
    // Deserialize input directly from JsValue (no JSON string roundtrip)
    let items: Vec<LocationItem> = serde_wasm_bindgen::from_value(products)
        .map_err(|e| JsValue::from_str(&format!("parse error: {}", e)))?;

    // Calculate distances
    let results = calculate_distances(user_lat, user_lng, &items);

    // Serialize results directly to JsValue
    serde_wasm_bindgen::to_value(&results)
        .map_err(|e| JsValue::from_str(&format!("serialize error: {}", e)))
}

/// Parse a PostGIS location and return coordinates.
///
/// # Arguments
/// * `location` - JS value holding the location (GeoJSON object or WKT string)
///
/// # Returns
/// JS object with lat/lng, or null if parsing fails
#[wasm_bindgen]
pub fn parse_location(location: JsValue) -> Result<JsValue, JsValue> {
    let value: serde_json::Value = serde_wasm_bindgen::from_value(location)
        .map_err(|e| JsValue::from_str(&format!("parse error: {}", e)))?;

    match parse_postgis_point(&value) {
        Some(coord) => {
            let result = serde_json::json!({
                "latitude": coord.latitude,
                "longitude": coord.longitude
            });
            serde_wasm_bindgen::to_value(&result)
                .map_err(|e| JsValue::from_str(&format!("serialize error: {}", e)))
        }
        None => Ok(JsValue::NULL),
    }
}

/// Batch distance calculation with sorting.
///
/// # Arguments
/// * `user_lat` - User's latitude
/// * `user_lng` - User's longitude
/// * `products` - JS array of products
/// * `max_results` - Maximum results to return (0 for all)
///
/// # Returns
/// JS array of sorted results
#[wasm_bindgen]
pub fn calculate_distances_sorted(
    user_lat: f64,
    user_lng: f64,
    products: JsValue,
    max_results: u32,
) -> Result<JsValue, JsValue> {
    let items: Vec<LocationItem> = serde_wasm_bindgen::from_value(products)
        .map_err(|e| JsValue::from_str(&format!("parse error: {}", e)))?;

    let max = if max_results == 0 {
        None
    } else {
        Some(max_results as usize)
    };
    let results = crate::batch::calculate_distances_sorted(user_lat, user_lng, &items, max);

    serde_wasm_bindgen::to_value(&results)
        .map_err(|e| JsValue::from_str(&format!("serialize error: {}", e)))
}

/// Filter products within a radius.
///
/// # Arguments
/// * `user_lat` - User's latitude
/// * `user_lng` - User's longitude
/// * `products` - JS array of products
/// * `radius_km` - Maximum distance in kilometers
///
/// # Returns
/// JS array of filtered and sorted results
#[wasm_bindgen]
pub fn filter_within_radius(
    user_lat: f64,
    user_lng: f64,
    products: JsValue,
    radius_km: f64,
) -> Result<JsValue, JsValue> {
    let items: Vec<LocationItem> = serde_wasm_bindgen::from_value(products)
        .map_err(|e| JsValue::from_str(&format!("parse error: {}", e)))?;

    let results =
        crate::batch::calculate_distances_within_radius(user_lat, user_lng, &items, radius_km);

    serde_wasm_bindgen::to_value(&results)
        .map_err(|e| JsValue::from_str(&format!("serialize error: {}", e)))
}
