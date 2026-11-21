# GitHub Release Readiness — Master Prompt Phase 2

**Date:** 2026-11-21 · **Status:** PREPARED — publication BLOCKED (NO-GO active)

## Requirements checklist

| Requirement | Status | Evidence |
|---|---|---|
| Windows build environment | COMPLETED | `.github/workflows/windows.yml` `windows-2022` |
| Reproducible build inputs | PARTIAL | `--locked` builds, pinned actions; Windows on CI not SHA-identical verified |
| Dependency installation | COMPLETED | `npm ci`, locked cargo |
| Rust compilation / frontend / installers | COMPLETED | FINAL_BUILD_EVIDENCE.md; 3 artifacts built |
| Automated verification | COMPLETED | `tools/verify.ps1` 7/7 (report-20261121-184958.json) |
| Artifact collection + SHA-256 | COMPLETED | candidate manifest 627 files, SHA256SUMS 627 lines, 0 mismatches |
| Manifest generation | COMPLETED | candidate-manifest.json with per-artifact hash/signature status |
| Signature verification when available | PREPARED | `release.yml` attestation gate checks signed=true + digests; no real cert yet |
| Release gate enforcement | COMPLETED | fail-closed gates in `release.yml` / `release-draft.yml` |
| Workflow security | COMPLETED | least privilege, no secrets, manual-only triggers (see workflow audit) |
| MSI/NSIS/exe version + arch | COMPLETED | 0.1.0 / x64 consistent |
| MSI upgrade/uninstall behavior | BLOCKED | clean-machine VM unavailable |
| Production release publication | BLOCKED | NO-GO, unsigned installers |

## Asset checklist (real artifacts)
- `SwiftFetch_0.1.0_x64-setup.exe` (NSIS) — 3,704,490 B — `2abe7ec1…4544` — NotSigned
- `SwiftFetch_0.1.0_x64_en-US.msi` — 6,283,264 B — `58df1a14…8f32` — NotSigned
- `swiftfetch.exe` — 10,375,680 B — `eebb5788…fceb` — NotSigned
- `SwiftFetch-0.1.0-source.zip` — 636,145 B — `c050aff2…f20b`
- `SHA256SUMS`, `THIRD_PARTY_NOTICES.md`, candidate manifest

## User verification (SHA-256 establishes integrity only)
```powershell
Get-FileHash .\SwiftFetch_0.1.0_x64-setup.exe -Algorithm SHA256
# Compare with SHA256SUMS from a trusted source. This does NOT prove publisher identity.
```

## Final readiness
Release infrastructure is **prepared and verified**; production publication is **blocked**
by signing, updater, clean-machine, license, and legal gates. NO-GO and
`ready_for_public_release: false` remain unchanged.
