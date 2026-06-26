//! Benchmark suite for query filtering and search operations.
//!
//! Uses Criterion.rs for statistically sound performance measurements.
//! Run with: cargo bench --workspace

use criterion::{black_box, criterion_group, criterion_main, Criterion};

// Include the BenchCanonicalData helper
mod bench_data;

/// Benchmark: create a Repository instance from scratch.
fn bench_create_repository(c: &mut Criterion) {
    c.bench_function("create_repository", |b| {
        b.iter(|| {
            let repo = bench_data::create_repo(
                "owner/repo",
                "Rust",
                5000,
                &["rust", "compiler", "systems"],
            );
            black_box(repo)
        })
    });
}

/// Benchmark: compute quality score.
fn bench_quality_score(c: &mut Criterion) {
    let repo = bench_data::create_repo("owner/repo", "Rust", 5000, &["rust"]);
    c.bench_function("quality_score", |b| {
        b.iter(|| {
            let score = bench_data::compute_quality_score(black_box(&repo));
            black_box(score)
        })
    });
}

/// Benchmark: classify activity status.
fn bench_activity_classification(c: &mut Criterion) {
    let repo = bench_data::create_repo("owner/repo", "Rust", 5000, &["rust"]);
    c.bench_function("activity_classification", |b| {
        b.iter(|| {
            let status = bench_data::classify_activity(black_box(&repo), 3, 12);
            black_box(status)
        })
    });
}

/// Benchmark: serialize repository to JSON.
fn bench_serialize_json(c: &mut Criterion) {
    let repo = bench_data::create_repo("owner/repo", "Rust", 5000, &["rust", "cli"]);
    c.bench_function("serialize_json", |b| {
        b.iter(|| {
            let json = serde_json::to_string(black_box(&repo)).unwrap();
            black_box(json)
        })
    });
}

/// Benchmark: deserialize repository from JSON.
fn bench_deserialize_json(c: &mut Criterion) {
    let repo = bench_data::create_repo("owner/repo", "Rust", 5000, &["rust", "cli"]);
    let json = serde_json::to_string(&repo).unwrap();
    c.bench_function("deserialize_json", |b| {
        b.iter(|| {
            let deserialized: rq_core::Repository = serde_json::from_str(black_box(&json)).unwrap();
            black_box(deserialized)
        })
    });
}

/// Benchmark: YAML round-trip serialization.
fn bench_yaml_roundtrip(c: &mut Criterion) {
    let repo = bench_data::create_repo("owner/repo", "Rust", 5000, &["rust", "cli"]);
    c.bench_function("yaml_roundtrip", |b| {
        b.iter(|| {
            let yaml = serde_yml::to_string(black_box(&repo)).unwrap();
            let deserialized: rq_core::Repository = serde_yml::from_str(&yaml).unwrap();
            black_box(deserialized)
        })
    });
}

/// Benchmark: filter repos from a batch.
fn bench_filter_repos(c: &mut Criterion) {
    let repos = bench_data::create_repo_batch(1000);
    c.bench_function("filter_repos_by_language", |b| {
        b.iter(|| {
            let filtered: Vec<_> = repos
                .iter()
                .filter(|r| r.metadata.primary_language == "Rust")
                .collect();
            black_box(filtered.len())
        })
    });
}

criterion_group!(
    benches,
    bench_create_repository,
    bench_quality_score,
    bench_activity_classification,
    bench_serialize_json,
    bench_deserialize_json,
    bench_yaml_roundtrip,
    bench_filter_repos,
);
criterion_main!(benches);
