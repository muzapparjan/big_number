mod support;

use std::str::FromStr;

use big_number::{BigFixed, BigUintCore, RoundingMode};

use support::*;

#[test]
fn fixed_powu_handles_zero_and_positive_exponents() {
    let value = BigFixed::<2>::from_str("1.50").unwrap();

    assert_eq!(value.powu(0).to_string(), oracle_powu::<2>("1.50", 0));
    assert_eq!(value.powu(1).to_string(), oracle_powu::<2>("1.50", 1));
    assert_eq!(value.powu(3).to_string(), oracle_powu::<2>("1.50", 3));
}

#[test]
fn fixed_powu_keeps_sign_for_odd_exponents() {
    let value = BigFixed::<2>::from_str("-2.00").unwrap();

    assert_eq!(value.powu(2).to_string(), oracle_powu::<2>("-2.00", 2));
    assert_eq!(value.powu(3).to_string(), oracle_powu::<2>("-2.00", 3));
}

#[test]
fn fixed_powi_supports_negative_exponents() {
    let value = BigFixed::<2>::from_str("2.00").unwrap();

    assert_eq!(
        value.powi(3).to_string(),
        oracle_powi::<2>("2.00", 3).unwrap()
    );
    assert_eq!(
        value.powi(-2).to_string(),
        oracle_powi::<2>("2.00", -2).unwrap()
    );
}

#[test]
fn fixed_checked_powi_rejects_zero_to_negative_power() {
    let zero = BigFixed::<2>::zero();

    assert_eq!(
        zero.checked_powi(-1),
        oracle_powi::<2>("0.00", -1).and_then(|s| BigFixed::<2>::from_str(&s).ok())
    );
}

#[test]
fn biguint_sqrt_rem_matches_oracle() {
    let value = BigUintCore::from_decimal_digits("18446744073709551616").unwrap();
    let (root, remainder) = value.sqrt_rem();

    let expected = parse_biguint("18446744073709551616");
    let expected_root = expected.sqrt();
    let expected_remainder = expected - (&expected_root * &expected_root);

    assert_eq!(root.to_decimal_string(), expected_root.to_str_radix(10));
    assert_eq!(
        remainder.to_decimal_string(),
        expected_remainder.to_str_radix(10)
    );
}

#[test]
fn fixed_sqrt_matches_oracle() {
    let value = BigFixed::<2>::from_str("2.00").unwrap();
    let perfect = BigFixed::<2>::from_str("2.25").unwrap();

    assert_eq!(value.sqrt().to_string(), oracle_sqrt::<2>("2.00").unwrap());
    assert_eq!(
        perfect.sqrt().to_string(),
        oracle_sqrt::<2>("2.25").unwrap()
    );
}

#[test]
fn fixed_checked_sqrt_rejects_negative_values() {
    let value = BigFixed::<2>::from_str("-1.00").unwrap();

    assert_eq!(
        value.checked_sqrt(),
        oracle_sqrt::<2>("-1.00").and_then(|s| BigFixed::<2>::from_str(&s).ok())
    );
}

#[test]
fn fixed_sqrt_accepts_math_context_rounding() {
    let value = BigFixed::<2>::from_str("2.00").unwrap();

    assert_eq!(
        value
            .checked_sqrt_with_context(ceil_context())
            .unwrap()
            .to_string(),
        oracle_sqrt_with_rounding::<2>("2.00", RoundingMode::Ceil).unwrap()
    );
}

#[test]
fn biguint_nth_root_rem_matches_oracle() {
    let value =
        BigUintCore::from_decimal_digits("340282366920938463463374607431768211455").unwrap();
    let (root, remainder) = value.nth_root_rem(3).unwrap();

    let expected = parse_biguint("340282366920938463463374607431768211455");
    let expected_root = expected.nth_root(3);
    let expected_remainder = expected - expected_root.pow(3);

    assert_eq!(root.to_decimal_string(), expected_root.to_str_radix(10));
    assert_eq!(
        remainder.to_decimal_string(),
        expected_remainder.to_str_radix(10)
    );
}

#[test]
fn fixed_nth_root_matches_oracle() {
    let value = BigFixed::<2>::from_str("8.00").unwrap();
    let negative = BigFixed::<2>::from_str("-8.00").unwrap();

    assert_eq!(
        value.nth_root(3).to_string(),
        oracle_nth_root_with_rounding::<2>("8.00", 3, RoundingMode::HalfEven).unwrap()
    );
    assert_eq!(
        negative.nth_root(3).to_string(),
        oracle_nth_root_with_rounding::<2>("-8.00", 3, RoundingMode::HalfEven).unwrap()
    );
}

#[test]
fn fixed_nth_root_accepts_math_context_rounding() {
    let value = BigFixed::<2>::from_str("2.00").unwrap();

    assert_eq!(
        value
            .checked_nth_root_with_context(3, ceil_context())
            .unwrap()
            .to_string(),
        oracle_nth_root_with_rounding::<2>("2.00", 3, RoundingMode::Ceil).unwrap()
    );
}

#[test]
fn fixed_checked_nth_root_rejects_invalid_domain() {
    let negative = BigFixed::<2>::from_str("-1.00").unwrap();

    assert_eq!(negative.checked_nth_root(2), None);
    assert_eq!(negative.checked_nth_root(0), None);
}
