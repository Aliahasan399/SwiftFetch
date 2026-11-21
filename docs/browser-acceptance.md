# Browser acceptance evidence

`node tools/browser-acceptance.mjs` launches Microsoft Edge using a new profile
under `artifacts/browser-acceptance/`. It loads the packaged Chromium extension,
finds that extension's own service worker through the DevTools protocol, and
tests `chrome.runtime.sendNativeMessage` against the actual Windows executable.

The test creates a NativeMessagingHosts key only when no SwiftFetch Edge host is
already registered, and removes only its exact temporary key in cleanup. It
refuses to replace an existing host registration. Test manifests, downloads and
database state stay in the new artifact directory. Personal browser profiles
are not used.

Completed acceptance: `artifacts/browser-acceptance/run-a7Lzm2/result.json` records
successful actual Edge extension load, native host process launch, queue intake,
and byte-correct downloaded output. Protocol and Firefox classic-bundle tests
are additional automated gates.

This evidence does not claim browser-store approval or a live Firefox install.
Firefox packages preload deterministic classic-script bundles and are tested for
bundle execution and package-contained references. A Firefox runtime test and
any required signing/review remain separate distribution checks.
