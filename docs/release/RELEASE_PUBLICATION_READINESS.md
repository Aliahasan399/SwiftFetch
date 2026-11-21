# Release Publication Readiness — Phase 2 Task 2

**Date:** 2026-11-21 · **Version:** 0.1.0 · **Status:** PREPARED — DO NOT PUBLISH (NO-GO active)

## Asset checklist (real, verified files)

| Asset | Source path | Bytes | SHA-256 |
|---|---|---:|---|
| Windows NSIS installer | `src-tauri/target/release/bundle/nsis/SwiftFetch_0.1.0_x64-setup.exe` | 3,704,490 | `2abe7ec122240b3bac4f8f22d61e5a5540da843ca2367faacaf8c9a6505e4454` |
| Windows MSI installer | `src-tauri/target/release/bundle/msi/SwiftFetch_0.1.0_x64_en-US.msi` | 6,283,264 | `58df1a1410cd43fb7480e08bb708b5e9f71d7db17c224c1f481907a92d588f32` |
| Desktop executable | `src-tauri/target/release/swiftfetch.exe` | 10,375,680 | `eebb578832936b29dc73e97e65178a942d026c719bed1045379180bec596fceb` |
| Source ZIP | `artifacts/source/0.1.0-07a66ecd41d54ccdbb8fedbd6e4162a1/SwiftFetch-0.1.0-source.zip` | 636,145 | `c050aff2e41c49b9fd6cc44485ee8ea976dc5fe197f0ddcf122747d3abf2f20b` |
| SHA256SUMS + manifest | `artifacts/SwiftFetch-0.1.0-candidate-c02155f813444b23b6b023f4a212bfe1/` | 627 assets | 0 hash mismatches |
| Third-party notices | `THIRD_PARTY_NOTICES.md` + generated notices in artifacts | — | ship in release |

All three executables are **NotSigned**. Do not label them signed. MSI is built but
clean-machine functional acceptance is incomplete — do not claim it passed testing.

## Release workflow (`.github/workflows/release-draft.yml`)
- **Manual trigger only** (`workflow_dispatch`) — no push/tag auto-publication.
- Requires owner to type `APPROVED` and `SIGNED`, validates version consistency
  (`package.json`, `Cargo.toml`, input), checks `GO_NO_GO_REPORT.md` says GO, checks
  `candidate-manifest.json` `ready_for_public_release=true`, verifies every line of
  `SHA256SUMS`, and then only **prints** the draft command — it never runs `gh`.
- Least privilege: `permissions: contents: read` (no write token).
- Actions pinned to full commit SHAs (v4.3.0 checkout, verified).

## Tag convention
Use `v0.1.0` only after GO. Note from `GITHUB_RELEASES_SETUP.md`: update URL
templates must include the `v` prefix (`v{{version}}`) or GitHub Releases lookups 404.

## User SHA-256 verification instructions
```powershell
Get-FileHash .\SwiftFetch_0.1.0_x64-setup.exe -Algorithm SHA256
# Compare the hash to the entry in SHA256SUMS.
```
```cmd
certutil -hashfile SwiftFetch_0.1.0_x64-setup.exe SHA256
```

## Publication status
| Item | Status |
|---|---|
| Asset checklist | COMPLETED |
| Manual-trigger, fail-closed workflow | COMPLETED |
| Checksum verification instructions | COMPLETED |
| GitHub Release created | **NOT DONE — NO-GO active** |
| Tag `v0.1.0` | **NOT CREATED** |
| Any artifact uploaded | **NOT DONE** |

Do not publish an unapproved candidate. The previous NO-GO verdict remains active.
