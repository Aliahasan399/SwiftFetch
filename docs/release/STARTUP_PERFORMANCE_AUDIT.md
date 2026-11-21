# SwiftFetch — Startup Performance Audit

**Date:** 2026-11-21 · **Lane:** `performance-agent` (read-only analysis; no source changes)
**Scope:** interpretation of already-captured startup artifacts only. No new benchmark runs,
no code changes, no config changes, no commits, no speculative optimizations. This document
reports what the evidence supports and labels plainly what it does not.

---

## 1. Sources read (exact paths)

| Category | Path |
|---|---|
| Timing traces (both runs) | `artifacts/startup-probe/timing-m7opgI/timing.json` |
| | `artifacts/startup-probe/timing-Mblblm/timing.json` |
| Full cold-render probes | `artifacts/startup-probe/run-TDPTEn/results.json` (12/12 rendered) |
| | `artifacts/startup-probe/run-fiWmSE/results.json` (12/12 rendered) |
| Historical failed probes (context only) | `artifacts/startup-probe/run-EsmcG1/results.json` + `1/failure.json` (`NO_WINDOW`) |
| | `artifacts/startup-probe/run-gg5ogA/results.json` + `1/failure.json` (`WINDOW_ONLY`) |
| | `artifacts/startup-probe/run-mpvnRe/results.json` + `1/failure.json` (`about:blank`) |
| | `artifacts/startup-probe/run-Z4orbO/results.json` + `1/failure.json` (`about:blank`) |
| Harness implementations | `tools/startup-timing.mjs`, `tools/startup-probe.mjs` |
| Setup ordering | `src-tauri/src/lib.rs` (`run()`, `startup_trace`, `.setup()` closure) |
| Frontend boot | `src/desktopBoot.ts` |
| Release context | `docs/release/GO_NO_GO_REPORT.md`, `docs/release/PHASE4_EVIDENCE.md` |

---

## 2. Methodology

### 2.1 What each harness measures

**`tools/startup-timing.mjs`** launches the instrumented release binary four times per run —
`fresh-cdp`, `fresh-nocdp`, `warm-cdp`, `warm-nocdp` — each in its own `mkdtemp` folder with its
own `state.sqlite` and its own `webview/EBWebView` profile. With
`SWIFTFETCH_STARTUP_TRACE=1`, `startup_trace()` in `lib.rs` writes phase labels plus elapsed
milliseconds from process start to stderr; the harness stores those lines verbatim in
`timing.json`. Render detection is attempted but **failed in every mode** (`rendered_ms` is
`null` in all 8 records): the CDP branch polls for a `.topbar` DOM node, and the no-CDP branch
polls native UIA for an element named `Add download`. Neither condition was ever observed
within the 60 s harness timeout. Consequently `timing.json` is a **Rust setup-phase trace
only** — it contains no first-paint or first-interactive measurement.

**`tools/startup-probe.mjs`** performs 12 independent cold launches (one fresh folder and fresh
profile per attempt). It polls UIA for the `Add download` element at ~400 ms intervals with an
8 s PowerShell timeout per poll and a 60 s overall cap, then records wall-clock milliseconds
from spawn to a `RENDERED` state. Both supplied runs are clean 12/12 passes.

### 2.2 Phase definitions used in this audit

Deltas are computed from the absolute `elapsed_ms` values, not from the labels' prose:

| Derived interval | Calculation |
|---|---|
| Pre-window / SQLite open | `store_opened − setup_enter` |
| WebView2 window build | `webview_window_built − engine_built` |
| Post-window app work | `engine_started − webview_window_built` |
| Rust setup total | `engine_started − process_start` |
| Full cold render | `results.json[].millis` (spawn → UIA `RENDERED`) |

`setup_enter − process_start` is 13–50 ms across all 8 traces (lock acquisition + update-journal
reconciliation only). `settings_and_http_ready`, `engine_built`, `tray_ready` are 1–2 ms apart.

### 2.3 Measurement error

UIA polling contributes at most one poll interval plus one PowerShell invocation (~0.4 s +
typically <0.6 s), i.e. **bounded at roughly 1 s** on a ~23–37 s measurement. That is ~2–4%
of the value and cannot explain the gap under investigation. CDP-vs-noCDP window-build means
are 2.96 s vs 2.80 s — no material harness effect.

---

## 3. Measured data

### 3.1 Rust setup phases, all 8 mode traces

| Run | Mode | SQLite open | Window build | Post-window | Rust setup total | `rendered_ms` |
|---|---|---:|---:|---:|---:|---|
| timing-m7opgI | fresh-cdp | 1,412 ms | 1,595 ms | 15 ms | 3,038 ms | `null` |
| timing-m7opgI | fresh-nocdp | 1,120 ms | 3,400 ms | 55 ms | 4,611 ms | `null` |
| timing-m7opgI | warm-cdp | 1,412 ms | 3,654 ms | 26 ms | 5,144 ms | `null` |
| timing-m7opgI | warm-nocdp | 1,167 ms | 2,026 ms | 39 ms | 3,252 ms | `null` |
| timing-Mblblm | fresh-cdp | 3,011 ms | 3,080 ms | 14 ms | 6,129 ms | `null` |
| timing-Mblblm | fresh-nocdp | 5,981 ms | 2,635 ms | 15 ms | 8,644 ms | `null` |
| timing-Mblblm | warm-cdp | 1,031 ms | 3,496 ms | 29 ms | 4,596 ms | `null` |
| timing-Mblblm | warm-nocdp | 1,228 ms | 3,123 ms | 42 ms | 4,423 ms | `null` |

### 3.2 Min / median / max (n = 8)

| Interval | Min | Median | Max | Mean |
|---|---:|---:|---:|---:|
| SQLite open (`store_opened`) | 1,031 ms | 1,320 ms | 5,981 ms | ~1,920 ms |
| WebView2 window build | 1,595 ms | 3,102 ms | 3,654 ms | ~2,876 ms |
| Post-window app work | 14 ms | 27.5 ms | 55 ms | ~29 ms |
| Rust setup total | 3,038 ms | 4,604 ms | 8,644 ms | ~4,980 ms |

The published characterization in `GO_NO_GO_REPORT.md` ("~4.4–8.6 s; SQLite 1–6 s;
window build 2.5–3.1 s; all others <40 ms") matches the upper end and the component ranges.
Direct recomputation from `timing.json` gives a **low end of 3.0 s** (the `fresh-cdp` trace in
`timing-m7opgI`) and window-build deltas of 1.6–3.7 s. Both characterizations agree on the
conclusion; the numbers above are the measured ones.

### 3.3 Cold vs warm, fresh vs reused profile — an important caveat

The harness labels four modes "fresh" and four "warm", but **`startup-timing.mjs` creates a new
folder, a new `state.sqlite` and a new `webview/EBWebView` profile for every mode** (one
`mkdtemp` folder per mode). Filesystem inspection confirms all four mode folders in each run
have independently created databases (created ~60 s apart, sequentially). Therefore:

- The "warm" label is **not backed by actual profile reuse**. Any difference between the
  "fresh" and "warm" groups is most plausibly OS page-cache warmth / run ordering, not a
  genuinely reused WebView2 profile.
- Within that limit, the observed SQLite-open medians are **cold-labeled 2,211 ms** vs
  **warm-labeled 1,198 ms** (max cold 5,981 ms). This is a real second-order effect: first-touch
  of the DB and its first-run schema/default-catalog initialization costs roughly ~1 s more.
- **True warm-start latency (relaunch with an existing profile and warm cache) is not measured
  by any supplied artifact.** `startup-probe.mjs` also creates a fresh folder per attempt, so
  all 24 full-render samples are cold-process/cold-profile.

### 3.4 Full cold render (spawn → UIA `RENDERED`)

| Run | n | Min | Median | Max | Mean |
|---|---:|---:|---:|---:|---:|
| `run-TDPTEn` | 12 | 26,574 ms | 32,225 ms | 37,438 ms | 31,970 ms |
| `run-fiWmSE` | 12 | 23,265 ms | 30,286.5 ms | 36,981 ms | 29,987 ms |
| Combined | 24 | 23,265 ms | 31,504 ms | 37,438 ms | 30,979 ms |

All 24 attempts report `rendered: true`, `state: "RENDERED"`. This is the figure the release
docs round to "~26–49 s"; the supplied clean runs span **23–37 s**. The 49 s figure is not
present in these two files and is presumably from an earlier/other run — it is not reproduced
here, so this audit uses 23–37 s as the evidence-backed range.

### 3.5 Historical failed runs (context, not latency measurements)

| Run | Final state | Wall clock | Reading |
|---|---|---:|---|
| `run-mpvnRe` | `about:blank`, `ready=complete`, CORS errors | 25,308 ms | Distinct blank-page defect |
| `run-Z4orbO` | same | 25,211 ms | Distinct blank-page defect |
| `run-gg5ogA` | `WINDOW_ONLY` | 30,648 ms | Window created, React never mounted in 60 s |
| `run-EsmcG1` | `NO_WINDOW` | 60,297 ms | Window never created in 60 s |

The `about:blank` runs are a separate correctness bug (eager navigation before adapters were
ready). `lib.rs` now explicitly defers window creation until `assets.setup()` and adapters are
ready and publishes native state first (comments at lines 317–330). That defect should be
tracked separately from the latency question. Notably its ~25 s failure cost is the same order
as a successful cold render, consistent with the dominant delay being WebView2-side and
independent of whether the page ultimately renders.

---

## 4. Hypothesis evaluation

| Hypothesis | Verdict | Evidence |
|---|---|---|
| **H1 — WebView2 first paint / React mount after window creation is the dominant cost** | **Best supported** | Rust setup ends at 3.0–8.6 s; full interactive render is 23–37 s. The entire 15–30 s gap lies **after** `webview_window_built`. Window build itself is 1.6–3.7 s and post-window app work is ≤55 ms. First-run profile dirs contain GPU/GrShader/Dawn caches and extension stores consistent with heavy WebView2 initialization. Confidence: **high** that cost is post-window; **medium** on the exact sub-cause because no paint marks were captured. |
| **H2 — CDP blank-page / eager navigation is the cause of the long delay** | **Rejected as the latency cause; separate defect** | `about:blank` runs never render at all; the current code has an explicit ordering fix. CDP attachment does not change window-build timing materially (2.96 s vs 2.80 s mean). Do not fold this into the latency estimate. |
| **H3 — Harness UIA polling overhead inflates the number** | **Ruled out** | ~400 ms poll + <~600 ms PowerShell exec ⇒ ≤~1 s bound on a 23–37 s signal (~2–4%). |
| **H4 — Application pre-window work is the primary cause** | **Ruled out as primary** | Everything before `webview_window_built` totals ≤~6 s (worst SQLite case 5,981 ms; all other phases 1–2 ms). SQLite open is a real, bounded second-order contributor that explains some cold-vs-warm variance; it cannot account for the 20+ s gap. |

---

## 5. First painted frame vs first interactive; hot vs cold

- **`webview_window_built` is not a paint.** It records that the native WebView window object
  was constructed, not that a pixel was drawn.
- **The UIA `RENDERED` signal is closer to first *interactive*.** It requires the `Add download`
  element to exist in the accessibility tree, i.e. React has mounted and the DOM is populated.
  A partially painted or blank frame would not satisfy it.
- **First Contentful Paint was never measured.** No `performance.getEntriesByType('paint')`,
  `PerformanceObserver`, or CDP `Page` lifecycle event appears in any harness or artifact. The
  interval between window construction (~1.6–3.7 s) and interactivity (23–37 s) is therefore
  **uncharacterized** — H1 is an inference from the gap, not a direct observation.
- **"Cold" here means cold process + cold profile + cold OS cache.** All 24 full-render samples
  satisfy that. There is no hot-process/hot-profile dataset; the "warm" label in `timing.json`
  is nominal only (§3.3). Any statement about hot-relaunch latency would be unsupported.

---

## 6. Instrumentation still missing

1. **Frontend performance marks exist but were never captured.** `src/desktopBoot.ts` emits
   `performance.mark("swiftfetch:frontend-ready")` (line 13) and
   `performance.mark("swiftfetch:health-acknowledged")` (line 15). They are intended "to be read
   by the startup probe," but `startup-timing.mjs` only runs `Runtime.evaluate` for a `.topbar`
   selector and never reads `performance` entries; `startup-probe.mjs` uses UIA only. As a result
   **no mark from `desktopBoot.ts` appears anywhere in `timing.json` or `results.json`**, and
   `rendered_ms` is `null` for all 8 modes. The instrumentation was added but the measurement
   path was not wired to it.
2. No first-paint metric (FCP / first contentful frame) from the WebView2 renderer.
3. No sub-phase breakdown *inside* WebView2 initialization (runtime creation, GPU process,
   navigation, HTML parse, JS bundle evaluation, React mount) — so H1 cannot be narrows further.
4. No genuine reused-profile / warm-relaunch dataset (see §3.3).
5. No capture of the 42 ms / 55 ms post-window outliers or their cause (low value given magnitude).
6. The 49 s upper figure cited in release docs is not present in the two supplied result files.

---

## 7. Honest conclusion on the <1 s target

The evidence does **not** support reaching the <1 s startup target with the current WebView2
architecture, and this lane makes no claim that it can:

- Even the *earliest structural milestone* — native WebView window built — is **≥3.0 s** in the
  best trace and ~3.1 s at the median. The target is already exceeded before any page content is
  guaranteed to exist.
- The measured time to a usable, interactive UI is **23–37 s**, with the dominant ~15–30 s
  occurring inside WebView2 first paint / React mount (H1). No application-side phase measured
  here exceeds 6 s.
- Closing a >20 s WebView2-side gap within a <1 s envelope is not a tuning problem; it would
  require an architectural change (e.g. a persistent/pre-warmed WebView2 process or a non-WebView
  shell) plus measurement the project does not currently have.

Per `GO_NO_GO_REPORT.md`, "Startup <1 s target" is an explicit **Not met** gate item and
"startup target accepted or fixed" is a stated condition to flip to GO. The decision to accept a
revised target or to fund an architectural change is an **owner decision**, not something this
analysis can settle. This report recommends no code change and promises no speedup; candidate
options and their risks are enumerated in `STARTUP_OPTIMIZATION_REPORT.md` for the parent to
evaluate and implement with regression tests.
