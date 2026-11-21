# Third-Party Notices — SwiftFetch

**Status: PRELIMINARY — pending completion of the license audit.** This file
summarizes the third-party dependency inventory for distribution purposes. It
is not legal advice and does not by itself satisfy redistribution obligations.
The full generated inventory and the collected license texts are the
authoritative artifacts and are referenced below.

## License audit result

A dependency license audit (GPL-3.0-or-later target) is recorded at
[docs/release/LICENSE_AUDIT.md](docs/release/LICENSE_AUDIT.md). Its verdict:
**CONDITIONAL APPROVAL — GPL-3.0-or-later**, conditional on (a) obtaining and
bundling the seven missing license texts listed below, (b) honoring the
MPL-2.0 notice/source-offer obligations, (c) resolving the CDLA-Permissive-2.0
uncertainty, and (d) shipping the GPLv3 LICENSE text and these notices with
the final installer. The audit is a technical compatibility finding, not legal
approval, and the project license itself remains an owner decision.

## Inventory

Generated from the locked npm runtime dependencies and the Cargo dependency
graph for Windows x64 (Rust inventory includes build and test dependencies —
a conservative list, not a claim that every package is linked into the binary):

| Ecosystem | Packages |
| --- | --- |
| cargo (direct + transitive, incl. build/dev) | 333 |
| npm (runtime, locked) | 5 |
| **Total** | **338** |

npm dev-only tooling (vite, typescript, @vitejs/plugin-react, @tauri-apps/cli,
@types/*) is build-time only, is not redistributed, and is out of scope for
the shipped artifact.

### Declared-license distribution (SPDX expressions)

| Declared expression | Packages |
| --- | --- |
| MIT OR Apache-2.0 | 151 |
| MIT | 70 |
| Apache-2.0 OR MIT | 34 |
| Unicode-3.0 | 18 |
| MIT/Apache-2.0 | 11 |
| Unlicense OR MIT | 9 |
| MPL-2.0 | 5 |
| MIT OR Apache-2.0 OR Zlib | 4 |
| BSD-3-Clause | 3 |
| Zlib (2), Unlicense/MIT (2), Apache-2.0 OR ISC OR MIT (2), BSD-3-Clause OR Apache-2.0 (2), ISC (2), MIT OR Zlib OR Apache-2.0 (2), Zlib OR Apache-2.0 OR MIT (2), CDLA-Permissive-2.0 (2), BSD-2-Clause OR Apache-2.0 OR MIT (2), BSL-1.0 (2), Apache-2.0 (2), CC0-1.0 OR MIT-0 OR Apache-2.0 (2) | 27 |
| 0BSD OR MIT OR Apache-2.0 (1), Apache-2.0 OR BSL-1.0 (1), (MIT OR Apache-2.0) AND Unicode-3.0 (1), Apache-2.0 AND MIT (1), BSD-3-Clause/MIT (1), BSD-3-Clause AND MIT (1), Apache-2.0 AND ISC (1), Apache-2.0 / MIT (1), (Apache-2.0 OR MIT) AND BSD-3-Clause (1) | 9 |

Grouped by SPDX identifier (an expression such as `MIT OR Apache-2.0` counts
once for each listed option): MIT (296), Apache-2.0 (221), Unicode-3.0 (19),
Unlicense (11), Zlib (10), BSD-3-Clause (8), MPL-2.0 (5), ISC (5), BSL-1.0 (3),
MIT-0 (2), CDLA-Permissive-2.0 (2), CC0-1.0 (2), BSD-2-Clause (2), 0BSD (1).

No proprietary, GPL-incompatible, or strong-copyleft dependency was found. The
generated SBOM records `licenseConcluded: NOASSERTION` for all entries — a
generator artifact (no per-file conclusion was performed), not an indication of
unknown licenses.

## Missing license text — 7 entries

The generated inventory bundles the license files found in each installed
package. Exactly seven packages declared an SPDX expression but shipped no
license text in the published crate tarball. Upstream canonical texts exist
for all seven (verified during the audit); they are **not yet bundled**:

| Package | Version | Declared license | Upstream source of canonical text |
| --- | --- | --- | --- |
| alloc-stdlib | 0.3.0 | BSD-3-Clause | https://github.com/dropbox/rust-alloc-no-stdlib |
| clipboard-win | 5.4.3 | BSL-1.0 | https://github.com/DoumanAsh/clipboard-win |
| selectors | 0.38.0 | MPL-2.0 (per-file §1.4/Exhibit A headers) | https://github.com/servo/stylo; https://www.mozilla.org/MPL/2.0/ |
| tauri-plugin | 2.7.2 | Apache-2.0 OR MIT | https://github.com/tauri-apps/tauri |
| webview2-com | 0.39.1 | MIT | https://github.com/wravery/webview2-rs |
| webview2-com-macros | 0.8.1 | MIT | https://github.com/wravery/webview2-rs |
| webview2-com-sys | 0.39.1 | MIT | https://github.com/wravery/webview2-rs |

These are text-shipping/attribution gaps, not license incompatibilities. They
must be resolved before the notices are final.

## Other conditions carried by the audit

- **MPL-2.0 (5 crates):** `cssparser`, `cssparser-macros`, `dtoa-short`,
  `option-ext`, `selectors` — file-level copyleft; compatible via MPL §3.3 with
  the GPLv3 text and per-file notices, and a source offer if MPL files are
  modified.
- **CDLA-Permissive-2.0 (2 entries):** `webpki-roots` 0.26.11 and 1.0.11
  (CA certificate data) — permissive data license with no formal FSF
  GPL-compatibility determination; the agreement text must ship with the data.
  Flagged for owner/counsel.
- **GPLv3 source offer:** if GPL-3.0-or-later is selected, distributing the
  binary requires offering the corresponding source (GPLv3 §6).
- No `LICENSE` file exists yet; the project license is an owner decision
  tracked in `docs/release/LEGAL_PRIVACY_CHECKLIST.md`.

## Full generated inventory

The authoritative machine-readable and per-package artifacts live in
`artifacts/dependencies/notices-a1587a6b-6e23-4a51-bf9b-493b0294a934/`:

- `THIRD-PARTY-NOTICES.md` — the full 338-row inventory table.
- `dependencies.json` — per-package declared license and license-file
  inventory with SHA-256 checksums.
- `sbom.spdx.json` — SPDX SBOM (338 entries).
  `<package>_<version>/` — 331 directories with the collected license texts.
- Sibling summaries: `artifacts/dependencies/cargo-windows.json`,
  `artifacts/dependencies/npm-runtime.spdx.json`.

Regenerate with `node tools/dependency-notices.mjs` after dependency changes,
and regenerate against the final locked dependency set before release
(`docs/release/RELEASE_PLAN.md`).