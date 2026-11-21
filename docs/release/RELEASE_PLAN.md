# SwiftFetch — Release Plan (Phase 0 baseline)

**Status:** Plan only. Created during read-only Phase 0; no implementation has started.
**Version:** 0.1.0. **Companion:** `docs/release/RELEASE_GAP_REGISTER.md`.

This plan follows the controlled phases of `D:\Downloads\SWIFTFETCH_IDM_LIKE_RELEASE_MASTER_PROMPT.md`.
It proceeds one bounded task at a time in phase order. Public release remains **NO-GO** until every
P0 gate below is closed. No phase may weaken security/consent, alter schema without approval,
suppress tests, fabricate evidence, or set `ready_for_public_release: true` prematurely.

## Phase 0 — Read-only baseline (THIS PHASE)

**Status: COMPLETE.** Deliverables: this plan + the gap register.
**Completion criteria:** repo/architecture confirmed (both architecture copies byte-identical; no git repo),
commands identified, audit findings reconciled against artifacts, owner-only inputs enumerated, no conflicts found.
No code/config/DB changes were made.

## Execution progress (2026-11-21)

- Phase 0: COMPLETE (read-only baseline; gap register + plan).
- Phase 1: COMPLETE — `tools/verify.ps1` now runs all 83 browser tests and writes
  `artifacts/verification/report-*.json`; `tools/smoke-desktop.mjs` writes a redacted
  `result.json` per run; doc counts corrected to source; verify.ps1 native-stderr
  false-error fixed. Canonical report `report-20261121-150418.json` succeeded.
- Phase 2: Chrome handoff resolved with real run `artifacts/browser-acceptance/run-7T7n8Y/result.json`.
  Other gaps documented (clipboard skip, AV fixture, server-filename source-only) with no
  speculative reimplementation.
- Phase 3: instrumentation added (env-gated Rust phases + frontend performance marks);
  timing harness `tools/startup-timing.mjs`; phase data at
  `artifacts/startup-probe/timing-*/timing.json` shows Rust setup 4.4–8.6 s, so the 26–49 s
  cold render is WebView2 first-paint, not app logic. No perf change applied.
- Phase 4: owner config, clean-machine, and evidence docs complete; signing/updater externally blocked.
- Phase 5: legal/privacy, a11y, stability checklists complete with owner placeholders.
- Phase 6: GO/NO-GO report created with NO-GO verdict; awaiting final verification/build artifact paths.

## Phase 1 — Evidence and test reliability (first implementation phase)

Tasks, in order. For each: state verified gap → expected files → risk → tests before editing.

1. **Wire all browser suites into canonical verification.**
   - Gap: `tools/verify.ps1` runs 70 browser tests; 13 Firefox-background tests are manual-only.
   - Files: `tools/verify.ps1`, `browser-extension/package.json`.
   - Risk: low. Tests: run full `verify.ps1`; total must be 83 browser tests.
2. **Persist machine-readable smoke results.**
   - Gap: no result JSON in `artifacts/smoke/run-*`.
   - Files: `tools/smoke-desktop.mjs` (write `result.json` per run: timestamp, version, group, pass/fail, artifact paths; redact sensitive data).
   - Risk: low. Tests: run smoke; assert JSON exists and non-zero exit on failure.
3. **Correct documentation test counts.**
   - Gap: docs say 34/37/41, 48/55, "six binaries/61"; source has 41 frontend, 57 unit, 70 integration, 83 browser.
   - Files: `VERIFICATION.md`, `COMPLETION.md`, `RELEASE-READINESS.md`.
   - Risk: low (docs only). Tests: counts derived from source match recorded results.
4. **Unique smoke run folder + human-readable summary** (combine with task 2).

**Phase 1 acceptance (from prompt §6):** one documented command produces a reproducible report;
counts match source; all required browser suites run; smoke artifacts persisted; failures exit non-zero.

## Phase 2 — Complete/verify core workflows

- Do not reimplement working features. Fix only evidenced gaps from the gap register:
  direct-media capture path confirmation, Chrome handoff (if a Chrome run is feasible),
  clipboard OS test in an isolated session, server-named download real-run artifact,
  AV adapter documented as fixture-only.
- Each task: verified gap, expected files, risk, deterministic fixture test + reproducible real-app run.
- **Acceptance:** deterministic fixture coverage plus reproducible real-app runs for critical workflows,
  no data/security regression.

## Phase 3 — Startup investigation

- Add narrowly scoped, privacy-safe phase timestamps (process start, Tauri window create, WebView2 create,
  navigation commit, frontend start, React first render, first interactive, engine init, tray, browser bridge).
- Measure warm/cold and fresh/reused WebView2 profiles; separate app time from UIA harness overhead.
- Do not make speculative performance changes. **Acceptance:** reproducible phase timings identify bottleneck;
  propose smallest safe fix + regression tests **before** implementing.

## Phase 4 — Packaging, updates, rollback, installation

- Verify locked deps and reproducible builds; validate signatures/hashes; keep signature verification on.
- Test bad signature/hash, interrupted update, incompatible version, unavailable endpoint, rollback.
- Verify data preservation through install/upgrade/rollback/uninstall.
- Prepare clean-VM test instructions; if unavailable, mark externally blocked (do not simulate success).
- Extension install/update/permissions/removal guidance per browser; no store-approval claim without evidence.
- **Owner-only inputs:** publisher identity/contact, trusted signing cert/key, hosting/endpoint, updater keys,
  approved license/terms/privacy. Never request private keys in chat; provide secure local config guide.
- **Acceptance:** signed installers + real signed update/rollback on clean Windows, or explicit external blocker.
  Never set `ready_for_public_release: true` while gates are open.

## Phase 5 — Legal, privacy, accessibility, stability

- Draft/review license + third-party notices (resolve 7 missing license texts); publisher identity/contact;
  Terms/Privacy/Support; browser permission rationale; local-only diagnostics; secret redaction; no auto-execution.
- WCAG 2.1 AA baseline, keyboard-only, focus/status announcements, real screen-reader test, contrast,
  localization review, 24-hour soak, permitted-site compatibility sampling.
- Never claim legal approval without owner decision. Placeholders only; never invent publisher/license data.

## Phase 6 — Final GO/NO-GO report

- Create `docs/release/GO_NO_GO_REPORT.md` with version/commit (or source snapshot hash, since no git),
  exact commands, test totals/failures, browser results, smoke artifact links, signature/updater status,
  signed update/rollback evidence, clean-machine acceptance, startup measurements,
  security/privacy/legal/a11y/stability status, unresolved risks, owner decisions, explicit **GO/NO-GO**.
- Use **NO-GO** if critical security, data-preservation, signing/update, legal-ownership, or installation gate is unverified.

## Prioritized gate order (what actually blocks public release)

| Priority | Gate | Completion criteria |
|---|---|---|
| P0 | Trusted signing certificate + signed installers | All executables signed; `stage-release.ps1` passes; hashes recorded |
| P0 | HTTPS update channel + keys | All three update env vars + signing key; signed immutable manifests; `configured` state verified |
| P0 | Clean-machine install/upgrade/rollback/uninstall | Documented run with evidence artifact or explicit blocked status |
| P0 | License + publisher identity/terms | Approved docs; 7 license-text gaps resolved |
| P0 | GO/NO-GO report | Exists with explicit verdict; NO-GO if any P0 open |
| P1 | Evidence reliability (Phase 1) | One command; 83 browser tests; smoke JSON; counts correct |
| P1 | Signed update/rollback acceptance | Real signed upgrade + automatic rollback evidence |
| P1 | Browser distribution decision | Permanent install + store review/signing decision |
| P2 | Startup diagnosis | Phase timings identify bottleneck; decision to accept/fix |
| P2 | A11y/i18n certification | SR/contrast report; translation decision |
| P2 | Soak + site compatibility | 24-hour soak result; sampled sites report |

## What can proceed now without owner input

- All of Phase 1 (test reliability, evidence persistence, doc counts).
- Phase 2 gap fixes that use local fixtures/existing artifacts.
- Phase 3 instrumentation and measurement.
- Draft documents/templates in Phase 5 with placeholders.
- GO/NO-GO report skeleton with current NO-GO status.

## What genuinely requires the owner / external resources

- P0 signing certificate/private key (no zero-cost publicly trusted path).
- Update hosting endpoints/archive/public key/signing key.
- Distribution license choice, publisher identity/contact, approved legal docs.
- Clean Windows VM/Sandbox access.
- Browser-store submission/review decision.
- Screen-reader hardware/test personnel; 24-hour soak window.
