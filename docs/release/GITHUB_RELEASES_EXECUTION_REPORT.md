# GitHub Releases Execution Report — Phase 3 Task 2

**Date:** 2026-11-21 · **Status:** PREPARED — NO RELEASE OR TAG PUBLISHED (NO-GO active)

## Repository
https://github.com/Aliahasan399/SwiftFetch (public). Release infrastructure is prepared
for future use; **v0.1.0 is not published**.

## Workflow inspection

| Workflow | Trigger | Permissions | Purpose |
|---|---|---|---|
| `.github/workflows/windows.yml` | PR, push to main, manual | `contents: read` | verify + build + smoke (existing, pinned SHAs) |
| `.github/workflows/release-draft.yml` | `workflow_dispatch` only | `contents: read` | manual, fail-closed release validation |
| `.github/workflows/pages.yml` | push main (docs/site) + manual | `contents: read`; job `pages: write, id-token: write` | publish website |

`release-draft.yml` validation: requires owner to type `APPROVED` and `SIGNED`, checks
`GO_NO_GO_REPORT.md` verdict is GO, checks candidate manifest
`ready_for_public_release=true`, verifies `package.json`/`Cargo.toml` version equals
input, and verifies every `SHA256SUMS` line. It then **prints** a draft command and
does not run `gh`. No auto-publish on tag/push. Actions pinned to commit SHAs where
already verified; Pages workflow uses version tags (not SHA-pinned) — documented as
a hardening follow-up.

## Safe checks performed
- Workflow files parsed as YAML-safe structure; no secrets referenced in workflow files.
- Asset inventory verified: NSIS 3,704,490 B `sha256 2abe7ec1…4544`; MSI 6,283,264 B
  `sha256 58df1a14…8f32`; source ZIP 636,145 B `sha256 c050aff2…f20b`; candidate
  manifest 627 artifacts, `SHA256SUMS` 627 lines, 0 hash mismatches.
- Third-party notices present at repo root (`THIRD_PARTY_NOTICES.md`) and in artifacts.
- User checksum instructions in `RELEASE_PUBLICATION_READINESS.md`.

## What was NOT done
- No `v0.1.0` tag created.
- No GitHub Release created or draft published.
- No installer uploaded (all are unsigned).
- No binary labeled signed or tested beyond existing evidence.
- No misleading release notes produced.

## Status table
| Item | Status |
|---|---|
| Release workflow syntax/permissions | COMPLETED |
| Manual-trigger fail-closed design | COMPLETED |
| Asset + checksum verification | COMPLETED |
| Release publication | BLOCKED — NO-GO, unsigned installers |
| Tag `v0.1.0` | BLOCKED — same |
