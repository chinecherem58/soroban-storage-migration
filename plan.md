# Wave Program Contribution Plan
## Soroban Storage Migration — Sprint Issue Strategy

---

## Overview

This plan outlines the types of scoped issues I would post as a maintainer
participating in the Wave Program sprint cycles. Each issue is designed to be
focused, actionable, and completable within a sprint window while delivering
real value to the Soroban ecosystem.

The work centres on a common but critical problem: early Soroban contracts
misused `instance()` storage for per-user data. The issues below address that
problem from every angle — code fixes, safety guards, documentation, and tests.

---

## 1. Bug Fixes

These are the highest-priority issues. They address real broken behaviour that
affects contracts already deployed or in active development.

### Issue: Migrate per-user balances from `instance()` to `persistent()` storage

**Complexity:** High (200 pts)
**Label:** `bug`, `storage`, `high-complexity`

**Description:**
Contracts storing per-user balances in `instance()` storage will fail once
enough users interact with the contract. All instance data shares a single
64 KB ledger entry. Once that fills up, every subsequent write panics and the
contract becomes permanently unusable.

**Scope of fix:**
- Replace all `env.storage().instance().set(&user, &amount)` calls with
  `env.storage().persistent().set(&DataKey::Balance(user), &amount)`
- Add `extend_ttl(threshold, target)` after every persistent write
- Remove stale instance keys via an admin `cleanup_instance` function
- Add a one-time `migrate_balance` admin function for existing on-chain state

**Acceptance criteria:**
- No per-user data stored in `instance()` storage
- Every `persistent()` write is followed by a TTL extension
- All existing tests pass; new tests added for migration path

---

### Issue: Replace `panic!` strings with typed `#[contracterror]` enum

**Complexity:** Medium (100 pts)
**Label:** `bug`, `dx`, `error-handling`

**Description:**
Contracts using bare `panic!("some string")` produce errors that are opaque
to callers, cross-contract invocations, and off-chain tooling. The Soroban SDK
provides `#[contracterror]` specifically for this purpose.

**Scope of fix:**
- Define an `Error` enum with `#[contracterror]` and `u32` discriminants
- Replace every `panic!("...")` with `panic_with_error!(&env, Error::Variant)`
- Update all `try_*` test assertions to match against the typed enum

**Acceptance criteria:**
- Zero bare `panic!` strings in contract code
- All error paths return a decodable `u32` code
- Tests assert on `Error::VariantName`, not string matching

---

### Issue: Guard `set_balance` against negative token amounts

**Complexity:** Low (50 pts)
**Label:** `bug`, `validation`

**Description:**
Token balances should never be negative. Without an explicit guard, a user
can call `set_balance` with `-1` and corrupt the contract's accounting logic.

**Scope of fix:**
- Add `if amount < 0 { panic_with_error!(&env, Error::NegativeAmount); }`
  at the top of `set_balance`
- Add a `NegativeAmount` variant to the error enum
- Add a test that asserts the error fires on negative input

**Acceptance criteria:**
- `set_balance(-1)` returns `Error::NegativeAmount`
- `set_balance(0)` succeeds (zero is valid)
- Existing positive-amount tests unchanged

---

## 2. New Features

These issues add capabilities that are missing and commonly needed.

### Issue: Add `transfer_admin` for admin key rotation

**Complexity:** Medium (100 pts)
**Label:** `feature`, `security`

**Description:**
Contracts with a permanently locked admin key are a security risk. If the
admin wallet is compromised there is no recovery path. A `transfer_admin`
function lets the current admin hand off rights to a new address.

**Scope of fix:**
- Add `transfer_admin(env, admin, new_admin)` with `admin.require_auth()`
- Validate caller against stored admin before writing new value
- Extend instance TTL after the write
- Add tests: successful transfer, non-admin rejection, old admin loses rights

**Acceptance criteria:**
- Only current admin can call `transfer_admin`
- After transfer, old admin is rejected on all privileged functions
- New admin can immediately exercise all admin rights

---

### Issue: Add `get_admin` read function

**Complexity:** Low (50 pts)
**Label:** `feature`, `dx`

**Description:**
There is no way for off-chain tooling or other contracts to read the current
admin address without attempting a privileged call. A simple `get_admin`
view function closes this gap.

**Scope of fix:**
- Add `pub fn get_admin(env: Env) -> Address`
- Return stored admin or fire `Error::NotInitialised` if absent
- Add test for the happy path and the not-initialised guard

**Acceptance criteria:**
- Returns correct admin address after `initialize`
- Returns `Error::NotInitialised` before `initialize` is called

---

### Issue: Add permissionless `extend_instance_ttl`

**Complexity:** Low (50 pts)
**Label:** `feature`, `ttl`

**Description:**
The contract instance TTL can expire if no admin is available to renew it.
A permissionless TTL bump lets any actor (including automated keepers) keep
the contract live without needing admin auth.

**Scope of fix:**
- Add `pub fn extend_instance_ttl(env: Env)` with no auth requirement
- Use the shared `TTL_THRESHOLD` / `TTL_TARGET` constants
- Add a test confirming idempotent calls do not panic

**Acceptance criteria:**
- Function callable by any address without authentication
- Multiple sequential calls are safe (idempotent)

---

## 3. Documentation

Good documentation reduces the time maintainers spend answering repeated
questions and helps new contributors understand the codebase faster.

### Issue: Document the three Soroban storage types in `lib.rs`

**Complexity:** Low (50 pts)
**Label:** `documentation`

**Description:**
The distinction between `instance()`, `persistent()`, and `temporary()` is
the root cause of this entire class of bugs. Adding a clear module-level doc
comment explaining when to use each type prevents future regressions.

**Scope of fix:**
- Add a doc table to the crate root showing the three types and their use cases
- Explain TTL behaviour differences between instance and persistent
- Add inline comments on every storage call explaining the choice

**Acceptance criteria:**
- `cargo doc` generates readable API docs with no warnings
- Storage type table is present at the module level
- Every `storage()` call has a one-line comment justifying the type used

---

### Issue: Add `CHANGELOG.md` following Keep a Changelog format

**Complexity:** Low (50 pts)
**Label:** `documentation`, `release`

**Description:**
There is no changelog. Maintainers and users cannot see what changed between
versions without reading raw git history.

**Scope of fix:**
- Create `CHANGELOG.md` with an `[Unreleased]` section
- Document every change made in the storage migration under Added / Fixed /
  Changed headings
- Link to Keep a Changelog and SemVer specs in the header

**Acceptance criteria:**
- `CHANGELOG.md` present at repo root
- All changes from this sprint documented under `[Unreleased]`
- Format matches Keep a Changelog v1.0.0

---

### Issue: Write a storage migration guide in `README.md`

**Complexity:** Low (50 pts)
**Label:** `documentation`

**Description:**
The README should explain the migration problem, the correct fix, and how to
run the migration against an existing deployed contract. New contributors
should be able to understand the entire context without reading the source code.

**Scope of fix:**
- Add a "Problem" section showing the incorrect `instance()` pattern
- Add a "Fix" section showing the correct `persistent()` pattern
- Add a "Migration Steps" section for existing deployments
- Add a storage decision table (what goes in which storage type)

**Acceptance criteria:**
- README explains the bug, the fix, and the migration path clearly
- Storage decision table is present
- Deploy and test commands are documented

---

## 4. Testing

Untested code is unshippable code. These issues improve confidence in every
code path.

### Issue: Add integration tests for all error codes

**Complexity:** Medium (100 pts)
**Label:** `testing`

**Description:**
The error enum has four variants but not every path has a dedicated test.
Missing coverage means silent regressions are possible.

**Scope of fix:**
- Add a test for every `Error` variant: `AlreadyInitialised`, `NotInitialised`,
  `Unauthorised`, `NegativeAmount`
- Use `try_*` client methods and assert on the typed enum value
- Add a helper macro `assert_err!` to reduce boilerplate across tests

**Acceptance criteria:**
- Every error variant is covered by at least one `#[test]`
- Tests use typed enum assertions, not string matching
- `cargo test` shows all tests passing

---

### Issue: Add auth boundary tests for all admin functions

**Complexity:** Medium (100 pts)
**Label:** `testing`, `security`

**Description:**
Admin functions must reject non-admin callers. Without explicit tests for
this, a refactor could accidentally remove the auth check without any test
failing.

**Scope of fix:**
- For each admin function (`migrate_balance`, `cleanup_instance`,
  `transfer_admin`), add a test where a random attacker address calls it
- Assert `Error::Unauthorised` is returned in every case
- Add a test confirming old admin loses rights after `transfer_admin`

**Acceptance criteria:**
- Every admin function has a corresponding "non-admin rejected" test
- `transfer_admin` has an "old admin loses rights" test
- All 17+ tests pass under `cargo test`

---

### Issue: Add GitHub Actions CI workflow

**Complexity:** Low (50 pts)
**Label:** `ci`, `testing`

**Description:**
Without CI, there is no automated gate on pull requests. A broken PR can be
merged without anyone noticing.

**Scope of fix:**
- Create `.github/workflows/ci.yml`
- Pipeline steps: format check → Clippy → WASM build → native tests →
  optimised-profile tests
- Cache Cargo registry and build artefacts for fast subsequent runs

**Acceptance criteria:**
- CI runs on every push and pull request
- All five pipeline steps must pass for a green check
- Build artefact caching is configured

---

## 5. Code Quality

These issues make the codebase easier to maintain long-term.

### Issue: Add `rustfmt.toml` and `clippy.toml` for enforced code style

**Complexity:** Low (50 pts)
**Label:** `dx`, `code-quality`

**Description:**
Without enforced formatting and lint rules, different contributors produce
inconsistent code. This creates noisy diffs and makes reviews harder.

**Scope of fix:**
- Add `rustfmt.toml` with edition, max line width, and import grouping rules
- Add `clippy.toml` with cognitive complexity and argument count thresholds
- Add `make lint` and `make fmt-check` targets to the Makefile
- Wire both into the CI pipeline

**Acceptance criteria:**
- `cargo fmt --check` passes on all source files
- `cargo clippy -- -D warnings` passes with zero warnings
- Both tools run as part of `make ci`

---

## Sprint Summary

| Type | Issues | Est. Points |
|---|---|---|
| Bug fixes | 3 | 350 |
| New features | 3 | 200 |
| Documentation | 3 | 150 |
| Testing | 3 | 250 |
| Code quality | 1 | 50 |
| **Total** | **13** | **1000** |

Each issue is scoped to be completable in one sprint. They are ordered by
dependency — bug fixes first, then features that build on the fixed base,
then documentation and tests that validate the whole system.
