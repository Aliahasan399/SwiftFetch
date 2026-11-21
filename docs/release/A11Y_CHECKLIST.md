# SwiftFetch — Accessibility Release Checklist (Phase 5)

**Created:** 2026-11-21. **Version:** 0.1.0. **Scope:** accessibility items only.
**Companion checklists:** `LEGAL_PRIVACY_CHECKLIST.md`, `STABILITY_CHECKLIST.md`; gate context in
`RELEASE_PLAN.md` (P2: SR/contrast report, translation decision) and `RELEASE_GAP_REGISTER.md`.

Status legend:

- **Ready** — implemented with recorded evidence; no further work known.
- **Partial** — implemented/drafted, but review, missing pieces, or limits remain.
- **Externally blocked** — requires an owner decision or external resource.
- **Not verified** — no evidence found; do not claim.

This checklist does **not** claim WCAG conformance, VPAT/ACR, or accessibility certification.
Existing evidence is source-level regression plus agent-driven desktop QA; no independent
human or assistive-technology acceptance exists (`VERIFICATION.md:58,222-224`).

## 1. Baseline & target

| # | Item | STATUS | Evidence | Required action |
| --- | --- | --- | --- | --- |
| 1.1 | WCAG 2.1 AA stated as the design baseline | Not verified | Repository target is architecture §2.1 row 13 (`RELEASE_GAP_REGISTER.md:47`); no audit artifact exists | Run a WCAG 2.1 AA evaluation (manual + tooling) on the packaged UI |
| 1.2 | Keyboard-only operation of all flows | Partial | `verify.mjs:358-361` locks `:focus-visible` outline, `.skip-link:focus` reveal; Ctrl+N opens Add download in the manual pass (`VERIFICATION.md:54-56`) | Keyboard-only walkthrough of queue/history/scheduler/settings/diagnostics |
| 1.3 | Focus management in dialogs | Ready | `src/components/Dialog.tsx` locks `showModal()`, `aria-modal="true"`, `aria-labelledby`, `aria-describedby`, `onCancel` Tab trap, `data-autofocus`, `previous.focus()`, main-content focus, `element.close()` (`src/testing/verify.mjs:348-356`); keyboard pass exercised Ctrl+N form (`VERIFICATION.md:54-56`) | Retain source-level regression on every UI change |
| 1.4 | Skip link and focus return to trigger | Ready | `app.skip` key (`src/i18n.tsx:41,117`); `getElementById("main-content")?.focus()` and `previous.focus()` locked (`verify.mjs:350`) | None known |

## 2. Status, structure, and announcements

| # | Item | STATUS | Evidence | Required action |
| --- | --- | --- | --- | --- |
| 2.1 | Progress exposed as accessible text with polite live region | Ready | `aria-live="polite"` progress announcement locked (`verify.mjs:388`); `App.tsx` progress region | Verify announced cadence with a real screen reader (3.1) |
| 2.2 | Errors announced assertively | Ready | `role="alert"` feedback region locked (`verify.mjs:393`) | None known |
| 2.3 | Colour never the only state indicator | Partial | `StatusPill` text labels + status text keys (`src/i18n.tsx:53-62`); no contrast measurement exists | Visual check that status is not conveyed by colour alone; measure text/background contrast (3.2) |
| 2.4 | Labelled icon-only buttons | Ready | Translated aria-labels on icon-only buttons locked (`verify.mjs:346,375`; `dialog.closeDialog` key `src/i18n.tsx:84,159`) | None known |
| 2.5 | Scrollable table regions captioned | Ready | `role="region"` + `aria-label` locked per scrollable table (`verify.mjs:381-382`) | None known |
| 2.6 | Visible labels/text on controls in the packaged app | Partial | Smoke label check passed (`VERIFICATION.md:71,261-263`, `run-zIBKJy`) | Extend to every view in a human pass |

## 3. Items requiring real assistive technology / human review

| # | Item | STATUS | Evidence | Required action |
| --- | --- | --- | --- | --- |
| 3.1 | Real screen-reader test (NVDA/JAWS) — Windows target | Not verified | `VERIFICATION.md:222-224` ("Screen-reader cadence and contrast still require real SR/visual verification"); `RELEASE_GAP_REGISTER.md:58` (no SR artifact) | [OWNER: screen-reader hardware/test personnel] — externally blocked (`RELEASE_PLAN.md:115`) |
| 3.2 | Contrast measurement against WCAG 2.1 AA ratios | Not verified | Dark theme default (`CONTRACT.md:305`); no measured ratios in any artifact | Measure normal/large text and UI-component contrast; record ratios per surface |
| 3.3 | Reduced-motion behavior | Not verified | No reduced-motion evidence found in the repository scan | Confirm whether animation exists; if added, honour `prefers-reduced-motion` |
| 3.4 | Independent human acceptance / certification | Not verified | Agent-driven desktop QA only (`VERIFICATION.md:56-58`) | [OWNER: acceptance personnel]; do not publish certification claims |

## 4. Localization review (accessibility-adjacent, same phase)

| # | Item | STATUS | Evidence | Required action |
| --- | --- | --- | --- | --- |
| 4.1 | Typed en/bn dictionaries with full key parity (72/72) | Ready | `src/i18n.tsx:21-171`; parity locked by regression (`VERIFICATION.md:218-220`); translated a11y labels checked (`verify.mjs:344-346`) | None known |
| 4.2 | Document language and direction set per locale | Ready | `applyDocumentLocale` sets `lang` and `dir="ltr"` (`src/i18n.tsx:15-19`) | None known |
| 4.3 | English fallback for untranslated text | Ready | `translate` fallback to `en` table (`src/i18n.tsx:173-176`); header comment forbids describing the product as fully localized (`src/i18n.tsx:1-5`) | None known |
| 4.4 | Full UI translation | Partial | "much view copy remains hard-coded English" (`VERIFICATION.md:223-224`); `RELEASE-READINESS.md:70-72` | [OWNER: translation decision]; complete view copy or state English-only scope |
| 4.5 | Native-speaker review of Bengali copy | Not verified | No linguistic review artifact | [OWNER: bn reviewer] |

## 5. Sign-off

- [ ] WCAG 2.1 AA evaluation executed on the packaged build (1.1)
- [ ] Keyboard-only walkthrough of all views recorded (1.2)
- [ ] [OWNER: SR personnel] NVDA/JAWS pass with cadence notes (3.1)
- [ ] Contrast ratios measured and recorded per surface (3.2, 2.3)
- [ ] [OWNER: translation decision] + [OWNER: bn reviewer] (4.4-4.5)

Blocking summary: public release is **NO-GO** while items 3.1-3.4 and 4.4-4.5 are open
(`RELEASE_PLAN.md:97-99` records these as P2 gates; `RELEASE-READINESS.md:70-72` concedes the
product "is not fully translated or independently certified for accessibility").