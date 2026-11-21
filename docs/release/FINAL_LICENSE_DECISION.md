# Final License Decision Report — GPL-3.0-or-later

**Date:** 2026-11-21 · **Version:** 0.1.0 · **Status:** PREPARED — REQUIRES OWNER APPROVAL

This report consolidates `docs/release/LICENSE_AUDIT.md`. It does **not** grant a license
or add a root `LICENSE` file — that requires the owner's explicit final decision.

## 1. Owner decisions on record
- Publisher: individual / personal publisher (no legal name provided).
- Software model: open source, free download for everyone, zero budget.
- Proposed license: GPL-3.0-or-later — **conditional** in the audit.
- No publisher identity, email, username, or domain has been supplied.

## 2. Audit outcome (from LICENSE_AUDIT.md)
- 338 dependencies (333 Cargo + 5 npm).
- No proprietary, `LicenseRef`, AGPL/SSPL, or GPL-2.0-only dependency.
- No Apache-2.0 vs GPL-2.0-only conflict for a GPL-3.0-or-later target.
- Verdict: **CONDITIONALLY APPROVED — GPL-3.0-or-later**.

## 3. The two CDLA-Permissive-2.0 dependencies (`webpki-roots`)
- Exact versions: `webpki-roots 0.26.11` and `1.0.11`.
- Declaration: CDLA-Permissive-2.0 — a permissive **data** license covering the
  bundled CA certificate data, with no copyleft and no FSF GPL-compatibility
  determination located. No formal legal/copyright-holder confirmation obtained.
- Conclusion: **UNCERTAIN — requires owner/counsel confirmation**. The condition in
  the audit is to ship the CDLA agreement text with the data. Do not assume
  compatibility merely because the license is permissive.
- Do not alter dependency versions merely to make the audit pass.

## 4. The seven missing license-text entries
`alloc-stdlib 0.3.0` (BSD-3-Clause), `clipboard-win 5.4.3` (BSL-1.0),
`selectors 0.38.0` (MPL-2.0), `tauri-plugin 2.7.2` (Apache-2.0 OR MIT),
`webview2-com 0.39.1`, `webview2-com-macros 0.8.1`, `webview2-com-sys 0.39.1`
(all MIT). SPDX declarations are valid and GPLv3-compatible; the published
tarballs omit the license text. See `docs/release/UPSTREAM_LICENSE_TEXTS.md` for
canonical upstream sources. Include these texts in distributed notices.

## 5. MPL-2.0 obligations (5 dependencies)
`cssparser`, `cssparser-macros`, `dtoa-short`, `option-ext`, `selectors` —
compatible via MPL §3.3 "Larger Work" using standard Exhibit A headers; modified
MPL files must remain MPL and be offered in source. These obligations attach to
the MPL files themselves and do not force the whole work under MPL.

## 6. Owner decision report

| Question | Answer |
|---|---|
| Is GPL-3.0-or-later appropriate? | Yes, conditionally — it matches the project's local-first, copyleft distribution intent and no dependency blocks it. |
| Can the final LICENSE be added now? | **No** — owner must first confirm the CDLA uncertainty and accept the conditions. |
| Has compatibility been finally approved? | **No** — audit is CONDITIONAL, not final. |
| Who owns copyright to project code? | [OWNER: state copyright owner / years; no attribution invented]. |
| What about contributor licensing? | Contributors retain copyright to their contributions and license them under the project license (document in CONTRIBUTING.md once license final). |
| Source distribution obligations | Provide Corresponding Source with any binary distribution; keep third-party notices and license texts in each release artifact. |

## 7. Recommendation
**RECOMMEND final owner approval ONLY after** the CDLA-Permissive-2.0 uncertainty is
acknowledged and the 7 upstream texts + MPL source-offer notes ship with releases.
Until then, preserve **CONDITIONAL** status. Do not represent the project as
finally GPL-licensed, and do not add the root `LICENSE` file.

## 8. Status
- COMPLETED: dependency inventory, 7 missing-text resolution, MPL verification, CDLA identification.
- REQUIRES OWNER APPROVAL: final license choice, CDLA risk acceptance, copyright statement, adding root `LICENSE`.
- BLOCKED: final GPL licensing representation until owner approval and implementation.
