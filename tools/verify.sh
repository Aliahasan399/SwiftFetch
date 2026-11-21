#!/usr/bin/env bash
# SwiftFetch verification pipeline.
#
# Runs every check the project has and prints a summary. Exits non-zero if any
# step fails. Safe to re-run; creates no files outside the project.
#
# Usage:  bash tools/verify.sh

set -u

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT" || exit 1

FAILURES=0

banner() { printf '\n\033[1m==> %s\033[0m\n' "$1"; }
ok()     { printf '  \033[32mPASS\033[0m %s\n' "$1"; }
bad()    { printf '  \033[31mFAIL\033[0m %s\n' "$1"; FAILURES=$((FAILURES + 1)); }

banner "Fixture server self-test"
if python tools/fixtures/selftest.py > /tmp/swiftfetch-fixture.log 2>&1; then
  ok "fixture self-test ($(tail -n 2 /tmp/swiftfetch-fixture.log | head -n 1))"
else
  bad "fixture self-test"
  tail -n 25 /tmp/swiftfetch-fixture.log | sed 's/^/      /'
fi

banner "Rust: cargo check"
if (cd src-tauri && cargo check --all-targets > /tmp/swiftfetch-check.log 2>&1); then
  ok "cargo check"
else
  bad "cargo check"
  tail -n 40 /tmp/swiftfetch-check.log | sed 's/^/      /'
fi

banner "Rust: cargo test"
if (cd src-tauri && cargo test > /tmp/swiftfetch-test.log 2>&1); then
  ok "cargo test"
  grep -E '^test result:' /tmp/swiftfetch-test.log | sed 's/^/      /'
else
  bad "cargo test"
  tail -n 60 /tmp/swiftfetch-test.log | sed 's/^/      /'
fi

banner "Frontend: npm run build"
if npm run build > /tmp/swiftfetch-build.log 2>&1; then
  ok "npm run build"
else
  bad "npm run build"
  tail -n 40 /tmp/swiftfetch-build.log | sed 's/^/      /'
fi

banner "Frontend: regression suite"
if node src/testing/verify.mjs > /tmp/swiftfetch-regress.log 2>&1; then
  ok "$(grep -E 'Frontend regression checks' /tmp/swiftfetch-regress.log | tail -n 1)"
else
  bad "frontend regression suite"
  tail -n 25 /tmp/swiftfetch-regress.log | sed 's/^/      /'
fi

banner "Browser extension: protocol tests"
if node --test browser-extension/test/test_protocol.js > /tmp/swiftfetch-ext.log 2>&1; then
  ok "extension protocol tests ($(grep -E '^ℹ pass' /tmp/swiftfetch-ext.log | tail -n 1 | tr -d 'ℹ '))"
else
  bad "extension protocol tests"
  tail -n 25 /tmp/swiftfetch-ext.log | sed 's/^/      /'
fi

banner "Summary"
if [ "$FAILURES" -eq 0 ]; then
  printf '  All checks passed.\n'
  exit 0
else
  printf '  %d check(s) failed.\n' "$FAILURES"
  exit 1
fi
