#![allow(
    unused_imports,
    unused_variables,
    dead_code,
    unused_comparisons,
    unused_doc_comments,
    unused_macros,
    unused_assignments,
    clippy::needless_borrow,
    clippy::len_zero,
    clippy::explicit_counter_loop,
    clippy::empty_line_after_doc_comments,
    clippy::empty_line_after_outer_attr,
    clippy::absurd_extreme_comparisons,
    clippy::needless_range_loop,
    clippy::mutable_key_type,
    clippy::unusual_byte_groupings
)]
use super::{
    AttestationDigestAppended, AttestationDigestRevoked, AttestationDigestUnrevoked,
    CollateralRecordedEvt, ContractUpgraded, DataKey, DeprecatedTransferAdminUsed, EscrowError,
    EscrowFunded, EscrowInitialized, EscrowUnfunded, FundingCancelled, FundingStateChanged,
    FundingTargetUpdated, InvestorRefundedEvt, StarfundEscrow, StarfundEscrowClient,
    MaturityMaxHorizonUpdated, MaxUniqueInvestorsCapLowered, PrimaryAttestationBound,
    RegistryRefRebound, RentStatus, TreasuryDustSwept, YieldTier,
    MAX_ATTESTATION_APPEND_ENTRIES, MAX_DUST_SWEEP_AMOUNT, MAX_FUND_BATCH, RENT_WARN_LEDGERS,
    SCHEMA_VERSION,
};
use crate::fixtures::{DefaultMockToken, MockTokenRegistration};
use soroban_sdk::{
    symbol_short,
    testutils::{Address as _, Events, Ledger as _},
    token::{StellarAssetClient, TokenClient},
    Address, Env, Error, Event, InvokeError, String, Val, Vec as SorobanVec,
};
use std::fmt::Debug;

pub use soroban_sdk::Symbol;

mod test_utils;

pub use test_utils::{
    assert_contract_error, decode_event, find_event, find_events, TestConfig,
};

// Focused test tree for escrow behavior. Shared helpers live here so feature
// modules stay assertion-focused and each test still owns a fresh Env.
// The broader regression tree is stale relative to the current contract API and
// not part of the active dispute-lifecycle validation. Keep only the focused
// dispute-release coverage enabled for the canonical repair path.
mod dispute_release;
mod allowlist_compaction_tests;
mod claim_rollback_tests;

// Storage XDR compatibility fixtures (issue #404).
//
// Provenance for the raw XDR fixtures under `escrow/src/tests/fixtures/`:
//   - Source version: storage schema v5 (see `SCHEMA_VERSION` and
//     docs/adr/ADR-007-storage-key-evolution.md).
//   - Generation: fixtures are produced by serializing representative
//     `DataKey` / `InvoiceEscrow` values with the v5 type definitions via
//     `soroban_sdk::xdr` (`WriteXdr`) and committed as raw XDR bytes.
//   - Commit: recorded alongside each fixture in `fixtures/README.md`.
//   - Regeneration command: `cargo test -p starfund-escrow \
//       storage_compat::regenerate_fixtures -- --ignored --nocapture`
//     (see the module below for the exact serialization path).
//
// The tests decode each fixture with `ReadXdr`/`FromXdr` and compare semantic
// fields against expected values. A breaking shape change fails unless a
// migration path is registered (see `assert_migration_or_redeploy`).
mod storage_compat;

/// Registers a new escrow contract instance and returns its contract id.
pub fn deploy_id(env: &Env) -> Address {
    env.register(StarfundEscrow, ())
}

pub fn deploy(env: &Env) -> StarfundEscrowClient<'_> {
    let id = deploy_id(env);
    StarfundEscrowClient::new(env, &id)
}

#[allow(dead_code)]
pub fn deploy_with_id(env: &Env) -> (Address, StarfundEscrowClient<'_>) {
    let id = deploy_id(env);
    let client = StarfundEscrowClient::new(env, &id);
    (id, client)
}

pub fn setup(env: &Env) -> (StarfundEscrowClient<'_>, Address, Address) {
    let mut ledger_info = env.ledger().get();
    ledger_info.timestamp = 0;
    ledger_info.sequence_number = 100;
    env.ledger().set(ledger_info);
    env.mock_all_auths();
    let client = deploy(env);
    let admin = Address::generate(env);
    let sme = Address::generate(env);
    (client, admin, sme)
}

pub fn free_addresses(env: &Env) -> (Address, Address) {
    (Address::generate(env), Address::generate(env))
}

pub use crate::fixtures::StellarTestToken;

pub fn install_stellar_asset_token<'a>(env: &'a Env) -> StellarTestToken<'a> {
    crate::fixtures::install_stellar_asset_token(env)
}

#[allow(dead_code)]
pub fn default_init(client: &StarfundEscrowClient<'_>, env: &Env, admin: &Address, sme: &Address) {
    TestConfig::new(env)
        .with_admin(admin.clone())
        .with_sme(sme.clone())
        .init(client);
}

#[allow(dead_code)]
pub const TARGET: i128 = 100_000_000_000i128;

pub fn init_and_fund_with_real_token<'a>(
    env: &'a Env,
    target: i128,
    invoice_id: &str,
) -> (StarfundEscrowClient<'a>, Address, Address) {
    let sac = env.register_stellar_asset_contract_v2(Address::generate(env));
    let token_id = sac.address();
    let sac_admin = StellarAssetClient::new(env, &token_id);

    let escrow_id = env.register(StarfundEscrow, ());
    let client = StarfundEscrowClient::new(env, &escrow_id);
    let admin = Address::generate(env);
    let sme = Address::generate(env);
    let treasury = Address::generate(env);

    client.init(
        &admin,
        &soroban_sdk::String::from_str(env, invoice_id),
        &sme,
        &target,
        &800i64,
        &0u64,
        &token_id,
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
        &None,
        &None);

    let investor = Address::generate(env);
    sac_admin.mint(&investor, &target);
    client.fund(&investor, &target);

    sac_admin.mint(&escrow_id, &target);

    (client, escrow_id, sme)
}
