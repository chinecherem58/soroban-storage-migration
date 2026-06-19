# ─────────────────────────────────────────────────────────────────────────────
# Makefile — Soroban Storage Migration
# ─────────────────────────────────────────────────────────────────────────────

CONTRACT  := token-migration
MANIFEST  := contracts/$(CONTRACT)/Cargo.toml
WASM_OUT  := target/wasm32-unknown-unknown/release/token_migration.wasm

.PHONY: all build test test-optimized fmt fmt-check lint clean ci

## Default: build then test
all: build test

## Compile to optimised WASM (wasm32-unknown-unknown)
build:
	cargo build --target wasm32-unknown-unknown --release \
	    --manifest-path $(MANIFEST)

## Run all tests on native target (fast iteration)
test:
	cargo test --manifest-path $(MANIFEST)

## Run tests with release optimisations (mirrors on-chain behaviour)
test-optimized:
	cargo test --profile test-optimized --manifest-path $(MANIFEST)

## Auto-format all source files
fmt:
	cargo fmt --all

## Check formatting without modifying files (for CI)
fmt-check:
	cargo fmt --all -- --check

## Lint with Clippy — deny all warnings
lint:
	cargo clippy --manifest-path $(MANIFEST) --all-targets -- -D warnings

## Full CI gate: format check → lint → build → test
ci: fmt-check lint build test

## Remove build artefacts
clean:
	cargo clean
