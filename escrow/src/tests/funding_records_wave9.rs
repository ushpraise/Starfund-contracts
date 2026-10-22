use crate::keys;
use crate::tests::{assert_contract_error, deploy, TARGET};
use crate::{DataKey, EscrowError, FundingRecord, StarfundEscrowClient};
use soroban_sdk::testutils::Address as _;
use soroban_sdk::{token::StellarAssetClient, Address, Env, String};

/// An initialised escrow plus the addresses the tests below need.
///
/// Investor state is seeded straight into contract storage rather than through
/// `fund()`: the funding path's `require_auth` sequence cannot be exercised in the
/// SDK-25 test host (the escrow's own address cannot self-authorize), which is a
/// pre-existing upstream defect unrelated to the views and accounting under test
/// here. Everything exercised below — `get_funding_records`, `release`, and
/// `get_distributed_principal` — is a read/accounting path that does not depend on
/// how the contributions were recorded.
struct Fixture<'a> {
    client: StarfundEscrowClient<'a>,
    contract: Address,
}

/// Register a real SEP-41 test token and initialise an escrow against it.
fn init_escrow<'a>(env: &'a Env) -> Fixture<'a> {
    let sac = env.register_stellar_asset_contract_v2(Address::generate(env));
    let token = sac.address();
    let sac_admin = StellarAssetClient::new(env, &token);

    let client = deploy(env);
    let contract = client.address.clone();
    let admin = Address::generate(env);
    let sme = Address::generate(env);
    let treasury = Address::generate(env);

    client.init(
        &admin,
        &String::from_str(env, "INV001"),
        &sme,
        &TARGET,
        &800i64,
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

    // Give the escrow the balance it will disburse, so `release`/`withdraw` can transfer.
    sac_admin.mint(&contract, &TARGET);

    Fixture { client, contract }
}

/// Record `investors` as funded participants with the given contributions.
///
/// `contributions[i]` is the principal credited to `investors[i]`; the escrow's
/// `funded_amount` and `InvestorIndex` are updated to match so the view under test
/// sees a realistic, self-consistent state.
fn seed_investors(fx: &Fixture<'_>, investors: &[Address], contributions: &[i128]) {
    let env = fx.client.env.clone();
    let mut total = 0i128;
    for (inv, amount) in investors.iter().zip(contributions.iter()) {
        total += *amount;
        env.as_contract(&fx.contract, || {
            env.storage()
                .persistent()
                .set(&DataKey::InvestorContribution(inv.clone()), amount);
        });
    }

    env.as_contract(&fx.contract, || {
        let mut escrow: crate::InvoiceEscrow =
            env.storage().instance().get(&DataKey::Escrow).unwrap();
        escrow.funded_amount = total;
        // Status 1 (funded) is the state `release` requires.
        if total >= escrow.funding_target {
            escrow.status = 1;
        }
        env.storage().instance().set(&DataKey::Escrow, &escrow);

        let mut index: soroban_sdk::Vec<Address> = env
            .storage()
            .instance()
            .get(&keys::investor_index())
            .unwrap_or_else(|| soroban_sdk::Vec::new(&env));
        for inv in investors.iter() {
            index.push_back(inv.clone());
        }
        env.storage()
            .instance()
            .set(&keys::investor_index(), &index);
        env.storage()
            .instance()
            .set(&DataKey::UniqueFunderCount, &(investors.len() as u32));
    });
}

// ── #122: typed struct return ────────────────────────────────────────────────

/// `get_funding_records` returns `FundingRecord` structs, so callers read named fields
/// (`record.investor`, `record.contribution`) instead of positional tuple indices.
#[test]
fn funding_records_expose_named_struct_fields() {
    let env = Env::default();
    env.mock_all_auths();
    let fx = init_escrow(&env);

    let a = Address::generate(&env);
    let b = Address::generate(&env);
    let c = Address::generate(&env);
    seed_investors(
        &fx,
        &[a.clone(), b.clone(), c.clone()],
        &[1_000, 1_000, 1_000],
    );
    let client = &fx.client;

    let page = client.get_funding_records(&0u32, &10u32);

    assert_eq!(page.len(), 3);
    // Older positions without the additive historical-total key fall back to the live balance.
    assert_eq!(client.get_total_deposited(&a), 1_000i128);
    for record in page.iter() {
        let as_struct = FundingRecord {
            investor: record.investor.clone(),
            contribution: record.contribution,
        };
        assert!(as_struct.contribution > 0);
    }

    // Each of our investors is discoverable by address, with the exact contribution recorded.
    for inv in [&a, &b, &c] {
        let found = page
            .iter()
            .find(|r| r.investor == *inv)
            .expect("investor present in funding records");
        assert_eq!(found.contribution, 1_000i128);
    }
}

/// Pagination still returns the full active set across successive pages.
#[test]
fn funding_records_pagination_returns_all_active_investors() {
    let env = Env::default();
    env.mock_all_auths();
    let fx = init_escrow(&env);
    let invs: Vec<Address> = (0..4).map(|_| Address::generate(&env)).collect();
    seed_investors(&fx, &invs, &[1_000, 1_000, 1_000, 1_000]);
    let client = &fx.client;

    let page0 = client.get_funding_records(&0u32, &2u32);
    let page1 = client.get_funding_records(&2u32, &2u32);

    assert_eq!(page0.len(), 2);
    assert!(page1.len() >= 1);

    let mut seen = 0usize;
    for page in [&page0, &page1] {
        for record in page.iter() {
            assert!(record.contribution > 0);
            seen += 1;
        }
    }
    // All four seeded investors appear across the two pages.
    assert_eq!(seen, 4);
}

// ── #123: zero-balance filtering ────────────────────────────────────────────

/// An investor who withdraws all of their principal keeps an `InvestorIndex` slot but must
/// not surface in `get_funding_records` with a zero contribution.
#[test]
fn funding_records_exclude_investor_who_unfunded_to_zero() {
    let env = Env::default();
    env.mock_all_auths();
    let fx = init_escrow(&env);
    let quitter = Address::generate(&env);
    let stayer = Address::generate(&env);
    seed_investors(&fx, &[quitter.clone(), stayer.clone()], &[1_000, 1_000]);
    let client = &fx.client;

    // Precondition: the investor is listed while funded.
    assert!(client
        .get_funding_records(&0u32, &50u32)
        .iter()
        .any(|r| r.investor == quitter));

    // Fully withdraw the investor's principal.
    client.unfund(&quitter, &1_000i128);
    assert_eq!(client.get_contribution(&quitter), 0i128);

    let page = client.get_funding_records(&0u32, &50u32);

    // The zero-balance slot is gone from the view...
    assert!(!page.iter().any(|r| r.investor == quitter));
    // ...and no returned record carries a zero contribution.
    for record in page.iter() {
        assert!(
            record.contribution > 0,
            "zero-contribution record surfaced for {:?}",
            record.investor
        );
    }
    // The other investor is still reported.
    assert!(page.iter().any(|r| r.investor == stayer));
}

/// A partially unfunded investor remains visible with their reduced (but positive) balance.
#[test]
fn funding_records_keep_partially_unfunded_investor_with_reduced_balance() {
    let env = Env::default();
    env.mock_all_auths();
    let fx = init_escrow(&env);
    let inv = Address::generate(&env);
    seed_investors(&fx, &[inv.clone()], &[1_000]);
    let client = &fx.client;

    client.unfund(&inv, &400i128);

    let page = client.get_funding_records(&0u32, &50u32);
    let found = page
        .iter()
        .find(|r| r.investor == inv)
        .expect("partially unfunded investor still listed");
    assert_eq!(found.contribution, 600i128);
}

// ── #124: DistributedPrincipal advances on release ───────────────────────────

/// After a partial `release`, `DistributedPrincipal` equals the released amount — the
/// accounting `sweep_terminal_dust` uses for its liability floor.
#[test]
fn partial_release_advances_distributed_principal() {
    let env = Env::default();
    env.mock_all_auths();
    let fx = init_escrow(&env);
    seed_investors(&fx, &[Address::generate(&env)], &[TARGET]);
    let client = &fx.client;

    assert_eq!(client.get_distributed_principal(), 0i128);

    let partial = TARGET / 4;
    client.release(&partial);

    assert_eq!(client.get_distributed_principal(), partial);
}

/// A final `release` of the full remaining obligation leaves `DistributedPrincipal` equal to
/// the full `funded_amount`.
#[test]
fn final_release_advances_distributed_principal_to_full_amount() {
    let env = Env::default();
    env.mock_all_auths();
    let fx = init_escrow(&env);
    seed_investors(&fx, &[Address::generate(&env)], &[TARGET]);
    let client = &fx.client;

    client.release(&TARGET);

    assert_eq!(client.get_escrow().status, 3u32);
    assert_eq!(client.get_distributed_principal(), TARGET);
}

/// Successive partial releases accumulate: the total equals the sum of the released amounts.
#[test]
fn successive_partial_releases_accumulate_distributed_principal() {
    let env = Env::default();
    env.mock_all_auths();
    let fx = init_escrow(&env);
    seed_investors(&fx, &[Address::generate(&env)], &[TARGET]);
    let client = &fx.client;

    let p1 = TARGET / 2;
    client.release(&p1);
    assert_eq!(client.get_distributed_principal(), p1);

    let p2 = TARGET - p1;
    client.release(&p2);

    assert_eq!(client.get_distributed_principal(), TARGET);
    // Both paths agree: reconciliation reports no outstanding liability.
    let view = client.get_reconciliation();
    assert_eq!(view.outstanding_liability, 0i128);
}

// ── #125: raise_min_contribution_floor upper bound ───────────────────────────

/// Initialise an escrow with an open floor and return the client plus its addresses.
fn open_floor_escrow(env: &Env, initial_floor: i128) -> (crate::StarfundEscrowClient<'_>, Address) {
    env.mock_all_auths();
    let client = crate::tests::deploy(env);
    let admin = Address::generate(env);
    let sme = Address::generate(env);
    client.init(
        &admin,
        &String::from_str(env, "FLOOR_RAISE"),
        &sme,
        &TARGET,
        &800i64,
        &0u64,
        &Address::generate(env),
        &None,
        &Address::generate(env),
        &None,
        &Some(initial_floor),
        &None,
        &None,
        &None,
        &None,
        &None,
        &None,
        &None,
        &None);
    (client, admin)
}

/// Raising the floor to a value at or below the funding target succeeds and persists.
#[test]
fn raise_min_contribution_floor_within_target_succeeds() {
    let env = Env::default();
    let (client, _admin) = open_floor_escrow(&env, 1_000i128);

    let new_floor = client.raise_min_contribution_floor(&2_000i128);
    assert_eq!(new_floor, 2_000i128);
    assert_eq!(client.get_min_contribution_floor(), 2_000i128);
}

/// The acceptance criterion for #125: a floor above `funding_target` reverts with
/// `MinContributionExceedsAmount`, matching the bound enforced at `init`.
#[test]
fn raise_min_contribution_floor_above_funding_target_reverts() {
    let env = Env::default();
    let (client, _admin) = open_floor_escrow(&env, 1_000i128);

    assert_contract_error(
        client.try_raise_min_contribution_floor(&(TARGET + 1)),
        EscrowError::MinContributionExceedsAmount,
    );

    // State is unchanged after the rejected call.
    assert_eq!(client.get_min_contribution_floor(), 1_000i128);
}

/// A floor exactly equal to the funding target is the boundary and must be accepted —
/// a single investor can still reach the target exactly.
#[test]
fn raise_min_contribution_floor_equal_to_target_accepted() {
    let env = Env::default();
    let (client, _admin) = open_floor_escrow(&env, 1_000i128);

    assert_eq!(client.raise_min_contribution_floor(&TARGET), TARGET);
    assert_eq!(client.get_min_contribution_floor(), TARGET);
}

/// A non-raising value is rejected — the setter only raises.
#[test]
fn raise_min_contribution_floor_requires_strict_increase() {
    let env = Env::default();
    let (client, _admin) = open_floor_escrow(&env, 5_000i128);

    assert_contract_error(
        client.try_raise_min_contribution_floor(&5_000i128),
        EscrowError::NewFloorNotHigher,
    );
    assert_contract_error(
        client.try_raise_min_contribution_floor(&1_000i128),
        EscrowError::NewFloorNotHigher,
    );
}

/// A non-positive floor is rejected.
#[test]
fn raise_min_contribution_floor_rejects_non_positive() {
    let env = Env::default();
    let (client, _admin) = open_floor_escrow(&env, 1_000i128);

    assert_contract_error(
        client.try_raise_min_contribution_floor(&0i128),
        EscrowError::NewFloorNotPositive,
    );
}

/// The setter is admin-only: it cannot run once the escrow has left the open state.
#[test]
fn raise_min_contribution_floor_rejects_when_not_open() {
    let env = Env::default();
    env.mock_all_auths();
    let fx = init_escrow(&env);
    seed_investors(&fx, &[Address::generate(&env)], &[TARGET]);
    let client = &fx.client;
    assert_eq!(client.get_escrow().status, 1u32);

    assert_contract_error(
        client.try_raise_min_contribution_floor(&1_000i128),
        EscrowError::FloorRaiseNotOpen,
    );
}
