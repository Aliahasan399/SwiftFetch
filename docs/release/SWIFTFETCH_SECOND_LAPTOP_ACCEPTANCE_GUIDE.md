# SWIFTFETCH_SECOND_LAPTOP_ACCEPTANCE_GUIDE.md

**Purpose:** install and test the unsigned **development** build of SwiftFetch on a second Windows 11
laptop. This is development testing only. NO-GO and `ready_for_public_release: false` remain in force.

## 0. Before you start

- Use a dedicated test user profile if possible.
- Keep the installer and its hash record together in one folder.
- Windows Security/SmartScreen may warn that an unsigned app is from an unknown publisher. That is
  expected for this development build. **Do not disable Windows Defender or other protection.**
  Verify the file hash first, then choose the Windows-provided "More info → Run anyway" option only
  for a file whose hash you have independently confirmed.
- If any hash does not match, stop. Do not run the file.

## 1. Which development installer to copy

Copy the unsigned NSIS development installer listed in
`docs/release/SWIFTFETCH_DEVELOPMENT_BUILD_REPORT.md`:
- Path: `artifacts\dev-build\SwiftFetch_0.1.0_x64-setup.exe`
- Expected SHA-256: `4d05c3491df93657b21e39ba6735288c05f42f342c90dc24f9419e1d4a186e81`
- Version: 0.1.0 · Architecture: x64 · Signing: `NotSigned`

Only copy this exact file. Do not use historical installer hashes from
`INSTALLER_HASH_BASELINE.md` to validate this development build.

## 2. Verify the SHA-256 hash in PowerShell

Open PowerShell in the folder containing the copied installer and run:

```powershell
$f = '.\SwiftFetch_0.1.0_x64-setup.exe'
(Get-Item -LiteralPath $f).Length
(Get-FileHash -LiteralPath $f -Algorithm SHA256).Hash.ToLowerInvariant()
(Get-AuthenticodeSignature -LiteralPath $f).Status
```

Expected: size and hash from the development build report; signature `NotSigned`.
If the hash differs, delete the file and obtain a fresh copy.

## 3. Check Windows Security before installation

1. Right-click the installer → **Properties** → note the digital-signature tab (there will be none).
2. Open **Windows Security → Virus & threat protection** and scan the copied file.
3. If SmartScreen shows "Windows protected your PC", click **More info** and confirm the file name,
   size and hash before proceeding. Click **Run anyway** only after that confirmation.
4. Do not turn off real-time protection or submit the file as trusted without verification.

## 4. Install the application

1. Double-click the verified installer.
2. The NSIS build is configured for the current user; it should install to
   `%LOCALAPPDATA%\SwiftFetch` without administrator elevation.
3. Complete the wizard and choose whether to launch SwiftFetch at the end.
4. Confirm Start menu and desktop shortcuts were created.

## 5. Launch SwiftFetch

1. Start SwiftFetch from the Start menu.
2. Confirm the window renders and shows version 0.1.0.
3. In **Settings**, confirm the default download folder and theme.

## 6. Enable browser integration

1. In SwiftFetch → **Settings**, enable **Browser integration**.
2. Leave the site rule at **Ask** for the first test.

## 7. Register the unsigned development Native Messaging host

Use the repository registration script with the unsigned-preview option:

```powershell
cd <repository>\swiftfetch
.\browser-extension\tools\register-host.ps1 -Browser Chrome -ExecutablePath '<installed swiftfetch.exe>' -AllowUnsignedPreview
.\browser-extension\tools\register-host.ps1 -Browser Edge -ExecutablePath '<installed swiftfetch.exe>' -AllowUnsignedPreview
.\browser-extension\tools\register-host.ps1 -Browser Firefox -ExecutablePath '<installed swiftfetch.exe>' -AllowUnsignedPreview
```

Replace `<installed swiftfetch.exe>` with the actual path, usually
`%LOCALAPPDATA%\SwiftFetch\swiftfetch.exe`. Run one command per browser you intend to test.
The script writes per-user registry keys and manifest files; it does not modify system-wide settings.

## 8. Load the Chrome / Edge development extension

1. Open `chrome://extensions` (or `edge://extensions`).
2. Enable **Developer mode**.
3. Click **Load unpacked** and select `<repository>\swiftfetch\artifacts\smart-download-dev-package\chromium`.
4. Confirm the extension name "SwiftFetch Download Bridge" and version 0.1.0 appear.
5. Pin the toolbar icon if desired.

## 9. Install the Firefox development extension

1. Open `about:debugging#/runtime/this-firefox`.
2. Click **Load Temporary Add-on** and select
   `<repository>\swiftfetch\artifacts\smart-download-dev-package\firefox\manifest.json`.
3. Confirm the add-on is listed. Temporary add-ons are removed when Firefox closes.

## 10. Test ordinary downloads

1. From a test page with an HTTP/HTTPS link, Alt+Click a link (or Ctrl+Shift+Click).
2. Use the context menu **Download with SwiftFetch** on a link.
3. Confirm a browser-initiated ordinary download opens the SwiftFetch confirmation dialog.
4. Verify no download starts until **Start download** or **Add to queue** is clicked.

## 11. Test the confirmation dialog

1. Check original filename is shown and can be edited.
2. Change the destination folder to a test folder.
3. Confirm type and size are shown when available; "Checking…" or "Not reported" is acceptable.
4. Select each category: Videos, Music, Documents, Programs, Compressed, Other.
5. Select a queue.
6. Click **Cancel** — no task should be created.
7. Repeat and click **Add to queue** — task should appear paused/queued.
8. Repeat and click **Start download** — transfer should begin.
9. Test a duplicate filename; SwiftFetch should keep both files (Rename policy).

## 12. Test media detection

1. Open a test page containing an ordinary `<video>` and `<audio>` element with direct HTTP(S) sources.
2. Confirm a small SwiftFetch chip appears near each player without covering its controls.
3. Scroll and resize; the chip should follow the player and hide when the player is off-screen.
4. Set the site rule to **Deny** in the popup; the chip should disappear.
5. Confirm protected/DRM media produces an explanation, not a successful download.

## 13. Test the floating indicator

1. Confirm the chip is keyboard reachable and labelled "SwiftFetch media options".
2. Click the chip; the media panel should open with available variants.
3. Confirm no download begins merely because the chip was clicked.

## 14. Test quality selection

1. On media with multiple detectable variants, confirm each variant is listed with its real label.
2. Select a variant and choose **Start download** or **Add to queue**.
3. Confirm exactly one task is created for the selected variant.
4. Confirm no fabricated resolution or quality labels are shown.

## 15. Test HLS/DASH fixtures

1. Use local fixtures from `tools/fixtures/server.py` (`/manifest/hls.m3u8`, `/manifest/dash.mpd`).
2. Confirm supported variants can be selected and downloaded as one assembled file.
3. Confirm encrypted, live, template-based or unsupported manifests show a clear explanation.
4. Confirm cancellation stops assembly and removes the working file when the "keep partials" option is off.

## 16. Collect logs and report failures safely

1. In SwiftFetch → **Diagnostics**, export the shareable bundle. It removes URLs and headers; keep it
   local and review it before sharing.
2. Save screenshots of any failure with the browser version and Windows build.
3. Report: browser + version, test case number, PASS/FAIL, exact steps, and evidence path.
4. Do not paste cookies, credentials, private keys or full authenticated URLs into a report.

## 17. Known limits

- The development installer is unsigned; SmartScreen warnings are expected.
- The shipped installer hash records are historical and do not describe this development build.
- Firefox may require re-loading the temporary add-on after a browser restart.
- HLS/DASH segment assembly supports the tested static formats only; protected streams are rejected.

Do not change NO-GO or `ready_for_public_release` based on development testing. The owner must review
second-laptop results before any production release recommendation.
