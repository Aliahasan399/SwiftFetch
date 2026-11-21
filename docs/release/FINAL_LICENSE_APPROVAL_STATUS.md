# Final License Approval Status — Phase 3 Additional Task

**Date:** 2026-11-21 · **Status:** CONDITIONAL — NOT FINALLY APPROVED

## Findings (from LICENSE_AUDIT.md, FINAL_LICENSE_DECISION.md, UPSTREAM_LICENSE_TEXTS.md)

- Dependency audit: 338 packages; no proprietary, LicenseRef, AGPL/SSPL, or
  GPL-2.0-only dependency; no Apache-2.0/GPL-2.0-only conflict for GPLv3 target.
- Verdict: **CONDITIONALLY APPROVED — GPL-3.0-or-later** (not final).
- **Root `LICENSE` file intentionally absent.** The public repository must not be
  represented as finally GPL-licensed.

## CDLA-Permissive-2.0 uncertainty (`webpki-roots` 0.26.11, 1.0.11)
- Permissive **data** license covering bundled CA certificate data; no copyleft.
- No formal FSF GPL-compatibility determination located.
- No legal/copyright-holder confirmation obtained. Compatibility is **not assumed**.
- Obligation regardless: ship the CDLA agreement text with the data
  (https://cdla.dev/permissive-2-0/).

## Seven missing-text entries
`alloc-stdlib 0.3.0`, `clipboard-win 5.4.3`, `selectors 0.38.0`, `tauri-plugin 2.7.2`,
`webview2-com 0.39.1`, `webview2-com-macros 0.8.1`, `webview2-com-sys 0.39.1` —
canonical upstream sources recorded in `UPSTREAM_LICENSE_TEXTS.md`. Texts are **not
yet bundled** into notices/installer.

## MPL-2.0 (5 crates)
File-level copyleft, §3.3-compatible with GPLv3 using standard Exhibit A headers;
MPL text + per-file notices must ship; modified MPL files (none observed) would
remain MPL and be offered in source.

## Final recommendation
**Do NOT finalize the license yet.** Preserve CONDITIONAL status. Owner must:
1. Select GPL-3.0-or-later (or alternative) explicitly.
2. Accept or resolve the CDLA uncertainty (owner/counsel/copyright-holder).
3. Confirm copyright holder and years.
4. Bundle the 7 upstream texts, CDLA text, MPL text, and GPLv3 text into releases.
5. Regenerate notices against the final locked dependency set.
6. Only then add the root `LICENSE` and update README/website license pages from "proposed" to final.

## Status
- COMPLETED: audit, source index, decision report, public repo created with accurate
  conditional wording (no final GPL claim in README/site).
- REQUIRES OWNER APPROVAL: final license selection, CDLA risk, copyright statement, adding LICENSE.
- BLOCKED: final GPL representation until owner approval and implementation.
