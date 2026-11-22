# SwiftFetch — Legal & Compliance Readiness (Phase 14)

**Created:** 2026-11-22. **Version:** 0.1.0. **Lane:** compliance-agent (read-only; this document only).

**Purpose.** Consolidate the legal/compliance state of SwiftFetch ahead of public release, covering the
ten Phase-14 tasks. **This document makes no legal decisions, grants no approval, gives no legal
advice, and invents no publisher, contact, copyright-holder, or year values.** Every legal draft it
references is marked **OWNER APPROVAL REQUIRED**. It records status and hands open decisions to the
owner.

**Inputs (read-only):** `docs/release/LICENSE_AUDIT.md`, `FINAL_LICENSE_DECISION.md`,
`FINAL_LICENSE_APPROVAL_STATUS.md`, `UPSTREAM_LICENSE_TEXTS.md`, `LEGAL_PRIVACY_CHECKLIST.md`,
`THIRD_PARTY_NOTICES.md` (repo root), `docs/PRIVACY.md`, `docs/TERMS-DRAFT.md`,
`docs/SUPPORT.md`, `docs/SECURITY.md`, `README.md`.

**Top-line status: NOT READY — legal release gates open.** The technical dependency audit is
*conditionally* compatible with a GPL-3.0-or-later target, but the project license is **not
selected**, the root `LICENSE` must stay **absent**, and Terms/Privacy/Support drafts await owner
approval.

---

## 1. Phase-14 task ledger

| # | Phase-14 task | Status | Evidence |
| --- | --- | --- | --- |
| 1 | Identify direct / bundled / redistributed dependencies (338 inventory) | **Partial** | Inventory generated (see §2); candidate snapshot, must be regenerated against final locked deps (`RELEASE_PLAN.md` §1.3) |
| 2 | Catalog licenses of identified dependencies | **Done (technical)** | 338 declared SPDX expressions; `licenseConcluded: NOASSERTION` is a generator artifact, not unknown licenses |
| 3 | Record attribution / redistribution obligations | **Done (technical)** | `LICENSE_AUDIT.md` §5; conditions partly unexecuted (see §4, §5) |
| 4 | Record the seven known license-text gaps and canonical upstream sources | **Partial** | Gaps identified, sources verified and indexed in `UPSTREAM_LICENSE_TEXTS.md`; texts **not yet bundled** |
| 5 | Record CDLA-Permissive-2.0 uncertainty status | **Open — unresolved** | `webpki-roots` 0.26.11 / 1.0.11; no FSF determination, no counsel/holder confirmation (see §5) |
| 6 | License inventory summary | **Done** | §3 of this document; mirrors `THIRD_PARTY_NOTICES.md` |
| 7 | Owner decision document (license + publisher identity) | **Open — OWNER APPROVAL REQUIRED** | `FINAL_LICENSE_DECISION.md` verdict: do **not** finalize yet; `LEGAL_PRIVACY_CHECKLIST.md` §1.1, §2.1–2.5 |
| 8 | Root license proposal — proposed, NOT approved; root `LICENSE` stays absent | **Enforced** | `FINAL_LICENSE_APPROVAL_STATUS.md`: root `LICENSE` intentionally absent; no file added by this lane |
| 9 | Third-party notices (bundle completeness) | **Partial** | `THIRD_PARTY_NOTICES.md` PRELIMINARY; 7 texts outstanding; installer bundling unconfirmed (see §6) |
| 10 | Terms / Privacy / Support drafts — approval state | **Open — OWNER APPROVAL REQUIRED** | All three docs are drafts with owner-decision placeholders; none approved (see §7) |

---

## 2. Dependency identification (task 1)

| Ecosystem | Count | Source of truth |
| --- | --- | --- |
| cargo (direct + transitive, incl. build/dev/test) | 333 | `Cargo.lock`, `artifacts/dependencies/cargo-windows.json`, generated `dependencies.json` |
| npm (runtime, locked) | 5 | `package-lock.json`, `artifacts/dependencies/npm-runtime.spdx.json` |
| **Total** | **338** | matches `LEGAL_PRIVACY_CHECKLIST.md` §1.3, `RELEASE_GAP_REGISTER.md:65` |

- **Direct Rust deps** (`src-tauri/Cargo.toml`): `tauri`, `tauri-plugin-opener`,
  `tauri-plugin-updater`, `tauri-plugin-clipboard-manager`, `serde`, `serde_json`, `tokio`,
  `futures-util`, `reqwest`, `rusqlite`, `sha2`, `uuid`, `chrono`, `thiserror`, `bytes`,
  `once_cell`, `url`, `sha1`, `md-5`; Windows target `windows-registry`; build `tauri-build`;
  dev `tauri`, `base64`, `tempfile`, `ureq`.
- **Direct npm runtime:** `react`, `react-dom`, `@tauri-apps/api`, `@tauri-apps/plugin-opener`.
- **Redistributed scope:** everything linked into the shipped installer is in scope. npm dev-only
  tooling (vite, typescript, @vitejs/plugin-react, @tauri-apps/cli, @types/*) is build-time only,
  not shipped, and carries no distribution obligations for the artifact.
- **Snapshot caveat:** the 338-entry inventory is a conservative candidate-build snapshot; it
  **must be regenerated against the final locked dependency set before release**.

## 3. License inventory summary (tasks 2, 3, 6)

Declared-license distribution (338 entries): MIT OR Apache-2.0 (151), MIT (70),
Apache-2.0 OR MIT (34), Unicode-3.0 (18), MIT/Apache-2.0 (11), Unlicense OR MIT (9), MPL-2.0 (5),
MIT OR Apache-2.0 OR Zlib (4), BSD-3-Clause (3), plus smaller groups (Zlib, ISC, BSL-1.0,
Apache-2.0, CC0-1.0 OR MIT-0 OR Apache-2.0, Unicode-3.0 AND (MIT OR Apache-2.0),
BSD-2-Clause OR Apache-2.0 OR MIT, CDLA-Permissive-2.0, etc.).

Compatibility verdict for a GPL-3.0-or-later target (`LICENSE_AUDIT.md` §4, §7):
**CONDITIONALLY APPROVED — GPL-3.0-or-later (technical finding only, not legal approval).**

- ~300 permissive Apache/MIT/BSD/ISC/Unicode/BSL/public-domain packages: compatible.
- MPL-2.0 (5 crates: `cssparser`, `cssparser-macros`, `dtoa-short`, `option-ext`, `selectors`):
  compatible **with conditions** (file-level copyleft, §3.3 Larger Work, standard Exhibit A headers).
- **No blocking incompatibility found:** no proprietary, `LicenseRef-*`, AGPL/SSSL, or
  GPL-2.0-only component; OpenSSL avoided (schannel/rustls used); static linking (LTO/crate types)
  creates no LGPL-style relinking obligation.

Attribution/redistribution obligations if GPL-3.0-or-later is selected:
per-crate copyright notice + license text + disclaimer; Apache-2.0 NOTICE preservation;
MPL-2.0 text + per-file notices + §3.1/§3.2 modified-file source offer if modified (none observed);
CDLA agreement text shipped with the data; GPLv3 §6 written source offer for the project's own code.

## 4. Seven known gaps and canonical upstream sources (task 4)

All seven are **text-shipping/attribution gaps, not license incompatibilities** — each declared a
valid SPDX expression in `Cargo.toml` but shipped no license file in the published crate tarball.
Canonical sources verified 2026-11-21 (`LICENSE_AUDIT.md` §3, `UPSTREAM_LICENSE_TEXTS.md` §1):

| # | Package | Version | Declared SPDX | Canonical upstream source |
| --- | --- | --- | --- | --- |
| 1 | `alloc-stdlib` | 0.3.0 | BSD-3-Clause | https://github.com/dropbox/rust-alloc-no-stdlib |
| 2 | `clipboard-win` | 5.4.3 | BSL-1.0 | https://github.com/DoumanAsh/clipboard-win |
| 3 | `selectors` | 0.38.0 | MPL-2.0 (per-file §1.4/Exhibit A headers) | https://github.com/servo/stylo ; https://www.mozilla.org/MPL/2.0/ |
| 4 | `tauri-plugin` | 2.7.2 | Apache-2.0 OR MIT | https://github.com/tauri-apps/tauri |
| 5 | `webview2-com` | 0.39.1 | MIT | https://github.com/wravery/webview2-rs |
| 6 | `webview2-com-macros` | 0.8.1 | MIT | https://github.com/wravery/webview2-rs |
| 7 | `webview2-com-sys` | 0.39.1 | MIT | https://github.com/wravery/webview2-rs |

Outstanding: copy each text **verbatim** (including the holder's own copyright line — no invented
holders) into the per-package notice directories, regenerate notices
(`node tools/dependency-notices.mjs`), and confirm the bundle ships in the final installer, not only
in candidate snapshots.

## 5. CDLA-Permissive-2.0 uncertainty — UNRESOLVED (task 5)

- Affects `webpki-roots` **0.26.11** and **1.0.11** (bundled CA certificate **data**).
- SPDX-listed permissive data license; no copyleft; **no formal FSF GPL-compatibility determination
  located**; **no legal/copyright-holder confirmation obtained**. Compatibility is **not assumed**
  by any source document and is **not assumed here**.
- Obligation applies regardless of the compatibility question: ship the CDLA agreement text with the
  data (https://cdla.dev/permissive-2-0/).
- Resolution path (owner): owner/counsel determination, copyright-holder confirmation, or a recorded
  owner risk acceptance. **This document does not resolve it.**

## 6. Owner decision document & root license proposal — NOT APPROVED (tasks 7, 8)

| Item | Status |
| --- | --- |
| Owner license decision (GPL-3.0-or-later vs. alternative) | **Open — OWNER APPROVAL REQUIRED** |
| Root `LICENSE` file with GPLv3 text | **Missing — and must stay missing until owner approval** |
| Project license proposal | **GPL-3.0-or-later proposed (README.md) — NOT approved** |
| Copyright holder / years | **[OWNER: to be confirmed] — nothing invented** |
| Technical compatibility of the dependency set | CONDITIONALLY APPROVED — GPL-3.0-or-later (technical only) |
| Blocking incompatibilities found | None |

`FINAL_LICENSE_APPROVAL_STATUS.md` records that the root `LICENSE` file is **intentionally absent**
so the public repository is never represented as finally GPL-licensed; `FINAL_LICENSE_DECISION.md`
records **"Final owner approval: NO — do not finalize the license yet."** The four open audit
conditions are: (a) bundle the seven texts; (b) execute MPL-2.0 notice/source-offer obligations;
(c) resolve the CDLA uncertainty; (d) ship the GPLv3 text. Plus regenerate the inventory against the
final locked set, and decide contributor-licensing (no CLA/DCO exists — decision flagged, not made).

Owner path to YES (from `FINAL_LICENSE_DECISION.md` §7, restated, none taken):
select license + add root `LICENSE`; bundle the 7 upstream texts and regenerate notices; bundle
MPL-2.0 + CDLA texts; confirm no MPL file modified (or set up §3.1/§3.2/§3.3); obtain CDLA
confirmation or record risk acceptance; decide CLA/DCO; regenerate and re-verify the inventory.

## 7. Third-party notices status (task 9)

`THIRD_PARTY_NOTICES.md` (repo root) is **PRELIMINARY**: full 338-row inventory, declared-license
distribution, the seven-gap table, MPL/CDLA conditions, and pointers to the generated artifacts
under `artifacts/dependencies/notices-a1587a6b-6e23-4a51-bf9b-493b0294a934/` (331 per-package license
text directories, `dependencies.json`, `sbom.spdx.json`). **Not final** until the seven texts are
bundled, the notices ship inside the installer
(`FINAL_BUILD_EVIDENCE.md:71` currently shows candidate-only bundling), and the owner approves the
attribution text (`LEGAL_PRIVACY_CHECKLIST.md` §1.5).

## 8. Terms / Privacy / Support drafts — OWNER APPROVAL REQUIRED (task 10)

All three documents are **drafts**. Nothing below has been approved, and no publisher name, privacy
contact, security contact, issue-tracker URL, or support policy has been invented or filled in.

| Document | State | Owner action required |
| --- | --- | --- |
| `docs/TERMS-DRAFT.md` | Draft (permitted use, no circumvention representation, speed/checksum/AV disclaimers) | Select distribution license; identify publisher; approve terms, warranty/support position, third-party attribution |
| `docs/PRIVACY.md` | Draft (local-only state, no telemetry upload claimed; clipboard opt-in; extension sends only the selected URL; explicitly claims **no** regulatory certification) | Identify publisher; provide privacy contact; choose support/retention procedures; re-review against the final release configuration |
| `docs/SUPPORT.md` | Draft (diagnostics, recovery, updates/installation guidance) | Publish supported versions, response expectations, issue-reporting location, private security contact |
| `docs/SECURITY.md` | Draft + incident-response table (never auto-execute; checksum/AV disclaimers; "not an independent security audit") | Provide the private vulnerability-reporting contact before public distribution |

---

## 9. Readiness status table

| Gate | Status |
| --- | --- |
| Dependency inventory (338) generated | **Partial** — candidate snapshot; regenerate against final locked deps |
| License catalog + compatibility analysis | **Done (technical)** — CONDITIONALLY APPROVED — GPL-3.0-or-later |
| Seven missing license texts identified + canonical sources recorded | **Done** |
| Seven missing license texts bundled into notices/installer | **Open** |
| CDLA-Permissive-2.0 (2 entries) compatibility | **Open — UNRESOLVED** |
| CDLA agreement text bundled with the data | **Open** |
| MPL-2.0 text + per-file notices bundled; §3.3 mechanism | **Open** |
| GPLv3 text + §6 source offer | **Open** (only if GPL selected) |
| Project license selected | **Open — OWNER APPROVAL REQUIRED** |
| Root `LICENSE` file | **Correctly absent — stays absent until owner approval** |
| Third-party notices finalized + shipped in installer | **Open** |
| Terms draft approved | **Open — OWNER APPROVAL REQUIRED** |
| Privacy notice approved | **Open — OWNER APPROVAL REQUIRED** |
| Support policy + contacts published | **Open — OWNER APPROVAL REQUIRED** |
| Security contact published | **Open — OWNER APPROVAL REQUIRED** |
| Publisher identity / contacts / support policy | **Open — OWNER APPROVAL REQUIRED** (nothing invented) |
| Contributor licensing (CLA/DCO) | **Open — decision flagged, not made** |
| Overall legal readiness | **NOT READY** — public release remains NO-GO while 1.1, 2.1–2.5, 4.4, 7.4 gates are open (`LEGAL_PRIVACY_CHECKLIST.md` §8) |

## 10. Unresolved decisions (all owner; none taken here)

1. **License selection** — GPL-3.0-or-later (proposed) vs. an alternative; root `LICENSE` stays
   absent until decided.
2. **Copyright holder and years** for the root `LICENSE` copyright line.
3. **CDLA-Permissive-2.0** — compatibility of `webpki-roots` 0.26.11 / 1.0.11: owner/counsel
   determination, copyright-holder confirmation, or recorded risk acceptance.
4. **MPL-2.0 §3.3 mechanism** — confirm the accompanying-GPL-notice mechanics with counsel before
   shipping (no MPL modifications observed; §3.1/§3.2 offer only if modified).
5. **Publisher identity, privacy contact, security contact, issue-tracker URL, support policy.**
6. **Approval of the Terms, Privacy, and Support drafts** against the final release configuration.
7. **Third-party attribution text approval** and installer bundling of notices.
8. **Contributor licensing** — CLA, DCO sign-off, or inbound=outbound terms before accepting
   external contributions against a GPLv3 project.
9. **Regeneration of the 338-package inventory** against the final locked dependency set.

## Limits of this document

Read-only synthesis of the documents listed above. No source, config, notices, or `LICENSE` files
were created or modified; no builds were run; no commits were made; no legal advice is given; no
approval is granted; no license, copyright holder, publisher, or contact value is decided or
invented.