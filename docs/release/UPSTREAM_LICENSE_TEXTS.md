# Upstream License Texts for the Seven Missing-Text Entries

**Created:** 2026-11-21. **Lane:** license-finalization-researcher.
**Companion to:** `docs/release/LICENSE_AUDIT.md` §3 (seven missing-license-text entries) and
`docs/release/THIRD_PARTY_NOTICES.md`.

**Purpose.** Record, for redistribution, where the authoritative license text for each of the seven
packages that shipped no license file in their published crate tarball can be obtained, and what the
audit already verified on 2026-11-21. This document is a **source-of-truth index and work record**,
not a substitute for bundling the texts. It is not legal advice.

**Status.** The seven entries are **text-shipping/attribution gaps, not license incompatibilities**.
Upstream texts were verified to exist (2026-11-21, see §3). The texts are **not yet copied into the
notices bundle or the installer** — that bundling remains an outstanding release task
(`LICENSE_AUDIT.md` §7 condition (a)).

---

## 1. Canonical upstream text sources per package

Copy each text **verbatim** from the canonical source, including its copyright holder line. Do not
paraphrase and do not invent or substitute a copyright holder.

| # | Package | Version | Declared SPDX | Canonical license text(s) | Upstream repository |
| --- | --- | --- | --- | --- | --- |
| 1 | `alloc-stdlib` | 0.3.0 | BSD-3-Clause | BSD-3-Clause text — https://spdx.org/licenses/BSD-3-Clause.html ; upstream `LICENSE`: https://github.com/dropbox/rust-alloc-no-stdlib | https://github.com/dropbox/rust-alloc-no-stdlib |
| 2 | `clipboard-win` | 5.4.3 | BSL-1.0 | BSL-1.0 text — https://spdx.org/licenses/BSL-1.0.html ; upstream `LICENSE`: https://github.com/DoumanAsh/clipboard-win | https://github.com/DoumanAsh/clipboard-win |
| 3 | `selectors` | 0.38.0 | MPL-2.0 | MPL-2.0 full text — https://www.mozilla.org/en-US/MPL/2.0/ ; SPDX ref — https://spdx.org/licenses/MPL-2.0.html ; plus the per-file MPL §1.4/Exhibit A headers already present in the source | https://github.com/servo/stylo |
| 4 | `tauri-plugin` | 2.7.2 | Apache-2.0 OR MIT | Apache-2.0 — https://www.apache.org/licenses/LICENSE-2.0 (SPDX: https://spdx.org/licenses/Apache-2.0.html) ; MIT — https://spdx.org/licenses/MIT.html ; upstream files: `LICENSE-APACHE-2.0`, `LICENSE-MIT` in https://github.com/tauri-apps/tauri | https://github.com/tauri-apps/tauri |
| 5 | `webview2-com` | 0.39.1 | MIT | MIT — https://spdx.org/licenses/MIT.html ; upstream `LICENSE`: https://github.com/wravery/webview2-rs | https://github.com/wravery/webview2-rs |
| 6 | `webview2-com-macros` | 0.8.1 | MIT | MIT — https://spdx.org/licenses/MIT.html ; upstream `LICENSE`: https://github.com/wravery/webview2-rs | https://github.com/wravery/webview2-rs |
| 7 | `webview2-com-sys` | 0.39.1 | MIT | MIT — https://spdx.org/licenses/MIT.html ; upstream `LICENSE`: https://github.com/wravery/webview2-rs | https://github.com/wravery/webview2-rs |

**Dual-license election note (`tauri-plugin`).** Apache-2.0 **OR** MIT is an election by the
distributor. Either text may be shipped; shipping one of the two (and saying which) satisfies the
notice. The audit noted MIT may be elected to simplify. This is a distribution choice, not a legal
conclusion; it does not change compatibility.

---

## 2. What was verified (2026-11-21, `LICENSE_AUDIT.md` §1.3/§3)

- Each of the seven declared a `license` in `Cargo.toml` but shipped **no** `LICENSE`/`LICENCE`/
  `COPYING`/`NOTICE` file in the published crate tarball (checked in the local Cargo registry source
  directory `%USERPROFILE%\.cargo\registry\src\index.crates.io-*\<crate>-<version>`).
- Upstream `LICENSE` files confirmed present via the GitHub `contents/` API for:
  `dropbox/rust-alloc-no-stdlib`, `DoumanAsh/clipboard-win`, `tauri-apps/tauri`,
  `wravery/webview2-rs`.
- `selectors` (from `servo/stylo`): upstream ships **no root LICENSE file** — MPL-2.0 projects use
  per-file notices. The MPL §1.4/Exhibit A header was verified present in every inspected source file
  (e.g. installed `lib.rs`: *"This Source Code Form is subject to the terms of the Mozilla Public
  License, v. 2.0…"*). Ship the canonical MPL-2.0 text **plus** retain those per-file headers.
- Sibling-crate check: `alloc-no-stdlib` 3.0.0 (same family as `alloc-stdlib` 0.3.0) **did** ship its
  license text, confirming the upstream repo carries the text even though the 0.3.0 tarball omitted
  it.

---

## 3. What remains open (not yet done)

1. Copy each canonical text verbatim into the corresponding per-package directory under
   `artifacts/dependencies/notices-*/`, and regenerate notices
   (`node tools/dependency-notices.mjs`).
2. Bundle `THIRD-PARTY-NOTICES.md` + all license texts with the final installer
   (`LICENSE_AUDIT.md` §5.2).
3. `clipboard-win` (BSL-1.0): preserve the copyright notice in **source** distributions
   (`LICENSE_AUDIT.md` §4).
4. `selectors` (MPL-2.0): see §3.3 obligations summarized in `FINAL_LICENSE_DECISION.md` §4.
5. Verify the transcribed texts against the URLs above on the day of bundling and re-check the
   upstream `LICENSE` files have not changed.

## 4. Related texts still owed by the distribution

- **CDLA-Permissive-2.0** (`webpki-roots` 0.26.11, 1.0.11): ship the agreement text with the data
  (§2.1 of the agreement) — canonical text: https://cdla.dev/permissive-2-0/ ;
  SPDX: https://spdx.org/licenses/CDLA-Permissive-2.0.html . Compatibility question unresolved —
  see `FINAL_LICENSE_DECISION.md` §6.
- **GPL-3.0-or-later** text (only if selected by the owner): https://www.gnu.org/licenses/gpl-3.0.txt .
- **MPL-2.0** text for the 5 MPL crates (`cssparser`, `cssparser-macros`, `dtoa-short`,
  `option-ext`, `selectors`): https://www.mozilla.org/en-US/MPL/2.0/ .

---

## Sources consulted

- https://spdx.org/licenses/ (canonical license texts: BSD-3-Clause, BSL-1.0, MPL-2.0,
  Apache-2.0, MIT, CDLA-Permissive-2.0)
- https://www.apache.org/licenses/LICENSE-2.0
- https://www.mozilla.org/en-US/MPL/2.0/ ; https://cdla.dev/permissive-2-0/
- https://www.gnu.org/licenses/gpl-3.0.txt
- Upstream repositories listed in §1 (LICENSE files confirmed present via the GitHub `contents/`
  API on 2026-11-21, as recorded in `LICENSE_AUDIT.md`).
- Local read-only evidence: `LICENSE_AUDIT.md` §1–§3, `THIRD_PARTY_NOTICES.md`.