# SwiftFetch — Auto-Update Execution Report

**Lane:** `autoupdate-execution-reporter`
**Report date:** 2026-11-21
**Write scope:** this file only (`docs/release/AUTO_UPDATE_EXECUTION_REPORT.md`). No source, config,
workflow, key material, endpoint, release asset, or commit was created or modified by this lane.

## 0. Honesty statement (read first)

This is a **read-only inspection-and-reporting record**. It documents what exists in source and what
is prepared on paper. It does **not** claim that automatic updates work.

- **No signing keys were generated.**
- **No update endpoint was activated.**
- **No signed manifest (`latest.json`) or `.sig` artifact was produced or published.**
- **No end-to-end (E2E) update test succeeded — none was run.**
- **Automatic updates are NOT operational.** The shipped build is silently `not_configured`.

Every statement below is either (a) a verified read-only source/config finding, (b) a prepared
procedure, or (c) an explicitly blocked item. Where tests are discussed, "unit-level" means an
assertion that exists in source with a public test vector; it does **not** mean this lane executed it.

## 1. Status legend

| Label | Meaning in this report |
| --- | --- |
| **COMPLETED** | Verified by read-only inspection of the named file/line; no runtime claim implied. |
| **PREPARED** | Procedure/plan documented; required inputs are owner-supplied and not yet executed. |
| **BLOCKED** | Cannot proceed until keys, an activated endpoint, signed artifacts, and a clean VM exist. |
| **REQUIRES OWNER APPROVAL** | A deliberate owner decision/action is a prerequisite; this lane will not perform it. |

## 2. Verified updater dependencies, plugin, and configuration — **COMPLETED**

Read-only findings from `src-tauri/tauri.conf.json`, `src-tauri/src/updates.rs`, and
`tools/build-windows.ps1`. Presence in source is not evidence of a successful run.

| Item | Verified state | Source |
| --- | --- | --- |
| Updater plugin dependency | `tauri_plugin_updater::{Update, UpdaterExt}` imported and used | `src-tauri/src/updates.rs:11`, builder at `:95`, `.verify(...)` at `:150, :217, :297, :337, :397, :437` |
| `plugins.updater.pubkey` | `""` — **empty string** | `src-tauri/tauri.conf.json:31` |
| `plugins.updater.requireSignedVersion` | `true` | `src-tauri/tauri.conf.json:31` |
| `plugins.updater.windows.installMode` | `"passive"` | `src-tauri/tauri.conf.json:31` |
| App version bound into signatures | `0.1.0` | `src-tauri/tauri.conf.json:4` |
| `SWIFTFETCH_UPDATE_ENDPOINT` / `_ARCHIVE` / `_PUBLIC_KEY` | **unset** in this workspace (read via `option_env!`) | `updates.rs:55-57` |
| `TAURI_SIGNING_PRIVATE_KEY` / `_PATH` | **unset** in this workspace | `tools/build-windows.ps1:23` |
| Effective runtime behavior | `publisher()` returns `None`; `UpdateManager::new` sets `configured = false`, phase `"not_configured"` | `updates.rs:54-61`, `:67-79` |
| `check_for_updates` with no publisher | returns snapshot, **no network request** | `updates.rs:258-275` (`:260`) |
| `install_update` / `rollback_update` with no publisher | return `UPDATE_NOT_CONFIGURED` | `updates.rs:280`, `:328` |
| Build guard | all-three-or-none for `SWIFTFETCH_*`; signing key required only when channel configured; adds `createUpdaterArtifacts = true`; builds `--ci --locked` | `tools/build-windows.ps1:19-25`, `:29-31` |
| `*.sig` artifacts in `src-tauri/target/release/bundle` | none present | corroborated in `UPDATER_KEY_SETUP.md` §1 |

**Conclusion:** the updater code and plugin are present and correctly wired, but with an empty
`pubkey` and unset env values the configured channel is off. The four commands
`get_update_state {}`, `check_for_updates {}`, `install_update {}`, `rollback_update {}` all return
the `CommandResult<UpdateState>` envelope (`updates.rs:17-28, :252-345`).

## 3. GitHub Releases hosting plan — **PREPARED** (activation **REQUIRES OWNER APPROVAL**)

Real repository supplied to this lane: **https://github.com/Aliahasan399/SwiftFetch**

A public GitHub Releases channel can host, at zero cost, the signed installer, its `.sig`, and a
`latest.json` manifest as release assets under an immutable version tag (`v0.1.0`). This section is
a **plan only**. No release, tag, asset, or endpoint was created here; selecting and activating the
host remains an owner decision (`UPDATER_KEY_SETUP.md` §4, §6).

Prepared endpoint values (filled from the real repo URL, **not activated**):

| Variable | Planned value |
| --- | --- |
| `SWIFTFETCH_UPDATE_ENDPOINT` | `https://github.com/Aliahasan399/SwiftFetch/releases/latest/download/latest.json` (or pinned: `https://github.com/Aliahasan399/SwiftFetch/releases/download/v0.1.0/latest.json`) |
| `SWIFTFETCH_UPDATE_ARCHIVE` | `https://github.com/Aliahasan399/SwiftFetch/releases/download/v{{version}}/latest.json` |
| `SWIFTFETCH_UPDATE_PUBLIC_KEY` | owner's **public** key string only (see §4) |

Endpoint constraints enforced in code: HTTPS only, host present, no embedded credentials, no
fragment (`safe_endpoint`, `updates.rs:44-52`); at most 5 redirects and no downgrade to non-HTTPS
for both manifest and package fetches (`updates.rs:107-112`, `:131-135`). The archive endpoint must
keep the installed version's signed installer available for signed rollback
(`UPDATER_KEY_SETUP.md` §6; `updates.rs:288-298`).

## 4. Secure key generation instructions — **PREPARED** (generation **REQUIRES OWNER APPROVAL**)

No key pair was generated by this lane. Owner-run procedure is already documented in
`docs/release/UPDATER_KEY_SETUP.md` §3–§4 and §7:

- Generate a minisign-format pair locally with the Tauri v2 signer:
  `npm run tauri -- signer generate -- -w "$env:USERPROFILE\.tauri\swiftfetch.key"`, or
  `minisign -G -p …\.tauri\swiftfetch.pub -s …\.tauri\swiftfetch.key`.
- Keep the **private** key and passphrase outside the repository, outside CI, and out of chat,
  source control, artifacts, and logs. `.gitignore` already ignores `*.key`, `*.pfx`, `*.p12`, `.env*`.
- Only the **public** key string may be embedded via `SWIFTFETCH_UPDATE_PUBLIC_KEY` /
  `tauri.conf.json` `pubkey`. The private key is exposed to build tools only through
  `TAURI_SIGNING_PRIVATE_KEY_PATH` (preferred) or `TAURI_SIGNING_PRIVATE_KEY`.
- Update signing is **not** Authenticode code signing; it establishes no Windows publisher identity.

## 5. Manifest format — **PREPARED** (format verified from source; no manifest produced)

`tools/release-channel.ps1:24-34` writes `artifacts/update-channel/<version>/latest.json`:

```json
{
  "version": "0.1.0",
  "notes": "<owner notes>",
  "pub_date": "<UTC ISO-8601>",
  "platforms": {
    "windows-x86_64": {
      "url": "<https installer URL>",
      "signature": "<base64 .sig content>"
    }
  }
}
```

Signature-binding and safety rules (verified):

- The `.sig` is read from `<installer>.sig`, decoded from base64, and must contain **exactly one**
  `trusted comment:` line whose `version:<v>` equals the `tauri.conf.json` version (`0.1.0`);
  otherwise the script throws "Use the current Tauri signer with --app-version…"
  (`release-channel.ps1:14-22`).
- The version directory is immutable — the script refuses to overwrite an existing one
  (`release-channel.ps1:30-32`).
- Manifest ≤ 128 KiB (`updates.rs:120`); package ≤ 128 MiB (`updates.rs:15, :139, :145`);
  single `windows-x86_64` platform entry (`release-channel.ps1:28`).

No `latest.json` and no `.sig` were produced by this lane.

## 6. Signature-verification tests that exist — **COMPLETED** (inspection only; **not executed**)

Three unit tests exist in `src-tauri/src/updates.rs` (`#[cfg(test)] mod tests`, `:535-583`). **This
lane did not run them.** They are source-level assertions using a public test vector; they are not a
real signed-installer verification.

| Test (line) | What it asserts |
| --- | --- |
| `signed_update_and_rollback_bytes_reject_tampering_and_version_replay` (`:554-582`) | Uses a **public** minisign vector from `tauri-plugin-updater 2.14.1` (MIT/Apache-2.0); no private key shipped (`:555-558`). Sets mock updater config `"requireSignedVersion": true` (`:561`). Valid bytes verify (`:571-572`); **tampered** bytes fail (`:573`); **version replay** — manifest `3.0.0` vs signature-bound `2.0.0` — fails (`:574-575`); a cached rollback written via `write_cache`/`journal`/`cached` still verifies (`:576-581`). |
| `update_endpoints_reject_credentials_and_insecure_transport` (`:539-545`) | HTTPS accepted; `http://`, `file://`, credentialed, and.fragment URLs rejected. |
| `corrupted_or_escaping_rollback_metadata_cannot_be_used` (`:547-551`) | A `rollback.json` whose package path escapes (`../elsewhere.exe`) is rejected by `cached()`. |

No private key is embedded in tests — only a public key/signature (`updates.rs:555-558`;
`UPDATER_KEY_SETUP.md` §1). The production counterpart of the test flag is
`requireSignedVersion: true` in `tauri.conf.json:31`, independently enforced at manifest creation
by `release-channel.ps1:18-22`.

## 7. Honest per-mechanism status — unit-level vs. real end-to-end

All real E2E checks are **BLOCKED** until the owner generates keys, approves/activates an HTTPS
endpoint, produces real signed artifacts, and provides a disposable Windows VM. **No E2E test was
run and none succeeded.**

| Mechanism | Unit-level evidence in source | Real end-to-end status |
| --- | --- | --- |
| **Detection** (manifest fetch/parse) | Endpoint URL safety asserted (`:539-545`); no live manifest fetch | **BLOCKED** — no active endpoint |
| **Download** (≤128 MiB bound, byte progress) | Not covered by any unit test | **BLOCKED** — no signed package / endpoint |
| **Signature & signed-version verification** | Public-vector tests: valid bytes, tampering, version replay, cached rollback (`:554-582`) | **BLOCKED** — no real signed installer |
| **Installation** (NSIS `/P` `/D`, MSI `INSTALLDIR`, helper readiness handshake) | Not covered by unit tests | **BLOCKED** — no signed artifacts or clean VM |
| **Failure handling / health check / automatic rollback** | Not covered end-to-end | **BLOCKED** — requires a real install and disposable VM |
| **Explicit rollback command** | Cached-package path validation at metadata level (`:547-551`, `:576-581`) | **BLOCKED** — requires cached signed bytes from a real release |
| **Startup reconciliation of interrupted attempts** | Not covered | **BLOCKED** — requires an interrupted real install |

Source does contain the production mechanisms — minisign verification (`:150, :297, :337, :397,
:437`), rollback staging/verification before upgrade (`:288-298`), explicit rollback (`:325-345`),
health check + 180 s watchdog + automatic rollback (`:465-533`), startup reconciliation (`:193-208`),
`INSTALLDIR`/NSIS `/D` preservation (`:434-463`), and a 1 GiB staging budget (`:157-182`) — but
their live operation is **unverified**. `docs/UPDATES.md:56-60` also states a real signed-installer
upgrade and recovery must be exercised in a disposable Windows environment.

## 8. Hard status summary

| Deliverable | Status |
| --- | --- |
| Inspect implementation, deps, plugin, config, pubkey, env (`not_configured`) | **COMPLETED** |
| Preserve architecture — no source/config changes by this lane | **COMPLETED** |
| GitHub Releases hosting plan (real repo URL filled) | **PREPARED** — activation **REQUIRES OWNER APPROVAL** |
| Secure keygen instructions referenced (`UPDATER_KEY_SETUP.md`) | **PREPARED** — key generation **REQUIRES OWNER APPROVAL** |
| Manifest format documented/verified | **PREPARED** |
| Signature-verification unit tests identified (public vector, tampering, version replay) | **COMPLETED** (inspection only; not executed) |
| Signed manifests / `.sig` artifacts | **BLOCKED** until owner generates keys |
| Detection / download / install / failure-handling / rollback — real E2E | **BLOCKED** until keys + approved endpoint + signed artifacts + clean VM |
| Publishing / endpoint activation | **REQUIRES OWNER APPROVAL** |
| No keys generated, no endpoint activated, no signed manifest, no E2E success | **COMPLETED** (this lane made none of these) |

**Bottom line:** the auto-update implementation exists in source and its supporting procedures and
tests are documented, but **no signing keys were generated, no endpoint was activated, no signed
manifest exists, no end-to-end test succeeded, and automatic updates are NOT operational.** The
shipped configuration remains `not_configured`. Use the labels **COMPLETED / PREPARED / BLOCKED /
REQUIRES OWNER APPROVAL** above rather than claiming a working update channel.
