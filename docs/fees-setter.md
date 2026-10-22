# `set_protocol_fee_bps` — Admin Fee Setter

This document describes the `set_protocol_fee_bps` entrypoint, which allows
an admin to update the protocol fee rate after initialization.

---

## Motivation

`init` writes `protocol_fee_bps` once. `set_protocol_fee_bps` provides a
post-init setter so operators can adjust the fee rate without redeploying the
contract. The valid range is enforced directly in the entrypoint against the
hard-coded `0..=10_000` envelope — there is no separate operator ceiling.

---

## Entrypoints

### `set_protocol_fee_bps(fee_bps: i64) → i64`

| Field | Value |
|-------|-------|
| **Auth** | Admin (`admin.require_auth()`) |
| **Parameter** | `fee_bps`: new protocol fee rate in basis points |
| **Validation** | Must satisfy `0 ≤ fee_bps ≤ 10_000` |
| **Rejection** | Outside range → `EscrowError::ProtocolFeeBpsOutOfRange` (215) |
| **Effect** | Overwrites `DataKey::ProtocolFeeBps`; emits `ProtocolFeeUpdated`. No-op if the new value equals the current value (no event emitted). |
| **Returns** | The stored fee value after the call. |

### `get_protocol_fee_bps() → i64`

Read-only. Returns the stored protocol fee rate, or `0` for legacy/unset
instances.

---

## Guard ordering for `set_protocol_fee_bps`

1. Escrow must be initialized — `get_admin` present  
2. `admin.require_auth()` — Soroban host auth check  
3. `ProtocolFeeBpsOutOfRange` (215) — `fee_bps < 0` or `fee_bps > 10_000`  
4. Early return (no-op) if `fee_bps == current_fee_bps`  
5. Storage write: `DataKey::ProtocolFeeBps ← fee_bps`  
6. Event emission: `ProtocolFeeUpdated { old_fee_bps, new_fee_bps }`  

---

## Validation envelope

The fee is validated directly against the hard-coded `0..=10_000` basis-point
range on every call. There is **no** separate operator ceiling (`set_fees_limit`
/ `get_fees_limit` do not exist in this contract). The admin may freely set
any value within `[0, 10_000]`.

---

## Event: `ProtocolFeeUpdated`

Emitted by `set_protocol_fee_bps` when the value changes.

| Field | Type | Meaning |
|-------|------|---------|
| `name` (topic) | `Symbol` | `"fee_upd"` |
| `invoice_id` (topic) | `Symbol` | Escrow invoice identifier |
| `old_fee_bps` | `i64` | Fee rate before this update |
| `new_fee_bps` | `i64` | Fee rate after this update |

---

## Conservation invariant

The fee split computed at `withdraw` time uses the **stored**
`DataKey::ProtocolFeeBps` value at that moment, not the value at `init`. The
conservation invariant `fee + sme_net == funded_amount` continues to hold
regardless of how many times the fee rate was updated between `init` and
`withdraw`.

---

## Cross-references

- [`docs/fees-auth.md`](fees-auth.md) — authorization rules
- [`docs/fees-errors.md`](fees-errors.md) — error codes including `ProtocolFeeBpsOutOfRange` (215)
- [`docs/fees-states.md`](fees-states.md) — state machine overview
- [`docs/escrow-fee-split-conservation.md`](escrow-fee-split-conservation.md) — conservation invariant proofs
