# SwiftFetch — ADDITIONAL TASK 4: auto-update implementation record

**Status of this document:** inspection-and-documentation record only. This lane made **no source or
config changes**, generated **no keys**, activated **no endpoint**, prepared **no signed artifacts**,
published **nothing**, created **no commits**, and ran **no tests**. Every statement below is a
read-only inspection finding or a procedure to be performed later by the release owner. **This
document does not claim that automatic updates work.**

## Files inspected (read-only)

- `src-tauri/src/updates.rs`
- `src-tauri/tauri.conf.json`
- `tools/build-windows.ps1`
- `tools/release-channel.ps1`
- `docs/UPDATES.md`
- `docs/release/UPDATER_KEY_SETUP.md`
- `docs/release/GITHUB_RELEASES_SETUP.md`

Line references below refer to the workspace state at the time of inspection.

## Status summary — steps 1–10

| Step | Topic | Status |
| --- | --- | --- |
| 1 | Inspect implementation (verification, rollback, health, reconciliation, INSTALLDIR, staging budget) | **COMPLETED** |
| 2 | Verify dependencies / config / pubkey / env (pubkey empty, env unset, `not_configured`) | **COMPLETED** |
| 3 | Preserve architecture (no code changes made) | **COMPLETED** |
| 4 | Secure keygen instructions (reference `UPDATER_KEY_SETUP.md`; keys outside repo/CI) | **PREPARED** |
| 5 | GitHub Releases as host (reference `GITHUB_RELEASES_SETUP.md`) | **PREPARED** |
| 6 | Signed manifests / signatures preparation | **BLOCKED** until owner generates keys |
| 7 | Metadata / version comparison verification (existing unit test; `requireSignedVersion`) | **COMPLETED** (inspection only; no tests executed) |
| 8 | Test detection / download / signature / install / failure / rollback | **BLOCKED** until keys + endpoint + clean VM |
| 9 | Document publishing future updates | **PREPARED** (publication itself **REQUIRES OWNER APPROVAL**) |
| 10 | State honestly: no end-to-end test succeeded, no endpoint activated | **COMPLETED** |

---

## Step 1 — Inspect the implementation — **COMPLETED**

Read-only inspection of `src-tauri/src/updates.rs` confirmed the following mechanisms are present in
source. Presence in source is not evidence of a successful end-to-end run.

| Mechanism | Where | What the code does |
| --- | --- | --- |
| Minisign signature verification | `updates.rs:95` (`pubkey(...)`), `:150`, `:217`, `:297`, `:337`, `:397`, `:437` (`update.verify(...)`) | The public key is supplied to `tauri-plugin-updater`; downloaded, cached, rollback and installer bytes are all verified before use. `updates.rs:435-437` states verified bytes are not treated as a download action. |
| `requireSignedVersion` | `tauri.conf.json:31` (`"requireSignedVersion": true`), also in the test mock at `updates.rs:561` | The signature must bind the release version; a signature for another version is rejected (see Step 7). |
| Rollback | `updates.rs:277-323` (`install_update`), `:325-345` (`rollback_update`), `:465-533` (`recovery_requested`) | The currently installed release is fetched from the immutable archive endpoint and verified **before** an upgrade is accepted (`:288-298`); a verified copy is cached. Explicit rollback re-verifies the cached package before launching the installer (`:331-340`). The recovery helper rolls back on install failure or failed health check (`:486-523`). |
| Health check | `updates.rs:377-385` (`mark_healthy`), `:359`, `:508-523` | `health.json` is written with `healthy: false` when staging; the updated app must report `healthy: true` for the matching attempt id and version. The helper polls for up to 180 seconds (≈3 minutes), then kills the process, installs the verified rollback, restores the bound metadata snapshot and writes `recovery-result.json`. |
| `reconcile_on_startup` | `updates.rs:193-208` | Scans `attempt-*.json`; attempts left in phase `installing` / `health` (or empty) are journaled as `interrupted` and reported as "A previous update was interrupted. The signed rollback remains available." |
| `INSTALLDIR` preservation | `updates.rs:434-463` | MSI path uses `msiexec /i … /passive /norestart INSTALLDIR={folder}` (`:441-449`); NSIS path uses `/P` plus `/D={folder}` (`:450-459`). The folder is derived from the current executable's parent and rejected if it contains `"`, CR or LF. |
| Staging budget | `updates.rs:157-182` (`STAGING_BUDGET = 1 GiB`, `staging_used`, `reserve_staging`) | One recursive budget covers signed packages, DB snapshots, helper copies and journal backups; reaching it returns "Update staging reached its 1 GiB limit…". Additional bounds: manifest ≤ 128 KiB (`:120`) and package ≤ 128 MiB (`:15`, `:139`, `:145`). |

Other observed structure: the four commands `get_update_state {}`, `check_for_updates {}`,
`install_update {}`, `rollback_update {}` all return the `CommandResult<UpdateState>` envelope
(`updates.rs:252-345`; `UpdateState` fields at `:17-28`). Endpoint handling is HTTPS-only with no
embedded credentials, no fragment, ≤5 redirects and no downgrade to non-HTTPS
(`safe_endpoint` at `:44-52`, custom redirect policies at `:107-112` and `:131-135`).

## Step 2 — Verify dependencies, config, pubkey and environment — **COMPLETED**

Inspection findings (not runtime test results):

| Item | Observed state |
| --- | --- |
| `src-tauri/tauri.conf.json:31` → `plugins.updater.pubkey` | `""` (empty string) |
| `plugins.updater.requireSignedVersion` | `true` |
| `plugins.updater.windows.installMode` | `"passive"` |
| `SWIFTFETCH_UPDATE_ENDPOINT` / `SWIFTFETCH_UPDATE_ARCHIVE` / `SWIFTFETCH_UPDATE_PUBLIC_KEY` | unset in this workspace (read with `option_env!` at `updates.rs:55-57`) |
| `TAURI_SIGNING_PRIVATE_KEY` / `TAURI_SIGNING_PRIVATE_KEY_PATH` | unset in this workspace |
| Updater artifacts (`*.sig`) in `src-tauri/target/release/bundle` | none present (also recorded in `UPDATER_KEY_SETUP.md` §1) |

Consequence in code: `publisher()` (`updates.rs:54-61`) returns `None` because the public key is
empty / env values are absent, so `UpdateManager::new` sets `configured = false` and phase
`"not_configured"` (`:68-76`); `check_for_updates` returns the snapshot without any network request
(`:258-275`), and `install_update` / `rollback_update` return `UPDATE_NOT_CONFIGURED`
(`:280`, `:328`). `tools/build-windows.ps1:19-25` enforces all-three-or-none and requires a signing
key only when the channel is configured. See `docs/UPDATES.md:3-5` and
`docs/release/UPDATER_KEY_SETUP.md` §1.

## Step 3 — Preserve architecture — **COMPLETED**

No source file, build script, configuration, manifest, endpoint or release artifact was modified by
this lane. The only write performed is this document. All existing update logic in
`src-tauri/src/updates.rs`, `tauri.conf.json`, `tools/build-windows.ps1` and
`tools/release-channel.ps1` is left exactly as inspected.

## Step 4 — Secure key generation instructions — **PREPARED**

Owner-run procedure is already documented in `docs/release/UPDATER_KEY_SETUP.md` §3–§4 and §7:

- Generate the minisign-format pair locally with the Tauri v2 signer
  (`npm run tauri -- signer generate -- -w "$env:USERPROFILE\.tauri\swiftfetch.key"`) or `minisign -G`;
  keep the private key and passphrase outside the repository, outside CI, and out of chat, source
  control, artifacts and logs. The repo `.gitignore` already covers `*.key`, `*.pfx`, `*.p12`, `.env*`.
- Only the **public** key string may be embedded via `SWIFTFETCH_UPDATE_PUBLIC_KEY` or
  `tauri.conf.json` `pubkey`. The private key is exposed to build tools only through
  `TAURI_SIGNING_PRIVATE_KEY_PATH` (preferred) or `TAURI_SIGNING_PRIVATE_KEY`.
- Update signing is not Authenticode signing and does not establish a Windows publisher identity.

No key pair was generated by this lane; key generation is an owner decision and action.

## Step 5 — GitHub Releases as the distribution host — **PREPARED**

Documented in `docs/release/GITHUB_RELEASES_SETUP.md` (asset names, tag convention `v0.1.0`,
`SHA256SUMS`, safe publication checklist) and `docs/release/UPDATER_KEY_SETUP.md` §6 (endpoint
templates):

- `SWIFTFETCH_UPDATE_ENDPOINT` → `https://github.com/[OWNER]/[REPO]/releases/latest/download/latest.json`
  (or a pinned `…/releases/download/v0.1.0/latest.json`).
- `SWIFTFETCH_UPDATE_ARCHIVE` → `https://github.com/[OWNER]/[REPO]/releases/download/v{{version}}/latest.json`
  so the installed version's signed installer remains available for rollback.

`[OWNER]` / `[REPO]` are placeholders. Selecting the host, creating the release and activating the
endpoint are owner actions; no host was chosen, no URL was set, and no release was created here.

## Step 6 — Signed manifests and signatures preparation — **BLOCKED** until the owner generates keys

Status: **BLOCKED** (and **REQUIRES OWNER APPROVAL** for the underlying key generation and endpoint
choice).

The chain cannot start without an owner-generated signing key:

1. `tools/build-windows.ps1` only emits updater artifacts (`bundle.createUpdaterArtifacts = true`,
   `:24`) when all three `SWIFTFETCH_*` channel values are set, and it refuses to build a configured
   channel without `TAURI_SIGNING_PRIVATE_KEY` / `_PATH` (`:22-24`).
2. `tools/release-channel.ps1:14-22` requires a `<installer>.sig` beside the chosen installer,
   decodes its base64 content and requires exactly one `trusted comment:` line carrying
   `version:<v>` equal to the `tauri.conf.json` version (`0.1.0`); otherwise it throws
   "Use the current Tauri signer with --app-version so the signature binds the release version."
3. The script then writes an immutable `artifacts/update-channel/0.1.0/latest.json` with
   `version`, `notes`, `pub_date` and `platforms."windows-x86_64" = { url, signature }`, copies the
   installer and `.sig`, and refuses to overwrite an existing version directory
   (`release-channel.ps1:23-38`).

No build, signing run, `.sig` file or `latest.json` was produced by this lane.

## Step 7 — Metadata / version comparison verification — **COMPLETED** (inspection only)

Status of inspection: **COMPLETED**. **No test command was executed by this lane** — the following
is what the source shows.

- The unit test named in the task exists at `src-tauri/src/updates.rs:554-582`:
  `signed_update_and_rollback_bytes_reject_tampering_and_version_replay`. It uses a **public**
  minisign test vector from `tauri-plugin-updater 2.14.1` (MIT/Apache-2.0); no private key is shipped
  (`:555-558`). It sets the mock updater config with `"requireSignedVersion": true` (`:561`) and
  asserts that valid signed bytes verify (`:571-572`), tampered bytes fail (`:573`), a manifest whose
  version (`3.0.0`) does not match the signature's bound version (`2.0.0`) fails — version-replay
  protection (`:574-575`) — and a cached rollback package written through `write_cache` /
  `journal` / `cached` still verifies (`:576-581`).
- Two further unit tests exist: `update_endpoints_reject_credentials_and_insecure_transport`
  (`:539-545`) and `corrupted_or_escaping_rollback_metadata_cannot_be_used` (`:547-551`).
- `requireSignedVersion: true` in `tauri.conf.json:31` is the production counterpart of the test
  flag; `tools/release-channel.ps1:18-22` independently enforces that the signed trusted comment
  binds the config version before a manifest is created.

These are source-level assertions with public test vectors, not a signed-installer verification in a
real release.

## Step 8 — Tests for detection, download, signature, install, failure and rollback — **BLOCKED**

Status: **BLOCKED** until owner-generated keys, an approved/activated HTTPS endpoint, real signed
release artifacts and a disposable Windows VM are available. No such test was run here.

| Check | What existing unit tests cover | Status for a real end-to-end run |
| --- | --- | --- |
| Update detection (manifest fetch/parse) | Endpoint URL safety only (`updates.rs:539-545`); no live manifest fetch | **BLOCKED** — no endpoint |
| Download (≤128 MiB bound, byte progress) | Not covered | **BLOCKED** — no signed package / endpoint |
| Signature & signed-version verification | Public-vector tests: valid bytes, tampering, version replay, cached rollback (`:554-582`) | **BLOCKED** — no real signed installer |
| Install (NSIS `/P` `/D`, MSI `INSTALLDIR`, helper readiness handshake) | Not covered | **BLOCKED** — no signed artifacts or clean VM |
| Failure handling / health check / automatic rollback | Not covered end-to-end | **BLOCKED** — requires a real install and a disposable VM |
| Explicit rollback command | Cached-package path validation is exercised at metadata level (`:547-551`, `:576-581`) | **BLOCKED** — requires cached signed bytes from a real release |
| Startup reconciliation of interrupted attempts | Not covered | **BLOCKED** — requires an interrupted real install |

As `docs/UPDATES.md:56-60` states, an actual signed installer upgrade and recovery must still be
exercised in a disposable Windows environment using the publisher's release artifacts.

## Step 9 — Publishing future updates (procedure only) — **PREPARED**

Status: **PREPARED**; performing any of it **REQUIRES OWNER APPROVAL**.

1. Owner generates/stores the key pair securely (Step 4) and chooses an HTTPS host (Step 5).
2. All three `SWIFTFETCH_*` values are set together (or all left unset); build with
   `tools/build-windows.ps1` plus the owner's signing overlay to emit signed `.sig` artifacts
   (`docs/UPDATES.md:7-29`, `UPDATER_KEY_SETUP.md` §4–§5).
3. For each new version, run `tools/release-channel.ps1` with the HTTPS base URL and chosen
   installer to create a fresh immutable version directory containing `latest.json`, the installer
   and its `.sig`; verify the single `windows-x86_64` entry, exact download URL and bound version.
4. Attach the installer, `.sig` and manifest as GitHub release assets under an immutable version tag
   (`GITHUB_RELEASES_SETUP.md` §2–§3, §6). Preserve every version the archive endpoint needs for
   rollback (`docs/UPDATES.md:28-29`).
5. Point `SWIFTFETCH_UPDATE_ENDPOINT` at the current manifest and publish only after the owner's
   explicit release approval. No manifest whose signature fails to bind the version may be
   published; a private key that left the owner's control must never be reused.

## Step 10 — Honest end-to-end status — **COMPLETED**

- **No end-to-end update test succeeded.** No real signed installer was downloaded, verified,
  installed, health-checked or rolled back in this workspace. The only verification evidence
  present in source is unit tests using a public test vector; this lane did not even execute those.
- **No update endpoint has been activated.** No `SWIFTFETCH_*` value was set, no host was chosen,
  no `latest.json` was published and no GitHub release or tag was created by this lane.
- The shipped configuration remains `not_configured`: empty `pubkey`, unset channel variables, no
  `*.sig` artifacts — the recommended current state per `UPDATER_KEY_SETUP.md` §1 and §4.
- **Do not claim that automatic updates work.** The implementation is present in source; its
  live operation is unverified and depends on owner actions (keys, endpoint, signed artifacts,
  approved publication) and a clean-VM end-to-end test that have not occurred.
