use std::str::FromStr;

use big_number::{BigFixed, MathContext, RoundingMode};
use num_bigfloat::BigFloat;

fn oracle(value: &str) -> BigFloat {
    BigFloat::parse(value).unwrap()
}

fn actual_float<const SCALE: u32>(value: &BigFixed<SCALE>) -> BigFloat {
    BigFloat::parse(&value.to_string()).unwrap()
}

fn tolerance<const SCALE: u32>() -> BigFloat {
    let mut text = String::from("0.");
    for _ in 0..SCALE.saturating_sub(1) {
        text.push('0');
    }
    text.push('1');
    BigFloat::parse(&text).unwrap()
}

fn multiple_tolerance<const SCALE: u32>(multiplier: u32) -> BigFloat {
    tolerance::<SCALE>().mul(&BigFloat::from_u32(multiplier))
}

fn format_scaled_i64<const SCALE: u32>(scaled: i64) -> String {
    let negative = scaled.is_negative();
    let digits = scaled.unsigned_abs().to_string();
    if SCALE == 0 {
        return if negative {
            format!("-{digits}")
        } else {
            digits
        };
    }

    let width = SCALE as usize + 1;
    let padded = if digits.len() < width {
        let mut buffer = String::with_capacity(width);
        for _ in 0..(width - digits.len()) {
            buffer.push('0');
        }
        buffer.push_str(&digits);
        buffer
    } else {
        digits
    };

    let split = padded.len() - SCALE as usize;
    if negative {
        format!("-{}.{}", &padded[..split], &padded[split..])
    } else {
        format!("{}.{}", &padded[..split], &padded[split..])
    }
}

fn lcg_next(state: &mut u64) -> u64 {
    *state = state
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    *state
}

fn assert_close<const SCALE: u32>(actual: BigFixed<SCALE>, expected: BigFloat) {
    let actual_value = actual_float(&actual);
    let diff = expected.sub(&actual_value).abs();
    let tol = tolerance::<SCALE>();
    assert!(
        diff <= tol,
        "expected {} within {}, got {} (diff {})",
        expected,
        tol,
        actual,
        diff
    );
}

fn assert_fixed_close<const SCALE: u32>(left: BigFixed<SCALE>, right: BigFixed<SCALE>, ulps: u32) {
    let left_value = actual_float(&left);
    let right_value = actual_float(&right);
    let diff = left_value.sub(&right_value).abs();
    let tol = multiple_tolerance::<SCALE>(ulps);
    assert!(
        diff <= tol,
        "expected {} within {}, got {} (diff {})",
        left,
        tol,
        right,
        diff
    );
}

#[test]
fn fixed_ln_and_log10_match_high_precision_oracle() {
    let values = ["0.125000", "0.500000", "2.000000", "10.000000", "42.500000"];
    let ln_ten = oracle("10").ln();

    for value in values {
        let input = BigFixed::<6>::from_str(value).unwrap();
        let ln_expected = oracle(value).ln();
        assert_close(input.ln(), ln_expected);

        let log_expected = oracle(value).ln().div(&ln_ten);
        assert_close(input.log10(), log_expected);
    }
}

#[test]
fn fixed_exp_matches_high_precision_oracle() {
    let values = [
        "-2.000000",
        "-0.500000",
        "0.000000",
        "0.500000",
        "1.000000",
        "2.000000",
    ];

    for value in values {
        let input = BigFixed::<6>::from_str(value).unwrap();
        assert_close(input.exp(), oracle(value).exp());
    }
}

#[test]
fn fixed_trigonometric_functions_match_high_precision_oracle() {
    let values = ["-3.000000", "-0.500000", "0.500000", "1.250000", "6.000000"];

    for value in values {
        let input = BigFixed::<6>::from_str(value).unwrap();
        let oracle_value = oracle(value);
        assert_close(input.sin(), oracle_value.sin());
        assert_close(input.cos(), oracle_value.cos());
        assert_close(input.tan(), oracle_value.tan());
    }
}

#[test]
fn transcendental_functions_accept_math_context_rounding() {
    let context = MathContext {
        rounding: RoundingMode::Ceil,
        guard_digits: 12,
    };
    let input = BigFixed::<4>::from_str("2.0000").unwrap();
    let oracle_value = oracle("2.0000").ln();

    let actual = input.checked_ln_with_context(context).unwrap();
    let actual_value = actual_float(&actual);
    let diff = actual_value.sub(&oracle_value).abs();
    assert!(actual_value >= oracle_value);
    assert!(diff <= tolerance::<4>());
}

#[test]
fn transcendental_functions_reject_invalid_domains() {
    let zero = BigFixed::<6>::zero();
    let negative = BigFixed::<6>::from_str("-1.000000").unwrap();
    let pole = BigFixed::<12>::from_str("1.570796326795").unwrap();

    assert!(zero.checked_ln().is_none());
    assert!(negative.checked_ln().is_none());
    assert!(zero.checked_log10().is_none());
    assert!(negative.checked_log10().is_none());
    assert!(pole.checked_tan().is_none());
}

#[test]
fn exp_ln_roundtrip_preserves_positive_values() {
    let values = ["0.125000", "0.500000", "1.250000", "2.000000", "25.500000"];

    for value in values {
        let input = BigFixed::<6>::from_str(value).unwrap();
        let roundtrip = input.ln().exp();
        assert_fixed_close(roundtrip, input, 16);
    }
}

#[test]
fn sin_cos_identity_stays_close_to_one() {
    let values = [
        "-9.250000",
        "-3.000000",
        "-0.500000",
        "0.500000",
        "2.750000",
        "12.000000",
    ];
    let one = oracle("1");

    for value in values {
        let input = BigFixed::<6>::from_str(value).unwrap();
        let sine = input.sin();
        let cosine = input.cos();
        let identity = actual_float(&sine)
            .mul(&actual_float(&sine))
            .add(&actual_float(&cosine).mul(&actual_float(&cosine)));
        let diff = identity.sub(&one).abs();

        assert!(
            diff <= multiple_tolerance::<6>(4),
            "sin^2 + cos^2 drifted for {}: {}",
            value,
            diff
        );
    }
}

#[test]
fn additional_guard_digits_keep_results_stable() {
    let inputs = ["0.125000", "1.500000", "2.000000", "6.250000"];
    let low = MathContext {
        rounding: RoundingMode::HalfEven,
        guard_digits: 4,
    };
    let high = MathContext {
        rounding: RoundingMode::HalfEven,
        guard_digits: 16,
    };

    for value in inputs {
        let input = BigFixed::<6>::from_str(value).unwrap();

        let ln_low = input.checked_ln_with_context(low).unwrap();
        let ln_high = input.checked_ln_with_context(high).unwrap();
        assert_fixed_close(ln_low, ln_high, 1);

        let exp_low = input.checked_exp_with_context(low).unwrap();
        let exp_high = input.checked_exp_with_context(high).unwrap();
        assert_fixed_close(exp_low, exp_high, 1);

        let sin_low = input.checked_sin_with_context(low).unwrap();
        let sin_high = input.checked_sin_with_context(high).unwrap();
        assert_fixed_close(sin_low, sin_high, 1);
    }
}

#[test]
fn deterministic_positive_sweep_matches_ln_log10_and_exp_oracles() {
    let mut state = 0x5eed_cafe_u64;

    for _ in 0..24 {
        let raw = (lcg_next(&mut state) % 49_900_000) as i64 + 100_000;
        let value = format_scaled_i64::<6>(raw);
        let input = BigFixed::<6>::from_str(&value).unwrap();

        assert_close(input.ln(), oracle(&value).ln());
        assert_close(input.log10(), oracle(&value).ln().div(&oracle("10").ln()));

        let exp_source = format_scaled_i64::<6>((raw % 4_000_000) - 2_000_000);
        let exp_input = BigFixed::<6>::from_str(&exp_source).unwrap();
        assert_close(exp_input.exp(), oracle(&exp_source).exp());
    }
}

#[test]
fn deterministic_trigonometric_sweep_matches_oracles() {
    let mut state = 0x1234_5678_9abc_def0_u64;

    for _ in 0..32 {
        let raw = (lcg_next(&mut state) % 40_000_000) as i64 - 20_000_000;
        let value = format_scaled_i64::<6>(raw);
        let input = BigFixed::<6>::from_str(&value).unwrap();
        let oracle_value = oracle(&value);

        assert_close(input.sin(), oracle_value.sin());
        assert_close(input.cos(), oracle_value.cos());

        if let Some(tan) = input.checked_tan() {
            assert_close(tan, oracle_value.tan());
        }
    }
}
