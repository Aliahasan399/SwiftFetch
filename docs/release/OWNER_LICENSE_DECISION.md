# Owner Licensing Decision — SwiftFetch

**Recorded:** 2026-11-22 · **Status:** owner decision recorded; implementation BLOCKED on two unresolved items

## Official owner decision (verbatim intent)

| Item | Owner decision |
|---|---|
| License | Adopt the **standard GNU GPL-3.0-or-later** license. |
| Scope | Commercial use, modification, and distribution are permitted under the **standard GPL terms**. |
| Extra conditions | **No** additional requirement to notify the owner, request permission, or pay for commercial use. |
| Copyright year | Proposed: **2026**. |
| Copyright holder | **Do not invent the legal name.** Owner must supply it. |
| LICENSE file | **Do not create or publish** the final `LICENSE` until (a) the CDLA-Permissive-2.0 compatibility question and (b) copyright ownership information are resolved. |
| Dependencies | **Do not modify or replace** dependencies without owner approval. |

## What this changes
- The license choice is no longer open. The earlier "owner has not selected a license"
  blocker (D-01 / LEGAL_COMPLIANCE_READINESS.md) is now **resolved as a decision**,
  but its **implementation** (root LICENSE) remains blocked by the two items above.
- No extra commercial-use restrictions are added beyond standard GPLv3 terms —
  consistent with GPLv3 §0/§2 and no additional terms under §10.

## What does NOT change
- `ready_for_public_release: false` and the NO-GO verdict — signing, updater,
  clean-machine, accessibility, soak, and startup gates are still open.
- The public repository's README/website must keep saying the license is proposed
  until the root `LICENSE` is actually added after both blockers clear.
- `THIRD_PARTY_NOTICES.md` and the 7 missing license texts still need to ship.

## Blocking items before LICENSE can be created
1. **CDLA-Permissive-2.0 compatibility** for `webpki-roots` 0.26.11 / 1.0.11 —
   under investigation (`CDLA_COMPATIBILITY_INVESTIGATION.md`).
2. **Copyright holder legal name** — owner-supplied; no name invented here.
   Proposed year 2026 is recorded; the holder line remains `[OWNER: copyright holder legal name]`.

## Owner actions still required
- Supply the copyright holder's legal name (or confirm the exact form to use).
- Decide whether to accept the CDLA risk based on the investigation, or seek
  counsel/copyright-holder confirmation, or approve a dependency change (the owner
  has not approved any dependency change; none was made).

No LICENSE file was created by this decision record.
