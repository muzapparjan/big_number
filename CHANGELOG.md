# Changelog

## 1.0.0 - 2026-05-08

- implemented a custom `BigUintCore` / `BigIntCore` bigint engine
- implemented `BigFixed<const SCALE: u32>` with decimal fixed-point semantics
- added exact arithmetic, rounding, reciprocal, powers, `sqrt`, and `nth_root`
- implemented bigint-backed `ln`, `log10`, `exp`, `sin`, `cos`, and `tan`
- made `MathContext::guard_digits` part of the real transcendental precision model
- added oracle-backed tests, property tests, and deterministic transcendental sweeps
- added Criterion benchmarks for core arithmetic, fixed-point operations, parsing/formatting, and transcendental guard-digit costs
- added README and release workflow documentation