# Pro-rata Payout Mathematics

This document specifies the rational math requirements for calculating investor shares and payouts based on the `FundingCloseSnapshot`.

## 🧮 Core Formula

The share of an investor is determined by their contribution relative to the **total principal** captured at the moment funding closed.

$$Share_{investor} = \frac{Contribution_{investor}}{TotalPrincipal_{snapshot}}$$

### Why `TotalPrincipal`?
Starfund escrows allow **over-funding** (deposits that exceed the `funding_target` in the same ledger). The `TotalPrincipal` in the `FundingCloseSnapshot` is the authoritative denominator for all pro-rata calculations.

## Rounding Policy

Investor payouts use floor-rounded pro-rata math. If a positive yield would
otherwise leave an investor with principal only, the contract adds one token
base unit as the minimum coupon. This avoids erasing a positive yield entirely
for small positions. Consequently, the aggregate claim liability may exceed the
nominal floor-rounded pool by at most one base unit per investor. Use
`get_settlement_pool()` for the exact aggregate claim liability.

## 📝 Example Calculation

**Scenario:**
- `funding_target`: 10,000 USDC
- `TotalPrincipal` (at close): 10,050 USDC (due to over-funding)
- `Investor A Contribution`: 1,000 USDC
- `Yield`: 500 USDC (5% fixed yield for this example)
- `Total Settle Amount`: 10,550 USDC

**Calculation:**
1.  **Pro-rata Share**: $1,000 / 10,050 \approx 0.09950248756...$
2.  **Gross Payout**: $0.09950248756 * 10,550 = 1,049.75124378...$
3.  **Final Payout**: the base-unit floor, plus one unit only if this investor's fractional remainder is among the largest remainders selected for aggregate allocation.

## ⚠️ Security Notes

### Pro-rata Denominator Stability
The `FundingCloseSnapshot` is **immutable** once written. It is captured in two scenarios:
1.  **Full Funding**: Automatically written when `funded_amount >= funding_target`.
2.  **Partial Settlement**: Explicitly written by an Admin or SME via `partial_settle` for under-funded invoices.

Once captured, the pro-rata denominator remains fixed even if the SME withdraws a partial amount or more funds are somehow transferred to the contract. This ensures predictable payouts regardless of how the funding phase ended.

### Integer Overflow
When implementing off-chain in JS/Python, ensure you are using libraries that handle large integers (e.g., `BigInt` in JS) to prevent overflow during the `Contribution * TotalSettle` multiplication step before division.

```javascript
// Example JS implementation
function calculatePayout(contribution, totalPrincipal, settleAmount) {
  const c = BigInt(contribution);
  const tp = BigInt(totalPrincipal);
  const sa = BigInt(settleAmount);
  
  // Multiply before divide for precision
  return (c * sa) / tp; 
}
```

## 🔗 On-Chain View: `compute_investor_payout`

The contract exposes an authoritative on-chain implementation of the formula above:

```
StarfundEscrow::compute_investor_payout(investor: Address) → i128
```

This view derives `effective_yield_bps` from `DataKey::InvestorEffectiveYield` (tiered ladder
selection from `fund_with_commitment`) and falls back to `InvoiceEscrow::yield_bps` for investors
who used plain `fund`. Off-chain tools **must** call this view rather than re-implementing the
formula to guarantee identical rounding.

### On-chain integer safety

- All intermediate multiplications use `i128::checked_mul`.
- All divisions use `i128::checked_div`.
- The function panics with `"compute_investor_payout: arithmetic overflow"` on overflow rather than
  silently returning a wrong value.
- `total_principal` is always positive when a `FundingCloseSnapshot` exists; the function
  guards against the `≤ 0` edge case and returns `0` early.

### Reference

See `docs/escrow-read-api.md` → `compute_investor_payout` for the full parameter, return-value,
and authorization documentation.

## ✅ Aggregate Payout Invariant (issue #483)

### Uniform yield

When all investors share the same `yield_bps`:

```
nominal_pool = total_principal + floor(total_principal × yield_bps / 10_000)
Σ payout_i ≤ nominal_pool + n_investors
```

The additional bound accounts for at most one minimum-coupon unit per investor.
`get_settlement_pool()` returns the exact sum of investor payouts and is the
authoritative funding target for claims.

### Tiered / mixed yield

When investors carry per-investor effective yields (from `fund_with_commitment` tier selection),
each investor `i` has their own `settle_pool_i`:

```
settle_pool_i = total_principal + floor(total_principal × effective_yield_bps_i / 10_000)
payout_i      = floor(contribution_i × settle_pool_i / total_principal)
if effective_yield_bps_i > 0 and payout_i == contribution_i:
    payout_i += 1
```

The minimum-coupon rule can add at most one base unit to each floor-rounded
entitlement, so:

```
Σ payout_i ≤ Σ exact_i + n_investors
           ≤ total_principal × (1 + max_effective_yield_bps / 10_000) + n_investors
```

> ⚠️ **The nominal base-yield pool is not the tier-weighted pool.** The single `settled_pool`
> quoted above (`total_principal + total_principal × yield_bps / 10_000`) is only valid
> when every investor shares the same effective yield. Under mixed tiers it is a **lower
> bound** on the real liability (and minimum-coupon units may increase it further), and repaying only that amount leaves the contract
> under-funded for the last claimants. `get_settlement_pool()` therefore returns the
> **exact aggregate sum** `Σ payout_i`, not the nominal base-yield figure — see
> [On-Chain Aggregate View](#-on-chain-aggregate-view-get_settlement_pool).

### Rounding guarantee

Floor rounding favors the contract; the one-base-unit minimum coupon intentionally
favors each investor whose positive yield would otherwise be rounded away. The
aggregate claim liability remains exactly queryable through `get_settlement_pool()`.

### Snapshot-denominator consistency

`FundingCloseSnapshot` is written exactly once when the escrow first reaches `status == 1`.
It is stored under `DataKey::FundingCloseSnapshot` and **never overwritten**. All
`compute_investor_payout` calls read the same `total_principal` denominator from this snapshot,
so the denominator cannot shift between investor claims.

This invariant is verified in `escrow/src/tests/properties.rs` by
`snapshot_denominator_consistent_across_all_payout_reads`, which reads the snapshot before
and after every individual payout call and asserts identity.

### Property-based test coverage (issue #483)

| Test | Coverage |
|------|----------|
| `prop_payout_sum_le_settle_pool` | Uniform yield, 2–6 investors, arbitrary amounts & yield; exact pool conservation |
| `prop_aggregate_payout_le_settle_pool_tiered` | Tiered/mixed yield, snapshot consistency, max-pool bound |
| `payout_single_investor_equals_settle_pool` | Single investor receives exact `settle_pool` |
| `payout_equal_split_conservation` | Equal contributions, minimum-coupon aggregate bound |
| `payout_zero_yield_returns_principal_only` | Zero yield: `payout_i == contribution_i` |
| `payout_max_yield_conservation` | 100% yield: conservation still holds |
| `payout_prime_denominator_residue_bounded` | Prime total, largest-remainder conservation |
| `payout_highly_skewed_contributions` | 99%/1% split; residue bounded |
| `payout_many_small_investors_conservation` | 8 investors × 1 unit; minimum coupon per investor |
| `payout_single_large_single_tiny` | Extreme asymmetry stress test |
| `payout_tiered_mixed_yield_conservation` | 3-tier mixed yield; per-investor & exact aggregate allocation |
| `snapshot_denominator_consistent_across_all_payout_reads` | Snapshot immutable across all reads |
| `fuzz_payout_conservation_multi_investor` | 64-case fuzz, 1–8 investors, full yield range |

## 🔗 On-Chain Payout Transfer: `claim_investor_payout`

When a settled investor calls `claim_investor_payout`, the contract:

1. Computes the gross payout via `compute_investor_payout`.
2. Guards against a zero payout (floor division edge case — `PayoutZero = 165`).
3. Marks the investor as claimed (persistent storage write).
4. Calls `external_calls::transfer_funding_token_with_balance_checks` to transfer the gross
   payout from the contract to the investor.
5. Emits `InvestorPayoutClaimed`.

### Atomicity

The claimed marker is written **before** the token transfer. If the transfer fails (e.g.,
insufficient contract balance, token transfer error, host trap), the Soroban host rolls back
all storage writes including the marker. The investor may retry.

### Idempotency

A second call from the same investor returns early (the marker is already set) and does **not**
re-transfer. The balance check in `transfer_funding_token_with_balance_checks` would fail a
second transfer anyway, but the early return avoids the call entirely.

## 🔗 On-Chain Aggregate View: `get_settlement_pool`

```
StarfundEscrow::get_settlement_pool(env) → i128
```

Returns the **total pool** the SME must repay to fully satisfy all investors, computed
entirely on-chain by enumerating the recorded investor set ([`DataKey::InvestorIndex`]) and
summing each investor's own effective-yield payout:

```text
settle_pool_i = total_principal + floor(total_principal × effective_yield_bps_i / 10_000)
base_i        = floor(contribution_i × settle_pool_i / total_principal)
payout_i      = base_i + largest_remainder_adjustment_i

get_settlement_pool() = Σ_i payout_i
```

The per-investor base amounts are floored, then `floor(sum(remainder_i) / total_principal)`
additional base units are distributed to the largest remainders (funding order breaks ties).
Therefore the sum exactly matches the aggregate liability rounded down once.

### Why this view exists

`compute_investor_payout` derives a per-investor share, including any allocated remainder.
SME repayment tooling and dashboards can use `get_settlement_pool` to read the authoritative
aggregate in a single host invocation.

### Yield note: base-yield pool vs. tier-weighted pool

`settle()` computes its `settle_pool` in both `SettlementResult` and `EscrowSettled` from the
aggregate investor liability:

```text
settle_pool = get_settlement_pool()
```

For uniform yield this equals the standard base-yield pool. For mixed tiers it includes each
investor's selected yield, and `settle()` verifies the contract balance covers the amount.

### Cost

O(n) over [`DataKey::InvestorIndex`], which is hard-capped at `MAX_UNIQUE_INVESTORS` (10 000)
by `fund_impl`. Both `settle()` and individual payout reads use this bounded calculation.

### Return value

| Condition | Returns |
|-----------|---------|
| [`DataKey::FundingCloseSnapshot`] absent (escrow not yet funded) | `0` |
| `total_principal <= 0` (degenerate snapshot) | `0` |
| No recorded investor (defensive fallback) | `total_principal + floor(total_principal × yield_bps / 10_000)` |
| Funded state with investors | `Σ_i payout_i` (tier-weighted total liability) |

### Overflow safety

All intermediate multiplications use `i128::checked_mul`; divisions use `i128::checked_div`.
Emits [`EscrowError::ComputePayoutArithmeticOverflow`] (code 129) rather than silently
producing a wrong value.

### Authorization

None — pure read; no auth required and no state mutation.

See `docs/escrow-read-api.md` → `get_settlement_pool` for the complete parameter and return
documentation.
