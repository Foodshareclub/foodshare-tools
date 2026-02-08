# @foodshare/geo-wasm

High-performance geospatial utilities compiled to WebAssembly from Rust.

[![npm version](https://img.shields.io/npm/v/@foodshare/geo-wasm.svg)](https://www.npmjs.com/package/@foodshare/geo-wasm)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

## Features

- **Haversine Distance** - Calculate great-circle distances between coordinates
- **Batch Processing** - Process thousands of distance calculations in milliseconds
- **PostGIS Parsing** - Parse GeoJSON and WKT point formats
- **Radius Filtering** - Filter locations within a specified radius
- **TypeScript Support** - Full type definitions included

## Installation

```bash
bun add @foodshare/geo-wasm
```

## Usage

### Initialization

```typescript
import init, { distance, calculate_product_distances } from '@foodshare/geo-wasm';

// Initialize WASM module (required once)
await init();
```

### Single Distance Calculation

```typescript
import init, { distance } from '@foodshare/geo-wasm';

await init();

// Calculate distance between Berlin and Paris
const km = distance(52.52, 13.405, 48.8566, 2.3522);
console.log(`Distance: ${km.toFixed(1)} km`); // ~878 km
```

### Batch Distance Calculation

Perfect for calculating distances from a user to multiple products/locations:

```typescript
import init, { calculate_product_distances } from '@foodshare/geo-wasm';

await init();

const userLat = 52.52;
const userLng = 13.405;

const products = JSON.stringify([
  { id: "product_1", location: { type: "Point", coordinates: [13.4, 52.51] } },
  { id: "product_2", location: { type: "Point", coordinates: [13.39, 52.48] } },
  { id: "product_3", location: { type: "Point", coordinates: [13.42, 52.53] } },
]);

const results = JSON.parse(calculate_product_distances(userLat, userLng, products));
// [{ id: "product_1", distance_km: 1.2 }, ...]
```

### Sorted Results with Limit

```typescript
import init, { calculate_distances_sorted } from '@foodshare/geo-wasm';

await init();

// Get 10 nearest products, sorted by distance
const nearest = JSON.parse(
  calculate_distances_sorted(userLat, userLng, products, 10)
);
```

### Filter Within Radius

```typescript
import init, { filter_within_radius } from '@foodshare/geo-wasm';

await init();

// Get products within 5km radius
const nearby = JSON.parse(
  filter_within_radius(userLat, userLng, products, 5.0)
);
```

### Parse PostGIS Location

```typescript
import init, { parse_location } from '@foodshare/geo-wasm';

await init();

// Parse GeoJSON
const geoJson = JSON.stringify({ type: "Point", coordinates: [13.405, 52.52] });
const coords = JSON.parse(parse_location(geoJson));
// { lat: 52.52, lng: 13.405 }

// Parse WKT
const wkt = JSON.stringify("POINT(13.405 52.52)");
const coords2 = JSON.parse(parse_location(wkt));
```

## API Reference

### `distance(lat1, lng1, lat2, lng2): number`
Calculate distance between two coordinates in kilometers.

### `calculate_product_distances(user_lat, user_lng, products_json): string`
Calculate distances from user to multiple products. Returns JSON array.

### `calculate_distances_sorted(user_lat, user_lng, products_json, max_results): string`
Calculate and sort distances, optionally limiting results.

### `filter_within_radius(user_lat, user_lng, products_json, radius_km): string`
Filter products within a radius, sorted by distance.

### `parse_location(location_json): string`
Parse a PostGIS location (GeoJSON or WKT) to coordinates.

## Product JSON Format

Products should have an `id` and `location` field:

```typescript
interface Product {
  id: string;
  location:
    | { type: "Point"; coordinates: [number, number] }  // GeoJSON [lng, lat]
    | string;  // WKT: "POINT(lng lat)"
}
```

## Performance

Benchmarks on Apple M1:

| Operation | Time | vs JavaScript |
|-----------|------|---------------|
| Single distance | 140 ns | ~10x faster |
| 10,000 products | 120 µs | 50-400x faster |
| PostGIS parse | 6.6 ns | ~100x faster |

The WASM module processes 10,000 products in under 1 millisecond.

## Framework Integration

### Next.js

```typescript
// hooks/useGeo.ts
import { useEffect, useState } from 'react';

export function useGeo() {
  const [geo, setGeo] = useState<typeof import('@foodshare/geo-wasm') | null>(null);

  useEffect(() => {
    import('@foodshare/geo-wasm').then(async (module) => {
      await module.default();
      setGeo(module);
    });
  }, []);

  return geo;
}
```

### Nuxt/Vue

```typescript
// composables/useGeo.ts
export const useGeo = () => {
  const geo = useState<typeof import('@foodshare/geo-wasm') | null>('geo', () => null);

  onMounted(async () => {
    const module = await import('@foodshare/geo-wasm');
    await module.default();
    geo.value = module;
  });

  return geo;
};
```

## Browser Support

Works in all modern browsers with WebAssembly support:
- Chrome 57+
- Firefox 52+
- Safari 11+
- Edge 16+

## License

MIT License - see [LICENSE](https://github.com/Foodshareclub/foodshare-tools/blob/main/LICENSE) for details.

## Related

- [foodshare-geo](https://crates.io/crates/foodshare-geo) - The Rust crate this package is built from
