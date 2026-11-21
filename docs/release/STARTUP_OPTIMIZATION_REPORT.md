# SwiftFetch — Startup Optimization Report (candidate backlog)

**Date:** 2026-11-21 · **Lane:** `performance-agent` (analysis only)
**Companion document:** `docs/release/STARTUP_PERFORMANCE_AUDIT.md`

> ## Status declaration
>
> **NO code changes were made in this lane.** No source file, configuration file, build script,
> harness, or release artifact was edited. No commits were created. No benchmark was re-run.
> Every item below is a *candidate* derived from existing artifacts. Any implementation must be
> carried out by the parent agent, must preserve the evidence chain, and must be accompanied by
> regression tests. **No speedup is promised** — the "ceiling" column is the upper bound of the
> phase cost observed in the data, not an achievable or measured improvement.

---

## 1. Evidence basis

All candidates are anchored to these measured facts (see the audit for full tables and paths):

| Fact | Value | Source |
|---|---|---|
| Rust setup total (process_start → engine_started) | 3.0–8.6 s, median 4.6 s | both `timing-*/timing.json` |
| SQLite open (pre-window) | 1.0–6.0 s, median 1.3 s; cold-labeled median 2.2 s | both `timing-*/timing.json` |
| WebView2 window build | 1.6–3.7 s, median 3.1 s | both `timing-*/timing.json` |
| All other app phases | 14–55 ms | both `timing-*/timing.json` |
| Full cold render (interactive) | 23–37 s, median 31.5 s (n=24) | `run-TDPTEn`, `run-fiWmSE` results.json |
| Unexplained post-window gap | ~15–30 s, uncharacterized | difference of the above |
| Frontend marks emitted but never captured | `swiftfetch:frontend-ready`, `swiftfetch:health-acknowledged` | `src/desktopBoot.ts:13,15` |
| `rendered_ms` in timing harness | `null` for all 8 modes | both `timing-*/timing.json` |
| Eager-navigation blank-page defect (fixed in code) | `about:blank`, 25 s | `run-mpvnRe`, `run-Z4orbO` |

Dominant hypothesis (H1): the cost is inside WebView2 first paint / React mount **after** window
creation. Application pre-window work and harness polling are ruled out as primary causes.

---

## 2. Prioritized candidate backlog

| # | Candidate | Targets | Evidence ceiling* | Risk | Priority |
|---|---|---|---|---|---|
| 1 | Wire instrumentation to the existing frontend marks + capture paint timing + true warm-relaunch runs | Attribution gap | none (measurement only) | Low | **P0 — prerequisite** |
| 2 | Persistent / pre-warmed WebView2 process (hidden window or resident helper) | 15–30 s post-window gap | up to ~15–30 s theoretical | **High** — architecture + owner acceptance | P1 |
| 3 | Overlap WebView2 window build with store/engine setup (parallelize `setup()`) | 1.6–3.7 s window build | ≤~3 s | Medium-high — blank-page regression risk | P2 |
| 4 | Defer SQLite migrations / default-catalog seeding off the startup path | 1.0–6.0 s SQLite open | ≤~6 s worst case | Medium — correctness, recovery, lock contract | P3 |
| 5 | Pre-seeded EBWebView profile / browser-argument tuning for first-run caches | first-run WebView2 init | unknown (possibly seconds) | Medium — WebView2 version compatibility | P4 |
| 6 | Lazy plugin / integration initialization (updater, tray, clipboard, opener) | post-window app work | ≤55 ms | Low | P5 — negligible benefit |
| 7 | Frontend bundle / React-mount trimming | post-window gap (JS side) | unknown | Low-medium — needs P0 data first | P6 |

\* "Evidence ceiling" = the size of the phase this candidate can address. It is **not** a
predicted improvement. Actual gains are unmeasured and may be zero.

---

## 3. Candidate detail

### P0 — Wire instrumentation first (prerequisite, not an optimization)

**What:** In the measurement harness (parent-owned), read `performance.getEntriesByName()` for
`swiftfetch:frontend-ready` and `swiftfetch:health-acknowledged`, capture
`performance.getEntriesByType('paint')` / CDP `Page` lifecycle events, and write them into
`timing.json` alongside the Rust phases. Add a harness mode that genuinely reuses an existing
profile for a warm-relaunch measurement.

**Why:** `src/desktopBoot.ts` already emits both marks, and the comment at line 12 says they are
"read by the startup probe" — but neither `tools/startup-timing.mjs` nor
`tools/startup-probe.mjs` reads them, which is why `rendered_ms` is `null` in every mode. Until
this is fixed, the 15–30 s gap cannot be attributed between WebView2 runtime start, navigation,
JS evaluation, and React mount, so candidates 2/5/7 cannot be sized.

**Risk:** Low — measurement-only. Must keep the privacy guarantee: labels and timings only, no
URLs, paths, or settings (`lib.rs` `startup_trace` already follows this rule).

**Regression tests:** confirm marks appear in `timing.json` for all modes; confirm
`rendered_ms` is populated; confirm no sensitive data in artifacts.

---

### P1 — Persistent / pre-warmed WebView2 process

**What:** Keep a WebView2 renderer resident (hidden window or long-lived helper) so subsequent
launches skip runtime start, GPU shader-cache warmup, first navigation, and JS bundle
evaluation. This is the only candidate that directly targets the dominant 15–30 s gap.

**Why:** Every measured phase before/around window creation is ≤6 s, yet interactive render is
23–37 s. The unexplained remainder is structural to WebView2 on this host, so only avoiding the
cold WebView2 path — not tuning it — can plausibly approach a sub-second figure.

**Risk / required owner acceptance:** **High.** Changes process lifetime and memory residency;
interacts with the single-scheduler `.lock` contract (`lib.rs:82-94, 229-239`), tray hide/exit
semantics (`lib.rs:252-263`), updater/recovery watchdogs, and installer/service design. Also
raises privacy/security questions about a permanently running renderer. Must not break the
"one scheduler per queue database" invariant or leave a hidden window that blocks shutdown.

**Regression tests:** full smoke suite, lock-exclusivity test, tray exit drains workers,
update/rollback paths, cold-start recovery, and a new warm-launch timing test.

**Honest bound:** This is the only candidate with a theoretical path to a large gain, but it is
an **architectural change requiring owner sign-off**, not a tuning fix. No gain is promised.

---

### P2 — Overlap WebView2 window build with store/engine setup

**What:** Currently `setup()` is strictly sequential: `Store::open` → settings/HTTP → engine
build → `WebviewWindowBuilder...build()` → tray → `engine.start()` (`lib.rs:264-338`). Explore
starting WebView2 construction earlier so the ~1.6–3.7 s window build overlaps part of the
~1–6 s SQLite open.

**Why:** These two phases are adjacent and independent in principle; their overlap is bounded by
the smaller of the two, i.e. ≤~3 s.

**Risk / required owner acceptance:** **Medium-high.** `lib.rs` deliberately publishes native
state before creating the WebView (lines 317–321) specifically because an eager initial
navigation previously remained on `about:blank` (`run-mpvnRe`, `run-Z4orbO`). Reordering risks
reintroducing that defect. Tauri setup ordering and the `app.manage(...)` state publishes must be
preserved exactly.

**Regression tests:** UIA `RENDERED` assertion across ≥12 cold runs (same shape as
`startup-probe.mjs`), explicit `about:blank` regression check via CDP frame URL, full smoke suite.

---

### P3 — Defer SQLite migrations / default-catalog seeding

**What:** Investigate moving `ensure_default_queue()` / `ensure_default_category()`
(`lib.rs:277-280`) and any implicit migrations off the critical path, e.g. to a background task
after first paint, while keeping the DB handle open and the lock intact.

**Why:** SQLite open is 1.0–6.0 s, cold-labeled median 2.2 s vs warm-labeled 1.2 s — the only
app-side phase with meaningful cold/warm variance. It is a genuine, bounded contributor.

**Risk / required owner acceptance:** **Medium.** Default catalogs must exist before commands
that read them run; corrupt-DB and recovery flows (`recovery::show_startup`) must still behave;
the single-scheduler lock must still be acquired before any queue access. Deferring schema work
can produce first-run races that only appear on truly fresh profiles — exactly the condition the
cold probe exercises.

**Regression tests:** corrupt-DB startup recovery (smoke group), cold fresh-profile render,
concurrent Native-Messaging enqueue vs scheduler ownership, command-level tests for lists of
queues/categories.

---

### P4 — Pre-seeded EBWebView profile / browser-argument tuning

**What:** Evaluate shipping a pre-initialized WebView2 user-data folder or adjusting
`WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS` to suppress first-run work (extension stores, GPU/GrShader/Dawn
cache population, background components) observed in the `timing-*` profile trees.

**Why:** First-run profile dirs contain many caches and LevelDB/extension stores; this is
consistent with a portion of the post-window WebView2 cost.

**Risk / required owner acceptance:** **Medium.** WebView2 runtime versions differ; pre-seeded
profiles can be invalidated or rejected, can bloat the installer, and can carry privacy
implications. Gains are unknown until P0 instrumentation isolates WebView2-runtime time.

**Regression tests:** install from a clean profile, all three browser acceptance runs, cache
invalidation on runtime upgrade, privacy check on shipped profile contents.

---

### P5 — Lazy plugin / integration initialization

**What:** Defer `tauri_plugin_updater`, clipboard/opener plugin init, and tray registration
beyond first paint.

**Why:** Listed for completeness, but **explicitly low priority**: every post-window app phase
is 14–55 ms, so the measured ceiling for this whole class is ≤55 ms — immaterial against a
23–37 s render. The current code already builds the WebView before the tray specifically for
this reason (`lib.rs:326-335`).

**Risk:** Low but benefit near zero. Do not spend parent effort here before P0–P3.

**Regression tests:** n/a unless implemented; then tray/exit and updater smoke groups.

---

### P6 — Frontend bundle / React-mount trimming

**What:** Reduce JS payload or defer non-critical React work until after first paint, once P0
 marks identify how much of the 15–30 s gap is JS evaluation vs WebView2 native startup.

**Why:** Plausible contributor to H1, but currently unmeasured. The `topbar` CDP selector and
`Add download` UIA element both require React mount, so mount time is inside the gap.

**Risk:** Low-medium — UX regressions, and changes here cannot be sized without P0 data.

**Regression tests:** frontend test suite (41 tests per GO_NO_GO_REPORT), UIA render probe,
bundle-size check.

---

## 4. Explicitly rejected / non-candidates

- **"The CDP harness is causing the slow startup."** Rejected: CDP vs noCDP window-build means
  are 2.96 s vs 2.80 s; the `about:blank` runs are a separate, now-addressed correctness defect.
- **"UIA polling overhead explains the 23–37 s."** Rejected: polling bound is ~1 s (~2–4%).
- **"App pre-window work is the main cost."** Rejected: ≤6 s worst case, ≤55 ms for everything
  other than SQLite open.
- **"Just reorder a few calls to get under 1 s."** Rejected: the floor structural milestone
  (window built) is already ≥3.0 s.

---

## 5. Cross-cutting risk analysis

| Risk | Affected candidates | Mitigation |
|---|---|---|
| Reintroduce `about:blank` first navigation | P2 | Preserve native-state-before-WebView ordering (`lib.rs:317-321`); CDP frame-URL regression check |
| Break single-scheduler lock contract | P1, P3 | Keep `acquire_desktop_lock` before any queue access; existing unit test `two_desktop_engines_cannot_own_the_same_queue` |
| Break corrupt-DB / recovery startup | P3 | Re-run smoke group 15/15 incl. full restore and corrupt-DB startup |
| WebView2 version incompatibility | P4, P1 | Test across installed runtime versions; avoid undocumented flags |
| Privacy leak in new timing data | P0, P1 | Labels + milliseconds only; no URLs/paths/settings |
| False speed claims | all | Re-measure with the same harness; report min/median/max from new artifacts; never publish a projected number as a result |

---

## 6. Handoff (required parent actions)

1. Decide the **owner question** first: accept a revised startup target, or fund the architectural
   change (P1). `GO_NO_GO_REPORT.md` lists "startup target accepted or fixed" as a GO condition;
   this lane cannot make that decision.
2. Implement **P0 instrumentation** in the parent lane before any optimization so the 15–30 s
   gap can be attributed.
3. Implement only owner-approved candidates, one at a time, with the regression tests listed
   above and a fresh `timing.json` / `results.json` captured for comparison.
4. Update `GO_NO_GO_REPORT.md` only after new measured evidence exists — not from the ceilings in
   this document.

Again: this lane produced **analysis documents only**. No fix was applied, no speed is promised,
and every candidate above remains unimplemented and unvalidated.
