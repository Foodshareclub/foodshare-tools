//! High-performance geospatial utilities for FoodShare.
//!
//! This crate provides:
//! - Haversine distance calculations
//! - PostGIS POINT parsing (JSON and WKT formats)
//! - Batch processing with optional parallelism
//! - WASM bindings for browser usage
//!
//! # Example
//!
//! ```
//! use foodshare_geo::{haversine_distance, Coordinate};
//!
//! let coord1 = Coordinate::new(52.5200, 13.4050); // Berlin
//! let coord2 = Coordinate::new(48.8566, 2.3522);  // Paris
//!
//! let distance_km = haversine_distance(&coord1, &coord2);
//! assert!((distance_km - 878.0).abs() < 10.0); // ~878 km
//! ```

mod haversine;
mod postgis;
pub mod batch;
mod error;

#[cfg(feature = "wasm")]
mod wasm;

pub use haversine::{haversine_distance, haversine_distance_meters, EARTH_RADIUS_KM, EARTH_RADIUS_M};
pub use postgis::{parse_postgis_point, PostGISPoint};
pub use batch::{calculate_distances, DistanceResult};
pub use error::{GeoError, Result};

/// A geographic coordinate with latitude and longitude.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Coordinate {
    /// Latitude in degrees (-90 to 90)
    pub latitude: f64,
    /// Longitude in degrees (-180 to 180)
    pub longitude: f64,
}

impl Coordinate {
    /// Creates a new coordinate.
    ///
    /// # Arguments
    /// * `latitude` - Latitude in degrees (-90 to 90)
    /// * `longitude` - Longitude in degrees (-180 to 180)
    #[inline]
    pub fn new(latitude: f64, longitude: f64) -> Self {
        Self { latitude, longitude }
    }

    /// Returns true if the coordinate has valid values.
    #[inline]
    pub fn is_valid(&self) -> bool {
        self.latitude >= -90.0
            && self.latitude <= 90.0
            && self.longitude >= -180.0
            && self.longitude <= 180.0
    }

    /// Converts degrees to radians for internal calculations.
    #[inline]
    pub(crate) fn to_radians(&self) -> (f64, f64) {
        (self.latitude.to_radians(), self.longitude.to_radians())
    }
}

impl From<(f64, f64)> for Coordinate {
    fn from((lat, lng): (f64, f64)) -> Self {
        Self::new(lat, lng)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_coordinate_creation() {
        let coord = Coordinate::new(52.5200, 13.4050);
        assert_eq!(coord.latitude, 52.5200);
        assert_eq!(coord.longitude, 13.4050);
    }

    #[test]
    fn test_coordinate_validation() {
        assert!(Coordinate::new(0.0, 0.0).is_valid());
        assert!(Coordinate::new(90.0, 180.0).is_valid());
        assert!(Coordinate::new(-90.0, -180.0).is_valid());
        assert!(!Coordinate::new(91.0, 0.0).is_valid());
        assert!(!Coordinate::new(0.0, 181.0).is_valid());
    }

    #[test]
    fn test_coordinate_from_tuple() {
        let coord: Coordinate = (52.5200, 13.4050).into();
        assert_eq!(coord.latitude, 52.5200);
    }
}
