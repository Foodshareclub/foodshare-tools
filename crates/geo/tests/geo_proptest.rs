//! Property-based geospatial tests for Haversine distances and PostGIS coordinates.

use foodshare_geo::{haversine_distance, Coordinate, parse_postgis_point};
use proptest::prelude::*;

proptest! {
    #[test]
    fn prop_haversine_symmetry(
        lat1 in -89.9f64..89.9f64,
        lon1 in -179.9f64..179.9f64,
        lat2 in -89.9f64..89.9f64,
        lon2 in -179.9f64..179.9f64,
    ) {
        let a = Coordinate::new(lat1, lon1);
        let b = Coordinate::new(lat2, lon2);
        let dist_ab = haversine_distance(&a, &b);
        let dist_ba = haversine_distance(&b, &a);
        prop_assert!((dist_ab - dist_ba).abs() < 1e-6);
        prop_assert!(dist_ab >= 0.0);
    }

    #[test]
    fn prop_haversine_self_zero(
        lat in -89.9f64..89.9f64,
        lon in -179.9f64..179.9f64,
    ) {
        let a = Coordinate::new(lat, lon);
        let dist = haversine_distance(&a, &a);
        prop_assert!(dist < 1e-6);
    }

    #[test]
    fn prop_haversine_triangle_inequality(
        lat1 in -60.0f64..60.0f64,
        lon1 in -120.0f64..120.0f64,
        lat2 in -60.0f64..60.0f64,
        lon2 in -120.0f64..120.0f64,
        lat3 in -60.0f64..60.0f64,
        lon3 in -120.0f64..120.0f64,
    ) {
        let a = Coordinate::new(lat1, lon1);
        let b = Coordinate::new(lat2, lon2);
        let c = Coordinate::new(lat3, lon3);
        let dist_ac = haversine_distance(&a, &c);
        let dist_ab = haversine_distance(&a, &b);
        let dist_bc = haversine_distance(&b, &c);

        // Triangular inequality: dist(A, C) <= dist(A, B) + dist(B, C) + epsilon
        prop_assert!(dist_ac <= dist_ab + dist_bc + 1e-3);
    }

    #[test]
    fn prop_postgis_wkt_parsing_roundtrip(
        lat in -89.0f64..89.0f64,
        lon in -179.0f64..179.0f64,
    ) {
        let val = serde_json::Value::String(format!("POINT({:.4} {:.4})", lon, lat));
        let parsed = parse_postgis_point(&val).expect("valid point");
        prop_assert!((parsed.latitude - lat).abs() < 1e-3);
        prop_assert!((parsed.longitude - lon).abs() < 1e-3);
    }
}
