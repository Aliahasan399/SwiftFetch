# SwiftFetch — Tauri updater keys and signed-manifest setup

**Status:** procedure document only. This document does **not** generate keys, set environment
variables, sign artifacts, create manifests, activate an endpoint, or publish anything. Every
command below is to be run by the release owner locally, at the owner's discretion.

Owner decisions recorded by the parent lane: individual/personal publisher, proposed
GPL-3.0-or-later, free download, zero budget. No endpoint URL, key material, or contact identity is
invented here.

## 1. Current state (verified by read-only inspection)

| Item | Observed state |
| --- | --- |
| `src-tauri/tauri.conf.json` → `plugins.updater.pubkey` | `""` (empty string) |
| `plugins.updater.requireSignedVersion` | `true` |
| `plugins.updater.windows.installMode` | `"passive"` |
| `SWIFTFETCH_UPDATE_ENDPOINT` / `_ARCHIVE` / `_PUBLIC_KEY` | unset in this workspace (no value embedded) |
| `TAURI_SIGNING_PRIVATE_KEY` / `_PATH` | unset in this workspace |
| Updater artifacts (`*.sig`) in `src-tauri/target/release/bundle` | none present |
| Effective behavior | `publisher()` in `src-tauri/src/updates.rs:54-61` returns `None`; the app reports `not_configured`, performs no release network request, and update/rollback commands return `UPDATE_NOT_CONFIGURED` |

The updater code is present and tested with a public test vector, but the channel is **not
configured**. The private key is never shipped; tests only embed a public key/signature
(`src-tauri/src/updates.rs:539-582`).

## 2. How the updater is wired (what the config actually controls)

These three values are **compile-time** variables read with `option_env!`
(`src-tauri/src/updates.rs:55-57`) and embedded into the binary at build time:

| Variable | Meaning | Constraint |
| --- | --- | --- |
| `SWIFTFETCH_UPDATE_ENDPOINT` | HTTPS URL of the "latest" signed manifest | Must be HTTPS, no embedded credentials, no fragment (`safe_endpoint`, `updates.rs:44-52`) |
| `SWIFTFETCH_UPDATE_ARCHIVE` | HTTPS immutable manifest URL template containing `{{version}}` | E.g. `https://…/v{{version}}/latest.json`; the installed version's signed installer must remain available here for rollback |
| `SWIFTFETCH_UPDATE_PUBLIC_KEY` | The **public** minisign/Tauri key content used to verify signatures | Public key only — never a private key |

`tools/build-windows.ps1:19-25` enforces an all-or-nothing rule: set **all three or none**. A partial
configuration fails the build. When all three are set, the script also requires a Tauri update
signing key in the environment and adds the build overlay `bundle.createUpdaterArtifacts = true`
(which makes Tauri emit `.sig` artifacts alongside the installers). builds run with `--ci --locked`.

`tools/release-channel.ps1` then turns a signed installer into an immutable manifest:
- reads the `.sig` file next to the installer (`<installer>.sig`);
- decodes it and requires exactly one `trusted comment:` line with `version:<v>` equal to the
  `tauri.conf.json` version (`0.1.0`) — that is why the signer must bind the app version (see §3);
- writes `latest.json` with `version`, `notes`, `pub_date`, and
  `platforms."windows-x86_64" = { url, signature }`;
- copies the installer and `.sig` into a new version directory and refuses to overwrite an
  existing immutable version directory.

The app validates the manifest (≤128 KiB), downloads the package (≤128 MiB), verifies the minisign
signature and signed version before anything is installed, and stages a verified rollback from the
archive endpoint (`src-tauri/src/updates.rs:92-151, 277-313`).

## 3. Generate the signing key pair LOCALLY

The updater uses minisign-format keys. The recommended path is the Tauri v2 signer CLI (already a
dev dependency via `@tauri-apps/cli`). Run from the project root. **Choose a passphrase and store it
in a password manager; never paste keys or the passphrase into chat, source control, artifacts, or
logs.**

```powershell
# Generate a key pair interactively (Tauri v2). The tool prompts for a password.
npm run tauri -- signer generate -- -w "$env:USERPROFILE\.tauri\swiftfetch.key"
```

Equivalent direct invocation:

```powershell
npx tauri signer generate -- -w "$env:USERPROFILE\.tauri\swiftfetch.key"
```

This writes a private key to `…\.tauri\swiftfetch.key` and a public key to
`…\.tauri\swiftfetch.key.pub`. The CLI also prints the public key string to paste into config.

Alternative using minisign directly (if preferred):

```powershell
minisign -G -p "$env:USERPROFILE\.tauri\swiftfetch.pub" -s "$env:USERPROFILE\.tauri\swiftfetch.key"
```

Security requirements:

- The private key (`swiftfetch.key`) is the signing identity. It must live only on the owner's
  trusted machine, protected by a passphrase, and backed up securely offline. It must **never** be
  committed. The repository `.gitignore` already ignores `*.key`, `*.pfx`, `*.p12`, and `.env*`.
- Only the **public** key text (`swiftfetch.key.pub` / the CLI-printed string) may be embedded in
  the app and set in `SWIFTFETCH_UPDATE_PUBLIC_KEY` / `tauri.conf.json` `pubkey`.
- Update signing is independent of Authenticode code signing. It does **not** establish a Windows
  publisher identity or reduce SmartScreen warnings (see `OWNER_CONFIG_GUIDE.md` and
  `SIGNPATH_ELIGIBILITY.md`).

## 4. Set the environment variables (locally, at build time)

Set these only in the release environment on the owner's machine — never in tracked files. Example
PowerShell session (owner fills real values):

```powershell
# --- embedded in the app (public values) ---
$env:SWIFTFETCH_UPDATE_ENDPOINT   = 'https://[OWNER]/[HOST]/releases/latest/download/latest.json'
$env:SWIFTFETCH_UPDATE_ARCHIVE    = 'https://[OWNER]/[HOST]/releases/download/v{{version}}/latest.json'
$env:SWIFTFETCH_UPDATE_PUBLIC_KEY = (Get-Content -Raw "$env:USERPROFILE\.tauri\swiftfetch.key.pub").Trim()

# --- used only by build/signing tools (secrets) ---
$env:TAURI_SIGNING_PRIVATE_KEY_PATH     = "$env:USERPROFILE\.tauri\swiftfetch.key"
# If the key is passphrase-protected (set locally, not committed):
$env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = '[OWNER: key passphrase]'

# Build the signed release (emits .sig artifacts; refuses unsigned/partial config)
./tools/build-windows.ps1
```

Alternative to `_PATH`: set `$env:TAURI_SIGNING_PRIVATE_KEY` to the private key **contents**; do not
set both. Prefer `_PATH` so the secret does not appear in process listings or shell history. If the
owner chooses to fill `plugins.updater.pubkey` in `tauri.conf.json` instead of relying on the
embedded env value, use the same public key string; do not put a private key there.

Do not enable the update channel partially. If any one of the three `SWIFTFETCH_*` values is
missing, `tools/build-windows.ps1` throws; this is intentional. If the owner is not ready to
operate an endpoint, leave all three unset — the shipped behavior is then a silent, offline,
`not_configured` update feature, which is the recommended current state.

**No endpoint is activated by this document.** The owner must choose and approve a concrete HTTPS
host before filling the endpoint variables. GitHub Releases can serve as that host (§6) at zero
cost for a public repository, but selecting it and publishing the first manifest are owner
decisions.

## 5. Producing manifests and signatures

1. With the channel configured, `./tools/build-windows.ps1` produces the installers plus Tauri
   updater artifact signatures (`.sig`) next to them, e.g.
   `SwiftFetch_0.1.0_x64-setup.exe.sig` and `SwiftFetch_0.1.0_x64_en-US.msi.sig`. The signature's
   trusted comment binds version `0.1.0` via the signer's `--app-version` handling.
2. Prepare an immutable manifest for a chosen installer:

   ```powershell
   ./tools/release-channel.ps1 `
     -ArtifactBaseUrl 'https://[OWNER]/[HOST]/releases/download/v0.1.0' `
     -Installer 'src-tauri/target/release/bundle/nsis/SwiftFetch_0.1.0_x64-setup.exe' `
     -Notes 'SwiftFetch 0.1.0 Windows update'
   ```

   This writes `artifacts/update-channel/0.1.0/latest.json` plus copies of the installer and `.sig`.
   The base URL must be HTTPS without credentials and the version directory is created fresh
   (immutability guard). Verify the resolved download URL before publishing.
3. Verify the generated `latest.json` contains a single `windows-x86_64` entry whose `url` points at
   the exact installer and whose `signature` is the base64 `.sig` content; confirm `version` is
   `0.1.0`.
4. Only after release approval, publish the manifest and installer together to the chosen host, then
   point `SWIFTFETCH_UPDATE_ENDPOINT` at the current manifest. Never publish a manifest whose
   signature does not bind the version, and never reuse a private key that has left the owner's
   control.

## 6. Hosting the manifest and signed installers on GitHub Releases

A public GitHub repository can host, at no cost:

- the signed installers and their `.sig` files as release assets (attached to the `v0.1.0` release —
  see `GITHUB_RELEASES_SETUP.md`);
- the `latest.json` manifest as an additional release asset;
- an immutable per-version manifest under a version-pinned asset/path so the archive endpoint keeps
  the current installer available for rollback.

Concretely, the owner can set:

- `SWIFTFETCH_UPDATE_ENDPOINT` → `https://github.com/[OWNER]/[REPO]/releases/latest/download/latest.json`
  (points at the newest manifest), **or** a pinned
  `https://github.com/[OWNER]/[REPO]/releases/download/v0.1.0/latest.json` for a controlled rollout;
- `SWIFTFETCH_UPDATE_ARCHIVE` →
  `https://github.com/[OWNER]/[REPO]/releases/download/v{{version}}/latest.json` so the installer
  for the currently installed version remains downloadable for signed rollback.

`[OWNER]` and `[REPO]` are placeholders the owner fills locally; none are invented here. Confirm
that GitHub serves these with `Content-Type: application/json` for `latest.json` and that the
`.sig`/installer URLs are reachable over HTTPS with no redirects to non-HTTPS targets (the app
rejects insecure redirects, `updates.rs:107-112, 131-135`). Release assets are immutable only if the
owner does not delete/replace them; preserve the v0.1.0 assets for as long as clients may need
rollback.

## 7. Rules (non-negotiable)

- **Never commit private keys**, passphrases, or signed-key material. Keep secrets in local
  environment variables / a secure store only. Do not place them under `E:\CloneIdm` in tracked
  files or release artifacts.
- **Never activate an endpoint before owner approval.** No build here sets an endpoint, signs a
  release, or publishes a manifest. Endpoint activation is a deliberate owner action.
- **Only the public key is embedded** in the app and in `SWIFTFETCH_UPDATE_PUBLIC_KEY` /
  `tauri.conf.json`. The private key is used exclusively by the build/signing tools.
- **Set all three or none.** Partial configuration is rejected by the build.
- Update signing is not Authenticode signing; do not describe a minisign-signed update as a trusted
  Windows publisher signature.

## 8. Handoff checklist

- [ ] Owner generated a key pair locally and stored the private key + passphrase securely.
- [ ] Owner chose an HTTPS host and filled `[OWNER]`/`[REPO]`/host values locally.
- [ ] All three `SWIFTFETCH_*` values set together — or intentionally left all unset (current state).
- [ ] Private key available to build tools only via `TAURI_SIGNING_PRIVATE_KEY_PATH` (or contents);
      password supplied only if required.
- [ ] `.sig` artifacts and `latest.json` verified (version `0.1.0`, single platform, HTTPS URLs).
- [ ] Owner approved endpoint activation and publication; only then publish manifest + installers.
- [ ] No keys, passphrases, or endpoints committed; no publication performed before approval.
