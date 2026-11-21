# Upstream License Texts (distribution preparation)

**Date:** 2026-11-21 · **Status:** PREPARED — ship these texts with every release.

The generated inventory at
`artifacts/dependencies/notices-a1587a6b-6e23-4a51-bf9b-493b0294a934/` contains
collected license text for 331 of 338 packages. The seven entries below had no
license text in their published tarballs but have valid, GPLv3-compatible SPDX
declarations. Use the canonical upstream source URLs to obtain and ship the full
text with the project's `THIRD_PARTY_NOTICES.md`.

| Package | Version | SPDX | Canonical upstream source (obtain full text) |
|---|---|---|---|
| `alloc-stdlib` | 0.3.0 | BSD-3-Clause | https://github.com/dropbox/rust-alloc-no-stdlib (BSD-3-Clause) |
| `clipboard-win` | 5.4.3 | BSL-1.0 | https://github.com/DoumanAsh/clipboard-win (Boost Software License 1.0) |
| `selectors` | 0.38.0 | MPL-2.0 | https://github.com/servo/servo/tree/master/components/selectors (MPL 2.0; canonical text https://www.mozilla.org/media/MPL/2.0/index.txt) |
| `tauri-plugin` | 2.7.2 | Apache-2.0 OR MIT | https://github.com/tauri-apps/tauri (LICENSE-APACHE / LICENSE-MIT) |
| `webview2-com` | 0.39.1 | MIT | https://github.com/wrwebgw/webview2-com-rs (LICENSE-MIT) |
| `webview2-com-macros` | 0.8.1 | MIT | same repository (LICENSE-MIT) |
| `webview2-com-sys` | 0.39.1 | MIT | same repository (LICENSE-MIT) |

## Additional texts to ship
- **CDLA-Permissive-2.0** for `webpki-roots` 0.26.11 and 1.0.11 — canonical text
  at https://cdla.dev/permissive-2-0/ (see FINAL_LICENSE_DECISION.md uncertainty).
- **MPL-2.0** text for the 5 MPL crates — canonical text at
  https://www.mozilla.org/media/MPL/2.0/index.txt.
- Project GPL-3.0-or-later text — only after owner approval (root `LICENSE` intentionally absent).

## Procedure at release time
1. Fetch each canonical text, record retrieval date and source URL.
2. Append to `THIRD_PARTY_NOTICES.md` or a `licenses/` folder in the release asset.
3. Verify `SHA256SUMS` covers the notices file.
4. Regenerate notices with `node tools/dependency-notices.mjs` after any dependency change.

No license text is fabricated here; only canonical sources are listed for the owner/release process to retrieve.
