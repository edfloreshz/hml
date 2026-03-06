use std::fs;
use std::path::Path;

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use hml::compile;

fn load_fixture(name: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("examples")
        .join(name);

    fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("failed to read fixture '{}': {}", path.display(), error))
}

fn bench_compile_fixture(c: &mut Criterion, fixture_name: &str) {
    let source = load_fixture(fixture_name);
    let bytes = source.len() as u64;

    let mut group = c.benchmark_group("compile_fixture");
    group.throughput(Throughput::Bytes(bytes));

    group.bench_with_input(
        BenchmarkId::from_parameter(fixture_name),
        &source,
        |b, source| {
            b.iter(|| {
                let result = compile(source, fixture_name);
                assert!(
                    result.diagnostics.iter().count() == 0,
                    "fixture '{}' produced diagnostics",
                    fixture_name
                );
                assert!(
                    result.is_success(),
                    "fixture '{}' failed to compile",
                    fixture_name
                );
            });
        },
    );

    group.finish();
}

fn bench_compile_all_examples(c: &mut Criterion) {
    let fixtures = [
        "article.hml",
        "blog.hml",
        "contact.hml",
        "dashboard.hml",
        "gallery.hml",
        "landing.hml",
    ];

    let sources: Vec<(&str, String)> = fixtures
        .iter()
        .map(|name| (*name, load_fixture(name)))
        .collect();

    let total_bytes = sources.iter().map(|(_, source)| source.len() as u64).sum();

    let mut group = c.benchmark_group("compile_suite");
    group.throughput(Throughput::Bytes(total_bytes));

    group.bench_function("all_examples", |b| {
        b.iter(|| {
            for (name, source) in &sources {
                let result = compile(source, *name);
                assert!(
                    result.diagnostics.iter().count() == 0,
                    "fixture '{}' produced diagnostics",
                    name
                );
                assert!(result.is_success(), "fixture '{}' failed to compile", name);
            }
        });
    });

    group.finish();
}

fn compiler_benchmarks(c: &mut Criterion) {
    for fixture in [
        "article.hml",
        "blog.hml",
        "contact.hml",
        "dashboard.hml",
        "gallery.hml",
        "landing.hml",
    ] {
        bench_compile_fixture(c, fixture);
    }

    bench_compile_all_examples(c);
}

criterion_group!(benches, compiler_benchmarks);
criterion_main!(benches);
