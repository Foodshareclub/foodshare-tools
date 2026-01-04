# foodshare-geo

High-performance geospatial utilities for distance calculations and PostGIS parsing.

[![Crates.io](https://img.shields.io/crates/v/foodshare-geo.svg)](https://crates.io/crates/foodshare-geo)
[![Documentation](https://docs.rs/foodshare-geo/badge.svg)](https://docs.rs/foodshare-geo)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

## Features

- **Haversine Distance** - Accurate great-circle distance calculations
- **PostGIS Parsing** - Parse coordinates from GeoJSON and WKT formats
- **Batch Processing** - Calculate distances for thousands of points efficiently
- **Parallel Processing** - Optional rayon support for multi-threaded batch operations
- **WASM Support** - Compile to WebAssembly for browser usage

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
foodshare-geo = "1.3"
```

### Feature Flags

| Feature | Default | Description |
|---------|---------|-------------|
| `parallel` | Yes | Enable rayon for parallel batch processing |
| `wasm` | No | Enable WebAssembly bindings |

## Usage

### Distance Calculation

```rust
use foodshare_geo::{Coordinate, haversine_distance};

let berlin = Coordinate::new(52.5200, 13.4050);
let paris = Coordinate::new(48.8566, 2.3522);

let distance_km = haversine_distance(&berlin, &paris);
println!("Berlin to Paris: {:.1} km", distance_km); // ~878 km
```

### PostGIS Point Parsing

Parse coordinates from Supabase/PostGIS location data:

```rust
use foodshare_geo::parse_postgis_point;
use serde_json::json;

// GeoJSON format (from PostGIS)
let geojson = json!({
    "type": "Point",
    "coordinates": [13.4050, 52.5200]  // [lng, lat]
});
let coord = parse_postgis_point(&geojson).unwrap();

// WKT format
let wkt = json!("POINT(13.4050 52.5200)");
let coord = parse_postgis_point(&wkt).unwrap();
```

### Batch Distance Calculation

Calculate distances from a user to multiple products:

```rust
use foodshare_geo::{Coordinate, batch::calculate_distances};

let user_location = Coordinate::new(52.5200, 13.4050);
let products = vec![
    ("product_1".to_string(), Coordinate::new(52.5100, 13.4000)),
    ("product_2".to_string(), Coordinate::new(52.4800, 13.3900)),
    ("product_3".to_string(), Coordinate::new(52.5300, 13.4200)),
];

let results = calculate_distances(&user_location, &products);
for result in results {
    println!("{}: {:.2} km", result.id, result.distance_km);
}
```

### Filtering by Radius

```rust
use foodshare_geo::{Coordinate, batch::calculate_distances};

let user = Coordinate::new(52.5200, 13.4050);
let products = vec![/* ... */];

let nearby: Vec<_> = calculate_distances(&user, &products)
    .into_iter()
    .filter(|r| r.distance_km <= 5.0)  // Within 5 km
    .collect();
```

## WASM Usage

For browser usage, see the [@foodshare/geo-wasm](https://www.npmjs.com/package/@foodshare/geo-wasm) npm package.

```typescript
import init, { distance, calculate_product_distances } from '@foodshare/geo-wasm';

await init();

// Single distance
const km = distance(52.52, 13.405, 48.8566, 2.3522);

// Batch calculation
const products = JSON.stringify([
  { id: "1", location: { type: "Point", coordinates: [13.4, 52.51] } },
  { id: "2", location: { type: "Point", coordinates: [13.39, 52.48] } },
]);
const results = JSON.parse(calculate_product_distances(52.52, 13.405, products));
```

## Performance

Benchmarks on Apple M1:

| Operation | Time | Throughput |
|-----------|------|------------|
| Single distance | 140 ns | ~7M/sec |
| 10,000 products batch | 120 µs | 83M/sec |
| PostGIS GeoJSON parse | 6.6 ns | ~150M/sec |

The batch operation processes 10,000 products in under 1 millisecond, making it 50-400x faster than equivalent JavaScript implementations.

## Accuracy

The Haversine formula provides accuracy within 0.5% for most use cases. For very long distances (>10,000 km), consider using Vincenty's formula for sub-meter accuracy.

## License

MIT License - see [LICENSE](LICENSE) for details.
