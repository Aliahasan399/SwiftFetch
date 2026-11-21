# SwiftFetch — SignPath Foundation application execution report

**Status: NOT SUBMITTED. REQUIRES OWNER APPROVAL. NO APPROVAL IS CLAIMED.**

**Prepared:** 2026-11-21 · **Version:** 0.1.0 · **Task lane:** signpath-execution-reporter
**Workspace inspected:** `E:\CloneIdm\swiftfetch` (read-only).
**Write scope:** this file only — no source/config changes, no commits, no certificate request, no
application submission, no invented facts.

This report records what was actually verified and prepared for a possible SignPath Foundation
application. It **does not apply**, does **not** represent the owner, and does **not** assert that
SwiftFetch qualifies. It updates the earlier `SIGNPATH_ELIGIBILITY.md` /
`SIGNPATH_APPLICATION_READINESS.md` findings with one material change verified this session: **the
project now has a real, publicly accessible repository** (previously a hard blocker). Every real
publisher, contact, license-decision, and security fact remains the owner's to supply or confirm.

---

## Status legend (applies to every finding below)

| Tag | Meaning |
| --- | --- |
| **COMPLETED** | Verified in this session against official source, a real remote, or local evidence; no owner action needed for the *documentation* fact. |
| **PREPARED** | A draft/template/input exists but is not submitted, published, or a claim of fact. |
| **BLOCKED** | A hard, observable gap prevents a credible application today. |
| **REQUIRES OWNER APPROVAL** | Cannot be asserted or performed without the owner's real decision/fact/action (especially formal declarations and legal commitments). |

---

## 1. Official eligibility requirements verified

**COMPLETED** — criteria were verified directly against the official site on **2026-11-21** (as
recorded in `SIGNPATH_APPLICATION_READINESS.md`, which fetched `https://signpath.org/terms.html`,
`https://signpath.org/`, and `https://signpath.org/apply` that day). The application form page
publishes no field list, so the answers in Section 6 are *anticipated*, not confirmed fields.

| Reference | URL | Verification |
| --- | --- | --- |
| SignPath Foundation terms / Code of Conduct ("conditions for Open Source projects") | `https://signpath.org/terms.html` | **COMPLETED** — 2026-11-21. Note: the page marks its *Code of conduct* section **"Draft"**. |
| SignPath Foundation overview | `https://signpath.org/` | **COMPLETED** — 2026-11-21. |
| Application entry point | `https://signpath.org/apply` | **COMPLETED** — 2026-11-21 (form; no public field list). |

**Verified conditions for a free SignPath.io OSS subscription:**

- No malware / no potentially unwanted programs.
- OSI-approved OSS license without commercial dual licensing, for all components.
- No proprietary code (system libraries are permitted).
- Actively maintained.
- Already released in the form to be signed.
- Documented functionality on the download page or store entry.

**Additional conditions when the certificate is issued by SignPath Foundation (the normal free path):**

- Sign only your own projects and own binaries; the team maintains all source and build scripts.
- No "hacking tools" as defined by their policy.
- Respect privacy/security, announce system changes, provide uninstallation.
- Multi-factor authentication for **all** team members on **both** SignPath and repository access.
- Defined code-signing roles: Authors, Reviewers, Approvers, with manual approval per signing request.
- A code-signing policy on the project home page, including the required SignPath attribution,
  team roles, and a privacy-policy statement.
- Artifact configuration: product-name and product-version metadata set and enforced.
- Verifiable builds: binaries built from source in a verifiable way; every release manually approved;
  cooperate with violation investigations.

**Caveat — REQUIRES OWNER APPROVAL / COMPLETED:** SignPath Foundation states it is under **no
obligation to accept any project**, decides at its own discretion with no independent arbitration,
and may change these conditions. Re-check the official URLs immediately before any submission.
The "3+ releases / 3+ contributors" heuristic found in some community summaries does **not** appear
on the current official terms page; it is treated here as **unverified/outdated** and is not asserted
as a rule.

---

## 2. Requirement-by-requirement comparison (official criteria vs. SwiftFetch actual state)

| # | Official requirement | SwiftFetch ACTUAL state (verified 2026-11-21) | Status |
| --- | --- | --- | --- |
| 1 | No malware / no PUP | Download manager with browser handoff; no exploitation/scanning features observed. No formal malware/PUP scan asserted. | **PREPARED** (favorable observation; owner confirm) |
| 2 | OSI-approved OSS license, no commercial dual licensing | GPL-3.0-or-later **proposed and CONDITIONALLY APPROVED** (`LICENSE_AUDIT.md`), but **no root `LICENSE` file** in the working tree or on the public repo; README explicitly defers adding it until compatibility is confirmed. OSI approval attaches only once adopted. | **BLOCKED** / **REQUIRES OWNER APPROVAL** |
| 3 | No proprietary code (system libraries allowed) | Rust/TypeScript + dependencies; no proprietary component identified. SBOM: 338 deps; 7 missing license *texts*; CDLA-Permissive-2.0 ×2 unresolved. No formal attestation. | **PREPARED** (partial/unverified) |
| 4 | Actively maintained | Public repo exists with **10 commits on `main`** — some history is now visible. No release cadence or contributor record beyond that. | **PREPARED** (partial; owner frame as maintained) |
| 5 | Already released **in the form to be signed** | 0.1.0 NSIS/MSI bundles exist but are **unsigned** (`NotSigned`); `candidate-manifest.json` = `ready_for_public_release: false`; no published GitHub release observed. | **BLOCKED** |
| 6 | Documented on download page / store entry | Public README documents functionality; `docs/site/` pages prepared with placeholders; no confirmed live download page. | **PREPARED** / **REQUIRES OWNER APPROVAL** |
| 7 | Sign only your own projects/binaries | App is first-party; dependencies are upstream OSS libraries. Fork/upstream-signing rules appear not to apply. | **PREPARED** (favorable; owner confirm) |
| 8 | No "hacking tools" | No vulnerability-scanning/exploitation features observed. | **PREPARED** (favorable; owner confirm) |
| 9 | Respect privacy/security; announce changes; provide uninstallation | `docs/PRIVACY.md`, `docs/SUPPORT.md` drafts describe uninstall/data preservation; not published/approved. No required telemetry. | **PREPARED** / **REQUIRES OWNER APPROVAL** |
| 10 | MFA for all team members on **both** SignPath and repo | Not observable. Individual publisher. **2FA status unknown.** | **REQUIRES OWNER APPROVAL** |
| 11 | Defined roles: Authors, Reviewers, Approvers; manual approval per request | No permission model; solo maintainer. Not established. | **BLOCKED** / **REQUIRES OWNER APPROVAL** |
| 12 | Code-signing policy on project home page (attribution, roles, privacy) | No such section published; drafts only. | **BLOCKED** / **REQUIRES OWNER APPROVAL** |
| 13 | Artifact config: product name/version set and **enforced** | Tauri config sets `productName: SwiftFetch`, `version: 0.1.0`; **no SignPath artifact configuration exists**; enforcement not evidenced. | **PREPARED** / **BLOCKED** |
| 14 | Binaries built from source in a **verifiable** way; manual approval; cooperate with investigations | Pinned build inputs + CI workflow exist, but no public build-to-binary mapping, no clean-room reproduction, no hash comparison, no tag-tied attestation, no SignPath integration. | **PREPARED** (inputs) / **BLOCKED** (evidence) |
| 15 | Verifiable reputation ("cannot sign binaries based on source code that nobody knows") | Public repo now visible (10 commits), but **0 stars, 0 watchers, 0 forks, no published releases** observed — thin reputation signal. | **PREPARED** / **REQUIRES OWNER APPROVAL** |

---

## 3. Publisher eligibility — **REQUIRES OWNER APPROVAL** (not disqualifying by itself)

The official program does not issue a certificate to a person or project. It states: *"Since we are
not a Certificate Authority (CA), we cannot issue certificates to you... What we actually do is get
certificates issued to 'SignPath Foundation' and let OSS projects use them."* The overview adds
*"No need for personal identification, we verify that the binary was built from your open source
repository and vouch for that with our name."*

- The recorded owner decision **individual / personal publisher** does **not** by itself bar an
  application — SignPath Foundation becomes the publisher of record.
- The *project* must still satisfy every condition, and a solo maintainer must show MFA, role
  separation (Authors/Reviewers/Approvers), manual approval, and a verifiable public origin.
- The owner must confirm they control the public repository and may act as the authorized
  applicant. **No identity is assumed here.**

---

## 4. Key gating findings (detail)

### 4.1 Public repository requirement — **NOW MET / COMPLETED**

This is the material change from the earlier assessments, which recorded "not a git repository /
no public URL" as the single largest blocker. Verified this session:

- `git remote -v` → `origin https://github.com/Aliahasan399/SwiftFetch.git` (fetch and push);
  branch `main`; working tree clean.
- The repository is **publicly accessible**: `https://github.com/Aliahasan399/SwiftFetch` renders
  as "SwiftFetch **Public**" (personal account `Aliahasan399`), `main` with 10 commits, and a file
  set including `.github/workflows`, `README.md`, `CONTRIBUTING.md`, `CODE_OF_CONDUCT.md`,
  `SECURITY.md`, `THIRD_PARTY_NOTICES.md`, `VERIFICATION.md`, `package-lock.json`, and `src-tauri/`.
- Therefore the **public, inspectable source-repository condition is satisfied**. `SIGNPATH_ELIGIBILITY.md`
  line 56 and `SIGNPATH_APPLICATION_READINESS.md` §3.3/§4 are now **outdated on this point only**;
  all other blockers below still stand.

**Real public repo URL (verified): `https://github.com/Aliahasan399/SwiftFetch`**

### 4.2 Public license requirement — **NOT MET / BLOCKED + REQUIRES OWNER APPROVAL**

- **No root `LICENSE` file** exists in the working tree, and the public repository root listing
  likewise shows no `LICENSE`. The README states the final `LICENSE` will be added only after the
  compatibility audit is confirmed.
- GPL-3.0-or-later is **proposed and CONDITIONALLY APPROVED** for technical compatibility only;
  `FINAL_LICENSE_DECISION.md` records **final approval: NO — do not finalize the license yet**, with
  four open conditions (7 missing license texts; MPL-2.0 notice/source-offer execution; unresolved
  CDLA-Permissive-2.0 ×2; GPLv3 text not shipped) plus contributor-licensing (CLA/DCO) undecided.
- Copyright holder is unconfirmed and was deliberately not invented.
- Selecting the license, adding the root `LICENSE` with a copyright line, bundling third-party
  notices, and resolving CDLA are **formal/legal commitments that require owner review**.

### 4.3 Reproducible / verifiable build requirement — **NOT EVIDENCED / PREPARED (inputs) + BLOCKED (proof)**

Positive, verifiable inputs:

- `.github/workflows/windows.yml` on `windows-2022`; triggers on PR, push to `main`, manual dispatch;
  `permissions: contents: read`; actions pinned to full commit SHAs; `persist-credentials: false`;
  `npm ci`, Rust `1.98.0`, Python `3.12`, `./tools/verify.ps1`, `./tools/build-windows.ps1`,
  `node tools/smoke-desktop.mjs`.
- `tools/build-windows.ps1`: `npm run tauri build -- --ci ... -- --locked`, fresh
  `artifacts/candidate-<guid>/ui` frontend directory; optional `-SigningConfig` hook.
- `src-tauri/Cargo.lock` and `package-lock.json` present; Cargo release profile uses `lto=true`,
  `codegen-units=1`, `panic="abort"`, `strip=true`.

Missing evidence: no documented build-to-binary mapping, no independent clean-room rebuild, no
cross-build SHA-256 comparison, no tag-tied SBOM/attestation, no published `SHA256SUMS`, no SignPath
artifact configuration, no manual-approval workflow. Publication of a `v0.1.0` release tag is pending
the NO-GO gates. **Inputs are a solid foundation; reproducibility of the shipped binaries is unproven.**

### 4.4 Account security (2FA/MFA) — **UNKNOWN / REQUIRES OWNER APPROVAL**

Official terms require MFA for all team members on both SignPath and repository access. No evidence
is observable in the repository or workspace. The owner must enable and affirm MFA for the GitHub
account (`Aliahasan399`) and any SignPath account; none is asserted here.

---

## 5. Prepared application materials (NOT submitted)

The following are drafts grounded in observed files and the now-verified public repository.
Bracketed `[OWNER: ...]` values remain owner-supplied; nothing is invented.

### 5.1 Project description — **PREPARED**

> SwiftFetch is a free Windows desktop download manager built with Tauri 2, React 19, TypeScript,
> and Rust. Users add an HTTP/HTTPS link to a local queue and can pause, retry, schedule, set
> transfer limits, and verify downloads with a trusted checksum. The transfer engine uses multiple
> connections where a server supports range requests and falls back to a single stream otherwise.
> Queue state, history, and checkpoints are kept in a local SQLite database under the user's
> `%APPDATA%\swiftfetch` directory. An optional browser handoff uses Windows Native Messaging and
> requires explicit desktop opt-in. There is no required account, hosted backend, telemetry, or paid
> service. Version 0.1.0 produces an NSIS installer and an MSI.
>
> **Public source repository:** `https://github.com/Aliahasan399/SwiftFetch` (public, personal
> account, `main` branch).
> **Publisher/contact:** [OWNER: name or alias], [OWNER: email].
> **License:** GPL-3.0-or-later intended (proposed, conditionally approved pending `LICENSE_AUDIT.md`).
> **Current status:** unsigned private release candidate; NO-GO for public release.

### 5.2 Technical description — **PREPARED**

> Architecture: a Tauri 2 desktop shell with a React 19 + TypeScript frontend (`src/`) and a Rust
> backend (`src-tauri/src/`) implementing the segmented transfer engine, scheduler, durable SQLite
> queue, diagnostics, and Windows Native Messaging host. IPC is typed. An optional browser extension
> (`browser-extension/`) hands off URLs via Windows Native Messaging only after explicit user
> action and does not forward cookies or credentials. Data is local-first: SQLite under
> `%APPDATA%\swiftfetch`, overridable via `SWIFTFETCH_DATA_DIR`, `SWIFTFETCH_DB_PATH`, and
> `SWIFTFETCH_DOWNLOADS_DIR`. No account, hosted backend, or telemetry upload. Optional
> publisher-configured signed updates/rollback are described in `docs/UPDATES.md` (not yet
> configured). Release toolchain: Rust 1.98.0 (`x86_64-pc-windows-msvc`), Node.js 24+, Python 3.12,
> Windows 11 x64; outputs NSIS (`SwiftFetch_0.1.0_x64-setup.exe`) and MSI
> (`SwiftFetch_0.1.0_x64_en-US.msi`) installers. Signing is not configured; a `-SigningConfig` hook
> exists in the build script for a future certificate path.

### 5.3 Build process explanation — **PREPARED**

> Prerequisites: Windows 11 x64, Microsoft C++ Build Tools (Desktop development with C++), WebView2
> Runtime, Rust 1.98.0 (`x86_64-pc-windows-msvc`), Node.js 24+, Python 3.12.
>
> From a clean checkout: `npm ci` → `./tools/verify.ps1` → `./tools/build-windows.ps1` →
> `node tools/smoke-desktop.mjs`. `tools/build-windows.ps1` invokes
> `npm run tauri build -- --ci --config <generated overlay> -- --locked`, building the frontend into
> a fresh `artifacts/candidate-<guid>/ui` directory so stale assets cannot enter the bundle.
> Dependencies are locked by `package-lock.json` and `src-tauri/Cargo.lock`; the release Rust profile
> uses LTO, a single codegen unit, `panic=abort`, and symbol stripping. GitHub Actions
> (`.github/workflows/windows.yml`) runs the same sequence on `windows-2022` with pinned action SHAs
> and `persist-credentials: false`. Outputs are the two installers plus the unsigned `swiftfetch.exe`.
> Signing is **not** currently configured.

### 5.4 Public repository URL — **COMPLETED (real, verified)**

`https://github.com/Aliahasan399/SwiftFetch` — confirmed public via read-only `git remote -v`
(origin fetch/push URL) and a read-only fetch of the repository page on 2026-11-21.

### 5.5 License status — **BLOCKED / REQUIRES OWNER APPROVAL (conditional)**

Intended GPL-3.0-or-later; technically **CONDITIONALLY APPROVED** by `LICENSE_AUDIT.md`, but final
approval is **NO** per `FINAL_LICENSE_DECISION.md`, no root `LICENSE` file exists, and four audit
conditions plus contributor-licensing remain open. Not an adopted OSI license yet.

### 5.6 Security practices — **PREPARED / REQUIRES OWNER APPROVAL**

Local-first data (SQLite under `%APPDATA%\swiftfetch`, no account/telemetry); browser handoff only
after explicit opt-in with no cookie/credential forwarding; no security/hacking-tool features
observed; `SECURITY.md` present in the public repo; NSIS/MSI uninstallers provided; drafts describe
queue/data preservation. Open items for the owner: publish privacy/support policy, enable private
vulnerability reporting and 2FA, and formally confirm no-telemetry/no-data-transmission claims.

### 5.7 Reproducible-build status — **PREPARED (inputs) / BLOCKED (evidence)**

Pinned toolchains, locked dependencies, a public CI workflow, and a documented single build command
exist; verifiable build-to-binary evidence (clean-room rebuild, hash comparison, tag-tied SBOM,
`SHA256SUMS`, SignPath artifact config, manual-approval gate) does not. See §4.3.

---

## 6. Honest answers to likely application questions — **PREPARED**

Do not submit as-is; owner-dependent answers require **REQUIRES OWNER APPROVAL**.

- **"Is your project open source?"** — Intended yes: public source (`https://github.com/Aliahasan399/SwiftFetch`),
  free download, intended OSI license GPL-3.0-or-later. **Caveat:** license not yet adopted.
  **BLOCKED / REQUIRES OWNER APPROVAL.**
- **"What license and where is the license file?"** — GPL-3.0-or-later proposed/conditionally
  approved; **no root `LICENSE` file** in the working tree or public repo. **BLOCKED / REQUIRES OWNER APPROVAL.**
- **"Where is the public repository?"** — `https://github.com/Aliahasan399/SwiftFetch` (public,
  `main`, 10 commits). **COMPLETED.**
- **"Has the project been released?"** — 0.1.0 artifacts built but unpublished and unsigned;
  `ready_for_public_release: false`. **BLOCKED.**
- **"Who maintains the project / who is on the team?"** — Individual maintainer
  [OWNER: name/alias]; no multi-person team recorded. **REQUIRES OWNER APPROVAL.**
- **"Do you use MFA?"** — Unknown; owner must affirm for repo + SignPath. **REQUIRES OWNER APPROVAL.**
- **"Who are the Authors, Reviewers, Approvers?"** — Not defined; owner must assign.
  **REQUIRES OWNER APPROVAL.**
- **"Can a third party rebuild your binary from source?"** — Inputs pinned and public CI exists, but
  no clean-room reproduction or hash evidence. **PREPARED / BLOCKED.**
- **"Does the software collect or transmit user data?"** — No telemetry/account; local SQLite;
  browser handoff only after explicit action; privacy draft not published.
  **PREPARED / REQUIRES OWNER APPROVAL.**
- **"Does it change system configuration or include an uninstaller?"** — Optional native-messaging
  registration is opt-in; NSIS/MSI provide uninstall; drafts describe data preservation.
  **PREPARED / REQUIRES OWNER APPROVAL.**
- **"Is it a security/hacking tool?"** — No; download manager. **PREPARED** (owner confirm).
- **"Why should SignPath accept a new/individual project?"** — Owner must articulate reputation
  evidence (public history, users, releases). **REQUIRES OWNER APPROVAL** — do not invent claims.

---

## 7. Legitimate free alternatives — **COMPLETED** (investigated 2026-11-21)

| Option | Free? | Verdict |
| --- | --- | --- |
| **SignPath Foundation** (`signpath.org`) | **Yes** — explicitly free of charge for OSS; HSM-held key; EXE/MSI support | **Best fit for the zero-budget constraint.** Acceptance discretionary; project/repo conditions apply. **REQUIRES OWNER APPROVAL to apply.** |
| **Certum "Open Source Code Signing"** (`shop.certum.eu/open-source-code-signing.html`) | **No** — registry verified 2026-11-16: "NOT free... from EUR 69.00 gross", physical crypto card + reader, identity verification | Publicly trusted and builds SmartScreen reputation, but **not viable under zero budget**. **REQUIRES OWNER APPROVAL** if ever reconsidered. |
| **Other commercial CAs** | No free OSS tier found | **Not viable under zero budget.** |
| **OSSign.org** | Unverified (page not retrievable); legitimacy/trust chain not established | **Do not rely on without independent due diligence.** |
| **Self-signed / unsigned** | — | **NOT trusted.** A self-signed certificate or the current unsigned binaries are **not equivalent** to a trusted Authenticode signature: no publisher validation, no source-to-binary link, no SmartScreen reputation; users still see unknown-publisher warnings. **Not recommended as a substitute.** |

---

## 8. Execution decision and final declaration

- **NOT SUBMITTED.** No SignPath application was created, no account opened, no certificate
  requested, and no publisher identity assumed. This report executes *documentation and
  preparation only*.
- **REQUIRES OWNER APPROVAL.** The decision to apply, and every formal declaration or legal
  commitment in an application — license selection/adoption, publisher identity/contact,
  MFA attestation, repository control, Authors/Reviewers/Approvers assignment, published
  privacy/code-signing policy, and any reputation claims — **requires owner review and sign-off**.
  An agent cannot make those commitments on the owner's behalf.
- **NO APPROVAL IS CLAIMED.** SignPath acceptance, certificate issuance, and any trusted signature
  remain unattained. The existing **NO-GO** release verdict and `ready_for_public_release: false`
  are unchanged.
- **Progress this session:** the **public-repository requirement is verified NOW MET** (§4.1) — the
  former largest blocker is resolved by a real public repo. The **public-license requirement remains
  NOT MET** (§4.2), reproducible builds remain **not evidenced** (§4.3), and 2FA remains **unknown**
  (§4.4). These remain owner-controlled gates.
- **Before any submission:** re-read the official terms at `https://signpath.org/terms.html` and
  re-verify the repository/license/MFA/role facts on that date.

---

## 9. Evidence and sources

- Official (verified 2026-11-21): `https://signpath.org/terms.html`, `https://signpath.org/`,
  `https://signpath.org/apply`.
- Real repository (read-only, 2026-11-21): `https://github.com/Aliahasan399/SwiftFetch` (public,
  `main`, 10 commits; root file listing has no `LICENSE`).
- Local read-only commands this session: `git remote -v` (origin fetch/push =
  `https://github.com/Aliahasan399/SwiftFetch.git`), `git branch --show-current` (`main`),
  `git status --short` (clean).
- Local evidence: `docs/release/SIGNPATH_ELIGIBILITY.md`, `SIGNPATH_APPLICATION_READINESS.md`,
  `FINAL_LICENSE_DECISION.md`, `ZERO_BUDGET_DISTRIBUTION_PLAN.md`, `LICENSE_AUDIT.md`,
  `UPSTREAM_LICENSE_TEXTS.md`, `GO_NO_GO_REPORT.md`, `CLEAN_MACHINE_ACCEPTANCE.md`,
  `GITHUB_RELEASES_SETUP.md`; `README.md`; `.github/workflows/windows.yml`;
  `tools/build-windows.ps1`; `package.json`; `src-tauri/Cargo.toml`; `src-tauri/Cargo.lock`.
- Third-party registry entries dated 2026-11-16 (Certum); re-verify before use.
