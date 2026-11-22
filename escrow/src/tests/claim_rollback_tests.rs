//! `claim_investor_payout` sets the claimed marker before the outgoing token transfer. If that
//! transfer fails, the whole call must revert so an unpaid investor can retry instead of being
//! left marked as paid.

use super::*;
use crate::fixtures::{register_default_mock_token, DefaultMockTokenClient};
use soroban_sdk::{testutils::Address as _, Address, Env, String};

const TARGET: i128 = 1_000;
const YIELD_ONLY: i128 = 40;

/// A settled escrow funded through the registered mock token, with one unpaid investor.
fn settled_escrow(env: &Env) -> (StarfundEscrowClient<'_>, DefaultMockTokenClient<'_>, Address) {
    let (client, admin, sme) = setup(env);
    let (_, treasury) = free_addresses(env);
    let token_id = register_default_mock_token(env);
    let token = DefaultMockTokenClient::new(env, &token_id);
    client.init(
        &admin,
        &String::from_str(env, "CLAIMRB001"),
        &sme,
        &TARGET,
        &400i64,
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
        &None::<i64>,
        &None::<u32>,
    );
    let investor = Address::generate(env);
    token.mint(&investor, &TARGET);
    client.fund(&investor, &TARGET);
    token.mint(&client.address, &YIELD_ONLY);
    client.settle();
    (client, token, investor)
}

#[test]
fn failed_payout_transfer_rolls_back_the_claim_and_a_retry_pays_once() {
    let env = Env::default();
    let (client, token, investor) = settled_escrow(&env);
    let escrow_addr = client.address.clone();

    let payout = client.compute_investor_payout(&investor);
    assert!(payout > 0);
    let investor_before = token.balance(&investor);
    let escrow_before = token.balance(&escrow_addr);
    let events_before = env.events().all().events().len();
    assert!(!client.is_investor_claimed(&investor));

    // The registered token rejects the outgoing transfer after the marker write.
    token.set_fail_outgoing(&escrow_addr, &true);
    assert!(client.try_claim_investor_payout(&investor).is_err());

    // Nothing from the failed call persists: marker, balances and events are as before.
    assert!(!client.is_investor_claimed(&investor), "marker must roll back with the failed call");
    assert_eq!(token.balance(&investor), investor_before);
    assert_eq!(token.balance(&escrow_addr), escrow_before);
    assert_eq!(env.events().all().events().len(), events_before);

    // Normal behaviour restored: the retry pays exactly once.
    token.set_fail_outgoing(&escrow_addr, &false);
    client.claim_investor_payout(&investor);
    assert!(client.is_investor_claimed(&investor));
    assert_eq!(token.balance(&investor), investor_before + payout);
    assert_eq!(token.balance(&escrow_addr), escrow_before - payout);
    let events_after_retry = env.events().all().events().len();
    assert!(events_after_retry > events_before, "the successful retry emits its claim event");

    // A further claim is idempotent: no transfer, no new event.
    client.claim_investor_payout(&investor);
    assert_eq!(token.balance(&investor), investor_before + payout);
    assert_eq!(token.balance(&escrow_addr), escrow_before - payout);
    assert_eq!(env.events().all().events().len(), events_after_retry);
}
