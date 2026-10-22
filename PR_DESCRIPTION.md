# Fix collateral batch handling and recording guards

## Summary

- Preserve every commitment submitted in a collateral batch and expose the batch through a new read endpoint, while keeping the existing latest-commitment view.
- Reject single and batch collateral writes while the escrow is paused or not open.
- The collateral key-helper usage and canonical collateral-clear implementation were already present.

## Validation

- `git diff --check` passed.
- Rust formatting, tests, and GitHub Actions were not run: Cargo is unavailable in the local environment.

Closes ushpraise/Starfund-contracts#216
Closes ushpraise/Starfund-contracts#217
Closes ushpraise/Starfund-contracts#218
Closes ushpraise/Starfund-contracts#219
