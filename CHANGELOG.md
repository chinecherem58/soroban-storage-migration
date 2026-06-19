# Changelog

All notable changes to this project are documented here.

Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).  
Versioning follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [Unreleased]

### Fixed
- **Soroban storage migration (High Complexity)** — per-user token balances
  were incorrectly stored in `instance()` storage, which is capped at a single
  64 KB ledger entry shared across all users and ties every user's TTL to the
  contract instance. All per-user state has been migrated to `persistent()`
  storage so each user owns a separate `LedgerEntry` with an independently
  managed TTL.

### Added
- `#[contracterror]` typed error enum (`AlreadyInitialised`, `NotInitialised`,
  `Unauthorised`, `NegativeAmount`) replacing bare `panic!` strings — errors
  are now machine-readable `u32` codes for off-chain tooling and cross-contract
  callers.
- `migrate_balance` — admin-only function to move a balance from the old
  `instance()` slot into the correct `persistent()` slot without data loss.
- `cleanup_instance` — admin-only function to remove stale keys from
  `instance()` storage after migration, reducing state rent.
- `transfer_admin` — rotate the admin key; the old admin immediately loses
  all privileged access.
- `get_admin` — read the current admin address.
- Lazy TTL extension on every `persistent()` write: only extends when the
  remaining lifetime drops below `100_000` ledgers, targeting `500_000`
  ledgers (~28.9 days). Avoids paying for extension on every single call.
- GitHub Actions CI workflow (`.github/workflows/ci.yml`): format check →
  Clippy → WASM build → native tests → optimised-profile tests.
- 17 integration tests covering all success paths, every error code, auth
  boundaries, TTL idempotency, and the not-initialised guard.

### Changed
- `set_balance` now rejects negative amounts (`NegativeAmount` error). Use
  `migrate_balance` (admin-only) to carry over any negative values from old
  storage.
- Storage keys are now a strongly-typed `#[contracttype] enum DataKey` instead
  of bare `Address` values, preventing accidental key collisions.
