# SwiftFetch — Legal & Privacy Release Checklist (Phase 5)

**Created:** 2026-11-21. **Version:** 0.1.0. **Scope:** legal/privacy release items only.
**Companion checklists:** `A11Y_CHECKLIST.md`, `STABILITY_CHECKLIST.md`; gate context in
`RELEASE_PLAN.md` (P0: license + publisher identity/terms) and `RELEASE_GAP_REGISTER.md`.

Status legend:

- **Ready** — implemented with recorded evidence; no further work known.
- **Partial** — implemented/drafted, but review, missing pieces, or limits remain.
- **Externally blocked** — requires an owner decision or external resource.
- **Not verified** — no evidence found; do not claim.

No item below claims legal approval, regulatory certification, or an assigned license.
All publisher/license values are placeholders; nothing was invented.

## 1. License & third-party notices

| # | Item | STATUS | Evidence | Required action |
| --- | --- | --- | --- | --- |
| 1.1 | Project distribution license selected | Externally blocked | `RELEASE-READINESS.md:37-40`; `docs/TERMS-DRAFT.md:12-14`; `RELEASE_PLAN.md:112` (P0 gate) | [OWNER: license choice] + LICENSE file at repo root; none assigned on owner's behalf |
| 1.2 | Distribution license text ships in package | Not verified | No LICENSE file observed at repo root | Add chosen license text after 1.1; verify in release folder |
| 1.3 | Third-party inventory generated (338 deps, SPDX) | Partial | `VERIFICATION.md:129-130`; `tools/dependency-notices.mjs`; `RELEASE_GAP_REGISTER.md:65` | Re-run against final locked deps at release time; confirm counts |
| 1.4 | Seven package roots lack license text | Partial | `VERIFICATION.md:129-130` ("Seven package roots lacked license text and require review"); `RELEASE_GAP_REGISTER.md:65` (confirmed doc record); inventory column "License text files = 0" (e.g. `alloc-stdlib` 0.3.0, `clipboard-win` 5.4.3 in the candidate snapshot) | Obtain text from each package's upstream source, record SPDX expression, regenerate notices |
| 1.5 | Third-party notice attribution approved | Externally blocked | Candidate notices: `artifacts/SwiftFetch-0.1.0-candidate-*/notices/THIRD-PARTY-NOTICES.md`; `docs/TERMS-DRAFT.md:14` | [OWNER: approve attribution text]; confirm notices bundle with the installer, not only candidate snapshots |
| 1.6 | Warranty/disclaimer position stated | Externally blocked | `docs/TERMS-DRAFT.md:10-14` | Part of 1.1/1.5 approval; do not draft legal wording without owner |

## 2. Publisher identity, contacts, and policy documents

| # | Item | STATUS | Evidence | Required action |
| --- | --- | --- | --- | --- |
| 2.1 | Publisher identity (legal name, distribution entity) | Externally blocked | `docs/TERMS-DRAFT.md:13-14`; `RELEASE-READINESS.md:84-86` | [OWNER: publisher legal name] in Terms/Privacy/Support + About view |
| 2.2 | Privacy contact | Externally blocked | `docs/PRIVACY.md:32-34` | [OWNER: privacy contact] |
| 2.3 | Private vulnerability-reporting contact | Externally blocked | `docs/SECURITY.md:6-7`; `docs/SUPPORT.md:33-34` | [OWNER: security contact] — required before public distribution |
| 2.4 | Public issue-reporting location | Externally blocked | `docs/SUPPORT.md:33-34` | [OWNER: issue tracker URL] |
| 2.5 | Supported versions + response expectations | Externally blocked | `docs/SUPPORT.md:33-34` | [OWNER: support policy] |
| 2.6 | Terms of use (permitted use) approved | Partial | `docs/TERMS-DRAFT.md` (draft: permitted use, no circumvention representation, speed/checksum/AV disclaimers) | [OWNER: approve terms] |
| 2.7 | Privacy notice approved and matched to release config | Partial | `docs/PRIVACY.md` (draft; explicitly claims no regulatory certification) | [OWNER: approve privacy notice]; re-review against final update/proxy/scanner configuration |
| 2.8 | Support/recovery documentation approved | Partial | `docs/SUPPORT.md`; `docs/RECOVERY.md`; `docs/UPDATES.md` | [OWNER: approve support content] |
| 2.9 | Regulatory certification claimed | Not applicable — never claimed | `docs/PRIVACY.md:34-35`; `RELEASE-READINESS.md:86` | Keep out of all published copy |

## 3. Privacy-behavior claims (evidence backing the notice)

| # | Item | STATUS | Evidence | Required action |
| --- | --- | --- | --- | --- |
| 3.1 | Local-only state; no telemetry upload | Partial | `docs/PRIVACY.md:5-12`; store at `%APPDATA%/swiftfetch/state.sqlite` (`CONTRACT.md:289-292`); "No account. No cloud upload." (`src/i18n.tsx:126-127`) | Owner review of final build; no automated proof of "no telemetry" exists |
| 3.2 | Local-only diagnostics; exports do not self-upload | Ready | `docs/PRIVACY.md:11-12`; `get_diagnostics` / `export_queue` (`CONTRACT.md:71,82`); URL-stripped inventory export in smoke run `artifacts/smoke/run-zIBKJy` (`VERIFICATION.md:261-263`) | Keep redaction covered by regression tests |
| 3.3 | Clipboard capture opt-in; detection never starts a transfer | Partial | `docs/PRIVACY.md:14-17`; unit URL-filter coverage; OS clipboard mutation test skipped by default (`VERIFICATION.md:78-81`; `RELEASE_GAP_REGISTER.md:55`) | Optional isolated-session clipboard run before release |
| 3.4 | Extension sends only the user-selected URL; no cookies/credentials forwarded | Partial | `docs/PRIVACY.md:19-21`; Edge acceptance `run-a7Lzm2`/`run-EDt1Fy`, Firefox `run-ae163dec…`/`run-ec9603821…` (`VERIFICATION.md:83-88,179-188`); credential-rejection subtest | Chrome handoff unconfirmed (`RELEASE_GAP_REGISTER.md:42,54`); permanent-install/store validation is [OWNER: browser distribution decision] |
| 3.5 | Retention/partial cleanup limited to task-owned files | Partial | `docs/PRIVACY.md:23-26`; `RELEASE-READINESS.md:61-64` | Keep collision/partial ownership tests green |
| 3.6 | Redacted inventory excludes URLs/headers; full backup marked private | Ready | `docs/PRIVACY.md:28-30`; `docs/SUPPORT.md:22-23`; smoke URL-stripped export (`VERIFICATION.md:267-268`) | None known |

## 4. Browser permission rationale

| # | Item | STATUS | Evidence | Required action |
| --- | --- | --- | --- | --- |
| 4.1 | Permission set documented and minimal | Partial | `browser-extension/chrome/manifest.json:7-8` (`contextMenus`, `storage`, `nativeMessaging`, `tabs`; `host_permissions: []`); Firefox manifest at `browser-extension/firefox/manifest.json` | Write the published rationale text per permission; [OWNER: approve before store submission] |
| 4.2 | Content script scope justified (`<all_urls>`, `document_idle`) | Partial | `browser-extension/chrome/manifest.json:13-21`; manifest description "Never inspects or transmits browsing traffic"; hidden page scanning forbidden (`RELEASE_GAP_REGISTER.md:80`) | Confirm no store policy conflict during review |
| 4.3 | Handoff is opt-in via registered native host | Ready | `RELEASE-READINESS.md:61`; `browser-extension/SETUP.md`; unpacked load + registration is user-driven | Registration procedure re-verified per release |
| 4.4 | Store signing/review + permanent install | Externally blocked | `RELEASE-READINESS.md:50-53`; `RELEASE_PLAN.md:96` (P1 gate) | [OWNER: browser-store submission decision] |

## 5. Secret handling & redaction

| # | Item | STATUS | Evidence | Required action |
| --- | --- | --- | --- | --- |
| 5.1 | Saved request headers encrypted at rest (opt-in, DPAPI) | Ready | `VERIFICATION.md:202-210` (`encrypt_saved_headers` default off; `sfenc1:` envelopes; 5 integration tests + real-DPAPI harness; no readable `Cookie:`/`Authorization:` strings after encryption) | Keep default-off behavior for legacy queues |
| 5.2 | Migration command fails closed on tampering | Ready | `VERIFICATION.md:207-208` (`encrypt_saved_secrets`, transactional, fails closed) | None known |
| 5.3 | Exports/diagnostics redacted by default | Ready | `CONTRACT.md:82` (redacted metadata inventory); `VERIFICATION.md:267-268` | None known |
| 5.4 | Known-secret-pattern scan of source/tools | Partial | `VERIFICATION.md:131-132` (limited pattern scan, zero matches) | Not a comprehensive secrets/dependency/security audit — state this honestly in any public claim |
| 5.5 | Report-handling guidance forbids publishing secrets | Ready | `docs/SECURITY.md:5-7`; `docs/SUPPORT.md:5-7` | Publish with 2.3/2.4 contacts filled in |

## 6. No auto-execution & safety disclaimers

| # | Item | STATUS | Evidence | Required action |
| --- | --- | --- | --- | --- |
| 6.1 | Downloaded files never auto-execute | Ready | `CONTRACT.md:280` ("Never auto-execute a downloaded file"); `docs/SECURITY.md:9-12` | Regression coverage retained |
| 6.2 | Checksum/AV meaning disclaimed | Ready | `docs/SECURITY.md:10-12`; `docs/TERMS-DRAFT.md:9-10`; AV is fixture-tested only (`RELEASE_GAP_REGISTER.md:56`) | Keep wording in published Terms |
| 6.3 | Optional scanner failure treated as incomplete scan | Ready | `docs/SECURITY.md:11-12`; `VERIFICATION.md:29-30` (hung-process termination) | None known |

## 7. Safe update failure (privacy/safety-relevant paths)

| # | Item | STATUS | Evidence | Required action |
| --- | --- | --- | --- | --- |
| 7.1 | Unconfigured build performs no update network access | Ready | `CONTRACT.md:221-224`; `src/i18n.tsx:90` ("no update check runs"); `RELEASE-READINESS.md:55` (`not_configured` without network access) | None known |
| 7.2 | Update check contacts release endpoint only on request | Partial | `docs/PRIVACY.md:9-10` | [OWNER: HTTPS update endpoint + public/signing keys] — a dev build must not invent a channel (`RELEASE-READINESS.md:54-57`) |
| 7.3 | Interrupted update reconciled on startup; rollback only after validation | Ready | `VERIFICATION.md:211-218` (phase persistence, `reconcile_on_startup`, validated snapshot restore, 1 GiB recursive budget; 3 hardening tests + signature/version-replay tests) | None known |
| 7.4 | Real publisher-signed install/update/rollback acceptance | Externally blocked | `VERIFICATION.md:90-94`; `RELEASE-READINESS.md:55-57`; `RELEASE_PLAN.md:94-95` | [OWNER: signing certificate + release environment] — see `STABILITY_CHECKLIST.md` §4-5 |

## 8. Sign-off (owner decisions required — none taken)

- [ ] [OWNER: license choice] recorded and LICENSE file added (1.1-1.2)
- [ ] Seven license-text gaps resolved and notices regenerated (1.3-1.5)
- [ ] [OWNER: publisher identity / privacy contact / security contact / issue URL / support policy] (2.1-2.5)
- [ ] [OWNER: Terms, Privacy, Support approval] against the release configuration (2.6-2.8)
- [ ] [OWNER: browser permission rationale approval + distribution decision] (4.1-4.4)
- [ ] [OWNER: update endpoint/keys + signed-channel acceptance] (7.2, 7.4)

Blocking summary: public release is **NO-GO** while items 1.1, 2.1-2.5, 4.4, and 7.4 are open
(`RELEASE_PLAN.md:85-100`; `candidate-manifest.json` records `ready_for_public_release: false`,
`VERIFICATION.md:271-272`).