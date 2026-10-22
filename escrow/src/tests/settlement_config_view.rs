//! Tests for [`StarfundEscrow::get_settlement_config`].
//!
//! Covers:
//! - Default values before [`StarfundEscrow::init`] is called.
//! - Values reflect what was passed to `init`.
//! - Values match the individual getters so the bundled view cannot drift.
//! - The bundled view is idempotent (pure read).
//! - The struct shape is pinned via field-by-field destructuring.

use super::super::{StarfundEscrow, StarfundEscrowClient, SettlementConfig};
use crate::DEFAULT_MATURITY_MAX_HORIZON_SECS;
use soroban_sdk::testutils::{Address as _, Ledger};
use soroban_sdk::{Address, Env};

// ── helpers ──────────────────────────────────────────────────────────────────

fn deploy(env: &Env) -> StarfundEscrowClient<'_> {
    let id = env.register(StarfundEscrow, ());
    StarfundEscrowClient::new(env, &id)
}

/// Init with caller-supplied `yield_bps`, `maturity`, and `protocol_fee_bps`.
fn init_escrow(
    env: &Env,
    client: &StarfundEscrowClient,
    yield_bps: i64,
    maturity: u64,
    protocol_fee_bps: Option<i64>,
) {
    let admin = Address::generate(env);
    let sme = Address::generate(env);
    let token = Address::generate(env);
    let treasury = Address::generate(env);
    client.init(
        &admin,
        &soroban_sdk::String::from_str(env, "SETCFG01"),
        &sme,
        &10_000i128,
        &yield_bps,
        &maturity,
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
        &protocol_fee_bps,
    );
}

// ── tests ────────────────────────────────────────────────────────────────────

/// Before `init`, every field must return its documented default.
#[test]
fn test_defaults_before_init() {
    let env = Env::default();
    let client = deploy(&env);

    let config = client.get_settlement_config();
    assert_eq!(config.yield_bps, 0);
    assert_eq!(config.maturity, 0);
    assert_eq!(config.protocol_fee_bps, 0);
    assert_eq!(config.yield_tiers.len(), 0);
    assert_eq!(
        config.maturity_max_horizon,
        DEFAULT_MATURITY_MAX_HORIZON_SECS
    );
    assert_eq!(config.funding_deadline, None);
    assert_eq!(config.min_contribution_floor, 0);
    assert_eq!(config.max_unique_investors_cap, None);
    assert_eq!(config.max_per_investor_cap, None);
}

/// After `init`, fields should reflect the values supplied at init time.
#[test]
fn test_values_after_init_basic() {
    let env = Env::default();
    env.mock_all_auths();
    let client = deploy(&env);
    init_escrow(&env, &client, 800i64, 0u64, None);

    let config = client.get_settlement_config();
    assert_eq!(config.yield_bps, 800);
    assert_eq!(config.maturity, 0);
    assert_eq!(config.protocol_fee_bps, 0);
}

/// Non-zero `maturity` must be reflected.
#[test]
fn test_values_after_init_with_maturity() {
    let env = Env::default();
    env.mock_all_auths();
    let client = deploy(&env);
    init_escrow(&env, &client, 800i64, 2_000_000u64, None);

    let config = client.get_settlement_config();
    assert_eq!(config.maturity, 2_000_000);
}

/// Explicit `protocol_fee_bps` at init must be reflected.
#[test]
fn test_values_after_init_with_protocol_fee() {
    let env = Env::default();
    env.mock_all_auths();
    let client = deploy(&env);
    init_escrow(&env, &client, 800i64, 0u64, Some(500i64));

    let config = client.get_settlement_config();
    assert_eq!(config.protocol_fee_bps, 500);
}

/// `get_settlement_config` must match the individual getters so the bundled view
/// cannot drift from the authoritative single-key reads.
#[test]
fn test_config_matches_individual_getters() {
    let env = Env::default();
    env.mock_all_auths();
    let client = deploy(&env);
    init_escrow(&env, &client, 800i64, 0u64, Some(500i64));

    let config = client.get_settlement_config();
    let escrow = client.get_escrow();
    assert_eq!(config.yield_bps, escrow.yield_bps);
    assert_eq!(config.maturity, escrow.maturity);
    assert_eq!(config.protocol_fee_bps, client.get_protocol_fee_bps());
}

/// `get_settlement_config` has the expected shape — verified via field-by-field
/// destructuring so a future struct change causes a compile error.
#[test]
fn test_config_struct_shape() {
    let env = Env::default();
    env.mock_all_auths();
    let client = deploy(&env);
    init_escrow(&env, &client, 800i64, 0u64, None);

    let SettlementConfig {
        yield_bps,
        maturity,
        protocol_fee_bps,
        yield_tiers,
        maturity_max_horizon,
        funding_deadline,
        min_contribution_floor,
        max_unique_investors_cap,
        max_per_investor_cap,
    } = client.get_settlement_config();

    assert_eq!(yield_bps, 800);
    assert_eq!(maturity, 0);
    assert_eq!(protocol_fee_bps, 0);
    assert_eq!(yield_tiers.len(), 0);
    assert_eq!(maturity_max_horizon, DEFAULT_MATURITY_MAX_HORIZON_SECS);
    assert_eq!(funding_deadline, None);
    assert_eq!(min_contribution_floor, 0);
    assert_eq!(max_unique_investors_cap, None);
    assert_eq!(max_per_investor_cap, None);
}

/// Zero `protocol_fee_bps` (omitted at init) returns `0`, not an error.
#[test]
fn test_zero_protocol_fee_default() {
    let env = Env::default();
    env.mock_all_auths();
    let client = deploy(&env);
    init_escrow(&env, &client, 800i64, 0u64, None);

    assert_eq!(client.get_settlement_config().protocol_fee_bps, 0);
}

/// All fields remain stable between multiple calls (pure read, no state mutation).
#[test]
fn test_config_is_idempotent() {
    let env = Env::default();
    env.mock_all_auths();
    let client = deploy(&env);
    init_escrow(&env, &client, 800i64, 2_000_000u64, Some(500i64));

    let a = client.get_settlement_config();
    let b = client.get_settlement_config();
    assert_eq!(a.yield_bps, b.yield_bps);
    assert_eq!(a.maturity, b.maturity);
    assert_eq!(a.protocol_fee_bps, b.protocol_fee_bps);
    assert_eq!(a.maturity_max_horizon, b.maturity_max_horizon);
    assert_eq!(a.funding_deadline, b.funding_deadline);
    assert_eq!(a.min_contribution_floor, b.min_contribution_floor);
    assert_eq!(a.max_unique_investors_cap, b.max_unique_investors_cap);
    assert_eq!(a.max_per_investor_cap, b.max_per_investor_cap);
}

/// Defaults before init are also idempotent.
#[test]
fn test_defaults_idempotent_before_init() {
    let env = Env::default();
    let client = deploy(&env);

    let a = client.get_settlement_config();
    let b = client.get_settlement_config();
    assert_eq!(a.yield_bps, b.yield_bps);
    assert_eq!(a.maturity, b.maturity);
    assert_eq!(a.protocol_fee_bps, b.protocol_fee_bps);
    assert_eq!(a.maturity_max_horizon, b.maturity_max_horizon);
}

// ── EscrowSummary new-field tests (#240) ─────────────────────────────────────

/// Helper: deploy and init with an optional funding_deadline and token_decimals.
fn init_summary_escrow(
    env: &Env,
    client: &StarfundEscrowClient,
    protocol_fee_bps: Option<i64>,
    funding_deadline: Option<u64>,
    token_decimals: Option<u32>,
) {
    let admin = Address::generate(env);
    let sme = Address::generate(env);
    let token = Address::generate(env);
    let treasury = Address::generate(env);
    client.init(
        &admin,
        &soroban_sdk::String::from_str(env, "SUMCFG01"),
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
        &funding_deadline,
        &protocol_fee_bps,
        &token_decimals,
    );
}

/// Before `init`, dispute_active, funding_deadline, and token_decimals all
/// return their documented defaults.
#[test]
fn test_summary_defaults_before_init() {
    let env = Env::default();
    let client = deploy(&env);
    let summary = client.get_escrow_summary();

    assert!(!summary.dispute_active, "dispute_active must default to false");
    assert_eq!(summary.funding_deadline, None, "funding_deadline must default to None");
    assert_eq!(summary.token_decimals, None, "token_decimals must default to None");
}

/// After `init` without a funding_deadline or token_decimals, the summary fields
/// reflect the absent-key defaults.
#[test]
fn test_summary_new_fields_default_after_init_no_deadline_no_decimals() {
    let env = Env::default();
    env.mock_all_auths();
    let client = deploy(&env);
    init_summary_escrow(&env, &client, None, None, None);

    let summary = client.get_escrow_summary();
    assert!(!summary.dispute_active, "dispute_active must be false when no dispute set");
    assert_eq!(summary.funding_deadline, None, "funding_deadline must be None when not set");
    assert_eq!(summary.token_decimals, None, "token_decimals must be None when not set");
}

/// When a `funding_deadline` is supplied at init it must appear in the summary.
#[test]
fn test_summary_funding_deadline_populated() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_timestamp(1_000);
    let client = deploy(&env);
    let deadline: u64 = 5_000;
    init_summary_escrow(&env, &client, None, Some(deadline), None);

    let summary = client.get_escrow_summary();
    assert_eq!(
        summary.funding_deadline,
        Some(deadline),
        "funding_deadline in summary must match value passed at init"
    );
}

/// When `token_decimals` is supplied at init it must appear in the summary.
#[test]
fn test_summary_token_decimals_populated() {
    let env = Env::default();
    env.mock_all_auths();
    let client = deploy(&env);
    init_summary_escrow(&env, &client, None, None, Some(7u32));

    let summary = client.get_escrow_summary();
    assert_eq!(
        summary.token_decimals,
        Some(7u32),
        "token_decimals in summary must match value passed at init"
    );
}

/// `dispute_active` in the summary reflects the live dispute state.
#[test]
fn test_summary_dispute_active_reflects_state() {
    let env = Env::default();
    env.mock_all_auths();
    let client = deploy(&env);
    init_summary_escrow(&env, &client, None, None, None);

    // Initially no dispute.
    assert!(!client.get_escrow_summary().dispute_active);

    // Activate dispute.
    client.set_dispute_active(&true);
    assert!(
        client.get_escrow_summary().dispute_active,
        "dispute_active must be true after set_dispute_active(true)"
    );

    // Clear dispute.
    client.set_dispute_active(&false);
    assert!(
        !client.get_escrow_summary().dispute_active,
        "dispute_active must be false after set_dispute_active(false)"
    );
}

/// `get_escrow_summary` must match the individual getters so the bundled view
/// cannot drift.
#[test]
fn test_summary_new_fields_match_individual_getters() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_timestamp(100);
    let client = deploy(&env);
    let deadline: u64 = 5_000;
    init_summary_escrow(&env, &client, Some(300i64), Some(deadline), Some(6u32));

    let summary = client.get_escrow_summary();
    assert_eq!(summary.dispute_active, client.is_dispute_active());
    assert_eq!(summary.funding_deadline, client.get_funding_deadline());
    // token_decimals matches what was passed at init (7 decimals stored as FundingTokenScale).
    assert_eq!(summary.token_decimals, Some(6u32));
}

/// `get_escrow_summary` is a pure read — calling it multiple times returns
/// identical results for the new fields.
#[test]
fn test_summary_new_fields_are_idempotent() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_timestamp(100);
    let client = deploy(&env);
    let deadline: u64 = 5_000;
    init_summary_escrow(&env, &client, Some(200i64), Some(deadline), Some(4u32));

    let a = client.get_escrow_summary();
    let b = client.get_escrow_summary();
    assert_eq!(a.dispute_active, b.dispute_active);
    assert_eq!(a.funding_deadline, b.funding_deadline);
    assert_eq!(a.token_decimals, b.token_decimals);
}
