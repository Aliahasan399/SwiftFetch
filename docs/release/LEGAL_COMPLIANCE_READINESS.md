# Legal Compliance Readiness — Master Prompt Phase 8

**Date:** 2026-11-21 · **Status:** PREPARED — license and legal decisions BLOCKED_OWNER

## Dependency inventory
338 packages (333 Cargo + 5 npm) audited in `LICENSE_AUDIT.md`. No proprietary,
LicenseRef, AGPL/SSPL, or GPL-2.0-only dependency; no Apache-2.0/GPL-2.0-only
conflict for a GPL-3.0-or-later target. OpenAI-free, no paid services.

## Redistribution obligations
| Obligation | Status | Evidence |
|---|---|---|
| Reproduce copyright + license text for permissive deps | PARTIAL — 7 texts missing | LICENSE_AUDIT.md §3, UPSTREAM_LICENSE_TEXTS.md |
| MPL-2.0 (5 crates) §3.3 source availability | PREPARED, execution pending | FINAL_LICENSE_DECISION.md §4 |
| CDLA-Permissive-2.0 agreement text (webpki-roots x2) | UNCERTAIN, ship text | FINAL_LICENSE_DECISION.md §6 |
| GPLv3 Corresponding Source offer (if GPL chosen) | PENDING owner selection | LICENSE_AUDIT.md §5 |
| Third-party notices bundled with installer | PENDING | THIRD_PARTY_NOTICES.md |
| NOTICE files preserved (Apache-2.0) | identified | dependency notices |

## Seven known gaps (canonical sources recorded)
`alloc-stdlib 0.3.0` BSD-3-Clause; `clipboard-win 5.4.3` BSL-1.0;
`selectors 0.38.0` MPL-2.0; `tauri-plugin 2.7.2` Apache-2.0 OR MIT;
`webview2-com 0.39.1`, `webview2-com-macros 0.8.1`, `webview2-com-sys 0.39.1`
(all MIT). Sources in `UPSTREAM_LICENSE_TEXTS.md`; texts NOT yet bundled.

## CDLA uncertainty (unresolved)
`webpki-roots` 0.26.11 / 1.0.11 use CDLA-Permissive-2.0, a permissive data
license with no formal FSF GPL-compatibility determination. Compatibility is
**not assumed**; owner/counsel or copyright-holder confirmation required.

## Legal drafts (all OWNER APPROVAL REQUIRED)
- Root `LICENSE` proposal: GPL-3.0-or-later — NOT added; conditional.
- `docs/TERMS-DRAFT.md`, `docs/PRIVACY.md`, `docs/SUPPORT.md`, `SECURITY.md` — drafts; no publisher identity/contact invented.
- `THIRD_PARTY_NOTICES.md` — preliminary, references generated notices.

## Readiness table
| Item | Status |
|---|---|
| Dependency inventory | COMPLETED |
| Compatibility audit | COMPLETED (conditional) |
| 7 upstream text sources | COMPLETED |
| Upstream texts bundled | BLOCKED_OWNER / PENDING |
| CDLA resolution | BLOCKED_OWNER |
| Final license decision | BLOCKED_OWNER |
| Publisher identity/contact | BLOCKED_OWNER |
| Terms/Privacy approval | BLOCKED_OWNER |

No legal approval claimed. Public repo accurately describes license as proposed.
