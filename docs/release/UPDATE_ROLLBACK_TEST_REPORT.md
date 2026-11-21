# Update & Rollback Test Report — Master Prompt Phase 4/10

**Date:** 2026-11-21 · **Status:** UNIT-LEVEL COMPLETED; REAL E2E BLOCKED

## Unit/integration evidence (executed)
From canonical verification `artifacts/verification/report-20261121-230706.json`:
- `updates.rs:539` endpoint safety (HTTP, credentials, fragments rejected).
- `updates.rs:547` corrupted/escaping rollback metadata rejected.
- `updates.rs:553` public test vector: valid signature verifies bytes `test`;
  tampered bytes rejected; version replay rejected; cached rollback verifies.
- `updater_hardening_tests.rs` (3): interrupted phases reconciled; completed
  phases untouched; invalid attempt files ignored without panic.

## What these prove
Signature verification logic, version binding, fail-closed metadata handling,
staged-attempt crash reconciliation, and rollback cache validation at code level.

## What is NOT proven (real E2E — BLOCKED)
1. New-version detection against a live HTTPS release manifest.
2. Real download, interrupted download, retry, corrupted artifact rejection.
3. Real installation of a signed update and application restart.
4. Real rollback after failed health check, with user-data preservation.
5. Wrong architecture / downgrade handling on a real release.

Reason: no production/owner updater keys, no approved HTTPS endpoint, no signed
release, no clean Windows VM. Per owner boundaries, dev keys may be used for local
testing but no production key/endpoint may be created without approval.

## Required future test procedure (when owner approves)
1. Generate dev keys locally; build with all three `SWIFTFETCH_*` channel vars.
2. Host `latest.json` + signed installer on a local HTTPS staging endpoint.
3. Install v0.1.0 in a clean VM with test data; trigger update to a test v0.1.1.
4. Verify detection, download, signature, install, restart, data preservation.
5. Force failed health check; verify signed rollback and usable application.
6. Record hashes, signatures, timings, and screenshots as evidence.

## Status
| Item | Status |
|---|---|
| Unit signature/rollback tests | VERIFIED (this run) |
| Real update E2E | BLOCKED_EXTERNAL |
| Real rollback E2E | BLOCKED_EXTERNAL |
