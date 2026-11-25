//! Test-only fixture types and registration scaffolding.
//!
//! This module is compiled only for tests (or when the `testutils` feature is
//! enabled) so that fixture contracts such as [`DefaultMockToken`] never end up
//! in the production WASM build. Keeping them here separates test backend code
//! from the production fund-flow implementation in `lib.rs`.
//!
//! Native tests can reuse the helpers in this module to register the mock token
//! without duplicating setup code.

#![cfg(any(test, feature = "testutils"))]

use soroban_sdk::{contract, contractimpl, contracttype, Address, Env, String};

/// Minimal native token fixture used by tests.
///
/// Implements the subset of the SEP-41 token interface exercised by the escrow
/// contract: `mint`, `balance`, `transfer`, `approve`, `allowance`, and
/// `transfer_from`.
#[contract]
pub struct DefaultMockToken;

#[contracttype]
#[derive(Clone)]
enum DataKey {
    Balance(Address),
    Allowance(Address, Address),
    /// When `true`, `transfer` out of this address is rejected (failure-injection switch).
    FailOutgoing(Address),
}

#[contractimpl]
impl DefaultMockToken {
    /// Initialize the token with a name and symbol. Returns the token address.
    pub fn initialize(env: Env, _name: String, _symbol: String) -> Address {
        env.current_contract_address()
    }

    /// Mint `amount` tokens to `to`.
    pub fn mint(env: Env, to: Address, amount: i128) {
        let key = DataKey::Balance(to);
        let current: i128 = env.storage().persistent().get(&key).unwrap_or(0);
        env.storage().persistent().set(&key, &(current + amount));
    }

    /// Return the balance of `id`.
    pub fn balance(env: Env, id: Address) -> i128 {
        env.storage()
            .persistent()
            .get(&DataKey::Balance(id))
            .unwrap_or(0)
    }

    /// Make every `transfer` out of `from` fail (`fail == true`) or behave normally again.
    ///
    /// Deterministic failure fixture for tests that need an outgoing token call to be rejected
    /// and later restored. The default is `false`, so existing tests are unaffected.
    pub fn set_fail_outgoing(env: Env, from: Address, fail: bool) {
        env.storage().persistent().set(&DataKey::FailOutgoing(from), &fail);
    }

    /// Transfer `amount` from `from` to `to`.
    pub fn transfer(env: Env, from: Address, to: Address, amount: i128) {
        from.require_auth();
        let rejected: bool = env
            .storage()
            .persistent()
            .get(&DataKey::FailOutgoing(from.clone()))
            .unwrap_or(false);
        if rejected {
            panic!("transfer rejected by the failure fixture");
        }
        Self::spend_balance(&env, &from, amount);
        Self::receive_balance(&env, &to, amount);
    }

    /// Approve `spender` to spend up to `amount` on behalf of `from`.
    pub fn approve(env: Env, from: Address, spender: Address, amount: i128) {
        from.require_auth();
        env.storage()
            .persistent()
            .set(&DataKey::Allowance(from, spender), &amount);
    }

    /// Return the allowance `from` has granted to `spender`.
    pub fn allowance(env: Env, from: Address, spender: Address) -> i128 {
        env.storage()
            .persistent()
            .get(&DataKey::Allowance(from, spender))
            .unwrap_or(0)
    }

    /// Transfer `amount` from `from` to `to` using an existing allowance.
    pub fn transfer_from(env: Env, spender: Address, from: Address, to: Address, amount: i128) {
        spender.require_auth();
        let allowance_key = DataKey::Allowance(from.clone(), spender);
        let current: i128 = env.storage().persistent().get(&allowance_key).unwrap_or(0);
        if current < amount {
            panic!("insufficient allowance");
        }
        env.storage()
            .persistent()
            .set(&allowance_key, &(current - amount));
        Self::spend_balance(&env, &from, amount);
        Self::receive_balance(&env, &to, amount);
    }

    fn spend_balance(env: &Env, from: &Address, amount: i128) {
        let key = DataKey::Balance(from.clone());
        let current: i128 = env.storage().persistent().get(&key).unwrap_or(0);
        if current < amount {
            panic!("insufficient balance");
        }
        env.storage().persistent().set(&key, &(current - amount));
    }

    fn receive_balance(env: &Env, to: &Address, amount: i128) {
        let key = DataKey::Balance(to.clone());
        let current: i128 = env.storage().persistent().get(&key).unwrap_or(0);
        env.storage().persistent().set(&key, &(current + amount));
    }
}

/// Register a [`DefaultMockToken`] in `env` and return its address.
///
/// Reusable helper so native tests do not need to duplicate registration
/// scaffolding.
pub fn register_default_mock_token(env: &Env) -> Address {
    env.register_contract(None, DefaultMockToken)
}

/// Register a [`DefaultMockToken`], initialize it, and return a client along
/// with its address.
pub fn deploy_default_mock_token(
    env: &Env,
    name: String,
    symbol: String,
) -> (Address, DefaultMockTokenClient<'_>) {
    let address = register_default_mock_token(env);
    let client = DefaultMockTokenClient::new(env, &address);
    client.initialize(&name, &symbol);
    (address, client)
}

/// Register a [`DefaultMockToken`] and mint `amount` to `to`.
pub fn register_and_mint(env: &Env, to: &Address, amount: i128) -> Address {
    let address = register_default_mock_token(env);
    let client = DefaultMockTokenClient::new(env, &address);
    client.mint(to, &amount);
    address
}