# SwiftFetch — Dependency License Audit (GPL-3.0-or-later)

**Created:** 2026-11-21. **Version:** 0.1.0. **Lane:** license-auditor (read-only audit + this document only).

**Scope and limits.** This audit answers one question: is the observed third-party dependency
license set compatible with distributing SwiftFetch under **GPL-3.0-or-later**? It is a technical
license review, **not legal advice and not an approval**. No LICENSE file was added, no source or
config was changed, no builds were run, and no commits were made. The project's own distribution
license remains an owner decision (`LEGAL_PRIVACY_CHECKLIST.md` §1.1); this document does not
choose it or claim legal clearance.

## 1. Methodology

1. **Sources of truth (read-only):**
   - `src-tauri/Cargo.toml` — direct Rust dependencies (incl. build/dev/target-specific).
   - `src-tauri/Cargo.lock` — locked Rust dependency graph.
   - `package.json` / `package-lock.json` — direct and locked npm dependencies.
   - `artifacts/dependencies/cargo-windows.json` — cargo metadata with per-package `license` fields.
   - `artifacts/dependencies/npm-runtime.spdx.json` — SPDX 2.3 runtime npm SBOM (5 packages).
   - `artifacts/dependencies/notices-a1587a6b-6e23-4a51-bf9b-493b0294a934/` — generated notices:
     `dependencies.json` (338 entries), `sbom.spdx.json` (338 entries), `THIRD-PARTY-NOTICES.md`,
     and 331 per-package license-text directories.
2. **Inventory:** parsed `dependencies.json` programmatically; grouped by ecosystem and by declared
   SPDX expression; cross-checked counts against `THIRD-PARTY-NOTICES.md` and the SBOM.
3. **Missing-text detection:** selected all entries where `license_files` is empty. Exactly seven
   matched; each was re-verified against the local cargo registry source directory
   (`C:\Users\Ali Ahasan\.cargo\registry\src\index.crates.io-1949cf8c6b5b557f\<crate>-<version>`):
   all seven declared a `license` in `Cargo.toml` but shipped **no** LICENSE/LICENCE/COPYING/NOTICE
   file in the published tarball. Upstream repositories were checked for the canonical text
   (GitHub `contents/` API), and per-file MPL headers were inspected in the installed source.
4. **Compatibility analysis:** each distinct license expression was classified for one-way
   compatibility with GPLv3 (permissive/public-domain licenses are GPLv3-compatible; weak copyleft
   MPL-2.0 is compatible via §3.3 with conditions; strong copyleft/proprietary would be blocking).
   Uncertain cases were researched against authoritative sources (OSI, SPDX, Mozilla FAQ).
5. **Evidence note:** `sbom.spdx.json` records `licenseConcluded: NOASSERTION` for all 338 packages.
   This is a generator artifact — the tool populated `licenseDeclared` from cargo/npm metadata and
   never performed a per-file conclusion. It does **not** indicate an actual unknown license.

## 2. Inventory summary

| Ecosystem | Count | Source |
| --- | --- | --- |
| cargo (direct + transitive, incl. build/test deps) | 333 | `dependencies.json`, `cargo-windows.json`, `Cargo.lock` |
| npm (runtime, locked) | 5 | `npm-runtime.spdx.json`, `package-lock.json`, notices |
| **Total** | **338** | matches `LEGAL_PRIVACY_CHECKLIST.md` §1.3 / `RELEASE_GAP_REGISTER.md:65` |

Direct Rust dependencies (from `Cargo.toml`): `tauri`, `tauri-plugin-opener`,
`tauri-plugin-updater`, `tauri-plugin-clipboard-manager`, `serde`, `serde_json`, `tokio`,
`futures-util`, `reqwest`, `rusqlite`, `sha2`, `uuid`, `chrono`, `thiserror`, `bytes`, `once_cell`,
`url`, `sha1`, `md-5`; Windows target: `windows-registry`; build: `tauri-build`; dev: `tauri`
(test feature), `base64`, `tempfile`, `ureq`. Direct npm runtime: `react`, `react-dom`,
`@tauri-apps/api`, `@tauri-apps/plugin-opener`. Everything else in the 338 is transitive.
npm dev-only tooling (vite, typescript, @vitejs/plugin-react, @tauri-apps/cli, @types/*) is
**build-time only, not shipped**, and therefore does not affect the distributed artifact's license
compatibility; it is out of scope for the redistribution obligations below.

Declared-license distribution (338 entries): MIT OR Apache-2.0 (151), MIT (70),
Apache-2.0 OR MIT (34), Unicode-3.0 (18), MIT/Apache-2.0 (11), Unlicense OR MIT (9),
MPL-2.0 (5), MIT OR Apache-2.0 OR Zlib (4), BSD-3-Clause (3), plus smaller groups
(Zlib, ISC, BSL-1.0, Apache-2.0, CC0-1.0 OR MIT-0 OR Apache-2.0, Unicode-3.0 AND
(MIT OR Apache-2.0), BSD-2-Clause OR Apache-2.0 OR MIT, CDLA-Permissive-2.0, etc.).

## 3. The seven missing-license-text entries — exact resolution

| # | Package | Version | Declared license (SPDX) | In published crate? | Upstream text exists? | Resolution |
| --- | --- | --- | --- | --- | --- | --- |
| 1 | `alloc-stdlib` | 0.3.0 | `BSD-3-Clause` | No LICENSE file; Cargo.toml declares it | **Yes** — `github.com/dropbox/rust-alloc-no-stdlib` root `LICENSE` (BSD-3-Clause). Sibling `alloc-no-stdlib` 3.0.0 in the same family *did* ship its text; this crate did not. | Obtain canonical BSD-3-Clause text from upstream repo; add to per-package notice dir; regenerate notices. |
| 2 | `clipboard-win` | 5.4.3 | `BSL-1.0` (Boost Software License 1.0) | No LICENSE file; Cargo.toml declares it | **Yes** — `github.com/DoumanAsh/clipboard-win` root `LICENSE` (BSL-1.0). | Obtain BSL-1.0 text from upstream; add to notices. Preserve copyright notice in source distributions. |
| 3 | `selectors` | 0.38.0 | `MPL-2.0` | No LICENSE file, but every source file carries the MPL §1.4/Exhibit A header | **Yes** — per-file header verified in installed `lib.rs` (`This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0…`); canonical full text at `mozilla.org/MPL/2.0/`. Upstream `servo/stylo` has no root LICENSE file (MPL uses per-file notices). | Ship canonical MPL-2.0 text plus the per-file notices; observe §3.2 source-offer obligations if MPL files are modified (see §5). |
| 4 | `tauri-plugin` | 2.7.2 | `Apache-2.0 OR MIT` | No LICENSE file; Cargo.toml declares it | **Yes** — `github.com/tauri-apps/tauri` root `LICENSE-APACHE-2.0`, `LICENSE-MIT`, `LICENSE.spdx`. | Obtain both texts (dual-licensed; MIT may be elected to simplify). Add to notices. |
| 5 | `webview2-com` | 0.39.1 | `MIT` | No LICENSE file; Cargo.toml declares it | **Yes** — `github.com/wravery/webview2-rs` root `LICENSE` (MIT). | Obtain MIT text; add to notices. |
| 6 | `webview2-com-macros` | 0.8.1 | `MIT` | No LICENSE file; Cargo.toml declares it | **Yes** — same `wravery/webview2-rs` repo `LICENSE`. | Obtain MIT text; add to notices. |
| 7 | `webview2-com-sys` | 0.39.1 | `MIT` | No LICENSE file; Cargo.toml declares it | **Yes** — same `wravery/webview2-rs` repo `LICENSE`. | Obtain MIT text; add to notices. |

All seven are **text-shipping/attribution gaps, not license incompatibilities**: their SPDX
identifiers are valid and GPLv3-compatible. The generator's "0 files" count means the published
crate tarball omitted the text, not that upstream lacks a license.

## 4. GPL-3.0-or-later compatibility table

| License expression (as declared) | Packages | GPLv3 status | Notes |
| --- | --- | --- | --- |
| `MIT`, `MIT OR Apache-2.0`, `Apache-2.0 OR MIT`, `MIT/Apache-2.0`, `Apache-2.0`, `Apache-2.0 / MIT`, `Apache-2.0 AND MIT` | ~300 | Compatible | Apache-2.0 is GPL**v3**-compatible. The Apache-2.0/GPL-2.0-only conflict cited in the task does **not** apply to a GPL-3.0-or-later target. If the owner ever picks GPL-2.0-only, this becomes blocking. |
| `BSD-3-Clause`, `BSD-2-Clause OR Apache-2.0 OR MIT`, `BSD-3-Clause OR Apache-2.0`, `BSD-3-Clause/MIT`, `BSD-3-Clause AND MIT`, `0BSD OR MIT OR Apache-2.0`, `ISC`, `Apache-2.0 AND ISC`, `Apache-2.0 OR ISC OR MIT` | ~20 | Compatible | Attribution + disclaimer retention required. |
| `Unlicense OR MIT`, `Unlicense/MIT`, `CC0-1.0 OR MIT-0 OR Apache-2.0`, `MIT OR Zlib OR Apache-2.0`, `Zlib`, `Zlib OR Apache-2.0 OR MIT`, `MIT OR Apache-2.0 OR Zlib`, `BSD-3-Clause/MIT` | ~20 | Compatible | Public-domain dedication / permissive. |
| `BSL-1.0`, `Apache-2.0 OR BSL-1.0` | 3 (incl. `clipboard-win`) | Compatible | Permissive; retain copyright/license in source distributions (not required for binary-only). |
| `Unicode-3.0`, `(MIT OR Apache-2.0) AND Unicode-3.0` | 19 | Compatible | Unicode License v3 is OSI-approved (2023-11-17), MIT-derived, permissive. |
| `MPL-2.0` | 5 (`cssparser`, `cssparser-macros`, `dtoa-short`, `option-ext`, `selectors`) | **Compatible with conditions** | Weak/file-level copyleft. Mozilla FAQ Q13/Q14: can combine with GPL code via §3.3 "Larger Work" provided the software is not marked "Incompatible With Secondary Licenses" (Exhibit B) and the MPL source is additionally distributed under (L)GPL. These crates use the standard Exhibit A header (verified for `selectors`), so the default is compatible. Modified MPL files must remain MPL and be offered in source. |
| `CDLA-Permissive-2.0` | 2 (`webpki-roots` 0.26.11, 1.0.11) | **UNCERTAIN** | SPDX-listed permissive **data** license (CA certificate data), no copyleft, no FSF GPL-compatibility determination located; only condition is shipping the agreement text with shared data. Not a linked-software issue in practice, but flag for owner/counsel. |

**No blocking incompatibility found:** no proprietary/`LicenseRef-*`/`UNLICENSED` entries, no
AGPL/SSPL/Commons Clause, no GPL-2.0-only or LGPL-2.1-only component that would create an
Apache-2.0 conflict. `native-tls`/`schannel` and `rustls` are used rather than OpenSSL, so the
classic OpenSSL/GPLv3 incompatibility is avoided. Static linking of permissive and MPL-2.0 code
into a GPLv3 binary is permitted; MPL source-offer conditions attach to the MPL files themselves.

## 5. Attribution and redistribution obligations

1. **Project license text:** choose GPL-3.0-or-later, add the LICENSE file at repo root, and ship
   the full GPLv3 text in the distributed package (currently absent — `LEGAL_PRIVACY_CHECKLIST.md` §1.1–1.2).
2. **Third-party notices:** `THIRD-PARTY-NOTICES.md` plus the per-package license texts must be
   bundled with the installer, not only in candidate snapshots (`LEGAL_PRIVACY_CHECKLIST.md` §1.5,
   `FINAL_BUILD_EVIDENCE.md:71`). Current candidate notices omit text for the seven packages in §3.
3. **Permissive/MIT/Apache/BSD/ISC/Zlib/Unicode/BSL:** reproduce copyright notice, license text and
   warranty disclaimer in distributions; Apache-2.0 additionally requires preserving NOTICE files
   where present and stating changes.
4. **MPL-2.0 (5 crates):** include MPL-2.0 text and per-file notices; if any MPL source file is
   modified, make those modified files available under MPL (§3.1/3.2) and, for a GPLv3 Larger Work,
   additionally distribute the MPL source under GPLv3 (§3.3). No modifications observed in this audit.
5. **CDLA-Permissive-2.0 (`webpki-roots`):** ship the CDLA agreement text with the data (§2.1).
6. **GPLv3 source offer:** when distributing the binary, offer the corresponding source of the
   GPL-covered work (the project's own code plus any GPL-licensed components; none of the audited
   dependencies are strong copyleft). Provide the written offer / download location per GPLv3 §6.
7. **Linking:** Rust release profile uses `lto = true` / staticlib+cdylib crate types
   (`Cargo.toml:15,59-64`), i.e. static linking — fine for permissive and MPL-2.0 dependencies;
   no LGPL-style relinking obligation arises from this inventory.
8. **Dev tooling:** npm dev dependencies are not redistributed; no obligations from them for the
   shipped artifact.

## 6. Incompatible / uncertain list

- **Incompatible:** none found.
- **Uncertain:** `CDLA-Permissive-2.0` (`webpki-roots` ×2) — permissive data license, no formal
  FSF GPL-compatibility determination; obliges shipping the agreement text. Owner/counsel to confirm.
- **Conditional:** `MPL-2.0` (`cssparser`, `cssparser-macros`, `dtoa-short`, `option-ext`,
  `selectors`) — compatible via §3.3, but MPL notice/source obligations must be honored.
- **Outstanding gaps (not incompatibilities):** the seven missing license texts in §3; confirmed
  bundling of `THIRD-PARTY-NOTICES.md` + texts in the final installer; owner selection and shipping
  of the GPLv3 LICENSE text. Note also that the inventory is a conservative snapshot from a
  candidate build; `RELEASE_PLAN.md` §1.3 requires regeneration against final locked deps.

## 7. Verdict

**CONDITIONALLY APPROVED — GPL-3.0-or-later.**

Rationale: every audited dependency license is either GPLv3-compatible-permissive, a public-domain
dedication, or MPL-2.0 (compatible with conditions). No proprietary, GPL-incompatible, or
Apache-2.0-vs-GPL-2.0-only conflict exists for a GPL-3.0-or-later target. Approval is conditional
on: (a) obtaining and bundling the seven missing license texts (§3); (b) honoring MPL-2.0
notice/source-offer obligations (§5.4); (c) resolving the CDLA-Permissive-2.0 uncertainty (§6);
and (d) shipping the GPLv3 LICENSE text and third-party notices with the final installer.
This is a technical compatibility finding only — it is **not** legal approval, does not select the
project license, and does not change the release's NO-GO status while owner decisions remain open.

### Sources consulted
- Local: paths listed in §1; `docs/release/LEGAL_PRIVACY_CHECKLIST.md`, `RELEASE_GAP_REGISTER.md`,
  `FINAL_BUILD_EVIDENCE.md`, `RELEASE_PLAN.md`, `GO_NO_GO_REPORT.md`.
- Upstream/authoritative:
  - https://opensource.org/license/Unicode-3.0 ; https://unicode.org/faq/unicode_license.html
  - https://spdx.org/licenses/CDLA-Permissive-2.0.html ; https://cdla.dev/permissive-2-0/
  - https://www.mozilla.org/en-US/MPL/2.0/FAQ/ (Q13, Q14)
  - https://spdx.org/licenses/BSL-1.0.html ; https://opensource.org/license/BSL-1.0
  - GitHub contents API: `dropbox/rust-alloc-no-stdlib`, `DoumanAsh/clipboard-win`,
    `tauri-apps/tauri`, `wravery/webview2-rs` (LICENSE files confirmed present); `servo/stylo`
    (per-file MPL notices instead of root LICENSE).
