# SwiftFetch — Release Blocker Register

**Version:** 0.1.0 · **Date:** 2026-11-21 · **Lane:** `release-audit-agent` (read-only audit + this document only)
**Source audit:** `docs/release/PRODUCTION_READINESS_AUDIT.md` · **Release verdict:** **NO-GO**

## How to read this register

Every blocker has a unique ID with a priority band: **P0** = must be cleared before GO;
**P1** = required by the owner's stated release bar; **P2** = hygiene/accuracy, non-blocking but
should be fixed; **P3** = optional/strategic. Fields are fixed-schema: Description, Severity,
Evidence, Root cause, Required action, Responsible agent, External dependency, Verification
procedure, Status. "Responsible agent" names a *role*, not a person; the owner is accountable for
all items marked `owner`.

Two P0 items were found broken at audit start and were remediated by the parent session during the
audit window (`dad1566`, `8935967`); they are kept here as RESOLVED with their original evidence so
the baseline is auditable. No blocker was closed by this lane.

## Summary

| ID | Blocker | Severity | Responsible agent | External dependency | Status |
|---|---|---|---|---|---|
| RB-P0-001 | Trusted Authenticode signing absent | Critical | signing-agent + owner | SignPath Foundation / certificate | OPEN |
| RB-P0-002 | Updater public/signing key + HTTPS endpoint unconfigured | Critical | updater-agent + owner | owner keygen + HTTPS host | OPEN |
| RB-P0-003 | Real signed update + automatic rollback E2E not tested | Critical | updater-agent + qa-agent | signed artifacts + clean VM | OPEN |
| RB-P0-004 | Clean-machine install/upgrade/rollback/uninstall not accepted | Critical | qa-agent | clean Windows VM | OPEN |
| RB-P0-005 | License not finalized (CDLA, 7 texts, MPL, root LICENSE) | Critical | license-agent + owner | owner/counsel/copyright holder | OPEN |
| RB-P0-006 | Legal docs + publisher identity/contact unresolved | Critical | owner + docs-agent | owner legal approval | OPEN |
| RB-P0-007 | Stale "not a git repo" claim in GO_NO_GO report | High | parent-session | none | RESOLVED (`8935967`) |
| RB-P0-008 | Malformed YAML in staged `release.yml` | High | parent-session | none | RESOLVED (`dad1566`) |
| RB-P1-009 | Screen-reader / contrast certification not verified | High | qa-agent / a11y-agent | SR personnel + tools | OPEN |
| RB-P1-010 | 24-hour soak not performed | High | qa-agent | uninterrupted run window | OPEN |
| RB-P1-011 | Permitted-site compatibility sampling not performed | Medium | qa-agent | representative site list | OPEN |
| RB-P1-012 | Cold-start latency far above <1 s target | High | owner (accept) / perf-agent | owner decision | OPEN (unmet) |
| RB-P1-013 | Dependabot alerts + 2FA not enabled | High | owner | GitHub account settings | OPEN |
| RB-P1-014 | `[OWNER:]` placeholders incl. git author identity | Medium | owner + docs-agent | owner-supplied facts | OPEN |
| RB-P1-015 | Two overlapping fail-closed draft workflows | Medium | release-agent | none | OPEN |
| RB-P2-016 | Residual stale "not a git repo" claims in two reports | Low | docs-agent | none | OPEN |
| RB-P2-017 | Build wrapper false `exit=1` (PowerShell stderr) | Low | parent-session / scripts-agent | none | OPEN (non-blocking) |
| RB-P2-018 | Candidate/source snapshots predate final doc edits | Low | release-agent | gates cleared first | DEFERRED |
| RB-P3-019 | Browser/store distribution strategy undecided | Low | owner | store review (optional) | OPEN (optional) |
| RB-P3-020 | SignPath reproducible-build / solo-maintainer gaps | Low | owner + signing-agent | SignPath review | OPEN |

---

## P0 — Release-blocking

### RB-P0-001 — Trusted Authenticode signing absent
- **Description:** All built executables/installers are `NotSigned`; no publisher trust, so SmartScreen/AV warnings are likely and the attestation gate of the release workflow cannot pass.
- **Severity:** Critical.
- **Evidence:** `docs/release/FINAL_BUILD_EVIDENCE.md:29-36` (exe, MSI, NSIS all `NotSigned`, no `.sig`); `artifacts/SwiftFetch-0.1.0-candidate-e79ab68b…/candidate-manifest.json` `artifacts[].authenticode = "NotSigned"`; `GO_NO_GO_REPORT.md:64,74,86`; `tools/stage-release.ps1:28-30` (requires `Valid` signature); `.github/workflows/release.yml:123-167` (requires `signed: true` attestation).
- **Root cause:** Zero-budget constraint; no SignPath Foundation application submitted, no owner certificate available. `SIGNPATH_APPLICATION_EXECUTION_REPORT.md` = PREPARED / NOT SUBMITTED.
- **Required action:** Obtain a free trusted signing route (SignPath Foundation) or owner-provided certificate; re-sign exe/MSI/NSIS; rebuild candidate manifest with signed hashes + `authenticode: Valid`; stage via `tools/stage-release.ps1`; produce the signed attestation the workflow requires.
- **Responsible agent:** `signing-agent` + `owner` (application submission is owner-gated).
- **External dependency:** SignPath Foundation eligibility/approval, or a purchased/owned certificate + hardware token.
- **Verification procedure:** `Get-AuthenticodeSignature -FilePath <exe|msi>` → `Status = Valid`, `SignerCertificate.Subject` matches chosen publisher, thumbprint matches `stage-release.ps1` input; manifest shows 3 signed artifacts; release workflow attestation gate passes on a dry run.
- **Status:** **OPEN**.

### RB-P0-002 — Updater public/signing key and HTTPS endpoint unconfigured
- **Description:** The Tauri updater has an empty public key and no configured endpoint, so no signed update manifest can be verified or delivered.
- **Severity:** Critical.
- **Evidence:** `src-tauri/tauri.conf.json:31` → `"pubkey": ""`, `requireSignedVersion: true`, `windows.installMode: "passive"`; `GO_NO_GO_REPORT.md:65`; `ZERO_BUDGET_DISTRIBUTION_PLAN.md:24,68`; `UPDATER_KEY_SETUP.md`; `OWNER_CONFIG_GUIDE.md`; `FINAL_BUILD_EVIDENCE.md:18` (no update channel configured, no `.sig`).
- **Root cause:** Owner key generation and endpoint approval not performed; repo secret/env intentionally left unset.
- **Required action:** Generate the Tauri signer keypair locally (private key never committed); embed only the public key in `tauri.conf.json`; host signed version manifests over HTTPS (GitHub Releases path per `GITHUB_RELEASES_SETUP.md`); obtain owner approval before activating an endpoint.
- **Responsible agent:** `updater-agent` + `owner` (key custody and endpoint approval).
- **External dependency:** Owner-generated key material, HTTPS hosting, updated build environment variables.
- **Verification procedure:** `pubkey` non-empty and matches generated public key; `tools/release-channel.ps1` produces a `.sig` that verifies with the Tauri signer; endpoint returns a signed JSON manifest; `requireSignedVersion: true` still enforced.
- **Status:** **OPEN**.

### RB-P0-003 — Real signed update + automatic rollback E2E not tested
- **Description:** Only unit-level updater tests exist; no real signed upgrade and automatic rollback has been exercised end to end.
- **Severity:** Critical.
- **Evidence:** `GO_NO_GO_REPORT.md:67` ("unit-level tests only"), `:75`; `ZERO_BUDGET_DISTRIBUTION_PLAN.md:68`; `AUTO_UPDATE` execution/implementation reports (no keys, endpoint, signed manifests, or E2E).
- **Root cause:** Depends on RB-P0-001/002 (signed artifacts + configured endpoint) and a clean VM (RB-P0-004).
- **Required action:** Publish a signed test release, install it on a clean VM, apply a subsequent signed update, verify checksum/signature acceptance, then corrupt/interrupt the manifest to prove automatic rollback restores the previous version and preserves user data.
- **Responsible agent:** `updater-agent` + `qa-agent`.
- **External dependency:** Signed artifacts, configured HTTPS endpoint, clean Windows VM.
- **Verification procedure:** Dated E2E evidence artifact showing versions before/after, signature verification success, forced-failure rollback to previous version, and intact queue/database.
- **Status:** **OPEN**.

### RB-P0-004 — Clean-machine install/upgrade/rollback/uninstall not accepted
- **Description:** No evidence that install, upgrade, rollback, and uninstall preserve user data on a clean Windows machine.
- **Severity:** Critical.
- **Evidence:** `GO_NO_GO_REPORT.md:66,76`; `CLEAN_MACHINE_ACCEPTANCE.md`; `ZERO_BUDGET_DISTRIBUTION_PLAN.md:61,69`; `PHASE_3_EXECUTION_REPORT.md:62`.
- **Root cause:** No clean Windows VM was available; acceptance checklist unexecuted.
- **Required action:** Provision a clean VM and execute the full `CLEAN_MACHINE_ACCEPTANCE.md` matrix, capturing per-step evidence (fresh install, populated queue, upgrade, rollback, uninstall, residual-data check).
- **Responsible agent:** `qa-agent`.
- **External dependency:** Clean Windows VM / restore-capable lab environment.
- **Verification procedure:** Completed acceptance checklist with dated per-step artifacts and no data-loss findings; reviewer sign-off.
- **Status:** **OPEN**.

### RB-P0-005 — License not finalized (CDLA uncertainty, 7 texts, MPL, root LICENSE)
- **Description:** The project has no root `LICENSE`, the GPL-3.0-or-later choice is only conditional, CDLA-Permissive-2.0 is unresolved, and required license texts are not bundled.
- **Severity:** Critical.
- **Evidence:** `Test-Path LICENSE` → `False`; `FINAL_LICENSE_APPROVAL_STATUS.md:3,9-11,13-38`; `LICENSE_AUDIT.md:89,123-126,132-143`; `ZERO_BUDGET_DISTRIBUTION_PLAN.md:30-40`; `GO_NO_GO_REPORT.md:68,78,89`.
- **Root cause:** Owner/counsel decision pending; text collection and notice regeneration deferred by design until license is chosen.
- **Required action:** (1) owner explicitly selects GPL-3.0-or-later (or alternative); (2) resolve/accept the CDLA-Permissive-2.0 uncertainty for `webpki-roots` ×2; (3) bundle the 7 missing upstream texts, CDLA agreement text, MPL-2.0 text, and GPLv3 text; (4) honor MPL-2.0 notice/source-offer obligations (5 crates); (5) add root `LICENSE`; (6) regenerate notices against the final locked dependency set and update README/website from "proposed" to final.
- **Responsible agent:** `license-agent` + `owner` (license choice and CDLA risk acceptance).
- **External dependency:** Owner/counsel/copyright-holder confirmation; canonical upstream license texts.
- **Verification procedure:** `LICENSE` present at repo root; installer/candidate `notices/` contains all 7 texts + CDLA + MPL + GPLv3; `node tools/dependency-notices.mjs` regenerated; no "proposed" wording remains on site/README; sign-off recorded.
- **Status:** **OPEN**.

### RB-P0-006 — Legal docs and publisher identity/contact unresolved
- **Description:** Terms, Privacy, Support and publisher identity/contact/copyright statements remain drafts/placeholders; no approved legal identity is established for a public release.
- **Severity:** Critical.
- **Evidence:** `GO_NO_GO_REPORT.md:68,78,83,89`; `LEGAL_PRIVACY_CHECKLIST.md`; `PHASE_3_EXECUTION_REPORT.md:54-58`; 120 `[OWNER:]` occurrences across 22 files (audit item 14); git author config = `[OWNER: git author name/email]`.
- **Root cause:** Owner has not supplied real identity/contact/copyright facts; agent prohibited from inventing them.
- **Required action:** Approve Terms/Privacy/Support content; supply publisher identity, contact, copyright holder and years; replace every `[OWNER:]` in root docs, `docs/site/`, and release docs; align published publisher identity with the signing certificate subject (RB-P0-001).
- **Responsible agent:** `owner` + `docs-agent`.
- **External dependency:** Owner-supplied legal facts and approval.
- **Verification procedure:** Repository-wide scan for `[OWNER:` returns 0 in shipped paths; approved legal docs committed; publisher string consistent across site, README, cert, and release metadata.
- **Status:** **OPEN**.

### RB-P0-007 — Stale "not a git repo" claim in GO_NO_GO report
- **Description:** `GO_NO_GO_REPORT.md:3` originally claimed the workspace was "not a git repo; source snapshot hash used instead of commit", contradicting the verified Git repository (`main`, 11 commits at audit start, `origin` → `https://github.com/Aliahasan399/SwiftFetch.git`).
- **Severity:** High (evidence integrity — blocks GO because the report is the gate authority).
- **Evidence:** Original `GO_NO_GO_REPORT.md:3`; direct `git status`/`branch`/`log`/`remote` evidence in `PRODUCTION_READINESS_AUDIT.md` §1–3.1; `PHASE_3_EXECUTION_REPORT.md:34-38`; `GITHUB_PUBLICATION_REPORT.md:22-32`.
- **Root cause:** Stale snapshot description — accurate when Phase 2 began with `git init` (`PHASE_2_EXECUTION_REPORT.md:21,46-49`) but never updated after Phase 3 created the remote and pushed; an authoring oversight in a report assembled from older snapshots.
- **Required action:** Correct the report to state the repository truth and note the statement's historical accuracy. **Done by parent** — commit `8935967` ("Correct stale git-repo claim in GO_NO_GO report"); lines 3-6 now carry the correction.
- **Responsible agent:** `parent-session` (done); this lane was instructed not to edit the report.
- **External dependency:** None.
- **Verification procedure:** `sed`/read lines 3-6 of `GO_NO_GO_REPORT.md` and re-run `git status/branch/log/remote`; confirm agreement. Residual stale claims in other reports are tracked under RB-P2-016.
- **Status:** **RESOLVED by parent (`8935967`)** — parent should re-verify; register retained for audit trail.

### RB-P0-008 — Malformed YAML in staged `release.yml`
- **Description:** At audit start `.github/workflows/release.yml` was untracked and contained an unterminated double-quoted YAML scalar at line 51 (`source_zip_dir.description`), which GitHub Actions would reject; a backup shows the same defect at lines 28, 37, 41, 46, 51.
- **Severity:** High (release automation cannot load).
- **Evidence:** Original `.github/workflows/release.yml:51`; `.github/workflows/release.yml.bak-20261121-225452:28,37,41,46,51`; `git status` at audit start showed the file untracked; tracked good workflow `release-draft.yml:1-70`.
- **Root cause:** Manual authoring error in a staged, not-yet-committed workflow; corrected partially before the backup, remaining line unclosed.
- **Required action:** Fix quoting, validate YAML, and commit the workflow. **Done by parent** — commit `dad1566` ("Fix malformed YAML in staged release workflow") added the 296-line corrected file; post-fix scan found 0 malformed `description:` lines.
- **Responsible agent:** `parent-session` (done).
- **External dependency:** None.
- **Verification procedure:** Re-run the `description:` quote scan and a YAML parse on `release.yml`; confirm workflow appears in `git ls-files`; dry-run `workflow_dispatch` validation in a sandbox. See RB-P1-015 for the residual duplicate-workflow concern.
- **Status:** **RESOLVED by parent (`dad1566`)** — parent should re-verify.

---

## P1 — Required by the owner's release bar

### RB-P1-009 — Screen-reader / contrast certification not verified
- **Description:** No real screen-reader or color-contrast (WCAG 2.2 AA) certification evidence exists.
- **Severity:** High.
- **Evidence:** `GO_NO_GO_REPORT.md:69,78,90`; `A11Y_CHECKLIST.md` (6 `[OWNER:]` placeholders, e.g. `[OWNER: SR personnel]`); `PHASE_3_EXECUTION_REPORT.md:62`.
- **Root cause:** Requires trained personnel and assistive technology not available to the agent.
- **Required action:** Assign SR personnel, execute the `A11Y_CHECKLIST.md` with real screen readers (NVDA/JAWS/Narrator) and contrast measurement, record dated findings and remediate issues.
- **Responsible agent:** `qa-agent` / `a11y-agent` + `owner` (personnel).
- **External dependency:** Screen-reader tools and trained tester.
- **Verification procedure:** Completed, signed `A11Y_CHECKLIST.md` with per-screen results and no unresolved critical findings.
- **Status:** **OPEN**.

### RB-P1-010 — 24-hour soak not performed
- **Description:** No 24-hour stability soak evidence for continuous operation.
- **Severity:** High.
- **Evidence:** `GO_NO_GO_REPORT.md:70,78,90`; `STABILITY_CHECKLIST.md` (11 `[OWNER:]` placeholders); `ZERO_BUDGET_DISTRIBUTION_PLAN.md:71`.
- **Root cause:** Needs an uninterrupted run window and monitoring.
- **Required action:** Run the app for 24 h under representative load (segmented downloads, queue churn), log memory/handles/database health, complete `STABILITY_CHECKLIST.md`.
- **Responsible agent:** `qa-agent`.
- **External dependency:** 24-hour uninterrupted machine/run window.
- **Verification procedure:** Dated soak log with stable resource metrics and completed checklist.
- **Status:** **OPEN**.

### RB-P1-011 — Permitted-site compatibility sampling not performed
- **Description:** No evidence that download handoff works across a representative set of permitted sites.
- **Severity:** Medium.
- **Evidence:** `GO_NO_GO_REPORT.md:70,83`; `STABILITY_CHECKLIST.md`; `ZERO_BUDGET_DISTRIBUTION_PLAN.md:71`.
- **Root cause:** No site sample set or sampling run was executed.
- **Required action:** Define a permitted-site sample and record handoff results per browser (Edge/Firefox/Chrome) per `CLEAN_MACHINE_ACCEPTANCE.md`/stability checklist.
- **Responsible agent:** `qa-agent`.
- **External dependency:** Network access to representative sites.
- **Verification procedure:** Sampling report with site/URL, browser, outcome, and timestamp per row.
- **Status:** **OPEN**.

### RB-P1-012 — Cold-start latency far above <1 s target
- **Description:** Measured cold start ~26–49 s versus the architecture's <1 s target; instrumented traces place Rust setup at ~4.4–8.6 s and the remainder in WebView2 first paint / React mount.
- **Severity:** High (documented product-target deviation).
- **Evidence:** `GO_NO_GO_REPORT.md:27-31,71,77`; `PHASE_2_EXECUTION_REPORT.md:70`; `ZERO_BUDGET_DISTRIBUTION_PLAN.md:72`; `artifacts/startup-probe/timing-*/timing.json`.
- **Root cause:** WebView2 first-paint / React mount after window creation; no speculative fix applied by design.
- **Required action:** Owner either formally accepts the deviation or funds a targeted performance investigation of WebView2 cold start and React mount.
- **Responsible agent:** `owner` (accept/decide) or `perf-agent` (if fixing).
- **External dependency:** Owner decision; possible WebView2 runtime instrumentation.
- **Verification procedure:** Written acceptance in release decision, or re-measured cold start meeting the accepted target.
- **Status:** **OPEN (unmet)**.

### RB-P1-013 — Dependabot alerts + 2FA not enabled
- **Description:** Repository security hardening incomplete: Dependabot alerts disabled (token lacked `admin:repo_hook`) and account 2FA not enabled.
- **Severity:** High.
- **Evidence:** `GITHUB_PUBLICATION_REPORT.md:42-43`; `PHASE_3_EXECUTION_REPORT.md:50-51,54-56`.
- **Root cause:** API token scope limitation and 2FA being a non-API owner action.
- **Required action:** Owner enables 2FA on the GitHub account; enable Dependabot alerts via repo Settings or `gh auth refresh -s admin:repo_hook` then re-apply.
- **Responsible agent:** `owner`.
- **External dependency:** Owner's GitHub account credentials/settings.
- **Verification procedure:** `gh api repos/Aliahasan399/SwiftFetch --jq .security_and_analysis` and Settings screenshots show Dependabot alerts ON and 2FA enabled.
- **Status:** **OPEN**.

### RB-P1-014 — `[OWNER:]` placeholders incl. git author identity
- **Description:** 120 placeholder tokens remain across 22 shipped/source doc files, and local git author identity is still placeholder text, so commits and documents are not attributable to a real publisher.
- **Severity:** Medium.
- **Evidence:** Scoped recursive scan (excludes `.git`, `artifacts`, `node_modules`, `target`, `*.bak-*`): **120 occurrences across 22 files**; `git config user.name` = `[OWNER: git author name]`, `git config user.email` = `[OWNER: git author email]`; `OWNER_CONFIG_GUIDE.md:11` placeholders.
- **Root cause:** Owner facts intentionally withheld; agent must not invent contacts.
- **Required action:** Set real git author identity for future commits (owner to decide whether to rewrite historical authorship); replace all 120 placeholders with approved values.
- **Responsible agent:** `owner` + `docs-agent`.
- **External dependency:** Owner-supplied identity/contact facts.
- **Verification procedure:** Repeat the scoped scan → 0 hits; `git config user.name/email` show real values; spot-check `docs/site/` and root docs.
- **Status:** **OPEN**.

### RB-P1-015 — Two overlapping fail-closed draft workflows
- **Description:** Both tracked `release-draft.yml` and newly committed `release.yml` implement manual, fail-closed draft creation; having two authorities for the same action is ambiguous and risks divergent gates.
- **Severity:** Medium.
- **Evidence:** `.github/workflows/release-draft.yml:1-70` and `.github/workflows/release.yml:1-296`; `git ls-files` lists both.
- **Root cause:** Phase 2 created `release-draft.yml`; parent later staged and committed `release.yml` as a richer replacement without removing the original.
- **Required action:** Choose one authoritative draft workflow, remove or explicitly alias/deprecate the other, and ensure the surviving one carries all fail-closed gates (NO-GO cleared, signed attestation, hash verification, manual publish).
- **Responsible agent:** `release-agent` (owner awareness).
- **External dependency:** None.
- **Verification procedure:** `git ls-files .github/workflows` shows a single release workflow; dry-run dispatch confirms gates fail closed with default inputs.
- **Status:** **OPEN** (residual of parent's `dad1566` fix; no source change made by this lane).

---

## P2 — Hygiene / accuracy (non-blocking)

### RB-P2-016 — Residual stale "not a git repo" claims in two reports
- **Description:** Two reports still assert in present tense that the workspace is not a Git repository, despite the corrected `GO_NO_GO_REPORT.md`.
- **Severity:** Low.
- **Evidence:** `docs/release/FINAL_BUILD_EVIDENCE.md:3` ("not a git repo; source snapshot hash used instead of a commit"); `docs/release/ZERO_BUDGET_DISTRIBUTION_PLAN.md:3` and `:49` ("not a git repo" / "The workspace is currently not a git repo.").
- **Root cause:** Same stale-snapshot authoring pattern as RB-P0-007; parent's correction targeted only the GO report.
- **Required action:** Amend both lines to reference the verified repo (`main`, remote `origin`, commit count) or mark them explicitly historical with a date qualifier; cross-reference `PRODUCTION_READINESS_AUDIT.md` §3.1.
- **Responsible agent:** `docs-agent` / `parent-session`.
- **External dependency:** None.
- **Verification procedure:** Scan `docs/release/*.md` for "not a git repo"; remaining hits must be clearly marked historical.
- **Status:** **OPEN**.

### RB-P2-017 — Build wrapper false `exit=1` (PowerShell stderr redirection)
- **Description:** `tools/build-windows.ps1` can report `exit=1` because npm informational stderr is treated as terminating under `$ErrorActionPreference='Stop'` with `*>` redirection, even when the build succeeds.
- **Severity:** Low (known false error; not a build failure).
- **Evidence:** `FINAL_BUILD_EVIDENCE.md:20-25`; `GO_NO_GO_REPORT.md:50-52`; contrast with `tools/verify.ps1:9-13` which already relaxes `$ErrorActionPreference` around native calls.
- **Root cause:** Native stderr handling in the wrapper script.
- **Required action:** Adopt the `verify.ps1` pattern (temporarily set `$ErrorActionPreference='Continue'` around the native call, trust `$LASTEXITCODE`) in the build wrapper.
- **Responsible agent:** `parent-session` / `scripts-agent`.
- **External dependency:** None.
- **Verification procedure:** Re-run the wrapper on a clean build and confirm `exit=0` with matching artifact hashes.
- **Status:** **OPEN (documented, non-blocking)**.

### RB-P2-018 — Candidate/source snapshots predate final doc edits
- **Description:** The candidate and source snapshots were packaged before the final `GO_NO_GO_REPORT.md` update; their embedded `docs/release/` copies are stale.
- **Severity:** Low.
- **Evidence:** `FINAL_BUILD_EVIDENCE.md:93-100` (ordering note; backup `GO_NO_GO_REPORT.md.bak-20261121-190606`); `GO_NO_GO_REPORT.md:54-55`.
- **Root cause:** Packaging ran before documentation was finalized; rebuilding would invalidate recorded hashes.
- **Required action:** Only when all P0 gates are cleared, regenerate candidate and source snapshots so embedded docs are current, and update manifest/SHA256SUMS accordingly.
- **Responsible agent:** `release-agent`.
- **External dependency:** P0 gates cleared (otherwise re-packaging is premature).
- **Verification procedure:** Fresh manifest with 0 hash mismatches and embedded docs matching working-tree `docs/release/`.
- **Status:** **DEFERRED / ACCEPTED** for the current NO-GO baseline.

---

## P3 — Optional / strategic

### RB-P3-019 — Browser/store distribution strategy undecided
- **Description:** No decision recorded on publishing browser extensions to stores or seeking store review, versus direct download.
- **Severity:** Low (optional).
- **Evidence:** `GO_NO_GO_REPORT.md:83`; `PHASE_3_EXECUTION_REPORT.md:62-63`; browser previews are `*-preview.zip` in the candidate (`FINAL_BUILD_EVIDENCE.md:66-69`).
- **Root cause:** Owner preference not yet expressed; store review is not required for direct download.
- **Required action:** Owner decides store vs direct-only distribution before any store submission is attempted.
- **Responsible agent:** `owner`.
- **External dependency:** Store review if pursued.
- **Verification procedure:** Recorded decision in `RELEASE_PLAN.md`.
- **Status:** **OPEN (optional)**.

### RB-P3-020 — SignPath reproducible-build / solo-maintainer gaps
- **Description:** SignPath Foundation eligibility gaps remain: no root LICENSE (RB-P0-005), unsigned candidate (RB-P0-001), no clean-VM acceptance (RB-P0-004), solo maintainer/reviewer, 2FA status uncertain, and no reproducible-build CI proof.
- **Severity:** Low (gates a P0 dependency).
- **Evidence:** `PHASE_2_EXECUTION_REPORT.md:62-64`; `SIGNPATH_ELIGIBILITY.md`; `SIGNPATH_APPLICATION_READINESS.md`; `GO_NO_GO_REPORT.md:81-83`.
- **Root cause:** Application intentionally not submitted; prerequisite gates unsatisfied.
- **Required action:** After P0-001/004/005 and RB-P1-013 are cleared, reassess eligibility, add reproducible-build evidence (pinned toolchain/CI), and have the owner decide on submission.
- **Responsible agent:** `owner` + `signing-agent`.
- **External dependency:** SignPath Foundation review.
- **Verification procedure:** Signed application receipt or documented owner decision to use an alternative signing route.
- **Status:** **OPEN**.

---

## Gate-to-GO mapping

The GO conditions in `GO_NO_GO_REPORT.md:85-91` map to this register as follows:

| GO condition (report lines) | Blockers that must reach CLOSED |
|---|---|
| 1 — Signed exe/MSI/NSIS with valid publisher | RB-P0-001, RB-P3-020 |
| 2 — HTTPS update channel, signed manifests, real upgrade + rollback on clean VM | RB-P0-002, RB-P0-003, RB-P0-004 |
| 3 — Documented clean-machine acceptance | RB-P0-004 |
| 4 — Approved license, publisher identity, legal docs, 7 gaps resolved | RB-P0-005, RB-P0-006, RB-P1-014 |
| 5 — Real SR/contrast and 24 h soak evidence; startup target accepted/fixed | RB-P1-009, RB-P1-010, RB-P1-012, RB-P1-011 |
| 6 — `ready_for_public_release: true` only after all above | Overall verdict; plus RB-P1-013, RB-P1-015, RB-P0-007/008 (resolution integrity), RB-P2-016/017/018 |

**Current verdict: NO-GO.** No P0 blocker was closed by this lane. Two P0 items (RB-P0-007, RB-P0-008)
were closed by the parent during the audit window and are retained here with evidence for auditability.
