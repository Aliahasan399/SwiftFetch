# Security Policy

## Supported versions

| Version | Supported |
| --- | --- |
| 0.1.0 (release candidate) | Pre-release only — no public production release yet |

## Reporting a vulnerability

Do **not** open a public issue for a suspected vulnerability.

Report privately using GitHub's private vulnerability reporting on this
repository, if enabled: **Settings → Security → Security advisories → Report a
vulnerability**. If private reporting is not enabled, the owner must enable it
before public distribution.

[OWNER: add a private security contact channel here if GitHub private reporting
is unavailable. Do not publish personal contact details without consent.]

Please include: affected version, reproduction steps, impact, and any suggested
fix. Do not include live credentials or private keys.

## Scope and boundaries

SwiftFetch is a local-first download manager. Security-relevant boundaries are:

- URL scheme, redirect, destination path, and filename validation in the Rust core.
- Browser Native Messaging consent, origin allow-listing, replay/expiry checks.
- Local SQLite state and optional Windows DPAPI protection for saved request headers.
- Signed update verification (code present; publisher configuration currently unset).

Out of scope: DRM or access-control circumvention, credential harvesting, hidden
background downloads, and executing downloaded files automatically.

## Disclosure

We will acknowledge private reports, investigate, and coordinate a fix and
disclosure timeline with the reporter. Public issues without a fix may be closed
or converted to private advisories at the maintainer's discretion.

See [docs/SECURITY.md](docs/SECURITY.md) for the implementation security notes and
[RELEASE-READINESS.md](RELEASE-READINESS.md) for current release gates.
