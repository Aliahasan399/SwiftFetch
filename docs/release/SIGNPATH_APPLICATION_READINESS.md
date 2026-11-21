# SwiftFetch — SignPath Foundation application-readiness package

**Status: NOT SUBMITTED. REQUIRES OWNER APPROVAL. NO APPROVAL IS CLAIMED.**

**Prepared:** 2026-11-21 · **Version:** 0.1.0 · **Lane:** signpath-application-preparer
**Workspace inspected:** `E:\CloneIdm\swiftfetch` (read-only). **Write scope:** this file only — no
source/config changes, no commits, no certificate request, no application submission, no invented
facts.

This package prepares the *materials and an honest gap analysis* for a possible SignPath Foundation
application. It does **not** apply, does **not** represent the owner, and does **not** assert that
SwiftFetch qualifies. Every real publisher, repository, contact, and security fact is left to the
owner as a `[OWNER: ...]` placeholder or an explicit "unknown/unverified" finding.

## Status legend (applies to every finding below)

| Tag | Meaning |
| --- | --- |
| **COMPLETED** | Verified against official source or local evidence in this session; no owner action needed for the *documentation* fact. |
| **PREPARED** | A draft/template exists but is not submitted and is not a claim of fact. |
| **BLOCKED** | A hard, observable gap prevents a credible application today. |
| **REQUIRES OWNER APPROVAL** | Cannot be asserted or performed without the owner's real decision/fact/action. |

---

## 1. Official reference URLs and date verified

All SignPath criteria below were fetched directly from the official site on **2026-11-21**
(today's date in this environment) and cross-checked against the existing local assessment.

| Reference | URL | Purpose | Verification |
| --- | --- | --- | --- |
| SignPath Foundation terms / Code of Conduct | `https://signpath.org/terms.html` | Authoritative "conditions for Open Source projects" and certificate promises | **COMPLETED** — fetched 2026-11-21. Note: the page itself marks its *Code of conduct* section as **"Draft"**. |
| SignPath Foundation overview | `https://signpath.org/` | Program purpose, "free of charge", no personal identification needed | **COMPLETED** — fetched 2026-11-21 |
| Application entry point | `https://signpath.org/apply` | Where the owner would apply | **COMPLETED** — fetched 2026-11-21. The page is a form; no public field list is published, so Section 10 answers are *anticipated*, not confirmed. |
| SignPath.io OSS solution | `https://signpath.io/solutions/open-source-community` | CI/CD integration, HSM key custody, EXE/MSI support | PREPARED — cited via third-party registry, not re-fetched directly; **re-verify before applying** |
| Accepted projects (examples) | `https://signpath.org/projects` | Illustrates the "verifiable reputation" expectation | PREPARED — not fetched in this session; **re-verify** |
| Local prior assessment | `docs/release/SIGNPATH_ELIGIBILITY.md` | Baseline concerns; its criteria section matches the official site as re-verified today | COMPLETED — read 2026-11-21 |

**Caveat (REQUIRES OWNER APPROVAL / COMPLETED):** the official page states SignPath Foundation
is under **no obligation to accept any project**, makes the final decision itself with no
independent arbitration, and may change these conditions. Re-check the official URLs immediately
before any submission. The "3+ releases / 3+ contributors" rule sometimes cited in community
summaries does **not** appear on the current official terms page; it is treated here as
**unverified/outdated** and is not presented as a rule.

---

## 2. Requirement-by-requirement assessment (official criteria vs. actual state)

Every row is a distinct finding with its status.

| # | Official requirement (2026-11-21) | SwiftFetch ACTUAL state | Status |
| --- | --- | --- | --- |
| 1 | No malware / no potentially unwanted programs | Download manager with browser handoff; no exploitation/scanning features observed. No formal malware scan/PUP review asserted. | **PREPARED** (favorable observation only; owner confirm) |
| 2 | OSI-approved open-source license, no commercial dual licensing | GPL-3.0-or-later **proposed and CONDITIONALLY APPROVED** by `LICENSE_AUDIT.md`, but **no `LICENSE` file at repo root**. OSI approval attaches only once actually adopted. | **BLOCKED** / **REQUIRES OWNER APPROVAL** |
| 3 | No proprietary code (system libraries allowed) | Rust/TypeScript + dependencies; no proprietary component identified. SBOM shows 338 deps; 7 missing license *texts*, CDLA-Permissive-2.0 ×2 uncertain. No formal "no proprietary code" attestation. | **PREPARED** (partial/unverified) |
| 4 | Actively maintained | No git history, no release cadence, no contributor record in workspace. | **BLOCKED** (no evidence) / **REQUIRES OWNER APPROVAL** |
| 5 | Already released **in the form to be signed** | 0.1.0 NSIS/MSI bundles exist but are **unsigned** (`NotSigned`), `candidate-manifest.json` = `ready_for_public_release: false`, no GitHub release published. | **BLOCKED** |
| 6 | Documented on download page / store entry | `docs/site/` pages prepared with placeholders; not published / no download page live. | **PREPARED** / **REQUIRES OWNER APPROVAL** |
| 7 | Sign only your own projects and own binaries | App is first-party; dependencies are upstream OSS, included as libraries (official terms permit unsigned upstream binaries in signed packages). Fork/upstream rules appear not to apply. | **PREPARED** (favorable; owner confirm) |
| 8 | No "hacking tools" | No vulnerability-scanning/exploitation features observed. | **PREPARED** (favorable; owner confirm) |
| 9 | Respect privacy/security; announce changes; provide uninstallation | `docs/PRIVACY.md`, `docs/SUPPORT.md` drafts exist (uninstall/queue preservation described); not published/approved. No telemetry required. | **PREPARED** / **REQUIRES OWNER APPROVAL** |
| 10 | Two-factor / multi-factor auth for all team members on **both** SignPath and repo | Not observable locally. Individual publisher. No 2FA evidence. | **REQUIRES OWNER APPROVAL** (unknown) |
| 11 | Defined roles: Authors, Reviewers, Approvers; manual approval per signing request | No permission model; solo maintainer. Not established. | **BLOCKED** / **REQUIRES OWNER APPROVAL** |
| 12 | Code signing policy on project home page (with exact attribution, roles, privacy statement) | No such section published. Draft docs exist only. | **BLOCKED** / **REQUIRES OWNER APPROVAL** |
| 13 | Artifact config: product name/version metadata set and **enforced** | Tauri config sets `productName: SwiftFetch`, `version: 0.1.0`; **no SignPath artifact configuration exists yet**; enforcement not evidenced. | **PREPARED** / **BLOCKED** |
| 14 | Binaries built from source in a **verifiable** way; every release manually approved; cooperate with investigations | Build inputs are pinned (`--ci --locked`, committed `Cargo.lock`, pinned action SHAs, CI workflow), but no public repo, no clean-room reproduction, no third-party evidence, no SignPath integration. | **BLOCKED** (no verifiable public origin) / **PREPARED** (build inputs) |
| 15 | Verifiable reputation ("cannot sign binaries based on source code that nobody knows") | No public history, releases, contributors, or downloads to evidence reputation. | **BLOCKED** / **REQUIRES OWNER APPROVAL** |

---

## 3. Detailed findings for the specifically requested topics

### 3.1 Is an individual / personal publisher eligible? — **REQUIRES OWNER APPROVAL** (not disqualifying by itself)

The official program does **not** issue a certificate to a person or project. It states plainly:
*"Since we are not a Certificate Authority (CA), we cannot issue certificates to you... What we
actually do is get certificates issued to 'SignPath Foundation' and let OSS projects use them."*
The overview adds *"No need for personal identification, we verify that the binary was built from
your open source repository and vouch for that with our name."*

Therefore:
- The recorded owner decision "individual / personal publisher" does **not**, by itself, bar an
  application — SignPath Foundation becomes the publisher of record, not the individual.
- However, the *project* must satisfy every condition, and a solo maintainer must still show MFA,
  role separation (Authors/Reviewers/Approvers), manual approval, and a verifiable public origin.
  The official terms say the *team* responsible for signing must own the source repository.
- The owner must confirm they control the future public repository and can act as the project's
  authorized applicant. **No identity is assumed here.**

### 3.2 Does the project qualify as open source? — **PREPARED / BLOCKED**

Potentially yes, once the conditions are actually met: the intended license GPL-3.0-or-later is
OSI-approved; the dependency audit found no proprietary/AGPL/SSPL/GPL-2.0-only component. But
qualification is **not established today** because (a) the `LICENSE` file is absent, (b) seven
license texts and the CDLA-Permissive-2.0 uncertainty remain open, and (c) there is no public repo
for SignPath to inspect. Conditionality is documented in `LICENSE_AUDIT.md` §7.

### 3.3 Public GitHub (source) repository requirement — **BLOCKED**

SignPath's automatic origin check requires a **public source repository** containing the source,
build scripts, and CI configuration; a signature confirms the binary is a valid automated build
from that repository. Actual state:
- `git rev-parse` → "fatal: not a git repository"; `Test-Path .git` → `False`.
- No public URL supplied.
- `.github/workflows/windows.yml` **does exist** (see 3.5) — this corrects the older
  `SIGNPATH_ELIGIBILITY.md` line 56 statement that "no `.github` workflows present". The workflow
  is local only until the project is put under git and published.

This is the single largest blocker: without a public repo there is no verifiable origin, no
inspectable CI, and no reputation signal. **REQUIRES OWNER APPROVAL** to create/point to the public
repository.

### 3.4 Public license requirement — **BLOCKED / REQUIRES OWNER APPROVAL**

An OSI-approved license without commercial dual licensing is a hard condition for *all* components.
GPL-3.0-or-later would satisfy it **once adopted**. Today there is **no root `LICENSE` file**, and
the audit verdict is explicitly **conditional**. Owner must (a) confirm the license choice after
resolving the CDLA uncertainty, (b) add the GPL-3.0 text at repo root, (c) bundle the seven missing
upstream texts and third-party notices in distributed artifacts. See `ZERO_BUDGET_DISTRIBUTION_PLAN.md`
§"Remaining owner actions" steps 1–2.

### 3.5 Reproducible / verifiable build requirement — **PREPARED (inputs) / BLOCKED (evidence)**

Official constraint: *"Binary artifacts must be built from source code in a verifiable way"* and,
for each release, SignPath verifies the binary is a valid automated build from repository source.
Verifiable positive inputs observed:
- `.github/workflows/windows.yml`: runs on `windows-2022`; triggers on PR, push to `main`, and
  manual dispatch; `permissions: contents: read`; actions pinned to full commit SHAs;
  `actions/checkout` with `persist-credentials: false`; `npm ci`; Rust toolchain pinned to
  `1.98.0`; Python `3.12`; `./tools/verify.ps1`; `./tools/build-windows.ps1`;
  `node tools/smoke-desktop.mjs`.
- `tools/build-windows.ps1`: `npm run tauri build -- --ci ... -- --locked`, fresh
  `artifacts/candidate-<guid>/ui` frontend directory to avoid stale assets, optional `-SigningConfig`.
- `src-tauri/Cargo.lock` present; `package-lock.json` present; Cargo release profile uses
  `lto=true`, `codegen-units=1`, `panic="abort"`, `strip=true`.

Missing evidence: no public repo exposing these files, no clean-VM build, no byte-for-byte or
hash Comparison across independent builds, no SBOM/attestation tied to a tag, no SignPath artifact
configuration, no manual-approval workflow. The older eligibility doc's "no reproducible-build CI
evidence" is **partly outdated**: a CI workflow now exists, but reproducibility of the shipped
binaries is still **unproven**.

### 3.6 Account security (2FA/MFA) requirement — **REQUIRES OWNER APPROVAL**

Official terms: *"All team members must use multi-factor authentication for both SignPath and
source code repository access (e.g. GitHub)."* Not observable in the workspace; no evidence for
either system. Owner must enable and be able to affirm MFA for the repository account and the
SignPath account, and define how role accounts are protected. **No MFA status is asserted.**

### 3.7 Clean-machine acceptance — **BLOCKED**

`docs/release/CLEAN_MACHINE_ACCEPTANCE.md` is **externally blocked (no clean Windows VM)** with no
companion `clean-machine-result.json`. Install/upgrade/rollback/uninstall on a clean VM, signature
verification, and 24-hour soak are not done. SignPath's "released in the form to be signed" and
reputation expectations are weakened by this, independently of the signing gap.

---

## 4. Gap analysis summary (actual state vs. application readiness)

| Gap | Actual state | Blocker severity | Status |
| --- | --- | --- | --- |
| Not a git repo / no public repository | Confirmed (`git rev-parse` fails, no `.git`) | **Hard blocker** — origin verification impossible | **BLOCKED** / **REQUIRES OWNER APPROVAL** |
| Root `LICENSE` absent; license conditional | Proposed GPL-3.0-or-later only; audit conditional; 7 text gaps; CDLA uncertain | **Hard blocker** — OSI license is a condition | **BLOCKED** / **REQUIRES OWNER APPROVAL** |
| Unsigned candidate | All three binaries `NotSigned`; `ready_for_public_release: false` | **Hard blocker** — "released in signable form" not met (this is the very thing SignPath would fix, but a published, approved release is still expected) | **BLOCKED** |
| No clean VM | Externally blocked; no result file | Major — release/acceptance evidence missing | **BLOCKED** |
| Individual publisher / role separation | Solo maintainer; no Authors/Reviewers/Approvers model | Major — official role requirement | **REQUIRES OWNER APPROVAL** |
| 2FA unknown | No evidence for repo or SignPath | Major — explicit condition | **REQUIRES OWNER APPROVAL** |
| Reproducible-build CI evidence | Workflow + pinned inputs exist; no clean-room/hash/tag evidence, no SignPath integration | Major — verifiable build unproven | **PREPARED** (inputs) / **BLOCKED** (proof) |
| Verifiable reputation | No public history, contributors, releases, downloads | Major — discretionary acceptance risk | **REQUIRES OWNER APPROVAL** |
| Code signing policy page + attribution | Not present; drafts only | Major — explicit website condition | **BLOCKED** / **REQUIRES OWNER APPROVAL** |
| Artifact metadata enforcement | Tauri product name/version set; no SignPath artifact config | Moderate — configurable later | **PREPARED** |

**Bottom line:** the project is **not application-ready today**. Most gaps are owner-controlled
(repository, license, MFA, roles, policy page, publication); the workflow/build inputs are a solid
foundation but not proof of a verifiable release. Nothing here changes the existing **NO-GO**
release verdict.

---

## 5. Prepared application materials (templates — NOT submitted)

All bracketed values are deliberately placeholders. **PREPARED** throughout; filling them is
**REQUIRES OWNER APPROVAL**.

### 5.1 Project description (draft for the application's "describe your project" field)

> SwiftFetch is a free Windows desktop download manager built with Tauri 2, React 19, TypeScript,
> and Rust. Users add an HTTP/HTTPS link to a local queue and can pause, retry, schedule, set
> transfer limits, and verify downloads with a trusted checksum. The transfer engine uses multiple
> connections where a server supports range requests and falls back to a single stream otherwise.
> Queue state, history, and checkpoints are kept in a local SQLite database under the user's
> `%APPDATA%\swiftfetch` directory. An optional browser handoff uses Windows Native Messaging and
> requires explicit desktop opt-in. There is no required account, hosted backend, telemetry, or paid
> service. Version 0.1.0 produces an NSIS installer and an MSI. Publisher/contact: [OWNER: name or
> alias], [OWNER: email].
>
> Intended license: GPL-3.0-or-later (proposed, conditionally approved pending the open items in
> `LICENSE_AUDIT.md`). Current status: unsigned private release candidate, NO-GO for public release.

### 5.2 Open-source project description (draft for "why is this open source / OSS details")

> SwiftFetch is developed as a fully open-source, free-to-download project. Its complete source
> (Rust backend, React/TypeScript frontend, browser extension, build scripts, and CI configuration)
> is intended to live in a public repository at [OWNER: public repository URL] under the
> [OWNER: confirm GPL-3.0-or-later] license. All 338 third-party dependencies are open source under
> OSI-approved or otherwise GPLv3-compatible licenses; a dependency license audit and SBOM are
> maintained in the repository (`docs/release/LICENSE_AUDIT.md`, `artifacts/dependencies/`). The
> project has no proprietary or commercially dual-licensed components. Contributions are governed
> by [OWNER: CONTRIBUTING.md URL]; community standards by [OWNER: CODE_OF_CONDUCT.md URL]; security
> reports by [OWNER: SECURITY.md / private vulnerability reporting URL]. Builds are intended to be
> reproducible from the public repository with pinned toolchains and locked dependencies.

### 5.3 Repository / project information placeholders

| Field | Value to be supplied by owner |
| --- | --- |
| Public source repository URL | `[OWNER: https://github.com/<owner>/swiftfetch]` |
| Default branch | `[OWNER: main]` |
| Release tag for candidate | `[OWNER: v0.1.0]` |
| Download / project home page | `[OWNER: https://<owner>.github.io/swiftfetch/ or equivalent]` |
| License file URL | `[OWNER: .../LICENSE]` |
| Code signing policy URL | `[OWNER: .../CODE_SIGNING_POLICY.md or home-page section]` |
| Privacy policy URL | `[OWNER: .../privacy.html]` |
| Contributor / team roles page | `[OWNER: teams/permissions URL]` |
| Applicant name / email / role | `[OWNER: name] / [OWNER: email] / [OWNER: maintainer role]` |
| SignPath account email + MFA status | `[OWNER: email] / [OWNER: MFA enabled yes/no]` |

### 5.4 Build process description (draft, grounded in observed files)

> Prerequisites: Windows 11 x64, Microsoft C++ Build Tools (Desktop development with C++), WebView2
> Runtime, Rust 1.98.0 (`x86_64-pc-windows-msvc`), Node.js 24+, Python 3.12.
>
> From a clean checkout: `npm ci` → `./tools/verify.ps1` → `./tools/build-windows.ps1` →
> `node tools/smoke-desktop.mjs`. `tools/build-windows.ps1` invokes
> `npm run tauri build -- --ci --config <generated overlay> -- --locked`, building the frontend
> into a fresh `artifacts/candidate-<guid>/ui` directory so stale assets cannot enter the bundle.
> Dependencies are locked by `package-lock.json` and `src-tauri/Cargo.lock`; release Rust profile
> uses LTO, single codegen unit, `panic=abort`, and symbol stripping. GitHub Actions
> (`.github/workflows/windows.yml`) runs the same sequence on `windows-2022` with pinned action
> SHAs and `persist-credentials: false`. Outputs: `SwiftFetch_0.1.0_x64-setup.exe` (NSIS) and
> `SwiftFetch_0.1.0_x64_en-US.msi`, plus the unsigned `swiftfetch.exe`. Signing is **not** currently
> configured; a `-SigningConfig` hook exists in the build script for a future certificate path.

---

## 6. Reproducibility evidence requirements (what the owner must produce)

**PREPARED** list; producing the evidence is **REQUIRES OWNER APPROVAL / BLOCKED** until a public
repo exists.

1. A public repository at a fixed commit/tag containing source, `Cargo.lock`, `package-lock.json`,
   `tauri.conf.json`, `tools/build-windows.ps1`, and `.github/workflows/windows.yml`.
2. A documented, single build command reproducible by a third party (Section 5.4).
3. Two independent clean builds (e.g. maintainer machine + clean GitHub Actions runner) of the
   same commit producing binaries whose SHA-256 hashes are compared and recorded. *Note: Tauri/
   MSVC builds are not guaranteed byte-identical; the goal is a verifiable, documented mapping from
   source → binary, which is what SignPath checks.*
4. A `SHA256SUMS` file for the NSIS, MSI, and `swiftfetch.exe`, published with the release.
5. A generated SBOM / dependency notices artifact tied to the exact tag (`artifacts/dependencies/`).
6. A release tag `v0.1.0` and a published release with documented notes (per
   `docs/release/GITHUB_RELEASES_SETUP.md`), only after `ready_for_public_release` is truthfully set.
7. A SignPath artifact configuration that sets and enforces product name `SwiftFetch` and product
   version `0.1.0` on every signed binary.
8. A defined manual-approval gate for each signing request, with named Authors/Reviewers/Approvers.
9. Clean-machine acceptance evidence with real signature verification
   (`docs/release/CLEAN_MACHINE_ACCEPTANCE.md` + `clean-machine-result.json`).

---

## 7. Signing application checklist

| # | Item | Status |
| --- | --- | --- |
| 1 | Re-read official terms/conditions on `signpath.org/terms.html` immediately before applying | **COMPLETED** (today) — re-verify at submission |
| 2 | Decide/confirm GPL-3.0-or-later and resolve CDLA uncertainty | **REQUIRES OWNER APPROVAL** |
| 3 | Add root `LICENSE` and bundle all third-party license texts | **BLOCKED** / **REQUIRES OWNER APPROVAL** |
| 4 | Initialize and publish a clean public git repo with history and tag `v0.1.0` | **BLOCKED** / **REQUIRES OWNER APPROVAL** |
| 5 | Enable MFA on repository and SignPath accounts | **REQUIRES OWNER APPROVAL** |
| 6 | Define Authors / Reviewers / Approvers and manual release approval | **REQUIRES OWNER APPROVAL** |
| 7 | Publish code signing policy with exact SignPath attribution + privacy statement | **BLOCKED** / **REQUIRES OWNER APPROVAL** |
| 8 | Publish download page documenting functionality | **PREPARED** / **REQUIRES OWNER APPROVAL** |
| 9 | Produce clean-room reproducibility + hash evidence (Section 6) | **BLOCKED** |
| 10 | Produce clean-VM acceptance evidence | **BLOCKED** |
| 11 | Configure SignPath artifact metadata enforcement | **BLOCKED** (no account yet) |
| 12 | Fill all `[OWNER: ...]` placeholders in Sections 5.1–5.3 | **REQUIRES OWNER APPROVAL** |
| 13 | Owner submits application at `https://signpath.org/apply` personally | **REQUIRES OWNER APPROVAL** — NOT DONE |
| 14 | Record the real decision (accepted / rejected / request for evidence) without claiming success | **REQUIRES OWNER APPROVAL** |

---

## 8. Honest answers to likely application questions

These are **PREPARED** drafts. Answers that depend on the owner's real facts are marked
**REQUIRES OWNER APPROVAL** and use placeholders. Do not submit them as-is.

- **"Is your project open source?"** — Intended yes: public source, free download, OSI-approved
  GPL-3.0-or-later. **Caveat:** license not yet adopted and repo not yet public.
  **BLOCKED / REQUIRES OWNER APPROVAL.**
- **"What license and where is the license file?"** — Proposed GPL-3.0-or-later; `LICENSE` file not
  present at root. **BLOCKED / REQUIRES OWNER APPROVAL.**
- **"Where is the public repository?"** — None today; workspace is not a git repo.
  `[OWNER: public URL]`. **BLOCKED.**
- **"Has the project been released?"** — 0.1.0 artifacts built but unpublished, unsigned, candidate
  manifest `ready_for_public_release: false`. **BLOCKED.**
- **"Who maintains the project / who is on the team?"** — Individual maintainer
  `[OWNER: name/alias]`; no multi-person team recorded. **REQUIRES OWNER APPROVAL.**
- **"Do you use MFA?"** — Unknown locally; must be affirmed by owner for repo + SignPath.
  **REQUIRES OWNER APPROVAL.**
- **"Who are the Authors, Reviewers, Approvers?"** — Not defined. Owner must assign.
  **REQUIRES OWNER APPROVAL.**
- **"Can a third party rebuild your binary from source?"** — Inputs are pinned and CI exists, but no
  public repo or clean-room reproduction evidence yet. **PREPARED / BLOCKED.**
- **"Does the software collect or transmit user data?"** — No telemetry/account; local SQLite;
  browser handoff only after explicit action; privacy draft prepared but not published.
  **PREPARED / REQUIRES OWNER APPROVAL.**
- **"Does it change system configuration or include an uninstaller?"** — Optional native-messaging
  registration is opt-in; NSIS/MSI provide uninstall; drafts describe uninstall/data preservation.
  **PREPARED / REQUIRES OWNER APPROVAL.**
- **"Is it a security/hacking tool?"** — No; download manager. **PREPARED** (owner confirm).
- **"Why should SignPath accept a new/individual project?"** — Owner must articulate reputation
  evidence (public history, users, releases). **REQUIRES OWNER APPROVAL** — do not invent claims.

---

## 9. Legitimate FREE alternatives (brief, honest)

**COMPLETED** — investigated 2026-11-21.

| Option | Free? | Notes | Verdict |
| --- | --- | --- | --- |
| **SignPath Foundation** (`signpath.org`) | **Yes** — explicitly "free of charge" for OSS; HSM-held key; EXE/MSI support | The primary fit for this zero-budget project. Acceptance discretionary; project/repo conditions apply. | **Recommended direction — REQUIRES OWNER APPROVAL to apply** |
| **Certum "Open Source Code Signing"** (`shop.certum.eu/open-source-code-signing.html`) | **No** — registry verified 2026-11-16: "NOT free... from EUR 69.00 gross", physical crypto card + reader, identity verification. Certum becomes a named "Open Source Developer" publisher. | Publicly trusted and SmartScreen-building, but conflicts with the recorded **zero-budget** constraint. | **Not viable under zero budget** — REQUIRES OWNER APPROVAL if ever reconsidered |
| **DigiCert / other commercial CAs** | **No** free OSS program found | Paid commercial certificates only; no no-cost OSS tier located in this search. | **Not viable under zero budget** |
| **OSSign.org** (`ossign.org`) | **Unverified** — page content could not be retrieved in this session | Claims free OSS signing; legitimacy, trust chain, and terms not established. | **Do not rely on / do not recommend without independent due diligence** — REQUIRES OWNER APPROVAL |
| **Microsoft Azure Trusted Signing / self-issued certificates** | Paid / not a free OSS foundation program | — | Out of scope for zero budget |

**Self-signed / unsigned caveat (COMPLETED, important):** a self-signed certificate, or distributing
the current unsigned binaries, is **NOT equivalent** to a trusted Authenticode signature from a
publicly trusted CA. It does not provide publisher identity validation, does not establish the
source-to-binary link, and does not build Windows SmartScreen reputation — users still see
unknown-publisher warnings. This package does **not** recommend self-signing as a substitute for
SignPath Foundation or any other trusted code-signing route.

---

## 10. Final declaration

- **NOT SUBMITTED.** No SignPath application was created, no account was opened, no certificate was
  requested, and no publisher identity was assumed.
- **REQUIRES OWNER APPROVAL.** Every placeholder, every "owner must" item, and the decision to
  apply at all belong to the owner.
- **NO APPROVAL IS CLAIMED.** SignPath acceptance, certificate issuance, and any trusted signature
  remain unattained. The existing **NO-GO** release verdict and `ready_for_public_release: false`
  are unchanged.
- **Evidence date:** official criteria verified **2026-11-21**; third-party offer registry entries
  were dated **2026-11-16** and must be re-verified before use.

## 11. Sources

- Official (fetched 2026-11-21): `https://signpath.org/terms.html`, `https://signpath.org/`,
  `https://signpath.org/apply`.
- Third-party registry (verified 2026-11-16, re-verify before use):
  `https://foropensource.com/company/signpath/`, `https://foropensource.com/company/certum/`,
  `https://foropensource.com/`; Certum shop `https://shop.certum.eu/open-source-code-signing.html`.
- Local evidence: `docs/release/SIGNPATH_ELIGIBILITY.md`, `LICENSE_AUDIT.md`,
  `ZERO_BUDGET_DISTRIBUTION_PLAN.md`, `FINAL_BUILD_EVIDENCE.md`, `CLEAN_MACHINE_ACCEPTANCE.md`,
  `GO_NO_GO_REPORT.md`, `RELEASE_PLAN.md`, `GITHUB_RELEASES_SETUP.md`; `README.md`;
  `.github/workflows/windows.yml`; `tools/build-windows.ps1`; `package.json`;
  `src-tauri/Cargo.toml`; `src-tauri/Cargo.lock`.
- Direct read-only commands this session: `git rev-parse` (not a repo), `Test-Path .git` (False),
  root `LICENSE*` enumeration (none), workflow file read.
