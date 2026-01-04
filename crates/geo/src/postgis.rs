//! PostGIS POINT parsing utilities.
//!
//! Supports parsing coordinates from:
//! - GeoJSON format: `{"type": "Point", "coordinates": [lng, lat]}`
//! - WKT format: `POINT(lng lat)`

use crate::{Coordinate, GeoError, Result};
use serde::{Deserialize, Serialize};

/// A PostGIS Point representation that can be parsed from JSON or WKT.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PostGISPoint {
    /// GeoJSON format: {"type": "Point", "coordinates": [lng, lat]}
    GeoJson(GeoJsonPoint),
    /// Raw WKT string: "POINT(lng lat)"
    Wkt(String),
}

/// GeoJSON Point format.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeoJsonPoint {
    /// Should be "Point"
    #[serde(rename = "type")]
    pub point_type: Option<String>,
    /// [longitude, latitude] array
    pub coordinates: [f64; 2],
}

impl PostGISPoint {
    /// Parse a PostGIS point from various formats.
    ///
    /// Supports:
    /// - GeoJSON: `{"type": "Point", "coordinates": [lng, lat]}`
    /// - WKT string: `"POINT(lng lat)"`
    /// - JSON string containing WKT
    pub fn to_coordinate(&self) -> Result<Coordinate> {
        match self {
            PostGISPoint::GeoJson(geojson) => {
                let [lng, lat] = geojson.coordinates;
                Ok(Coordinate::new(lat, lng))
            }
            PostGISPoint::Wkt(wkt) => parse_wkt_point(wkt),
        }
    }
}

/// Parse a PostGIS point from a JSON value.
///
/// This is the main entry point for parsing location data from Supabase.
///
/// # Arguments
/// * `value` - A serde_json::Value that may contain location data
///
/// # Returns
/// * `Some(Coordinate)` if parsing succeeds
/// * `None` if the value is null or cannot be parsed
///
/// # Example
/// ```
/// use foodshare_geo::parse_postgis_point;
/// use serde_json::json;
///
/// // GeoJSON format
/// let geojson = json!({"type": "Point", "coordinates": [13.4050, 52.5200]});
/// let coord = parse_postgis_point(&geojson).unwrap();
/// assert!((coord.latitude - 52.5200).abs() < 0.0001);
///
/// // WKT format
/// let wkt = json!("POINT(13.4050 52.5200)");
/// let coord = parse_postgis_point(&wkt).unwrap();
/// assert!((coord.latitude - 52.5200).abs() < 0.0001);
/// ```
pub fn parse_postgis_point(value: &serde_json::Value) -> Option<Coordinate> {
    if value.is_null() {
        return None;
    }

    // Try parsing as GeoJSON object
    if value.is_object() {
        if let Some(coords) = value.get("coordinates").and_then(|c| c.as_array()) {
            if coords.len() >= 2 {
                let lng = coords[0].as_f64()?;
                let lat = coords[1].as_f64()?;
                return Some(Coordinate::new(lat, lng));
            }
        }
    }

    // Try parsing as WKT string
    if let Some(wkt) = value.as_str() {
        return parse_wkt_point(wkt).ok();
    }

    None
}

/// Parse a WKT POINT string.
///
/// Format: `POINT(longitude latitude)`
fn parse_wkt_point(wkt: &str) -> Result<Coordinate> {
    // Match POINT(lng lat) format
    let wkt = wkt.trim();

    if !wkt.starts_with("POINT(") && !wkt.starts_with("POINT (") {
        return Err(GeoError::InvalidWkt(format!("Expected POINT, got: {}", wkt)));
    }

    // Find the coordinates between parentheses
    let start = wkt.find('(').ok_or_else(|| GeoError::InvalidWkt("Missing '('".into()))?;
    let end = wkt.find(')').ok_or_else(|| GeoError::InvalidWkt("Missing ')'".into()))?;

    if start >= end {
        return Err(GeoError::InvalidWkt("Invalid parentheses".into()));
    }

    let coords_str = &wkt[start + 1..end];
    let parts: Vec<&str> = coords_str.split_whitespace().collect();

    if parts.len() != 2 {
        return Err(GeoError::InvalidWkt(format!(
            "Expected 2 coordinates, got {}",
            parts.len()
        )));
    }

    let lng: f64 = parts[0]
        .parse()
        .map_err(|_| GeoError::InvalidWkt(format!("Invalid longitude: {}", parts[0])))?;
    let lat: f64 = parts[1]
        .parse()
        .map_err(|_| GeoError::InvalidWkt(format!("Invalid latitude: {}", parts[1])))?;

    Ok(Coordinate::new(lat, lng))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_parse_geojson_point() {
        let value = json!({
            "type": "Point",
            "coordinates": [13.4050, 52.5200]
        });

        let coord = parse_postgis_point(&value).unwrap();
        assert!((coord.latitude - 52.5200).abs() < 0.0001);
        assert!((coord.longitude - 13.4050).abs() < 0.0001);
    }

    #[test]
    fn test_parse_geojson_without_type() {
        let value = json!({
            "coordinates": [13.4050, 52.5200]
        });

        let coord = parse_postgis_point(&value).unwrap();
        assert!((coord.latitude - 52.5200).abs() < 0.0001);
    }

    #[test]
    fn test_parse_wkt_point() {
        let value = json!("POINT(13.4050 52.5200)");
        let coord = parse_postgis_point(&value).unwrap();
        assert!((coord.latitude - 52.5200).abs() < 0.0001);
        assert!((coord.longitude - 13.4050).abs() < 0.0001);
    }

    #[test]
    fn test_parse_wkt_with_space() {
        let value = json!("POINT (13.4050 52.5200)");
        let coord = parse_postgis_point(&value).unwrap();
        assert!((coord.latitude - 52.5200).abs() < 0.0001);
    }

    #[test]
    fn test_parse_null_returns_none() {
        let value = json!(null);
        assert!(parse_postgis_point(&value).is_none());
    }

    #[test]
    fn test_parse_invalid_wkt() {
        let result = parse_wkt_point("POLYGON((0 0, 1 1, 1 0, 0 0))");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_negative_coordinates() {
        let value = json!("POINT(-74.0060 40.7128)");
        let coord = parse_postgis_point(&value).unwrap();
        assert!((coord.latitude - 40.7128).abs() < 0.0001);
        assert!((coord.longitude - (-74.0060)).abs() < 0.0001);
    }
}
