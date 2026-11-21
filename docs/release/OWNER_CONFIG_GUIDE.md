# SwiftFetch — Owner configuration guide (secure local setup)

**Status:** template with placeholders. Do **not** put any secret in chat, source control,
artifacts, or this file. The owner fills these values locally at release time.

## 1. Publisher identity (owner decision)
- Publisher legal name: `[OWNER: legal entity name]`
- Support contact: `[OWNER: support email / URL]`
- Privacy notice URL: `[OWNER: published privacy notice]`
- Terms URL: `[OWNER: published terms]`
- Distribution license: `[OWNER: selected license + license file]`

## 2. Code signing (external dependency — no zero-cost trusted path)
- Obtain a publicly trusted Authenticode code-signing certificate and private key.
- Configure the release environment with the signing identity/key path **locally only**.
- Never commit the key, paste it into chat, or place it under `E:\CloneIdm`.
- Verify with `Get-AuthenticodeSignature` on exe/MSI/NSIS; `tools/stage-release.ps1` refuses unsigned artifacts.

## 3. Update channel (external dependency)
Set all three or none (partial config is rejected by `tools/build-windows.ps1`):
```
SWIFTFETCH_UPDATE_ENDPOINT   = [OWNER: HTTPS URL to signed latest.json manifest]
SWIFTFETCH_UPDATE_ARCHIVE    = [OWNER: HTTPS URL template containing {{version}}]
SWIFTFETCH_UPDATE_PUBLIC_KEY = [OWNER: minisign public key for the update signature]
TAURI_SIGNING_PRIVATE_KEY_PATH = [OWNER: local path to updater signing key]
```
- Endpoints must be HTTPS, no embedded credentials, no fragments.
- Keep immutable, signed version manifests. A public HTTPS endpoint does **not** replace Authenticode signing.

## 4. Distribution/registry (owner decision)
- Browser-store submissions: `[OWNER: Edge/Chrome/Firefox store strategy, signing, review]`
- Hosting for installers and manifests: `[OWNER: HTTPS host with stable URLs]`

## 5. How to hand off
Give this file to the owner, fill placeholders locally, then run:
```
./tools/build-windows.ps1            # build with signed config
./tools/stage-release.ps1            # refuses unsigned artifacts
```
Report which gates remain open. Do not set `ready_for_public_release: true` until all P0 gates in
`docs/release/RELEASE_PLAN.md` are closed.
