use criterion::{black_box, criterion_group, criterion_main, Criterion};
use foodshare_search::{apply_rrf, cosine_similarity, l2_distance};

fn bench_cosine_similarity_384(c: &mut Criterion) {
    let a: Vec<f32> = (0..384).map(|i| (i as f32) * 0.01).collect();
    let b: Vec<f32> = (0..384).map(|i| (i as f32) * 0.02).collect();

    c.bench_function("cosine_similarity_384d", |bencher| {
        bencher.iter(|| {
            cosine_similarity(black_box(&a), black_box(&b))
        });
    });
}

fn bench_cosine_similarity_1536(c: &mut Criterion) {
    let a: Vec<f32> = (0..1536).map(|i| (i as f32) * 0.01).collect();
    let b: Vec<f32> = (0..1536).map(|i| (i as f32) * 0.02).collect();

    c.bench_function("cosine_similarity_1536d", |bencher| {
        bencher.iter(|| {
            cosine_similarity(black_box(&a), black_box(&b))
        });
    });
}

fn bench_l2_distance_384(c: &mut Criterion) {
    let a: Vec<f32> = (0..384).map(|i| (i as f32) * 0.01).collect();
    let b: Vec<f32> = (0..384).map(|i| (i as f32) * 0.02).collect();

    c.bench_function("l2_distance_384d", |bencher| {
        bencher.iter(|| {
            l2_distance(black_box(&a), black_box(&b))
        });
    });
}

fn bench_rrf_rank_fusion(c: &mut Criterion) {
    let list_a: Vec<String> = (0..100).map(|i| format!("listing_{}", i)).collect();
    let list_b: Vec<String> = (50..150).map(|i| format!("listing_{}", i)).collect();
    let list_c: Vec<String> = (25..125).map(|i| format!("listing_{}", i)).collect();
    let lists = vec![list_a, list_b, list_c];

    c.bench_function("rrf_rank_fusion_3x100", |bencher| {
        bencher.iter(|| {
            apply_rrf(black_box(&lists), black_box(60.0))
        });
    });
}

fn bench_hybrid_score(c: &mut Criterion) {
    let q_vec: Vec<f32> = (0..384).map(|i| (i as f32) * 0.01).collect();
    let item_vec: Vec<f32> = (0..384).map(|i| (i as f32) * 0.015).collect();
    let weights = foodshare_search::HybridWeights::default();

    c.bench_function("hybrid_multi_modal_score_384d", |bencher| {
        bencher.iter(|| {
            foodshare_search::calculate_hybrid_score(
                black_box("organic sourdough bread"),
                black_box("fresh organic artisan sourdough bread"),
                black_box(Some(&q_vec)),
                black_box(Some(&item_vec)),
                black_box(Some(4.5)),
                black_box(&weights),
            )
        });
    });
}

fn bench_distance_decay(c: &mut Criterion) {
    c.bench_function("geo_distance_decay", |bencher| {
        bencher.iter(|| {
            foodshare_search::calculate_distance_decay(black_box(7.5), black_box(10.0))
        });
    });
}

criterion_group!(
    benches,
    bench_cosine_similarity_384,
    bench_cosine_similarity_1536,
    bench_l2_distance_384,
    bench_rrf_rank_fusion,
    bench_hybrid_score,
    bench_distance_decay
);
criterion_main!(benches);
