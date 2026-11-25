# Fees: authorization and access rules

Documents **who** may configure or realize the protocol fee, **when**, and **what happens on
rejection** — for auditors, integrators, and reviewers. Verified directly against
`escrow/src/lib.rs` (line references below); this is not a design proposal.

> **Scope:** "fees" in this contract means one thing applied two ways — the
> `protocol_fee_bps` split applied to the SME's principal at
> [`StarfundEscrow::withdraw`]. The rate is set at `init` but is **not immutable**:
> the admin can change it later, while the escrow is still `Open`, via
> [`StarfundEscrow::set_protocol_fee_bps`] — see
> [`set_protocol_fee_bps` — updates the rate](#set_protocol_fee_bps--updates-the-rate-admin-authorized-open-only)
> below. `withdraw` always realizes whatever rate is stored *at withdrawal time*, not
> necessarily the rate configured at `init`.

---

## Roles

| Role | Fee-related capability |
|------|------------------------|
| **Admin** | Sets `protocol_fee_bps` at `init`, and may change it again at any time while the escrow is `Open` via `set_protocol_fee_bps` (nonce-protected, bounds-checked). No fee-related power once the escrow leaves `Open`. |
| **SME** (`sme_address`) | Triggers fee *realization* by calling `withdraw`. Does not choose or see the fee rate change — it was fixed at `init`. |
| **Treasury** | Passive recipient of the fee transfer inside `withdraw`. **Does not authorize anything on this path** — see [Treasury is not authorized here](#treasury-is-not-authorized-on-the-withdraw-path). |
| **Investor** | No interaction with fees at all. Settlement (`settle`), claims (`claim_investor_payout`), and refunds (`refund`) never apply or reference `protocol_fee_bps`. |

---

## Entrypoints

### `init` — configures the fee (admin-authorized)

`escrow/src/lib.rs:1807` (`pub fn init`), fee handling at `1826`, `1841`–`1848`.

- **Auth:** `admin.require_auth()` (`lib.rs:1828`) — the same single authorization that gates the
  entire `init` call; there is no fee-specific auth check separate from this.
- **Parameter:** `protocol_fee_bps: Option<i64>`. `None` defaults to `0` (no fee).
- **Validation:** must satisfy `0..=10_000` (basis points), the same envelope as `yield_bps`.
  `10_000` is a legal value — it routes the entire `funded_amount` to the treasury.
- **Rejection:** value outside `0..=10_000` → `EscrowError::ProtocolFeeBpsOutOfRange` (**215**),
  before any storage write. A caller that isn't `admin` fails Soroban's host-level authorization
  check (`Error(Auth, InvalidAction)`) — **not** a typed `EscrowError` — and no part of `init`
  (including the fee write) executes; see [ADR-002](adr/ADR-002-auth-boundaries.md) on why typed
  errors and auth failures are distinct failure classes.
- **Effect:** `protocol_fee_bps` (or `0`) is written once to `DataKey::ProtocolFeeBps` and never
  read-modified again by any other entrypoint.

### `set_protocol_fee_bps` — updates the rate (admin-authorized, `Open`-only)

`escrow/src/lib.rs:4749` (`pub fn set_protocol_fee_bps`).

- **Auth:** `admin.require_auth()`, via `load_escrow_require_admin` (`lib.rs:4474`). Only the
  current escrow admin can call this; no other role can change the fee rate post-`init`.
- **Lifecycle restriction:** only callable while `status == 0` (`Open`). Once the escrow has
  advanced (funded, settled, withdrawn, cancelled), the rate is frozen for the life of that
  instance. Rejected with the error named `ProtocolFeeUpdateNotOpen` at `lib.rs:4755` when
  `status != 0` — see the note in [ADR cross-references](#cross-references); this identifier is
  not currently a documented numeric `EscrowError` code elsewhere in this repo, so treat the
  numeric value as unverified until confirmed against the compiled contract.
- **Replay protection:** takes `expected_nonce: u32` and consumes it via `consume_admin_nonce`
  (`lib.rs:4511`) — the call aborts with `AdminNonceMismatch` (**257**) if `expected_nonce` doesn't
  match the stored admin nonce. This is the same nonce-consumption pattern used by the other
  admin-gated setters in this contract (e.g. cap/floor raises and lowers); it prevents a captured
  or replayed authorization from being used twice.
- **Bounds:** `new_fee_bps` must satisfy `0..=10_000` (basis points) — the same envelope enforced
  at `init`. Rejected with `ProtocolFeeBpsOutOfRange` (**215**) outside that range.
- **No-op short-circuit:** if `new_fee_bps == old_fee_bps`, the call returns the unchanged value
  without writing storage or emitting an event.
- **Effect on success:** `protocol_fee_bps` is overwritten in `DataKey::ProtocolFeeBps`, and the
  function returns the new value. Emits [`ProtocolFeeUpdated`](#event-protocolfeeupdated) with both
  the old and new rate.

### `withdraw` — realizes the fee (SME-authorized)

`escrow/src/lib.rs:4371` (`pub fn withdraw`).

- **Auth:** `escrow.sme_address.require_auth()`, via the shared helper `load_escrow_require_sme`
  (`lib.rs:2299`). Admin, investor, and treasury cannot call `withdraw`.
- **Guard order** (read-only checks before auth, storage writes only after all checks pass):

  | # | Check | On failure |
  |---|-------|------------|
  | 1 | Operational pause inactive | `PausedBlocksWithdrawal` (212) |
  | 2 | Legal hold inactive | `LegalHoldBlocksWithdrawal` (123) |
  | 3 | **`sme_address.require_auth()`** | host-level auth failure (not a typed code) |
  | 4 | `status == 1` (funded) | `WithdrawalNotFunded` (124) |
  | 5 | Contract token balance `>= funded_amount` | `InsufficientContractBalance` (165) |
  | 6 | `fee = funded_amount * fee_bps / 10_000` (checked, floor) | `WithdrawFeeArithmeticOverflow` (216) |
  | 7 | `net = funded_amount - fee` (checked) | `WithdrawNetArithmeticUnderflow` (217) — see note below |

  Checks 1–2 are read-only and run **before** the SME authorization call, so an unauthorized
  caller cannot even learn whether the escrow is paused/held from this call's failure mode
  ordering advantage — but note check 3 itself still fails independently of 4–7 regardless of
  fee configuration.

  **Note on 217 (`WithdrawNetArithmeticUnderflow`):** with `fee_bps` validated to `0..=10_000` at
  `init`, `fee` can never exceed `funded_amount`, so this underflow is **unreachable in practice**
  today. It exists as a defensive guard (checked arithmetic on principle, not because a live input
  can trigger it) rather than a reachable rejection path — do not design integration tests
  expecting to hit it under normal configuration.

- **Effect on success:** `status` → `3` (withdrawn, terminal); `DistributedPrincipal` increases by
  the **full gross** `funded_amount` (fee + net combined) so liability accounting stays correct
  regardless of the split; `fee` transfers to treasury (skipped entirely when `fee == 0` — no
  transfer call is made, preserving the pre-fee gas profile); `net` transfers to `sme_address`.
  Emits [`SmeWithdrew`](#event-smewithdrew). The `fee_bps` used here is whatever is stored in
  `DataKey::ProtocolFeeBps` **at the moment `withdraw` runs** — if `set_protocol_fee_bps` changed
  it after `init`, this is the updated rate, not the `init`-time rate.

  Do not confuse this applied rate with the entirely separate `FeeSchedule`
  governance/off-chain-tracking mechanism (`submit_fee_schedule`, `get_active_fee_schedule`, etc.,
  keyed by `FeeScheduleStorageKey`) documented in
  [`escrow-read-api.md`](escrow-read-api.md#fee-governance-informational-only) — that schedule is
  **informational only** and is never read by `withdraw` or `release`.

### `get_protocol_fee_bps` — reads the configured rate (unauthenticated)

`escrow/src/lib.rs:2397`.

- **Auth:** none — pure read, callable by anyone, no state mutation.
- **Returns:** `0` for escrows created before this field existed (additive-key default per
  [ADR-007](adr/ADR-007-storage-key-evolution.md)) or for escrows where `protocol_fee_bps` was
  never supplied at `init`.

---

## Update path: `set_protocol_fee_bps`

Like `MaxUniqueInvestorsCap`, `MaxPerInvestorCap`, and `MinContributionFloor` — each of which has a
dedicated admin `raise_*`/`lower_*` entrypoint — `protocol_fee_bps` also has a dedicated
admin-gated setter: [`set_protocol_fee_bps`](#set_protocol_fee_bps--updates-the-rate-admin-authorized-open-only).
It differs from those cap/floor setters in two ways: it is a single bidirectional setter (no
separate `raise_`/`lower_` split) and it is restricted to the `Open` status rather than being
callable across `Open`/`Funded`.

Once the escrow leaves `Open` (funded, settled, withdrawn, or cancelled), the rate is frozen for
the rest of that instance's life — there is no setter reachable from any later status.

## Treasury is not authorized on the withdraw path

The treasury address only appears in `withdraw` as a **payment destination** —
`Self::treasury_or_fail(&env)` (`lib.rs:1676`) reads the stored address with no `require_auth()`
call attached. This is intentional and matches normal payment semantics (a recipient doesn't need
to authorize receiving funds), but it is easy to conflate with
[`sweep_terminal_dust`](escrow-token-safety.md), a *different* entrypoint where the treasury **is**
the caller and **does** authorize (`treasury.require_auth()`). Do not assume "treasury involved" ⇒
"treasury authorized" — check the specific entrypoint.

## Event: `SmeWithdrew`

`escrow/src/lib.rs:1382` (`pub struct SmeWithdrew`). Emitted exactly once per successful `withdraw`, after both transfers
complete.

| Field | Type | Meaning |
|-------|------|---------|
| `name` (topic) | `Symbol` | `"sme_wd"` |
| `invoice_id` (topic) | `Symbol` | The escrow's invoice identifier |
| `amount` | `i128` | **Net** amount transferred to the SME (`funded_amount - fee`) — not the gross `funded_amount` |
| `recipient` | `Address` | The `sme_address` that received `amount` |
| `fee` | `i128` | Protocol fee transferred to treasury (`0` when `protocol_fee_bps == 0`) |

Indexers reconstructing gross principal must compute `amount + fee`, not read `amount` alone.

## Event: `ProtocolFeeUpdated`

`escrow/src/lib.rs:2712` (`pub struct ProtocolFeeUpdated`). Emitted exactly once per successful
`set_protocol_fee_bps` call that actually changes the rate (the no-op `new_fee_bps == old_fee_bps`
path returns early without publishing this event).

| Field | Type | Meaning |
|-------|------|---------|
| `name` (topic) | `Symbol` | `"fee_upd"` |
| `invoice_id` (topic) | `Symbol` | The escrow's invoice identifier |
| `old_fee_bps` | `i64` | The rate in effect immediately before this call |
| `new_fee_bps` | `i64` | The rate in effect immediately after this call |

---

## Worked example

Escrow created with `funded_amount = 1,000,000` (base units) at withdrawal time.

| `protocol_fee_bps` | `fee` (`funded × bps ÷ 10 000`, floor) | `net` to SME | Treasury transfer made? |
|---|---|---|---|
| `0` (default / unset) | `0` | `1,000,000` | No — zero-fee path skips the treasury call entirely |
| `250` (2.5%) | `25,000` | `975,000` | Yes |
| `10,000` (100%) | `1,000,000` | `0` | Yes — SME transfer is skipped (`net == 0`), only treasury is paid |

**Rounding example** showing the floor favors the SME: `funded_amount = 1,000`, `protocol_fee_bps =
333` (3.33%): `fee = 1000 × 333 ÷ 10 000 = 33.3 → 33` (floored). `net = 967`. The `0.3` unit of
sub-basis-point residue stays with the SME; the treasury is never over-charged by rounding.

**Conservation invariant** (holds for every successful `withdraw`, verified by
`checked_add`/`checked_sub` at every step): `net + fee == funded_amount`, exactly, with no principal
created or destroyed by the split.

---

## Cross-references

- [ADR-002: Authorization Boundaries](adr/ADR-002-auth-boundaries.md) — the general
  canonical-sequence rule (`guard_not_legal_hold` → `require_auth` → writes) that `withdraw`
  follows; this document is the fee-specific instance of that pattern.
- [ADR-007: Storage Key Evolution](adr/ADR-007-storage-key-evolution.md) — why
  `get_protocol_fee_bps` defaults to `0` for pre-existing escrow instances.
- `consume_admin_nonce` (`lib.rs:4511`) — the shared nonce-consumption helper used by
  `set_protocol_fee_bps` and every other nonce-protected admin setter in this contract.
- [`docs/escrow-token-safety.md`](escrow-token-safety.md) — the balance-delta-checked transfer
  wrapper both the fee and net legs of `withdraw` use.
