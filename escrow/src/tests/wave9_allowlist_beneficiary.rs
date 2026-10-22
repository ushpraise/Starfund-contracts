//! Focused regression tests for the Wave 9 allowlist and beneficiary issues.
//!
//! - #114 `set_investors_allowlisted` never persisted the allowlist index.
//! - #115 `get_allowlisted_investors_count` performed unbounded persistent reads.
//! - #116 `AllowlistIndex` lived in unbounded instance storage.
//! - #117 `rotate_beneficiary` lacked operational-pause and dispute gates.

use super::*;
use crate::{PauseReason, PauseScope, ALLOWLIST_PAGE_SIZE, MAX_INVESTOR_ALLOWLIST_BATCH};
use soroban_sdk::Vec as SorobanVec;

// ── #117: rotate_beneficiary pause / dispute gates ───────────────────────────

#[test]
fn rotate_beneficiary_blocked_while_paused() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin, sme) = setup(&env);
    let new_sme = Address::generate(&env);
    default_init(&client, &env, &admin, &sme);

    client.set_paused(&true, &PauseScope::All, &PauseReason::Security);

    assert_contract_error(
        client.try_rotate_beneficiary(&new_sme, &0u32),
        EscrowError::PausedBlocksBeneficiaryRotation,
    );
    assert_eq!(
        client.get_escrow().sme_address,
        sme,
        "beneficiary must not change while paused"
    );
}

#[test]
fn rotate_beneficiary_blocked_while_disputed() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin, sme) = setup(&env);
    let new_sme = Address::generate(&env);
    default_init(&client, &env, &admin, &sme);

    client.open_dispute(&admin);

    assert_contract_error(
        client.try_rotate_beneficiary(&new_sme, &0u32),
        EscrowError::DisputeBlocksBeneficiaryRotation,
    );
    assert_eq!(
        client.get_escrow().sme_address,
        sme,
        "beneficiary must not change while disputed"
    );
}

#[test]
fn rotate_beneficiary_succeeds_after_pause_cleared() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin, sme) = setup(&env);
    let new_sme = Address::generate(&env);
    default_init(&client, &env, &admin, &sme);

    client.set_paused(&true, &PauseScope::All, &PauseReason::Incident);
    // A guarded rejection must not consume the admin nonce.
    assert_contract_error(
        client.try_rotate_beneficiary(&new_sme, &0u32),
        EscrowError::PausedBlocksBeneficiaryRotation,
    );

    client.set_paused(&false, &PauseScope::All, &PauseReason::Incident);
    let updated = client.rotate_beneficiary(&new_sme, &0u32);
    assert_eq!(updated.sme_address, new_sme);
}

// ── #114: batch writes persist the index ─────────────────────────────────────

#[test]
fn batch_allowlist_persists_paged_index_and_count() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin, sme) = setup(&env);
    default_init(&client, &env, &admin, &sme);

    let batch: u32 = 5;
    let mut v: SorobanVec<Address> = SorobanVec::new(&env);
    let mut expected = SorobanVec::new(&env);
    for _ in 0..batch {
        let addr = Address::generate(&env);
        expected.push_back(addr.to_string());
        v.push_back(addr);
    }

    client.set_investors_allowlisted(&v, &true, &0u32);

    assert_eq!(client.get_allowlisted_investors_count(), batch);

    let page = client.get_allowlisted_investors(&0, &50);
    assert_eq!(page.len(), batch);
    for i in 0..page.len() {
        let item = page.get(i).unwrap().to_string();
        assert!(expected.iter().any(|s| s == item));
    }
}

// ── #115: O(1) count stays accurate across adds and revocations ──────────────

#[test]
fn allowlist_count_tracks_adds_and_revocations() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin, sme) = setup(&env);
    default_init(&client, &env, &admin, &sme);

    let a = Address::generate(&env);
    let b = Address::generate(&env);
    let c = Address::generate(&env);

    client.set_investor_allowlisted(&a, &true, &0u32);
    assert_eq!(client.get_allowlisted_investors_count(), 1);
    client.set_investor_allowlisted(&b, &true, &1u32);
    assert_eq!(client.get_allowlisted_investors_count(), 2);
    client.set_investor_allowlisted(&c, &true, &2u32);
    assert_eq!(client.get_allowlisted_investors_count(), 3);

    // Revoking the middle entry removes it from the index while preserving order.
    client.set_investor_allowlisted(&b, &false, &3u32);
    assert_eq!(client.get_allowlisted_investors_count(), 2);
    let page = client.get_allowlisted_investors(&0, &50);
    assert_eq!(page.len(), 2);
    assert_eq!(page.get(0).unwrap(), a);
    assert_eq!(page.get(1).unwrap(), c);

    // Re-allowlisting a previously revoked address increments the count again.
    client.set_investor_allowlisted(&b, &true, &4u32);
    assert_eq!(client.get_allowlisted_investors_count(), 3);

    // Revoking everything returns the count and enumeration to empty.
    client.set_investor_allowlisted(&a, &false, &5u32);
    client.set_investor_allowlisted(&b, &false, &6u32);
    client.set_investor_allowlisted(&c, &false, &7u32);
    assert_eq!(client.get_allowlisted_investors_count(), 0);
    assert_eq!(client.get_allowlisted_investors(&0, &50).len(), 0);
}

// ── #116: paged persistent index at scale ────────────────────────────────────

#[test]
fn paged_index_handles_500_allowlisted_addresses() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin, sme) = setup(&env);
    default_init(&client, &env, &admin, &sme);
    let contract_id = client.address.clone();

    let total: u32 = 500;
    let mut nonce: u32 = 0;
    let mut written: u32 = 0;
    while written < total {
        let take = (total - written).min(MAX_INVESTOR_ALLOWLIST_BATCH);
        let mut v: SorobanVec<Address> = SorobanVec::new(&env);
        for _ in 0..take {
            v.push_back(Address::generate(&env));
        }
        client.set_investors_allowlisted(&v, &true, &nonce);
        nonce += 1;
        written += take;
    }

    assert_eq!(client.get_allowlisted_investors_count(), total);

    // The index lives in fixed-size persistent pages; no unbounded address
    // collection remains in instance storage.
    let expected_pages = total.div_ceil(ALLOWLIST_PAGE_SIZE);
    env.as_contract(&contract_id, || {
        assert!(
            !env.storage().instance().has(&DataKey::AllowlistIndex),
            "allowlist index must not live in instance storage"
        );
        for page_idx in 0..expected_pages {
            let page: SorobanVec<Address> = env
                .storage()
                .persistent()
                .get(&DataKey::AllowlistPage(page_idx))
                .unwrap();
            assert!(page.len() <= ALLOWLIST_PAGE_SIZE);
        }
    });

    // Full enumeration across every page returns each address exactly once.
    let mut seen = SorobanVec::new(&env);
    let mut start: u32 = 0;
    loop {
        let page = client.get_allowlisted_investors(&start, &50);
        let len = page.len();
        if len == 0 {
            break;
        }
        for i in 0..len {
            let item = page.get(i).unwrap().to_string();
            assert!(
                !seen.iter().any(|s| s == item),
                "duplicate allowlisted address found during enumeration"
            );
            seen.push_back(item);
        }
        start += len;
    }
    assert_eq!(seen.len(), total);
}
