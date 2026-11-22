# CDLA-Permissive-2.0 / GPL-3.0-or-later Compatibility Investigation

**Date verified:** 2026-11-22 · **Status:** investigation COMPLETED; owner decision still required
**Scope:** read-only research. No LICENSE created, no dependency changed, no legal approval claimed.

## 1. The exact dependency and license in question
| Fact | Evidence |
|---|---|
| `webpki-roots 0.26.11` and `1.0.11` are in the locked build | `src-tauri/Cargo.lock` (checksums `521bc38a…` / `f5f280bb…`) |
| Both declare `CDLA-Permissive-2.0` | `artifacts/dependencies/notices-*/sbom.spdx.json` (`licenseDeclared: CDLA-Permissive-2.0`, `licenseConcluded: NOASSERTION`) |
| Upstream describes CA certificate **data** as CCADB data under CDLA-2.0-Permissive; its **tooling** is MIT/Apache | https://github.com/rustls/webpki-roots (verified 2026-11-22) |

This is a **data license** applied to compiled Mozilla/CCADB root-certificate data, not
to linked program code.

## 2. Official license text (SPDX, verified 2026-11-22)
Source: https://spdx.org/licenses/CDLA-Permissive-2.0.html

- **§2.1 (the only substantive condition):** a recipient may share Data, with or
  without modifications, "so long as the Data Recipient makes available the text of
  this agreement with the shared Data."
- **§3.1:** "This agreement does not impose any restriction or obligations with
  respect to the use, modification, or sharing of Results."
- §1.2: no restriction on public-domain portions.
- §4: standard AS-IS warranty/liability disclaimer.

## 3. What the official compatibility sources say
- **CDLA's own compatibility page** (https://cdla.dev/faq-resources/compatibility/,
  last updated 2021-09-22, verified 2026-11-22): addresses only **CC0-1.0 and
  CC-BY-4.0** data licenses. It explicitly says compatibility "is ultimately a
  matter of license text interpretation", that the page "is not intended as legal
  advice", and that readers "will likely want to consult with your own legal counsel."
  **It does not mention GPLv3.**
- **FSF "Various Licenses and Comments about Them"**
  (https://www.gnu.org/licenses/license-list.en.html, verified 2026-11-22): the
  GPL-compatible and GPL-incompatible lists do **not** include CDLA-Permissive-2.0.
  No formal FSF determination exists. FSF states its page "is not legal advice".
- **SPDX** lists the identifier and text; it makes no GPL-compatibility assertion.

## 4. Analysis
The single CDLA condition (§2.1) is an **attribution/notice-style obligation**: make
the agreement text available with the shared data. It does not restrict use,
modification, distribution, fields of endeavour, or downstream relicensing, and §3.1
explicitly disclaims restrictions on Results. Structurally this is closer to a
permissive notice condition (e.g. MIT/BSD notice preservation) than to a
copyleft or use restriction that would conflict with GPLv3 §10.

Two mitigating facts specific to SwiftFetch:
1. The licensed material is **data** (CA root certificates), not linked library code;
   the GPLv3 "combined work" analysis for linked code does not squarely apply.
2. The crate's own tooling is MIT/Apache-2.0 (both GPLv3-compatible per FSF).

## 5. Residual uncertainty (must be stated)
- There is **no formal FSF ruling** and **no statement from CDLA** about GPLv3.
- CDLA itself recommends consulting legal counsel.
- Therefore compatibility is **strongly indicated by the license text and structure**,
  but **not formally confirmed**. This report does not convert it into legal approval.

## 6. Conclusion and options
- **Option A (recommended, zero cost):** treat compatibility as sufficiently established
  on the text/structure evidence, document the residual risk, and **ship the CDLA
  agreement text with the notices** (§2.1 condition) — this also satisfies the
  distribution obligation regardless of the GPL question. Owner accepts residual risk.
- **Option B:** seek confirmation from the copyright holder (CCADB / Mozilla) or legal
  counsel before finalizing.
- **Option C:** owner-approved dependency change/removal — **not done**, and the owner
  has explicitly forbidden dependency changes without approval.

## 7. Owner information still required
1. Copyright holder's **legal name** for the 2026 copyright line (do not invent).
2. A decision among Options A / B / C above.
3. If Option A: explicit acceptance of the documented residual uncertainty.

No LICENSE file was created by this investigation. NO-GO unchanged.
