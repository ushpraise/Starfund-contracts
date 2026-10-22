//! Integration tests for [`StarfundEscrow::batch_record_collateral`].
//!
//! Covers all boundary and failure paths required by issue #241:
//! - Empty batch rejected (`CollateralBatchEmpty`)
//! - Batch exceeding `MAX_COLLATERAL_BATCH` rejected (`CollateralBatchTooLarge`)
//! - Atomic rollback when an invalid item occurs midway through the batch
//! - Backward timestamp against existing commitment (`CollateralTimestampBackwards`)
//! - `prior_amount` chaining in emitted events across the batch loop

use super::{assert_contract_error, deploy};
use crate::{
    CollateralRecordedEvt, EscrowError, StarfundEscrowClient, MAX_COLLATERAL_BATCH,
};
use soroban_sdk::{
    symbol_short,
    testutils::{Address as _, Events as _, Ledger},
    Address, Env, Symbol, Vec as SorobanVec,
};

// ── helpers ───────────────────────────────────────────────────────────────────

/// Deploy and initialize a minimal escrow so the SME role is available.
fn init_collateral_escrow(env: &Env) -> StarfundEscrowClient<'_> {
    env.mock_all_auths();
    let client = deploy(env);
    let admin = Address::generate(env);
    let sme = Address::generate(env);
    let token = Address::generate(env);
    let treasury = Address::generate(env);
    client.init(
        &admin,
        &soroban_sdk::String::from_str(env, "BATCHCOL"),
        &sme,
        &10_000i128,
        &500i64,
        &0u64,
        &token,
        &None,
        &treasury,
        &None,
        &None,
        &None,
        &None,
        &None,
        &None,
        &None,
        &None,
        &None::<i64>,
        &None::<u32>,
    );
    client
}

// ── test: empty batch rejected ────────────────────────────────────────────────

/// `batch_record_collateral` with an empty `items` vector must return
/// `CollateralBatchEmpty` (code 64) and leave storage unmodified.
#[test]
fn test_batch_collateral_empty_rejected() {
    let env = Env::default();
    let client = init_collateral_escrow(&env);

    let items: SorobanVec<(Symbol, i128)> = SorobanVec::new(&env);

    assert_contract_error(
        client.try_batch_record_collateral(&items),
        EscrowError::CollateralBatchEmpty,
    );

    // Storage must remain unmodified.
    assert!(
        client.get_sme_collateral_commitment().is_none(),
        "no collateral must be stored after empty-batch rejection"
    );
}

// ── test: batch exceeding MAX_COLLATERAL_BATCH rejected ───────────────────────

/// A batch with `items.len() > MAX_COLLATERAL_BATCH` must be rejected with
/// `CollateralBatchTooLarge` (code 65) before any storage write occurs.
#[test]
fn test_batch_collateral_exceeding_max_batch_rejected() {
    let env = Env::default();
    let client = init_collateral_escrow(&env);

    // Build a batch one item over the limit.
    let over_limit = MAX_COLLATERAL_BATCH + 1;
    let mut items: SorobanVec<(Symbol, i128)> = SorobanVec::new(&env);
    for i in 0..over_limit {
        let asset = Symbol::new(&env, "GOLD");
        items.push_back((asset, (i as i128 + 1) * 100));
    }

    assert_contract_error(
        client.try_batch_record_collateral(&items),
        EscrowError::CollateralBatchTooLarge,
    );

    // No side-effects: storage must still be empty.
    assert!(
        client.get_sme_collateral_commitment().is_none(),
        "no collateral must be stored after over-limit rejection"
    );
}

// ── test: atomic rollback on invalid item (zero amount) ──────────────────────

/// If item 0 is valid but item 1 has `amount <= 0`, the entire batch must revert.
/// Storage must remain completely unmodified.
#[test]
fn test_batch_collateral_atomic_rollback_on_invalid_item() {
    let env = Env::default();
    let client = init_collateral_escrow(&env);

    let gold = Symbol::new(&env, "GOLD");
    let usdc = Symbol::new(&env, "USDC");

    // Item 0 is valid; item 1 has a zero (non-positive) amount.
    let mut items: SorobanVec<(Symbol, i128)> = SorobanVec::new(&env);
    items.push_back((gold.clone(), 500i128)); // valid
    items.push_back((usdc.clone(), 0i128));   // invalid: amount <= 0

    assert_contract_error(
        client.try_batch_record_collateral(&items),
        EscrowError::CollateralAmountNotPositive,
    );

    // Entire batch must have been rolled back — no commitment written.
    assert!(
        client.get_sme_collateral_commitment().is_none(),
        "storage must be unmodified after atomic rollback"
    );
}

/// If item 0 is valid but item 1 has an empty symbol, the entire batch must revert.
#[test]
fn test_batch_collateral_atomic_rollback_on_empty_symbol() {
    let env = Env::default();
    let client = init_collateral_escrow(&env);

    let gold = Symbol::new(&env, "GOLD");
    let empty = Symbol::new(&env, "");

    let mut items: SorobanVec<(Symbol, i128)> = SorobanVec::new(&env);
    items.push_back((gold.clone(), 500i128));  // valid
    items.push_back((empty.clone(), 100i128)); // invalid: empty symbol

    assert_contract_error(
        client.try_batch_record_collateral(&items),
        EscrowError::CollateralAssetEmpty,
    );

    assert!(
        client.get_sme_collateral_commitment().is_none(),
        "storage must be unmodified after atomic rollback on empty symbol"
    );
}

// ── test: backward timestamp rejected ────────────────────────────────────────

/// When a prior commitment exists at timestamp T, a batch submitted at ledger
/// time T-1 must be rejected with `CollateralTimestampBackwards` (code 62).
#[test]
fn test_batch_collateral_backward_timestamp_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let client = deploy(&env);
    let admin = Address::generate(&env);
    let sme = Address::generate(&env);
    let token = Address::generate(&env);
    let treasury = Address::generate(&env);

    // Initialize at time 2000.
    env.ledger().set_timestamp(2_000);
    client.init(
        &admin,
        &soroban_sdk::String::from_str(&env, "BKWD01"),
        &sme,
        &10_000i128,
        &500i64,
        &0u64,
        &token,
        &None,
        &treasury,
        &None,
        &None,
        &None,
        &None,
        &None,
        &None,
        &None,
        &None,
        &None::<i64>,
        &None::<u32>,
    );

    // Record a single commitment at time 2000.
    client.record_sme_collateral_commitment(&Symbol::new(&env, "GOLD"), &500i128);

    // Roll the ledger clock backward to 1999 for the batch submission.
    // The guard checks `now >= existing.recorded_at`; 1999 < 2000 triggers the error.
    env.ledger().set_timestamp(1_999);

    let mut items: SorobanVec<(Symbol, i128)> = SorobanVec::new(&env);
    items.push_back((Symbol::new(&env, "SILVER"), 200i128));

    assert_contract_error(
        client.try_batch_record_collateral(&items),
        EscrowError::CollateralTimestampBackwards,
    );

    // The original commitment must still be intact.
    let stored = client.get_sme_collateral_commitment().unwrap();
    assert_eq!(stored.amount, 500i128, "original commitment must be preserved after backward-ts rejection");
}

// ── test: prior_amount chaining in emitted events ────────────────────────────

/// Verifies that successive items in a batch correctly chain `prior_amount`
/// across emitted `CollateralRecordedEvt` events:
/// - Event for item 0: `prior_amount` == amount of the pre-existing commitment (0 if none).
/// - Event for item 1: `prior_amount` == item 0's `amount`.
/// - Event for item 2: `prior_amount` == item 1's `amount`.
#[test]
fn test_batch_collateral_event_prior_amount_progression() {
    let env = Env::default();
    env.mock_all_auths();
    let client = deploy(&env);
    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let treasury = Address::generate(&env);
    let sme = Address::generate(&env);

    env.ledger().set_timestamp(1_000);
    client.init(
        &admin,
        &soroban_sdk::String::from_str(&env, "EVTCHAIN"),
        &sme,
        &10_000i128,
        &500i64,
        &0u64,
        &token,
        &None,
        &treasury,
        &None,
        &None,
        &None,
        &None,
        &None,
        &None,
        &None,
        &None,
        &None::<i64>,
        &None::<u32>,
    );

    let gold = Symbol::new(&env, "GOLD");
    let usdc = Symbol::new(&env, "USDC");
    let eth = Symbol::new(&env, "ETH");

    // No prior commitment — first batch of three items.
    let mut items: SorobanVec<(Symbol, i128)> = SorobanVec::new(&env);
    items.push_back((gold.clone(), 100i128));
    items.push_back((usdc.clone(), 200i128));
    items.push_back((eth.clone(), 300i128));

    let last = client.batch_record_collateral(&items);

    // The stored commitment must be the last item.
    assert_eq!(last.amount, 300i128);
    assert_eq!(last.asset, eth);

    // Verify emitted events by inspecting the event log.
    // CollateralRecordedEvt fields: name (topic), invoice_id, amount, prior_amount.
    let invoice_id = client.get_escrow().invoice_id;
    let all_events = env.events().all();

    let expected_evt0 = CollateralRecordedEvt {
        name: symbol_short!("coll_rec"),
        invoice_id: invoice_id.clone(),
        amount: 100i128,
        prior_amount: 0i128,   // No prior commitment before the batch.
    }
    .to_xdr(&env, &client.address);

    let expected_evt1 = CollateralRecordedEvt {
        name: symbol_short!("coll_rec"),
        invoice_id: invoice_id.clone(),
        amount: 200i128,
        prior_amount: 100i128, // Item 0's amount chains as prior for item 1.
    }
    .to_xdr(&env, &client.address);

    let expected_evt2 = CollateralRecordedEvt {
        name: symbol_short!("coll_rec"),
        invoice_id: invoice_id.clone(),
        amount: 300i128,
        prior_amount: 200i128, // Item 1's amount chains as prior for item 2.
    }
    .to_xdr(&env, &client.address);

    // The last three events in the log must match our expected chain.
    let n = all_events.len() as usize;
    assert!(n >= 3, "expected at least 3 events but got {n}");
    let evt0 = all_events.get((n - 3) as u32).unwrap();
    let evt1 = all_events.get((n - 2) as u32).unwrap();
    let evt2 = all_events.get((n - 1) as u32).unwrap();

    assert_eq!(evt0, expected_evt0, "item 0 event must chain prior_amount=0");
    assert_eq!(evt1, expected_evt1, "item 1 event must chain prior_amount=100");
    assert_eq!(evt2, expected_evt2, "item 2 event must chain prior_amount=200");
}

// ── test: successful single-item batch ───────────────────────────────────────

/// A batch with a single valid item must succeed and update the stored commitment.
#[test]
fn test_batch_collateral_single_valid_item_succeeds() {
    let env = Env::default();
    let client = init_collateral_escrow(&env);
    env.ledger().set_timestamp(500);

    let mut items: SorobanVec<(Symbol, i128)> = SorobanVec::new(&env);
    items.push_back((Symbol::new(&env, "SILVER"), 750i128));

    let result = client.batch_record_collateral(&items);
    assert_eq!(result.amount, 750i128);
    assert_eq!(result.asset, Symbol::new(&env, "SILVER"));
    assert_eq!(result.recorded_at, 500u64);

    let stored = client.get_sme_collateral_commitment().unwrap();
    assert_eq!(stored.amount, 750i128);
}
