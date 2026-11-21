# Zero-Budget Open-Source Distribution Plan

**Version:** 0.1.0 · **Date:** 2026-11-21 · **Repo:** `E:\CloneIdm\swiftfetch` (not a git repo).

## Owner decisions recorded

| Decision | Value |
|---|---|
| Publisher | Individual / personal publisher |
| License | GPL-3.0-or-later — **proposed, CONDITIONALLY APPROVED** pending the conditions below |
| Distribution | Free download for everyone |
| Budget | Zero — no paid certificates, hosting, domains, or subscriptions |

No application rebuild or redesign was performed for this plan. All work is
documentation/audit only. **NO-GO status and all prior release evidence are preserved.**

## Completed work

| Task | Deliverable | Result |
|---|---|---|
| 1 — License audit | `docs/release/LICENSE_AUDIT.md` | 338 deps (333 Cargo + 5 npm). No proprietary/LicenseRef/AGPL/SSPL/GPL-2.0-only. Verdict: **CONDITIONALLY APPROVED** for GPL-3.0-or-later. Seven missing-text entries identified with valid compatible SPDX IDs. |
| 2 — Public repo docs | `README.md`, `CONTRIBUTING.md`, `CODE_OF_CONDUCT.md`, `SECURITY.md`, `CHANGELOG.md`, `THIRD_PARTY_NOTICES.md` | Prepared with `[OWNER: ...]` placeholders; no invented contacts. `LICENSE` intentionally **not** added (audit conditional). Existing `docs/SECURITY.md` retained. |
| 3 — Free hosting | `docs/release/GITHUB_RELEASES_SETUP.md` | Asset naming, `v0.1.0` tag, checksums, user verification, safe publication checklist. Nothing published. |
| 4 — Tauri updater | `docs/release/UPDATER_KEY_SETUP.md` | Local keygen, env vars, GitHub Releases hosting, manifest/signature production, no endpoint activation. Current state: `pubkey:""`, env unset, `not_configured`. |
| 5 — SignPath | `docs/release/SIGNPATH_ELIGIBILITY.md` | Official criteria documented; concerns listed (no git, no LICENSE yet, unsigned candidate, individual publisher, 2FA unknown, no reproducible-build CI). No application submitted. |
| 6 — Website | `docs/site/` (7 pages + `assets/style.css`) | GitHub Pages-compatible Home/Features/Download/Documentation/Privacy/License/Support; accessible offline HTML; honest unsigned-preview and proposed-license language; `[OWNER: ...]` placeholders. No invented claims. |

## Dependency concerns

- **Seven missing license-text entries** (valid, GPLv3-compatible SPDX IDs, but tarballs omit text):
  `alloc-stdlib 0.3.0` BSD-3-Clause; `clipboard-win 5.4.3` BSL-1.0;
  `selectors 0.38.0` MPL-2.0; `tauri-plugin 2.7.2` Apache-2.0 OR MIT;
  `webview2-com 0.39.1`, `webview2-com-macros 0.8.1`, `webview2-com-sys 0.39.1` all MIT.
  Upstream text confirmed available; it must be included in distributed notices.
- **MPL-2.0 ×5** — GPLv3-compatible via SPDX §3.3, but source-availability obligations apply.
- **CDLA-Permissive-2.0 ×2** (`webpki-roots`) — **UNCERTAIN**; no formal FSF GPL ruling.
  Ship the agreement text; seek owner/legal confirmation.
- SPDX `licenseConcluded` is `NOASSERTION` for most packages while
  `licenseDeclared` is populated — this is an inventory limitation, not a license conflict.
- Regenerate with `node tools/dependency-notices.mjs` after any dependency change and re-audit.

## Remaining owner actions (exact manual steps)

1. **License:** confirm the GPL-3.0-or-later choice after resolving the CDLA uncertainty;
   only then add the final `LICENSE` file at repo root (not done here by design).
2. **Third-party text:** place upstream license text for the 7 entries into the shipped
   notices; bundle `THIRD_PARTY_NOTICES.md` + the generated notice file in every release.
3. **Git repository:** initialize a public repository, commit with clean history, enable
   2FA, and enable private vulnerability reporting. (The workspace is currently not a git repo.)
4. **Placeholders:** replace every `[OWNER: ...]` in README/CONTRIBUTING/SECURITY/
   CODE_OF_CONDUCT/root docs, `docs/site/`, and the release docs. Do not invent contacts.
5. **Repository URL:** set the real source URL in website documentation page and README links.
6. **GitHub Pages:** enable Pages from `docs/site/` on the default branch.
7. **Release assets:** follow `GITHUB_RELEASES_SETUP.md` — create tag `v0.1.0`, attach the
   NSIS exe, MSI, source ZIP and `SHA256SUMS`, and publish release notes. **Do not publish
   until the NO-GO gates below are satisfied.**
8. **Updater keys (if desired):** follow `UPDATER_KEY_SETUP.md` locally; embed only the
   public key; never commit private keys; activate an endpoint only after owner approval.
9. **Signing:** decide on SignPath Foundation (see eligibility concerns) or another
   free/trusted signing route. Without Authenticode signing the candidate remains NO-GO.
10. **Clean-machine acceptance:** run install/upgrade/rollback/uninstall on a clean Windows VM.
11. **Community/legal:** approve Terms/Privacy/Support, WCAG/SR checks, 24-hour soak, and
    permitted-site sampling if those gates remain required for the owner's release bar.

## Remaining external blockers (unchanged)

- Trusted Authenticode signing (all executables `NotSigned`).
- Configured signed update channel (`pubkey:""`, env unset) and real signed upgrade/rollback.
- Clean Windows VM acceptance; permanent browser/store review if desired.
- Legal approval of license/publisher identity/contact/terms; CDLA uncertainty.
- Screen-reader/contrast certification, 24-hour soak, site compatibility sampling.
- Startup <1 s target (measured ~26–49 s cold; Rust setup 4.4–8.6 s, remainder WebView2 first-paint).

## Final verdict

**NO-GO.** Free/open-source preparation is complete at the documentation level, and
GPL-3.0-or-later is conditionally compatible, but mandatory signing, update,
clean-machine, legal, and external-test gates remain unsatisfied. The candidate
manifest still records `ready_for_public_release: false`. Nothing was published,
no keys were generated, no endpoints activated, no certificate or approval claimed.
