# SwiftFetch — Author / Reviewer / Approver arrangement for a solo maintainer

**Status:** ANALYSIS AND RECOMMENDATION ONLY. No roles are assigned in SignPath; no account exists.
**Phase 3.1 · NO-GO preserved.**

## 1. What SignPath publishes

From `signpath.org/terms.html` (fetched 2026-11-22), an OSS project must:
- use MFA for all team members on both SignPath and the source repository;
- define **Authors** (can modify source without additional review), **Reviewers** (review changes
  from non-committers), and **Approvers** (each signing request is manually approved by a team
  member trusted by the whole team).

From `docs.signpath.io/projects` (fetched 2026-11-22):
- a signing policy can require manual approval with a configurable number of approvals;
- **a single deny aborts** the request (k-out-of-n);
- trusted build system verification is required for Open Source Code Signing;
- when trusted build system verification is enabled, **interactive users cannot be Submitters** — CI
  must submit;
- origin verification is required and needs a project repository URL;
- release-signing should be restricted to release branches such as `main`;
- a separate `test-signing` policy with a self-signed certificate is recommended.

## 2. Current SwiftFetch reality

- One repository owner is the only identified maintainer.
- No second maintainer, team, or organization is configured.
- GitHub branch protection beyond defaults is not verified here.
- No SignPath organization, project, roles, or policies exist yet.

A single natural person can technically be assigned all three role labels, but the *control* each
label represents is then held by one person. This is a disclosure question for SignPath Foundation,
not something to assert silently.

## 3. Risk analysis

| Risk | Effect | Mitigation available now |
|---|---|---|
| Single compromised account can author, review and approve a malicious build | Signed malware under the Foundation's name | MFA; branch protection; signed commits; CI-only submission with origin verification; manual approval |
| No independent reviewer for a compromised maintainer | Review and approval separation collapses | Disclose to Foundation; recruit a second trusted maintainer, or accept with explicit written confirmation |
| CI token misuse | Unauthorized signing requests | Token restricted to submitter only, never approver; secret not exposed to forks; rotation plan; audit log review |
| Signed test builds | Release identity used for experiments | Separate test-signing policy with self-signed certificate; release policy restricted to `main` |
| Accidental release from a feature branch | Wrong code signed | Signing policy allowed-branch restriction to `main`; origin verification |

## 4. Recommended arrangement (to propose, not yet implemented)

1. **Submitter:** the GitHub Actions workflow only (CI user in SignPath). Interactive users are not
   submitters, matching the OSS requirement.
2. **Authors / Reviewers / Approvers:** owner holds all three today. Disclose this explicitly in
   the application and in the prepared questions (`SIGNPATH_QUESTIONS.md`).
3. **Approval threshold:** 1 approval (the owner) until a second trusted maintainer exists. Never
   allow the CI token to approve. If a second maintainer joins, raise to 2 approvals and assign
   distinct Reviewer/Approver identities.
4. **Branch controls:** protect `main` with required pull-request review (by any second trusted
   person if available) and no force pushes; restrict the release-signing policy to `main`.
5. **Separation of duties substitutes until then:** signed commits, mandatory verification of build
   provenance from GitHub metadata, manual review of each signing request, and post-release audit-log
   inspection.
6. **Test policy:** `test-signing` with a self-signed certificate for pipeline rehearsals; release
   certificate only in `release-signing`.

## 5. Decision gate for the owner

- If SignPath Foundation confirms a solo maintainer holding all three roles is acceptable, proceed
  with the arrangement above and record the confirmation.
- If not, either recruit a second trusted maintainer before applying, or defer the application.
- Do not claim role separation that does not exist.

## 6. Status

No SignPath account, role assignment, approval policy, or branch protection change was made in this
phase. This document is a recommendation for owner review only.
