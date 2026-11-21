# SwiftFetch — Phase 4 packaging/update evidence

**Status:** compiled from direct repo/artifact reads. No signing keys or endpoints are fabricated.

## Reproducible build inputs
- Dependencies locked: `src-tauri/Cargo.lock`, `package-lock.json`; build uses `--locked`
  (`tools/verify.ps1`, `tools/build-windows.ps1`).
- Build commands: `npm run build` (Vite/TS) + `cargo build --release --locked` +
  `./tools/build-windows.ps1` (Tauri NSIS + MSI). Last release build produced
  `SwiftFetch_0.1.0_x64-setup.exe` and `SwiftFetch_0.1.0_x64_en-US.msi`.
- Candidate packaging: `tools/package-candidate.ps1` (manifest with `sha256`,
  `authenticode` per file, `SHA256SUMS`); `tools/package-source.ps1` (allowlisted source zip).

## Signature / updater status (externally blocked)
| Check | Result | Evidence |
|---|---|---|
| Executables signed | **No** — all `NotSigned` | candidate `candidate-manifest.json` `authenticode: NotSigned` |
| `ready_for_public_release` | `false` | candidate manifest |
| Updater requireSignedVersion | `true` with empty `pubkey: ""` | `src-tauri/tauri.conf.json:31` |
| Update env vars | all unset | `SWIFTFETCH_UPDATE_ENDPOINT/_ARCHIVE/_PUBLIC_KEY`, `TAURI_SIGNING_PRIVATE_KEY(_PATH)` |
| Staging gate rejects unsigned | Yes | `tools/stage-release.ps1` throws on unsigned artifact |
| Signature verification code + tests | Yes | `updates.rs` minisign verify; `updates.rs:539,546,553` tests (valid/tampered/version replay); `updater_hardening_tests.rs` (3) |
| Real signed install/upgrade/rollback | **Not performed** | no artifact; clean-machine blocker |
| Clean Windows VM | **Unavailable** | Sandbox not installed; see `CLEAN_MACHINE_ACCEPTANCE.md` |

## Update failure-path coverage (unit level)
- Bad signature / tampered bytes / version replay: `updates.rs` signature tests pass.
- Corrupted or escaping rollback metadata rejected: `updates.rs` test passes.
- Interrupted attempt phase reconciliation: `updater_hardening_tests.rs` (3) pass.
- Offline/unconfigured channel returns `not_configured` without network: smoke group passes.
- Not covered by real run: publisher-signed upgrade, automatic rollback, unavailable real endpoint,
  real interrupted update against a live signed endpoint.

## Required owner inputs
See `docs/release/OWNER_CONFIG_GUIDE.md`: trusted Authenticode certificate/key (local only),
HTTPS update endpoint/archive/public key/signing key, publisher identity/contact, license, terms.

## Completion criterion
Phase 4 is complete for internal work when the final signed installers and a real signed
update/rollback run exist with evidence, **or** an explicit external blocker is recorded.
Current state: **externally blocked (NO-GO)**, not simulated.
