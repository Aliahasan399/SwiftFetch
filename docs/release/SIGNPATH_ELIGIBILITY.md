# SwiftFetch — SignPath Foundation eligibility assessment

**Status: informational only. No application is submitted, no approval is claimed, and no publisher
identity is assumed.** This document compares the observable repository state against publicly
published SignPath Foundation conditions and records honest concerns. The owner alone decides
whether to apply and supplies all real publisher/repository facts.

Owner decisions recorded by the parent lane: **individual/personal publisher**, proposed
**GPL-3.0-or-later**, **free download**, **zero budget**. SignPath Foundation is relevant here
because it can provide a publicly trusted Authenticode certificate at no cost for qualifying open
source projects — the repository currently records that no trusted code-signing certificate exists
(`OWNER_CONFIG_GUIDE.md`, `GO_NO_GO_REPORT.md`).

## 1. Public criteria used (source and caveat)

Criteria were checked against the SignPath Foundation "conditions for Open Source projects" page
(`https://signpath.org/terms.html`, retrieved 2026-11-21; the page itself is marked "Draft" for its
code of conduct). Re-verify at the official site before applying — these conditions can change and
are ultimately interpreted by SignPath Foundation, which states it has no obligation to accept a
project.

Conditions for free OSS SignPath.io subscriptions:

- **No malware / no potentially unwanted programs.**
- **OSI-approved OSS license** without commercial dual-licensing for all components.
- **No proprietary code** (system libraries may be included).
- **Maintained** — actively maintained.
- **Released** — already released in the form to be signed.
- **Documented** — functionality described on the download page or store entry.

Additional conditions when the certificate is issued by SignPath Foundation (the normal free path):

- **Sign your own projects and own binaries only**; your team maintains all source and build
  scripts, and any upstream/third-party code follows the fork/upstream-signing rules.
- **No "hacking tools"** as defined by their policy (security diagnosis/exploitation features).
- **Respect privacy/security**; announce system changes; provide uninstallation.
- **Two-factor authentication** for all team members for both SignPath and repository access.
- **Code-signing roles** defined: Authors, Reviewers, Approvers (each signing request manually
  approved).
- **Code signing policy on the project home page**, including the required SignPath attribution,
  team roles, and privacy-policy statement.
- **Artifact configuration**: product-name/product-version metadata set and enforced consistently.
- **Verifiable builds**: binaries built from source in a verifiable way; **every release needs manual
  approval**; cooperate with violation investigations.

Note on the "3+ releases and 3+ contributors" heuristic mentioned in some community summaries: that
numeric threshold does **not** appear in the current official terms page. The official page uses the
qualitative "actively maintained", "already released", and a "verifiable reputation" requirement
("we cannot sign binaries based on source code that nobody knows"). Treat any specific numeric
threshold as unverified/possibly outdated; do not present it as an official rule.

## 2. Observable repository state (read-only inspection)

| Criterion | Observed in `E:\CloneIdm\swiftfetch` | Status |
| --- | --- | --- |
| Public source repository | No `.git` directory in the workspace (`git rev-parse` → "not a git repository"); no public URL supplied; no `.github` workflows present | **Blocker / concern** — a public, verifiable repository is central to eligibility |
| OSI-approved license | Proposed GPL-3.0-or-later, but no `LICENSE` file at repo root; `LEGAL_PRIVACY_CHECKLIST.md` records "No LICENSE file observed" and seven third-party packages missing license texts | **Concern** — license not yet applied; GPL-3.0-or-later is OSI-approved once actually adopted |
| No proprietary/unapproved code | Rust/TypeScript source plus dependencies; third-party license-text gaps exist; no proprietary component identified in inspection, but no formal audit/SBOM evidence in repo | **Partial / unverified** |
| Already released in signable form | 0.1.0 bundles exist (`SwiftFetch_0.1.0_x64-setup.exe`, `SwiftFetch_0.1.0_x64_en-US.msi`), but `candidate-manifest.json` reports `ready_for_public_release: false`; artifacts are **unsigned** per `VERIFICATION.md`; no GitHub release performed | **Concern** — artifacts exist but are not a published, approved release |
| Maintained / verifiable reputation | No git history, no release cadence evidence, no contributor history available locally | **Concern / unverified** |
| Build reproducibility / verifiable build | `tools/build-windows.ps1` builds with `--ci --locked` from a fresh frontend directory and `Cargo.lock` is packaged in the source allowlist — a positive sign — but there is no CI configuration or external evidence that a third party can reproduce the shipped binaries | **Partial / unverified** |
| 2FA for all team members | Not observable in repo; individual publisher | **Unknown** — owner must confirm |
| Defined roles (Authors/Reviewers/Approvers) | Not observable; no repository permission model in a solo/individual project | **Concern** — a solo maintainer can struggle to show independent reviewer/approver separation |
| Code-signing policy on home page | Not present; `docs/TERMS-DRAFT.md`, `docs/PRIVACY.md`, `docs/SUPPORT.md` are drafts/placeholders | **Concern** |
| Privacy / uninstall / system-change notices | Drafts present (`docs/PRIVACY.md`, `docs/SUPPORT.md` documents uninstall/queue preservation), but not published/approved | **Partial** |
| No hacking tools | App is a download manager with browser integration; no exploitation/scanning features observed | **Looks favorable**, subject to final owner review |
| Artifact metadata (product name/version) | Tauri config sets `productName: SwiftFetch`, `version: 0.1.0`; needs enforcement in the actual signed binaries via SignPath artifact configuration | **Partial / not yet configured** |
| Per-release manual approval | Consistent with project's existing release-approval gates, but no SignPath workflow exists | **Not applicable yet** |

## 3. Honest eligibility concerns

1. **No public repository / no git history.** Eligibility and SignPath's automatic origin check
   depend on a public source repository with verifiable build inputs. The workspace is not a git
   repository. This must be resolved by the owner before any application is meaningful.
2. **License not yet applied.** GPL-3.0-or-later is proposed but no `LICENSE` file is committed and
   third-party license-text gaps are open. Adopting an OSI-approved license is a hard condition.
3. **Individual publisher / role separation.** SignPath requires Authors, Reviewers, and Approvers
   with 2FA and manual per-release approval. A single maintainer must explain how independent
   review and approval will work; this is not established here.
4. **No published, approved release.** Current artifacts are unsigned and the candidate manifest is
   `ready_for_public_release: false`. The "already released" and "verifiable reputation" conditions
   are not yet met by evidence in this workspace.
5. **No 2FA evidence.** Cannot be verified locally; owner must confirm for both the repository and
   SignPath access.
6. **Reproducible-build evidence incomplete.** `--ci --locked` and a committed `Cargo.lock` help,
   but no CI pipeline or third-party reproduction evidence exists. SignPath verifies that signed
   binaries are valid automated builds from the repository's source and build scripts.
7. **Publisher identity/contact and policy page missing.** The required code-signing policy section,
   published privacy policy, and owner-defined identity cannot be asserted without invention, which
   is out of scope.
8. **Zero budget is compatible in principle, but not sufficient.** SignPath Foundation's free tier
   fits the owner's zero-budget constraint; acceptance is discretionary and not guaranteed.

## 4. What the owner would need to do (do not perform here)

- Make the project a real public repository with history and a release tag (`v0.1.0`); see
  `GITHUB_RELEASES_SETUP.md`.
- Commit the chosen OSI-approved `LICENSE` and close license-notice gaps.
- Enable 2FA everywhere; define and document Authors/Reviewers/Approvers; add a "Code signing
  policy" section to the project home/download page with the required SignPath attribution and
  privacy statement.
- Produce a clean, reproducible, signed release from CI with enforced product-name/version metadata,
  with manual approval per release.
- Submit an application themselves at `https://signpath.org/apply` and provide all real facts.
  **This document does not submit an application and does not claim approval.**

## 5. Bottom line

SignPath Foundation is a plausible zero-cost route to a trusted Authenticode signature for this
project, but **the project is not shown to be eligible today**: the repository is not public/git-
backed, the license is not applied, there is no published approved release, role separation and 2FA
are unverified, and no reproducible-build or code-signing-policy evidence exists in the workspace.
These are concerns to resolve, not conclusions about the project's merits. No submission or approval
is asserted.

## 6. Sources

- SignPath Foundation — conditions for Open Source projects: `https://signpath.org/terms.html`
  (retrieved 2026-11-21).
- SignPath Foundation — overview: `https://signpath.org/`.
- Local evidence: `RELEASE_PLAN.md`, `GO_NO_GO_REPORT.md`, `LEGAL_PRIVACY_CHECKLIST.md`,
  `OWNER_CONFIG_GUIDE.md`, `VERIFICATION.md`, `tools/build-windows.ps1`, `tools/stage-release.ps1`,
  `src-tauri/tauri.conf.json`, `package.json`.
