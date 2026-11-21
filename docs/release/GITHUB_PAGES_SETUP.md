# SwiftFetch — GitHub Pages configuration and `docs/site` review (version 0.1.0)

**Status: NOT PUBLISHED / REQUIRES OWNER APPROVAL.** This is a procedure and verification
document only. It publishes nothing, creates no workflow file, changes no repository settings,
creates no git tag or release, and invents no owner name, repository URL, contact, certificate,
or domain. Every value the owner must supply is left as `[OWNER: …]` for the owner to fill in
locally.

Scope of this lane (github-pages-verifier): **read-only** inspection of `docs/site/**` (7 HTML
pages + `assets/style.css`), plus preparation of a GitHub Pages configuration that uses
`docs/site/` as the publishing directory. No edits were made under `docs/site/**`; no source or
config files were changed; no commit was created; nothing was published.

Owner decisions used only for framing (from the parent lane, not invented here): individual /
personal publisher, **free download**, **zero budget**, and proposed license
**GPL-3.0-or-later** (explicitly *not final*).

---

## 1. Findings from `docs/site/` inspection

### 1.1 Inventory inspected

| File | Present |
| --- | --- |
| `docs/site/index.html` | yes |
| `docs/site/features.html` | yes |
| `docs/site/download.html` | yes |
| `docs/site/documentation.html` | yes |
| `docs/site/privacy.html` | yes |
| `docs/site/license.html` | yes |
| `docs/site/support.html` | yes |
| `docs/site/assets/style.css` | yes |

### 1.2 Navigation links — PASS

All 7 pages carry an identical `<nav class="site-nav" aria-label="Main navigation">` with the same
7 links: `index.html`, `features.html`, `download.html`, `documentation.html`, `privacy.html`,
`license.html`, `support.html`. Every target file exists. There are no dead nav targets.

`aria-current="page"` is present on exactly the correct item on each page (index→Home,
features→Features, download→Download, documentation→Documentation, privacy→Privacy,
license→License, support→Support). The brand link on every page points to `index.html`.

### 1.3 Internal links and relative paths — PASS

Every `href` and `src` in the site is either a same-directory relative filename (e.g.
`features.html`, `assets/style.css`) or the in-page fragment `#main`. There are no absolute
paths, no `../` traversals, no root-anchored `/…` links, no `http`/`https`, no `www.`, no
`mailto:`, and no `.zip`/`.msi`/`.exe` download URLs anywhere in `docs/site/**`. All `#main`
targets resolve to `<main id="main">` on the corresponding page. Because links are relative and
not root-anchored, the site is safe to publish under a project subpath such as
`https://[OWNER].github.io/[REPO]/`.

Note (not a defect): `documentation.html` lists canonical repository document paths
(`README.md`, `ARCHITECTURE.md`, `docs/PRIVACY.md`, etc.) as plain `<code>` text, not clickable
links, so there is nothing there to break. The repository link itself is the placeholder
`[OWNER: source repository URL]`.

### 1.4 Mobile responsiveness (CSS) — PASS, two watch items

All 7 pages include `<meta name="viewport" content="width=device-width, initial-scale=1">`.
`assets/style.css` is dependency-free (no CDN, no external fonts, no network assets) and includes
responsive rules: `.container` uses `max-width: 60rem` with horizontal padding; `.header-inner`
and `.site-nav ul` use `flex-wrap: wrap`; `.feature-grid` uses
`repeat(auto-fit, minmax(15rem, 1fr))`; `p, li` are capped at `68ch`; `code` uses
`overflow-wrap: anywhere`; and an `@media (max-width: 40rem)` rule reduces `h1` size.

Watch items (not current breakage):
- Tables use `width:100%; max-width:100%` but have no `overflow-x:auto` wrapper. All current cell
  content (placeholders and prose) wraps, so there is no present overflow, but a long unbroken
  URL substituted for a placeholder later could overflow on a narrow viewport.
- `html { scroll-behavior: smooth; }` does not honour `prefers-reduced-motion`. This is a minor
  progressive-enhancement nicety, not a release blocker.

### 1.5 Accessibility — PASS, minor notes

Present on all 7 pages: `<html lang="en">`, `<meta charset="utf-8">`, a keyboard skip link
(`<a class="skip-link" href="#main">`) whose CSS reveals it on `:focus`, `:focus-visible`
outlines for links/buttons, landmark elements (`header`, `nav`, `main`, `aside`, `footer`) with
`aria-label` on `nav` and `aside`, one `h1` per page followed by `h2`/`h3` sections, and
`<th scope="col">` on all tables. There are no images, so no `alt`-text concerns.

Minor notes (not defects): `support.html` goes from `h1` directly to `h2` without a lead
paragraph; the `aside` aria-labels vary (`Release status`, `Notice`, `License status`) but are
meaningful in context. No ARIA role misuse was found. A full automated axe audit was not run in
this lane; the above is a static code review.

### 1.6 Download links — PASS (placeholders only)

`download.html` lists four release items (NSIS installer, MSI installer, source snapshot,
SHA-256 checksums), each rendered as `<span class="placeholder">[OWNER: GitHub Releases URL]</span>`
with explanatory text that they are not active download links. There are **zero** `<a href>`
download links and **no fabricated or fake download URLs** anywhere on the site. The page states
that no direct installer download link is published because the candidates are unsigned.

### 1.7 Legal wording — PASS

Consistent release-candidate caveats are present: unsigned preview candidates, "not a safe public
release yet", signing/licensing/publisher identity pending owner decisions, and no certification
claimed. Permitted-use language (download only content you have permission for; respect
copyright, authentication, DRM, paywalls, site policies and rate limits; no circumvention claim)
appears on `index.html`, `documentation.html` and `license.html`.

### 1.8 Privacy claims vs `docs/PRIVACY.md` — PASS, one reconciliation note

`privacy.html` is substantively consistent with `docs/PRIVACY.md`: local storage of queue,
settings, filenames, destination paths, URLs and request metadata in `%APPDATA%\swiftfetch`; a
URL/header can contain private tokens so backups must be private; network requests only to
sources, proxies and policy-accepted redirects; publisher-configured update check only when
requested; no account/telemetry required; clipboard off by default with confirmation form and
explicit Paste URL button; extension via Native Messaging after user action with no cookies or
credentials; retention removes metadata not completed files; shareable inventory excludes URLs
and headers but includes filenames and local paths; full SQLite backup is private and not a
default support attachment; owner must identify publisher/privacy contact/procedures before
publication; no regulatory certification claimed.

Reconciliation note (not a contradiction): `privacy.html` adds details not present in the
`PRIVACY.md` draft — the three environment-variable overrides (`SWIFTFETCH_DATA_DIR`,
`SWIFTFETCH_DB_PATH`, `SWIFTFETCH_DOWNLOADS_DIR`) and the claim that the extension has no broad
host permissions and logs only scheme/host with paths and queries stripped. The owner should
reconcile these additions into the canonical `PRIVACY.md` (or confirm them) before publication.

### 1.9 License wording — PASS

`license.html` is correctly conditional: **GPL-3.0-or-later is proposed**, explicitly *pending a
dependency compatibility audit*, *not final*, and "no open-source license has been assigned on
the owner's behalf yet." It records 338 dependencies inventoried, 7 package roots lacking
license text, and that `docs/TERMS-DRAFT.md` is a product-use draft, not a software license. The
free download is described as a price decision only, not a license selection. No license is
asserted as final.

### 1.10 Contact placeholders — PASS

All contact points are placeholders, with no invented details: `support.html` has four
(`[OWNER: issue-reporting URL]`, `[OWNER: support email]`, `[OWNER: private security contact]`,
`[OWNER: supported versions and response expectations]`); `privacy.html` has
`[OWNER: privacy contact]`; `documentation.html` has `[OWNER: source repository URL]`.

### 1.11 Broken links / issues summary

- **Broken links found: none.** All 7 internal links per page, the brand links, the skip links,
  the stylesheet reference, and all in-body links resolve to existing files/fragments.
- **Fake/active download links found: none.**
- Non-blocking items to note before publication: reconcile the extra privacy details into
  `PRIVACY.md` (§1.8); consider a table `overflow-x` wrapper for future long URLs (§1.4);
  consider a `prefers-reduced-motion` guard for smooth scrolling (§1.4); replace every
  `[OWNER: …]` placeholder; and run an automated axe/contrast audit as a final gate.

---

## 2. Why "Settings → Pages → Deploy from branch → main → /docs" is wrong

GitHub's classic **Deploy from a branch** Pages source only lets you pick a branch and either the
repository root `/` or the single `docs/` folder. It **cannot** point at a nested folder such as
`docs/site/`. Selecting `main` + `/docs` would therefore publish the entire `docs/` tree at the
site root: the real homepage would end up at `/site/index.html`, not at `/`, and the stylesheet
would be at `/site/assets/style.css`. Because every link in the site is a same-directory relative
path (e.g. `index.html`, `assets/style.css`), the top-level `docs/` directory contains no
`index.html` for `/` and the published site would be broken or show a directory listing of
Markdown files. Do not use the branch source with `/docs` for this layout.

---

## 3. Owner decision: two ways to publish `docs/site`

**Option A — GitHub Actions Pages workflow (recommended).** Keep the repository layout exactly as
it is. A Pages workflow uploads `docs/site/` as the site artifact, so the *contents* of
`docs/site/` become the site root (`index.html` at `/`, `assets/style.css` at `/assets/`). No
files are moved, the release docs under `docs/` stay out of the published site, and relative
links resolve correctly under a project subpath. This requires setting the Pages source to
**GitHub Actions**. This is the option detailed in §4–§7.

**Option B — move the site to the root `docs/` folder (owner decision).** Copy the contents of
`docs/site/` (the 7 HTML files and the `assets/` folder) into `docs/` so that `docs/index.html`
and `docs/assets/style.css` exist alongside the existing Markdown docs, then use
**Settings → Pages → Deploy from branch → main → /docs**. This works with the classic source but
changes the repository layout, mixes generated site files with prose docs, requires deleting or
relocating duplicates, and risks publishing stray Markdown files as part of the site. The owner
must explicitly choose this if they prefer the no-workflow route; this document does not perform
the move.

---

## 4. Exact recommended workflow YAML (Option A)

The owner should add this file at **`.github/workflows/deploy-pages.yml`**. This document does
**not** create it. Actions are pinned to their current major-version tags (verified against the
official release pages on 2026-11-21): `actions/checkout@v7`, `actions/configure-pages@v6`,
`actions/upload-pages-artifact@v5`, and `actions/deploy-pages@v5`. Re-check the major tags at
implementation time and prefer a full commit-SHA pin for a supply-chain-hardened repository.

Sources for current majors:
- `actions/checkout` releases — https://github.com/actions/checkout/releases (v7.0.1 is latest)
- `actions/configure-pages` releases — https://github.com/actions/configure-pages/releases (v6.0.0 is latest)
- `actions/upload-pages-artifact` releases — https://github.com/actions/upload-pages-artifact/releases (v5.0.0 is latest)
- `actions/deploy-pages` releases — https://github.com/actions/deploy-pages/releases (v5.0.1 is latest)

```yaml
name: Deploy static content to Pages

on:
  push:
    branches: ["main"]
  workflow_dispatch:

# Least privilege: exactly the three scopes Pages deployment needs.
permissions:
  contents: read
  pages: write
  id-token: write

# Allow one concurrent deployment; do not cancel an in-progress deploy.
concurrency:
  group: "pages"
  cancel-in-progress: false

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - name: Checkout
        uses: actions/checkout@v7
      - name: Configure Pages
        uses: actions/configure-pages@v6
      - name: Upload Pages artifact
        uses: actions/upload-pages-artifact@v5
        with:
          path: ./docs/site

  deploy:
    environment:
      name: github-pages
      url: ${{ steps.deployment.outputs.page_url }}
    runs-on: ubuntu-latest
    needs: build
    steps:
      - name: Deploy to GitHub Pages
        id: deployment
        uses: actions/deploy-pages@v5
```

Notes on the YAML:
- `permissions` is set to the least-privilege set requested: `contents: read`, `pages: write`,
  `id-token: write`. If desired, these can be scoped per job instead of at workflow level
  (`build` needs `contents: read` and `pages: write`; `deploy` needs `pages: write` and
  `id-token: write`). Do **not** add `contents: write`.
- `path: ./docs/site` is the critical line — it makes the *contents* of `docs/site/` the site
  root. Do not change it to `./docs`.
- `workflow_dispatch` lets the owner redeploy manually after content changes without a new push.
- The `github-pages` environment (with an optional required reviewer) is the standard Pages
  deployment environment; using it gives the owner a natural approval gate.
- `upload-pages-artifact` v4+ excludes hidden dotfiles by default. This site has no hidden files
  that need publishing, and Pages deployments from an Actions artifact do not run Jekyll, so no
  `.nojekyll` file is required. If the owner later adds a `CNAME` (§7), it is a normal file and is
  included automatically; if any future hidden file must be shipped, set
  `include-hidden-files: true` on the upload step (supported by v5).
- Compatibility: `upload-pages-artifact` v3+ requires `deploy-pages` v4+; v5 + v5 is valid.

---

## 5. Manual setup steps for the owner (Option A)

1. Confirm the project is hosted on GitHub as a repository the owner controls and that GitHub
   Pages is available for it (public repositories get Pages at no cost; compatible with the
   recorded **zero-budget** decision).
2. Add `.github/workflows/deploy-pages.yml` with the exact YAML from §4. (This document does not
   create it.)
3. In the repository, go to **Settings → Pages → Build and deployment**.
   - Set **Source** to **GitHub Actions**. (Do not choose "Deploy from a branch"; see §2.)
4. Optionally, under **Settings → Environments → github-pages**, add a required reviewer so a
   deploy needs explicit owner approval. This reinforces the "REQUIRES OWNER APPROVAL" status.
5. Ensure the branch named in the `on.push.branches` filter (`main`) matches the repository's
   default branch; edit the filter if the owner uses a different default branch.
6. Commit and push the workflow file to `main`. The workflow runs automatically on push and can
   also be started from the **Actions** tab via **Run workflow**.
7. Before treating the site as public, complete the content gates in §6 and replace every
   `[OWNER: …]` placeholder (download URLs, privacy contact, source repository URL, and the four
   support contacts). Publishing the site while placeholders are live is an owner decision, not
   something this document authorizes.

---

## 6. Verification steps (owner, after the first workflow run)

1. **Workflow health:** open the **Actions** tab and confirm the latest
   "Deploy static content to Pages" run completed with both `build` and `deploy` green. If
   `configure-pages` fails, re-check the `pages: write` permission and that Pages is enabled.
2. **Pages URL:** in **Settings → Pages**, confirm the banner reads "Your site is live at
   `https://[OWNER].github.io/[REPO]/`" (or the configured custom domain).
3. **Homepage and assets:** open the live URL. Confirm the styled homepage renders (which proves
   `assets/style.css` resolved) and that the release-status notice and headings appear.
4. **All 7 pages:** from the homepage, click every navigation item and confirm each page loads
   with styling and that the active item is underlined (`aria-current` styling). Verify the
   brand link returns to the homepage and the skip link jumps to main content on keyboard focus.
5. **No 404s:** open browser DevTools → Network on each page and confirm no failed requests
   (all `*.html` and `assets/style.css` return 200). Re-check after the owner replaces
   placeholders with real URLs.
6. **Mobile layout:** use DevTools device toolbar at a narrow width (e.g. 375 px) and confirm the
   header/nav wrap cleanly and the feature cards reflow without horizontal scrolling. Revisit the
   two watch items in §1.4 if a long URL is added later.
7. **Content accuracy:** confirm the published privacy and license wording still matches
   `docs/PRIVACY.md` and the proposed/conditional license framing after any owner edits.
8. **Redeploy test:** make a trivial content change, push to `main`, and confirm the workflow
   redeploys automatically; or trigger **Run workflow** manually.
9. **Record evidence:** note the live URL, the date, the committing owner, and the workflow run
   number for the release audit trail.

---

## 7. Custom-domain note

If the owner wants a custom domain (left as `[OWNER: your-domain.example]` here — no domain is
invented): with Option A, add a `CNAME` file containing only the domain name to **`docs/site/`**
(i.e. `docs/site/CNAME`) so it is included in the uploaded artifact; then, in
**Settings → Pages → Custom domain**, enter the same domain and enable **Enforce HTTPS** after
DNS propagates. Configure DNS at the registrar (an `A` record to the GitHub Pages IP addresses or
a `CNAME` to `[OWNER].github.io`, depending on apex vs subdomain). Do not add the domain here or
to any source file without the owner's decision. With Option B, the `CNAME` would instead go at
the root of `docs/`.

---

## 8. Current state (honest status)

- **Status: NOT PUBLISHED / REQUIRES OWNER APPROVAL.** No Pages site has been enabled, no
  workflow file has been created, no branch setting has been changed, and no domain has been
  configured by this lane or this document.
- `docs/site/**` was inspected read-only and found internally consistent with no broken links,
  no fake download links, correct accessibility scaffolding, and correct proposed/conditional
  legal wording (§1).
- The recommended publishing method is the GitHub Actions workflow in §4, using
  `path: ./docs/site`; the classic branch source cannot target `docs/site` (§2).
- Every owner-supplied value remains a `[OWNER: …]` placeholder; no identity, contact,
  repository URL, certificate, or domain was invented.
