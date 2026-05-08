#![allow(dead_code)]

use std::str::FromStr;

use big_number::{MathContext, RoundingMode};
use num_bigint::{BigInt, BigUint, Sign as NumSign};
use num_traits::{Signed, Zero};

pub fn parse_biguint(value: &str) -> BigUint {
    BigUint::from_str(value).unwrap()
}

pub fn parse_scaled<const SCALE: u32>(value: &str) -> BigInt {
    let trimmed = value.trim();
    let (negative, digits) = if let Some(rest) = trimmed.strip_prefix('-') {
        (true, rest)
    } else if let Some(rest) = trimmed.strip_prefix('+') {
        (false, rest)
    } else {
        (false, trimmed)
    };

    let mut parts = digits.split('.');
    let int_part = parts.next().unwrap_or("");
    let frac_part = parts.next().unwrap_or("");
    assert!(
        parts.next().is_none(),
        "oracle only handles simple decimals"
    );
    assert!(
        frac_part.len() <= SCALE as usize,
        "oracle input exceeds scale"
    );

    let mut combined = String::with_capacity(int_part.len() + SCALE as usize);
    combined.push_str(int_part);
    combined.push_str(frac_part);
    for _ in frac_part.len()..SCALE as usize {
        combined.push('0');
    }

    let magnitude = if combined.is_empty() {
        BigInt::zero()
    } else {
        BigInt::from_str(&combined).unwrap()
    };

    if negative && !magnitude.is_zero() {
        -magnitude
    } else {
        magnitude
    }
}

pub fn pow10(exp: u32) -> BigInt {
    BigInt::from(10_u8).pow(exp)
}

pub fn format_scaled<const SCALE: u32>(value: &BigInt) -> String {
    let sign_prefix = if value.sign() == NumSign::Minus {
        "-"
    } else {
        ""
    };
    let digits = value.abs().to_str_radix(10);

    if SCALE == 0 {
        return format!("{sign_prefix}{digits}");
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
    format!("{sign_prefix}{}.{}", &padded[..split], &padded[split..])
}

pub fn oracle_round_half_even(numerator: BigInt, denominator: BigInt) -> BigInt {
    let quotient = &numerator / &denominator;
    let remainder = &numerator % &denominator;
    if remainder.is_zero() {
        return quotient;
    }

    let abs_double_remainder = remainder.abs() * 2_u8;
    let abs_denominator = denominator.abs();
    if abs_double_remainder < abs_denominator {
        quotient
    } else if abs_double_remainder > abs_denominator {
        if numerator.sign() == denominator.sign() {
            quotient + 1
        } else {
            quotient - 1
        }
    } else if (&quotient & BigInt::from(1_u8)).is_zero() {
        quotient
    } else if numerator.sign() == denominator.sign() {
        quotient + 1
    } else {
        quotient - 1
    }
}

pub fn oracle_round_with_mode(
    numerator: BigInt,
    denominator: BigInt,
    rounding: RoundingMode,
) -> BigInt {
    let quotient = &numerator / &denominator;
    let remainder = &numerator % &denominator;
    if remainder.is_zero() {
        return quotient;
    }

    let same_sign = numerator.sign() == denominator.sign();

    match rounding {
        RoundingMode::Down => quotient,
        RoundingMode::Up => {
            if same_sign {
                quotient + 1
            } else {
                quotient - 1
            }
        }
        RoundingMode::Floor => {
            if same_sign {
                quotient
            } else {
                quotient - 1
            }
        }
        RoundingMode::Ceil => {
            if same_sign {
                quotient + 1
            } else {
                quotient
            }
        }
        RoundingMode::HalfUp => {
            let abs_double_remainder = remainder.abs() * 2_u8;
            let abs_denominator = denominator.abs();
            if abs_double_remainder >= abs_denominator {
                if same_sign {
                    quotient + 1
                } else {
                    quotient - 1
                }
            } else {
                quotient
            }
        }
        RoundingMode::HalfEven => oracle_round_half_even(numerator, denominator),
    }
}

pub fn oracle_mul<const SCALE: u32>(left: &str, right: &str) -> String {
    let lhs = parse_scaled::<SCALE>(left);
    let rhs = parse_scaled::<SCALE>(right);
    let factor = pow10(SCALE);
    let scaled = oracle_round_half_even(lhs * rhs, factor);
    format_scaled::<SCALE>(&scaled)
}

pub fn oracle_div<const SCALE: u32>(left: &str, right: &str) -> String {
    oracle_div_with_rounding::<SCALE>(left, right, RoundingMode::HalfEven)
}

pub fn oracle_div_with_rounding<const SCALE: u32>(
    left: &str,
    right: &str,
    rounding: RoundingMode,
) -> String {
    let lhs = parse_scaled::<SCALE>(left);
    let rhs = parse_scaled::<SCALE>(right);
    let scaled = oracle_round_with_mode(lhs * pow10(SCALE), rhs, rounding);
    format_scaled::<SCALE>(&scaled)
}

pub fn oracle_recip_with_rounding<const SCALE: u32>(
    value: &str,
    rounding: RoundingMode,
) -> Option<String> {
    if parse_scaled::<SCALE>(value).is_zero() {
        None
    } else {
        Some(oracle_div_with_rounding::<SCALE>("1", value, rounding))
    }
}

pub fn oracle_powu<const SCALE: u32>(value: &str, exponent: u32) -> String {
    let mut result = BigInt::from(1_u8) * pow10(SCALE);
    let base = parse_scaled::<SCALE>(value);
    let factor = pow10(SCALE);
    let mut power = base;
    let mut exp = exponent;

    while exp != 0 {
        if exp & 1 == 1 {
            result = oracle_round_half_even(result * &power, factor.clone());
        }
        exp >>= 1;
        if exp != 0 {
            power = oracle_round_half_even(&power * &power, factor.clone());
        }
    }

    format_scaled::<SCALE>(&result)
}

pub fn oracle_powi<const SCALE: u32>(value: &str, exponent: i32) -> Option<String> {
    if exponent >= 0 {
        return Some(oracle_powu::<SCALE>(value, exponent as u32));
    }

    let numerator = format_scaled::<SCALE>(&(BigInt::from(1_u8) * pow10(SCALE)));
    let denominator = oracle_powu::<SCALE>(value, exponent.unsigned_abs());
    if parse_scaled::<SCALE>(&denominator).is_zero() {
        None
    } else {
        Some(oracle_div::<SCALE>(&numerator, &denominator))
    }
}

pub fn oracle_sqrt<const SCALE: u32>(value: &str) -> Option<String> {
    oracle_sqrt_with_rounding::<SCALE>(value, RoundingMode::HalfEven)
}

pub fn oracle_sqrt_with_rounding<const SCALE: u32>(
    value: &str,
    rounding: RoundingMode,
) -> Option<String> {
    let scaled = parse_scaled::<SCALE>(value);
    if scaled.sign() == NumSign::Minus {
        return None;
    }

    let radicand = scaled * pow10(SCALE);
    let radicand_uint = radicand.to_biguint().unwrap();
    let floor = radicand_uint.sqrt();
    let floor_sq = &floor * &floor;
    let remainder = &radicand_uint - &floor_sq;
    let ceil = &floor + BigUint::from(1_u8);
    let scaled_remainder = &remainder * BigUint::from(4_u8);
    let midpoint = &floor * BigUint::from(4_u8) + BigUint::from(1_u8);
    let rounded = match rounding {
        RoundingMode::Down | RoundingMode::Floor => floor,
        RoundingMode::Up | RoundingMode::Ceil => {
            if remainder.is_zero() {
                floor
            } else {
                ceil
            }
        }
        RoundingMode::HalfUp => match scaled_remainder.cmp(&midpoint) {
            std::cmp::Ordering::Less => floor,
            std::cmp::Ordering::Greater | std::cmp::Ordering::Equal => ceil,
        },
        RoundingMode::HalfEven => match scaled_remainder.cmp(&midpoint) {
            std::cmp::Ordering::Less => floor,
            std::cmp::Ordering::Greater => ceil,
            std::cmp::Ordering::Equal => {
                if (&floor & BigUint::from(1_u8)).is_zero() {
                    floor
                } else {
                    ceil
                }
            }
        },
    };

    Some(format_scaled::<SCALE>(&BigInt::from(rounded)))
}

pub fn oracle_rescale<const FROM_SCALE: u32, const TO_SCALE: u32>(value: &str) -> String {
    oracle_rescale_with_rounding::<FROM_SCALE, TO_SCALE>(value, RoundingMode::HalfEven)
}

pub fn oracle_rescale_with_rounding<const FROM_SCALE: u32, const TO_SCALE: u32>(
    value: &str,
    rounding: RoundingMode,
) -> String {
    let scaled = parse_scaled::<FROM_SCALE>(value);

    let result = if TO_SCALE >= FROM_SCALE {
        scaled * pow10(TO_SCALE - FROM_SCALE)
    } else {
        oracle_round_with_mode(scaled, pow10(FROM_SCALE - TO_SCALE), rounding)
    };

    format_scaled::<TO_SCALE>(&result)
}

pub fn oracle_nth_root_with_rounding<const SCALE: u32>(
    value: &str,
    degree: u32,
    rounding: RoundingMode,
) -> Option<String> {
    if degree == 0 {
        return None;
    }

    let scaled = parse_scaled::<SCALE>(value);
    if scaled.sign() == NumSign::Minus && degree.is_multiple_of(2) {
        return None;
    }

    let negative = scaled.sign() == NumSign::Minus;
    let magnitude = scaled.abs();
    let radicand = magnitude * pow10(SCALE * (degree - 1));
    let radicand_uint = radicand.to_biguint().unwrap();
    let floor = radicand_uint.nth_root(degree);
    let floor_power = floor.pow(degree);
    let remainder = &radicand_uint - &floor_power;

    let rounded = match rounding {
        RoundingMode::Down | RoundingMode::Floor => floor,
        RoundingMode::Up | RoundingMode::Ceil => {
            if remainder.is_zero() {
                floor
            } else {
                &floor + BigUint::from(1_u8)
            }
        }
        RoundingMode::HalfUp | RoundingMode::HalfEven => {
            let ceil = &floor + BigUint::from(1_u8);
            let ceil_power = ceil.pow(degree);
            let delta_down = remainder;
            let delta_up = &ceil_power - &radicand_uint;
            match delta_down.cmp(&delta_up) {
                std::cmp::Ordering::Less => floor,
                std::cmp::Ordering::Greater => ceil,
                std::cmp::Ordering::Equal => match rounding {
                    RoundingMode::HalfUp => ceil,
                    RoundingMode::HalfEven => {
                        if (&floor & BigUint::from(1_u8)).is_zero() {
                            floor
                        } else {
                            ceil
                        }
                    }
                    _ => unreachable!(),
                },
            }
        }
    };

    let result = if negative {
        -BigInt::from(rounded)
    } else {
        BigInt::from(rounded)
    };

    Some(format_scaled::<SCALE>(&result))
}

pub fn oracle_integer_round<const SCALE: u32>(value: &str) -> (String, String, String, String) {
    let scaled = parse_scaled::<SCALE>(value);
    let factor = pow10(SCALE);
    let quotient = &scaled / &factor;
    let remainder = &scaled % &factor;

    let trunc = &quotient * &factor;

    let floor_q = if scaled.sign() == NumSign::Minus && !remainder.is_zero() {
        &quotient - 1
    } else {
        quotient.clone()
    };

    let ceil_q = if scaled.sign() != NumSign::Minus && !remainder.is_zero() {
        &quotient + 1
    } else {
        quotient.clone()
    };

    let round_q = oracle_round_half_even(scaled, factor.clone());

    (
        format_scaled::<SCALE>(&trunc),
        format_scaled::<SCALE>(&(floor_q * &factor)),
        format_scaled::<SCALE>(&(ceil_q * &factor)),
        format_scaled::<SCALE>(&(round_q * factor)),
    )
}

pub fn ceil_context() -> MathContext {
    MathContext {
        rounding: RoundingMode::Ceil,
        guard_digits: 8,
    }
}
