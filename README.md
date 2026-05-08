# big_number

`big_number` is a decimal fixed-point big-number library built on a custom bigint core.

The public type is `BigFixed<const SCALE: u32>`. A value is represented as an integer mantissa divided by `10^SCALE`, so decimal scale is fixed at compile time and rounding stays explicit.

## What Is Exact

These operations stay inside the fixed-point model and round only where the public API says they should:

- parsing and formatting
- addition and subtraction
- multiplication
- division and reciprocal
- `round`, `floor`, `ceil`, `trunc`, and `rescale`
- `powu`, `powi`
- `sqrt` and `nth_root`

These are backed by the crate's own `BigUintCore` and `BigIntCore` implementation.

## What Is Iterative

These functions are still bigint-backed, but they are iterative rather than algebraically exact:

- `ln`
- `log10`
- `exp`
- `sin`
- `cos`
- `tan`

They do **not** fall back to `f32` or `f64`. Instead, they evaluate at a wider internal working scale and only round once when converting back to the requested public `SCALE`.

## MathContext And guard_digits

`MathContext` currently controls:

- `rounding`: final public rounding mode
- `guard_digits`: extra internal decimal digits used by iterative evaluation

Example:

```rust
use std::str::FromStr;

use big_number::{BigFixed, MathContext, RoundingMode};

let context = MathContext {
    rounding: RoundingMode::HalfEven,
    guard_digits: 12,
};

let value = BigFixed::<6>::from_str("2.000000").unwrap();
let ln = value.checked_ln_with_context(context).unwrap();

assert_eq!(ln.to_string(), "0.693147");
```

Increasing `guard_digits` usually improves stability for `ln`, `exp`, and trigonometric functions, but it also increases cost.

## Domain Rules

- `checked_div*` and `checked_recip*` return `None` on division by zero.
- `checked_sqrt*` returns `None` for negative inputs.
- `checked_nth_root*` returns `None` for degree `0`, and for even roots of negative inputs.
- `checked_ln*` and `checked_log10*` return `None` for non-positive inputs.
- `checked_tan*` returns `None` when the input is at a pole within the current representable precision.

## Examples

```rust
use std::str::FromStr;

use big_number::BigFixed;

let left = BigFixed::<2>::from_str("12.34").unwrap();
let right = BigFixed::<2>::from_str("-2.50").unwrap();

assert_eq!((left.clone() + right.clone()).to_string(), "9.84");
assert_eq!((left * right).to_string(), "-30.85");
```

```rust
use std::str::FromStr;

use big_number::{BigFixed, MathContext, RoundingMode};

let context = MathContext {
    rounding: RoundingMode::Ceil,
    guard_digits: 12,
};

let value = BigFixed::<4>::from_str("2.0000").unwrap();

assert_eq!(value.checked_log10_with_context(context).unwrap().to_string(), "0.3010");
assert_eq!(value.checked_exp_with_context(context).unwrap().to_string(), "7.3891");
```

## Validation

Run the full test suite:

```powershell
cargo test
```

Run doctests only:

```powershell
cargo test --doc
```

## Benchmarks

This repository includes a Criterion benchmark target at `benches/benchmark_suite.rs`.

It covers:

- `BigUintCore` multiply, divide, and `sqrt`
- `BigFixed` exact multiply, divide, `sqrt`, and `nth_root`
- parse/format throughput
- `ln`, `exp`, and `sin` cost across multiple `guard_digits` settings

Run the full suite:

```powershell
cargo bench --bench benchmark_suite
```

Run a single benchmark family:

```powershell
cargo bench --bench benchmark_suite transcendentals_guard_digits
```

Run a single benchmark with a smaller sample size while iterating locally:

```powershell
cargo bench --bench benchmark_suite parse_scale_6 -- --sample-size 10
```

## License

This crate uses a custom source-available license in `LICENSE.md`.

In short:

- anyone may use the crate freely, including in commercial applications
- attribution and license preservation are required
- private modification for internal use is allowed
- republishing this library itself, or a modified version of this library, as another library/package is not allowed without permission

This is intentionally not a standard OSI open-source license, because the project does not allow modified re-release of the library itself.

If `gnuplot` is not installed, Criterion will automatically fall back to the plotters backend.

## Repository Notes

- Production code uses the custom bigint core, not `num-bigint`.
- `num-bigint` and `num-bigfloat` are only used in tests and differential validation.
- The architecture rationale lives in `docs/architecture.md`.