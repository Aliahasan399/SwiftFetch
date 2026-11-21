#!/usr/bin/env sh
# Run the SwiftFetch browser-extension protocol tests.
#
# Usage:
#   bash browser-extension/test/run_tests.sh
# or directly:
#   node --test browser-extension/test/test_protocol.js
set -e
cd "$(dirname "$0")/.."
exec node --test test/test_protocol.js
