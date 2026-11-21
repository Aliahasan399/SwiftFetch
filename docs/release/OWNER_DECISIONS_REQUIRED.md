# Owner Decisions Required — Master Prompt Phase 1

**Date:** 2026-11-21 · **Status:** open decisions listed; none decided by the agent

## D-01 — Final software license
- Question: Adopt GPL-3.0-or-later finally, or choose another license?
- Options: (a) GPL-3.0-or-later (recommended: conditionally compatible, matches free/open-source intent); (b) alternative permissive license; (c) keep unresolved.
- Consequences: (a) requires accepting CDLA risk + adding root LICENSE; (b) requires new compatibility audit; (c) public repo cannot claim a license.
- Current state: CONDITIONAL approval only; root LICENSE absent.

## D-02 — CDLA-Permissive-2.0 (`webpki-roots` 0.26.11 / 1.0.11)
- Question: Accept the unresolved permissive-data-license compatibility risk, seek counsel/copyright-holder confirmation, or replace the dependency?
- Recommendation: seek confirmation; do not assume compatibility; do not change dependency versions merely to pass the audit.
- Consequence: unresolved = license remains conditional; public release blocked.

## D-03 — Copyright ownership statement
- Question: Who is the copyright holder and which years?
- Recommendation: owner supplies exact text; no attribution invented. Needed before root LICENSE.

## D-04 — Publisher identity / contact / legal documents
- Question: What personal publisher identity, support contact, privacy contact, Terms and Privacy details should be published?
- Recommendation: use `[OWNER: ...]` placeholders until supplied; never invent.

## D-05 — Authenticode signing route
- Question: Submit SignPath Foundation application (free, trusted) or another eligible route?
- Recommendation: SignPath fits zero budget; eligibility gaps (public repo now met; LICENSE not met; reproducible-build proof; 2FA) must be addressed first. Requires explicit owner approval to submit formal declarations.

## D-06 — Automatic updater activation
- Question: Generate production updater keys and activate a GitHub Releases endpoint?
- Recommendation: owner generates keys locally, approves HTTPS endpoint, then real E2E test; never before. Dev keys may be used for local verification only.

## D-07 — Production release authorization (GO)
- Question: Authorize final production release after all gates are evidenced?
- Recommendation: NO until all mandatory gates pass; the owner alone grants publication authorization.

## D-08 — Startup performance target
- Question: Accept ~23–37 s cold render (WebView2 architecture) or fund/expect optimization?
- Recommendation: owner formally accepts measured result or requests a scoped optimization follow-up; do not silently redefine the <1 s target.

## D-09 — Repository identity/security
- Question: Real Git author name/email; enable 2FA; enable Dependabot alerts; fill remaining `[OWNER:]` placeholders (~120 across 22 files).
- Recommendation: owner applies; agent cannot set 2FA or supply personal data.

## D-10 — Accessibility / stability acceptance
- Question: Provide screen-reader personnel and a 24-hour soak window/site list?
- Recommendation: owner supplies test resources; agent prepared procedures; no certification claimed.
