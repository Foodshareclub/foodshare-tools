use criterion::{Criterion, black_box, criterion_group, criterion_main};
use foodshare_crypto::{
    DEFAULT_DIGITS, DEFAULT_TIME_STEP, constant_time_compare, generate_totp, hmac_sha256,
    verify_totp,
};

fn bench_totp_generation(c: &mut Criterion) {
    let secret = b"12345678901234567890";
    let timestamp = 1724790000;

    c.bench_function("totp_generate_token", |bencher| {
        bencher.iter(|| {
            generate_totp(
                black_box(secret),
                black_box(timestamp),
                black_box(DEFAULT_TIME_STEP),
                black_box(DEFAULT_DIGITS),
            )
            .unwrap()
        });
    });
}

fn bench_totp_verification(c: &mut Criterion) {
    let secret = b"12345678901234567890";
    let code = "123456";
    let timestamp = 1724790000;

    c.bench_function("totp_verify_with_window", |bencher| {
        bencher.iter(|| {
            verify_totp(
                black_box(secret),
                black_box(code),
                black_box(timestamp),
                black_box(1),
            )
        });
    });
}

fn bench_hmac_sha256(c: &mut Criterion) {
    let key = b"webhook_secret_key_production_long_secret";
    let payload = br#"{"event":"listing.created","id":"uuid-1234","user_id":"uuid-5678","timestamp":1724790000}"#;

    c.bench_function("hmac_sha256_hex", |bencher| {
        bencher.iter(|| hmac_sha256(black_box(key), black_box(payload)));
    });
}

fn bench_constant_time_compare(c: &mut Criterion) {
    let a = b"e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
    let b = b"e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

    c.bench_function("constant_time_compare_64bytes", |bencher| {
        bencher.iter(|| constant_time_compare(black_box(a), black_box(b)));
    });
}

criterion_group!(
    benches,
    bench_totp_generation,
    bench_totp_verification,
    bench_hmac_sha256,
    bench_constant_time_compare
);
criterion_main!(benches);
