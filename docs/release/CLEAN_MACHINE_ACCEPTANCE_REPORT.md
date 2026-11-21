# Clean-Machine Acceptance Report — index

**Status: BLOCKED_EXTERNAL — no clean Windows VM available. No acceptance executed.**

The prepared test plan is in `docs/release/CLEAN_MACHINE_ACCEPTANCE.md` (environment
requirements, install/upgrade/rollback/uninstall sequence, failure tests, data
preservation verification). Historical isolated-profile runtime evidence exists in
`artifacts/smoke/run-*` and `VERIFICATION.md`, but those are **not** clean-machine runs
and must not be presented as such.

## Required scenarios (not yet executed)
Fresh MSI and NSIS installation; launch/shutdown; shortcuts; data directory;
permissions; uninstall; version-to-version upgrade with populated queue/history/
settings; failed update; uninstall preservation; interrupted install; low disk;
locked files; corrupted installer.

## Completion criteria
A `clean-machine-result.json` with real signatures, hashes, environment details,
and per-scenario pass/fail from an isolated Windows VM. Until then this gate is open
and the final verdict remains NO-GO.

Owner dependency: `[OWNER: provide a clean Windows VM/Sandbox environment]`.
