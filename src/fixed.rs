use std::cmp::Ordering;
use std::fmt;
use std::ops::{Add, Div, Mul, Neg, Sub};
use std::str::FromStr;

use crate::core::BigUintCore;
use crate::error::ParseBigFixedError;
use crate::int::{BigIntCore, Sign};

const EXTRA_GUARD_DIGITS: u32 = 6;
const REDUCTION_DIVISOR: u32 = 8;
const MIN_SERIES_TERMS: usize = 24;
const PRECOMPUTED_CONSTANT_SCALE: u32 = 96;
const LN_10_DECIMAL: &str =
    "2.30258509299404568401799145468436420760110148862877297603332790096757260967735248023599";
const PI_DECIMAL: &str =
    "3.14159265358979323846264338327950288419716939937510582097494459230781640628620899";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RoundingMode {
    HalfEven,
    HalfUp,
    Down,
    Up,
    Floor,
    Ceil,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MathContext {
    pub rounding: RoundingMode,
    pub guard_digits: u32,
}

impl Default for MathContext {
    fn default() -> Self {
        Self {
            rounding: RoundingMode::HalfEven,
            guard_digits: 8,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct BigFixed<const SCALE: u32> {
    mantissa: BigIntCore,
}

impl<const SCALE: u32> BigFixed<SCALE> {
    pub fn zero() -> Self {
        Self {
            mantissa: BigIntCore::zero(),
        }
    }

    pub fn from_mantissa(mantissa: BigIntCore) -> Self {
        Self { mantissa }
    }

    pub fn mantissa(&self) -> &BigIntCore {
        &self.mantissa
    }

    pub fn is_zero(&self) -> bool {
        self.mantissa.is_zero()
    }

    pub fn abs(&self) -> Self {
        Self::from_mantissa(self.mantissa.abs())
    }

    pub fn one() -> Self {
        Self::from_mantissa(BigIntCore::from_parts(
            Sign::Positive,
            BigUintCore::from_u64(1).mul_pow10(SCALE),
        ))
    }

    pub fn rescale<const NEW_SCALE: u32>(&self) -> BigFixed<NEW_SCALE> {
        self.rescale_with_context(MathContext::default())
    }

    pub fn rescale_with_context<const NEW_SCALE: u32>(
        &self,
        context: MathContext,
    ) -> BigFixed<NEW_SCALE> {
        BigFixed::<NEW_SCALE>::from_mantissa(rescale_mantissa(
            &self.mantissa,
            SCALE,
            NEW_SCALE,
            context.rounding,
        ))
    }

    pub fn trunc(&self) -> Self {
        let (integer, _, has_fraction) = self.integer_parts();
        if !has_fraction {
            return self.clone();
        }

        Self::from_scaled_integer(self.mantissa.sign(), integer)
    }

    pub fn floor(&self) -> Self {
        let (mut integer, _, has_fraction) = self.integer_parts();
        if self.mantissa.sign() == Sign::Negative && has_fraction {
            integer = integer.add_small(1);
        }

        Self::from_scaled_integer(self.mantissa.sign(), integer)
    }

    pub fn ceil(&self) -> Self {
        let (mut integer, _, has_fraction) = self.integer_parts();
        if self.mantissa.sign() == Sign::Positive && has_fraction {
            integer = integer.add_small(1);
        }

        Self::from_scaled_integer(self.mantissa.sign(), integer)
    }

    pub fn round(&self) -> Self {
        self.round_with_mode(RoundingMode::HalfEven)
    }

    pub fn round_with_mode(&self, rounding: RoundingMode) -> Self {
        let (mut integer, fractional_digits, has_fraction) = self.integer_parts();
        if !has_fraction {
            return self.clone();
        }

        let increment = match rounding {
            RoundingMode::Down => false,
            RoundingMode::Up => true,
            RoundingMode::Floor => self.mantissa.sign() == Sign::Negative,
            RoundingMode::Ceil => self.mantissa.sign() == Sign::Positive,
            RoundingMode::HalfUp => is_half_up(&fractional_digits),
            RoundingMode::HalfEven => is_half_even(&integer, &fractional_digits),
        };

        if increment {
            integer = integer.add_small(1);
        }

        Self::from_scaled_integer(self.mantissa.sign(), integer)
    }

    pub fn powu(&self, mut exponent: u32) -> Self {
        if exponent == 0 {
            return Self::one();
        }

        let mut base = self.clone();
        let mut result = Self::one();

        while exponent != 0 {
            if exponent & 1 == 1 {
                result = result * base.clone();
            }

            exponent >>= 1;
            if exponent != 0 {
                base = base.clone() * base;
            }
        }

        result
    }

    pub fn checked_powi(&self, exponent: i32) -> Option<Self> {
        if exponent >= 0 {
            return Some(self.powu(exponent as u32));
        }

        let positive = self.powu(exponent.unsigned_abs());
        Self::one().checked_div(&positive)
    }

    pub fn powi(&self, exponent: i32) -> Self {
        self.checked_powi(exponent)
            .expect("powi is undefined for zero with a negative exponent")
    }

    pub fn checked_sqrt(&self) -> Option<Self> {
        self.checked_sqrt_with_mode(RoundingMode::HalfEven)
    }

    pub fn checked_sqrt_with_context(&self, context: MathContext) -> Option<Self> {
        self.checked_sqrt_with_mode(context.rounding)
    }

    pub fn checked_sqrt_with_mode(&self, rounding: RoundingMode) -> Option<Self> {
        if self.mantissa.sign() == Sign::Negative {
            return None;
        }

        if self.is_zero() {
            return Some(Self::zero());
        }

        let radicand = self.mantissa.magnitude().mul_pow10(SCALE);
        let (mut root, remainder) = radicand.sqrt_rem();

        if should_increment_sqrt(&root, &remainder, rounding) {
            root = root.add_small(1);
        }

        Some(Self::from_mantissa(BigIntCore::from_parts(
            Sign::Positive,
            root,
        )))
    }

    pub fn sqrt(&self) -> Self {
        self.checked_sqrt()
            .expect("sqrt is undefined for negative fixed-point values")
    }

    pub fn checked_nth_root(&self, degree: u32) -> Option<Self> {
        self.checked_nth_root_with_mode(degree, RoundingMode::HalfEven)
    }

    pub fn checked_nth_root_with_context(&self, degree: u32, context: MathContext) -> Option<Self> {
        self.checked_nth_root_with_mode(degree, context.rounding)
    }

    pub fn checked_nth_root_with_mode(&self, degree: u32, rounding: RoundingMode) -> Option<Self> {
        if degree == 0 {
            return None;
        }

        if self.is_zero() {
            return Some(Self::zero());
        }

        let sign = self.mantissa.sign();
        if sign == Sign::Negative && degree.is_multiple_of(2) {
            return None;
        }

        let radicand = self
            .mantissa
            .magnitude()
            .mul_pow10(SCALE.saturating_mul(degree - 1));
        let (mut root, remainder) = radicand.nth_root_rem(degree)?;

        if should_increment_nth_root(&root, &remainder, degree, rounding) {
            root = root.add_small(1);
        }

        let result_sign = if sign == Sign::Negative {
            Sign::Negative
        } else {
            Sign::Positive
        };

        Some(Self::from_mantissa(BigIntCore::from_parts(
            result_sign,
            root,
        )))
    }

    pub fn nth_root(&self, degree: u32) -> Self {
        self.checked_nth_root(degree)
            .expect("nth_root is undefined for zero degree or even roots of negative values")
    }

    pub fn checked_div(&self, rhs: &Self) -> Option<Self> {
        self.checked_div_with_mode(rhs, RoundingMode::HalfEven)
    }

    pub fn checked_recip(&self) -> Option<Self> {
        Self::one().checked_div(self)
    }

    pub fn checked_recip_with_context(&self, context: MathContext) -> Option<Self> {
        Self::one().checked_div_with_context(self, context)
    }

    pub fn recip(&self) -> Self {
        self.checked_recip().expect("recip is undefined for zero")
    }

    pub fn checked_log10(&self) -> Option<Self> {
        self.checked_log10_with_context(MathContext::default())
    }

    pub fn checked_log10_with_context(&self, context: MathContext) -> Option<Self> {
        let work_scale = working_scale(SCALE, context.guard_digits);
        let value = EvalFixed::from_bigfixed(self, work_scale);
        let natural = value.ln()?;
        let ln_ten = EvalFixed::from_i64(work_scale, 10).ln()?;
        natural
            .div(&ln_ten, RoundingMode::HalfEven)
            .map(|result| result.to_bigfixed(context))
    }

    pub fn log10(&self) -> Self {
        self.checked_log10()
            .expect("log10 is undefined for non-positive values")
    }

    pub fn checked_ln(&self) -> Option<Self> {
        self.checked_ln_with_context(MathContext::default())
    }

    pub fn checked_ln_with_context(&self, context: MathContext) -> Option<Self> {
        let value = EvalFixed::from_bigfixed(self, working_scale(SCALE, context.guard_digits));
        value.ln().map(|result| result.to_bigfixed(context))
    }

    pub fn ln(&self) -> Self {
        self.checked_ln()
            .expect("ln is undefined for non-positive values")
    }

    pub fn checked_exp(&self) -> Option<Self> {
        self.checked_exp_with_context(MathContext::default())
    }

    pub fn checked_exp_with_context(&self, context: MathContext) -> Option<Self> {
        let value = EvalFixed::from_bigfixed(self, working_scale(SCALE, context.guard_digits));
        value.exp().map(|result| result.to_bigfixed(context))
    }

    pub fn exp(&self) -> Self {
        self.checked_exp()
            .expect("exp failed to converge at the requested precision")
    }

    pub fn checked_sin(&self) -> Option<Self> {
        self.checked_sin_with_context(MathContext::default())
    }

    pub fn checked_sin_with_context(&self, context: MathContext) -> Option<Self> {
        let value = EvalFixed::from_bigfixed(self, working_scale(SCALE, context.guard_digits));
        value.sin().map(|result| result.to_bigfixed(context))
    }

    pub fn sin(&self) -> Self {
        self.checked_sin()
            .expect("sin failed to converge at the requested precision")
    }

    pub fn checked_cos(&self) -> Option<Self> {
        self.checked_cos_with_context(MathContext::default())
    }

    pub fn checked_cos_with_context(&self, context: MathContext) -> Option<Self> {
        let value = EvalFixed::from_bigfixed(self, working_scale(SCALE, context.guard_digits));
        value.cos().map(|result| result.to_bigfixed(context))
    }

    pub fn cos(&self) -> Self {
        self.checked_cos()
            .expect("cos failed to converge at the requested precision")
    }

    pub fn checked_tan(&self) -> Option<Self> {
        self.checked_tan_with_context(MathContext::default())
    }

    pub fn checked_tan_with_context(&self, context: MathContext) -> Option<Self> {
        let work_scale = working_scale(SCALE, context.guard_digits);
        let value = EvalFixed::from_bigfixed(self, work_scale);
        value.tan(SCALE).map(|result| result.to_bigfixed(context))
    }

    pub fn tan(&self) -> Self {
        self.checked_tan()
            .expect("tan is undefined at odd pi/2 poles within the requested precision")
    }

    pub fn checked_div_with_context(&self, rhs: &Self, context: MathContext) -> Option<Self> {
        self.checked_div_with_mode(rhs, context.rounding)
    }

    pub fn checked_div_with_mode(&self, rhs: &Self, rounding: RoundingMode) -> Option<Self> {
        if rhs.is_zero() {
            return None;
        }

        if self.is_zero() {
            return Some(Self::zero());
        }

        let sign = sign_product(self.mantissa.sign(), rhs.mantissa.sign());
        let scaled_numerator = self.mantissa.magnitude().mul_pow10(SCALE);
        let (mut quotient, remainder) = scaled_numerator.div_rem(rhs.mantissa.magnitude());

        if should_increment_quotient(
            &quotient,
            &remainder,
            rhs.mantissa.magnitude(),
            sign,
            rounding,
        ) {
            quotient = quotient.add_small(1);
        }

        Some(Self::from_mantissa(BigIntCore::from_parts(sign, quotient)))
    }

    pub fn to_trimmed_string(&self) -> String {
        if SCALE == 0 {
            return self.to_string();
        }

        let mut value = self.to_string();
        while value.ends_with('0') {
            value.pop();
        }
        if value.ends_with('.') {
            value.pop();
        }
        value
    }

    fn parse_impl(input: &str) -> Result<Self, ParseBigFixedError> {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return Err(ParseBigFixedError::Empty);
        }

        let (sign, digits) = if let Some(rest) = trimmed.strip_prefix('-') {
            (Sign::Negative, rest)
        } else if let Some(rest) = trimmed.strip_prefix('+') {
            (Sign::Positive, rest)
        } else {
            (Sign::Positive, trimmed)
        };

        if digits.is_empty() {
            return Err(ParseBigFixedError::InvalidFormat);
        }

        let mut parts = digits.split('.');
        let int_part = parts.next().unwrap_or("");
        let frac_part = parts.next();
        if parts.next().is_some() {
            return Err(ParseBigFixedError::InvalidFormat);
        }

        let frac_part = frac_part.unwrap_or("");
        if int_part.is_empty() && frac_part.is_empty() {
            return Err(ParseBigFixedError::InvalidFormat);
        }

        if frac_part.len() > SCALE as usize {
            return Err(ParseBigFixedError::FractionalDigitsExceedScale {
                found: frac_part.len(),
                scale: SCALE,
            });
        }

        for ch in int_part.chars().chain(frac_part.chars()) {
            if !ch.is_ascii_digit() {
                return Err(ParseBigFixedError::InvalidCharacter(ch));
            }
        }

        let mut digits = String::with_capacity(int_part.len() + SCALE as usize);
        digits.push_str(int_part);
        digits.push_str(frac_part);
        for _ in frac_part.len()..SCALE as usize {
            digits.push('0');
        }

        let magnitude =
            BigUintCore::from_decimal_digits(&digits).ok_or(ParseBigFixedError::InvalidFormat)?;
        let mantissa = BigIntCore::from_parts(sign, magnitude);
        Ok(Self::from_mantissa(mantissa))
    }

    fn from_scaled_integer(sign: Sign, integer: BigUintCore) -> Self {
        let mantissa = BigIntCore::from_parts(sign, integer.mul_pow10(SCALE));
        Self::from_mantissa(mantissa)
    }

    fn integer_parts(&self) -> (BigUintCore, Vec<u8>, bool) {
        if SCALE == 0 {
            return (self.mantissa.magnitude().clone(), Vec::new(), false);
        }

        let mut quotient = self.mantissa.magnitude().clone();
        let mut digits = Vec::with_capacity(SCALE as usize);
        let mut has_fraction = false;

        for _ in 0..SCALE {
            let (next, remainder) = quotient.div_rem_small(10);
            digits.push(remainder as u8);
            has_fraction |= remainder != 0;
            quotient = next;
        }

        digits.reverse();
        (quotient, digits, has_fraction)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct EvalFixed {
    mantissa: BigIntCore,
    scale: u32,
}

impl EvalFixed {
    fn zero(scale: u32) -> Self {
        Self {
            mantissa: BigIntCore::zero(),
            scale,
        }
    }

    fn one(scale: u32) -> Self {
        Self::from_i64(scale, 1)
    }

    fn from_i64(scale: u32, value: i64) -> Self {
        if value == 0 {
            return Self::zero(scale);
        }

        let sign = if value.is_negative() {
            Sign::Negative
        } else {
            Sign::Positive
        };
        let magnitude = BigUintCore::from_u64(value.unsigned_abs()).mul_pow10(scale);
        Self {
            mantissa: BigIntCore::from_parts(sign, magnitude),
            scale,
        }
    }

    fn from_bigfixed<const SCALE: u32>(value: &BigFixed<SCALE>, scale: u32) -> Self {
        Self {
            mantissa: rescale_mantissa(&value.mantissa, SCALE, scale, RoundingMode::HalfEven),
            scale,
        }
    }

    fn to_bigfixed<const SCALE: u32>(&self, context: MathContext) -> BigFixed<SCALE> {
        BigFixed::<SCALE>::from_mantissa(rescale_mantissa(
            &self.mantissa,
            self.scale,
            SCALE,
            context.rounding,
        ))
    }

    fn is_zero(&self) -> bool {
        self.mantissa.is_zero()
    }

    fn sign(&self) -> Sign {
        self.mantissa.sign()
    }

    fn cmp_value(&self, rhs: &Self) -> Ordering {
        assert_eq!(self.scale, rhs.scale);
        self.mantissa.cmp(&rhs.mantissa)
    }

    fn abs_cmp(&self, rhs: &Self) -> Ordering {
        assert_eq!(self.scale, rhs.scale);
        self.mantissa.magnitude().cmp(rhs.mantissa.magnitude())
    }

    fn abs(&self) -> Self {
        Self {
            mantissa: self.mantissa.abs(),
            scale: self.scale,
        }
    }

    fn negated(&self) -> Self {
        Self {
            mantissa: self.mantissa.negated(),
            scale: self.scale,
        }
    }

    fn add(&self, rhs: &Self) -> Self {
        assert_eq!(self.scale, rhs.scale);
        Self {
            mantissa: self.mantissa.add(&rhs.mantissa),
            scale: self.scale,
        }
    }

    fn sub(&self, rhs: &Self) -> Self {
        assert_eq!(self.scale, rhs.scale);
        Self {
            mantissa: self.mantissa.sub(&rhs.mantissa),
            scale: self.scale,
        }
    }

    fn mul(&self, rhs: &Self) -> Self {
        assert_eq!(self.scale, rhs.scale);
        let sign = sign_product(self.sign(), rhs.sign());
        if sign == Sign::Zero {
            return Self::zero(self.scale);
        }

        let product = self.mantissa.magnitude().mul(rhs.mantissa.magnitude());
        let scaled =
            round_unsigned_pow10_division(&product, self.scale, sign, RoundingMode::HalfEven);
        Self {
            mantissa: BigIntCore::from_parts(sign, scaled),
            scale: self.scale,
        }
    }

    fn square(&self) -> Self {
        self.mul(self)
    }

    fn mul_small(&self, multiplier: i64) -> Self {
        if multiplier == 0 || self.is_zero() {
            return Self::zero(self.scale);
        }

        let sign = sign_product(
            self.sign(),
            if multiplier.is_negative() {
                Sign::Negative
            } else {
                Sign::Positive
            },
        );
        let magnitude = self.mantissa.magnitude().mul_small(
            u32::try_from(multiplier.unsigned_abs()).expect("small multiplier overflowed u32"),
        );
        Self {
            mantissa: BigIntCore::from_parts(sign, magnitude),
            scale: self.scale,
        }
    }

    fn mul_int(&self, rhs: &BigIntCore) -> Self {
        let sign = sign_product(self.sign(), rhs.sign());
        if sign == Sign::Zero {
            return Self::zero(self.scale);
        }

        let magnitude = self.mantissa.magnitude().mul(rhs.magnitude());
        Self {
            mantissa: BigIntCore::from_parts(sign, magnitude),
            scale: self.scale,
        }
    }

    fn div(&self, rhs: &Self, rounding: RoundingMode) -> Option<Self> {
        assert_eq!(self.scale, rhs.scale);
        if rhs.is_zero() {
            return None;
        }

        if self.is_zero() {
            return Some(Self::zero(self.scale));
        }

        let sign = sign_product(self.sign(), rhs.sign());
        let scaled_numerator = self.mantissa.magnitude().mul_pow10(self.scale);
        let (mut quotient, remainder) = scaled_numerator.div_rem(rhs.mantissa.magnitude());

        if should_increment_quotient(
            &quotient,
            &remainder,
            rhs.mantissa.magnitude(),
            sign,
            rounding,
        ) {
            quotient = quotient.add_small(1);
        }

        Some(Self {
            mantissa: BigIntCore::from_parts(sign, quotient),
            scale: self.scale,
        })
    }

    fn div_small(&self, divisor: u32, rounding: RoundingMode) -> Self {
        assert_ne!(divisor, 0, "division by zero");
        if self.is_zero() {
            return Self::zero(self.scale);
        }

        let (mut quotient, remainder) = self.mantissa.magnitude().div_rem_small(divisor);
        if should_increment_quotient(
            &quotient,
            &BigUintCore::from_u64(remainder as u64),
            &BigUintCore::from_u64(divisor as u64),
            self.sign(),
            rounding,
        ) {
            quotient = quotient.add_small(1);
        }

        Self {
            mantissa: BigIntCore::from_parts(self.sign(), quotient),
            scale: self.scale,
        }
    }

    fn sqrt(&self) -> Option<Self> {
        if self.sign() == Sign::Negative {
            return None;
        }

        if self.is_zero() {
            return Some(Self::zero(self.scale));
        }

        let radicand = self.mantissa.magnitude().mul_pow10(self.scale);
        let (mut root, remainder) = radicand.sqrt_rem();

        if should_increment_sqrt(&root, &remainder, RoundingMode::HalfEven) {
            root = root.add_small(1);
        }

        Some(Self {
            mantissa: BigIntCore::from_parts(Sign::Positive, root),
            scale: self.scale,
        })
    }

    fn div_to_integer(&self, rhs: &Self, rounding: RoundingMode) -> Option<BigIntCore> {
        assert_eq!(self.scale, rhs.scale);
        if rhs.is_zero() {
            return None;
        }

        if self.is_zero() {
            return Some(BigIntCore::zero());
        }

        let sign = sign_product(self.sign(), rhs.sign());
        let (mut quotient, remainder) = self.mantissa.magnitude().div_rem(rhs.mantissa.magnitude());
        if should_increment_quotient(
            &quotient,
            &remainder,
            rhs.mantissa.magnitude(),
            sign,
            rounding,
        ) {
            quotient = quotient.add_small(1);
        }

        Some(BigIntCore::from_parts(sign, quotient))
    }

    fn ulp(scale: u32) -> Self {
        Self {
            mantissa: BigIntCore::from_parts(Sign::Positive, BigUintCore::one()),
            scale,
        }
    }

    fn half_target_ulp(eval_scale: u32, target_scale: u32) -> Self {
        if eval_scale <= target_scale {
            return Self::ulp(eval_scale);
        }

        let exponent = eval_scale - target_scale;
        let mut magnitude = BigUintCore::from_u64(5);
        if exponent > 1 {
            magnitude = magnitude.mul_pow10(exponent - 1);
        }

        Self {
            mantissa: BigIntCore::from_parts(Sign::Positive, magnitude),
            scale: eval_scale,
        }
    }

    fn is_within(&self, limit: &Self) -> bool {
        self.abs_cmp(limit) != Ordering::Greater
    }

    fn convergence_limit(&self) -> Self {
        Self::ulp(self.scale)
    }

    fn ln(&self) -> Option<Self> {
        if self.sign() != Sign::Positive {
            return None;
        }

        let exponent = self.decimal_exponent()?;
        if exponent == 0 {
            return self.ln_core_reduced();
        }

        let normalized = self.shift_decimal_exponent(exponent)?;
        let normalized_ln = normalized.ln_core_reduced()?;
        let ln_ten = Self::ln_ten_constant(self.scale)?;

        Some(normalized_ln.add(&ln_ten.mul_small(exponent)))
    }

    fn ln_core_reduced(&self) -> Option<Self> {
        if self.sign() != Sign::Positive {
            return None;
        }

        let one = Self::one(self.scale);
        let threshold = one.div_small(REDUCTION_DIVISOR, RoundingMode::HalfEven);
        let lower = one.sub(&threshold);
        let upper = one.add(&threshold);

        let mut reduced = self.clone();
        let mut doublings = 0_u32;
        while reduced.cmp_value(&lower) == Ordering::Less
            || reduced.cmp_value(&upper) == Ordering::Greater
        {
            reduced = reduced.sqrt()?;
            doublings += 1;
        }

        let numerator = reduced.sub(&one);
        let denominator = reduced.add(&one);
        let z = numerator.div(&denominator, RoundingMode::HalfEven)?;
        let z2 = z.square();
        let mut power = z.clone();
        let mut sum = power.clone();
        let limit = self.convergence_limit();

        for n in 1..=series_term_limit(self.scale) {
            power = power.mul(&z2);
            let addend = power.div_small((2 * n + 1) as u32, RoundingMode::HalfEven);
            if addend.is_zero() {
                break;
            }

            sum = sum.add(&addend);
            if addend.is_within(&limit) {
                break;
            }
        }

        let mut result = sum.mul_small(2);
        for _ in 0..doublings {
            result = result.mul_small(2);
        }

        Some(result)
    }

    fn decimal_exponent(&self) -> Option<i64> {
        if self.is_zero() {
            return None;
        }

        let digits = self.mantissa.magnitude().to_decimal_string();
        let digits_len = i64::try_from(digits.len()).ok()?;
        Some(digits_len - 1 - i64::from(self.scale))
    }

    fn shift_decimal_exponent(&self, exponent: i64) -> Option<Self> {
        if exponent == 0 || self.is_zero() {
            return Some(self.clone());
        }

        if exponent > 0 {
            let divisor_digits = u32::try_from(exponent).ok()?;
            let magnitude = round_unsigned_pow10_division(
                self.mantissa.magnitude(),
                divisor_digits,
                self.sign(),
                RoundingMode::HalfEven,
            );
            return Some(Self {
                mantissa: BigIntCore::from_parts(self.sign(), magnitude),
                scale: self.scale,
            });
        }

        let multiplier_digits = u32::try_from(exponent.unsigned_abs()).ok()?;
        Some(Self {
            mantissa: BigIntCore::from_parts(
                self.sign(),
                self.mantissa.magnitude().mul_pow10(multiplier_digits),
            ),
            scale: self.scale,
        })
    }

    fn from_decimal_literal(scale: u32, literal: &str) -> Option<Self> {
        let mut parts = literal.split('.');
        let integer = parts.next().unwrap_or("0");
        let fraction = parts.next().unwrap_or("");
        if parts.next().is_some() {
            return None;
        }

        let literal_scale = u32::try_from(fraction.len()).ok()?;

        let mut digits = String::with_capacity(integer.len() + fraction.len());
        digits.push_str(integer);
        digits.push_str(fraction);
        let magnitude = BigUintCore::from_decimal_digits(&digits)?;
        let mantissa = BigIntCore::from_parts(Sign::Positive, magnitude);

        Some(Self {
            mantissa: rescale_mantissa(&mantissa, literal_scale, scale, RoundingMode::HalfEven),
            scale,
        })
    }

    fn ln_ten_constant(scale: u32) -> Option<Self> {
        if scale <= PRECOMPUTED_CONSTANT_SCALE {
            Self::from_decimal_literal(scale, LN_10_DECIMAL)
        } else {
            Self::from_i64(scale, 10).ln_core_reduced()
        }
    }

    fn pi_constant(scale: u32) -> Option<Self> {
        if scale <= PRECOMPUTED_CONSTANT_SCALE {
            Self::from_decimal_literal(scale, PI_DECIMAL)
        } else {
            Self::pi(scale)
        }
    }

    fn exp(&self) -> Option<Self> {
        let negative = self.sign() == Sign::Negative;
        let mut reduced = if negative { self.abs() } else { self.clone() };
        let threshold = Self::one(self.scale).div_small(REDUCTION_DIVISOR, RoundingMode::HalfEven);

        let mut halvings = 0_u32;
        while reduced.cmp_value(&threshold) == Ordering::Greater {
            reduced = reduced.div_small(2, RoundingMode::HalfEven);
            halvings += 1;
        }

        let mut term = Self::one(self.scale);
        let mut sum = term.clone();
        let limit = self.convergence_limit();

        for n in 1..=series_term_limit(self.scale) {
            term = term.mul(&reduced);
            term = term.div_small(n as u32, RoundingMode::HalfEven);
            if term.is_zero() {
                break;
            }

            sum = sum.add(&term);
            if term.is_within(&limit) {
                break;
            }
        }

        let mut result = sum;
        for _ in 0..halvings {
            result = result.mul(&result);
        }

        if negative {
            Self::one(self.scale).div(&result, RoundingMode::HalfEven)
        } else {
            Some(result)
        }
    }

    fn sin(&self) -> Option<Self> {
        let (reduced, sin_sign, _) = self.reduce_for_trig()?;
        let mut result = reduced.sin_series();
        if sin_sign < 0 {
            result = result.negated();
        }
        Some(result)
    }

    fn cos(&self) -> Option<Self> {
        let (reduced, _, cos_sign) = self.reduce_for_trig()?;
        let mut result = reduced.cos_series();
        if cos_sign < 0 {
            result = result.negated();
        }
        Some(result)
    }

    fn tan(&self, target_scale: u32) -> Option<Self> {
        let (reduced, sin_sign, cos_sign) = self.reduce_for_trig()?;
        let quarter_turn = Self::pi_constant(self.scale)?.div_small(2, RoundingMode::HalfEven);
        let pole_distance = reduced.abs().sub(&quarter_turn).abs();
        if pole_distance.is_within(&Self::half_target_ulp(self.scale, target_scale)) {
            return None;
        }

        let mut sine = reduced.sin_series();
        if sin_sign < 0 {
            sine = sine.negated();
        }

        let mut cosine = reduced.cos_series();
        if cos_sign < 0 {
            cosine = cosine.negated();
        }

        if cosine
            .abs()
            .is_within(&Self::half_target_ulp(self.scale, target_scale))
        {
            return None;
        }

        sine.div(&cosine, RoundingMode::HalfEven)
    }

    fn sin_series(&self) -> Self {
        let x2 = self.square();
        let mut term = self.clone();
        let mut sum = term.clone();
        let limit = self.convergence_limit();

        for n in 1..=series_term_limit(self.scale) {
            term = term.mul(&x2).negated();
            term = term.div_small((2 * n) as u32, RoundingMode::HalfEven);
            term = term.div_small((2 * n + 1) as u32, RoundingMode::HalfEven);
            if term.is_zero() {
                break;
            }

            sum = sum.add(&term);
            if term.abs().is_within(&limit) {
                break;
            }
        }

        sum
    }

    fn cos_series(&self) -> Self {
        let x2 = self.square();
        let mut term = Self::one(self.scale);
        let mut sum = term.clone();
        let limit = self.convergence_limit();

        for n in 1..=series_term_limit(self.scale) {
            term = term.mul(&x2).negated();
            term = term.div_small((2 * n - 1) as u32, RoundingMode::HalfEven);
            term = term.div_small((2 * n) as u32, RoundingMode::HalfEven);
            if term.is_zero() {
                break;
            }

            sum = sum.add(&term);
            if term.abs().is_within(&limit) {
                break;
            }
        }

        sum
    }

    fn reduce_for_trig(&self) -> Option<(Self, i8, i8)> {
        let pi = Self::pi_constant(self.scale)?;
        let tau = pi.mul_small(2);
        let quarter_turn = pi.div_small(2, RoundingMode::HalfEven);
        let turns = self.div_to_integer(&tau, RoundingMode::HalfEven)?;
        let mut reduced = self.sub(&tau.mul_int(&turns));
        let mut sin_sign = 1_i8;
        let mut cos_sign = 1_i8;

        if reduced.cmp_value(&quarter_turn) == Ordering::Greater {
            reduced = pi.sub(&reduced);
            cos_sign = -1;
        } else {
            let neg_quarter = quarter_turn.negated();
            if reduced.cmp_value(&neg_quarter) == Ordering::Less {
                reduced = pi.add(&reduced);
                sin_sign = -1;
                cos_sign = -1;
            }
        }

        Some((reduced, sin_sign, cos_sign))
    }

    fn pi(scale: u32) -> Option<Self> {
        let atan_fifth = Self::arctan_recip(scale, 5)?;
        let atan_two_thirty_nine = Self::arctan_recip(scale, 239)?;
        Some(
            atan_fifth
                .mul_small(16)
                .sub(&atan_two_thirty_nine.mul_small(4)),
        )
    }

    fn arctan_recip(scale: u32, reciprocal: u32) -> Option<Self> {
        let one = Self::one(scale);
        let z = one.div_small(reciprocal, RoundingMode::HalfEven);
        let z2 = z.square();
        let mut power = z.clone();
        let mut sum = power.clone();
        let limit = Self::ulp(scale);

        for n in 1..=series_term_limit(scale) {
            power = power.mul(&z2);
            let addend = power.div_small((2 * n + 1) as u32, RoundingMode::HalfEven);
            if addend.is_zero() {
                break;
            }

            sum = if n % 2 == 1 {
                sum.sub(&addend)
            } else {
                sum.add(&addend)
            };

            if addend.abs().is_within(&limit) {
                break;
            }
        }

        Some(sum)
    }
}

fn working_scale(scale: u32, guard_digits: u32) -> u32 {
    scale
        .saturating_add(guard_digits)
        .saturating_add(EXTRA_GUARD_DIGITS)
}

fn series_term_limit(scale: u32) -> usize {
    MIN_SERIES_TERMS.max((scale as usize + EXTRA_GUARD_DIGITS as usize + 4) * 4)
}

fn sign_product(lhs: Sign, rhs: Sign) -> Sign {
    match (lhs, rhs) {
        (Sign::Zero, _) | (_, Sign::Zero) => Sign::Zero,
        (Sign::Positive, Sign::Positive) | (Sign::Negative, Sign::Negative) => Sign::Positive,
        (Sign::Positive, Sign::Negative) | (Sign::Negative, Sign::Positive) => Sign::Negative,
    }
}

fn rescale_mantissa(
    mantissa: &BigIntCore,
    from_scale: u32,
    to_scale: u32,
    rounding: RoundingMode,
) -> BigIntCore {
    if mantissa.is_zero() || from_scale == to_scale {
        return mantissa.clone();
    }

    if to_scale > from_scale {
        return BigIntCore::from_parts(
            mantissa.sign(),
            mantissa.magnitude().mul_pow10(to_scale - from_scale),
        );
    }

    let scale_diff = from_scale - to_scale;
    let divisor = BigUintCore::one().mul_pow10(scale_diff);
    let (mut quotient, remainder) = mantissa.magnitude().div_rem(&divisor);

    if should_increment_quotient(&quotient, &remainder, &divisor, mantissa.sign(), rounding) {
        quotient = quotient.add_small(1);
    }

    BigIntCore::from_parts(mantissa.sign(), quotient)
}

fn round_unsigned_pow10_division(
    value: &BigUintCore,
    scale_digits: u32,
    sign: Sign,
    rounding: RoundingMode,
) -> BigUintCore {
    if scale_digits == 0 {
        return value.clone();
    }

    let divisor = BigUintCore::one().mul_pow10(scale_digits);
    let (mut quotient, remainder) = value.div_rem(&divisor);
    if should_increment_quotient(&quotient, &remainder, &divisor, sign, rounding) {
        quotient = quotient.add_small(1);
    }
    quotient
}

fn is_half_up(fractional_digits: &[u8]) -> bool {
    if fractional_digits.is_empty() {
        return false;
    }

    match fractional_digits[0].cmp(&5) {
        Ordering::Less => false,
        Ordering::Greater => true,
        Ordering::Equal => true,
    }
}

fn is_half_even(integer: &BigUintCore, fractional_digits: &[u8]) -> bool {
    if fractional_digits.is_empty() {
        return false;
    }

    match fractional_digits[0].cmp(&5) {
        Ordering::Less => false,
        Ordering::Greater => true,
        Ordering::Equal => {
            if fractional_digits[1..].iter().any(|digit| *digit != 0) {
                true
            } else {
                integer.div_rem_small(2).1 == 1
            }
        }
    }
}

fn should_increment_quotient(
    quotient: &BigUintCore,
    remainder: &BigUintCore,
    divisor: &BigUintCore,
    sign: Sign,
    rounding: RoundingMode,
) -> bool {
    if remainder.is_zero() {
        return false;
    }

    match rounding {
        RoundingMode::Down => false,
        RoundingMode::Up => true,
        RoundingMode::Floor => sign == Sign::Negative,
        RoundingMode::Ceil => sign == Sign::Positive,
        RoundingMode::HalfUp => {
            let doubled = remainder.mul_small(2);
            doubled >= *divisor
        }
        RoundingMode::HalfEven => {
            let doubled = remainder.mul_small(2);
            match doubled.cmp(divisor) {
                Ordering::Less => false,
                Ordering::Greater => true,
                Ordering::Equal => quotient.div_rem_small(2).1 == 1,
            }
        }
    }
}

fn should_increment_sqrt(
    root: &BigUintCore,
    remainder: &BigUintCore,
    rounding: RoundingMode,
) -> bool {
    if remainder.is_zero() {
        return false;
    }

    match rounding {
        RoundingMode::Down | RoundingMode::Floor => return false,
        RoundingMode::Up | RoundingMode::Ceil => return true,
        RoundingMode::HalfUp | RoundingMode::HalfEven => {}
    }

    let scaled_remainder = remainder.mul_small(4);
    let midpoint = root.mul_small(4).add_small(1);

    match scaled_remainder.cmp(&midpoint) {
        Ordering::Less => false,
        Ordering::Greater => true,
        Ordering::Equal => match rounding {
            RoundingMode::HalfUp => true,
            RoundingMode::HalfEven => root.div_rem_small(2).1 == 1,
            _ => false,
        },
    }
}

fn should_increment_nth_root(
    root: &BigUintCore,
    remainder: &BigUintCore,
    degree: u32,
    rounding: RoundingMode,
) -> bool {
    if remainder.is_zero() {
        return false;
    }

    match rounding {
        RoundingMode::Down | RoundingMode::Floor => return false,
        RoundingMode::Up | RoundingMode::Ceil => return true,
        RoundingMode::HalfUp | RoundingMode::HalfEven => {}
    }

    let delta_down = remainder.clone();
    let next = root.add_small(1);
    let delta_up = next
        .pow_u32(degree)
        .sub(&root.pow_u32(degree).add(remainder));

    match delta_down.cmp(&delta_up) {
        Ordering::Less => false,
        Ordering::Greater => true,
        Ordering::Equal => match rounding {
            RoundingMode::HalfUp => true,
            RoundingMode::HalfEven => root.div_rem_small(2).1 == 1,
            _ => false,
        },
    }
}

impl<const SCALE: u32> Default for BigFixed<SCALE> {
    fn default() -> Self {
        Self::zero()
    }
}

impl<const SCALE: u32> fmt::Display for BigFixed<SCALE> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let sign_prefix = match self.mantissa.sign() {
            Sign::Negative => "-",
            Sign::Zero | Sign::Positive => "",
        };

        let digits = self.mantissa.magnitude().to_decimal_string();

        if SCALE == 0 {
            return write!(f, "{sign_prefix}{digits}");
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
        write!(f, "{sign_prefix}{}.{}", &padded[..split], &padded[split..])
    }
}

impl<const SCALE: u32> FromStr for BigFixed<SCALE> {
    type Err = ParseBigFixedError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse_impl(s)
    }
}

impl<const SCALE: u32> Neg for BigFixed<SCALE> {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self::from_mantissa(self.mantissa.negated())
    }
}

impl<const SCALE: u32> Add for BigFixed<SCALE> {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self::from_mantissa(self.mantissa.add(&rhs.mantissa))
    }
}

impl<const SCALE: u32> Sub for BigFixed<SCALE> {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self::from_mantissa(self.mantissa.sub(&rhs.mantissa))
    }
}

impl<const SCALE: u32> Mul for BigFixed<SCALE> {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        let sign = sign_product(self.mantissa.sign(), rhs.mantissa.sign());

        if sign == Sign::Zero {
            return Self::zero();
        }

        let product = self.mantissa.magnitude().mul(rhs.mantissa.magnitude());
        let scaled_mantissa =
            round_unsigned_pow10_division(&product, SCALE, sign, RoundingMode::HalfEven);

        Self::from_mantissa(BigIntCore::from_parts(sign, scaled_mantissa))
    }
}

impl<const SCALE: u32> Div for BigFixed<SCALE> {
    type Output = Self;

    fn div(self, rhs: Self) -> Self::Output {
        self.checked_div(&rhs).expect("division by zero")
    }
}
