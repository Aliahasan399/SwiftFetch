# SwiftFetch Windows release

SwiftFetch is a local desktop download manager. The Windows x64 build produces an
NSIS setup executable and an MSI. The browser extension is an optional, separately
installed component and is off by default in desktop Settings.

The NSIS build installs for the current Windows user. The MSI is configured for
all users and can require elevation. Queue data stays in the relevant Windows
user's data directory. Review both installer types on a clean test machine.

## Build a candidate

From the project root on Windows, using the build tools in [README.md](README.md):

```powershell
npm ci
./tools/verify.ps1
./tools/build-windows.ps1
node tools/smoke-desktop.mjs
node tools/dependency-notices.mjs
```

The release binary and installers are under `src-tauri/target/release/`. The
smoke test uses a temporary queue, downloads directory and WebView2 profile
under `artifacts/smoke/`. It uses loopback fixtures and launches the packaged
binary. See the [verification record](VERIFICATION.md) for the checks run on the
current candidate.

`tools/dependency-notices.mjs` collects the locked Windows Rust graph, npm runtime
dependencies and available license text into a fresh folder under
`artifacts/dependencies/`. Review entries without license files. Its output omits
the package-manager cache paths on the build machine.

## Sign and stage a public build

1. Set up the release owner's Windows code-signing certificate or signing service
   in the release environment. Keep access keys out of the project and its logs.
2. Build with `./tools/build-windows.ps1 -SigningConfig <path-to-tauri-config>`
   using a Tauri JSON overlay with the publisher's `bundle.windows` signing
   settings. Use the [Tauri Windows signing guide](https://v2.tauri.app/distribute/sign/windows/).
3. Stage the signed installers and their SHA-256 manifest with
   `./tools/stage-release.ps1 -PublisherThumbprint <certificate-thumbprint>`.
   The staging command checks the application binary and both installers against
   the selected certificate before it copies the installers.
4. Install each signed installer on a clean Windows x64 test machine. Check first
   launch, a fixture download, and Windows publisher information. Upgrade from
   the intended previous build with a paused download, then verify that history,
   checkpoints, and completed files survive. Test uninstall and reinstall.
5. Publish the selected installers with `SHA256SUMS`, `release-manifest.json`,
   release notes, dependency notices and a license approved by the owner. Browser
   packages require their own install and handoff checks before store submission.

## Updates and recovery

Development builds without a publisher channel use manual installer updates.
Configured builds implement the signed update/health/rollback path described in
[docs/UPDATES.md](docs/UPDATES.md). Exit SwiftFetch before a manual upgrade and
keep a copy of the previous signed installer. Back up the SwiftFetch data directory
while the app is closed, along with any partial files you need. Start the new
build and confirm that it can open the queue and manage an existing download.
If it cannot, exit, reinstall a compatible signed build, and restore the
matching backup if required. Completed downloads stay in the selected destination.

By default queue state is stored under `%APPDATA%\swiftfetch`. If you set
`SWIFTFETCH_DATA_DIR` or `SWIFTFETCH_DB_PATH`, use that location for backup and
restore. The queue can contain private URLs, filenames, and paths. Treat a
backup as private data.

## Source snapshot

`./tools/package-source.ps1` creates a ZIP from an explicit allowlist of project
source, manifests, icons, tools and documentation. It excludes download data,
databases, build output, signing keys and timestamped working backups. It creates
the snapshot locally; it does not publish or grant a software license. Select a
distribution license with the project owner before making the archive public.
