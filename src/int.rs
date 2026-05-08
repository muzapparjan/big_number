use std::cmp::Ordering;

use crate::core::BigUintCore;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum Sign {
    Negative,
    Zero,
    Positive,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BigIntCore {
    sign: Sign,
    magnitude: BigUintCore,
}

impl Default for BigIntCore {
    fn default() -> Self {
        Self::zero()
    }
}

impl BigIntCore {
    pub fn zero() -> Self {
        Self {
            sign: Sign::Zero,
            magnitude: BigUintCore::zero(),
        }
    }

    pub fn from_i64(value: i64) -> Self {
        match value.cmp(&0) {
            Ordering::Less => {
                Self::from_parts(Sign::Negative, BigUintCore::from_u64(value.unsigned_abs()))
            }
            Ordering::Equal => Self::zero(),
            Ordering::Greater => {
                Self::from_parts(Sign::Positive, BigUintCore::from_u64(value as u64))
            }
        }
    }

    pub fn from_parts(sign: Sign, magnitude: BigUintCore) -> Self {
        if magnitude.is_zero() || sign == Sign::Zero {
            return Self::zero();
        }

        Self { sign, magnitude }
    }

    pub fn sign(&self) -> Sign {
        self.sign
    }

    pub fn magnitude(&self) -> &BigUintCore {
        &self.magnitude
    }

    pub fn is_zero(&self) -> bool {
        self.sign == Sign::Zero
    }

    pub fn abs(&self) -> Self {
        Self::from_parts(Sign::Positive, self.magnitude.clone())
    }

    pub fn negated(&self) -> Self {
        match self.sign {
            Sign::Negative => Self::from_parts(Sign::Positive, self.magnitude.clone()),
            Sign::Zero => Self::zero(),
            Sign::Positive => Self::from_parts(Sign::Negative, self.magnitude.clone()),
        }
    }

    pub fn add(&self, rhs: &Self) -> Self {
        match (self.sign, rhs.sign) {
            (Sign::Zero, _) => rhs.clone(),
            (_, Sign::Zero) => self.clone(),
            (Sign::Positive, Sign::Positive) => {
                Self::from_parts(Sign::Positive, self.magnitude.add(&rhs.magnitude))
            }
            (Sign::Negative, Sign::Negative) => {
                Self::from_parts(Sign::Negative, self.magnitude.add(&rhs.magnitude))
            }
            (Sign::Positive, Sign::Negative) => self.sub_magnitude(rhs),
            (Sign::Negative, Sign::Positive) => rhs.sub_magnitude(self),
        }
    }

    pub fn sub(&self, rhs: &Self) -> Self {
        self.add(&rhs.negated())
    }

    pub fn mul(&self, rhs: &Self) -> Self {
        match (self.sign, rhs.sign) {
            (Sign::Zero, _) | (_, Sign::Zero) => Self::zero(),
            (Sign::Positive, Sign::Positive) | (Sign::Negative, Sign::Negative) => {
                Self::from_parts(Sign::Positive, self.magnitude.mul(&rhs.magnitude))
            }
            (Sign::Positive, Sign::Negative) | (Sign::Negative, Sign::Positive) => {
                Self::from_parts(Sign::Negative, self.magnitude.mul(&rhs.magnitude))
            }
        }
    }

    fn sub_magnitude(&self, rhs: &Self) -> Self {
        match self.magnitude.cmp(&rhs.magnitude) {
            Ordering::Greater => Self::from_parts(self.sign, self.magnitude.sub(&rhs.magnitude)),
            Ordering::Equal => Self::zero(),
            Ordering::Less => Self::from_parts(rhs.sign, rhs.magnitude.sub(&self.magnitude)),
        }
    }
}

impl Ord for BigIntCore {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.sign.cmp(&other.sign) {
            Ordering::Equal => match self.sign {
                Sign::Negative => other.magnitude.cmp(&self.magnitude),
                Sign::Zero => Ordering::Equal,
                Sign::Positive => self.magnitude.cmp(&other.magnitude),
            },
            ordering => ordering,
        }
    }
}

impl PartialOrd for BigIntCore {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
