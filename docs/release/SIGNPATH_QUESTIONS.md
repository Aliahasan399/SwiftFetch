# SwiftFetch — questions for SignPath Foundation (Phase 3.1)

**Status:** INQUIRY REPORTED SENT BY OWNER — question-level send status UNVERIFIED.
The owner reports sending an eligibility inquiry to `support@signpath.io` with subject
"Eligibility clarification — SwiftFetch open-source Windows download manager". The email body was
not provided to this workspace, so **no individual question below is marked SENT**. No reply, ticket
number, response date, support confirmation, or answer is recorded here.
**Owner action:** archive the sent email and any written reply; confirm which questions it contained;
do not invent answers or treat silence as approval.

## Context to include with the message

"SwiftFetch is a public GPL-3.0-or-later Windows download manager (Tauri/Rust/React) maintained as
a free, local-first open-source project. Repository: https://github.com/Aliahasan399/SwiftFetch.
We are preparing an application for the free OSS code-signing program and need written clarification
before submitting. The installers are MSI (WiX) and NSIS EXE."

## Questions

### 1. Bangladesh eligibility
- Are projects whose maintainer resides in **Bangladesh** eligible for a free SignPath Foundation
  code-signing certificate?
- Are there any country, residency, sanctions, or export-control restrictions that would exclude
  this project? If so, which?
- If residence is not checked, is any billing/payment or tax detail required from countries outside
  the US/EU? (We expect none for a free OSS subscription; please confirm in writing.)

### 2. Individual maintainer eligibility
- Does the free program accept an **individual maintainer** (not a registered organization or
  legal entity), or is an organization/entity required?
- If an individual is accepted, what identity or reputation evidence is required beyond the public
  repository and MFA?
- Is a personal website or a specific project age/release count required? (The official terms page
  does not state a numeric rule; please confirm.)

### 3. Solo role separation
- The project currently has one maintainer. Can that person hold **Author, Reviewer and Approver**
  simultaneously?
- If not, may a trusted external reviewer/approver who is not a committer fill the role, and how
  should that be recorded?
- Is a single required approval acceptable for a solo project, or is a higher quorum mandatory?
- Are CI users permitted as the only Submitters while the maintainer is the sole Approver, given
  the trusted-build-system requirement?

### 4. Certificate renewal
- What is the validity period of a Foundation-issued code-signing certificate?
- How and when does renewal happen (automatic, or application required)?
- Are there conditions that cause revocation or non-renewal that the project must plan for?
- Will the publisher name in Windows remain "SignPath Foundation" across renewals?

### 5. MSI deep signing and NSIS executable signing
- Confirm that one signing policy can sign both:
  - `SwiftFetch_0.1.0_x64_en-US.msi` with **deep signing** of its embedded PE files
    (`swiftfetch.exe`, `swiftfetch_lib.dll`) before the MSI is signed; and
  - `SwiftFetch_0.1.0_x64-setup.exe` (NSIS installer, a PE file) with Authenticode.
- Can both artifacts be submitted in one ZIP-based signing request with an artifact configuration
  containing `<msi-file>` and `<pe-file>` entries?
- Are there mandatory metadata restrictions (subject, product name, product version, company name)
  for OSS subscriptions, and what exact values should SwiftFetch use?
- Is a timestamping service applied by default, and will it be recorded in the signed artifact?

## Additional contextual questions

- How should "already released in the form to be signed" be interpreted when the candidate exists
  but is intentionally unsigned and unpublished pending NO-GO?
- Are verifiable-build requirements satisfied by pinned toolchain, `npm ci`, Cargo `--locked` and
  GitHub-hosted runners even though byte-identical reproducible builds are not yet proven?
- Which exact wording must appear on the policy page and where (home page, download page, dedicated
  page)? We currently use: "Free code signing provided by SignPath.io, certificate by SignPath
  Foundation".

## Response-recording section — written answers only

No answer, ticket number, or response date is recorded. "Prepared in 3.1" means the topic appears in
this document; it does **not** prove it was included in the owner's sent email. "Identified in 3.2"
means the topic was found in the gap review and was **not** in this document before that review.

| # | Topic | Origin | Question-level send status | Written answer | Date received | Source / ticket | Impact on application |
|---|---|---|---|---|---|---|---|
| 1 | Bangladesh eligibility | Prepared in 3.1 | UNVERIFIED — awaiting written answer | — | — | — | — |
| 2 | Individual-maintainer eligibility | Prepared in 3.1 | UNVERIFIED — awaiting written answer | — | — | — | — |
| 3 | Solo Author / Reviewer / Approver arrangement | Prepared in 3.1 | UNVERIFIED — awaiting written answer | — | — | — | — |
| 4 | CDLA-Permissive-2.0 compatibility with OSI license condition | Identified in 3.2 | UNVERIFIED — inclusion in sent email not evidenced | — | — | — | — |
| 5 | Existing public release requirement ("released in the form to be signed") | Prepared in 3.1 (contextual) | UNVERIFIED — awaiting written answer | — | — | — | — |
| 6 | Reputation expectations for a new OSS download manager | Identified in 3.2 | UNVERIFIED — inclusion in sent email not evidenced | — | — | — | — |
| 7 | Website and privacy contact requirements | Identified in 3.2 | UNVERIFIED — inclusion in sent email not evidenced | — | — | — | — |
| 8 | Certificate renewal | Prepared in 3.1 | UNVERIFIED — awaiting written answer | — | — | — | — |
| 9 | MSI deep signing and NSIS executable signing | Prepared in 3.1 | UNVERIFIED — awaiting written answer | — | — | — | — |

### Recording rules
- Fill a row only from an actual written reply from SignPath; record its date and source (email,
  ticket URL or support thread). Do not copy assumptions into the answer column.
- If the owner confirms a question was in the sent email, change only that row's send status to
  `SENT / AWAITING RESPONSE` and record the owner's confirmation as the basis. Do not mark all rows
  sent on the basis of the subject line alone.
- Do not treat absence of a reply as approval. Do not submit the application until items 1–4 and 7
  have written answers, and until the "released" condition (5) is resolved in writing or by an
  owner decision that does not change NO-GO.
