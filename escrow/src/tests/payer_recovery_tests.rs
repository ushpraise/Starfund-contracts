use crate::{
    tests::{assert_contract_error, deploy, free_addresses, setup},
    EscrowError, PayerRecoveryAccepted, PayerRecoveryProposed, PendingPayerCancelled,
};
use soroban_sdk::{
    symbol_short,
    testutils::{Address as _, Events},
    Address, Env, String,
};

#[test]
fn test_propose_payer_recovery_admin_gated() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin, sme) = setup(&env);

    let (token, treasury) = free_addresses(&env);
    client.init(
        &admin,
        &String::from_str(&env, "PAYER_REC_001"),
        &sme,
        &100_000_000_000i128,
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

    let new_payer = Address::generate(&env);
    let result = client.propose_payer_recovery(&new_payer, &None);
    assert_eq!(result, new_payer);

    let escrow = client.get_escrow();
    // Current payer should not have changed yet
    assert_eq!(escrow.payer, admin);
}

#[test]
fn test_propose_payer_recovery_non_admin_fails() {
    let env = Env::default();
    let (client, admin, sme) = setup(&env);

    let (token, treasury) = free_addresses(&env);
    client.init(
        &admin,
        &String::from_str(&env, "PAYER_REC_002"),
        &sme,
        &100_000_000_000i128,
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

    let new_payer = Address::generate(&env);
    // Don't mock auth for non-admin
    let res = client.try_propose_payer_recovery(&new_payer, &None);
    assert!(res.is_err());
}

#[test]
fn test_propose_payer_recovery_same_payer_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin, sme) = setup(&env);

    let (token, treasury) = free_addresses(&env);
    client.init(
        &admin,
        &String::from_str(&env, "PAYER_REC_003"),
        &sme,
        &100_000_000_000i128,
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

    // Try to propose the current payer (admin) as the new payer
    assert_contract_error(
        client.try_propose_payer_recovery(&admin, &None),
        EscrowError::NewPayerSameAsCurrent,
    );
}

#[test]
fn test_propose_payer_recovery_not_open_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin, sme) = setup(&env);

    let (token, treasury) = free_addresses(&env);
    client.init(
        &admin,
        &String::from_str(&env, "PAYER_REC_004"),
        &sme,
        &100_000_000_000i128,
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

    // Cancel the escrow to move it out of open/funded status
    client.cancel_funding();

    let new_payer = Address::generate(&env);
    assert_contract_error(
        client.try_propose_payer_recovery(&new_payer, &None),
        EscrowError::PayerRecoveryNotOpen,
    );
}

#[test]
fn test_accept_payer_recovery_succeeds() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin, sme) = setup(&env);

    let (token, treasury) = free_addresses(&env);
    client.init(
        &admin,
        &String::from_str(&env, "PAYER_REC_005"),
        &sme,
        &100_000_000_000i128,
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

    let new_payer = Address::generate(&env);
    client.propose_payer_recovery(&new_payer, &None);

    let updated_escrow = client.accept_payer_recovery();
    assert_eq!(updated_escrow.payer, new_payer);
}

#[test]
fn test_accept_payer_recovery_new_payer_auth_required() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin, sme) = setup(&env);

    let (token, treasury) = free_addresses(&env);
    client.init(
        &admin,
        &String::from_str(&env, "PAYER_REC_006"),
        &sme,
        &100_000_000_000i128,
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

    let new_payer = Address::generate(&env);
    client.propose_payer_recovery(&new_payer, &None);

    // Attempt accept without new_payer auth should fail
    env.mock_all_auths();
    env.set_default_account(&Address::generate(&env)); // Different account
    let res = client.try_accept_payer_recovery();
    assert!(res.is_err());
}

#[test]
fn test_accept_payer_recovery_no_pending_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin, sme) = setup(&env);

    let (token, treasury) = free_addresses(&env);
    client.init(
        &admin,
        &String::from_str(&env, "PAYER_REC_007"),
        &sme,
        &100_000_000_000i128,
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

    // Try to accept without proposing first
    assert_contract_error(
        client.try_accept_payer_recovery(),
        EscrowError::NoPendingPayer,
    );
}

#[test]
fn test_accept_payer_recovery_expired_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin, sme) = setup(&env);

    let (token, treasury) = free_addresses(&env);
    client.init(
        &admin,
        &String::from_str(&env, "PAYER_REC_008"),
        &sme,
        &100_000_000_000i128,
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

    let new_payer = Address::generate(&env);
    // Propose with a very short validity window (1 second)
    client.propose_payer_recovery(&new_payer, &Some(1));

    // Advance ledger time past expiry
    let mut ledger_info = env.ledger().get();
    ledger_info.timestamp += 2;
    env.ledger().set(ledger_info);

    assert_contract_error(
        client.try_accept_payer_recovery(),
        EscrowError::PayerProposalExpired,
    );
}

#[test]
fn test_cancel_pending_payer_admin_gated() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin, sme) = setup(&env);

    let (token, treasury) = free_addresses(&env);
    client.init(
        &admin,
        &String::from_str(&env, "PAYER_REC_009"),
        &sme,
        &100_000_000_000i128,
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

    let new_payer = Address::generate(&env);
    client.propose_payer_recovery(&new_payer, &None);

    // Admin can cancel
    client.cancel_pending_payer();

    // Verify payer hasn't changed
    let escrow = client.get_escrow();
    assert_eq!(escrow.payer, admin);
}

#[test]
fn test_cancel_pending_payer_non_admin_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin, sme) = setup(&env);

    let (token, treasury) = free_addresses(&env);
    client.init(
        &admin,
        &String::from_str(&env, "PAYER_REC_010"),
        &sme,
        &100_000_000_000i128,
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

    let new_payer = Address::generate(&env);
    client.propose_payer_recovery(&new_payer, &None);

    // Non-admin try to cancel
    let non_admin = Address::generate(&env);
    env.mock_all_auths();
    let res = client.try_cancel_pending_payer();
    assert!(res.is_err());
}

#[test]
fn test_cancel_pending_payer_no_pending_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin, sme) = setup(&env);

    let (token, treasury) = free_addresses(&env);
    client.init(
        &admin,
        &String::from_str(&env, "PAYER_REC_011"),
        &sme,
        &100_000_000_000i128,
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

    // Try to cancel without proposing first
    assert_contract_error(
        client.try_cancel_pending_payer(),
        EscrowError::NoPendingPayer,
    );
}

#[test]
fn test_payer_recovery_emits_correct_events() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin, sme) = setup(&env);
    let contract_id = client.address.clone();

    let (token, treasury) = free_addresses(&env);
    client.init(
        &admin,
        &String::from_str(&env, "PAYER_REC_012"),
        &sme,
        &100_000_000_000i128,
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

    let new_payer = Address::generate(&env);
    client.propose_payer_recovery(&new_payer, &None);

    let all_events = env.events().all();
    let propose_event = all_events
        .iter()
        .find(|e| {
            if let Ok(e) = PayerRecoveryProposed::from_xdr(&env, &e.clone().xdr) {
                true
            } else {
                false
            }
        })
        .expect("PayerRecoveryProposed event not found");

    // Accept and verify accepted event
    client.accept_payer_recovery();
    let all_events_after = env.events().all();
    let accept_event = all_events_after
        .iter()
        .rev()
        .find(|e| {
            if let Ok(e) = PayerRecoveryAccepted::from_xdr(&env, &e.clone().xdr) {
                true
            } else {
                false
            }
        })
        .expect("PayerRecoveryAccepted event not found");
}

#[test]
fn test_payer_recovery_full_lifecycle() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin, sme) = setup(&env);

    let (token, treasury) = free_addresses(&env);
    client.init(
        &admin,
        &String::from_str(&env, "PAYER_REC_013"),
        &sme,
        &100_000_000_000i128,
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

    let initial_payer = admin.clone();
    let escrow_before = client.get_escrow();
    assert_eq!(escrow_before.payer, initial_payer);

    // Propose recovery
    let new_payer = Address::generate(&env);
    client.propose_payer_recovery(&new_payer, &None);

    // Accept recovery
    let escrow_after = client.accept_payer_recovery();
    assert_eq!(escrow_after.payer, new_payer);
    assert_ne!(escrow_after.payer, initial_payer);
}
