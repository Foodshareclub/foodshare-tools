//! Benchmarks for compression algorithms.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use foodshare_compression::{brotli_compress, gzip_compress};

fn generate_test_data(size: usize) -> Vec<u8> {
    // Generate compressible data (repeated text)
    let text = "Hello, World! This is test data for compression benchmarks. ";
    text.repeat(size / text.len() + 1).into_bytes()[..size].to_vec()
}

fn bench_compression(c: &mut Criterion) {
    let mut group = c.benchmark_group("compression");

    for size in [1024, 10240, 102400].iter() {
        let data = generate_test_data(*size);

        group.bench_with_input(BenchmarkId::new("brotli", size), &data, |b, data| {
            b.iter(|| brotli_compress(black_box(data), 6))
        });

        group.bench_with_input(BenchmarkId::new("gzip", size), &data, |b, data| {
            b.iter(|| gzip_compress(black_box(data), 6))
        });
    }

    group.finish();
}

criterion_group!(benches, bench_compression);
criterion_main!(benches);
