//! Property and regression tests for the paged allowlist index (`allowlist_index_remove`).
//!
//! Removing an entry pulls the first entry of each following page forward. These tests drive
//! generated sequences of valid add/remove operations over an index spanning several pages and
//! compare membership, enumeration and count with an in-memory reference set after every
//! mutation. Enumeration is compared as a set; no stable ordering is assumed.

use super::*;
use crate::{ALLOWLIST_PAGE_SIZE, MAX_INVESTOR_ALLOWLIST_BATCH};
use proptest::prelude::*;
use soroban_sdk::{testutils::Address as _, Address, Env, String, Vec as SorobanVec};
use std::collections::BTreeSet;

/// Candidate addresses: enough to fill three pages (50 per page) and have spares to refill with.
const POOL: usize = 130;
/// Allowlist capacity used by these tests; the prefill reaches it exactly.
const LIMIT: u32 = 120;

struct Harness<'a> {
    client: StarfundEscrowClient<'a>,
    pool: std::vec::Vec<Address>,
    members: BTreeSet<usize>,
    nonce: u32,
}

/// An initialised escrow whose allowlist already holds pool entries `0..LIMIT`, so the index
/// spans three pages (50 + 50 + 20) and is at capacity.
fn harness(env: &Env) -> Harness<'_> {
    let (client, admin, sme) = setup(env);
    let (_, treasury) = free_addresses(env);
    let token = env.register_stellar_asset_contract_v2(Address::generate(env)).address();
    client.init(
        &admin,
        &String::from_str(env, "ALLOWCMP01"),
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
    client.set_allowlist_limit(&LIMIT);

    let pool: std::vec::Vec<Address> = (0..POOL).map(|_| Address::generate(env)).collect();
    let mut nonce = 0u32;
    let mut next = 0usize;
    while next < LIMIT as usize {
        let take = (LIMIT as usize - next).min(MAX_INVESTOR_ALLOWLIST_BATCH as usize);
        let mut batch: SorobanVec<Address> = SorobanVec::new(env);
        for addr in &pool[next..next + take] {
            batch.push_back(addr.clone());
        }
        client.set_investors_allowlisted(&batch, &true, &nonce);
        nonce += 1;
        next += take;
    }

    let mut h = Harness { client, pool, members: (0..LIMIT as usize).collect(), nonce };
    h.check();
    h
}

impl Harness<'_> {
    /// Applies one add (`allowed`) or remove of pool entry `ix`, then checks all invariants.
    /// An add that would exceed the capacity is not a valid operation and is skipped.
    fn apply(&mut self, ix: usize, allowed: bool) {
        if allowed && !self.members.contains(&ix) && self.members.len() as u32 >= LIMIT {
            return;
        }
        self.client.set_investor_allowlisted(&self.pool[ix], &allowed, &self.nonce);
        self.nonce += 1;
        if allowed {
            self.members.insert(ix);
        } else {
            self.members.remove(&ix);
        }
        assert_eq!(
            self.client.is_investor_allowlisted(&self.pool[ix]),
            allowed,
            "membership flag of pool entry {ix} after set_investor_allowlisted({allowed})"
        );
        self.check();
    }

    /// Count, capacity and enumeration must all agree with the reference set.
    fn check(&self) {
        let count = self.client.get_allowlisted_investors_count();
        assert_eq!(count as usize, self.members.len(), "count differs from the reference set");
        assert!(count <= self.client.get_allowlist_limit(), "count exceeds the allowlist limit");

        let mut listed: std::vec::Vec<usize> = std::vec::Vec::new();
        let mut start = 0u32;
        loop {
            let page = self.client.get_allowlisted_investors(&start, &ALLOWLIST_PAGE_SIZE);
            if page.len() == 0 {
                break;
            }
            for addr in page.iter() {
                let ix = self
                    .pool
                    .iter()
                    .position(|candidate| *candidate == addr)
                    .expect("enumeration returned an address that was never allowlisted");
                listed.push(ix);
            }
            start += page.len();
            assert!(start <= count, "enumeration returned more entries than the count");
        }

        assert_eq!(listed.len(), self.members.len(), "enumeration length differs from the count");
        let listed_set: BTreeSet<usize> = listed.iter().copied().collect();
        assert_eq!(listed_set.len(), listed.len(), "enumeration contains a duplicate address");
        assert_eq!(listed_set, self.members, "enumeration differs from the reference set");
    }
}

#[test]
fn removing_first_middle_and_last_entries_compacts_across_pages() {
    let env = Env::default();
    let mut h = harness(&env);

    // First entry of page 0, a middle entry of page 1, and the final entry of the last page.
    for ix in [0usize, 75, 119] {
        h.apply(ix, false);
    }
    assert_eq!(h.client.get_allowlisted_investors_count(), LIMIT - 3);

    // Refill the freed capacity, with the removed addresses and with spares.
    h.apply(0, true);
    h.apply(75, true);
    h.apply(120, true);
    assert_eq!(h.client.get_allowlisted_investors_count(), LIMIT);
}

#[test]
fn removing_entries_on_page_boundaries_keeps_neighbours() {
    let env = Env::default();
    let mut h = harness(&env);

    // Last entry of page 0 and first of page 1, then the same pair at the next boundary.
    for ix in [49usize, 50, 99, 100] {
        h.apply(ix, false);
    }
    for ix in [50usize, 49, 100, 99] {
        h.apply(ix, true);
    }
}

#[test]
fn draining_the_final_page_and_refilling_it() {
    let env = Env::default();
    let mut h = harness(&env);

    // Empty the last page (pool entries 100..120), one entry at a time.
    for ix in 100..LIMIT as usize {
        h.apply(ix, false);
    }
    assert_eq!(h.client.get_allowlisted_investors_count(), 100);

    // Refill it with spares that were never allowlisted.
    for ix in LIMIT as usize..POOL {
        h.apply(ix, true);
    }
    for ix in 100..110usize {
        h.apply(ix, true);
    }
    assert_eq!(h.client.get_allowlisted_investors_count(), LIMIT);
}

#[test]
fn adding_at_capacity_is_rejected_and_leaves_the_index_unchanged() {
    let env = Env::default();
    let h = harness(&env);

    assert_contract_error(
        h.client.try_set_investor_allowlisted(&h.pool[POOL - 1], &true, &h.nonce),
        EscrowError::AllowlistCapacityReached,
    );
    h.check();
}

/// Fixed seeds so a failure reproduces exactly; each drives a pseudo-random operation sequence.
fn run_seeded(seed: u64, steps: usize) {
    let env = Env::default();
    let mut h = harness(&env);
    let mut x = seed;
    for _ in 0..steps {
        // xorshift64*: deterministic and independent of any external RNG.
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        let r = x.wrapping_mul(0x2545_F491_4F6C_DD1D);
        let ix = (r % POOL as u64) as usize;
        let allowed = (r >> 40) & 1 == 1;
        h.apply(ix, allowed);
    }
}

#[test]
fn fixed_seed_regression_sequences() {
    for seed in [1u64, 0x9E37_79B9_7F4A_7C15, 0xDEAD_BEEF_CAFE_F00D] {
        run_seeded(seed, 40);
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(12))]

    #[test]
    fn generated_add_remove_sequences_match_the_reference_set(
        ops in proptest::collection::vec((any::<bool>(), 0usize..POOL), 1..=40),
    ) {
        let env = Env::default();
        let mut h = harness(&env);
        for (allowed, ix) in ops {
            h.apply(ix, allowed);
        }
    }
}
