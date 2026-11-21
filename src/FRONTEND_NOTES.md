# Frontend implementation handoff

## Scope and structure

Only src source files were authored or changed. App.tsx and App.css were backed up beside themselves as .bak-20261110-102858 before modification. Existing main.tsx, vite-env.d.ts and assets were retained. No dependencies, backend files, tools, commits, or Git history were changed. The working directory was not a Git repository.

The frontend has typed snake_case IPC, a useSyncExternalStore projection cache, seven views, paginated task/catalog/event lists, native focus-trapped dialogs, core-acknowledged actions, explicit destructive-action confirmations, dark/light/system themes, and handwritten CSS. Private task URLs, paths and checksum values are excluded from the global task summaries and fetched for the open details dialog.

All command wrappers use exactly the top-level request objects shown in CONTRACT.md section 2. No speculative command or action was added. Confirm the Rust command parameter binding during integration: the contract table describes top-level fields, not a nested `request` or `settings` parameter.

## Verification

Required build: `npm run build` (tsc && vite build).
Regression runner: `node src/testing/verify.mjs`.
The regression runner uses explicitly synthetic IPC fixtures and no native backend. It checks Rust/TS field parity and nullability, settings control coverage, validation/redaction, progress completion semantics, event mapping/cleanup, snapshot races and command payloads.

For repeat builds, use `npm run build -- --emptyOutDir false` to retain existing generated artifacts instead of Vite deleting the output directory. The first exact `npm run build` passed before additional review fixes.

Final executed command: `npm run build -- --emptyOutDir false && node src/testing/verify.mjs`.
Result: exit 0; TypeScript and Vite build passed (50 modules, built in 4.10s). Vite reported an informational plugin-timing notice. The final JavaScript artifact is `dist/assets/index-AxFifuHK.js` (313.71 kB; gzip 95.11 kB), CSS is `dist/assets/index-Drnj_2et.css` (26.39 kB; gzip 6.10 kB). Regression result: 27 passed, 0 failed, 27 total. The source review identified store/read-back ordering and modal feedback issues; fixes and regression coverage were included before this final run.

Browser/keyboard visual QA, real Tauri IPC, native file transfers and a long-session soak were not executed in this frontend task. Native integration remains the main agent's responsibility.

## Contract gaps (not simulated)

- There is no command/action for editing an existing task's priority. Priority is editable on Add download; queue priorities are read-only with an explanation.
- Individual segment ranges/progress, raw headers, ETag/Last-Modified and per-attempt retry history are absent. Details show available counters, content type/length, checksum and lifecycle metadata, and explicitly state what is missing.
- Scanner results/readiness, browser bridge connectivity and disk free-space telemetry are absent. The UI never claims these checks passed and provides manual-add or recovery guidance.
- Ask overwrite mode is specified as replacement in this core contract. Settings and Add download warn explicitly and require consent for replacement; Rename remains the safe default. No nonexistent overwrite event is invented.
- Resume metadata invalidation may cause the core to restart automatically. The UI confirms explicit restart/resume actions when incompatibility or non-resumability is known, but cannot intercept an unexposed native restart decision.
- list_tasks has only limit, no offset/cursor/filter. UI pages contain 25 items; queries start at 1,000 and can expand to 10,000. Counts/search are labeled as covering loaded tasks when capped.
- Scheduler days use Monday bit 0 through Sunday bit 6. After-window strings offered are `nothing`, `pause`, `shutdown`; unknown persisted values are preserved. Verify these semantics against the engine during integration because the contract does not enumerate them.
- Settings are persisted requests, not proof of functioning OS startup, power, scanner or browser adapters.

No file-delete action is exposed. control_download always sends delete_file: false. History clearing only requests metadata removal. Files never open automatically; open_file requires explicit confirmation.
