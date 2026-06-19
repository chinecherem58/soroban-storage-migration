# Wave Program Contribution Plan

## Project: Soroban Storage Migration

The Wave Program pairs contributors with scoped, sprint-ready issues.
Below are the types of work I would post as a maintainer, covering the full
lifecycle of a Soroban smart contract fix.

---

## 1. Bug Fixes

**Issue: Migrate per-user balances from `instance()` to `persistent()` storage**
_High Complexity — 200 pts_

Early Soroban contracts stored per-user data in `instance()` storage, which
is a single 64 KB ledger entry shared by all users. Once full, the contract
panics permanently. The fix moves every per-user balance to `persistent()`
so each user owns a separate `LedgerEntry` with an independent TTL.

- Replace `instance().set(&user, &amount)` with `persistent().set(&DataKey::Balance(user), &amount)`
- Add `extend_ttl(100_000, 500_000)` on every persistent write
- Add admin `migrate_balance` for existing on-chain state
- Add admin `cleanup_instance` to remove stale instance keys

**Issue: Replace `panic!` strings with typed `#[contracterror]` enum**
_Medium — 100 pts_

Bare `panic!("string")` errors are undecodable by callers and off-chain tools.
Replace all panic strings with a `#[contracterror]` enum so every failure
returns a machine-readable `u32` code.

**Issue: Guard `set_balance` against negative amounts**
_Low — 50 pts_

Token balances must be ≥ 0. Add a `NegativeAmount` error check at the top
of `set_balance` and a matching test.

---

## 2. New Features

**Issue: Add `transfer_admin` for admin key rotation**
_Medium — 100 pts_

A permanently locked admin key is a security risk. Add `transfer_admin` so
the current admin can hand off rights. The old admin must immediately lose
all privileged access after the call.

**Issue: Add `get_admin` read function**
_Low — 50 pts_

Off-chain tooling and other contracts need to read the current admin without
making a privileged call. Add a simple `get_admin` view function that returns
`Error::NotInitialised` if called before `initialize`.

**Issue: Add permissionless `extend_instance_ttl`**
_Low — 50 pts_

Allow any actor (including automated keepers) to bump the contract instance
TTL so the contract stays live even without admin intervention.

---

## 3. Documentation

**Issue: Document the three Soroban storage types in `lib.rs`**
_Low — 50 pts_

Add a module-level doc table explaining when to use `instance()`,
`persistent()`, and `temporary()`. Add inline comments on every storage call.
This directly prevents the class of bug this project fixes.

**Issue: Write a storage migration guide in `README.md`**
_Low — 50 pts_

The README should show the wrong pattern, the correct pattern, and step-by-step
migration instructions for existing deployments. Include a storage decision table.

**Issue: Add `CHANGELOG.md`**
_Low — 50 pts_

Document every change under `[Unreleased]` following Keep a Changelog format
so maintainers and users can track what changed without reading raw git history.

---

## 4. Testing

**Issue: Add tests for all error codes and auth boundaries**
_Medium — 100 pts_

Every `#[contracterror]` variant needs a dedicated test using `try_*` client
methods with typed enum assertions. Every admin function needs a "non-admin
rejected" test. Add an `assert_err!` macro to reduce boilerplate.

**Issue: Add GitHub Actions CI workflow**
_Low — 50 pts_

Create `.github/workflows/ci.yml` running: format check → Clippy →
WASM build → native tests → optimised-profile tests. Cache Cargo artefacts.

---

## 5. Code Quality

**Issue: Add `rustfmt.toml` and `clippy.toml`**
_Low — 50 pts_

Lock formatting and lint rules so all contributors produce consistent code.
Wire `cargo fmt --check` and `cargo clippy -- -D warnings` into the CI gate.

---

## Sprint Summary

| Category | Issues | Points |
|---|---|---|
| Bug fixes | 3 | 350 |
| New features | 3 | 200 |
| Documentation | 3 | 150 |
| Testing | 2 | 150 |
| Code quality | 1 | 50 |
| **Total** | **12** | **900** |

Each issue is scoped to one sprint, ordered by dependency: bugs first,
then features, then documentation and tests that validate the full system.
