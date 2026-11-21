# GitHub Release Workflow Audit — Master Prompt Phase 2

**Date:** 2026-11-21 · **Status:** COMPLETED (audit), no unauthorized publication

## Workflow inventory (all validated with PyYAML 6.0.3)

| Workflow | Triggers | Top permissions | Job permissions | Purpose |
|---|---|---|---|---|
| `windows.yml` | `pull_request`, `push` (main), `workflow_dispatch` | `contents: read` | inherited read | verify + build + smoke |
| `pages.yml` | `push` (main, docs/site), `workflow_dispatch` | `contents: read` | `pages: write, id-token: write, contents: read` | publish website |
| `release-draft.yml` | **`workflow_dispatch` only** | `contents: read` | `contents: read` | manual fail-closed draft validation |
| `release.yml` | **`workflow_dispatch` only** | `contents: read` | gate/verify `contents: read`; draft job `contents: write` only when `create_draft=true` | comprehensive fail-closed draft creation |

All 4 parse as valid YAML. `release.yml` originally had 5 unterminated `description:` quotes
(from a stalled lane); parent fixed them (backup `.bak-20261121-225452`) and re-validated.

## Security review
- **No auto-publish triggers**: neither release workflow listens to `push`, `tags`, `release`,
  `schedule`, or `pull_request`. Manual dispatch only.
- **Least privilege**: top-level `contents: read`; `release.yml` elevates to `contents: write`
  only on the single draft-creation job behind `create_draft == 'true'`.
- **No secrets read**: workflows reference no `secrets.*`; only built-in `GITHUB_TOKEN`.
- **Fail-closed gates** (`release.yml`): exact tag `v0.1.0`; version consistency package.json vs
  tauri.conf.json; `source_ref` required; explicit owner confirmation string; `no_go_cleared=true`;
  signed artifact attestation URL/path + expected SHA-256 must be supplied and match; attestation
  JSON must state `release_approved=true`, `signed=true`, and list installer digests that match
  `SHA256SUMS`. Any missing input fails the job.
- **Checksum verification** (`release.yml` verify-assets job): re-hashes assets against `SHA256SUMS`.
- **Action pinning**: `windows.yml` uses full SHA pins; `release.yml`/`release-draft.yml` use
  major-version pins with comments; `pages.yml` uses version tags (documented follow-up to pin SHAs).
- **Injection risk**: workflow inputs are passed via `env:` (not interpolated into shell
  directly) in the checked steps — acceptable. Artifact path inputs are used in shell paths;
  owner-controlled manual dispatch only, low risk.
- **PR execution**: release workflows cannot run from PRs (no trigger).

## Build outputs verified (from FINAL_BUILD_EVIDENCE.md)
- exe `eebb5788…fceb`, MSI `58df1a14…8f32`, NSIS `2abe7ec1…4544` — all `NotSigned`.
- Version `0.1.0` consistent across package.json, Cargo.toml, tauri.conf.json, bundle filenames.
- Candidate `c02155f8…`: 627 artifacts, `SHA256SUMS` 627 lines, 0 hash mismatches.
- Checksums establish integrity only against a trusted manifest, **not** publisher authenticity.

## Publication status
No official release, tag, or upload performed. NO-GO remains active.

## Status
| Item | Status |
|---|---|
| YAML validity (4 workflows) | COMPLETED |
| Workflow security review | COMPLETED |
| Malformed release.yml fix | COMPLETED |
| Production publication | BLOCKED — NO-GO, unsigned |
