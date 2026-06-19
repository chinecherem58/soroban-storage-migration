# Soroban Storage Migration
**High Complexity — 200 Points**

Demonstrates and fixes the most common early Soroban mistake: storing
**per-user data** inside `instance()` storage.

---

## The Problem

```rust
// ❌ INCORRECT — all users share one 64 KB ledger entry
env.storage().instance().set(&user, &amount);
```

| Issue | Consequence |
|---|---|
| Everything lives in **one** 64 KB ledger entry | Contract breaks after a few thousand users |
| TTL is shared with the contract itself | Cannot extend one user's lifetime cheaply |
| Every call loads the full map | Gas scales with total number of users |

---

## The Fix

```rust
// ✅ CORRECT — each user owns a separate LedgerEntry
env.storage()
    .persistent()
    .set(&DataKey::Balance(user.clone()), &amount);

// Extend only when close to expiry — cheap and per-user
env.storage()
    .persistent()
    .extend_ttl(&DataKey::Balance(user), 100_000, 500_000);
```

---

## Storage Decision Guide

| What you're storing | Correct type |
|---|---|
| Admin, fee config, contract metadata | `instance()` |
| Per-user balances, allowances, positions | `persistent()` |
| Temporary nonces, session data | `temporary()` |

---

## Project Structure

```
.
├── Cargo.toml                    ← Workspace root (shared deps, release profile)
├── Makefile                      ← build / test / lint / ci shortcuts
├── rustfmt.toml                  ← enforced formatting rules
├── clippy.toml                   ← Clippy thresholds
├── .cargo/config.toml            ← default wasm32 build target
├── .gitignore
└── contracts/
    └── token-migration/
        ├── Cargo.toml
        └── src/
            ├── lib.rs            ← contract + error types + migration logic
            └── test.rs           ← 17 integration tests
```

---

## Quick Start

### Prerequisites

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Add WASM target
rustup target add wasm32-unknown-unknown

# Install Stellar CLI (for deployment)
cargo install --locked stellar-cli --features opt
```

### Commands

```bash
make build          # compile to optimised WASM
make test           # run all tests (native, fast)
make test-optimized # tests with release optimisations
make lint           # Clippy — deny all warnings
make fmt            # auto-format
make ci             # full gate: fmt-check → lint → build → test
```

---

## Contract API

| Function | Auth | Description |
|---|---|---|
| `initialize(admin)` | — | One-time setup; stores admin in `instance()` |
| `set_balance(user, amount)` | user | Write user balance ≥ 0 into `persistent()` |
| `get_balance(user)` | — | Read balance; returns `0` if absent |
| `migrate_balance(admin, user, amount)` | admin | Admin-driven migration from old `instance()` slot |
| `cleanup_instance(admin, stale_key)` | admin | Remove old key from `instance()` to reduce rent |
| `transfer_admin(admin, new_admin)` | admin | Hand off admin rights |
| `get_admin()` | — | Read current admin address |
| `extend_instance_ttl()` | — | Bump contract instance TTL (permissionless) |

---

## Error Codes

| Code | Name | When |
|---|---|---|
| 1 | `AlreadyInitialised` | `initialize` called twice |
| 2 | `NotInitialised` | Called before `initialize` |
| 3 | `Unauthorised` | Non-admin calls admin function |
| 4 | `NegativeAmount` | `set_balance` receives amount < 0 |

---

## Deploy to Testnet

```bash
stellar contract deploy \
    --wasm target/wasm32-unknown-unknown/release/token_migration.wasm \
    --source <YOUR_SECRET_KEY> \
    --network testnet
```

---

*Built for the Stellar / Soroban ecosystem — production-ready.*
