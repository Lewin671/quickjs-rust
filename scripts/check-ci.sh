#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

# Mirrors the GitHub Actions check job. Use this when validating changes that
# can affect hosted-runner behavior, runtime test scheduling, or CI artifacts.
export RUST_TEST_THREADS="${RUST_TEST_THREADS:-1}"
export QJS_CHECK_SPLIT_RUNTIME_TESTS="${QJS_CHECK_SPLIT_RUNTIME_TESTS:-1}"
export QJS_CHECK_SKIP_TEST262="${QJS_CHECK_SKIP_TEST262:-1}"

exec "$ROOT_DIR/scripts/check.sh"
