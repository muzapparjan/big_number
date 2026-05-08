//! Fixed-scale decimal big-number primitives.
//!
//! The crate exposes a decimal fixed-point type backed by a custom bigint core.
//! Values are represented as an integer mantissa with a compile-time decimal scale.
//!
//! # Examples
//!
//! Basic parsing and arithmetic:
//!
//! ```rust
//! use std::str::FromStr;
//!
//! use big_number::BigFixed;
//!
//! let left = BigFixed::<2>::from_str("12.34")?;
//! let right = BigFixed::<2>::from_str("-2.50")?;
//!
//! assert_eq!((left.clone() + right.clone()).to_string(), "9.84");
//! assert_eq!((left * right).to_string(), "-30.85");
//! # Ok::<(), big_number::ParseBigFixedError>(())
//! ```
//!
//! Explicit scale conversion:
//!
//! ```rust
//! use std::str::FromStr;
//!
//! use big_number::BigFixed;
//!
//! let value = BigFixed::<2>::from_str("12.34")?;
//! assert_eq!(value.rescale::<4>().to_string(), "12.3400");
//! # Ok::<(), big_number::ParseBigFixedError>(())
//! ```
//!
//! Context-aware rounding for division and square root:
//!
//! ```rust
//! use std::str::FromStr;
//!
//! use big_number::{BigFixed, MathContext, RoundingMode};
//!
//! let context = MathContext {
//!     rounding: RoundingMode::Ceil,
//!     guard_digits: 8,
//! };
//!
//! let quotient = BigFixed::<2>::from_str("1.00")?
//!     .checked_div_with_context(&BigFixed::<2>::from_str("8.00")?, context)
//!     .unwrap();
//! assert_eq!(quotient.to_string(), "0.13");
//!
//! let recip = BigFixed::<2>::from_str("8.00")?
//!     .checked_recip_with_context(context)
//!     .unwrap();
//! assert_eq!(recip.to_string(), "0.13");
//!
//! let root = BigFixed::<2>::from_str("2.00")?
//!     .checked_sqrt_with_context(context)
//!     .unwrap();
//! assert_eq!(root.to_string(), "1.42");
//! # Ok::<(), big_number::ParseBigFixedError>(())
//! ```
//!
//! Higher roots and explicit nth-root evaluation:
//!
//! ```rust
//! use std::str::FromStr;
//!
//! use big_number::{BigFixed, MathContext, RoundingMode};
//!
//! let cube = BigFixed::<2>::from_str("8.00")?;
//! assert_eq!(cube.nth_root(3).to_string(), "2.00");
//!
//! let context = MathContext {
//!     rounding: RoundingMode::Ceil,
//!     guard_digits: 8,
//! };
//! let value = BigFixed::<2>::from_str("2.00")?;
//! assert_eq!(
//!     value.checked_nth_root_with_context(3, context).unwrap().to_string(),
//!     "1.26"
//! );
//! # Ok::<(), big_number::ParseBigFixedError>(())
//! ```
//!
//! Bigint-backed transcendental functions use a wider internal working scale
//! derived from `MathContext::guard_digits`, then round back to the public scale:
//!
//! ```rust
//! use std::str::FromStr;
//!
//! use big_number::{BigFixed, MathContext, RoundingMode};
//!
//! let context = MathContext {
//!     rounding: RoundingMode::HalfEven,
//!     guard_digits: 12,
//! };
//!
//! let value = BigFixed::<4>::from_str("2.0000")?;
//! assert_eq!(value.checked_ln_with_context(context).unwrap().to_string(), "0.6931");
//! assert_eq!(value.checked_log10_with_context(context).unwrap().to_string(), "0.3010");
//! assert_eq!(BigFixed::<4>::from_str("1.0000")?.checked_exp_with_context(context).unwrap().to_string(), "2.7183");
//! # Ok::<(), big_number::ParseBigFixedError>(())
//! ```
mod core;
mod error;
mod fixed;
mod int;

pub use crate::core::BigUintCore;
pub use crate::error::ParseBigFixedError;
pub use crate::fixed::{BigFixed, MathContext, RoundingMode};
pub use crate::int::{BigIntCore, Sign};
