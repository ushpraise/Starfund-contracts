# Escrow Contract Events

This document provides a reference for indexers and block explorers to consume events emitted by the Starfund Escrow contract.

## 📡 Event Structure

All events follow the Soroban `contractevent` format. Key fields like `invoice_id` and `investor` are marked as **topics** to enable efficient filtering by indexers.

### Common Topics

The RPC event envelope carries the emitting contract's address as **metadata separate from the
topic array** (field `contractId`, not a topic index) — filter on it directly, the way
[`escrow-indexer.md`](escrow-indexer.md#rpc-event-filter-baseline) does (`contractId` = escrow
contract id). The `topics` array itself is numbered from the event symbol at index **0**:

- **`topic[0]`**: Event Name (Symbol, e.g., `funded`, `escrow_sd`) — always present, always first.
- **`topic[1]`**: `invoice_id` (Symbol) — present on most events (see exceptions below).
- **`topic[2]`**: `investor`/`origin` (Address) — present only on funding, claim, allowlist-change,
  and callback events.

Below, each event's **"Topics:"** list is a 1-based ordinal list for readability — list item *N*
corresponds to `topic[N-1]` in the 0-indexed RPC array. So item 1 (the event name) is always
`topic[0]`, item 2 (when present) is `topic[1]`, and so on.

**Exceptions — no `invoice_id` topic:** [`EscrowInitialized`](#escrowinitialized) and
[`InvestorAllowlistBatchApplied`](#investorallowlistbatchapplied) have only one topic (the event
name). Neither carries `invoice_id` as a topic — `EscrowInitialized` carries it nested inside the
`escrow` field of its data payload, and `InvestorAllowlistBatchApplied` carries it as a top-level
`invoice_id` field in its data payload. An indexer that filters strictly by `topic[1] ==
invoice_id` will miss both; it must instead read `invoice_id` out of the data payload for these
two event types.

---

## 📋 Event Catalog

### `FeeScheduleSubmitted`
Emitted when the admin successfully stages a pending fee schedule. Re-submitting the identical
pending schedule is idempotent and emits no additional event.

**Topics:**
1. `fee_sub` (Symbol)
2. `invoice_id` (Symbol)

**Data Payload:**
- `schedule` (`FeeSchedule`)

### `FeeScheduleActivated`
Emitted when a pending fee schedule is promoted at or after its activation ledger.

**Topics:**
1. `fee_act` (Symbol)
2. `invoice_id` (Symbol)

**Data Payload:**
- `schedule` (`FeeSchedule`) — newly active schedule.
- `previous` (`Option<FeeSchedule>`) — schedule active before promotion, if any.

**Emission semantics:** `FeeScheduleActivated` is emitted **only** by the mutating
`activate_fee_schedule` entrypoint. It is never emitted by a getter, and no event is
promised from any read-only call. See [Projected vs. Persisted Fee Schedule State](#-projected-vs-persisted-fee-schedule-state).

### `EscrowInitialized`
Emitted once by `init()`. Carries the escrow snapshot plus immutable bound references so
indexers can register `funding_token`, `treasury`, and optional `registry` without follow-up reads.

**Topics:**
1. `escrow_ii` (Symbol)

**Data Payload:**
- `escrow` (`InvoiceEscrow`)
- `funding_token` (`Address`) — equals `DataKey::FundingToken`
- `treasury` (`Address`) — equals `DataKey::Treasury`
- `registry` (`Option<Address>`) — equals `DataKey::RegistryRef`
- `has_maturity_lock` (`bool`) — false when `maturity == 0`, meaning settlement has no maturity time lock

**Example (JSON Decoded):**
```json
{
  "topics": ["escrow_ii"],
  "data": {
    "escrow": { "invoice_id": "INV_001", "status": 0 },
    "funding_token": "CTOKEN...",
    "treasury": "GTREAS...",
    "registry": "GREG...",
    "has_maturity_lock": true
  }
}
```

### `MaxUniqueInvestorsCapLowered`
Emitted when admin calls `lower_max_unique_investors` while the escrow is open.

**Topics:**
1. `inv_cap` (Symbol)
2. `invoice_id` (Symbol)

**Data Payload:**
- `old_cap` (u32)
- `new_cap` (u32)

### `MinContributionFloorRaised` and `MinContributionFloorLowered`
Emitted when an admin raises or lowers the per-deposit minimum contribution floor while the
escrow is open.

**Topics:**
1. `floor_hi` or `floor_lo` (Symbol)
2. `invoice_id` (Symbol)

**Data Payload:**
- `old_floor` (i128)
- `new_floor` (i128)

### `MaxPerInvestorCapRaised` and `MaxPerInvestorCapLowered`
Emitted when an admin raises or lowers the cumulative per-investor contribution cap while the
escrow is open.

**Topics:**
1. `inv_cap` (Symbol)
2. `invoice_id` (Symbol)

**Data Payload:**
- `old_cap` (i128)
- `new_cap` (i128)

### `MaturityMaxHorizonRaised` and `MaturityMaxHorizonLowered`
Emitted when an admin adjusts the maximum maturity horizon. Lowering is rejected if the
proposed horizon would invalidate the escrow's current maturity.

**Topics:**
1. `mtry_rse` or `mtry_lwr` (Symbol)
2. `invoice_id` (Symbol)

**Data Payload:**
- `old_horizon` (u64)
- `new_horizon` (u64)

### `EscrowFunded`
Emitted when an investor deposits principal.

**Topics:**
1. `funded` (Symbol)
2. `invoice_id` (Symbol)
3. `investor` (Address)

**Data Payload:**
- `amount` (i128)
- `funded_amount` (i128)
- `status` (u32)
- `investor_effective_yield_bps` (i64)

**Example (JSON Decoded):**
```json
{
  "topics": ["funded", "INV_001", "G...INVESTOR"],
  "data": {
    "amount": "1000000000",
    "funded_amount": "5000000000",
    "status": 0,
    "investor_effective_yield_bps": 500
  }
}
```

### `EscrowSettled`
Emitted when the SME finalizes the escrow after maturity.

**Topics:**
1. `escrow_sd` (Symbol)
2. `invoice_id` (Symbol)

**Data Payload:**
- `funded_amount` (i128)
- `yield_bps` (i64)
- `maturity` (u64)
- `settled_at_ledger_timestamp` (u64) — the ledger timestamp when `settle` was called
- `settle_pool` (i128) — nominal base-yield settlement pool: `total_principal + floor(total_principal × yield_bps / 10_000)`. The exact aggregate claim liability, including minimum-coupon units, is returned by `get_settlement_pool()`.

**Example (JSON Decoded):**
```json
{
  "topics": ["escrow_sd", "INV_001"],
  "data": {
    "funded_amount": "10000000000",
    "yield_bps": 500,
    "maturity": 1714184400,
    "settled_at_ledger_timestamp": 1714184400,
    "settle_pool": "10500000000"
  }
}
```

### `EscrowPartialSettle`
Emitted when the SME or admin closes funding early and the escrow transitions from open to funded.

**Topics:**
1. `part_set` (Symbol)
2. `invoice_id` (Symbol)

**Data Payload:**
- `funded_amount` (`i128`) — principal recorded when funding was closed.
- `partial_settled_at_ledger_timestamp` (`u64`) — ledger timestamp written to
  `DataKey::PartialSettledAt` and returned by `get_partial_settled_at`.

### `InvestorPayoutClaimed`
Emitted when an investor records their payout claim.

**Topics:**
1. `inv_claim` (Symbol)
2. `invoice_id` (Symbol)
3. `investor` (Address)

**Data:**
1. `payout` (i128) — gross payout transferred to the investor
2. `effective_yield_bps` (i64) — investor-specific annualized yield used to calculate the payout
3. `commitment_lock_secs` (u64) — original lock duration committed on the investor's first deposit

**Example (JSON Decoded):**
```json
{
  "topics": ["inv_claim", "INV_001", "G...INVESTOR"],
  "data": {
    "payout": "80000000",
    "effective_yield_bps": 900,
    "commitment_lock_secs": 100
  }
}
```

### `FundingDeadlineExtended`
Emitted when the admin extends an existing funding deadline while the escrow is still open.

**Topics:**
1. `fund_ext` (Symbol)
2. `invoice_id` (Symbol)

**Data Payload:**
- `old_deadline` (`u64`): prior funding deadline ledger timestamp.
- `new_deadline` (`u64`): strictly later funding deadline ledger timestamp.

**Notes:**
- This event is additive; existing funding and settlement events are unchanged.
- The entrypoint rejects equal or earlier deadlines, elapsed funding windows, closed escrows, and deadlines at or after maturity.

**Example (JSON Decoded):**
```json
{
  "topics": ["fund_ext", "INV_001"],
  "data": {
    "old_deadline": 1714180000,
    "new_deadline": 1714183600
  }
}
```

### `FundingDeadlineUpdated`
Emitted by `update_funding_deadline` whenever the admin sets or clears the
funding deadline while the escrow is open.

**Topics:**
1. `fund_dl` (Symbol)
2. `invoice_id` (Symbol)

**Data Payload:**
- `prior_deadline` (`Option<u64>`): the previously configured ledger timestamp,
  or `None` when no deadline was configured.
- `new_deadline` (`Option<u64>`): the new ledger timestamp, or `None` when the
  deadline was cleared.

**Emission semantics:**
- `Some(d)` stores `d` after verifying it is later than the current ledger
  timestamp.
- `None` removes the stored deadline and allows funding without a time limit.
- The event is emitted after the storage update and includes both the previous
  and resulting values, so indexers can represent set, replace, and clear
  operations without an additional read.

**Example (JSON Decoded):**
```json
{
  "topics": ["fund_dl", "INV_001"],
  "data": {
    "prior_deadline": 1714180000,
    "new_deadline": 1714183600
  }
}
```

### `InvestorAllowlistChanged`
Emitted when an admin adds or removes an investor from the allowlist. This event is
emitted per-address even when the change is performed via the batch entrypoint
`set_investors_allowlisted`, so indexers receive one `InvestorAllowlistChanged` event
for each address in the batch.

**Topics:**
1. `al_set` (Symbol)
2. `invoice_id` (Symbol)
3. `investor` (Address)

**Data Payload:**
- `allowed` (u32): `1` for allowed, `0` for blocked.

**Notes:**
- Batch mutations via `set_investors_allowlisted` emit one `al_set` event per affected
  investor to preserve parity with individual `set_investor_allowlisted` calls.
- Existing indexers that only consume `al_set` do not need to change — the per-investor
  event sequence is identical regardless of whether it originated from a single call or
  a batch call.

### `InvestorAllowlistBatchApplied`
Emitted **once per `set_investors_allowlisted` call**, after all per-investor
`InvestorAllowlistChanged` events have been emitted in the same transaction.

**Topics:**
1. `al_batch` (Symbol)

**Data Payload:**
- `invoice_id` (Symbol): the escrow invoice identifier.
- `batch_size` (u32): total number of investors processed in this batch.
- `allowed` (u32): `1` if the batch allowed investors, `0` if it blocked them.

**Why `al_batch` exists:**
The per-investor `al_set` events emitted by `set_investors_allowlisted` are
structurally identical to those emitted by the single-address entrypoint
`set_investor_allowlisted`. Without an additional marker, an indexer cannot
determine whether a run of `al_set` events in a transaction came from one
batch call or many individual calls. `al_batch` provides that disambiguation
in a single event.

**Relationship to `al_set`:**
`al_batch` supplements — it does not replace — the per-investor `al_set` events.
Both are emitted for every `set_investors_allowlisted` call:
- N × `al_set` (one per address, in input order)
- 1 × `al_batch` (after the loop, carrying the full count)

**Backward compatibility:**
Existing indexers that only subscribe to `al_set` remain fully compatible. The
`al_batch` event is purely additive and is never emitted by the single-address
`set_investor_allowlisted` entrypoint.

**Auditor usage:**
Auditors can cross-check `batch_size` against the number of `al_set` events in
the same transaction to verify that no per-investor events were dropped or
duplicated during a batch operation.

### `RegistryRefRebound`
Emitted when the admin calls `rebind_registry_ref` (including via the `clear_registry_ref`
convenience wrapper). Signals that the off-chain registry hint stored at `DataKey::RegistryRef`
has changed. **This event carries no settlement authority** — it exists purely so off-chain
indexers can re-sync their local pointer without polling the contract.

**Topics:**
1. `reg_rebd` (Symbol)
2. `invoice_id` (Symbol)

**Data Payload:**
- `registry` (`Option<Address>`): new hint value. `None` means the pointer was cleared (unbound state).

**Non-authority guarantee:**
The emitted address is a discoverability hint only. No on-chain logic in the escrow contract
reads `DataKey::RegistryRef` when moving funds, settling, or authorizing any call. Presence of
a `Some(addr)` value does **not** imply registry membership — query the registry contract directly
to verify on-chain state.

**Integrator guidance:**
- `None` (unbound): no off-chain registry is currently associated with this escrow. Treat as "not registered" for UI/UX purposes.
- `Some(addr)` (bound): an off-chain indexer hint is set; verify membership with the registry at `addr` if authoritative state is required.
- On receiving this event, re-sync any cached pointer and do **not** infer fund-flow changes.

**Example (JSON Decoded):**
```json
{
  "topics": ["reg_rebd", "INV_001"],
  "data": {
    "registry": "CREG..."
  }
}
```

**Related entrypoints:** `rebind_registry_ref`, `clear_registry_ref`, `get_registry_ref`.
See also: [`docs/escrow-registry-ref.md`](escrow-registry-ref.md).

---

### `LegalHoldChanged`
Emitted when an admin toggles the compliance hold.

**Topics:**
1. `legalhld` (Symbol)
2. `invoice_id` (Symbol)

**Data Payload:**
- `active` (u32): `1` for enabled, `0` for cleared.

### `PausedChanged`
Emitted when an admin toggles the lightweight operational pause via `set_paused`.
Orthogonal to `LegalHoldChanged` — it signals the incident-response circuit
breaker (no compliance semantics, no clear delay), not the compliance hold.

**Topics:**
1. `paused` (Symbol)
2. `invoice_id` (Symbol)

**Data Payload:**
- `active` (u32): `1` for enabled, `0` for cleared.

### `CollateralClearedEvt`
Emitted when the SME clears the metadata-only collateral commitment recorded under
`DataKey::SmeCollateralPledge`.

**Topics:**
1. `coll_clr` (Symbol)
2. `invoice_id` (Symbol)

**Data Payload:**
- `asset` (Symbol): SME-reported off-chain asset label from the stored commitment.
- `amount` (i128): SME-reported amount from the stored commitment.
- `recorded_at` (u64): ledger timestamp from the original commitment record.

**Indexer guidance:**
Use `coll_clr` to remove or mark retired the active collateral commitment for the
invoice. This event is metadata-only and does not prove custody, asset movement,
or enforceable collateral.

### `CallbackRegisteredEvent`
Emitted by `register_callback` when a cross-contract callback is registered with an authorized origin contract and expected phase.

**Topics:**
1. `cb_reg` (Symbol)
2. `invoice_id` (Symbol)
3. `origin` (Address)

**Data Payload:**
- `nonce` (u64): invocation nonce assigned to the callback.
- `phase` (u32): expected lifecycle phase for the callback.

### `CallbackExecutedEvent`
Emitted by `execute_callback` when a registered cross-contract callback is successfully verified and consumed.

**Topics:**
1. `cb_exec` (Symbol)
2. `invoice_id` (Symbol)
3. `origin` (Address)

**Data Payload:**
- `nonce` (u64): invocation nonce consumed by the callback.
- `phase` (u32): lifecycle phase executed by the callback.

### `CallbackCancelledEvent`
Emitted by `cancel_callback` when an admin retires an obsolete pending callback registration.
The pending index entry for the callback is removed; other registered callbacks are left unchanged.

**Topics:**
1. `cb_cncl` (Symbol)
2. `invoice_id` (Symbol)
3. `origin` (Address)

**Data Payload:**
- `nonce` (u64): invocation nonce of the cancelled callback.
- `phase` (u32): lifecycle phase the cancelled callback was registered for.

**Notes:**
- Only the admin may cancel a callback; unauthorized callers are rejected.
- After cancellation the callback can no longer execute: a later `execute_callback`
  for the same `origin`/`phase`/`nonce` fails with the contract's callback-not-found error.
- Cancellation is per-callback; other pending registrations remain active and executable.

---

## 🔮 Projected vs. Persisted Fee Schedule State

Fee schedule reads and fee schedule activation are **two separate concerns**:

- **Projected (read-only) view.** Getters such as `get_active_fee_schedule` and
  `get_pending_fee_schedule` compute a *projection* from stored state plus the current
  ledger. `get_active_fee_schedule` returns the pending schedule once its activation
  ledger has been reached, even if no transaction has promoted it yet. This projection is
  a convenience for callers; it does **not** mutate storage and does **not** emit events.
- **Persisted (mutating) state.** The stored active schedule only changes when
  `activate_fee_schedule` is submitted. That call promotes the pending schedule, writes
  the new active value, and emits `FeeScheduleActivated`. `activate_fee_schedule` is
  permissionless: anyone may submit it once the activation ledger is reached.

Because the projection can advance while the persisted state (and therefore the event
stream) has not, an **event-only consumer can lag the projected active view**. A schedule
whose activation ledger has passed is *due* but not yet *activated* until a transaction
calls `activate_fee_schedule`.

### Timeline: due schedule with no activation transaction

| Step | Ledger / action | Persisted active schedule | `get_active_fee_schedule` projection | Events emitted |
|------|-----------------|---------------------------|--------------------------------------|----------------|
| 1 | Admin calls `submit_fee_schedule` with `activation_ledger = L` | unchanged (old schedule) | old schedule | `FeeScheduleSubmitted` |
| 2 | Ledger advances to `L` (no transaction) | unchanged (old schedule) | **new schedule** (due) | none |
| 3 | Indexer calls `get_active_fee_schedule` | unchanged (old schedule) | **new schedule** (due) | none — getters emit nothing |
| 4 | Anyone calls `activate_fee_schedule` at or after `L` | **new schedule** | new schedule | `FeeScheduleActivated` |

Between steps 2 and 4 the projection and the persisted state disagree. No event marks
the transition at step 2, because nothing was written on-chain.

### Indexer guidance

- Do **not** assume a `FeeScheduleActivated` event will arrive merely because the
  activation ledger has passed. The event only exists once `activate_fee_schedule` runs.
- When a `FeeScheduleSubmitted` event carries an `activation_ledger`, schedule a poll of
  `get_active_fee_schedule` at (or after) that ledger to learn the projected schedule.
- To make the projected schedule authoritative and observable, submit the permissionless
  `activate_fee_schedule` call once the activation ledger is reached. This persists the
  change and emits `FeeScheduleActivated`, which other event-only consumers can rely on.
- Treat `get_active_fee_schedule` / `get_pending_fee_schedule` as read-only projections:
  they never emit events, and their results may differ from the last persisted activation
  until `activate_fee_schedule` is called.

---

## 🛠️ Indexing Recommendations

### Filtering by Invoice
To track all activity for a specific invoice, indexers should filter for events where
`topic[1]` matches the `invoice_id`. This covers every event **except**
[`EscrowInitialized`](#escrowinitialized) and
[`InvestorAllowlistBatchApplied`](#investorallowlistbatchapplied) — those two have no
`invoice_id` topic at all; for them, read `invoice_id` out of the event's data payload instead
(nested in the `escrow` field for `EscrowInitialized`, top-level for
`InvestorAllowlistBatchApplied`). See [Common Topics](#common-topics) above.

### Filtering by Investor
To track an investor's portfolio, filter for events where `topic[2]` matches the investor's
`Address`. This applies to `EscrowFunded`, `InvestorPayoutClaimed`, and
`InvestorAllowlistChanged`; the same `topic[2]` position also carries `origin` (an `Address`,
not an investor) on the callback events (`CallbackRegisteredEvent`,
`CallbackExecutedEvent`, `CallbackCancelledEvent`) — check the event name (`topic[0]`) before
assuming `topic[2]` is an investor address.

### Decoding payloads
Payloads are XDR-encoded. Use the `starfund_escrow` WASM/interface or the `Stellar SDK` to decode the `data` field into the corresponding Rust structs.
