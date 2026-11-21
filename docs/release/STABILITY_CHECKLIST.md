# SwiftFetch — Stability Release Checklist (Phase 5)

**Created:** 2026-11-21. **Version:** 0.1.0. **Scope:** stability items only (soak, cold start,
clean-machine installation, update/rollback failure paths, permitted-site compatibility).
**Companion checklists:** `LEGAL_PRIVACY_CHECKLIST.md`, `A11Y_CHECKLIST.md`; gate context in
`RELEASE_PLAN.md` and `RELEASE_GAP_REGISTER.md`.

Status legend:

- **Ready** — implemented with recorded evidence; no further work known.
- **Partial** — implemented/drafted, but review, missing pieces, or limits remain.
- **Externally blocked** — requires an owner decision or external resource.
- **Not verified** — no evidence found; do not claim.

All items below are **local evidence**, not public-release approval (`VERIFICATION.md:6,153`).
`candidate-manifest.json` records `ready_for_public_release: false` and all three Windows
executables are `NotSigned` (`VERIFICATION.md:267-272`).

## 1. 24-hour soak

| # | Item | STATUS | Evidence | Required action |
| --- | --- | --- | --- | --- |
| 1.1 | 24-hour continuous run (queue, scheduler, memory, log growth) | Not verified | `VERIFICATION.md:276-278` ("No ... 24-hour soak ... has been performed"); `RELEASE-READINESS.md:72-73` | [OWNER: soak window] on the final signed binary; record duration, task counts, resource trend |
| 1.2 | Short-session stability evidence that exists | Partial | 256 MiB transfer with SHA-256 match and 9 MiB peak app working-set growth (`VERIFICATION.md:74-76` — not total WebView memory, not a universal guarantee); 15/15 desktop acceptance groups (`VERIFICATION.md:245`) | Treat as supporting, not substitute for 1.1 |
| 1.3 | Log/retention growth bounded over long runs | Partial | Bounded logs and retention policies operative (`RELEASE-READINESS.md:61`); no soak measurement | Covered by 1.1 |

## 2. Cold-start reliability

| # | Item | STATUS | Evidence | Required action |
| --- | --- | --- | --- | --- |
| 2.1 | Consecutive cold starts render | Partial | 12/12 fresh-profile renders at 26,574-37,438 ms (`artifacts/startup-probe/run-TDPTEn`; `VERIFICATION.md:244`); three earlier blank-document CDP results remain recorded, not erased (`VERIFICATION.md:96-99,103-107,253-258`) | Re-probe on the final signed binary |
| 2.2 | Root cause of slow start (26-49 s) | Partial | Attributed to WebView2 first-run profile creation — hypothesis, not instrumented (`RELEASE_GAP_REGISTER.md:61`); "Cold start in this environment remains slow (~30-49 s) ... reported rather than claimed fixed" (`VERIFICATION.md:226-228`) | Phase 3 phase timings per `RELEASE_PLAN.md:48-55`; [OWNER: accept or fix decision] |
| 2.3 | Blank-document render failure mode | Partial | Blank only with CDP attached; normal launch renders (`VERIFICATION.md:253-256`) | Keep UIA-without-CDP methodology for acceptance probes |

## 3. Permitted-site compatibility sampling

| # | Item | STATUS | Evidence | Required action |
| --- | --- | --- | --- | --- |
| 3.1 | Broad site-compatibility sampling | Not verified | "broad site-compatibility sampling ... not established by the short local suites" (`RELEASE-READINESS.md:72-73`); `RELEASE_PLAN.md:99` (P2 gate); `RELEASE_GAP_REGISTER.md:75` | [OWNER: permitted site list] — only sites the owner is permitted to test; record ranges/redirect/checksum outcomes per site |
| 3.2 | Local fixture coverage that exists | Ready | 46 fixture self-tests, 90 Rust tests incl. segmented/single-stream, ignored ranges, 404, checksum mismatch, cancellation, truncated partials (`VERIFICATION.md:10-29`) | None known |
| 3.3 | Live cross-browser handoff | Partial | Edge + Firefox live acceptance passed with checksum-matched output (`VERIFICATION.md:83-88,179-188`); **Chrome not confirmed** (`RELEASE_GAP_REGISTER.md:42,54`); proxy path verified under `.invalid` (`VERIFICATION.md:47`) | Chrome handoff run or explicit scoped-out decision [OWNER] |

## 4. Clean-machine installation acceptance

| # | Item | STATUS | Evidence | Required action |
| --- | --- | --- | --- | --- |
| 4.1 | Clean Windows install/upgrade/rollback/uninstall | Externally blocked | `RELEASE-READINESS.md:42-44`; `RELEASE_GAP_REGISTER.md:60`; local runs use isolated DB/profile and are not clean-machine tests | [OWNER: clean VM/Sandbox] — `RELEASE_PLAN.md:114`; document run with evidence artifact or keep explicitly blocked |
| 4.2 | Uninstall preserves queue state and user downloads | Not verified | Required by `docs/SUPPORT.md:29-31` ("verify that behavior on a disposable Windows machine for each public release") | Part of 4.1 |
| 4.3 | Administrative MSI extraction/runtime | Partial | Extraction passed; first payload attempt blank, retry passed, then three consecutive extracted-MSI runtime passes (`VERIFICATION.md:101-115`) | Qualified local check only — not clean-machine evidence |

## 5. Update & rollback failure paths

| # | Item | STATUS | Evidence | Required action |
| --- | --- | --- | --- | --- |
| 5.1 | Signed-package verification (valid/tampered/replayed) | Ready | Updater signature + version-replay tests (`VERIFICATION.md:90-93,211-218`) | Keep verification on in release builds |
| 5.2 | Safe failure: unconfigured endpoint performs no network access | Ready | `CONTRACT.md:221-224`; `src/i18n.tsx:90`; `RELEASE-READINESS.md:55` | None known |
| 5.3 | Safe failure: interrupted attempt reconciled on startup; rollback only after validation | Ready | Phase persistence, `reconcile_on_startup`, validated snapshot restore, `INSTALLDIR` preservation, 1 GiB recursive budget; 3 hardening tests (`VERIFICATION.md:211-218`) | None known |
| 5.4 | Real publisher-signed install/update/rollback | Externally blocked | `VERIFICATION.md:90-94`; `RELEASE-READINESS.md:55-57`; `RELEASE_PLAN.md:94-95` | [OWNER: signing certificate + HTTPS endpoints + signing key] |
| 5.5 | Unsigned installer gate | Ready (tooling) / Externally blocked (certificate) | `tools/stage-release.ps1` rejects unsigned artifacts; no cert in user/machine stores (`VERIFICATION.md:127-128`; `RELEASE-READINESS.md:34-37`) | [OWNER: trusted Authenticode certificate + private key] — P0 (`RELEASE_PLAN.md:88`) |

## 6. Data-integrity recovery stability (supporting)

| # | Item | STATUS | Evidence | Required action |
| --- | --- | --- | --- | --- |
| 6.1 | Kill/restart/resume with nonzero-range recovery | Ready | Smoke kill→restart→resume with verified hash (`VERIFICATION.md:40-43`; three extracted-MSI runs each included a killed-process restart) | None known |
| 6.2 | Corrupt/missing working files restart safely | Ready | Transfer + recovery coverage (`VERIFICATION.md:26-29`); 5 focused recovery tests, corrupt DB/WAL/SHM preserved (`VERIFICATION.md:172-174,251`) | None known |
| 6.3 | Full-SQLite restore with consent reset and rollback | Ready | Online restore with safety snapshot; offline restore preserving corrupt bytes; consent reset closes clipboard/browser/scanner/shutdown/Replace re-enable (`VERIFICATION.md:157-167,260-265`) | None known |

## 7. Sign-off

- [ ] [OWNER: soak window] 24-hour run on the final signed binary with a recorded report (1.1)
- [ ] Startup phase timings + [OWNER: accept/fix decision] (2.2)
- [ ] [OWNER: permitted site list] sampled compatibility report (3.1); Chrome handoff decision (3.3)
- [ ] [OWNER: clean VM] install/upgrade/rollback/uninstall evidence (4.1-4.2)
- [ ] [OWNER: certificate/endpoints/keys] signed-channel acceptance (5.4-5.5)

Blocking summary: public release is **NO-GO** while items 4.1, 5.4, and 5.5 (P0) and 1.1, 3.1
(P2) are open (`RELEASE_PLAN.md:85-100`; `RELEASE-READINESS.md:32-57`).