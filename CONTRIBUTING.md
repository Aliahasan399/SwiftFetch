# Contributing to SwiftFetch

SwiftFetch is an IDM-style desktop download manager for Windows: a Rust core
(`src-tauri`) behind a Tauri v2 + React 19 + TypeScript (Vite) frontend (`src`),
plus a browser extension (`browser-extension`). Thanks for helping out.

## Ground rules

- `CONTRACT.md` is the single source of truth for the IPC wire protocol. Do not
  rename Tauri command names, event names, or JSON field names without
  coordinating the whole stack first.
- Never commit secrets: no credentials, API keys, signing keys, tokens, cookies,
  private download URLs, database backups, or `.env` files. Issues and pull
  requests must be reproducible from public information alone.
- Keep changes focused and small. One concern per pull request.
- Do not commit build outputs, `node_modules`, or `target/` directories.

## Local development

Prerequisites: a stable Rust toolchain (edition 2024), Node.js with npm,
Python 3 (for the HTTP fixtures), and a Windows machine (the product and CI
target Windows x64).

```bash
npm install          # install frontend dependencies
npm run dev          # Vite dev server
npm run build        # tsc && vite build
npm run tauri -- <cmd>   # Tauri CLI (e.g. dev, build)
```

Rust commands run from the repo root:

```powershell
cargo check --locked --all-targets --manifest-path src-tauri/Cargo.toml
cargo test  --locked --manifest-path src-tauri/Cargo.toml
```

## Verification before a pull request

Run the full pipeline (it also writes a JSON report under
`artifacts/verification/`):

```powershell
powershell -File tools/verify.ps1   # or: bash tools/verify.sh
```

It runs, in order:

1. HTTP fixture self-test — `python -B tools/fixtures/selftest.py`
2. Rust compilation — `cargo check --locked --all-targets`
3. Rust tests — `cargo test --locked` (suites in `src-tauri/tests/`)
4. Desktop UI build — `npm run build`
5. Desktop UI regressions — `node src/testing/verify.mjs`
6. Browser protocol and package tests — `node --test browser-extension/test/test_protocol.js browser-extension/test/test_release.js`
7. Firefox background behavior tests — `node --test browser-extension/test/test_firefox_background.test.mjs`

A pull request is expected to land with all steps green. For transfer-engine
work, use the local fixture server for reproduction — it binds to loopback only
and has deterministic routes for ranges, redirects, retries, and checksums:

```bash
python tools/fixtures/server.py --host 127.0.0.1 --port 8734
```

## Code style

Rust:

- 4-space indent; Rust 2024 edition.
- Serde JSON uses snake_case inside request/response structs (no rename
  attributes); Tauri camelCases the top-level argument. See `CONTRACT.md` §0.
- Keep warnings clean in code you touch; `cargo check --locked` must pass.

TypeScript / React:

- Match existing conventions in `src/` (`types.ts` mirrors `domain.rs`,
  `ipc.ts` wraps typed `invoke`/`listen`, small modules with focused files).
- Hand-written CSS only — no Tailwind, no component libraries. Dark theme is
  the default.
- Accessibility is a requirement, not polish: keyboard reachable controls,
  focus-trapped dialogs, accessible progress text, and color never the only
  state indicator.

## Reporting issues

Include the application version, the error code, what happened, what you
expected, and a redacted reproduction (the local fixture is ideal). Never
include private URLs, request headers, cookies, or database backups.
Issue tracker: [OWNER: issue tracker URL].

## Pull requests

- Reference the issue you solve, describe what and why, and list the
  verification steps you ran (paste the `tools/verify.ps1` result).
- Add or update tests for changed behavior; bug fixes should include a
  regression test.
- Sign-off is optional: a `Signed-off-by` / DCO line is welcome but not
  required for review.
- Expect review feedback on wire-protocol compatibility, security behavior
  (`src-tauri/src/security.rs`), and test coverage.

## Security reports

Do not report vulnerabilities in public issues. Use the private contact in
`docs/SECURITY.md` / [OWNER: private security contact] and follow the incident
process documented there.

## Licensing and conduct

The proposed distribution license is GPL-3.0-or-later (owner decision, pending
completion of the dependency license audit — see `docs/release/LICENSE_AUDIT.md`
and `THIRD_PARTY_NOTICES.md`). By contributing you agree that your contribution
falls under the project license. Participation is governed by
`CODE_OF_CONDUCT.md`.