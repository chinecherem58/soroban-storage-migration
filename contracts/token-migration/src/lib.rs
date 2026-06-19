//! # Token Migration Contract
//!
//! ## Background — Why This Migration Exists
//!
//! Early Soroban contracts stored **per-user balances** inside `instance()`
//! storage.  That was incorrect for two reasons:
//!
//! 1. **Size cap** — all instance storage lives in a single 64 KB ledger entry.
//!    A handful of users can exhaust it, making the contract permanently broken.
//!
//! 2. **TTL semantics** — instance storage shares the contract's own TTL.
//!    You cannot extend one user's entry independently; every renewal pays for
//!    the entire instance map.
//!
//! ## Correct storage assignment
//!
//! | Data                           | Storage type   |
//! |--------------------------------|----------------|
//! | Contract metadata (admin, fee) | `instance()`   |
//! | Per-user / per-token state     | `persistent()` |
//! | Short-lived ephemeral data     | `temporary()`  |
//!
//! Each `persistent()` entry owns a separate `LedgerEntry` with an
//! independently manageable TTL.

#![no_std]

use soroban_sdk::{
    contract, contractimpl, contracterror, contracttype,
    panic_with_error,
    Address, Env,
};

// ── Error codes ───────────────────────────────────────────────────────────────

/// Typed error enum.
///
/// `#[contracterror]` (not `#[contracttype]`) is the correct macro here.
/// It maps each variant to a `u32` error code that callers and off-chain
/// tooling can decode — unlike a bare `panic!` string.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
pub enum Error {
    /// `initialize` was called more than once.
    AlreadyInitialised = 1,
    /// Called before `initialize`.
    NotInitialised = 2,
    /// Caller is not the stored admin.
    Unauthorised = 3,
    /// Amount must be ≥ 0 for a token balance.
    NegativeAmount = 4,
}

// ── Storage keys ─────────────────────────────────────────────────────────────

/// Strongly-typed storage keys prevent accidental key collisions.
#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    /// Per-user token balance — stored in `persistent()`.
    Balance(Address),
    /// Contract-wide admin address — stored in `instance()`.
    Admin,
}

// ── TTL constants ─────────────────────────────────────────────────────────────

/// Only extend TTL when remaining lifetime drops below this ledger count.
/// 1 ledger ≈ 5 s  →  100_000 ledgers ≈ 5.8 days.
const TTL_THRESHOLD: u32 = 100_000;

/// Extend to this many ledgers of remaining lifetime.
/// 500_000 ledgers ≈ 28.9 days — reasonable window for active users.
const TTL_TARGET: u32 = 500_000;

// ── Internal helpers ──────────────────────────────────────────────────────────

/// Read the stored admin, panicking with `NotInitialised` if absent.
fn read_admin(env: &Env) -> Address {
    env.storage()
        .instance()
        .get(&DataKey::Admin)
        .unwrap_or_else(|| panic_with_error!(env, Error::NotInitialised))
}

/// Assert `caller == stored admin`.
fn assert_admin(env: &Env, caller: &Address) {
    if read_admin(env) != *caller {
        panic_with_error!(env, Error::Unauthorised);
    }
}

// ── Contract ──────────────────────────────────────────────────────────────────

#[contract]
pub struct TokenMigration;

#[contractimpl]
impl TokenMigration {
    // ── Initialisation ───────────────────────────────────────────────────────

    /// One-time setup: record the admin in `instance()` storage.
    ///
    /// Reverts with `AlreadyInitialised` on a second call.
    pub fn initialize(env: Env, admin: Address) {
        if env.storage().instance().has(&DataKey::Admin) {
            panic_with_error!(&env, Error::AlreadyInitialised);
        }

        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().extend_ttl(TTL_THRESHOLD, TTL_TARGET);
    }

    // ── User-facing balance operations ───────────────────────────────────────

    /// Write a per-user balance into `persistent()` storage.
    ///
    /// Only the user themselves may call this.
    /// `amount` must be ≥ 0 (use `migrate_balance` for admin overrides).
    pub fn set_balance(env: Env, user: Address, amount: i128) {
        user.require_auth();

        if amount < 0 {
            panic_with_error!(&env, Error::NegativeAmount);
        }

        env.storage()
            .persistent()
            .set(&DataKey::Balance(user.clone()), &amount);

        // Lazy TTL extension: only pays when entry is close to expiry.
        env.storage()
            .persistent()
            .extend_ttl(&DataKey::Balance(user), TTL_THRESHOLD, TTL_TARGET);
    }

    /// Read a per-user balance.  Returns `0` when no entry exists yet.
    pub fn get_balance(env: Env, user: Address) -> i128 {
        env.storage()
            .persistent()
            .get(&DataKey::Balance(user))
            .unwrap_or(0)
    }

    // ── Admin migration helpers ───────────────────────────────────────────────

    /// Move a balance that was previously stuck in `instance()` into the
    /// correct `persistent()` slot.
    ///
    /// Admin-only. After migrating all users call `cleanup_instance` to shrink
    /// the instance entry and lower rent costs.
    ///
    /// Accepts any `i128` value (including negatives) to faithfully reproduce
    /// whatever was in the old storage — the admin is responsible for
    /// supplying correct values.
    pub fn migrate_balance(env: Env, admin: Address, user: Address, amount: i128) {
        admin.require_auth();
        assert_admin(&env, &admin);

        env.storage()
            .persistent()
            .set(&DataKey::Balance(user.clone()), &amount);

        env.storage()
            .persistent()
            .extend_ttl(&DataKey::Balance(user), TTL_THRESHOLD, TTL_TARGET);
    }

    /// Remove a stale key from `instance()` storage after migration is complete.
    ///
    /// Shrinks the instance ledger entry → lower state rent.
    pub fn cleanup_instance(env: Env, admin: Address, stale_key: Address) {
        admin.require_auth();
        assert_admin(&env, &admin);

        env.storage().instance().remove(&stale_key);
    }

    // ── Admin utilities ───────────────────────────────────────────────────────

    /// Transfer admin rights to a new address.
    pub fn transfer_admin(env: Env, admin: Address, new_admin: Address) {
        admin.require_auth();
        assert_admin(&env, &admin);

        env.storage().instance().set(&DataKey::Admin, &new_admin);
        env.storage().instance().extend_ttl(TTL_THRESHOLD, TTL_TARGET);
    }

    /// Read the current admin address.
    pub fn get_admin(env: Env) -> Address {
        read_admin(&env)
    }

    /// Bump the contract instance TTL (permissionless — anyone can pay).
    pub fn extend_instance_ttl(env: Env) {
        env.storage()
            .instance()
            .extend_ttl(TTL_THRESHOLD, TTL_TARGET);
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────
mod test;
