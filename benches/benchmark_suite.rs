use std::str::FromStr;

use big_number::{BigFixed, BigUintCore, MathContext, RoundingMode};
use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};

fn parse_biguint(value: &str) -> BigUintCore {
    BigUintCore::from_decimal_digits(value).unwrap()
}

fn parse_fixed<const SCALE: u32>(value: &str) -> BigFixed<SCALE> {
    BigFixed::<SCALE>::from_str(value).unwrap()
}

fn bench_biguint_core(c: &mut Criterion) {
    let mut group = c.benchmark_group("biguint_core");

    let small_a = BigUintCore::from_u64(123_456_789);
    let small_b = BigUintCore::from_u64(987_654_321);
    group.throughput(Throughput::Elements(1));
    group.bench_function("mul_inline_small", |b| {
        b.iter(|| black_box(&small_a).mul(black_box(&small_b)))
    });

    let medium_a = parse_biguint("1234567890123456789012345678901234567890");
    let medium_b = parse_biguint("9988776655443322110099887766554433221100");
    group.bench_function("mul_medium_decimal", |b| {
        b.iter(|| black_box(&medium_a).mul(black_box(&medium_b)))
    });
    group.bench_function("div_medium_decimal", |b| {
        b.iter(|| black_box(&medium_b).div_rem(black_box(&medium_a)))
    });

    let large =
        parse_biguint("1234567890123456789012345678901234567890123456789012345678901234567890");
    group.bench_function("sqrt_large_decimal", |b| {
        b.iter(|| black_box(&large).sqrt_rem())
    });

    group.finish();
}

fn bench_fixed_exact(c: &mut Criterion) {
    let mut group = c.benchmark_group("fixed_exact");

    let lhs = parse_fixed::<6>("12345.678901");
    let rhs = parse_fixed::<6>("98765.432109");
    group.throughput(Throughput::Elements(1));
    group.bench_function("mul_scale_6", |b| {
        b.iter(|| black_box(lhs.clone()) * black_box(rhs.clone()))
    });
    group.bench_function("div_scale_6", |b| {
        b.iter(|| black_box(&lhs).checked_div(black_box(&rhs)).unwrap())
    });

    let sqrt_input = parse_fixed::<6>("2.000000");
    group.bench_function("sqrt_scale_6", |b| {
        b.iter(|| black_box(&sqrt_input).checked_sqrt().unwrap())
    });

    let root_input = parse_fixed::<6>("12345.678901");
    group.bench_function("nth_root_5_scale_6", |b| {
        b.iter(|| black_box(&root_input).checked_nth_root(5).unwrap())
    });

    group.finish();
}

fn bench_transcendentals(c: &mut Criterion) {
    let mut group = c.benchmark_group("transcendentals_guard_digits");
    let ln_input = parse_fixed::<6>("12345.678901");
    let exp_input = parse_fixed::<6>("1.250000");
    let sin_input = parse_fixed::<6>("12.000000");

    for guard_digits in [4_u32, 8, 12, 16] {
        let context = MathContext {
            rounding: RoundingMode::HalfEven,
            guard_digits,
        };

        group.bench_with_input(
            BenchmarkId::new("ln", guard_digits),
            &context,
            |b, context| {
                b.iter(|| {
                    black_box(&ln_input)
                        .checked_ln_with_context(*context)
                        .unwrap()
                })
            },
        );

        group.bench_with_input(
            BenchmarkId::new("exp", guard_digits),
            &context,
            |b, context| {
                b.iter(|| {
                    black_box(&exp_input)
                        .checked_exp_with_context(*context)
                        .unwrap()
                })
            },
        );

        group.bench_with_input(
            BenchmarkId::new("sin", guard_digits),
            &context,
            |b, context| {
                b.iter(|| {
                    black_box(&sin_input)
                        .checked_sin_with_context(*context)
                        .unwrap()
                })
            },
        );
    }

    group.finish();
}

fn bench_transcendental_diagnostics(c: &mut Criterion) {
    let mut group = c.benchmark_group("transcendental_diagnostics");
    let ln_near_one = parse_fixed::<6>("1.031250");
    let ln_large = parse_fixed::<6>("12345.678901");
    let sin_small = parse_fixed::<6>("0.500000");
    let sin_large = parse_fixed::<6>("12.000000");

    for guard_digits in [4_u32, 8, 12, 16] {
        let context = MathContext {
            rounding: RoundingMode::HalfEven,
            guard_digits,
        };

        group.bench_with_input(
            BenchmarkId::new("ln_near_one", guard_digits),
            &context,
            |b, context| {
                b.iter(|| {
                    black_box(&ln_near_one)
                        .checked_ln_with_context(*context)
                        .unwrap()
                })
            },
        );

        group.bench_with_input(
            BenchmarkId::new("ln_large", guard_digits),
            &context,
            |b, context| {
                b.iter(|| {
                    black_box(&ln_large)
                        .checked_ln_with_context(*context)
                        .unwrap()
                })
            },
        );

        group.bench_with_input(
            BenchmarkId::new("sin_small", guard_digits),
            &context,
            |b, context| {
                b.iter(|| {
                    black_box(&sin_small)
                        .checked_sin_with_context(*context)
                        .unwrap()
                })
            },
        );

        group.bench_with_input(
            BenchmarkId::new("sin_large", guard_digits),
            &context,
            |b, context| {
                b.iter(|| {
                    black_box(&sin_large)
                        .checked_sin_with_context(*context)
                        .unwrap()
                })
            },
        );
    }

    group.finish();
}

fn bench_parsing_and_formatting(c: &mut Criterion) {
    let mut group = c.benchmark_group("parse_format");
    let parse_input = "12345678901234567890.123456";
    let format_input = parse_fixed::<6>(parse_input);

    group.bench_function("parse_scale_6", |b| {
        b.iter(|| BigFixed::<6>::from_str(black_box(parse_input)).unwrap())
    });
    group.bench_function("display_scale_6", |b| {
        b.iter(|| black_box(&format_input).to_string())
    });
    group.bench_function("trimmed_display_scale_6", |b| {
        b.iter(|| black_box(&format_input).to_trimmed_string())
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_biguint_core,
    bench_fixed_exact,
    bench_transcendentals,
    bench_transcendental_diagnostics,
    bench_parsing_and_formatting
);
criterion_main!(benches);
