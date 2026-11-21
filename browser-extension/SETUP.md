# Browser integration for the Windows candidate

Build self-contained extension folders from the project root:

```powershell
node browser-extension/tools/package.mjs
node --test browser-extension/test/test_protocol.js browser-extension/test/test_release.js
```

The packaging command prints a new directory under `artifacts/browser-extension`.
Use its `chromium` folder for Chrome or Edge. Use its `firefox` folder for
Firefox. Each folder contains its own protocol module, scripts, popup and icons.
The source directories under `browser-extension/chrome` and `firefox` are inputs
to packaging.

## Chrome and Edge

1. Open the browser's Extensions page, enable Developer mode, and load the
   generated `chromium` folder as an unpacked extension.
2. Copy the extension ID shown in that browser.
3. Install or select a tested SwiftFetch executable, and enable browser integration
   in its desktop Settings.
4. Preview the per-user host registration, substituting the installed executable
   path and the extension ID:

```powershell
./browser-extension/tools/register-host.ps1 -Browser Edge `
  -ExecutablePath 'C:/path/to/swiftfetch.exe' `
  -ExtensionId 'the-extension-id-from-the-browser' -WhatIf
```

Use `-Browser Chrome` for Chrome. Once the printed target is correct, run the
same command without `-WhatIf`. A locally built unsigned candidate requires the
explicit `-AllowUnsignedPreview` switch. Public distribution requires the
publisher's signed build.

## Firefox

Load the generated `firefox/manifest.json` with Firefox's temporary add-on
loader, then register the host with `-Browser Firefox`. Its configured add-on
ID is `swiftfetch@localhost`; no Chromium extension ID is needed. Permanent
installation requires the appropriate Mozilla signing and review process.

## What happens when sending a link

Use the popup button, the link context menu, or a trusted Alt+Click. The extension
sends a short-lived intent to the installed executable through Native Messaging.
The executable checks desktop opt-in and saves the task to the local queue.
Open the desktop app to process queued tasks if it is closed. Browser handoffs
use the Rename collision policy and do not replace existing downloads.

The host uses a 4-byte little-endian length followed by UTF-8 JSON. Its accepted
reply is `{ "ok": true, "value": { "task_id": "..." } }`. Rejected replies have
`ok: false`, `code`, and a safe `message`. Requests expire within five minutes;
repeat request IDs are rejected across host processes, including after the
corresponding task is removed from history.

The browser's native-host manifest restricts the allowed extension ID. Registration
uses that browser's `NativeMessagingHosts/com.swiftfetch.app` key under
`HKEY_CURRENT_USER`. It does not require a background HTTP listener, Python host,
or shared-secret environment variable. An existing different registration is
left for the user to review.

## Privacy and verification

The existing `contextMenus`, `storage`, `nativeMessaging`, and `tabs` permissions
are used. A content script is registered on pages to detect explicit gestures;
it does not collect browsing history or read cookies. The active page URL is
read when the user opens the popup. Selected URLs are sent only to the local
desktop queue. Cookies, credentials, and unknown intent fields are rejected.

Protocol tests, package path/permission tests, and the native executable handoff
have automated coverage. Browser-store approval and a live Chrome/Edge/Firefox
installation test remain separate release checks. See [../VERIFICATION.md](../VERIFICATION.md).
