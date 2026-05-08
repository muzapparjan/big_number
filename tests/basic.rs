mod support;

use std::str::FromStr;

use big_number::{BigFixed, BigIntCore, BigUintCore, ParseBigFixedError, Sign};
use num_bigint::{BigInt, BigUint};
use num_traits::Signed;

use support::*;

#[test]
fn biguint_add_handles_carry_across_limbs() {
    let left = BigUintCore::from_u64(u64::MAX);
    let right = BigUintCore::from_u64(1);

    let sum = left.add(&right);
    let expected = (BigUint::from(u64::MAX) + BigUint::from(1_u8)).to_str_radix(10);
    assert_eq!(sum.to_decimal_string(), expected);
}

#[test]
fn bigint_add_and_sub_respect_signs() {
    let positive = BigIntCore::from_i64(125);
    let negative = BigIntCore::from_i64(-40);

    let expected_add = BigInt::from(125) + BigInt::from(-40);
    let expected_sub = BigInt::from(-40) - BigInt::from(125);

    assert_eq!(positive.add(&negative).sign(), Sign::Positive);
    assert_eq!(
        positive.add(&negative).magnitude().to_decimal_string(),
        expected_add.abs().to_str_radix(10)
    );
    assert_eq!(negative.sub(&positive).sign(), Sign::Negative);
    assert_eq!(
        negative.sub(&positive).magnitude().to_decimal_string(),
        expected_sub.abs().to_str_radix(10)
    );
}

#[test]
fn fixed_parse_and_display_preserve_scale() {
    let value = BigFixed::<4>::from_str("123.45").unwrap();
    assert_eq!(value.to_string(), "123.4500");
    assert_eq!(value.to_trimmed_string(), "123.45");
}

#[test]
fn fixed_parse_normalizes_negative_zero() {
    let value = BigFixed::<3>::from_str("-0.000").unwrap();
    assert!(value.is_zero());
    assert_eq!(value.to_string(), "0.000");
    assert_eq!(value.mantissa().sign(), Sign::Zero);
}

#[test]
fn fixed_addition_and_subtraction_work() {
    let left = BigFixed::<2>::from_str("12.34").unwrap();
    let right = BigFixed::<2>::from_str("-2.50").unwrap();
    let expected_sum =
        format_scaled::<2>(&(parse_scaled::<2>("12.34") + parse_scaled::<2>("-2.50")));
    let expected_diff =
        format_scaled::<2>(&(parse_scaled::<2>("12.34") - parse_scaled::<2>("-2.50")));

    assert_eq!((left.clone() + right.clone()).to_string(), expected_sum);
    assert_eq!((left - right).to_string(), expected_diff);
}

#[test]
fn fixed_ordering_uses_numeric_value() {
    let left = BigFixed::<3>::from_str("1.001").unwrap();
    let right = BigFixed::<3>::from_str("1.010").unwrap();
    assert!(left < right);
}

#[test]
fn fixed_rejects_too_many_fractional_digits() {
    let error = BigFixed::<2>::from_str("1.234").unwrap_err();
    assert_eq!(
        error,
        ParseBigFixedError::FractionalDigitsExceedScale { found: 3, scale: 2 }
    );
}

#[test]
fn fixed_integer_rounding_variants_work() {
    let positive = BigFixed::<2>::from_str("12.50").unwrap();
    let negative = BigFixed::<2>::from_str("-12.25").unwrap();
    let positive_expected = oracle_integer_round::<2>("12.50");
    let negative_expected = oracle_integer_round::<2>("-12.25");

    assert_eq!(positive.trunc().to_string(), positive_expected.0);
    assert_eq!(positive.ceil().to_string(), positive_expected.2);
    assert_eq!(negative.floor().to_string(), negative_expected.1);
    assert_eq!(negative.trunc().to_string(), negative_expected.0);
}

#[test]
fn fixed_round_defaults_to_half_even() {
    let half_even_down = BigFixed::<2>::from_str("2.50").unwrap();
    let half_even_up = BigFixed::<2>::from_str("3.50").unwrap();
    let negative = BigFixed::<2>::from_str("-2.51").unwrap();

    assert_eq!(
        half_even_down.round().to_string(),
        oracle_integer_round::<2>("2.50").3
    );
    assert_eq!(
        half_even_up.round().to_string(),
        oracle_integer_round::<2>("3.50").3
    );
    assert_eq!(
        negative.round().to_string(),
        oracle_integer_round::<2>("-2.51").3
    );
}
