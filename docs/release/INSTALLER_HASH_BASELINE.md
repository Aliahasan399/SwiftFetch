# SwiftFetch — installer hash baseline policy

**Date:** 2026-11-22 · **Status:** policy established; no signing or publication performed
**NO-GO preserved.** Unsigned hashes below are historical evidence only.

## 1. Historical unsigned baseline (build evidence only)

These hashes are retained exactly as recorded in `FINAL_BUILD_EVIDENCE.md` and re-measured by
`tools/build-installers.ps1 -InspectOnly` (`artifacts/installer-builds/build-20261122-093740.json`).
They identify the unsigned build that exists on disk. They are **not** distribution hashes.

| Artifact | Bytes | SHA-256 | Authenticode | Role |
|---|---:|---|---|---|
| `SwiftFetch_0.1.0_x64_en-US.msi` | 6,283,264 | `58df1a1410cd43fb7480e08bb708b5e9f71d7db17c224c1f481907a92d588f32` | `NotSigned` | historical unsigned build evidence |
| `SwiftFetch_0.1.0_x64-setup.exe` | 3,704,490 | `2abe7ec122240b3bac4f8f22d61e5a5540da843ca2367faacaf8c9a6505e4454` | `NotSigned` | historical unsigned build evidence (pre-cleanup-fix) |

Reuse of these values for any release, manifest, or `SHA256SUMS` is prohibited.

## 2. Signed artifact record (separate — currently pending)

| Artifact | SHA-256 | Signer | Certificate validity | Status |
|---|---|---|---|---|
| MSI, 0.1.0 signed | `PENDING_SIGNING` | `PENDING_SIGNING` | `PENDING_SIGNING` | not produced |
| NSIS, 0.1.0 signed | `PENDING_SIGNING` | `PENDING_SIGNING` | `PENDING_SIGNING` | not produced |

No signed artifact, certificate, or signed hash exists in this repository. Do not fill these
rows from the historical unsigned values.

## 3. Policy rules

1. An unsigned hash is historical build evidence. It must be labelled `NotSigned` and must never
   be presented as the final artifact hash.
2. Distribution hashes are generated **after** signing, from the exact files that will be
   distributed, into a separate directory and record.
3. `tools/generate-distribution-hashes.ps1` refuses to run unless every artifact has
   Authenticode `Status = Valid`. It also supports an optional expected-publisher check.
4. The generated `SHA256SUMS` and `distribution-hashes.json` are the only sources for release
   manifests and verification instructions.
5. If an artifact is rebuilt, resigned, or patched (including the NSIS cleanup fix), all signed
   hashes are invalidated and must be regenerated.
6. Signing, publishing, release creation and updater activation remain out of scope and require
   separate owner authorization.

## 4. Procedure after signing (not executed here)

```powershell
# 1. Confirm the signer is the expected trusted publisher.
Get-AuthenticodeSignature <msi> | Format-List Status, SignerCertificate

# 2. Generate hashes only from valid signed artifacts.
./tools/generate-distribution-hashes.ps1 -ExpectedPublisher '<owner-approved publisher>'

# 3. Verify the generated SHA256SUMS against the artifact files.
Get-Content artifacts/distribution-hashes/hashes-*/SHA256SUMS
```

The script writes `artifacts/distribution-hashes/hashes-<timestamp>/SHA256SUMS` and
`distribution-hashes.json`; it does not publish or create a release.

## 5. Evidence of the refusal rule

`./tools/generate-distribution-hashes.ps1` was executed against the current unsigned artifacts:
it refused with a non-zero exit and the message that unsigned hashes are historical evidence
only. No `SHA256SUMS` was generated. This confirms the unsigned baseline cannot accidentally
become the distribution record.

## 6. Relationship to other records

- Historical build context: `docs/release/FINAL_BUILD_EVIDENCE.md`
- NSIS post-build fix: `docs/release/NSIS_CLEANUP_FIX.md` (the historical NSIS artifact predates
  the fix; a rebuilt installer is required before any signed artifact is produced)
- Verification: `tools/verify-installer.ps1` accepts expected hashes for pinning either record;
  callers must choose the record appropriate to the artifact they are testing.
