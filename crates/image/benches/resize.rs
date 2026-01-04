//! Benchmarks for image processing.

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use foodshare_image::{detect_format, calculate_target_width};

fn bench_format_detection(c: &mut Criterion) {
    // JPEG magic bytes
    let jpeg_data = vec![0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46, 0x49, 0x46];
    // PNG magic bytes
    let png_data = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00];

    c.bench_function("detect_jpeg", |b| {
        b.iter(|| detect_format(black_box(&jpeg_data)))
    });

    c.bench_function("detect_png", |b| {
        b.iter(|| detect_format(black_box(&png_data)))
    });
}

fn bench_target_width(c: &mut Criterion) {
    c.bench_function("calculate_target_width", |b| {
        b.iter(|| {
            calculate_target_width(
                black_box(3 * 1024 * 1024), // 3MB
                black_box(4000),
                black_box(3000),
            )
        })
    });
}

criterion_group!(benches, bench_format_detection, bench_target_width);
criterion_main!(benches);
