# Fees State Machine Documentation

This document describes the state machine, allowed state transitions, entrypoint enforcement, and invariants governing protocol fees in `StarfundEscrow`.

---

## Overview & Scope

"Fees" in this contract refers exclusively to the `protocol_fee_bps` split applied to the SME principal during `StarfundEscrow::withdraw`. The rate is set at contract initialization (`init`) and may be changed afterward by the admin via `set_protocol_fee_bps`, but only while the escrow is still in the `Open` status (`status == 0`); once the escrow advances past `Open`, the rate is frozen for the rest of that instance's life.

---

## State Diagram

```mermaid
stateDiagram-v2
    [*] --> Unconfigured : Contract Deployment

    Unconfigured --> Configured : init(protocol_fee_bps)

    note right of Configured
        Stored under DataKey::ProtocolFeeBps.
        Mutable via set_protocol_fee_bps while
        escrow.status == 0 (Open); frozen once
        the escrow leaves Open.
    end note

    state Configured {
        [*] --> Idle : Escrow Active / Awaiting SME Withdrawal

        Idle --> Idle : set_protocol_fee_bps() [status == Open]
        Idle --> FeeCalculation : withdraw()
        FeeCalculation --> DisburseTreasury : protocol_fee_bps > 0
        FeeCalculation --> DirectDisburse : protocol_fee_bps == 0
        
        DisburseTreasury --> WithdrawalComplete : Route fee to Treasury & net payout to SME
        DirectDisburse --> WithdrawalComplete : Transfer full principal to SME
        
        WithdrawalComplete --> Idle : Escrow Active / Closed
    }
| Current State | Description | Trigger / Entrypoint | Next Allowed State |
| :--- | :--- | :--- | :--- |
| **`Unconfigured`** | Contract is deployed but uninitialized. `DataKey::ProtocolFeeBps` is unset in storage. | Contract Deployment | `Configured` |
| **`Configured`** | `protocol_fee_bps` (0 <= bps <= 10,000) is stored under `DataKey::ProtocolFeeBps`. | `StarfundEscrow::init` | `Idle`; re-enters `Configured` via `set_protocol_fee_bps` while `status == 0` (Open) |
| **`Idle`** | Contract initialized and operational; waiting for SME withdrawal. | `init` complete | `FeeCalculation` |
| **`FeeCalculation`** | Calculates fee split and SME payout floor division. | `StarfundEscrow::withdraw` | `DisburseTreasury` or `DirectDisburse` |
| **`WithdrawalComplete`** | Transfers executed according to the computed fee split. | Settlement completion | `Idle` |

---

## Entrypoints & State Enforcement

### 1. `StarfundEscrow::init`
* **Auth Guard:** `admin.require_auth()`
* **Parameter:** `protocol_fee_bps: Option<i64>` (defaults `None` to `0`)
* **Validation:** Enforces 0 <= protocol_fee_bps <= 10,000.
* **Rejection:** Values outside this range revert with `EscrowError::ProtocolFeeBpsOutOfRange` (**215**).
* **Storage Write:** Saves `protocol_fee_bps` under `DataKey::ProtocolFeeBps`.

### 2. `StarfundEscrow::set_protocol_fee_bps`
* **Auth Guard:** `admin.require_auth()` (via `load_escrow_require_admin`)
* **Lifecycle Guard:** `escrow.status == 0` (Open); otherwise reverts (the error named `ProtocolFeeUpdateNotOpen` at `lib.rs:4755`).
* **Replay Guard:** consumes `expected_nonce` via `consume_admin_nonce`; mismatch reverts with `EscrowError::AdminNonceMismatch` (**257**).
* **Validation:** Enforces `0 <= new_fee_bps <= 10,000`; out-of-range reverts with `EscrowError::ProtocolFeeBpsOutOfRange` (**215**).
* **State Operations:** No-op (no storage write, no event) if `new_fee_bps == old_fee_bps`; otherwise overwrites `DataKey::ProtocolFeeBps` and emits `ProtocolFeeUpdated { old_fee_bps, new_fee_bps }`.

### 3. `StarfundEscrow::withdraw`
* **Auth Guard:** `sme_address.require_auth()`
* **State Operations:**
  1. Reads `protocol_fee_bps` from `DataKey::ProtocolFeeBps` — whatever value is currently stored, reflecting any `set_protocol_fee_bps` update made before this call.
  2. Computes the protocol fee and SME net payout:
     * `fee = floor(funded_amount * protocol_fee_bps / 10_000)`
     * `sme_payout = funded_amount - fee`
  3. Transfers `fee` to `DataKey::Treasury` and `sme_payout` to `InvoiceEscrow::sme_address`.
* **Conservation Invariant:** Strictly guarantees `sme_payout + fee == funded_amount`.
