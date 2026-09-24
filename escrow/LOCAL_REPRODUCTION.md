# Local Reproduction Guide for Coverage Enforcement

This document describes how to locally reproduce the CI coverage check for the escrow crate.

## Prerequisites

```bash
# Install Rust toolchain with required components
rustup component add rustfmt clippy llvm-tools-preview
rustup target add wasm32v1-none

# Install cargo-llvm-cov
cargo install cargo-llvm-cov
```

## Running the Coverage Check

### From workspace root:
```bash
cd /path/to/Starfund-contracts
cargo llvm-cov --features testutils --summary-only -p starfund_escrow
```

### From escrow directory:
```bash
cd escrow
cargo llvm-cov --features testutils --summary-only
```

## Expected Output

The command outputs coverage statistics. CI currently treats this as a report-only step, so coverage does not enforce a minimum threshold.

Example successful output:
```
lib.rs: 98.17% lines covered
Total: 93.48% lines covered
```

## Troubleshooting

### Workspace Configuration Issue
If you encounter "multiple workspace roots found" error when running `cargo fmt`:
```bash
# Workaround: Run from escrow directory instead
cd escrow
cargo fmt -- --check
```

### Failing Tests
Some tests are marked as `#[ignore]` due to edge cases in investor cap logic:
- 46 tests across the escrow test modules

These tests can be run with:
```bash
cargo test -- --ignored
```

## CI Configuration

The CI is configured in `.github/workflows/ci.yml` to run:
1. `cargo fmt --all -- --check`
2. `cargo clippy -p starfund_escrow -- -D warnings`
3. `cargo build`
4. `cargo test`
5. `cargo llvm-cov --features testutils --summary-only -p starfund_escrow`

## Notes

- Current coverage: **98.17%** for `lib.rs` (main contract code)
- CI coverage is **report-only** and does not apply a line threshold
- **46 tests** are currently ignored while latent test/API drift and edge cases are investigated