# foodshare-geo

High-performance geospatial utilities for distance calculations and PostGIS parsing.

## Installation

```toml
[dependencies]
foodshare-geo = "1.4"
```

## Features

- Haversine distance calculations
- PostGIS point parsing (GeoJSON, WKT)
- Batch distance processing
- WASM support for browser usage

## Usage

### Distance Calculation

```rust
use foodshare_geo::{Coordinate, haversine_distance};

let berlin = Coordinate::new(52.5200, 13.4050);
let paris = Coordinate::new(48.8566, 2.3522);

let distance_km = haversine_distance(&berlin, &paris);
println!("Distance: {:.1} km", distance_km); // ~878 km
```

### PostGIS Parsing

```rust
use foodshare_geo::parse_postgis_point;
use serde_json::json;

// GeoJSON format
let geojson = json!({
    "type": "Point",
    "coordinates": [13.4050, 52.5200]
});
let coord = parse_postgis_point(&geojson)?;

// WKT format
let wkt = json!("POINT(13.4050 52.5200)");
let coord = parse_postgis_point(&wkt)?;
```

### Batch Processing

```rust
use foodshare_geo::{Coordinate, batch::calculate_distances};

let user = Coordinate::new(52.5200, 13.4050);
let products = vec![
    ("p1".to_string(), Coordinate::new(52.51, 13.40)),
    ("p2".to_string(), Coordinate::new(52.48, 13.39)),
];

let results = calculate_distances(&user, &products);
```

## WASM Usage

```typescript
import init, { distance } from '@foodshare/geo-wasm';

await init();
const km = distance(52.52, 13.405, 48.8566, 2.3522);
```

## Performance

| Operation | Time |
|-----------|------|
| Single distance | ~140 ns |
| 10,000 batch | ~120 µs |
| PostGIS parse | ~6.6 ns |

## Links

- [crates.io](https://crates.io/crates/foodshare-geo)
- [npm](https://www.npmjs.com/package/@foodshare/geo-wasm)
