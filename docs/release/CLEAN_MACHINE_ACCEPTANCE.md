# SwiftFetch — Clean-machine acceptance instructions

**Status:** externally blocked (no clean Windows VM available in this environment).
Do not simulate or claim these results. Record real evidence in a result file per run.

## Environment
- Clean Windows 10/11 x64 VM, fresh user profile, WebView2 runtime present or installer provided.
- No SwiftFetch data, no browser extension registration, no native-host key.
- Isolated network where needed; fixture URLs permitted.

## Sequence (record pass/fail + artifact paths for each)
1. **Install** signed NSIS and MSI builds separately on fresh VMs.
   - Verify publisher signature (`Get-AuthenticodeSignature` = Valid, correct publisher).
   - Verify installed executable hash equals `SHA256SUMS` in the candidate folder.
   - Launch; confirm UI renders and `get_settings` works over real IPC.
2. **Data preservation on upgrade**
   - Add a download, create a SQLite backup, set a distinctive setting.
   - Install the next signed version over the existing one.
   - Confirm queue, history, partial files, settings, and encrypted headers survive.
   - Confirm no unrelated files are deleted.
3. **Signed update + automatic rollback**
   - Configure the real HTTPS update channel per `OWNER_CONFIG_GUIDE.md`.
   - Trigger a signed update to a healthy build; confirm health check passes.
   - Force a failed health check; confirm signed rollback restores the prior version **and** the bound metadata snapshot.
   - Test bad signature/hash, interrupted update, incompatible version, and unavailable endpoint.
4. **Uninstall preservation**
   - Uninstall; confirm user data (downloads, backups, settings) remains unless the user explicitly chose removal.
   - Confirm browser native-host registration and extension removal behavior.
5. **Browser distribution**
   - Install the extension per supported browser; confirm permissions rationale, native host handoff, site-rule consent, and removal.
   - Do not claim store approval without a store receipt.
6. **Startup/soak**
   - Record warm/cold launch timings from the instrumented build (Phase 3).
   - Run a 24-hour soak; record crashes, memory, and completion rates.

## Blocking status
Until this document has a companion `clean-machine-result.json` with real signatures and hashes,
Phase 4 clean-machine acceptance remains **externally blocked / NO-GO**.
