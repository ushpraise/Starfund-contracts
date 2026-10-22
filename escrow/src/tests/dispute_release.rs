use super::*;

/// Builds an escrow that is fully funded to its `funding_target`, so `status == 1`
/// (Funded). `withdraw()` requires `EscrowStatus::Funded`; withdrawing while the
/// escrow is still open (status 0) reverts with `WithdrawalNotFunded`.
///
/// A real Stellar Asset Contract is used because `withdraw()` transfers the
/// released principal to the SME and therefore needs an actual token balance on
/// the contract.
fn funded_client(env: &Env) -> (StarfundEscrowClient<'_>, Address, Address) {
    env.mock_all_auths();
    let sac = env.register_stellar_asset_contract_v2(Address::generate(env));
    let token = sac.address();
    let sac_admin = StellarAssetClient::new(env, &token);

    let (client, admin, sme) = setup(env);
    let (_, treasury) = free_addresses(env);
    client.init(
        &admin,
        &String::from_str(env, "DISPUTE001"),
        &sme,
        &100_000_000_000i128,
        &100i64,
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
    let investor = Address::generate(env);
    sac_admin.mint(&investor, &100_000_000_000i128);
    client.fund(&investor, &100_000_000_000i128);
    sac_admin.mint(&client.address, &100_000_000_000i128);
    (client, admin, sme)
}

#[test]
fn release_before_dispute_succeeds() {
    let env = Env::default();
    let (client, _, _) = funded_client(&env);
    let before = client.get_escrow();
    // `withdraw()` is gated on `EscrowStatus::Funded`; asserting status 1 here
    // documents the precondition the release path depends on.
    assert_eq!(before.status, 1);
    assert!(!before.dispute_active);

    let released = client.withdraw();
    assert_eq!(released.status, 3);
    assert!(!client.is_dispute_active());
}

#[test]
fn disputes_cannot_open_after_withdrawal_or_cancellation() {
    let (_, withdrawn_client, withdrawn_admin, _) = funded_client();
    withdrawn_client.withdraw();
    assert_eq!(withdrawn_client.get_escrow().status, 3);
    assert_contract_error(
        withdrawn_client.try_open_dispute(&withdrawn_admin),
        EscrowError::DisputeEscrowTerminated,
    );
    assert!(!withdrawn_client.is_dispute_active());
    assert_eq!(withdrawn_client.get_dispute_record(), None);

    let env = Env::default();
    env.mock_all_auths();
    let (cancelled_client, admin, sme) = setup(&env);
    default_init(&cancelled_client, &env, &admin, &sme);
    cancelled_client.cancel_funding(&0u32);
    assert_eq!(cancelled_client.get_escrow().status, 4);
    assert_contract_error(
        cancelled_client.try_open_dispute(&admin),
        EscrowError::DisputeEscrowTerminated,
    );
    assert!(!cancelled_client.is_dispute_active());
    assert_eq!(cancelled_client.get_dispute_record(), None);
}

#[test]
fn release_before_funding_is_rejected() {
    // Status 0 (open) is not withdrawable: `withdraw()` requires status 1.
    let env = Env::default();
    env.mock_all_auths();
    let sac = env.register_stellar_asset_contract_v2(Address::generate(&env));
    let token = sac.address();
    let sac_admin = StellarAssetClient::new(&env, &token);
    let (client, admin, sme) = setup(&env);
    let (_, treasury) = free_addresses(&env);
    client.init(
        &admin,
        &String::from_str(&env, "DISPUTE000"),
        &sme,
        &100_000_000_000i128,
        &100i64,
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
        &None,
        &None);
    let investor = Address::generate(&env);
    sac_admin.mint(&investor, &90_000_000_000i128);
    client.fund(&investor, &90_000_000_000i128);
    assert_eq!(client.get_escrow().status, 0);

    let result = client.try_withdraw();
    assert_contract_error(result, EscrowError::WithdrawalNotFunded);
}

#[test]
fn release_during_dispute_is_blocked() {
    let env = Env::default();
    let (client, admin, _) = funded_client(&env);
    client.open_dispute(&admin);
    assert!(client.is_dispute_active());
    assert!(client.get_escrow().dispute_active);

    let result = client.try_withdraw();
    assert_contract_error(result, EscrowError::DisputeBlocksWithdrawal);
}

#[test]
fn close_during_dispute_is_blocked() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin, sme) = setup(&env);
    client.init(
        &admin,
        &String::from_str(&env, "CLOSE-DISPUTE"),
        &sme,
        &1000i128,
        &100i64,
        &0u64,
        &Address::generate(&env),
        &None,
        &Address::generate(&env),
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
    client.open_dispute(&admin);

    let result = client.try_close_escrow();
    match result {
        Err(Err(InvokeError::Contract(code))) => {
            assert_eq!(code, crate::CloseError::ActiveDispute as u32);
        }
        other => panic!("expected active dispute error, got {other:?}"),
    }
}

#[test]
fn cancelled_escrow_cannot_close_until_refunds_clear_its_balance() {
    let env = Env::default();
    env.mock_all_auths();
    let sac = env.register_stellar_asset_contract_v2(Address::generate(&env));
    let token = sac.address();
    let sac_admin = StellarAssetClient::new(&env, &token);
    let (client, admin, sme) = setup(&env);
    let (_, treasury) = free_addresses(&env);
    client.init(
        &admin,
        &String::from_str(&env, "CLOSE-CANCELLED"),
        &sme,
        &1_000i128,
        &100i64,
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

    let investor = Address::generate(&env);
    sac_admin.mint(&investor, &500i128);
    client.fund(&investor, &500i128);
    client.cancel_funding(&0u32);

    let result = client.try_close_escrow();
    match result {
        Err(Err(InvokeError::Contract(code))) => {
            assert_eq!(code, crate::CloseError::ActiveBalance as u32);
        }
        other => panic!("expected active balance error, got {other:?}"),
    }

    client.refund(&investor);
    client.close_escrow();
    assert!(client.get_closure_metadata().is_some());
}

#[test]
fn dispute_opened_during_release_flow_blocks_release() {
    let env = Env::default();
    let (client, admin, _) = funded_client(&env);
    client.open_dispute(&admin);
    let result = client.try_withdraw();
    assert_contract_error(result, EscrowError::DisputeBlocksWithdrawal);
}

#[test]
fn dispute_resolved_then_release_succeeds() {
    let env = Env::default();
    let (client, admin, _) = funded_client(&env);
    client.open_dispute(&admin);
    // `close_dispute` no longer takes a `resolved` flag: closing a dispute
    // always resolves it.
    client.close_dispute(&admin);

    let released = client.withdraw();
    assert_eq!(released.status, 3);
    assert!(!client.is_dispute_active());
}

#[test]
fn close_dispute_resolves_and_clears_active_flag() {
    let env = Env::default();
    let (client, admin, _) = funded_client(&env);
    client.open_dispute(&admin);
    assert!(client.is_dispute_active());

    client.close_dispute(&admin);

    let record = client.get_dispute_record().unwrap();
    assert_eq!(record.state, crate::DisputeState::Resolved);
    assert!(record.resolved_at.is_some());
    assert!(record.resolved_by.is_some());
    assert!(!client.is_dispute_active());
    assert!(!client.get_escrow().dispute_active);
}

#[test]
fn set_dispute_active_resolves_through_dispute_lifecycle() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin, sme) = setup(&env);
    let (_, treasury) = free_addresses(&env);
    let sac = env.register_stellar_asset_contract_v2(Address::generate(&env));
    let token = sac.address();
    let sac_admin = StellarAssetClient::new(&env, &token);

    client.init(
        &admin,
        &String::from_str(&env, "DISPUTE-LC"),
        &sme,
        &100_000_000_000i128,
        &100i64,
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
    let investor = Address::generate(&env);
    sac_admin.mint(&investor, &100_000_000_000i128);
    client.fund(&investor, &100_000_000_000i128);
    sac_admin.mint(&client.address, &100_000_000_000i128);

    client.set_dispute_active(&true);
    assert!(client.is_dispute_active());
    assert!(client.get_escrow().dispute_active);
    assert_eq!(
        client.get_dispute_record().unwrap().state,
        crate::DisputeState::Open
    );

    let events_before = env.events().all().events().len();
    client.set_dispute_active(&false);

    let record = client.get_dispute_record().unwrap();
    assert_eq!(record.state, crate::DisputeState::Resolved);
    assert!(record.resolved_at.is_some());
    assert!(record.resolved_by.is_some());
    assert!(!client.is_dispute_active());
    assert!(!client.get_escrow().dispute_active);
    assert_eq!(env.events().all().events().len(), events_before + 1);
}

#[test]
fn unauthorized_dispute_close_panics() {
    let env = Env::default();
    let (client, _, _) = funded_client(&env);
    let outsider = Address::generate(&env);
    let err = client.try_close_dispute(&outsider);
    assert_contract_error(err, EscrowError::Unauthorized);
}

#[test]
fn cancel_funding_during_dispute_is_blocked() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin, sme) = setup(&env);
    default_init(&client, &env, &admin, &sme);

    client.open_dispute(&admin);
    assert!(client.is_dispute_active());

    let result = client.try_cancel_funding(&0u32);
    assert_contract_error(result, EscrowError::DisputeBlocksCancelFunding);
}

#[test]
fn reopening_dispute_preserves_previous_resolution_history() {
    let (_, client, admin, _) = funded_client();

    // First incident: open and resolve.
    client.open_dispute(&admin);
    let first = client.get_dispute_record().unwrap();
    client.close_dispute(&admin, &true);
    let resolved_first = client.get_dispute_record().unwrap();
    assert_eq!(resolved_first.state, DisputeState::Resolved);

    // Second incident: reopening must not destroy the first resolution record.
    client.open_dispute(&admin);
    let reopened = client.get_dispute_record().unwrap();
    assert_eq!(reopened.state, DisputeState::Open);

    // The historical audit log retains the resolved first incident.
    let history = client.get_dispute_history(&0u32, &10u32);
    assert_eq!(history.len(), 1);
    let archived = history.get(0).unwrap();
    assert_eq!(archived.state, DisputeState::Resolved);
    assert_eq!(archived.resolved_at, resolved_first.resolved_at);
    assert_eq!(archived.resolved_by, resolved_first.resolved_by);
    assert_ne!(archived.opened_at, first.opened_at);
}
