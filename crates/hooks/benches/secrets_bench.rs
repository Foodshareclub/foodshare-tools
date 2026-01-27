use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use foodshare_hooks::{SecretScanner, Severity, PatternDef, PatternCategory};

const SAMPLE_CONTENT: &str = r#"
# Configuration
AWS_ACCESS_KEY_ID=AKIAIOSFODNN7EXAMPLE
AWS_SECRET_ACCESS_KEY=wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY
GITHUB_TOKEN=ghp_xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx
DATABASE_URL=postgres://user:password@localhost:5432/db
STRIPE_SECRET_KEY=sk_test_EXAMPLEKEYDONOTUSE12345678
SENDGRID_API_KEY=SG.abcdefghij12345678901a.abcdefghijklmnopqrstuvwxyz12345678901234567
GOOGLE_API_KEY=AIzaSyD-9tSrke72PouQMnMX-a7eZSW0jkFMBWY
NPM_TOKEN=npm_abcdefghijklmnopqrstuvwxyz0123456789

# Safe content
DEBUG=true
LOG_LEVEL=info
APP_NAME=foodshare
"#;

fn bench_scan_str(c: &mut Criterion) {
    let scanner = SecretScanner::new();

    c.bench_function("scan_str_mixed", |b| {
        b.iter(|| scanner.scan_str(black_box(SAMPLE_CONTENT), "test.env"))
    });
}

fn bench_scanner_high_severity(c: &mut Criterion) {
    let scanner = SecretScanner::new().min_severity(Severity::High);

    c.bench_function("scanner_high_severity", |b| {
        b.iter(|| scanner.scan_str(black_box(SAMPLE_CONTENT), "test.env"))
    });
}

fn bench_clean_content(c: &mut Criterion) {
    let scanner = SecretScanner::new();
    let clean = "DEBUG=true\nLOG_LEVEL=info\n".repeat(100);

    c.bench_function("scan_clean_content", |b| {
        b.iter(|| scanner.scan_str(black_box(&clean), "test.env"))
    });
}

fn bench_entropy_detection(c: &mut Criterion) {
    let scanner = SecretScanner::new().with_entropy_detection();
    let content = r#"
SECRET_KEY=aB3xY9mK2pQwE8rT5nZvL4cGhJk
RANDOM_TOKEN=Xy7mN3pK9qW2eR8tY5uI0oP6aS1dF
ANOTHER_SECRET=Qw3rTy7Ui0pAs2Df5Gh8Jk1Lz4Xc
DEBUG=true
NORMAL_VALUE=hello_world
"#;

    c.bench_function("scan_with_entropy", |b| {
        b.iter(|| scanner.scan_str(black_box(content), "test.env"))
    });
}

fn bench_custom_patterns(c: &mut Criterion) {
    let scanner = SecretScanner::new()
        .add_pattern(PatternDef {
            id: "custom-1".into(),
            name: "Custom Pattern 1".into(),
            pattern: r"CUSTOM_[A-Z]{20}".into(),
            severity: Severity::Medium,
            category: PatternCategory::Custom,
            description: String::new(),
            enabled: true,
        })
        .add_pattern(PatternDef {
            id: "custom-2".into(),
            name: "Custom Pattern 2".into(),
            pattern: r"SECRET_[0-9]{10}".into(),
            severity: Severity::Medium,
            category: PatternCategory::Custom,
            description: String::new(),
            enabled: true,
        });

    c.bench_function("scan_with_custom_patterns", |b| {
        b.iter(|| scanner.scan_str(black_box(SAMPLE_CONTENT), "test.env"))
    });
}

fn bench_with_exclusions(c: &mut Criterion) {
    let scanner = SecretScanner::new()
        .exclude_pattern("noqa")
        .exclude_pattern("pragma: allowlist")
        .allowlist_value("EXAMPLE");

    c.bench_function("scan_with_exclusions", |b| {
        b.iter(|| scanner.scan_str(black_box(SAMPLE_CONTENT), "test.env"))
    });
}

fn bench_scaling(c: &mut Criterion) {
    let scanner = SecretScanner::new();

    let mut group = c.benchmark_group("scaling");
    for size in [10, 100, 1000].iter() {
        let content = SAMPLE_CONTENT.repeat(*size);
        group.bench_with_input(BenchmarkId::from_parameter(size), &content, |b, content| {
            b.iter(|| scanner.scan_str(black_box(content), "test.env"))
        });
    }
    group.finish();
}

fn bench_finding_callback(c: &mut Criterion) {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    let count = Arc::new(AtomicUsize::new(0));
    let count_clone = count.clone();

    let scanner = SecretScanner::new()
        .on_finding(move |_| {
            count_clone.fetch_add(1, Ordering::Relaxed);
        });

    c.bench_function("scan_with_callback", |b| {
        b.iter(|| {
            count.store(0, Ordering::Relaxed);
            scanner.scan_str(black_box(SAMPLE_CONTENT), "test.env")
        })
    });
}

criterion_group!(
    benches,
    bench_scan_str,
    bench_scanner_high_severity,
    bench_clean_content,
    bench_entropy_detection,
    bench_custom_patterns,
    bench_with_exclusions,
    bench_scaling,
    bench_finding_callback,
);
criterion_main!(benches);
