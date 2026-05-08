use std::cmp::Ordering;
use std::slice;

#[derive(Clone, Debug, Eq, PartialEq)]
enum Repr {
    Inline(u64),
    Heap(Vec<u64>),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BigUintCore {
    repr: Repr,
}

impl Default for BigUintCore {
    fn default() -> Self {
        Self::zero()
    }
}

impl BigUintCore {
    pub const fn zero() -> Self {
        Self {
            repr: Repr::Inline(0),
        }
    }

    pub const fn one() -> Self {
        Self {
            repr: Repr::Inline(1),
        }
    }

    pub const fn from_u64(value: u64) -> Self {
        Self {
            repr: Repr::Inline(value),
        }
    }

    pub fn is_zero(&self) -> bool {
        matches!(self.repr, Repr::Inline(0))
    }

    pub fn add(&self, rhs: &Self) -> Self {
        let lhs_limbs = self.limbs();
        let rhs_limbs = rhs.limbs();
        let max_len = lhs_limbs.len().max(rhs_limbs.len());

        let mut out = Vec::with_capacity(max_len + 1);
        let mut carry = 0_u128;

        for index in 0..max_len {
            let lhs = lhs_limbs.get(index).copied().unwrap_or(0) as u128;
            let rhs = rhs_limbs.get(index).copied().unwrap_or(0) as u128;
            let sum = lhs + rhs + carry;
            out.push(sum as u64);
            carry = sum >> 64;
        }

        if carry != 0 {
            out.push(carry as u64);
        }

        Self::from_limbs(out)
    }

    pub fn sub(&self, rhs: &Self) -> Self {
        assert!(self >= rhs, "BigUintCore subtraction requires lhs >= rhs");

        let lhs_limbs = self.limbs();
        let rhs_limbs = rhs.limbs();
        let mut out = Vec::with_capacity(lhs_limbs.len());
        let mut borrow = false;

        for (index, lhs_limb) in lhs_limbs.iter().copied().enumerate() {
            let lhs = lhs_limb as u128;
            let rhs = rhs_limbs.get(index).copied().unwrap_or(0) as u128;
            let borrow_value = u128::from(borrow);

            if lhs >= rhs + borrow_value {
                out.push((lhs - rhs - borrow_value) as u64);
                borrow = false;
            } else {
                out.push(((1_u128 << 64) + lhs - rhs - borrow_value) as u64);
                borrow = true;
            }
        }

        debug_assert!(!borrow);
        Self::from_limbs(out)
    }

    pub fn mul_small(&self, rhs: u32) -> Self {
        if self.is_zero() || rhs == 0 {
            return Self::zero();
        }

        if rhs == 1 {
            return self.clone();
        }

        let rhs = rhs as u128;
        let mut out = Vec::with_capacity(self.limbs().len() + 1);
        let mut carry = 0_u128;

        for limb in self.limbs() {
            let product = (*limb as u128) * rhs + carry;
            out.push(product as u64);
            carry = product >> 64;
        }

        if carry != 0 {
            out.push(carry as u64);
        }

        Self::from_limbs(out)
    }

    pub fn mul(&self, rhs: &Self) -> Self {
        if self.is_zero() || rhs.is_zero() {
            return Self::zero();
        }

        let lhs_limbs = self.limbs();
        let rhs_limbs = rhs.limbs();
        let mut out = vec![0_u64; lhs_limbs.len() + rhs_limbs.len()];

        for (lhs_index, lhs_limb) in lhs_limbs.iter().copied().enumerate() {
            let lhs = lhs_limb as u128;
            let mut carry = 0_u128;

            for (rhs_index, rhs_limb) in rhs_limbs.iter().copied().enumerate() {
                let slot = lhs_index + rhs_index;
                let acc = (out[slot] as u128) + lhs * (rhs_limb as u128) + carry;
                out[slot] = acc as u64;
                carry = acc >> 64;
            }

            let mut slot = lhs_index + rhs_limbs.len();
            while carry != 0 {
                let acc = (out[slot] as u128) + carry;
                out[slot] = acc as u64;
                carry = acc >> 64;
                slot += 1;
            }
        }

        Self::from_limbs(out)
    }

    pub fn pow_u32(&self, mut exponent: u32) -> Self {
        if exponent == 0 {
            return Self::one();
        }

        let mut base = self.clone();
        let mut result = Self::one();

        while exponent != 0 {
            if exponent & 1 == 1 {
                result = result.mul(&base);
            }

            exponent >>= 1;
            if exponent != 0 {
                base = base.mul(&base);
            }
        }

        result
    }

    pub fn div_rem(&self, rhs: &Self) -> (Self, Self) {
        assert!(!rhs.is_zero(), "division by zero");

        if self < rhs {
            return (Self::zero(), self.clone());
        }

        if rhs == &Self::one() {
            return (self.clone(), Self::zero());
        }

        if let Some(divisor) = rhs.as_u64() {
            let (quotient, remainder) = self.div_rem_limb(divisor);
            return (quotient, Self::from_u64(remainder));
        }

        let mut quotient = Self::zero();
        let mut remainder = self.clone();
        let max_shift = remainder.bit_len() - rhs.bit_len();
        let mut shifted_divisor = rhs.shl_bits(max_shift);
        let mut quotient_bit = Self::one().shl_bits(max_shift);

        for _ in (0..=max_shift).rev() {
            if shifted_divisor <= remainder {
                remainder = remainder.sub(&shifted_divisor);
                quotient = quotient.add(&quotient_bit);
            }

            shifted_divisor = shifted_divisor.shr1();
            quotient_bit = quotient_bit.shr1();
        }

        (quotient, remainder)
    }

    pub fn sqrt_rem(&self) -> (Self, Self) {
        if self.is_zero() {
            return (Self::zero(), Self::zero());
        }

        if self == &Self::one() {
            return (Self::one(), Self::zero());
        }

        let mut estimate = Self::one().shl_bits(self.bit_len().div_ceil(2));

        loop {
            let quotient = self.div_rem(&estimate).0;
            let next = estimate.add(&quotient).div_rem_small(2).0;
            if next >= estimate {
                break;
            }
            estimate = next;
        }

        let one = Self::one();
        let mut square = estimate.mul(&estimate);

        while square > *self {
            estimate = estimate.sub(&one);
            square = estimate.mul(&estimate);
        }

        loop {
            let next = estimate.add(&one);
            let next_square = next.mul(&next);
            if next_square > *self {
                break;
            }
            estimate = next;
            square = next_square;
        }

        (estimate.clone(), self.sub(&square))
    }

    pub fn nth_root_rem(&self, degree: u32) -> Option<(Self, Self)> {
        if degree == 0 {
            return None;
        }

        if self.is_zero() {
            return Some((Self::zero(), Self::zero()));
        }

        if degree == 1 {
            return Some((self.clone(), Self::zero()));
        }

        if degree == 2 {
            let (root, remainder) = self.sqrt_rem();
            return Some((root, remainder));
        }

        let mut low = Self::one();
        let mut high = Self::one().shl_bits(self.bit_len().div_ceil(degree as usize));

        while low < high {
            let midpoint = low.add(&high).add_small(1).div_rem_small(2).0;
            let mid_power = midpoint.pow_u32(degree);
            if mid_power <= *self {
                low = midpoint;
            } else {
                high = midpoint.sub(&Self::one());
            }
        }

        let floor_power = low.pow_u32(degree);
        Some((low, self.sub(&floor_power)))
    }

    pub fn add_small(&self, rhs: u32) -> Self {
        if rhs == 0 {
            return self.clone();
        }

        let mut out = self.limbs().to_vec();
        if out.is_empty() {
            out.push(rhs as u64);
            return Self::from_limbs(out);
        }

        let mut carry = rhs as u128;
        for limb in &mut out {
            let sum = (*limb as u128) + carry;
            *limb = sum as u64;
            carry = sum >> 64;
            if carry == 0 {
                return Self::from_limbs(out);
            }
        }

        out.push(carry as u64);
        Self::from_limbs(out)
    }

    pub fn mul_pow10(&self, exp: u32) -> Self {
        let mut value = self.clone();
        for _ in 0..exp {
            value = value.mul_small(10);
        }
        value
    }

    pub fn div_rem_small(&self, divisor: u32) -> (Self, u32) {
        assert!(divisor != 0, "division by zero");

        let (quotient, remainder) = self.div_rem_limb(divisor as u64);
        (quotient, remainder as u32)
    }

    pub fn div_rem_limb(&self, divisor: u64) -> (Self, u64) {
        assert!(divisor != 0, "division by zero");

        if self.is_zero() {
            return (Self::zero(), 0);
        }

        let divisor = divisor as u128;
        let mut out = Vec::with_capacity(self.limbs().len());
        let mut remainder = 0_u128;

        for limb in self.limbs().iter().rev() {
            let value = (remainder << 64) | (*limb as u128);
            out.push((value / divisor) as u64);
            remainder = value % divisor;
        }

        out.reverse();
        (Self::from_limbs(out), remainder as u64)
    }

    pub fn from_decimal_digits(digits: &str) -> Option<Self> {
        if digits.is_empty() {
            return Some(Self::zero());
        }

        let mut value = Self::zero();
        for ch in digits.chars() {
            let digit = ch.to_digit(10)?;
            value = value.mul_small(10).add_small(digit);
        }
        Some(value)
    }

    pub fn to_decimal_string(&self) -> String {
        if self.is_zero() {
            return String::from("0");
        }

        let mut value = self.clone();
        let mut digits = Vec::new();
        while !value.is_zero() {
            let (quotient, remainder) = value.div_rem_small(10);
            digits.push(char::from(b'0' + remainder as u8));
            value = quotient;
        }

        digits.iter().rev().collect()
    }

    pub fn bit_len(&self) -> usize {
        match &self.repr {
            Repr::Inline(0) => 0,
            Repr::Inline(value) => u64::BITS as usize - value.leading_zeros() as usize,
            Repr::Heap(limbs) => {
                let high = *limbs.last().unwrap();
                (limbs.len() - 1) * 64 + (u64::BITS as usize - high.leading_zeros() as usize)
            }
        }
    }

    pub fn shl_bits(&self, bits: usize) -> Self {
        if self.is_zero() {
            return Self::zero();
        }

        if bits == 0 {
            return self.clone();
        }

        let limb_shift = bits / 64;
        let bit_shift = bits % 64;
        let limbs = self.limbs();
        let mut out = vec![0_u64; limbs.len() + limb_shift + usize::from(bit_shift != 0)];

        let mut carry = 0_u64;
        for (index, limb) in limbs.iter().copied().enumerate() {
            let target = index + limb_shift;
            out[target] = (limb << bit_shift) | carry;
            carry = if bit_shift == 0 {
                0
            } else {
                limb >> (64 - bit_shift)
            };
        }

        if bit_shift != 0 {
            out[limbs.len() + limb_shift] = carry;
        }

        Self::from_limbs(out)
    }

    fn limbs(&self) -> &[u64] {
        match &self.repr {
            Repr::Inline(0) => &[],
            Repr::Inline(value) => slice::from_ref(value),
            Repr::Heap(limbs) => limbs.as_slice(),
        }
    }

    fn as_u64(&self) -> Option<u64> {
        match &self.repr {
            Repr::Inline(value) => Some(*value),
            Repr::Heap(_) => None,
        }
    }

    fn shr1(&self) -> Self {
        if self.is_zero() {
            return Self::zero();
        }

        let limbs = self.limbs();
        let mut out = vec![0_u64; limbs.len()];
        let mut carry = 0_u64;

        for (index, limb) in limbs.iter().copied().enumerate().rev() {
            out[index] = (limb >> 1) | carry;
            carry = (limb & 1) << 63;
        }

        Self::from_limbs(out)
    }

    fn from_limbs(mut limbs: Vec<u64>) -> Self {
        while limbs.last().copied() == Some(0) {
            limbs.pop();
        }

        match limbs.len() {
            0 => Self::zero(),
            1 => Self::from_u64(limbs[0]),
            _ => Self {
                repr: Repr::Heap(limbs),
            },
        }
    }
}

impl Ord for BigUintCore {
    fn cmp(&self, other: &Self) -> Ordering {
        let lhs_limbs = self.limbs();
        let rhs_limbs = other.limbs();

        match lhs_limbs.len().cmp(&rhs_limbs.len()) {
            Ordering::Equal => lhs_limbs.iter().rev().cmp(rhs_limbs.iter().rev()),
            ordering => ordering,
        }
    }
}

impl PartialOrd for BigUintCore {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
