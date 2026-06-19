//! Integration tests for the TokenMigration contract.
//!
//! Run:   cargo test
//! Opt:   cargo test --profile test-optimized

#![cfg(test)]

use soroban_sdk::{
    testutils::Address as _,
    Address, Env,
};

use crate::{Error, TokenMigration, TokenMigrationClient};

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Deploy a fresh contract, initialise it, return (env, client, admin).
fn setup() -> (Env, TokenMigrationClient<'static>, Address) {
    let env = Env::default();
    env.mock_all_auths();

    let id     = env.register_contract(None, TokenMigration);
    let client = TokenMigrationClient::new(&env, &id);
    let admin  = Address::generate(&env);
    client.initialize(&admin);

    (env, client, admin)
}

/// Extract the contract Error from a failed try_* call.
///
/// soroban-sdk wraps contracterror variants inside `Ok(Err(Error::Contract(e)))`.
/// This helper unwraps that chain and asserts the expected error code.
macro_rules! assert_err {
    ($result:expr, $expected:expr) => {{
        let sdk_err = $result
            .expect_err("expected an error but call succeeded")
            .expect("expected Ok(Err(..)) but got Err(..)");
        assert_eq!(sdk_err, $expected);
    }};
}

// ── Initialisation ────────────────────────────────────────────────────────────

#[test]
fn test_initialize_stores_admin() {
    let (_env, client, admin) = setup();
    assert_eq!(client.get_admin(), admin);
}

#[test]
fn test_double_initialize_returns_already_initialised() {
    let (env, client, _) = setup();
    assert_err!(
        client.try_initialize(&Address::generate(&env)),
        Error::AlreadyInitialised
    );
}

// ── set_balance / get_balance ─────────────────────────────────────────────────

#[test]
fn test_balance_defaults_to_zero() {
    let (env, client, _) = setup();
    assert_eq!(client.get_balance(&Address::generate(&env)), 0);
}

#[test]
fn test_set_and_get_balance() {
    let (env, client, _) = setup();
    let user = Address::generate(&env);
    client.set_balance(&user, &1_000_000);
    assert_eq!(client.get_balance(&user), 1_000_000);
}

#[test]
fn test_overwrite_balance() {
    let (env, client, _) = setup();
    let user = Address::generate(&env);
    client.set_balance(&user, &500);
    client.set_balance(&user, &750);
    assert_eq!(client.get_balance(&user), 750);
}

#[test]
fn test_multiple_users_are_independent() {
    let (env, client, _) = setup();
    let alice = Address::generate(&env);
    let bob   = Address::generate(&env);
    client.set_balance(&alice, &100);
    client.set_balance(&bob,   &200);
    assert_eq!(client.get_balance(&alice), 100);
    assert_eq!(client.get_balance(&bob),   200);
}

#[test]
fn test_zero_balance_is_valid() {
    let (env, client, _) = setup();
    let user = Address::generate(&env);
    client.set_balance(&user, &0);
    assert_eq!(client.get_balance(&user), 0);
}

#[test]
fn test_negative_amount_rejected() {
    let (env, client, _) = setup();
    let user = Address::generate(&env);
    assert_err!(
        client.try_set_balance(&user, &-1),
        Error::NegativeAmount
    );
}

// ── Admin: migrate_balance ────────────────────────────────────────────────────

#[test]
fn test_migrate_balance_writes_persistent_slot() {
    let (env, client, admin) = setup();
    let user = Address::generate(&env);
    client.migrate_balance(&admin, &user, &9_999);
    assert_eq!(client.get_balance(&user), 9_999);
}

/// Admin migration must reproduce negative values faithfully (old storage may
/// have held them).
#[test]
fn test_migrate_balance_allows_negative_for_admin() {
    let (env, client, admin) = setup();
    let user = Address::generate(&env);
    client.migrate_balance(&admin, &user, &-100);
    assert_eq!(client.get_balance(&user), -100);
}

#[test]
fn test_migrate_balance_non_admin_rejected() {
    let (env, client, _) = setup();
    let attacker = Address::generate(&env);
    let user     = Address::generate(&env);
    assert_err!(
        client.try_migrate_balance(&attacker, &user, &500),
        Error::Unauthorised
    );
}

// ── Admin: cleanup_instance ───────────────────────────────────────────────────

#[test]
fn test_cleanup_instance_is_noop_for_absent_key() {
    let (env, client, admin) = setup();
    // Key does not exist — must be a silent no-op, not a panic.
    client.cleanup_instance(&admin, &Address::generate(&env));
}

#[test]
fn test_cleanup_instance_non_admin_rejected() {
    let (env, client, _) = setup();
    let attacker = Address::generate(&env);
    assert_err!(
        client.try_cleanup_instance(&attacker, &Address::generate(&env)),
        Error::Unauthorised
    );
}

// ── Admin: transfer_admin ─────────────────────────────────────────────────────

#[test]
fn test_transfer_admin_updates_stored_admin() {
    let (env, client, admin) = setup();
    let new_admin = Address::generate(&env);
    client.transfer_admin(&admin, &new_admin);
    assert_eq!(client.get_admin(), new_admin);
}

#[test]
fn test_transfer_admin_non_admin_rejected() {
    let (env, client, _) = setup();
    let attacker  = Address::generate(&env);
    let new_admin = Address::generate(&env);
    assert_err!(
        client.try_transfer_admin(&attacker, &new_admin),
        Error::Unauthorised
    );
}

/// Old admin must lose all privileged access after a transfer.
#[test]
fn test_old_admin_loses_rights_after_transfer() {
    let (env, client, old_admin) = setup();
    let new_admin = Address::generate(&env);
    client.transfer_admin(&old_admin, &new_admin);

    let user = Address::generate(&env);
    assert_err!(
        client.try_migrate_balance(&old_admin, &user, &100),
        Error::Unauthorised
    );
}

// ── TTL extension ─────────────────────────────────────────────────────────────

#[test]
fn test_extend_instance_ttl_is_idempotent() {
    let (_, client, _) = setup();
    client.extend_instance_ttl();
    client.extend_instance_ttl(); // second call must not panic
}

// ── Not-initialised guard ─────────────────────────────────────────────────────

#[test]
fn test_get_admin_before_init_returns_not_initialised() {
    let env = Env::default();
    env.mock_all_auths();
    let id     = env.register_contract(None, TokenMigration);
    let client = TokenMigrationClient::new(&env, &id);

    assert_err!(client.try_get_admin(), Error::NotInitialised);
}
