# Releasing

This repository is set up for a normal Cargo crate release.

## Pre-release Checklist

1. Confirm that the crate name and version are publishable on crates.io.
2. If the project has a public repository, add `repository` and optionally `homepage` to `Cargo.toml`.
3. Review `LICENSE.md` and confirm its terms still match the intended distribution model.
4. Run the full validation suite:

```powershell
cargo test
cargo test --doc
cargo clippy --all-targets --all-features -- -D warnings
cargo bench --bench benchmark_suite
```

5. Review `CHANGELOG.md` and adjust the release date or release notes if needed.
6. Re-run package validation:

```powershell
cargo package
```

## Publish

After the checklist is complete:

```powershell
cargo publish
```

## Notes

- `cargo package` should be clean once `LICENSE.md` is present and referenced by `license-file`.
- `cargo publish --dry-run` currently reports that `big_number` already exists on crates.io, so the package name may need to change unless you control that crate.
- The crate passes `cargo test`, `cargo test --doc`, and `cargo clippy --all-targets --all-features -- -D warnings`.
- Benchmarks are provided through `benches/benchmark_suite.rs`.