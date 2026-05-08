# Big Number Architecture

## Goal

Build a high-performance decimal big-number library for Rust with these properties:

- fixed maximum fractional digits
- predictable decimal semantics
- support for most common mathematical operations
- architecture suitable for long-term optimization, not a prototype wrapper over an existing bigint crate

This document records the selected implementation strategy and the reasons behind it.

## Final Choice

The library should use a fixed-scale decimal model backed by a custom big-integer core.

Conceptually:

```text
value = mantissa / 10^SCALE
```

Where:

- `mantissa` is a signed arbitrary-precision integer
- `SCALE` is fixed for the number type

Recommended public type shape:

```rust
BigFixed<const SCALE: u32>
```

This is a decimal fixed-point design, not a dynamic-scale decimal and not a floating-point format.

## Why This Model

This choice best matches the stated requirements.

### Efficiency

With fixed scale, addition, subtraction, and comparison do not need per-value decimal alignment. The implementation can focus on big-integer throughput instead of dynamic decimal bookkeeping.

### Predictable Semantics

All values of the same type share the same fractional precision contract. That makes parsing, formatting, rounding, division, and higher math functions easier to reason about and easier to keep consistent.

### Decimal Correctness

The external model is decimal, so values such as `0.1` and `1.23` are represented naturally. This avoids the semantic mismatch of binary floating-point and binary fixed-point for decimal-facing workloads.

## Rejected Alternatives

### Dynamic-Scale BigDecimal

Form: `value = mantissa * 10^-scale`, with each value carrying its own scale.

Pros:

- flexible
- familiar decimal model
- natural for parsing and arbitrary incoming precision

Cons:

- repeated scale alignment in common arithmetic
- normalization overhead becomes part of many hot paths
- weaker fit for a library whose core contract is fixed fractional precision

Decision: reject as the core representation.

### BCD or Decimal-Digit Storage

Form: store decimal digits or packed decimal digits directly.

Pros:

- intuitive decimal representation
- some formatting operations are straightforward

Cons:

- poor arithmetic throughput compared with binary limbs
- worse memory density
- more expensive multiplication, division, and advanced algorithms

Decision: reject for a performance-oriented core.

### Rational Core

Form: numerator and denominator as arbitrary-precision integers.

Pros:

- exact for many operations

Cons:

- numerator and denominator growth is difficult to control
- output still must be quantized back to fixed decimal digits
- too expensive for the intended primary representation

Decision: reject as the main numeric model.

### Binary Fixed-Point

Form: `value = mantissa / 2^k`.

Pros:

- efficient bit-level operations
- attractive for some internal numerical workloads

Cons:

- poor match for decimal I/O and decimal business semantics
- awkward exact representation of common decimal inputs

Decision: reject as the user-facing core format.

## About `num-bigint`

`num-bigint` is the standard Rust ecosystem crate for arbitrary-precision integers. It provides:

- `BigUint` for unsigned big integers
- `BigInt` for signed big integers
- operator overloading for common arithmetic
- parsing, formatting, conversions, and optional features such as `serde` and random generation

It is mature and useful, but it is not the right final core for this library.

### Why `num-bigint` Is Not the Final Backend

`num-bigint` is designed as a general-purpose bigint crate. That means:

- its storage and APIs are optimized for broad applicability, not fixed-scale decimal specialization
- it does not encode the decimal fixed-point contract we need
- it does not provide the rounding, quantization, and high-level math policy layer required by this library
- it limits how far we can push small-object optimization and fixed-scale fast paths

### Why `num-bigint` Is Still Useful as a Reference

It remains useful for:

- validating expected bigint behavior
- comparing operator ergonomics
- checking edge-case semantics for parsing and conversion
- benchmarking the custom core against a widely used baseline

Decision: use `num-bigint` as a reference point, not as the long-term implementation foundation.

## Core Representation

The implementation should be layered.

### Layer 1: `BigUintCore`

Unsigned arbitrary-precision integer using binary limbs.

Recommended representation:

- small-object inline storage for small magnitudes
- heap-backed `Vec<u64>` limbs for larger magnitudes
- little-endian limb order internally

Responsibilities:

- normalization and removal of leading zero limbs
- comparison
- add and subtract magnitude
- multiply and divide magnitude
- shifts and bit-length operations
- multiplication and division by small integers
- decimal power helpers used by fixed-point operations

### Layer 2: `BigIntCore`

Signed integer wrapper around `BigUintCore`.

Recommended representation:

- `Sign` enum: negative, zero, positive
- magnitude stored separately

Responsibilities:

- signed arithmetic composition
- sign normalization so zero always has a canonical sign
- signed comparison

### Layer 3: `BigFixed<SCALE>`

Fixed-scale decimal type wrapping `BigIntCore`.

Responsibilities:

- parsing from strings and primitive numbers
- formatting to strings
- fixed-scale arithmetic semantics
- rounding and quantization
- higher mathematical functions
- conversion between scales through explicit APIs only

## Why Binary Limbs Instead of Decimal Limbs

The library is decimal in meaning, but arithmetic should be performed on binary limbs.

Reasons:

- `u64` limb arithmetic maps well to modern CPUs
- addition, multiplication, carry propagation, and comparison are faster than decimal-digit math
- high-performance bigint algorithms are naturally expressed on binary limbs
- decimal formatting and parsing can remain boundary operations instead of infecting the whole arithmetic core

Decimal semantics belong at the fixed-point layer. Binary-limb throughput belongs at the integer core.

## Type-Level Fixed Scale

Use const generics for the public scale parameter.

```rust
pub struct BigFixed<const SCALE: u32> {
    mantissa: BigIntCore,
}
```

Why type-level scale is preferred:

- removes repeated runtime scale checks among equal-scale values
- improves API clarity
- allows compiler specialization
- avoids mixing incompatible precisions implicitly

Cross-scale operations should be explicit. For example, conversion from `BigFixed<2>` to `BigFixed<6>` is allowed, but should not happen silently inside generic arithmetic.

## Precision and Rounding Model

Fixed scale alone is not enough. The library must define how inexact results are reduced back to the target scale.

Recommended rounding modes:

- `HalfEven`
- `HalfUp`
- `Down`
- `Up`
- `Floor`
- `Ceil`

Recommended policy:

- exact operations preserve exact results when possible
- division, square root, logarithms, exponentials, trigonometric functions, and scale reductions may produce inexact intermediate values
- internal computations should use extra guard precision and round once at the final boundary

Suggested context type:

```rust
pub struct MathContext {
    pub rounding: RoundingMode,
    pub guard_digits: u32,
}
```

The public type keeps fixed `SCALE`, while the context controls rounding and the extra working precision used by transcendental evaluation. `guard_digits` must not be a dead field: implementations that need iterative refinement should evaluate at `SCALE + guard_digits + implementation_guard` and only round once at the public boundary.

## Required Algorithm Strategy

### Integer Core

`BigUintCore` should grow by tiers.

Small sizes:

- schoolbook addition and multiplication
- short division by single limb or small integers

Medium sizes:

- Karatsuba multiplication

Larger sizes, only when profiling justifies it:

- Toom-Cook or other asymptotically faster multiplication
- more advanced division algorithms

The implementation should remain profile-driven. Do not add expensive algorithmic complexity before size thresholds are measured.

### Fixed-Point Arithmetic

Addition and subtraction:

- operate directly on mantissas

Multiplication:

- multiply mantissas
- divide by `10^SCALE`
- apply rounding if necessary

Division:

- scale numerator by `10^SCALE`
- divide by denominator mantissa
- apply rounding if remainder exists

Modulo:

- define precisely for fixed-point values or restrict to integer-compatible cases first

### Elementary Functions

Square root:

- Newton iteration on scaled integers or fixed-point values

Integer powers:

- exponentiation by squaring

General exponential and logarithm:

- argument reduction
- iterative methods or carefully selected series expansion
- extra guard precision before final rounding
- repeated square-root reduction is acceptable for `ln` if it keeps the reduced argument close to `1`
- repeated halving plus squaring back up is acceptable for `exp` if the final public rounding still happens only once

Trigonometric functions:

- compute constants such as `pi` inside the bigint-backed engine, not via binary floating point
- reduce arguments modulo `2*pi` before series evaluation
- tie pole rejection for `tan` to the current representable precision, not to a hard-coded floating threshold

## Power-of-Ten Strategy

Because the format is decimal fixed-point, powers of ten are a core performance surface.

The library should provide:

- cached small `10^k` values
- efficient multiplication and division by `10^k`
- fast scale conversion helpers
- decimal parse and format paths that avoid repeated generic exponentiation

This is a better optimization target than over-engineering generic decimal-digit storage.

## Parsing and Formatting

Parsing should:

- accept optional sign
- accept integer and fractional parts
- reject precision beyond `SCALE` unless an explicit rounding-aware parse path is requested
- normalize zero consistently

Formatting should:

- always honor `SCALE` where required by the API
- provide trimmed and fixed-width variants
- avoid unnecessary temporary allocations in the hot path

Recommended API split:

- strict parse that rejects extra fractional digits
- context-aware parse that rounds into `SCALE`
- display modes for fixed and trimmed decimal output

## API Direction

Recommended initial public surface:

- construction from strings and primitive integers
- checked and exact conversions where possible
- `Add`, `Sub`, `Mul`, `Div`, `Neg`, `PartialOrd`, `Ord`, `Eq`
- `abs`, `signum`, `is_zero`
- `round`, `floor`, `ceil`, `trunc`
- `powi`, `sqrt`
- context-based `exp`, `ln`, `log10`

Functions that should likely come later:

- `sin`, `cos`, `tan`
- inverse trig functions
- gamma and other special functions

The key rule is to prioritize correctness and deterministic rounding semantics over surface-area growth.

## Error Model

The library should use explicit error types for:

- parse failure
- scale overflow or unsupported rescale
- division by zero
- domain errors such as square root of a negative value
- non-convergence in iterative math functions

Avoid panicking for normal invalid-input cases.

## Memory and Performance Considerations

Key performance decisions:

- inline storage for small magnitudes to reduce heap traffic
- `u64` limbs as the default arithmetic unit
- minimal normalization work in hot paths
- specialize multiply and divide by small constants and powers of ten
- avoid allocating temporary decimal strings during arithmetic

The implementation should be benchmarked across:

- small operands that fit in inline storage
- medium operands with a few limbs
- large operands where multiplication thresholds matter
- parse and format heavy workloads
- division and `sqrt` throughput

## Comparison Summary

| Approach | Decimal semantics | Fixed-scale fit | Performance ceiling | Final decision |
| --- | --- | --- | --- | --- |
| Dynamic-scale BigDecimal | strong | medium | medium | reject |
| BCD / decimal digits | strong | strong | low | reject |
| Rational core | strong | low | low | reject |
| Binary fixed-point | weak | medium | high | reject |
| Custom bigint + decimal fixed-point | strong | strong | high | select |

## Implementation Phases

### Phase 1

- `BigUintCore`
- `BigIntCore`
- `BigFixed<SCALE>` basic structure
- parse and format
- add, subtract, compare
- multiply and divide with rounding

### Phase 2

- small-object optimization tuning
- cached powers of ten
- `round`, `floor`, `ceil`, `trunc`
- `powi`
- `sqrt`

### Phase 3

- `exp`, `ln`, `log10`
- benchmark-driven algorithm thresholds
- broader conversion APIs

### Phase 4

- trigonometric functions
- additional numerical utilities
- serialization and optional ecosystem integrations

## Practical Rule for This Repository

When implementation begins, do not introduce `num-bigint` as the core dependency. If it is used at all, use it only in isolated experiments, benchmarks, or differential tests. The production library should be built around its own integer core from the start.

## Final Recommendation

The library should be built as a decimal fixed-point type with a custom binary-limb bigint engine and explicit rounding context. That design is the best match for high performance, fixed fractional precision, and broad mathematical capability.