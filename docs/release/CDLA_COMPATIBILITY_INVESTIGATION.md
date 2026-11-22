# CDLA-Permissive-2.0 / GPL-3.0-or-later Compatibility Investigation

**Date verified:** 2026-11-22 · **Status:** investigation COMPLETED; owner decisions recorded (CDLA Option A, GPL-3.0-or-later, residual uncertainty explicitly accepted). No formal legal determination claimed.
**Scope:** read-only research. No LICENSE created, no dependency changed, no legal approval claimed.

## 1. Owner decisions and constraints (context for this report)

- Owner decision recorded: adopt **GPL-3.0-or-later** (standard terms, commercial use allowed,
  no extra notification/permission/payment), copyright year **2026**.
- Copyright holder supplied by the owner: **Ali Ahasan**.
- The owner chose **CDLA Option A** and **explicitly accepted the residual uncertainty**
  documented in §6.
- GPL-3.0-or-later license implementation was subsequently **authorized and completed** by the
  owner (root `LICENSE`, CDLA notice, third-party notices). No extra permission, payment, or
  notification requirement was added.
- No dependency changes without owner approval.
- This report decides nothing legally. Compatibility remains **strongly indicated but not
  formally confirmed**; no legal approval or formal determination is claimed.

## 2. The exact dependency and license in question

| Fact | Evidence (verified 2026-11-22) |
|---|---|
| `webpki-roots 0.26.11` and `1.0.11` are in the locked build | `src-tauri/Cargo.lock` (checksums `521bc38a…` and `f5f280bb…`) |
| Both declare `CDLA-Permissive-2.0` | `artifacts/dependencies/notices-*/sbom.spdx.json`: `licenseDeclared: CDLA-Permissive-2.0`, `licenseConcluded: NOASSERTION`, `copyrightText: NOASSERTION` |
| Sole reverse dependency is `ureq` (HTTP client) | `src-tauri/Cargo.lock` package graph scan; `webpki-roots 0.26.11` depends on `webpki-roots 1.0.11`; this is how the CA root data reaches the shipped binary via rustls/webpki |
| Upstream describes CA certificate **data** as CCADB data under CDLA-2.0-Permissive; its **tooling** is MIT/Apache-2.0 | https://github.com/rustls/webpki-roots (verified 2026-11-22) |

This is a **data license** applied to compiled CCADB root-certificate data, not to linked
program code. Observation only: `webpki-root-certs 1.0.12` is present in `Cargo.lock` but is
not in the SBOM's declared package set; no conclusion is drawn here and no change is proposed.

## 3. Exact license text conditions (verbatim)

Sources: https://spdx.org/licenses/CDLA-Permissive-2.0.html and
https://cdla.dev/permissive-2-0/ (canonical text; verified 2026-11-22).
SPDX short identifier: `CDLA-Permissive-2.0`; full name: "Community Data License Agreement
Permissive 2.0"; there is no standard license header.

- **§1.1:** "A Data Recipient may use, modify, and share the Data made available by Data
  Provider(s) under this agreement if that Data Recipient follows the terms of this agreement."
- **§1.2:** no restriction on portions of the Data that are public domain or usable under
  another legal exception or limitation.
- **§2.1 (the only substantive condition):** "A Data Recipient may share Data, with or without
  modifications, so long as the Data Recipient makes available the text of this agreement with
  the shared Data."
- **§3.1:** "This agreement does not impose any restriction or obligations with respect to the
  use, modification, or sharing of Results."
- **§4.1:** standard "AS IS" warranty disclaimer and broad limitation of liability.
- **§5.1:** "'Data' means the material received by a Data Recipient under this agreement."
- **§5.4:** "'Results' means any outcome obtained by computational analysis of Data, including
  for example machine learning models and models' insights."

Naming variance to be aware of (same license, different label): CCADB and the
`webpki-roots` README call it **CDLA-2.0-Permissive**; the SPDX identifier is
**CDLA-Permissive-2.0**. This matters for SBOM matching, not for the legal analysis.

## 4. Official compatibility evidence

- **CDLA's own compatibility page** — https://cdla.dev/faq-resources/compatibility/
  (page last updated 2021-09-22; verified 2026-11-22): addresses only **CC0-1.0 and CC-BY-4.0**
  data licenses. It explicitly states compatibility "is ultimately a matter of license text
  interpretation", that the page "is not intended as legal advice", and that readers "will
  likely want to consult with your own legal counsel." **It does not mention GPLv3 at all.**
  Note: the FAQ URL linked from that page (`https://cdla.dev/faq/`) returned **HTTP 404** at
  verification time, so the expanded FAQ text could not be read; only the live compatibility
  page is evidence here.
- **CCADB Data Usage Terms** — https://www.ccadb.org/rootstores/usage (verified 2026-11-22):
  the data is licensed under CDLA-2.0-Permissive; **(2) "Attribution to the Common CA
  Database (CCADB) is required when redistributing or using this data in derived works"**;
  **(3) the names and trademarks of CCADB or contributors may not be used to endorse or
  promote products derived from this data without prior written permission.**
- **webpki-roots upstream** — https://github.com/rustls/webpki-roots (verified 2026-11-22):
  "The underlying data is from Common CA Database (CCADB) and is used under the
  CDLA-2.0-Permissive license… The data in `webpki-roots` and `webpki-root-certs` is a derived
  work of the CCADB data." and "The tooling in `webpki-ccadb` is licensed under both MIT and
  Apache licenses."
- **FSF "Various Licenses and Comments about Them"** —
  https://www.gnu.org/licenses/license-list.en.html (verified 2026-11-22): the complete page
  was fetched and searched; **CDLA-Permissive-2.0 does not appear in any section** (neither the
  GPL-compatible list nor the GPL-incompatible list — nor any other section). **No formal FSF
  classification or determination exists for this license.** The FSF also states its page "is
  not legal advice". Absence of a listing is *not* an adverse finding: the FSF page itself says
  it "cannot list them all" and that it answers questions about licenses whether or not they
  are listed.
- **SPDX** (verified 2026-11-22): lists the identifier and canonical text and nothing else; it
  makes no GPL-compatibility assertion. The SBOM's `licenseConcluded: NOASSERTION` is
  consistent with there being no authoritative determination to conclude.

## 5. Analysis: does the single §2.1 condition conflict with GPLv3?

The only substantive CDLA condition (§2.1) is an **attribution/notice-style obligation**:
make the agreement text available with the shared Data. It does not restrict use,
modification, distribution, fields of endeavour, or downstream relicensing; §3.1 explicitly
disclaims any restriction on Results. Structurally this is closer to permissive notice
preservation (MIT/BSD/Apache notice conditions) than to a copyleft or use restriction.

Against the GPLv3 text:

- **GPLv3 §10** bars imposing *further restrictions* on the rights GPLv3 grants recipients.
  CDLA imposes no restriction on what anyone may do with the Data or with Results; the §2.1
  obligation runs against the **distributor** (ship a notice with the data you share), not
  against recipients' rights. It is satisfiable alongside GPLv3 compliance, exactly the way
  preserving MIT/BSD notices is, so it does not operate as a "further restriction".
- **GPLv3 §7** governs additional terms on top of the GPL; the CDLA condition attaches to
  *data*, and nothing in the CDLA text conflicts with GPLv3 conveyance requirements (source
  offer, corresponding source, installation information). Distributing the CDLA text with the
  notices adds to what GPLv3 already requires of us rather than taking anything away.
- **Scope point specific to SwiftFetch:** the licensed material is **data** (compiled CCADB
  root certificates) reached transitively via `ureq`, not linked library code, so the GPLv3
  "single combined work" linking analysis does not squarely apply. Upstream's own tooling is
  MIT/Apache-2.0 (both GPLv3-compatible per the FSF list).
- **CCADB items** (attribution; no trademark/endorsement use) are provider-side obligations
  fulfilled at distribution time — ship the CDLA-Permissive-2.0 text plus a CCADB attribution
  line in the notices, and do not imply CCADB endorsement in naming or marketing. They are
  not restrictions on end-user rights under the GPL. (Trademark-permission clauses are
  generally not read as further restrictions on a copyright license — that is a common
  interpretation, e.g. the treatment of the Apache-2.0 trademark clause, not a formal ruling
  for CDLA.)
- **Practical obligation regardless of the GPL question:** because the shipped binary contains
  derived CCADB data, distributing SwiftFetch triggers CDLA §2.1. Shipping the CDLA text with
  the notices satisfies the license either way and is required in all scenarios.

## 6. Residual uncertainty (stated honestly)

- There is **no formal FSF determination** and **no statement from CDLA** about GPLv3
  compatibility.
- CDLA's own compatibility guidance says to **consult legal counsel** and disclaims being
  legal advice; the linked FAQ page was dead (404) at verification time.
- The CCADB usage terms add an attribution characterization and a trademark clause beyond the
  bare §2.1 text; their precise interaction with GPLv3 conveyance is interpretation, not
  adjudication.
- This is a **data license used for compiled CA certificate data, not linked code**, so
  ordinary software license-compatibility lists are not directly on point.
- Net: compatibility is **strongly indicated by the license text and structure, but not
  formally confirmed**. This report does not convert it into legal approval.

## 7. Conclusion (direct answer)

**Yes — compatibility can be treated as sufficiently established for the purpose of
proceeding, on the documentary evidence, on these express conditions:** (a) the distribution
carries the full CDLA-Permissive-2.0 text plus CCADB attribution in the notices (an
obligation owed regardless of the GPL question), and (b) the owner explicitly accepts the
residual uncertainty documented in §6. **No — it is not legally confirmed:** no formal FSF
determination exists, CDLA itself recommends counsel, and the CCADB trademark/attribution
terms are not formally adjudicated against GPLv3. The owner has recorded the decision to
proceed under CDLA Option A with the residual uncertainty of §6 explicitly accepted and
supplied the copyright holder (Ali Ahasan); the GPL-3.0-or-later license implementation was
then authorized and completed. Nothing in this section should be read as a formal legal
approval. The external confirmations in §8 remain the only route to formal closure, and the
residual caveats in §6 remain in force.

## 8. Exactly what additional owner/external information is needed

**From the owner — status recorded:**

1. Explicit acceptance of the residual uncertainty in §6 — **RESOLVED: owner chose CDLA Option A
   and explicitly accepted the documented residual uncertainty.**
2. The copyright holder's **legal name** for the 2026 copyright line — **RESOLVED: owner supplied
   Ali Ahasan; year 2026 confirmed.** The name was owner-supplied, not invented here.
3. Confirmation that notices/distribution include (a) the full CDLA-Permissive-2.0 text and
   (b) CCADB attribution — the authorized license implementation added `CDLA-PERMISSIVE-2.0.txt`
   and `THIRD_PARTY_NOTICES.md`. No separate owner statement is recorded in this investigation.
4. Confirmation that product naming and marketing do not imply CCADB endorsement (CCADB usage
   terms, point 3) — retained as a caveat; no separate owner statement is recorded here.
5. A note/decision on the observation that `webpki-root-certs 1.0.12` is in `Cargo.lock` but
   absent from the SBOM's declared package set (SBOM scope only; no dependency change implied) —
   retained as a caveat; no dependency change was made.

**External (any one of these closes the formal gap):**

6. Written confirmation from the data provider (CCADB / Linux Foundation, per the CCADB usage
   terms) that distributing CDLA-Permissive-2.0 data inside a GPL-3.0-or-later application
   requires only §2.1 notice compliance.
7. A written opinion from the owner's legal counsel addressing (a) the §2.1 notice condition
   against GPLv3 §§7/10, and (b) the CCADB attribution/trademark terms.
8. A formal FSF determination — the FSF answers license questions whether or not a license is
   listed, but this is realistically not a viable release-gate path and is listed only for
   completeness.

## 9. What this investigation did NOT do

- This investigation lane created no LICENSE file. The separately authorized license implementation
  created the root `LICENSE`, CDLA notice and third-party notices; NO-GO is unchanged.
- No dependency was changed; `webpki-roots 0.26.11` / `1.0.11` and `webpki-root-certs 1.0.12`
  are untouched.
- This investigation claims no legal approval or formal compatibility determination.
- Copyright holder identity was supplied separately by the owner, not derived from this report.

## Sources (all verified 2026-11-22)

| Source | URL |
|---|---|
| CDLA-Permissive-2.0 full text | https://cdla.dev/permissive-2-0/ |
| CDLA compatibility (last updated 2021-09-22; CC0/CC-BY only; not legal advice) | https://cdla.dev/faq-resources/compatibility/ |
| SPDX license entry (identifier, canonical text) | https://spdx.org/licenses/CDLA-Permissive-2.0.html |
| FSF license list (no CDLA listing; no formal determination) | https://www.gnu.org/licenses/license-list.en.html |
| webpki-roots (CCADB data under CDLA-2.0-Permissive; tooling MIT/Apache) | https://github.com/rustls/webpki-roots |
| CCADB Data Usage Terms (CDLA-2.0-Permissive; attribution; trademark) | https://www.ccadb.org/rootstores/usage |
| Local evidence | `src-tauri/Cargo.lock`; `artifacts/dependencies/notices-*/sbom.spdx.json` |