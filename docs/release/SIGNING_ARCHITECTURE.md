# SwiftFetch — secure signing architecture (design only, NOT ACTIVE)

**Status:** DESIGN. No signing workflow is installed in `.github/workflows/`; no signing has been
performed; no secret has been created. NO-GO and `ready_for_public_release: false` preserved.

## 1. Trust model

| Element | Design | Current state |
|---|---|---|
| Signing key | Generated and stored on SignPath's HSM. Never exported, never in repo, never on a local disk. | No key exists for this project |
| Signing service | SignPath.io OSS subscription using a SignPath Foundation certificate (subject: SignPath Foundation) | Not applied for |
| Build origin | GitHub Actions on GitHub-hosted runners; SignPath GitHub App verifies origin from GitHub metadata, not from script-supplied values | Not configured |
| Approval | Each release signing request manually approved by a named Approver in SignPath | Not configured |
| Secret | One API token in GitHub Actions secrets; no password shared in chat | Not created |
| Publication | Separate, later, owner-authorized step. Signing ≠ publishing | Not authorized |

The CI job has no ability to publish a release, create a tag, activate the updater, or push a
commit. It only builds, submits for signing, verifies, hashes, and uploads artifacts.

## 2. Artifact flow

```
source (public repo, pinned toolchain + lockfiles)
  -> build-windows.ps1 (unsigned MSI + NSIS; NSIS post-build patch + recompile)
  -> upload unsigned artifacts as a GitHub Actions artifact
  -> submit signing request (official SignPath action, waits for completion)
  -> download signed artifacts
  -> Authenticode verification (publisher, chain, timestamp, nested PE)
  -> generate-distribution-hashes.ps1 (signed artifacts only) -> SHA256SUMS
  -> upload signed artifacts + hashes as CI artifacts
  -> [manual, separate] owner decision to publish
```

If any verification step fails, the job fails. It must never fall back to unsigned artifacts.

## 3. Artifact configuration draft (SignPath)

Store as documentation; apply in SignPath only after acceptance. The NSIS setup is a PE file; the
MSI is deep-signed (nested `swiftfetch.exe` / `swiftfetch_lib.dll` first, then the MSI).

```xml
<artifact-configuration xmlns="http://signpath.io/artifact-configuration/v1">
  <parameters>
    <parameter name="version" required="true" />
  </parameters>
  <zip-file>
    <pe-file path="SwiftFetch_${version}_x64-setup.exe">
      <authenticode-sign/>
    </pe-file>
    <msi-file path="SwiftFetch_${version}_x64_en-US.msi" product-name="SwiftFetch">
      <pe-file-set product-name="SwiftFetch" product-version="${version}" file-version="${version}">
        <include path="swiftfetch.exe" />
        <include path="swiftfetch_lib.dll" />
        <for-each><authenticode-sign /></for-each>
      </pe-file-set>
      <authenticode-sign/>
    </msi-file>
  </zip-file>
</artifact-configuration>
```

The exact file names inside the MSI must be confirmed against a generated administrative-extracted
payload before this configuration is submitted. Metadata restriction values must match the existing
version (`0.1.0`) enforced by `tauri.conf.json` and the build wrapper.

## 4. GitHub Actions workflow template (INACTIVE)

This is a **template**, kept here as documentation. Do not copy into `.github/workflows/` until the
owner explicitly authorizes production signing.

```yaml
name: sign-installers  # TEMPLATE ONLY - NOT ENABLED
on:
  workflow_dispatch:
    inputs:
      version:
        description: 'Release version to sign'
        required: true
        type: string

permissions:
  contents: read
  actions: read

jobs:
  build-and-sign:
    runs-on: windows-latest
    steps:
      - uses: actions/checkout@v4
        with: { persist-credentials: false }
      - uses: actions/setup-node@v4
        with: { node-version: '24', cache: 'npm' }
      - run: npm ci
      - name: Build unsigned installers
        run: ./tools/build-installers.ps1
      - name: Upload unsigned artifacts
        id: upload
        uses: actions/upload-artifact@v4
        with:
          name: unsigned-installers
          path: |
            src-tauri/target/release/bundle/msi/*.msi
            src-tauri/target/release/bundle/nsis/*.exe
          retention-days: 7
      - name: Submit signing request
        uses: signpath/github-action-submit-signing-request@v3
        with:
          api-token: ${{ secrets.SIGNPATH_API_TOKEN }}
          organization-id: ${{ vars.SIGNPATH_ORGANIZATION_ID }}
          project-slug: swiftfetch
          signing-policy-slug: release-signing
          artifact-configuration-slug: swiftfetch-installers
          github-artifact-id: ${{ steps.upload.outputs.artifact-id }}
          wait-for-completion: true
          output-artifact-directory: signed/
          parameters: |
            version: ${{ toJSON(inputs.version) }}
      - name: Verify Authenticode signatures
        shell: pwsh
        run: ./tools/verify-signed-installers.ps1 -Directory signed -ExpectedPublisher 'SignPath Foundation'
      - name: Generate distribution hashes
        shell: pwsh
        run: ./tools/generate-distribution-hashes.ps1 -Msi signed/<msi> -Nsis signed/<nsis> -ExpectedPublisher 'SignPath Foundation'
      - name: Upload signed artifacts and hashes
        uses: actions/upload-artifact@v4
        with:
          name: signed-installers
          path: |
            signed/
            artifacts/distribution-hashes/
          retention-days: 7
```

`tools/verify-signed-installers.ps1` is a **planned** helper, not yet implemented. It must be
written and tested before the workflow is enabled. Placeholders `<msi>` / `<nsis>` must be replaced
with deterministic resolution logic (resolve by glob and assert exactly one match).

## 5. Secret and key protection requirements

1. **No private key material** in the repository, CI logs, artifacts, memory dumps, or chat. With
   SignPath the key stays in the HSM; this is the primary reason for the design.
2. Create `SIGNPATH_API_TOKEN` as an encrypted GitHub Actions secret (repository or environment
   scope). Never print it; the action consumes it directly.
3. Restrict the token to a single user with **submitter** permission in the SwiftFetch project; do
   not grant approver rights to the CI token. Rotation plan: revoke and reissue in SignPath, update
   the secret, record the rotation date (no secret value in records).
4. Use an environment with required reviewers for the signing job if manual approval is also desired
   at GitHub level; SignPath's own Approver step remains authoritative.
5. Do not expose secrets to pull requests from forks. Use `workflow_dispatch` (as above), not
   `pull_request_target`. Keep `persist-credentials: false`.
6. Pin third-party actions to commit SHAs before enabling, and follow the existing `windows.yml`
   least-privilege conventions (`permissions: contents: read`).
7. Never log signing requests, tokens, download URLs with embedded credentials, or certificate
   private data.

## 6. Authenticode verification procedure (to be automated)

Required checks on each signed artifact, both in CI and before distribution:

```powershell
# 1. Signature status and publisher
$sig = Get-AuthenticodeSignature -LiteralPath $artifact
$sig.Status                       # must be 'Valid'
$sig.SignerCertificate.Subject    # must contain the approved publisher (e.g. SignPath Foundation)

# 2. Full chain and timestamp diagnostic
signtool verify /pa /v $artifact  # exit 0; chain ends in a trusted root; timestamp valid

# 3. Hash integrity against the signed-artifact record
(Get-FileHash -LiteralPath $artifact -Algorithm SHA256).Hash

# 4. MSI nested binaries
#    administrative-extract the signed MSI and repeat steps 1-3 on swiftfetch.exe and swiftfetch_lib.dll

# 5. NSIS setup executable
#    the setup.exe itself is a PE file; verify it directly (no extraction needed)
```

Acceptance rules:
- `Status = Valid`; any other value (`NotSigned`, `HashMismatch`, `UnknownError`, etc.) fails.
- Signer subject matches the owner-approved publisher; a changed publisher is a release blocker.
- Timestamp must be within the certificate validity window.
- The MSI's embedded PE files must also verify; a signed wrapper with unsigned payload is not
  acceptable for this design.
- Verification output and hashes are archived as evidence.

Note: SmartScreen reputation is **not** verifiable by these commands. A new signing identity may
still show warnings until reputation accumulates; this must be communicated, not claimed away.

## 7. Signed-installer hash generation and distribution verification

1. Run `tools/generate-distribution-hashes.ps1` **only after** the verification in §6 passes. It
   refuses unsigned artifacts (already tested; see `INSTALLER_HASH_BASELINE.md` §5).
2. It writes `artifacts/distribution-hashes/hashes-<timestamp>/SHA256SUMS` and
   `distribution-hashes.json`. These supersede the historical unsigned hashes in every distribution
   context.
3. Update the separate **signed** record in `INSTALLER_HASH_BASELINE.md` with the new values and the
   signer subject. Never overwrite the historical unsigned table.
4. Distribution verification for a user:
   ```powershell
   Get-FileHash -LiteralPath .\SwiftFetch_0.1.0_x64-setup.exe -Algorithm SHA256
   # compare (case-insensitive) with the line in SHA256SUMS
   ```
5. If an artifact is rebuilt, resigned, patched, or its certificate changes, delete the old signed
   hash record entry, regenerate hashes, and re-verify.

## 8. Current status and next implementation work

- Not applied for; not configured; not active.
- Before enabling: implement `tools/verify-signed-installers.ps1`, test the workflow in a non-release
  branch/test-signing policy, obtain owner approval for the publisher trade-off and for any
  publication, and re-check `signpath.org/terms.html`.
- No `ready_for_public_release` change, no tag, no release, no updater activation.
