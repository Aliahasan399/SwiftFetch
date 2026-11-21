# SwiftFetch — Production Readiness Audit (Phase 1 Baseline)

**Version:** 0.1.0 · **Date:** 2026-11-21 · **Lane:** `release-audit-agent` (read-only audit + this document only)
**Workspace:** `E:\CloneIdm\swiftfetch` · **Audit verdict:** **NO-GO** (unchanged)

## Purpose and scope

This document is an independent, evidence-backed baseline of the 17 Phase-1 release-readiness
items. It re-verifies the repository state directly (git, files, config, workflow YAML, candidate
manifest, live site) rather than trusting narrative reports. It does **not** change source, config,
workflows, or any existing report, and it makes no commit.

Two items were found broken at the start of the audit window and were remediated **by the parent
session concurrently** (while this audit was running). They are recorded here as
"found → parent-fixed" with the exact commits so the baseline remains truthful:

- `dad1566 Fix malformed YAML in staged release workflow`
- `8935967 Correct stale git-repo claim in GO_NO_GO report`

No new release action (tag, draft, publish, key generation, signing) was taken by this lane.

---

## 1. Direct command evidence (captured this audit)

Git state at the **start** of the audit window:

```
$ git status
On branch main
Your branch is up to date with 'origin/main'.
Untracked files:
        .github/workflows/release.yml
nothing added to commit but untracked files present

$ git branch -a
* main
  remotes/origin/main

$ git log --oneline -15
0156277 Add Phase 3 execution and publication reports
832212f Add GitHub Actions Pages workflow for docs/site
29325d6 Keep detailed final license decision report from audit lane
2f41f55 Keep license researcher's detailed upstream-text source index
04635ae Add Phase 2 execution report
d47f172 Add license decision, upstream texts, release workflow and readiness docs
1b9dec6 Add SignPath application readiness package
6899b46 Sanitize personal path in public license audit
43ae415 Document local git preparation and remote-creation boundary
85f82f5 Exclude transient scratch files from version control
8816e80 Initial import: SwiftFetch 0.1.0 release candidate

$ git remote -v
origin  https://github.com/Aliahasan399/SwiftFetch.git (fetch)
origin  https://github.com/Aliahasan399/SwiftFetch.git (push)
```

Git state at the **end** of the audit window (after the parent's two fix commits):

```
$ git status --short
<empty — clean working tree>

$ git log --oneline -5
8935967 Correct stale git-repo claim in GO_NO_GO report
dad1566 Fix malformed YAML in staged release workflow
0156277 Add Phase 3 execution and publication reports
832212f Add GitHub Actions Pages workflow for docs/site
29325d6 Keep detailed final license decision report from audit lane

$ git rev-list --count HEAD
13
$ git branch --show-current
main
```

Other direct checks:

```
$ git config user.name
[OWNER: git author name]
$ git config user.email
[OWNER: git author email]

$ Test-Path LICENSE
False

$ Get-ChildItem -Force -Recurse .github
.github/workflows/pages.yml
.github/workflows/release-draft.yml
.github/workflows/release.yml
.github/workflows/release.yml.bak-20261121-225452
.github/workflows/windows.yml

$ git ls-files | Select-String 'workflow|candidate-manifest|verify.ps1|tauri.conf'
.github/workflows/pages.yml
.github/workflows/release-draft.yml
.github/workflows/windows.yml
src-tauri/tauri.conf.json
tools/verify.ps1
```

Candidate manifest (final candidate `e79ab68b…`), measured directly:

```
ready_for_public_release = False
manifest artifacts        = 626
SHA256SUMS lines          = 626
authenticode values       = NotSigned
```

Live site check performed this audit:

```
Invoke-WebRequest https://aliahasan399.github.io/SwiftFetch/ -UseBasicParsing
=> HTTP 200
```

---

## 2. The 17 Phase-1 audit items

Status legend: **PASS** = verified present/correct · **FAIL** = verified absent/wrong
· **BLOCKED** = externally gated · **WARN** = present but deficient · **INFO** = contextual.

| # | Audit item | Status | Verified fact / evidence |
|---|---|---|---|
| 1 | Project directory | PASS | `E:\CloneIdm\swiftfetch` exists; `package.json`, `src-tauri/`, `tools/`, `docs/release/` present; product `SwiftFetch`, version `0.1.0` (`tauri.conf.json:3-5`). |
| 2 | Git config / author identity | **FAIL** | `git config user.name` = `[OWNER: git author name]`; `git config user.email` = `[OWNER: git author email]` — commits carry placeholder identity. |
| 3 | Git remote | PASS | `origin` = `https://github.com/Aliahasan399/SwiftFetch.git` (fetch + push), matching `GITHUB_PUBLICATION_REPORT.md:9,57`. |
| 4 | Branches | PASS | Local `main` and `remotes/origin/main`; `git status` reports branch up to date with `origin/main`. |
| 5 | Uncommitted changes | PASS (resolved) | At audit start one untracked file: `.github/workflows/release.yml`. Parent committed it in `dad1566` plus the report fix in `8935967`; end-of-audit tree is clean. |
| 6 | Workflow files | WARN | Four workflow-named files (§1). Tracked: `pages.yml`, `release-draft.yml`, `windows.yml`. `release.yml` was untracked and had a malformed YAML `description:` at line 51; parent fixed and committed it (`dad1566`). Two fail-closed draft workflows now coexist (`release-draft.yml` and `release.yml`) — see blocker RB-P1-015. |
| 7 | Tauri config | PASS | `tauri.conf.json:2-5,33-35`: product `SwiftFetch`, version `0.1.0`, identifier `com.swiftfetch.app`, `bundle.active: true`, `targets: "all"`, CSP set (`:27`). |
| 8 | Updater config | **FAIL / BLOCKED** | `tauri.conf.json:31`: `"pubkey": ""`, `requireSignedVersion: true`, `windows.installMode: passive`. No public key, no endpoint, no `.sig` artifacts (cf. `GO_NO_GO_REPORT.md:65`, `ZERO_BUDGET_DISTRIBUTION_PLAN.md:24`). |
| 9 | Installer config | PARTIAL | `targets: "all"` produced MSI + NSIS (`FINAL_BUILD_EVIDENCE.md:31-33`), all `NotSigned`; no updater/signature artifacts (`:35-36`). |
| 10 | Signing integration | BLOCKED | Tooling exists but is inoperative: `tools/build-windows.ps1:23` requires `TAURI_SIGNING_PRIVATE_KEY(_PATH)` for a release channel; `tools/stage-release.ps1:28-30` requires a `Valid` Authenticode signature from a chosen thumbprint; `tools/release-channel.ps1:14-21` requires `.sig` files; `tools/package-candidate.ps1:34-41` records `NotSigned`. No certificate/key present. |
| 11 | Release docs | PASS | `docs/release/` contains 30+ reports; all authoritative docs end in **NO-GO** and `candidate-manifest.json` stays `ready_for_public_release=false`. Note: report copies inside candidate/source snapshots predate final edits (`FINAL_BUILD_EVIDENCE.md:93-100`). |
| 12 | Candidate manifests | PASS (integrity) / BLOCKED (release) | Final candidate `e79ab68b…`: 626 manifest entries = 626 SHA256SUMS lines, `ready_for_public_release=false`, all installers `NotSigned`; independent recheck 0 missing / 0 mismatches (`FINAL_BUILD_EVIDENCE.md:60-65`). |
| 13 | Verify script | PASS (per evidence; not re-run here) | `tools/verify.ps1:35-45` wires 7 checks (HTTP fixture, `cargo check`, `cargo test`, Vite build, UI regressions, browser protocol/release, Firefox background) and writes `artifacts/verification/report-*.json`. Reported exit 0 / `success: true` (`GO_NO_GO_REPORT.md:14-18`, `FINAL_BUILD_EVIDENCE.md:85-91`). This lane did not re-run it. |
| 14 | Owner placeholders | **FAIL** | Scoped recursive scan of source/doc file types (excluding `.git`, `artifacts`, `node_modules`, `target`, `*.bak-*`) found **120 `[OWNER: …]` occurrences across 22 files**; plus git author config (item 2). Examples: `SIGNPATH_APPLICATION_READINESS.md` ×20, `LEGAL_PRIVACY_CHECKLIST.md` ×20, `STABILITY_CHECKLIST.md` ×11, `OWNER_CONFIG_GUIDE.md` ×11, `A11Y_CHECKLIST.md` ×6; root `CONTRIBUTING.md`, `CODE_OF_CONDUCT.md`, `SECURITY.md`; `docs/site/` `support.html`, `download.html`, `privacy.html`, `documentation.html`. |
| 15 | Dependency / licensing | BLOCKED | Root `LICENSE` absent (`Test-Path LICENSE` → `False`). 338 deps audited, GPL-3.0-or-later **CONDITIONALLY APPROVED** (`LICENSE_AUDIT.md:132-143`); CDLA-Permissive-2.0 uncertain (`:89,123-124`), MPL-2.0 ×5 conditional (`:88,125-126`), 7 license texts not bundled (`FINAL_LICENSE_APPROVAL_STATUS.md:20-24`). |
| 16 | GitHub Pages status | PASS | Live check this audit returned **HTTP 200** for `https://aliahasan399.github.io/SwiftFetch/`. Reports confirm all 8 URLs 200 (`PHASE_3_EXECUTION_REPORT.md:22,29-32`, `GITHUB_PAGES_PUBLICATION_REPORT.md`). |
| 17 | Report discrepancies | WARN (partly resolved) | See §3. Primary stale claim corrected by parent (`8935967`); two residual present-tense stale claims remain (RB-P2-016). |

---

## 3. Discrepancy investigation (explicit task item)

### 3.1 The reported discrepancy — confirmed, then remediated by parent

**Finding (as observed at audit start).** `docs/release/GO_NO_GO_REPORT.md:3` originally read:

> `**Version:** 0.1.0 · **Date:** 2026-11-21 · **Repo:** E:\CloneIdm\swiftfetch (not a git repo; source snapshot hash used instead of commit).`

Direct git verification contradicts this. The workspace **is** a Git repository: branch `main`,
11 commits at audit start (`8816e80…0156277`), remote `origin` →
`https://github.com/Aliahasan399/SwiftFetch.git`, `remotes/origin/main` present. Phase 3 reports
already describe a public repo, a push, and matched heads (`PHASE_3_EXECUTION_REPORT.md:34-38`,
`GITHUB_PUBLICATION_REPORT.md:22-32`).

**Root cause.** The sentence is a *stale snapshot description*: it was accurate when the release
evidence was first drafted (Phase 2 began with `git init -b main`, `PHASE_2_EXECUTION_REPORT.md:21,46-49`)
but was never updated after Phase 3 created the remote and pushed. It was a documentation-time
artifact, not a current-state claim — an authoring oversight in a report assembled from earlier
snapshots (cf. `FINAL_BUILD_EVIDENCE.md:93-100`, which explains that snapshot copies of
`docs/release/` predate later edits).

**Recommendation recorded.** Correct the report to state the repository truth (branch, commit
count, remote) and explicitly note the sentence's historical accuracy. This lane did **not** edit
the report, per the task's "parent will fix; do not edit it" instruction.

**Resolution status.** The parent session applied exactly this correction in
commit `8935967` during the audit window. `GO_NO_GO_REPORT.md:3-6` now reads:

```
**Version:** 0.1.0 · **Date:** 2026-11-21 · **Repo:** `E:\CloneIdm\swiftfetch`.
**Correction (Phase 1 audit):** the original snapshot text said "not a git repo; source
snapshot hash used instead of commit". That was accurate when written but is now stale:
the workspace is a Git repository on `main` with a remote at …
```

### 3.2 Broken `release.yml` — confirmed, then remediated by parent

**Finding.** At audit start `.github/workflows/release.yml` was an untracked file whose
`on.workflow_dispatch.inputs.source_zip_dir.description` (line 51) was an unterminated double-quoted
YAML scalar, so GitHub Actions would reject the workflow. A timestamped backup
`.github/workflows/release.yml.bak-20261121-225452` still shows five such unterminated
`description:` lines (28, 37, 41, 46, 51). The tracked, known-good fail-closed draft workflow is
`release-draft.yml`.

**Resolution status.** Parent fixed and committed the file in `dad1566` (`296` lines added);
a post-fix scan found no malformed `description:` lines and the working tree is clean. Residual
risk: `release.yml` and `release-draft.yml` are now two overlapping manual fail-closed draft
workflows (RB-P1-015).

### 3.3 Residual stale claims not fixed by the parent

Two present-tense stale claims remain and should be reviewed by the parent:

- `docs/release/FINAL_BUILD_EVIDENCE.md:3` — still says "not a git repo; source snapshot hash used instead of a commit".
- `docs/release/ZERO_BUDGET_DISTRIBUTION_PLAN.md:3` (and `:49`) — still says "not a git repo" / "The workspace is currently not a git repo."

These are tracked in blocker **RB-P2-016**.

---

## 4. Baseline conclusion

The build/verification/manifest side of production readiness is internally consistent and
independently re-checkable (items 1, 3, 4, 7, 11, 12, 13, 16). The blocking side is dominated by
**externally gated** items — trusted signing, updater key/endpoint/E2E/rollback, clean-machine
acceptance, and legal/license finalization — none of which can be closed by this agent, plus
**owner-controlled** items (git author identity, placeholders, 2FA/Dependabot, SR/soak resources).

The two agent-fixable defects found (stale git claim, malformed `release.yml`) were corrected by
the parent during the audit window. **NO-GO remains the correct verdict**; see
`RELEASE_BLOCKER_REGISTER.md` for the itemized blocker register and per-item verification
procedures.
