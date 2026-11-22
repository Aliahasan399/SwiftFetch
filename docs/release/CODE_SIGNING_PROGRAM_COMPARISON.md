# SwiftFetch — free Windows code-signing program comparison (Phase 3)

**Date researched:** 2026-11-22 · **Status:** READ-ONLY RESEARCH — no account created, no application submitted, no certificate requested, no signing performed
**Verdict remains NO-GO** and `ready_for_public_release: false`.

## 1. Scope and method

Official program pages, official product documentation, and a current third-party offer registry were
fetched on 2026-11-22. Claims marked "not published" mean the item was not stated on the official
pages checked; they are gaps to confirm with the provider, not adverse findings. No personal or
organizational details were invented and no paid agreement was accepted.

## 2. Comparison at a glance

| Option | Zero cost? | Certificate type / publisher | Identity verification | Geography | Key custody | MSI + NSIS EXE | Renewal terms | Main limitation |
|---|---|---|---|---|---|---|---|---|
| **SignPath Foundation + SignPath.io** | **Yes** | OV-level, issued to **SignPath Foundation** (Foundation is the publisher) | No personal ID to SignPath; project reputation, repo control, MFA, named roles required | No country restriction stated on official pages checked — must confirm | HSM at SignPath; no local private key | **Yes** — `<pe-file>` for NSIS setup.exe and deep `<msi-file>` signing | Not published on official pages checked — must confirm | Users see "SignPath Foundation" as publisher; manual approval per release; verifiable builds mandatory |
| Azure Artifact Signing (formerly Trusted Signing) | No (~$9.99/mo) | Microsoft-managed certificate | Microsoft validates org/individual | **Individuals: USA/Canada only**; orgs: US/CA/EU/UK | Managed service | Yes via SignTool | Paid subscription | Excluded by zero-budget rule; no free/trial subscription supported |
| Certum Open Source Code Signing | No (from EUR 69 gross) | OV with hardware token | CA identity verification with documents | Not fully verified (paid path) | Physical token/card | Yes | Paid | Not free; excluded |
| Traditional OV/EV CA (DigiCert, Sectigo, GlobalSign) | No ($150–300/yr OV; EV $400+/yr) | OV/EV in owner's name | CA legal-identity validation | Worldwide | HSM/token required since 2023 | Yes | Paid annual | Excluded by zero-budget rule |
| Microsoft Store (MSIX) | Store re-signs free | Store certificate | Developer account | Worldwide | Microsoft-managed | MSIX only; MSI/EXE must be signed by publisher | N/A | Requires MSIX packaging and Store distribution; not applicable to current MSI/NSIS candidate |
| Self-signed | Yes | Untrusted | None | N/A | Local | Yes | N/A | Strong SmartScreen block; public distribution unsuitable |
| Unsigned (current) | Yes | None | None | N/A | N/A | N/A | N/A | Current NO-GO state |

## 3. Recommended option

**SignPath Foundation free code signing via SignPath.io** is the only zero-cost, publicly trusted
option that fits an individual OSS maintainer and the existing MSI + NSIS architecture.

Rationale:
- Free for qualifying OSS projects; certificate issued to SignPath Foundation with the private key
  generated and stored on SignPath's HSM — no key material ever enters this repository or the
  owner's machine.
- No personal identification is required by SignPath itself; the Foundation vouches for the
  repository-to-binary link with its own name.
- Official artifact configuration supports `<pe-file>` (`.exe`, `.dll`) and `<msi-file>` including
  **deep signing** of nested PE files, so both the NSIS `SwiftFetch_0.1.0_x64-setup.exe` and the MSI
  (with its embedded `swiftfetch.exe` / `swiftfetch_lib.dll`) can be signed in one request.
- Native GitHub Actions integration with origin verification and GitHub-hosted runner enforcement
  matches this repository's existing CI.

### Trade-offs to accept explicitly
- The certificate subject is **SignPath Foundation**, not "Ali Ahasan" or "SwiftFetch". Users will
  see the Foundation as the publisher. This still removes "Unknown Publisher" and gives a verifiable
  repository link, but it is not the same as a certificate in the owner's name.
- New signing identities do **not** get instant SmartScreen trust; reputation accumulates over
  consecutive signed releases.
- Every release requires **manual approval** by a named Approver, and binaries must be built from
  source in a verifiable way. Current reproducible-build status is "not proven" — see
  `INSTALLER_ENGINEERING.md` §5; this must be improved or explicitly accepted before application.
- Eligibility conditions are interpreted by the Foundation, which states it is under no obligation
  to accept any project and may change conditions. Re-verify `signpath.org/terms.html` immediately
  before applying.

## 4. Detailed eligibility check against published conditions

| Published condition | SwiftFetch evidence | Status |
|---|---|---|
| No malware / PUP | Download manager; no scanning/exploitation features. Website states permitted-use and no circumvention claims. | Appears satisfied; owner confirms |
| OSI-approved license, no commercial dual licensing | Root `LICENSE` = GPL-3.0-or-later; `THIRD_PARTY_NOTICES.md` records dependencies. CDLA data-license uncertainty documented and accepted. | Appears satisfied; CDLA caveat retained |
| No proprietary code, system libraries allowed | All source in public repo; no proprietary components identified. | Owner confirms |
| Actively maintained | Recent commits and verification activity in repo. | Appears satisfied |
| Already released in the form to be signed | Unsigned candidate artifacts exist; **no public release yet** (NO-GO). | **Gap** — must be addressed truthfully in the application |
| Functionality documented on download page | GitHub Pages site has Home/Features/Download/Privacy/License/Support pages; Download page currently has owner placeholders, no real release links. | Partially satisfied — owner must prepare final download page |
| Sign own projects / own binaries only | Source and build scripts are maintained in this repo. | Appears satisfied |
| No hacking tools | Not a security-diagnosis or exploitation tool. | Appears satisfied |
| Respect privacy/security; describe collection; provide uninstall | Local-first design; Privacy page; MSI/NSIS uninstall present and tested offline. | Appears satisfied |
| Announce system changes | Installer changes install dir/shortcuts; no silent system configuration changes. | Owner confirms |
| MFA for all team members on SignPath and GitHub | Not verifiable here. | **Owner action required** |
| Roles: Authors, Reviewers, Approvers | Solo maintainer may hold all three, but this must be disclosed and accepted; see application packet. | **Owner decision required** |
| Code signing policy page with exact attribution text | Not yet present. | **Owner action required** |
| Artifact metadata restrictions (product name/version enforced) | Version `0.1.0` consistent across config and binaries; metadata restrictions drafted in architecture doc. | Appears satisfiable |
| Verifiable builds; manual approval per release | Build wrapper pins toolchain and lockfiles; reproducibility **not proven**. | **Gap** — disclose honestly |
| Two-factor + GitHub origin verification in CI | Not configured. | **Owner action required after acceptance** |

## 5. Certificate usability for MSI and NSIS

SignPath artifact configuration reference lists:
- `<pe-file>` — `.exe`, `.dll`, … with `<authenticode-sign/>` → applies to the **NSIS setup executable**.
- `<msi-file>` — `.msi`, `.msm`, `.msp` with `<authenticode-sign/>`, supporting **deep signing** of
  nested PE files then re-packaging and signing the MSI → applies to the **MSI**.
- `<zip-file>` — can carry both artifacts in a single signing request.

A drafted configuration is in `docs/release/SIGNING_ARCHITECTURE.md`. Signing both installers with
one certificate/profile is therefore supported by documented capabilities; it has **not** been
executed here.

## 6. Questions to confirm with the provider (do not guess)

1. Are there country/residency restrictions for an individual maintainer? (Not stated on the pages checked.)
2. Certificate validity and renewal terms for Foundation-issued certificates. (Not published on the pages checked.)
3. Can a single maintainer hold Author, Reviewer and Approver roles simultaneously?
4. What exact publisher name appears in Windows `Get-AuthenticodeSignature`?
5. How is "already released in the form to be signed" interpreted when the candidate is unsigned and unpublished?
6. Which artifact-configuration restrictions are mandatory for OSS subscriptions at application time?

## 7. Sources (all fetched 2026-11-22)

| Source | URL |
|---|---|
| SignPath Foundation terms / OSS conditions | https://signpath.org/terms.html |
| SignPath Foundation overview | https://signpath.org/ |
| SignPath Foundation application form | https://signpath.org/apply |
| SignPath OSS solution | https://signpath.io/solutions/open-source-community |
| SignPath artifact configuration examples (MSI deep signing) | https://docs.signpath.io/artifact-configuration/examples |
| SignPath artifact configuration reference (PE/MSI formats) | https://docs.signpath.io/artifact-configuration/reference |
| SignPath GitHub trusted build system / official action | https://docs.signpath.io/trusted-build-systems/github |
| SignPath project/policy setup | https://docs.signpath.io/projects |
| Microsoft code-signing options (cost, geography, SmartScreen) | https://learn.microsoft.com/en-us/windows/apps/package-and-deploy/code-signing-options |
| Azure Artifact Signing FAQ (no free subscriptions) | https://learn.microsoft.com/en-us/azure/artifact-signing/faq |
| Certum offer verification (not free) | https://foropensource.com/company/certum/ |
| SignPath offer verification | https://foropensource.com/company/signpath/ |
| GitHub Actions billing (public repos free) | https://docs.github.com/en/billing/concepts/product-billing/github-actions |

## 8. Status

No account was registered, no form was submitted, no certificate or key was created, and no
production signing was activated. This is a comparison and recommendation only.
