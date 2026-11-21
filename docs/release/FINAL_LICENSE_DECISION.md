# Final License Decision Report — SwiftFetch

**Created:** 2026-11-21. **Lane:** license-finalization-researcher.
**Audience:** the project owner. **This is a decision report, not legal advice and not an approval.**

**Inputs (read-only):** `docs/release/LICENSE_AUDIT.md` (verdict: CONDITIONALLY APPROVED —
GPL-3.0-or-later), `docs/release/UPSTREAM_LICENSE_TEXTS.md`, `THIRD_PARTY_NOTICES.md`,
`docs/release/LEGAL_PRIVACY_CHECKLIST.md`, `docs/release/GO_NO_GO_REPORT.md`,
`docs/release/RELEASE_GAP_REGISTER.md`, `CONTRIBUTING.md`.

---

## 1. Recommendation (answer first)

**Final owner approval: NO — do not finalize the license yet.**

Rationale:

1. **The project license has not been selected.** No root `LICENSE` file exists; the choice of
   GPL-3.0-or-later (vs. an alternative) is explicitly an owner decision
   (`LEGAL_PRIVACY_CHECKLIST.md` §1.1). Nothing in the audit can select it.
2. **Four audit conditions are still open** (`LICENSE_AUDIT.md` §7):
   (a) the seven missing license texts are identified and their sources recorded in
   `UPSTREAM_LICENSE_TEXTS.md`, but they are **not yet copied into the notices bundle or installer**;
   (b) MPL-2.0 notice/source-offer obligations are defined but their execution (bundling, and the
   §3.3 GPL offer *only if* MPL files are modified) is pending;
   (c) the `CDLA-Permissive-2.0` uncertainty on `webpki-roots` 0.26.11 / 1.0.11 is **unresolved** —
   no formal FSF GPL-compatibility determination is known for this permissive *data* license, and this
   report does **not** assume compatibility;
   (d) the GPLv3 text itself is not yet shipped.
3. **Regenerate against the final locked dependency set** before closing (`RELEASE_PLAN.md` §1.3) —
   the 338-package inventory is a candidate-build snapshot.

**NO is a "not yet", not a "no".** The audit found **no blocking incompatibility** (no proprietary,
no strong-copyleft, no Apache-2.0-vs-GPL-2.0-only conflict for a GPL-3.0-or-later target). The path
to YES is the checklist in §7.

---

## 2. Copyright ownership

- **[OWNER: to be confirmed by the project owner.]** The audited sources do not state the copyright
  holder of SwiftFetch. A root `LICENSE` file requires an explicit copyright line
  (e.g. `Copyright (C) <year> <holder>`); this report deliberately does not invent one.
- Third-party copyrights remain with their holders; the seven upstream license texts
  (`UPSTREAM_LICENSE_TEXTS.md`) include holder-specific copyright lines that must be transcribed
  verbatim, not invented.

## 3. License selection status

| Item | Status |
| --- | --- |
| Owner decision (GPL-3.0-or-later vs. alternative) | **Open — owner decision** |
| Root `LICENSE` file with GPLv3 text | **Missing** |
| Technical compatibility of the dependency set | **CONDITIONALLY APPROVED — GPL-3.0-or-later** (`LICENSE_AUDIT.md` §7) |
| Blocking incompatibilities found | **None** |
| Open uncertainties | CDLA-Permissive-2.0 (2 entries); MPL-2.0 conditions execution |

## 4. Third-party notices & source distribution obligations

1. **Third-party notices.** `THIRD_PARTY_NOTICES.md` + per-package license texts must be bundled with
   the installer (not only in candidate snapshots). Seven texts are missing from the current bundle —
   canonical sources now recorded in `UPSTREAM_LICENSE_TEXTS.md`.
2. **GPLv3 §6 source offer** (only if GPL-3.0-or-later is selected): distribute the corresponding
   source of the GPL-covered work (the project's own code; no audited dependency is strong copyleft)
   with a written offer or download location.
3. **MPL-2.0 (5 crates: `cssparser`, `cssparser-macros`, `dtoa-short`, `option-ext`,
   `selectors`).** File-level copyleft, §3.3-compatible with GPLv3 **if**: the crates are not marked
   "Incompatible With Secondary Licenses" (verified: they use the standard Exhibit A header), the
   MPL text and per-file notices ship, and **if any MPL file is modified**, the modified files remain
   under MPL and are offered in source; no modifications were observed in the audit. §3.3 also
   requires the MPL source to be **additionally** distributed under GPLv3 for a Larger Work — confirm
   the mechanism (e.g. accompanying GPL notice) with counsel before shipping.
4. **CDLA-Permissive-2.0 (`webpki-roots`).** Obligation regardless of the compatibility question:
   ship the agreement text with the data (https://cdla.dev/permissive-2-0/). The *compatibility*
   question itself needs **owner/counsel or copyright-holder confirmation** — stated as unresolved,
   not assumed.
5. **Permissive licenses (MIT/Apache-2.0/BSD/ISC/Zlib/Unicode/BSL).** Reproduce copyright notice,
   license text and disclaimer; Apache-2.0 additionally requires preserving NOTICE files where
   present and stating changes.

## 5. Contributor licensing expectations

- Current inbound/outbound position is **not documented as settled**: `CONTRIBUTING.md` and the
  repository contain **no CLA or DCO configuration** that this lane located. **[OWNER/COUNSEL: decide
   and record whether contributions require a CLA, DCO sign-off, or inbound=outbound terms]** before
   accepting external contributions against a GPLv3 project — this is required for the owner to
  assert sole relicensing/control rights later.
- No contributor agreements were invented here; the decision is flagged, not made.

## 6. Distribution obligations summary (if GPL-3.0-or-later is selected)

| Obligation | Scope | Status |
| --- | --- | --- |
| GPLv3 LICENSE text + copyright line | project | Missing — owner to add |
| Written source offer (GPLv3 §6) | project code (+ any GPL components; none found) | Not set up |
| Third-party notices bundle | 338-package inventory | Pending — 7 texts outstanding |
| MPL-2.0 text + per-file notices; modified-file source offer if any | 5 crates | Not bundled |
| CDLA-Permissive-2.0 agreement text shipped with the data | `webpki-roots` ×2 | Not bundled; compatibility unresolved |
| Dev-only npm tooling | excluded (not redistributed) | N/A |

## 7. Path to YES (owner checklist)

1. Owner selects the project license and adds the root `LICENSE` with copyright line (§2).
2. Bundle the seven upstream texts per `UPSTREAM_LICENSE_TEXTS.md`; regenerate notices; confirm the
   bundle ships in the installer.
3. Bundle MPL-2.0 + CDLA texts; confirm no MPL file is modified (or set up the §3.1/§3.2/§3.3
   mechanism).
4. Obtain **copyright-holder / counsel confirmation** for CDLA-Permissive-2.0 (2 entries), or record
   an owner risk acceptance in this document.
5. Run §5 contributor-licensing decision.
6. Regenerate the dependency inventory against the final locked set (`RELEASE_PLAN.md` §1.3) and
   re-verify.

Each item above is a fact-gathering or decision task; **none of them is decided in this report**.

---

## Verdict restated

- Audit verdict stands: **CONDITIONALLY APPROVED — GPL-3.0-or-later** (technical compatibility).
- Final approval for release under a chosen license: **NO** until §7 is complete.
- This document selects nothing and clears nothing; it records status and hands the open decisions to
  the owner.