# First-release completion ledger

The authoritative scope is the first Windows release in the architecture at
`docs/reference/IDM-DOWNLOAD-MANAGER-ARCHITECTURE.md`, including its release gates. Deferred
torrents, FTP, cloud/enterprise services, and other platforms are not first-release
requirements. This ledger tracks code, runtime evidence, and external release
requirements separately.

| Workstream | Owner | Acceptance |
| --- | --- | --- |
| Task lifecycle, collision decision, retention and safe recovery | Cent AI HY4 + main | Implemented; 90 Rust tests passed, including six lifecycle tests and 18 actual transfer/scanner tests. This pass added safe server filenames (12 focused tests) and five full-restore recovery tests |
| Clipboard/drop capture, localized UI, accessibility, new controls | HCNsec HY4 + main | Implemented; TypeScript/build and 37 regressions passed; packaged drop/labels/collision/power-cancel checks passed; update health, Cancel gating and unsaved-dialog preservation fixed |
| Browser compatibility and privacy/support documentation | Nvidia Step 5 + main | Edge native-host acceptance passed; **live Firefox 160.0.1 acceptance passed** with real add-on load, native messaging and checksum-matched output; 83 browser tests pass; temporary host registration removed |
| Signed updater, rollback, tray, power and platform adapters | Main + independent review | Signature/tamper/version-replay/cache tests passed; final updater hardening pass pending; real signed installer recovery remains unverified |
| Resource, installer, final integration and packaging | Main | 256 MiB streaming output/hash passed with 9 MiB Rust working-set growth; deferred WebView creation, 12-launch cold-start probe and packaged restore acceptance; see VERIFICATION.md |
| Encrypted local state (DPAPI) | Cent AI HY4 + main | Implemented opt-in Windows DPAPI header encryption, transparent decrypt and atomic migration; 5 integration + 59 lib tests + real-DPAPI harness passed |
| Updater hardening | Main | DB snapshot bound to rollback attempts, durable phases + startup reconciliation, MSI INSTALLDIR and one recursive 1 GiB budget; 3 hardening tests pass |
| Accessibility and full dictionary parity | HCNsec HY4 + main | en/bn 72/72 key parity, dialog focus/focus-return, labelled controls, polite progress announcements; 41 frontend tests pass (SR/visual certification still external) |

Canonical verification (Phase 1, 2026-11-21): `./tools/verify.ps1` passed all
7 checks with machine-readable report
`artifacts/verification/report-20261121-184958.json` (full log `artifacts/final-verify.log`); totals are 46 fixture,
128 Rust (59 unit + 69 integration across 8 binaries), 41 frontend, 83 browser
(70 + 13 Firefox background, now wired into the canonical command). Older counts
above are historical records of earlier runs, preserved as evidence of progression.

No external distribution, browser-store submission, signing-key generation for a
publisher, license selection, or certificate purchase is implied by passing a
local test. Actual evidence and owner-supplied inputs are recorded at completion.

## Remaining external gates and acceptance limits

- Publisher Authenticode certificate, real update HTTPS endpoints/public key,
  signing environment, distribution license, and approved publisher/contact terms.
- Clean Windows install/upgrade/rollback/uninstall acceptance. Windows Sandbox
  is not installed here; tools prepare a test configuration without enabling OS
  features or altering this machine's installed SwiftFetch.
- Firefox temporary add-on load and native handoff passed; a permanent installation
  and any browser-store signing/review are still required. Edge acceptance is real
  but does not establish all browser/store combinations.
- A 24-hour soak, broad real-site compatibility, full translation and independent
  accessibility/security certification are not claimed by the short local tests.

These remaining items mean the project is not represented as an owner-approved
public release, despite the completed implementation and local verification.
