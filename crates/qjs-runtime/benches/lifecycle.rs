//! Public-boundary parser/compiler lifecycle diagnostics.

use std::hint::black_box;
use std::time::Duration;

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use qjs_parser::parse_script;
use qjs_runtime::compile_script;

struct Fixture {
    id: &'static str,
    source: &'static str,
    expected_bytes: usize,
    expected_fnv1a64: u64,
}

const FIXTURES: &[Fixture] = &[
    Fixture {
        id: "small-v1",
        source: include_str!("fixtures/small-v1.js"),
        expected_bytes: 553,
        expected_fnv1a64: 0x834b_63ad_0ede_94c3,
    },
    Fixture {
        id: "medium-v1",
        source: include_str!("fixtures/medium-v1.js"),
        expected_bytes: 1_644,
        expected_fnv1a64: 0x96df_4a20_e8f1_9e57,
    },
];

fn fnv1a64(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(0x0000_0100_0000_01b3)
    })
}

fn verify_fixture(fixture: &Fixture) {
    assert_eq!(
        fixture.source.len(),
        fixture.expected_bytes,
        "{} length changed; bump the fixture version and sentinel",
        fixture.id,
    );
    assert_eq!(
        fnv1a64(fixture.source.as_bytes()),
        fixture.expected_fnv1a64,
        "{} content changed; bump the fixture version and sentinel",
        fixture.id,
    );
}

fn lifecycle_benches(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("lifecycle");

    for fixture in FIXTURES {
        verify_fixture(fixture);
        let bytes = fixture.source.len() as u64;
        group.throughput(Throughput::Bytes(bytes));

        group.bench_with_input(
            BenchmarkId::new("parse", fixture.id),
            fixture.source,
            |bencher, source| {
                bencher.iter_with_large_drop(|| {
                    let script = parse_script(black_box(source))
                        .expect("versioned lifecycle fixture must parse");
                    black_box(script)
                });
            },
        );

        let parsed = parse_script(fixture.source).expect("versioned lifecycle fixture must parse");
        group.bench_with_input(
            BenchmarkId::new("compile", fixture.id),
            &parsed,
            |bencher, script| {
                bencher.iter_with_large_drop(|| {
                    let bytecode = compile_script(black_box(script))
                        .expect("versioned lifecycle fixture must compile");
                    black_box(bytecode)
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("parse_and_compile", fixture.id),
            fixture.source,
            |bencher, source| {
                bencher.iter_with_large_drop(|| {
                    let script = parse_script(black_box(source))
                        .expect("versioned lifecycle fixture must parse");
                    let bytecode = compile_script(black_box(&script))
                        .expect("versioned lifecycle fixture must compile");
                    black_box((script, bytecode))
                });
            },
        );
    }

    group.finish();
}

fn criterion_config() -> Criterion {
    Criterion::default()
        .sample_size(50)
        .warm_up_time(Duration::from_secs(2))
        .measurement_time(Duration::from_secs(5))
        .confidence_level(0.95)
        .noise_threshold(0.02)
}

criterion_group! {
    name = lifecycle;
    config = criterion_config();
    targets = lifecycle_benches
}
criterion_main!(lifecycle);
