mod support;

use std::str::FromStr;

use big_number::{BigFixed, BigUintCore, RoundingMode};
use num_bigint::BigUint;

use support::*;

#[test]
fn biguint_mul_handles_multi_limb_result() {
    let left = BigUintCore::from_u64(u64::MAX);
    let right = BigUintCore::from_u64(u64::MAX);

    let product = left.mul(&right);
    let expected = (BigUint::from(u64::MAX) * BigUint::from(u64::MAX)).to_str_radix(10);
    assert_eq!(product.to_decimal_string(), expected);
}

#[test]
fn fixed_multiplication_preserves_scale() {
    let left = BigFixed::<2>::from_str("12.34").unwrap();
    let right = BigFixed::<2>::from_str("2.00").unwrap();

    assert_eq!((left * right).to_string(), oracle_mul::<2>("12.34", "2.00"));
}

#[test]
fn fixed_multiplication_uses_half_even_rescaling() {
    let even_down = BigFixed::<2>::from_str("0.25").unwrap();
    let odd_up = BigFixed::<2>::from_str("0.15").unwrap();

    assert_eq!(
        (even_down.clone() * even_down).to_string(),
        oracle_mul::<2>("0.25", "0.25")
    );
    assert_eq!(
        (odd_up.clone() * odd_up).to_string(),
        oracle_mul::<2>("0.15", "0.15")
    );
    assert_eq!(
        (BigFixed::<2>::from_str("-1.25").unwrap() * BigFixed::<2>::from_str("2.00").unwrap())
            .to_string(),
        oracle_mul::<2>("-1.25", "2.00")
    );
}

#[test]
fn biguint_div_rem_handles_multi_limb_values() {
    let dividend = BigUintCore::from_decimal_digits("18446744073709551616").unwrap();
    let divisor = BigUintCore::from_u64(3);

    let (quotient, remainder) = dividend.div_rem(&divisor);
    let expected_dividend = parse_biguint("18446744073709551616");
    let expected_divisor = BigUint::from(3_u8);
    assert_eq!(
        (&expected_dividend / &expected_divisor).to_str_radix(10),
        quotient.to_decimal_string()
    );
    assert_eq!(
        (&expected_dividend % &expected_divisor).to_str_radix(10),
        remainder.to_decimal_string()
    );
}

#[test]
fn biguint_div_rem_uses_u64_fast_path_consistently() {
    let dividend =
        BigUintCore::from_decimal_digits("340282366920938463463374607431768211455").unwrap();
    let divisor = BigUintCore::from_u64(u64::MAX - 58);

    let (quotient, remainder) = dividend.div_rem(&divisor);
    let expected_dividend = parse_biguint("340282366920938463463374607431768211455");
    let expected_divisor = BigUint::from(u64::MAX - 58);

    assert_eq!(
        (&expected_dividend / &expected_divisor).to_str_radix(10),
        quotient.to_decimal_string()
    );
    assert_eq!(
        (&expected_dividend % &expected_divisor).to_str_radix(10),
        remainder.to_decimal_string()
    );
}

#[test]
fn biguint_div_rem_handles_multi_limb_divisor() {
    let dividend =
        BigUintCore::from_decimal_digits("1234567890123456789012345678901234567890").unwrap();
    let divisor = BigUintCore::from_decimal_digits("98765432109876543210").unwrap();

    let (quotient, remainder) = dividend.div_rem(&divisor);
    let expected_dividend = parse_biguint("1234567890123456789012345678901234567890");
    let expected_divisor = parse_biguint("98765432109876543210");

    assert_eq!(
        (&expected_dividend / &expected_divisor).to_str_radix(10),
        quotient.to_decimal_string()
    );
    assert_eq!(
        (&expected_dividend % &expected_divisor).to_str_radix(10),
        remainder.to_decimal_string()
    );
}

#[test]
fn fixed_division_preserves_scale() {
    let left = BigFixed::<2>::from_str("10.00").unwrap();
    let right = BigFixed::<2>::from_str("4.00").unwrap();

    assert_eq!((left / right).to_string(), oracle_div::<2>("10.00", "4.00"));
}

#[test]
fn fixed_division_uses_half_even_rounding() {
    let half_even_down = BigFixed::<2>::from_str("1.00").unwrap();
    let three = BigFixed::<2>::from_str("3.00").unwrap();
    let eight = BigFixed::<2>::from_str("8.00").unwrap();

    assert_eq!(
        (half_even_down.clone() / eight).to_string(),
        oracle_div::<2>("1.00", "8.00")
    );
    assert_eq!(
        (half_even_down / three).to_string(),
        oracle_div::<2>("1.00", "3.00")
    );
}

#[test]
fn fixed_checked_div_reports_division_by_zero() {
    let value = BigFixed::<2>::from_str("1.00").unwrap();
    let zero = BigFixed::<2>::zero();

    assert_eq!(value.checked_div(&zero), None);
}

#[test]
fn fixed_recip_matches_oracle() {
    let value = BigFixed::<2>::from_str("8.00").unwrap();

    assert_eq!(
        value.recip().to_string(),
        oracle_recip_with_rounding::<2>("8.00", RoundingMode::HalfEven).unwrap()
    );
}

#[test]
fn fixed_recip_accepts_math_context_rounding() {
    let value = BigFixed::<2>::from_str("8.00").unwrap();

    assert_eq!(
        value
            .checked_recip_with_context(ceil_context())
            .unwrap()
            .to_string(),
        oracle_recip_with_rounding::<2>("8.00", RoundingMode::Ceil).unwrap()
    );
}

#[test]
fn fixed_checked_recip_rejects_zero() {
    let zero = BigFixed::<2>::zero();

    assert_eq!(zero.checked_recip(), None);
}

#[test]
fn fixed_division_accepts_math_context_rounding() {
    let value = BigFixed::<2>::from_str("1.00").unwrap();
    let divisor = BigFixed::<2>::from_str("8.00").unwrap();

    assert_eq!(
        value
            .checked_div_with_context(&divisor, ceil_context())
            .unwrap()
            .to_string(),
        oracle_div_with_rounding::<2>("1.00", "8.00", RoundingMode::Ceil)
    );
}

#[test]
fn fixed_rescale_up_preserves_value_exactly() {
    let value = BigFixed::<2>::from_str("12.34").unwrap();

    assert_eq!(
        value.rescale::<4>().to_string(),
        oracle_rescale::<2, 4>("12.34")
    );
}

#[test]
fn fixed_rescale_down_uses_default_half_even() {
    let value = BigFixed::<4>::from_str("1.2350").unwrap();

    assert_eq!(
        value.rescale::<2>().to_string(),
        oracle_rescale::<4, 2>("1.2350")
    );
}

#[test]
fn fixed_rescale_accepts_math_context_rounding() {
    let value = BigFixed::<3>::from_str("1.231").unwrap();

    assert_eq!(
        value.rescale_with_context::<2>(ceil_context()).to_string(),
        oracle_rescale_with_rounding::<3, 2>("1.231", RoundingMode::Ceil)
    );
}
