# SwiftFetch — SignPath Foundation application packet (FINALIZED, NOT submitted)

**Status:** NOT SUBMITTED · NO OWNER IDENTITY INVENTED · NO PASSWORDS REQUESTED · NO-GO preserved
**Prepared:** 2026-11-22 · **Phase 3.1:** technical answers finalized; personal fields remain
`[OWNER: …]` placeholders. Submitting the form, attesting to facts, and completing identity
verification are **owner actions**. This packet is ready for owner review, not for submission as-is.

## 1. Rules for this packet

- Every `[OWNER: …]` token must be replaced by the owner with a real value before submission.
- Do not paste account passwords, recovery codes, private keys, API tokens, or certificate material
  into chat, issues, commits, or this document.
- Do not accept or sign any paid agreement. SignPath Foundation's program is free; if a payment is
  requested, stop and consult the owner.
- Do not claim approval, certification, or publisher identity until it is actually granted.

## 2. Pre-application checklist (owner actions)

| # | Action | Why | Done |
|---|---|---|---|
| 1 | Enable MFA on the GitHub account that owns the repository (and on any collaborator accounts). | Published Foundation condition. | ☐ |
| 2 | Create/secure the SignPath account with MFA. | Published Foundation condition. | ☐ |
| 3 | Decide role holders: Authors, Reviewers, Approvers. A solo maintainer holding all three must be disclosed and accepted. | Published Foundation condition. | ☐ |
| 4 | Fill the `[OWNER: …]` role links in the prepared policy page `docs/site/code-signing-policy.html` and publish it, with a "Code signing policy" link on the home and Download pages. | Mandatory Foundation condition; page prepared in Phase 3.1. | ☐ |
| 5 | Publish/refresh the Privacy page text (site already has one) and confirm no unreviewed data transfers. | Published condition. | ☐ |
| 6 | Prepare a real Download page with the signed artifact links and hashes — only after signing and owner approval. | "Released in the form to be signed" and "documented" conditions. | ☐ |
| 7 | Decide how to handle the "verifiable builds / reproducible builds" gap honestly (see `INSTALLER_ENGINEERING.md` §5). | Foundation asks for verifiable builds; reproducibility is not proven. | ☐ |
| 8 | Send the prepared questions in `SIGNPATH_QUESTIONS.md` (Bangladesh eligibility, individual maintainer, solo roles, renewal, MSI/NSIS signing) through the official SignPath channel and record the answers. | Not published; must confirm before submission. | ☐ |
| 9 | Re-read `signpath.org/terms.html` on submission day; conditions can change. | Foundation reserves the right to change conditions. | ☐ |

## 3. Verified facts available for the form (no invention)

| Field | Value | Verification |
|---|---|---|
| Project name | SwiftFetch | `src-tauri/tauri.conf.json` |
| Version | 0.1.0 | `src-tauri/tauri.conf.json`, `package.json` |
| Source repository | https://github.com/Aliahasan399/SwiftFetch | git remote, public repository |
| Project website | https://aliahasan399.github.io/SwiftFetch/ | fetched 2026-11-22 |
| License | GPL-3.0-or-later | root `LICENSE` |
| Copyright holder / year | Ali Ahasan / 2026 | owner decision (recorded in `OWNER_LICENSE_DECISION.md`) |
| Platforms | Windows x64 (MSI and NSIS EXE) | `FINAL_BUILD_EVIDENCE.md` |
| Architecture | Tauri 2, Rust, React, TypeScript | repo source |
| Distribution model | Free download, local-first, no account/telemetry | project website |
| Installers to sign | `SwiftFetch_0.1.0_x64_en-US.msi`, `SwiftFetch_0.1.0_x64-setup.exe` (plus nested PE files in MSI) | `FINAL_BUILD_EVIDENCE.md` |

## 4. Finalized technical answers (owner attests personal facts only)

All technical wording below is final as of 2026-11-22. Only `[OWNER: …]` fields require real
personal/organizational input. Do not convert any "not yet" statement into a completed claim.

1. **Project description:** "SwiftFetch is a free, local-first Windows download manager built with
   Tauri, Rust and React. It queues HTTP/HTTPS downloads with resumable segmented transfers, keeps
   state in SQLite under the user's profile, and offers an optional opt-in browser handoff."
2. **Why signing is needed:** "Unsigned MSI/NSIS installers show Unknown Publisher warnings and are
   blocked by some enterprise policies. Signing gives users a verifiable link between the public
   repository and the published binary."
3. **License:** "GPL-3.0-or-later. The full license text and third-party notices are included in the
   source and distribution."
4. **Proprietary components:** "None identified. All first-party source and build scripts are in the
   public repository. Third-party dependencies are documented in `THIRD_PARTY_NOTICES.md`."
5. **Malware / PUP / hacking tools:** "SwiftFetch is a download manager. It has no vulnerability
   scanning, exploitation, or security-control circumvention features. Website usage terms prohibit
   circumvention claims."
6. **Privacy:** "No telemetry. Network requests go only to the download source, configured proxies,
   and accepted redirects. Clipboard capture is off by default. Privacy statement is on the project
   website." (Confirm before attesting.)
7. **Uninstall:** "Both MSI and NSIS provide uninstall. Data is preserved by default; NSIS offers an
   explicit opt-in 'Delete app data' action targeting only `%APPDATA%\swiftfetch`."
8. **Team and roles (final technical wording):** "The project is currently maintained by a single
   person. That maintainer performs the Author, Reviewer and Approver functions, and all three
   role assignments will be disclosed to SignPath Foundation for confirmation. The CI workflow is
   the Submitters; the maintainer alone approves each signing request. MFA is used on GitHub and
   will be used on SignPath." `[OWNER: supply the real name/account for each role.]`
   See `SOLO_MAINTAINER_ROLES.md` for the separation analysis.
9. **Release process (final):** "Release builds are produced by GitHub Actions on GitHub-hosted
   runners from the public `main` branch, with Rust 1.98.0, `npm ci` and Cargo `--locked`. The
   SignPath GitHub App verifies build origin from GitHub metadata; origin verification and trusted
   build system checks are required in the release-signing policy. No release is signed without
   manual approval by the named Approver."
10. **Build reproducibility disclosure (final — disclose honestly):** "Toolchain, dependency
    lockfiles and isolated frontend directories are enforced, but byte-identical reproducible
    builds are **not yet proven**. This is disclosed as an open gap; no reproducibility claim is
    made in the application."
11. **Publisher identity expectation:** "I understand the certificate is issued to SignPath
    Foundation and that Windows will show SignPath Foundation as the publisher."
12. **Contact / legal identity:** "[OWNER: supply the real contact details the form requests.]"

## 5. Code-signing policy page (prepared separately)

The policy page is prepared as a standalone website document:
`docs/site/code-signing-policy.html` (publish path `/code-signing-policy.html`).
It contains the exact required attribution:

> Free code signing provided by SignPath.io, certificate by SignPath Foundation

It also states team roles (with `[OWNER: …]` links), the manual-approval release process, and the
required privacy-policy statement, and links to the Privacy page and third-party notices.
"Code signing policy" links are added to the home and Download pages to satisfy the Foundation's
placement requirement.

The page must not be treated as authoritative until the owner fills the role links and confirms the
privacy statement; it carries a visible draft notice until then.

## 6. Step-by-step owner instructions

### Before applying
1. Complete checklist §2.
2. Read `CODE_SIGNING_PROGRAM_COMPARISON.md` and accept the SignPath Foundation publisher trade-off.
3. Verify there is no paid step in the flow; stop if payment is requested.

### Application (owner-only)
4. Open https://signpath.org/apply.
5. Enter the repository URL, download page URL, and project description from §3/§4.
6. Attest only to facts the owner has verified. Do **not** copy `[OWNER: …]` placeholders.
7. Save a copy of what was submitted (no passwords) under `docs/release/` for the record.
8. Wait for the Foundation's decision. No approval is assumed.

### Only after written acceptance
9. SignPath organization/project setup: create the project, set repository URL, configure a
   `release-signing` policy with manual approval, restricted branch (`main`), trusted build system
   GitHub.com, and origin verification.
10. Link the SignPath GitHub App to the repository and the SignPath project (required for audit-log
    based origin verification).
11. Define artifact configurations for the MSI (deep signing of nested PE files) and the NSIS EXE —
    draft in `SIGNING_ARCHITECTURE.md`.
12. Add the API token as a GitHub Actions secret named `SIGNPATH_API_TOKEN`; never commit it.
13. Copy the prepared template `docs/release/templates/sign-release.yml.template` into
    `.github/workflows/sign-release.yml` only when the owner explicitly authorizes production
    signing. It is intentionally inactive and outside `.github/workflows/` today.
14. Test with a test-signing policy first; then perform one manually approved release signing.
15. Verify Authenticode signatures and generate distribution hashes per `SIGNING_ARCHITECTURE.md`.
16. Only then may the owner decide on publication. Publication still requires lifting NO-GO and
    setting `ready_for_public_release: true` explicitly.

## 7. Explicitly not done here

No form was filled on any external site, no account was created, no identity was asserted, no
organisation details were invented, no password or key was requested or stored, and no signing or
publication was activated.
