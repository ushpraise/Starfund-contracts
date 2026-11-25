# Escrow Read API

Complete catalog of all public read-only views on `StarfundEscrow`. All functions are pure reads:
no state mutation, no authorization required unless specified otherwise.

**Integrator note:** Return types, defaults, and absent-key behavior documented for each view match
the on-chain implementation exactly. Off-chain tooling should use these views rather than
re-implementing storage reads to guarantee identical semantics.

---

## Index

**Core Escrow State:**
- [get_escrow](#get_escrow--invoiceescrow)
- [get_version](#get_version--u32)
- [get_escrow_summary](#get_escrow_summary--escrowsummary)

**Immutable Bindings:**
- [get_funding_token](#get_funding_token--address)
- [get_treasury](#get_treasury--address)
- [get_registry_ref](#get_registry_ref--optionaddress)

**Admin & Governance:**
- [get_pending_admin](#get_pending_admin--optionaddress)
- [get_pending_admin_expiry](#get_pending_admin_expiry--optionu64)
- [get_pending_admin_remaining_secs](#get_pending_admin_remaining_secs--optionu64)
- [get_pending_payer_recovery](#get_pending_payer_recovery--optionpendingpayerrecoveryview)
- [get_legal_hold](#get_legal_hold--bool)
- [get_legal_hold_clear_delay](#get_legal_hold_clear_delay--u64)
- [get_legal_hold_clearable_at](#get_legal_hold_clearable_at--optionu64)

**Funding Constraints:**
- [get_funding_deadline](#get_funding_deadline--optionu64)
- [is_funding_expired](#is_funding_expired--bool)
- [get_min_contribution_floor](#get_min_contribution_floor--i128)
- [get_max_unique_investors_cap](#get_max_unique_investors_cap--optionu32)
- [get_remaining_investor_slots](#get_remaining_investor_slots--optionu32)
- [get_max_per_investor_cap](#get_max_per_investor_cap--optioni128)

**Maturity & Settlement:**
- [has_maturity_lock](#has_maturity_lock--bool)
- [get_funding_close_snapshot](#get_funding_close_snapshot--optionfundingclosesnapshot)

**Tier Lookup:**
- [preview_yield_tier](#preview_yield_tieramount-i128-lock-u64--yieldresolution)

**Per-Investor State:**
- [get_contribution](#get_contributioninvestor-address--i128)
- [get_total_deposited](#get_total_depositedinvestor-address--i128)
- [get_total_deposited_with_provenance](#get_total_deposited_with_provenanceinvestor-address--deposittotalview)
- [get_unique_funder_count](#get_unique_funder_count--u32)
- [get_investor_yield_bps](#get_investor_yield_bpsinvestor-address--i64)
- [get_investor_claim_not_before](#get_investor_claim_not_beforeinvestor-address--u64)
- [is_investor_claimed](#is_investor_claimedinvestor-address--bool)
- [is_investor_refunded](#is_investor_refundedinvestor-address--bool)
- [compute_investor_payout](#compute_investor_payoutinvestor-address--i128)
- [get_claimable_payout](#get_claimable_payoutinvestor-address--i128)
- [get_settlement_pool](#get_settlement_pool--i128)

**Attestations:**
- [get_primary_attestation_hash](#get_primary_attestation_hash--optionbytesn32)
- [get_attestation_append_log](#get_attestation_append_log--vecbytesn32)
- [get_attestation_log_stats](#get_attestation_log_stats--u32-u32)
- [is_attestation_revoked](#is_attestation_revokedindex-u32--bool)

**Collateral Metadata:**
- [get_sme_collateral_commitment](#get_sme_collateral_commitment--optionsmecollateralcommitment)
- [get_collateral_state](#get_collateral_state--collateralstate)

**Allowlist:**
- [is_allowlist_active](#is_allowlist_active--bool)
- [is_investor_allowlisted](#is_investor_allowlistedinvestor-address--bool)

**Distributed Principal:**
- [get_distributed_principal](#get_distributed_principal--i128)
- [get_reconciliation](#get_reconciliation--reconciliationview)

**Fee Governance (Informational Only):**
- [get_active_fee_schedule](#get_active_fee_schedule--optionfeeschedule)
- [get_pending_fee_schedule](#get_pending_fee_schedule--optionfeeschedule)
- [get_previous_fee_schedule](#get_previous_fee_schedule--optionfeeschedule)

---

## Core Escrow State

### `get_escrow() → InvoiceEscrow`

**Storage key:** `DataKey::Escrow`  
**Signature:** `pub fn get_escrow(env: Env) -> InvoiceEscrow`

Returns the full escrow snapshot containing all core state fields.

**Requires initialization:** Yes — emits [`EscrowError::EscrowNotInitialized`] (code 20) if called before `init`.

**Return value:**
- `InvoiceEscrow` struct with fields: `invoice_id`, `admin`, `sme_address`, `amount`, `funding_target`, `funded_amount`, `yield_bps`, `maturity`, `status`.

---

### `get_version() → u32`

**Storage key:** `DataKey::Version`  
**Signature:** `pub fn get_version(env: Env) -> u32`

Returns the stored schema version written by `init` (see `SCHEMA_VERSION`).

**Requires initialization:** No  
**Default when absent:** `0`

**Return value:**
- `u32` schema version (current production: `6`).
- Returns `0` if called before `init`.

---

### `get_escrow_summary() → EscrowSummary`

**Signature:** `pub fn get_escrow_summary(env: Env) -> EscrowSummary`

Bundles multiple read-only values in a single host invocation, optimizing read latency and gas efficiency for off-chain indexers and frontend rendering.

**Requires initialization:** Yes — panics via `get_escrow` if escrow is not initialized.

**Return value:** `EscrowSummary` struct containing:
- `escrow: InvoiceEscrow` — Full escrow snapshot.
- `status_classification: EscrowStatus` — Human-readable lifecycle state; unknown numeric
  status codes are returned as `Unknown(code)` so migrations remain observable.
- `has_maturity_lock: bool` — True when `escrow.maturity > 0`.
- `legal_hold: bool` — True if compliance hold is active.
- `funding_close_snapshot: EscrowCloseSnapshot` — Custom option-like enum (`None` or `Some(FundingCloseSnapshot)`).
- `unique_funder_count: u32` — Distinct address count.
- `is_allowlist_active: bool` — Allowlist gate status.
- `schema_version: u32` — Contract schema version.
- `sme_collateral_commitment: CollateralCommitmentSnapshot` — Custom option-like enum (`None` or `Some(SmeCollateralCommitment)`).
- `has_primary_attestation: bool` — Primary attestation binding status.
- `attestation_log_length: u32` — Number of append-log entries.
- `paused: bool` — Operational pause flag; mirrors `is_paused()`. Reads `false` for instances that have never been paused.
- `protocol_fee_bps: i64` — Configured protocol fee in basis points; mirrors `get_protocol_fee_bps()`. Reads `0` for pre-fee instances.

The `paused` and `protocol_fee_bps` fields are read from the same storage keys (`DataKey::Paused`, `DataKey::ProtocolFeeBps`) as their standalone getters, so the summary can never drift from `is_paused()` / `get_protocol_fee_bps()`.

---

## Immutable Bindings

### `get_funding_token() → Address`

**Storage key:** `DataKey::FundingToken`  
**Signature:** `pub fn get_funding_token(env: Env) -> Address`

Returns the SEP-41 token contract address bound to this escrow instance at `init`.

**Immutable:** Set once at `init`; cannot change after deploy.  
**Requires initialization:** Yes — emits [`EscrowError::FundingTokenNotSet`] (code 21) if called before `init`.

**Return value:**
- `Address` of the funding token contract.
- This is the only token that `sweep_terminal_dust` may transfer to the treasury.

---

### `get_treasury() → Address`

**Storage key:** `DataKey::Treasury`  
**Signature:** `pub fn get_treasury(env: Env) -> Address`

Returns the protocol treasury address that receives terminal dust sweeps.

**Immutable:** Set once at `init`; cannot change after deploy.  
**Requires initialization:** Yes — emits [`EscrowError::TreasuryNotSet`] (code 22) if called before `init`.

**Return value:**
- `Address` of the treasury.
- The treasury must authorize `sweep_terminal_dust`; the admin cannot sweep unless it is also the treasury.

---

### `get_registry_ref() → Option<Address>`

**Storage key:** `DataKey::RegistryRef`  
**Signature:** `pub fn get_registry_ref(env: Env) -> Option<Address>`

Returns the optional registry contract address supplied at `init`, or `None` when absent.

**Immutable:** Set once at `init`; cannot change after deploy.  
**Requires initialization:** No  
**Default when absent:** `None`

**Non-authority model:**
- `RegistryRef` is a **read-only discoverability hint** for off-chain indexers only.
- No on-chain logic in this contract reads or calls this address.
- Its presence **does not** prove registry membership; call the registry contract directly to verify.
- The key is omitted from instance storage entirely when `registry = None` at `init`.

**Return value:**
- `Some(Address)` when a registry was configured.
- `None` otherwise.

---

## Admin & Governance

### `get_pending_admin() → Option<Address>`

**Storage key:** `DataKey::PendingAdmin`  
**Signature:** `pub fn get_pending_admin(env: Env) -> Option<Address>`

Returns the proposed successor admin waiting for `accept_admin`, or `None` when no handover is in progress.

**Requires initialization:** No  
**Default when absent:** `None`

**Return value:**
- `Some(Address)` when a handover is pending.
- `None` when no `propose_admin` has been issued, or after a successful `accept_admin`.

### `get_pending_admin_expiry() → Option<u64>`

**Storage key:** `DataKey::PendingAdminExpiry`

**Signature:** `pub fn get_pending_admin_expiry(env: Env) -> Option<u64>`

Returns the absolute ledger timestamp recorded by `propose_admin`, or `None` when no expiry has been recorded.

**Requires initialization:** No

**Default when absent:** `None`

**Return value:**
- `Some(timestamp)` when a handover proposal with an expiry exists.
- `None` before `propose_admin`, after `accept_admin`, after `cancel_pending_admin`, or when no expiry key is present.

### `get_pending_admin_remaining_secs() → Option<u64>`

**Storage keys:** `DataKey::PendingAdmin`, `DataKey::PendingAdminExpiry`

**Signature:** `pub fn get_pending_admin_remaining_secs(env: Env) -> Option<u64>`

Returns the pending-admin proposal's remaining validity window computed against `Env::ledger().timestamp()`.

**Requires initialization:** No

**Default when absent:** `None`

**Return value:**
- `None` when no pending admin proposal is active.
- `Some(expiry - now)` while `now < expiry`.
- `Some(0)` when `now >= expiry`, using saturating arithmetic.

**Boundary parity with `accept_admin`:**
- At `now == expiry`, this view returns `Some(0)` and `accept_admin` still accepts the proposal.
- At `now > expiry`, this view still returns `Some(0)` and `accept_admin` rejects with `AdminProposalExpired`.
- Pure read: no authorization, no storage writes, no TTL bump.

### `get_pending_payer_recovery() → Option<PendingPayerRecoveryView>`

**Storage keys:** `DataKey::PendingPayer`, `DataKey::PendingPayerExpiry`

**Signature:** `pub fn get_pending_payer_recovery(env: Env) -> Option<PendingPayerRecoveryView>`

Returns the active payer-recovery proposal — the pending replacement payer address and its
expiry — in a single read, so clients can display the recovery target and timing without
decoding storage directly.

**Requires initialization:** No

**Default when absent:** `None`

**Return value:**
- `Some(PendingPayerRecoveryView)` when a recovery proposal is pending, with fields:
  - `pending_payer: Address` — the proposed replacement payer (from `DataKey::PendingPayer`).
  - `expires_at: u64` — absolute ledger timestamp at which the proposal lapses (from `DataKey::PendingPayerExpiry`).
- `None` when no proposal is pending.

**Lifecycle (view transitions):**
- `None` before any recovery proposal is made.
- `Some(...)` after a proposal is recorded; a replacement proposal overwrites the view with the new payer and expiry.
- `None` again after the proposal is accepted (acceptance clears both keys).
- `None` again after the proposal is cancelled (cancellation clears both keys).

The view reads the same `PendingPayer` / `PendingPayerExpiry` keys written by the recovery
flow, so it can never drift from the stored proposal.

**Pure read:** no authorization, no storage writes, no TTL bump.

---

## `get_remaining_funding_capacity() → i128`

**Storage key:** `DataKey::Escrow`

Returns the remaining funding capacity before the funding target is reached.

- **Calculation**: `funding_target.saturating_sub(funded_amount)` clamped at `0` (via `.max(0)`) so it never goes negative when over-funded.
- **Informational only**: This view is for frontend guidance. The `fund` method may still accept deposits that over-fund past the target while the escrow status is `0` (Open).
- **No authorization**: Pure read; no auth or signature required.
- **Complexity**:
  - Time Complexity: $O(1)$ read from storage.
  - Space Complexity: $O(1)$ in-memory calculation.
- Panics with `"Escrow not initialized"` before `init`.

---

## `get_version() → u32`

**Storage key:** `DataKey::LegalHold`  
**Signature:** `pub fn get_legal_hold(env: Env) -> bool`

Returns `true` when a compliance hold is active; blocks `settle`, `withdraw`, `claim_investor_payout`, `fund`, and `sweep_terminal_dust`.

**Requires initialization:** No  
**Default when absent:** `false`

---

## `is_fully_funded() → bool`

**Derived from:** `DataKey::Escrow` (`funded_amount`, `funding_target`)

Returns `true` when `funded_amount >= funding_target`.

### Purpose

Exposes the contract's authoritative funding-completion predicate as a pure read view so
frontends no longer need to reimplement the funding logic client-side. Frontends and
indexers should call this view instead of reading `get_escrow()` and comparing fields
manually, because this view exactly mirrors the predicate used internally by the funding
transition logic and is therefore guaranteed to stay in sync with any future changes.

### Return value

| Condition | Returns |
|-----------|---------|
| `funded_amount < funding_target` | `false` |
| `funded_amount == funding_target` | `true` |
| `funded_amount > funding_target` | `true` |

### Exact predicate

```text
funded_amount >= funding_target
```

This is identical to the condition in `fund_impl` that transitions `status` from `0`
(open) to `1` (funded).

### Atomicity note

A `true` result before the funded status transition cannot occur because the transition
is atomic: `funded_amount` is updated and `status` is set to `1` in the same storage
write within `fund_impl`. Consequently `is_fully_funded() == true` implies `status == 1`.

### Authorization

None — pure read; no auth required, no state mutation, no side effects.

---

## `get_legal_hold() → bool`

**Storage key:** `DataKey::LegalHoldClearDelay`  
**Signature:** `pub fn get_legal_hold_clear_delay(env: Env) -> u64`

Returns the configured minimum delay (in seconds) between `request_clear_legal_hold` and `set_legal_hold(false)`.

**Requires initialization:** No  
**Default when absent:** `0` (no delay enforced; hold can be cleared immediately)

---

### `get_legal_hold_clearable_at() → Option<u64>`

**Storage key:** `DataKey::LegalHoldClearableAt`  
**Signature:** `pub fn get_legal_hold_clearable_at(env: Env) -> Option<u64>`

Returns the earliest ledger timestamp at which a pending legal-hold clear may be applied, or `None` when no clear request has been recorded.

**Requires initialization:** No  
**Default when absent:** `None`

**Return value:**
- `Some(timestamp)` after `request_clear_legal_hold` is called.
- `None` when no request is pending (or after a successful clear removes the key).

---

## Funding Constraints

### `get_funding_deadline() → Option<u64>`

**Storage key:** `DataKey::FundingDeadline`  
**Signature:** `pub fn get_funding_deadline(env: Env) -> Option<u64>`

Returns the optional funding deadline (ledger timestamp). After this timestamp passes, `fund` calls are rejected.

**Requires initialization:** No  
**Default when absent:** `None` (no deadline — funding is open indefinitely)

**Return value:**
- `Some(timestamp)` when configured at `init`.
- `None` when no deadline was set.

---

### `is_funding_expired() → bool`

**Signature:** `pub fn is_funding_expired(env: Env) -> bool`

Returns `true` when a funding deadline is set **and** `Env::ledger().timestamp() > deadline`.

**Requires initialization:** No  
**Default when absent:** `false` (no deadline set → never expired)

**Logic:**
```
if FundingDeadline exists:
    return ledger.timestamp() > deadline
else:
    return false
```

---

### `get_min_contribution_floor() → i128`

**Storage key:** `DataKey::MinContributionFloor`  
**Signature:** `pub fn get_min_contribution_floor(env: Env) -> i128`

Returns the minimum per-call funding amount in token base units. Applies to every `fund` / `fund_with_commitment` call.

**Requires initialization:** No (but written as `0` at `init`)  
**Default when absent:** `0` (no extra floor beyond "amount must be positive")

**Notes:**
- The floor applies to **each individual deposit**, not to cumulative principal.
- Written as `0` even when unconfigured at `init`, so reads always succeed post-init.

---

### `get_max_unique_investors_cap() → Option<u32>`

**Storage key:** `DataKey::MaxUniqueInvestorsCap`  
**Signature:** `pub fn get_max_unique_investors_cap(env: Env) -> Option<u32>`

Returns the optional cap on distinct investor addresses. Reflects the current stored cap, including any reduction via `lower_max_unique_investors`.

**Requires initialization:** No  
**Default when absent:** `None` (unlimited investors)

**Return value:**
- `Some(u32)` when configured.
- `None` when no cap was set at `init`.

---

### `get_remaining_investor_slots() -> Option<u32>`

**Signature:** `pub fn get_remaining_investor_slots(env: Env) -> Option<u32>`

Returns the number of remaining investor slots before the `MaxUniqueInvestorsCap` is reached. This safely resolves the gap between the cap and the `get_unique_funder_count`. 

**Requires initialization:** No  
**Default when absent:** `None` (unlimited investors)

**Return value:**
- `None` when no cap is configured (i.e., the escrow accepts unlimited distinct investors).
- `Some(u32)` indicating the exact remaining capacity of new distinct investors. Calculated as `cap - unique_funder_count`. Floored at zero (saturating subtraction) ensuring it stays completely consistent and safe even if the cap is reduced via `lower_max_unique_investors`.

---

### `get_max_per_investor_cap() → Option<i128>`

**Storage key:** `DataKey::MaxPerInvestorCap`  
**Signature:** `pub fn get_max_per_investor_cap(env: Env) -> Option<i128>`

Returns the optional immutable cap on cumulative principal for a single investor address.

**Requires initialization:** No  
**Default when absent:** `None` (unlimited per-investor)

**Return value:**
- `Some(i128)` when configured at `init`.
- `None` when unconfigured.

---

## Maturity & Settlement

### `has_maturity_lock() → bool`

**Derived from:** `DataKey::Escrow.maturity`  
**Signature:** `pub fn has_maturity_lock(env: Env) -> bool`

Returns `true` when `InvoiceEscrow::maturity > 0` and `settle()` is gated by ledger time.

**Requires initialization:** Yes — calls `get_escrow` internally.

**Logic:**
```
return get_escrow().maturity > 0
```

**Return value:**
- `true` — settlement requires `Env::ledger().timestamp() >= maturity`.
- `false` — `maturity == 0`; no time lock, funded escrow can settle immediately.

---

### `get_funding_close_snapshot() → Option<FundingCloseSnapshot>`

**Storage key:** `DataKey::FundingCloseSnapshot`  
**Signature:** `pub fn get_funding_close_snapshot(env: Env) -> Option<FundingCloseSnapshot>`

Returns the pro-rata denominator snapshot captured exactly once when the escrow first transitioned from open (0) to funded (1).

**Requires initialization:** No  
**Default when absent:** `None` (escrow has not yet reached funded status)

**Immutable once written:** the snapshot is never updated after the status-0-to-1 transition.

**Return value:**
- `None` until the escrow reaches `status == 1`.
- `Some(FundingCloseSnapshot)` with fields:
  - `total_principal: i128` — `funded_amount` at close (includes over-funding past target).
  - `funding_target: i128` — Snapshot of target at close time.
  - `closed_at_ledger_timestamp: u64` — Ledger timestamp of the funding transition.
  - `closed_at_ledger_sequence: u32` — Ledger sequence at transition.

Historical alias of [`get_effective_yield_bps`](#get_effective_yield_bpsinvestor-address--i64) —
same return value, documented around the per-investor storage slot.

---

## `get_effective_yield_bps(investor: Address) → i64`

**Storage key:** `DataKey::InvestorEffectiveYield(investor)`, falling back to `DataKey::Escrow.yield_bps`

Returns the **resolved effective yield (bps)** the investor would receive at settlement — exactly the
rate `compute_investor_payout` applies when computing the coupon. The resolution is identical to the
payout math:

```text
effective_yield_bps = InvestorEffectiveYield(investor)   // tier locked at first deposit
                      .unwrap_or(escrow.yield_bps)        // else the escrow base yield
```

| Investor state | Returns |
| --- | --- |
| Tiered (funded via `fund_with_commitment`) | the tier `yield_bps` selected at first deposit |
| Base-only / non-tiered | the escrow base `yield_bps` |
| Unknown (never funded) | the escrow base `yield_bps` |

### Stored vs resolved

`DataKey::InvestorEffectiveYield` is the **stored** per-investor slot: present only after a tiered
first deposit, absent otherwise. This view returns the **resolved** value — the stored slot when
present, otherwise the base-yield fallback — so integrators read the same number the payout math uses
without re-implementing the `unwrap_or` fallback themselves.

`get_investor_yield_bps` returns the same value; prefer `get_effective_yield_bps` when the intent is
"the rate `compute_investor_payout` will actually apply."

---

## Tier Lookup

### `preview_yield_tier(amount: i128, lock: u64) → YieldResolution`

**Signature:** `pub fn preview_yield_tier(env: Env, amount: i128, lock: u64) -> YieldResolution`

Pure read — no auth, no storage writes, safe for simulation.

Returns a [`YieldResolution`](#yieldresolution) for a hypothetical first deposit of `amount`
with `lock` seconds of commitment, using the **exact same tier-selection rule** applied by
`fund_with_commitment`. This lets a prospective investor see which tier they would receive before
depositing, without re-implementing the selection logic.

The `amount` parameter mirrors the `fund_with_commitment` signature. In the current release, tier
selection is lock-only; `amount` is accepted for API parity and forward-compatibility.

**Return fields (`YieldResolution`):**

| Condition | `effective_yield_bps` | `matched_lock_secs` |
|---|---|---|
| No `YieldTierTable` configured | escrow base `yield_bps` | `0` |
| `lock == 0` | escrow base `yield_bps` | `0` |
| `lock` below every tier threshold | escrow base `yield_bps` | `0` |
| `lock >= min_lock_secs` of a tier | highest qualifying tier's `yield_bps` | that tier's `min_lock_secs` |

> **Note:** this preview reflects the rule applied at **first deposit only**. A follow-on
> `fund` call does not re-select a tier.

**Security note:** the preview is guaranteed to agree with `fund_with_commitment` because it delegates
to the same internal `effective_yield_for_commitment` helper — there is no separate selection path.

---

### `YieldResolution`

Named return type for [`preview_yield_tier`](#preview_yield_tieramount-i128-lock-u64--yieldresolution)
and the internal `effective_yield_for_commitment` helper.

| Field | Type | Description |
|---|---|---|
| `effective_yield_bps` | `i64` | Resolved yield in basis points. Equals the escrow base yield when no tier matched, or the highest qualifying tier's `yield_bps` otherwise. |
| `matched_lock_secs` | `u64` | `min_lock_secs` of the matched tier, or `0` when base yield applies (no tier table, empty table, zero-lock, or no qualifying tier). |

---

## Per-Investor State

### `get_contribution(investor: Address) → i128`

**Storage key:** `DataKey::InvestorContribution(investor)` (persistent)  
**Signature:** `pub fn get_contribution(env: Env, investor: Address) -> i128`

Returns the current net principal contributed by `investor` in token base units. Successful
unfunds and refunds reduce this amount.

**Requires initialization:** No  
**Default when absent:** `0` (never contributed)  
**Storage type:** Persistent (independent TTL per address; see ADR-007)

---

### `get_total_deposited(investor: Address) → i128`

**Storage key:** `DataKey::InvestorTotalDeposited(investor)` (persistent)
**Signature:** `pub fn get_total_deposited(env: Env, investor: Address) -> i128`

Returns the gross sum of successful deposits made by `investor`. Unfunds, refunds, and
withdrawals do not reduce this value.

**Requires initialization:** No
**Default when absent:** current `get_contribution(investor)` value
**Storage type:** Persistent (independent TTL per address; see ADR-007)

For positions stored before this key was introduced, the current contribution is the
best available lower bound; earlier unfunds cannot be reconstructed on-chain.

---

### `get_total_deposited_with_provenance(investor: Address) → DepositTotalView`

**Storage keys:** `DataKey::InvestorTotalDeposited(investor)` (persistent), falling back to
`DataKey::InvestorContribution(investor)` (persistent)
**Signature:** `pub fn get_total_deposited_with_provenance(env: Env, investor: Address) -> DepositTotalView`

Companion view to [`get_total_deposited`](#get_total_depositedinvestor-address--i128) that returns the
same numeric total **plus** its provenance, so integrators can distinguish a fully recorded cumulative
total from a legacy contribution-only lower bound — even when the two numeric values are equal.

**Requires initialization:** No
**Storage type:** Persistent (independent TTL per address; see ADR-007)

**Return value (`DepositTotalView`):**

| Field | Type | Description |
|---|---|---|
| `total` | `i128` | The same numeric value `get_total_deposited(investor)` returns. |
| `provenance` | `DepositTotalProvenance` | Where `total` came from (see below). |

**Provenance semantics (`DepositTotalProvenance`):**

| Variant | Meaning |
|---|---|
| `Recorded` | `DataKey::InvestorTotalDeposited(investor)` is present; `total` is the complete recorded cumulative deposit sum. |
| `Fallback` | The cumulative key is absent but the investor has a non-zero contribution; `total` is the current `get_contribution(investor)` value, a legacy lower bound (earlier unfunds cannot be reconstructed on-chain). |
| `Unknown` | The investor has neither a cumulative key nor a contribution; `total` is `0` and the account has no recorded position. |

**Notes:**
- `get_total_deposited` is **unchanged**: it still returns the bare `i128` total with the same fallback
  behavior. This view is purely additive and compatible.
- `total` always equals `get_total_deposited(investor)` for the same address, so the two views can never
  disagree numerically; only the provenance differs.
- `Unknown` is reported explicitly rather than being conflated with a recorded zero, so consumers can
  tell "never funded" apart from "funded and fully unwound".
- **Pure read** — no authorization required, no state mutation, no TTL extension.

---

### `get_contributions(investors: Vec<Address>) → Vec<i128>`

**Storage key:** `DataKey::InvestorContribution(investor)` (persistent, one read per input)
**Signature:** `pub fn get_contributions(env: Env, investors: Vec<Address>) -> Vec<i128>`

Returns one contribution amount per supplied address, preserving input order. Unknown addresses
return `0`, matching `get_contribution`.

**Requires initialization:** No
**Default when absent:** `0` per address
**Batch bound:** `investors.len() <= MAX_INVESTOR_READ_BATCH` (50)
**Error:** `EscrowError::ContributionReadBatchTooLarge` when the input exceeds the bound
**Security note:** Pure read-only; performs no authorization, storage writes, or TTL extension.

---

### `get_unique_funder_count() → u32`

**Derived from:** active non-zero contributions in `DataKey::InvestorIndex`
**Signature:** `pub fn get_unique_funder_count(env: Env) -> u32`

Returns the count of listed investor addresses whose current contribution is non-zero. The
cached `DataKey::UniqueFunderCount` is not trusted by this view, so stale cache values do not
affect the result.

**Requires initialization:** No
**Default when the investor index is absent:** `0`

**Notes:** counts distinct chain accounts, not real-world persons (Sybil resistance is not a goal of this counter).

---

### `get_investor_yield_bps(investor: Address) → i64`

**Storage key:** `DataKey::InvestorEffectiveYield(investor)` (persistent)  
**Signature:** `pub fn get_investor_yield_bps(env: Env, investor: Address) -> i64`

Returns the effective annualized yield in basis points locked in at the investor's first deposit.

**Requires initialization:** Yes — reads `get_escrow()` for the base yield fallback.  
**Default when absent:** falls back to `InvoiceEscrow::yield_bps` (base yield for legacy / simple `fund` positions)  
**Storage type:** Persistent

**Return value:**
- Investor's tier-selected `yield_bps` when set via `fund_with_commitment`.
- Base `InvoiceEscrow::yield_bps` for simple `fund` deposits or pre-v2 positions.

---

## `get_distributed_principal() → i128`

**Storage key:** `DataKey::DistributedPrincipal`

Returns the total principal already returned to investors via [`StarfundEscrow::refund`].

- Used by [`StarfundEscrow::sweep_terminal_dust`] to compute outstanding liabilities.
- Absent ⇒ `0` (no refunds have occurred).

---

## `get_token_balance() → i128`

**Storage key:** None (reads [`DataKey::FundingToken`] and queries token contract)

Returns the contract's current funding-token balance for on-chain custody reconciliation.

- Emits [`EscrowError::FundingTokenNotSet`] if called before `init`.
- **Pure read** — no authorization required, no state mutation.

### Reconciliation relationship

Auditors can reconcile on-chain custody against recorded liabilities:

```
balance = get_token_balance()
funded_amount = get_escrow().funded_amount
distributed_principal = get_distributed_principal()

outstanding_liability = funded_amount - distributed_principal
excess_balance = balance - outstanding_liability  // tokens available for sweep

// After the cancelled escrow's liability is fully discharged (all refunds complete):
// balance == distributed_principal == funded_amount  (or less if partial sweep occurred)
```

This view surfaces the balance already consulted internally by [`StarfundEscrow::sweep_terminal_dust`]
and [`StarfundEscrow::withdraw`] for liability-floor enforcement.

---

## `get_reconciliation() → ReconciliationView`

**Storage keys:** reads `DataKey::Escrow`, `DataKey::DistributedPrincipal`, and
`DataKey::FundingToken` (then queries the token contract for the live balance).

Returns the contract's full reconciliation position in a single call, so operators
no longer have to fetch the balance, funded amount, distributed principal, and
settlement state separately and re-implement the liability arithmetic off-chain
(see the [Reconciliation relationship](#reconciliation-relationship) above).

```text
outstanding_liability = max(funded_amount - distributed_principal, 0)
surplus               = token_balance - outstanding_liability
```

`outstanding_liability` uses the **identical floor** that
[`StarfundEscrow::sweep_terminal_dust`] enforces, so the view and the sweep guard
can never disagree. `surplus` is the sweepable dust when positive and a deficit
when negative.

### `ReconciliationView` fields

| Field | Type | Description |
|-------|------|-------------|
| `token_balance` | `i128` | Live SEP-41 funding-token balance held by the contract. |
| `outstanding_liability` | `i128` | Principal still owed to investors: `max(funded_amount - distributed_principal, 0)`. |
| `surplus` | `i128` | `token_balance - outstanding_liability`. Positive = sweepable surplus; negative = deficit. |

- **Pure read** — no authorization required, no state mutation.
- **Never panics on values** — all arithmetic is saturating.
- Emits [`EscrowError::EscrowNotInitialized`] / [`EscrowError::FundingTokenNotSet`]
  only when the escrow has not been initialized.

**Security note:** in withdrawn (`3`) and cancelled (`4`) states, `outstanding_liability`
uses the same `DistributedPrincipal` floor as `sweep_terminal_dust`. In settled (`2`) state
the view remains informational; the dust sweep does not apply this principal floor.

---

### `is_investor_claimed(investor: Address) → bool`

**Storage key:** `DataKey::InvestorClaimed(investor)` (persistent)  
**Signature:** `pub fn is_investor_claimed(env: Env, investor: Address) -> bool`

Returns `true` when the investor has exercised `claim_investor_payout` after settlement.

**Requires initialization:** No  
**Default when absent:** `false`  
**Storage type:** Persistent

**Notes:** written once and never unset. A second `claim_investor_payout` call is a no-op (idempotent) rather than an error.

---

### `is_investor_refunded(investor: Address) → bool`

**Storage key:** `DataKey::InvestorRefunded(investor)` (persistent)
**Signature:** `pub fn is_investor_refunded(env: Env, investor: Address) -> bool`

Returns `true` when an investor's principal has been returned via `refund` in a cancelled (status 4) escrow.

**Requires initialization:** No  
**Default when absent:** `false`

**Notes:** written once; prevents double-refund. After `refund` succeeds, `get_contribution` for the same address returns `0`.

---

### `compute_investor_payout(investor: Address) → i128`

**Signature:** `pub fn compute_investor_payout(env: Env, investor: Address) → i128`

- `None` — Escrow is not yet funded; no close snapshot exists.
- `Some(FundingCloseSnapshot)` — The pro-rata denominator snapshot captured when the escrow first transitioned to **funded**.

---

### `get_settlement_pool() → i128`

**Storage keys:** `DataKey::FundingCloseSnapshot`, `DataKey::Escrow`  
**Signature:** `pub fn get_settlement_pool(env: Env) -> i128`

Returns the **total settlement pool** owed by the SME — the aggregate of every investor's
pro-rata payout, using each investor's own effective yield. Avoids rounding divergence that
arises when off-chain tooling re-derives the formula from raw snapshot fields.

**Storage keys:** `DataKey::FundingCloseSnapshot`, `DataKey::Escrow`, `DataKey::InvestorIndex`,
`DataKey::InvestorContribution`, `DataKey::InvestorEffectiveYield`

#### Formula (floor plus largest-remainder allocation)

```text
settle_pool_i = total_principal + total_principal × effective_yield_bps_i / 10_000  (floor)
payout_i      = contribution_i × settle_pool_i / total_principal   (floor)
if effective_yield_bps_i > 0 and payout_i == contribution_i:
    payout_i += 1

get_settlement_pool() = Σ_i payout_i
```

The extra base unit is the minimum coupon when positive yield would otherwise be rounded
away for an investor. The aggregate view includes these units.

Where `total_principal` is from `DataKey::FundingCloseSnapshot`, `contribution_i` from
`DataKey::InvestorContribution`, and `effective_yield_bps_i` is the investor's tiered yield
(`DataKey::InvestorEffectiveYield`) or the escrow base yield (`InvoiceEscrow::yield_bps`).

Fractional remainders are ranked by size; equal remainders are resolved by `InvestorIndex`
funding order. The settlement pool is the total rational liability rounded down once.

#### Yield note

The pool is **tier-weighted**, not base-yield. `settle()` reports this same aggregate liability
in `SettlementResult` / `EscrowSettled` and verifies that the escrow holds enough funding tokens.

#### Return values

| Condition | Returns |
|-----------|---------|
| `DataKey::FundingCloseSnapshot` absent (escrow not yet funded) | `0` |
| `total_principal <= 0` (degenerate snapshot) | `0` |
| No recorded investor (defensive fallback) | `total_principal + floor(total_principal × yield_bps / 10_000)` |
| Funded state with investors | `Σ_i payout_i` (tier-weighted total liability) |

#### Cost

O(n) over `DataKey::InvestorIndex`, hard-capped at `MAX_UNIQUE_INVESTORS` (10 000).

#### Overflow safety

All multiplications use `i128::checked_mul`; divisions and the running sum use
`i128::checked_div` / `i128::checked_add`. Emits
`EscrowError::ComputePayoutArithmeticOverflow` (code 129) on overflow.

#### Rounding invariant

`get_settlement_pool()` is the **exact** sum of all per-investor `compute_investor_payout`
values, with the aggregate fractional remainders assigned by largest remainder and funding-order
tie-break. `settle()` reports and checks this same liability.

#### Authorization

None — pure read; no auth required and no state mutation.

---

## Paginated Read Consistency

Several read views return **offset-paginated** slices of a mutable index:

| View | Backing collection | Mutated by |
|------|--------------------|------------|
| `get_investors(start, limit)` | `DataKey::InvestorIndex` | `fund` (append), `unfund` / `refund` (contribution → 0) |
| `get_allowlist_page(start, limit)` | `DataKey::Allowlist` | `add_to_allowlist` / `remove_from_allowlist` |
| `get_yield_tiers_page(start, limit)` | `DataKey::YieldTierTable` | never (immutable after `init`) |

**Offset pagination is not snapshot-isolated.** Each page is evaluated against the ledger
state at the moment that page's read executes. There is no snapshot token, no cursor, and no
version stamp tying successive pages together. If the underlying index mutates between two
page reads, the concatenated result can **skip** or **repeat** entries.

### Worked example: removal between pages

Assume `get_investors` with `limit = 2` over an index of five addresses:

```text
index (before): [A, B, C, D, E]
page 1: get_investors(0, 2) → [A, B]
```

Now `B` is removed from the index (its contribution drops to zero via `unfund`/`refund`, so
`get_investors` no longer lists it). The index compacts:

```text
index (after):  [A, C, D, E]
page 2: get_investors(2, 2) → [D, E]     // C is skipped: it shifted into offset 1
```

The naive concatenation `[A, B, D, E]` **silently omits `C`**. The mirror case — an insertion
before the current offset — causes an entry to be **returned twice**. The same hazard applies
to the allowlist: a `remove_from_allowlist` between pages shifts every later entry down by one,
and an `add_to_allowlist` shifts them up by one. A swap-style removal (remove one address,
add another in the same window) can produce both a skip and a duplicate in a single scan.

### Ledger-pinned reads

Where the host supports it, pin the whole scan to a single ledger:

- **Simulation / `simulateTransaction`:** pass the same `ledger_sequence` (or the same
  `LedgerInfo`) for every page so all pages observe one ledger state. Soroban simulation
  accepts an explicit ledger override; use it rather than the "latest" default.
- **Off-chain indexers:** read from a fixed ledger via the RPC `getLedgerEntries` /
  historical-ledger endpoints, or replay from a captured ledger close, so the index cannot
  move mid-scan.
- **On-chain callers:** there is no snapshot token in the contract API. A single transaction
  that reads all pages executes atomically against one ledger, but a multi-transaction scan
  cannot be pinned on-chain — use the reconciliation recipe below.

### Reconciliation / retry recipe

Because offset scans can be inconsistent, clients must be able to **detect** a bad scan and
**retry** it. Recommended procedure:

1. **Capture a boundary signal before the scan.** Record `get_unique_funder_count()` (for
   `get_investors`) or the allowlist length, plus the current ledger sequence.
2. **Page through** with `start += limit` until a page returns fewer than `limit` entries.
3. **Re-read the boundary signal after the scan.** If the count or ledger sequence changed,
   the scan spanned a mutation — **discard it and retry** from offset `0`.
4. **Cross-check the total.** The number of distinct addresses collected must equal the
   post-scan count. A mismatch (skip or duplicate) means the scan is inconsistent; retry.
5. **Deduplicate defensively.** If a retry is not possible, deduplicate by address and treat
   any shortfall against the boundary count as an error rather than a complete result.

A scan is only trustworthy when the boundary signal is unchanged across the whole scan, or
when every page was pinned to the same ledger. Never treat a single unpinned multi-page scan
as an authoritative snapshot.

---

## `get_yield_tiers() → Vec<YieldTier>`

**Storage key:** `DataKey::YieldTierTable`

Returns the yield-tier ladder configured at `init`, or an empty `Vec` when no tiers were configured (base yield applies to all investors).

- **Immutable** — set once at `init`; the contract never mutates this key after initialization.
- **Order** — returned order matches the validated non-decreasing ordering enforced at `init`: `min_lock_secs` strictly increasing, `yield_bps` non-decreasing.
- **Empty vec** — returned for both "no tiers passed at init" and "legacy instance predating tier support"; callers must not treat an empty result as an error.
- **Pure read** — no auth required, no state mutation.

## `get_yield_tiers_page(start, limit) → Vec<YieldTier>`

Returns a read-only paginated view of the configured yield-tier ladder using the same `start`/`limit` contract as the existing investor and allowlist read views.

- **Start beyond the end** — returns an empty page.
- **Limit zero** — returns an empty page.
- **Limit above the pagination ceiling** — clamps to the shared pagination ceiling (`MAX_INVESTOR_READ_BATCH`).
- **Ordering** — preserves the immutable tier-table order established at `init`.

### `YieldTier` fields

| Field | Type | Description |
|-------|------|-------------|
| `min_lock_secs` | `u64` | Minimum `committed_lock_secs` an investor must pass to qualify for this tier |
| `yield_bps` | `i64` | Effective annualized yield in basis points for qualifying investors |

## `get_rent_bump_plan() → RentBumpPlan`

**Signature:** `pub fn get_rent_bump_plan(env: Env) -> RentBumpPlan`

Returns a read-only plan describing which storage keys are **present** and how they relate to
the configured rent-bump threshold. It is a planning aid for operators; it does **not** measure
remaining TTL.

### `RentBumpPlan` fields

| Field | Type | Description |
|-------|------|-------------|
| `contribution_ttl` | `u32` | **Presence sentinel, not a measured TTL.** `1` when the contribution key is present, `0` when absent. It is **not** a count of remaining ledgers. |
| `threshold` | `u32` | The configured bump threshold used for the `Warning` policy below. |
| `status` | `RentBumpStatus` | `Ok` or `Warning`, derived from presence and threshold (see policy). |

> **⚠️ `contribution_ttl` is a sentinel.** Despite the name, it is only ever `0` or `1`. Do not
> interpret it as remaining ledgers, seconds, or any measured duration. Real TTL must be
> fetched from the RPC (`getLedgerEntries` returns `liveUntilLedgerSeq` per key) and compared
> against the current ledger sequence off-chain.

### Warning policy

The `status` field is computed from **presence only**, not from measured TTL:

| `threshold` | Key present | `contribution_ttl` | `status` |
|-------------|-------------|--------------------|----------|
| `0` | yes | `1` | `Ok` |
| `0` | no | `0` | `Ok` |
| nonzero | yes | `1` | `Warning` |
| nonzero | no | `0` | `Ok` |

With a nonzero threshold, **every present key is `Warning`** regardless of its actual remaining
TTL — the plan cannot distinguish a key that expires next ledger from one that is fresh. With
`threshold == 0`, `Warning` is never emitted. Over identical present keys, changing only the
threshold flips the status from `Ok` to `Warning`; this is expected and reflects presence, not
urgency.

### RPC TTL lookup

To obtain the real remaining TTL, query the RPC for the key's `liveUntilLedgerSeq` and subtract
the current ledger sequence:

```text
remaining_ledgers = liveUntilLedgerSeq - current_ledger_sequence
```

Use this value — not `contribution_ttl` — to decide whether a bump is actually needed.

### Planned API evolution

A future compatible release will add a clearly named presence field (e.g. `is_present: bool`)
so callers no longer have to read the sentinel semantics out of `contribution_ttl`. The existing
field will be retained for backward compatibility; new integrations should prefer the presence
field once available.

---

## `get_collateral_state() → CollateralState`

**Storage keys:** `DataKey::SmeCollateralPledge`, `DataKey::CollateralLimit`

Single O(1) read view of the current collateral state, so callers never have to reconstruct it from `get_sme_collateral_commitment()` + `get_collateral_limit()`.

- **Pure read** — no auth required, no state mutation; safe for simulation.
- **Never panics** — an unset commitment returns the default row below instead of trapping, and the view works before `init`.
- **No recomputation** — values are read straight from storage through `get_collateral_config()`, so this view can never drift from `get_collateral_config()` / `get_collateral_limit()` / `get_sme_collateral_commitment()`.
- **Record-only** — the same metadata caveat as `record_sme_collateral_commitment` applies: this is reported collateral, not proof of custody, lien, or token movement.

### `CollateralState` fields

| Field | Type | Description | Value when unset |
|-------|------|-------------|------------------|
| `is_set` | `bool` | Whether an SME collateral commitment is currently recorded | `false` |
| `asset` | `Symbol` | Reported asset symbol | empty `Symbol` |
| `amount` | `i128` | Reported collateral amount | `0` |
| `recorded_at` | `u64` | Ledger timestamp the commitment was written | `0` |
| `collateral_limit` | `i128` | Admin-configured ceiling on `record_sme_collateral_commitment` | `MAX_INVOICE_AMOUNT` |

Note that `collateral_limit` is independent of `is_set`: it reflects the stored limit (or the `MAX_INVOICE_AMOUNT` default) even when no commitment exists.

---

## Fee Governance (Informational Only)

> **⚠️ Security Warning:** The fee schedules documented in this section are **informational and governance-tracking only**. They do **NOT** affect actual disbursement calculations. This creates an important trust boundary for auditors.

### Actual Fees Applied

| Entrypoint | Fee source | Behavior |
|---|---|---|
| `withdraw()` | `DataKey::ProtocolFeeBps` (set at `init`, admin-updatable while `Open` via `set_protocol_fee_bps`) | Splits `funded_amount` according to the rate stored *at withdrawal time*; **ignores all fee schedules** |
| `release()` | `DataKey::ProtocolFeeBps` (set at `init`, admin-updatable while `Open` via `set_protocol_fee_bps`) | Splits each tranche according to the rate stored *at release time*; **ignores all fee schedules** |
| `claim_investor_payout()` | Per-investor yield only (no additional protocol fee) | No protocol fee deduction; uses base or tiered yield set at first deposit |

### Fee Schedule Storage (Governance Records)

The following endpoints return fee governance records stored in `FeeScheduleStorageKey`:

```rust
pub enum FeeScheduleStorageKey {
    Active,   // Currently active schedule (may be future-dated pending schedule)
    Pending,  // Pending schedule waiting for activation ledger
    Previous, // Schedule that was active before a recent activation
}
```

These storage keys are **separate from the actual fee system** and exist only for off-chain audit trails and governance transparency.

### `get_active_fee_schedule() → Option<FeeSchedule>`

**Storage key:** `FeeScheduleStorageKey::Active`  
**Signature:** `pub fn get_active_fee_schedule(env: Env) -> Option<FeeSchedule>`

Returns the active fee schedule for the current ledger, computing any not-yet-promoted boundary activation on the fly.

**Requires initialization:** No  
**Default when absent:** `None`

**Return value:**
- `Some(FeeSchedule)` with fields:
  - `fee_bps: u32` — Configured fee in basis points (0–10,000).
  - `min_fee_bps: u32` — Minimum enforced fee bound.
  - `max_fee_bps: u32` — Maximum enforced fee bound.
  - `activation_ledger: u32` — Ledger sequence number when this schedule became active.
- `None` when no schedule has been configured (see `submit_fee_schedule`).

**⚠️ Important:** This schedule **does NOT govern `withdraw()` or `release()` payouts**. See the [Actual Fees Applied](#actual-fees-applied) table above. The returned schedule is for governance and off-chain record-keeping only.

### `get_pending_fee_schedule() → Option<FeeSchedule>`

**Storage key:** `FeeScheduleStorageKey::Pending`  
**Signature:** `pub fn get_pending_fee_schedule(env: Env) -> Option<FeeSchedule>`

Returns the pending fee schedule that will activate at a future ledger, or `None` if no pending schedule exists.

**Requires initialization:** No  
**Default when absent:** `None`

**Return value:**
- `Some(FeeSchedule)` with the fields listed above.
- `None` when no future schedule has been staged.

**Activation condition:** This schedule becomes active (and moves to `Active` storage) once `activation_ledger <= env.ledger().sequence()`. The transition is automatic; no explicit activation call is required.

**⚠️ Important:** Even when `Some(...)`, this pending schedule has **zero effect on current disbursements** because actual fees come only from `DataKey::ProtocolFeeBps` (set at `init`, admin-updatable while `Open`). This view is for governance transparency only.

### `get_previous_fee_schedule() → Option<FeeSchedule>`

**Storage key:** `FeeScheduleStorageKey::Previous`  
**Signature:** `pub fn get_previous_fee_schedule(env: Env) -> Option<FeeSchedule>`

Returns the previously active fee schedule after a boundary activation, or `None` if no activation has occurred or no prior schedule exists.

**Requires initialization:** No  
**Default when absent:** `None`

**Lifecycle:**
1. `submit_fee_schedule(s1)` → `Active = s1`, `Previous = None`
2. `submit_fee_schedule(s2)` where `s2.activation_ledger` is future → `Active = s1`, `Pending = s2`, `Previous = None`
3. Ledger advances past `s2.activation_ledger` → `Active = s2`, `Pending = None`, `Previous = s1`

**⚠️ Important:** This historical record is for audit trails and governance tracking. Like `get_active_fee_schedule`, it has **zero effect on actual disbursement calculations**.

### Trust Boundary

Integrators **must not** assume that `get_active_fee_schedule()` returns the fees being applied to investor payouts. Instead:

1. **For auditing:** Query `get_active_fee_schedule`, `get_pending_fee_schedule`, `get_previous_fee_schedule` to track governance decisions and off-chain accounting records.
2. **For payout calculations:** Use `get_escrow_summary() → protocol_fee_bps` (equivalently `get_protocol_fee_bps()`), which reflects the current `DataKey::ProtocolFeeBps` — the value set at `init` and, while the escrow is `Open`, replaceable by the admin via `set_protocol_fee_bps` — used by all disbursals.
3. **For per-investor payouts:** Use `compute_investor_payout(investor)`, which applies the investor's locked-in yield and the current `DataKey::ProtocolFeeBps`, **not** any dynamic schedule.

The separation prevents accidental payout miscalculations if an integration layer assumes dynamic fee governance when the on-chain system uses the single `DataKey::ProtocolFeeBps` value (init-set, optionally admin-updated pre-funding) instead.
