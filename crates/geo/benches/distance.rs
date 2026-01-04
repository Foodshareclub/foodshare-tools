//! Benchmarks for geo crate distance calculations.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use foodshare_geo::{batch::LocationItem, calculate_distances, haversine_distance, Coordinate};
use serde_json::json;

fn create_test_items(count: usize) -> Vec<LocationItem> {
    (0..count)
        .map(|i| {
            // Generate points in a grid around Berlin
            let lat = 52.0 + (i as f64 * 0.01) % 2.0;
            let lng = 13.0 + (i as f64 * 0.01) % 2.0;
            LocationItem {
                id: i as i64,
                location: json!({"type": "Point", "coordinates": [lng, lat]}),
            }
        })
        .collect()
}

fn bench_single_distance(c: &mut Criterion) {
    let berlin = Coordinate::new(52.5200, 13.4050);
    let paris = Coordinate::new(48.8566, 2.3522);

    c.bench_function("haversine_single", |b| {
        b.iter(|| haversine_distance(black_box(&berlin), black_box(&paris)))
    });
}

fn bench_batch_distances(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_distances");

    for size in [10, 100, 1000, 10000].iter() {
        let items = create_test_items(*size);
        let user_lat = 50.0;
        let user_lng = 10.0;

        group.bench_with_input(BenchmarkId::new("sequential", size), size, |b, _| {
            b.iter(|| calculate_distances(black_box(user_lat), black_box(user_lng), black_box(&items)))
        });
    }

    group.finish();
}

fn bench_postgis_parsing(c: &mut Criterion) {
    let geojson = json!({"type": "Point", "coordinates": [13.4050, 52.5200]});
    let wkt = json!("POINT(13.4050 52.5200)");

    let mut group = c.benchmark_group("postgis_parsing");

    group.bench_function("geojson", |b| {
        b.iter(|| foodshare_geo::parse_postgis_point(black_box(&geojson)))
    });

    group.bench_function("wkt", |b| {
        b.iter(|| foodshare_geo::parse_postgis_point(black_box(&wkt)))
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_single_distance,
    bench_batch_distances,
    bench_postgis_parsing
);
criterion_main!(benches);
