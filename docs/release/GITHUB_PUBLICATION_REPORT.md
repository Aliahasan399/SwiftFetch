# GitHub Publication Report — Phase 3 Task 1

**Date:** 2026-11-21 · **Status:** COMPLETED

## Repository

| Field | Value |
|---|---|
| Repository URL | https://github.com/Aliahasan399/SwiftFetch |
| Account username | `Aliahasan399` (verified via `gh auth status`, keyring) |
| Visibility | PUBLIC |
| Default branch | `main` |
| Description | "Local-first Windows download manager with segmented HTTP/HTTPS transfers, durable queue, browser handoff, and signed-update support (unsigned release candidate)." |

Account verification: `gh auth status` confirmed the authenticated personal account
`Aliahasan399`, active, HTTPS protocol, token scopes `gist, read:org, repo, workflow`.
No password was requested or stored. `gh repo view SwiftFetch` confirmed the repo did
not exist before creation (no overwrite). Creation used
`gh repo create SwiftFetch --public --source=. --remote=origin --description ...`
with **no** second README/license/gitignore initialization.

## Git verification

| Check | Result |
|---|---|
| Branch | `main` |
| Commits before push | 9 (preserved; no history rewrite) |
| Working tree before push | clean |
| `git push -u origin main` | exit 0 |
| Local HEAD | `29325d6…` at push time (later Pages workflow commit `832212f…`) |
| Remote HEAD after push | matched local |
| Commit counts local/remote | matched |

## Security settings applied

| Setting | Status |
|---|---|
| Private vulnerability reporting | **ENABLED** (`PUT /repos/.../private-vulnerability-reporting`, exit 0) |
| Secret scanning | **ENABLED** (public repos; `security_and_analysis.secret_scanning.status = enabled`) |
| Secret scanning push protection | **ENABLED** (`status = enabled`) |
| Branch protection on `main` | **ENABLED** — force pushes and deletions disallowed |
| Dependabot alerts | **NOT ENABLED** — API returned 403, token lacks `admin:repo_hook` scope. Owner must enable in repo Settings or refresh token with that scope. |
| Two-factor authentication | **OWNER ACTION** — cannot be set via API; must be enabled on the GitHub account. |
| Actions permissions | default (read) retained; workflows use least-privilege `permissions` blocks. |
| Paid settings | none enabled. |

## Secret scan (pre-publication)
188 source/doc files scanned (excluding `node_modules`, `target`, `artifacts`, `.bak-*`):
0 API-key patterns, 0 private-key headers, 0 password literals, no `.env`/key files.
One personal path in `LICENSE_AUDIT.md` was sanitized before publication.

## Verification commands
```
git status                       # clean before push
git branch --show-current        # main
git rev-list --count HEAD        # 9 at push
git remote -v                    # origin https://github.com/Aliahasan399/SwiftFetch.git
git push -u origin main          # exit 0
gh repo view SwiftFetch --json nameWithOwner,visibility,url,defaultBranchRef
```
