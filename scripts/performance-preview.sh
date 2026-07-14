#!/usr/bin/env bash
set -Eeuo pipefail

HARNESS_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
export PYTHONDONTWRITEBYTECODE=1

usage() {
  cat <<'EOF'
Usage: ./scripts/performance-preview.sh \
  --harness-mode <base_owned_harness|main_push_head_owned_harness> \
  --candidate-source <path> --base-source <path> \
  --candidate-sha <full-sha> --base-sha <full-sha> \
  --candidate-repo <https-github-clone-url> \
  --base-repo <https-github-clone-url> [--output <directory>]
EOF
}

HARNESS_MODE=""
CANDIDATE_SOURCE=""
BASE_SOURCE=""
CANDIDATE_REVISION=""
BASE_REVISION=""
CANDIDATE_REPO=""
BASE_REPO=""
OUTPUT=""

while [ "$#" -gt 0 ]; do
  case "$1" in
    --harness-mode|--candidate-source|--base-source|--candidate-sha|--base-sha|--candidate-repo|--base-repo|--output)
      if [ "$#" -lt 2 ]; then
        echo "error: $1 requires a value" >&2
        exit 2
      fi
      ;;
  esac
  case "$1" in
    --harness-mode) HARNESS_MODE="$2"; shift 2 ;;
    --candidate-source) CANDIDATE_SOURCE="$2"; shift 2 ;;
    --base-source) BASE_SOURCE="$2"; shift 2 ;;
    --candidate-sha) CANDIDATE_REVISION="$2"; shift 2 ;;
    --base-sha) BASE_REVISION="$2"; shift 2 ;;
    --candidate-repo) CANDIDATE_REPO="$2"; shift 2 ;;
    --base-repo) BASE_REPO="$2"; shift 2 ;;
    --output) OUTPUT="$2"; shift 2 ;;
    -h|--help) usage; exit 0 ;;
    *) echo "error: unknown argument: $1" >&2; usage >&2; exit 2 ;;
  esac
done

for value_name in HARNESS_MODE CANDIDATE_SOURCE BASE_SOURCE CANDIDATE_REVISION BASE_REVISION CANDIDATE_REPO BASE_REPO OUTPUT; do
  if [ -z "${!value_name}" ]; then
    echo "error: missing required $value_name" >&2
    exit 2
  fi
done
case "$CANDIDATE_REVISION$BASE_REVISION" in
  *[!0-9a-f]*) invalid_revision=1 ;;
  *) invalid_revision=0 ;;
esac
if [ "$invalid_revision" -eq 1 ] || [ "${#CANDIDATE_REVISION}" -ne 40 ] \
  || [ "${#BASE_REVISION}" -ne 40 ]; then
  echo "error: candidate and base revisions must be full lowercase git SHAs" >&2
  exit 2
fi

canonical_path() {
  python3 -c 'import pathlib,sys; print(pathlib.Path(sys.argv[1]).expanduser().resolve())' "$1"
}
CANDIDATE_SOURCE="$(canonical_path "$CANDIDATE_SOURCE")"
BASE_SOURCE="$(canonical_path "$BASE_SOURCE")"
OUTPUT="$(canonical_path "$OUTPUT")"
case "$HARNESS_MODE" in
  base_owned_harness)
    [ "$HARNESS_ROOT" = "$BASE_SOURCE" ] || {
      echo "error: base-owned harness must execute from the base source" >&2; exit 2;
    }
    OUTPUT_OWNER="$BASE_SOURCE"
    ;;
  main_push_head_owned_harness)
    [ "$HARNESS_ROOT" = "$CANDIDATE_SOURCE" ] || {
      echo "error: main-push harness must execute from the candidate source" >&2; exit 2;
    }
    [ "$CANDIDATE_REPO" = "$BASE_REPO" ] || {
      echo "error: main-push candidate and base repositories must match" >&2; exit 2;
    }
    OUTPUT_OWNER="$CANDIDATE_SOURCE"
    ;;
  *) echo "error: invalid harness mode" >&2; exit 2 ;;
esac
case "$OUTPUT" in
  "$OUTPUT_OWNER"/target/*) ;;
  *) echo "error: --output must stay under the selected harness target directory" >&2; exit 2 ;;
esac
if [ -e "$OUTPUT" ]; then
  [ -d "$OUTPUT" ] || { echo "error: output exists and is not a directory" >&2; exit 2; }
  unexpected="$(find "$OUTPUT" -mindepth 1 -maxdepth 1 ! -name summary.md ! -name status.json -print -quit)"
  [ -z "$unexpected" ] || {
    echo "error: refusing output directory containing prior evidence: $unexpected" >&2; exit 2;
  }
fi

BUILD_ROOT="$(dirname "$OUTPUT")/build"
QUICKJS_SOURCE="$HARNESS_ROOT/third_party/quickjs-ng"
MANIFEST="$HARNESS_ROOT/benchmarks/.hosted-preview-${CANDIDATE_REVISION:0:12}-${BASE_REVISION:0:12}-$$.json"
REFERENCE_REPO="$(cd "$HARNESS_ROOT" && python3 -m tools.benchmark.preview reference \
  --manifest benchmarks/manifest.json --field repo)"
REFERENCE_REVISION="$(cd "$HARNESS_ROOT" && python3 -m tools.benchmark.preview reference \
  --manifest benchmarks/manifest.json --field revision)"
HARNESS_REVISION="$(git -C "$HARNESS_ROOT" rev-parse HEAD)"
RUN_COMPLETED=0
CURRENT_PHASE="initialization"
FAILURE_PHASE=""

record_error() {
  exit_status="$?"
  [ -n "$FAILURE_PHASE" ] || FAILURE_PHASE="$CURRENT_PHASE"
  return "$exit_status"
}

cleanup() {
  exit_status="$?"
  rm -f "$MANIFEST"
  if [ "$RUN_COMPLETED" -ne 1 ] && [ -d "$OUTPUT" ]; then
    (cd "$HARNESS_ROOT" && python3 -m tools.benchmark.preview status \
      --state failed --output-dir "$OUTPUT" \
      --phase "${FAILURE_PHASE:-$CURRENT_PHASE}" \
      --harness-mode "$HARNESS_MODE" --harness-revision "$HARNESS_REVISION" \
      --candidate-revision "$CANDIDATE_REVISION" --base-revision "$BASE_REVISION" \
      --reference-revision "$REFERENCE_REVISION" \
      --message "orchestration failed in phase ${FAILURE_PHASE:-$CURRENT_PHASE} with exit status $exit_status") || true
  fi
  exit "$exit_status"
}
trap cleanup EXIT
trap record_error ERR
trap 'exit 130' INT
trap 'exit 143' TERM

mkdir -p "$OUTPUT" "$BUILD_ROOT"
(cd "$HARNESS_ROOT" && python3 -m tools.benchmark.preview status \
  --state pending --phase initialization --output-dir "$OUTPUT" \
  --harness-mode "$HARNESS_MODE" --harness-revision "$HARNESS_REVISION" \
  --candidate-revision "$CANDIDATE_REVISION" --base-revision "$BASE_REVISION" \
  --reference-revision "$REFERENCE_REVISION" \
  --message "measurement did not finish; no performance conclusion is available")

# Candidate compilation and execution are not a security sandbox. Remove the
# GitHub command channels and ambient service credentials before candidate code
# can run, while preserving the ordinary build environment.
for name in \
  GITHUB_ENV GITHUB_PATH GITHUB_STEP_SUMMARY GITHUB_OUTPUT GITHUB_STATE \
  GITHUB_TOKEN ACTIONS_RUNTIME_TOKEN ACTIONS_RUNTIME_URL ACTIONS_CACHE_URL \
  ACTIONS_RESULTS_URL ACTIONS_ID_TOKEN_REQUEST_TOKEN ACTIONS_ID_TOKEN_REQUEST_URL; do
  unset "$name" || true
done

verify_source() {
  (cd "$HARNESS_ROOT" && python3 -m tools.benchmark.preview verify-source \
    --source "$1" --revision "$2")
}
CURRENT_PHASE="source_validation"
verify_source "$CANDIDATE_SOURCE" "$CANDIDATE_REVISION"
verify_source "$BASE_SOURCE" "$BASE_REVISION"
for source in "$CANDIDATE_SOURCE" "$BASE_SOURCE"; do
  if [ -e "$source/.cargo/config" ] || [ -e "$source/.cargo/config.toml" ]; then
    echo "error: source-local Cargo config is not allowed in hosted preview" >&2
    exit 2
  fi
done

CURRENT_PHASE="reference_setup"
git -C "$HARNESS_ROOT" submodule sync -- third_party/quickjs-ng
git -C "$HARNESS_ROOT" submodule update --init --depth 1 third_party/quickjs-ng
if [ "$(git -C "$QUICKJS_SOURCE" remote get-url origin)" != "$REFERENCE_REPO" ]; then
  echo "error: initialized QuickJS-NG repository does not match trusted pin" >&2
  exit 2
fi
verify_source "$QUICKJS_SOURCE" "$REFERENCE_REVISION"

CURRENT_PHASE="toolchain_validation"
candidate_rust="$(cd "$CANDIDATE_SOURCE" && rustc -vV)"
base_rust="$(cd "$BASE_SOURCE" && rustc -vV)"
candidate_cargo="$(cd "$CANDIDATE_SOURCE" && cargo -V)"
base_cargo="$(cd "$BASE_SOURCE" && cargo -V)"
if [ "$candidate_rust" != "$base_rust" ] || [ "$candidate_cargo" != "$base_cargo" ]; then
  echo "error: candidate and base did not resolve the same Rust toolchain" >&2
  exit 2
fi
RUST_TARGET="$(printf '%s\n' "$candidate_rust" | sed -n 's/^host: //p')"
[ -n "$RUST_TARGET" ] || { echo "error: rustc did not report a host target" >&2; exit 2; }
RUST_TOOLCHAIN="$(printf '%s\n' "$candidate_rust" | paste -sd ';' -); $candidate_cargo"
RUST_FLAGS="-Ctarget-cpu=generic"
CARGO_ARGS=(
  --locked --release -p qjs-cli
  --config=profile.release.opt-level=3
  --config=profile.release.debug=false
  --config=profile.release.debug-assertions=false
  --config=profile.release.overflow-checks=false
  --config=profile.release.lto=false
  --config=profile.release.codegen-units=16
  '--config=profile.release.panic="unwind"'
  --config=profile.release.incremental=false
  '--config=profile.release.strip="none"'
)
build_rust() {
  source="$1"
  target_dir="$2"
  (cd "$source" && \
    CARGO_TARGET_DIR="$target_dir" \
    CARGO_BUILD_TARGET="$RUST_TARGET" \
    CARGO_INCREMENTAL=0 \
    CARGO_ENCODED_RUSTFLAGS="$RUST_FLAGS" \
    cargo build "${CARGO_ARGS[@]}")
}
CURRENT_PHASE="build_candidate"
build_rust "$CANDIDATE_SOURCE" "$BUILD_ROOT/candidate-target"
verify_source "$CANDIDATE_SOURCE" "$CANDIDATE_REVISION"
CURRENT_PHASE="build_base"
build_rust "$BASE_SOURCE" "$BUILD_ROOT/base-target"
verify_source "$BASE_SOURCE" "$BASE_REVISION"
CANDIDATE_BINARY="$BUILD_ROOT/candidate-target/$RUST_TARGET/release/qjs"
BASE_BINARY="$BUILD_ROOT/base-target/$RUST_TARGET/release/qjs"

CURRENT_PHASE="build_quickjs_ng"
QUICKJS_CC="$(command -v cc)"
QUICKJS_TOOLCHAIN="$(printf '%s; %s; %s' \
  "$("$QUICKJS_CC" --version | sed -n '1p')" \
  "$(cmake --version | sed -n '1p')" "$(make --version | sed -n '1p')")"
QUICKJS_TARGET="$("$QUICKJS_CC" -dumpmachine)"
make -C "$QUICKJS_SOURCE" "CC=$QUICKJS_CC" BUILD_TYPE=Release all
verify_source "$QUICKJS_SOURCE" "$REFERENCE_REVISION"
QUICKJS_BINARY="$QUICKJS_SOURCE/build/qjs"
for binary in "$CANDIDATE_BINARY" "$BASE_BINARY" "$QUICKJS_BINARY"; do
  [ -x "$binary" ] || { echo "error: expected executable was not built: $binary" >&2; exit 2; }
done

CURRENT_PHASE="receipt_preparation"
PROFILE_PLATFORM="$(uname -s)-$(uname -m)"
PROFILE_ID="github-hosted-$(printf '%s' "$PROFILE_PLATFORM" | tr '[:upper:]' '[:lower:]')-informational-v1"
(cd "$HARNESS_ROOT" && python3 -m tools.benchmark.preview prepare \
  --template benchmarks/manifest.json --manifest-output "$MANIFEST" \
  --candidate-binary "$CANDIDATE_BINARY" --base-binary "$BASE_BINARY" \
  --quickjs-binary "$QUICKJS_BINARY" \
  --candidate-receipt "$OUTPUT/candidate-receipt.json" \
  --base-receipt "$OUTPUT/base-receipt.json" \
  --quickjs-receipt "$OUTPUT/quickjs-ng-receipt.json" \
  --candidate-repo "$CANDIDATE_REPO" --candidate-revision "$CANDIDATE_REVISION" \
  --base-repo "$BASE_REPO" --base-revision "$BASE_REVISION" \
  --profile-id "$PROFILE_ID" --platform "$PROFILE_PLATFORM" \
  --rust-toolchain "$RUST_TOOLCHAIN" --rust-target "$RUST_TARGET" \
  --quickjs-toolchain "$QUICKJS_TOOLCHAIN" --quickjs-target "$QUICKJS_TARGET" \
  --quickjs-cc "$QUICKJS_CC")

CURRENT_PHASE="measurement"
(cd "$HARNESS_ROOT" && ./scripts/benchmark.sh --manifest "$MANIFEST" --blocks 3 \
  --candidate "$CANDIDATE_BINARY" --candidate-receipt "$OUTPUT/candidate-receipt.json" \
  --base "$BASE_BINARY" --base-receipt "$OUTPUT/base-receipt.json" \
  --quickjs-ng "$QUICKJS_BINARY" --quickjs-ng-receipt "$OUTPUT/quickjs-ng-receipt.json" \
  --output "$OUTPUT/raw.jsonl")
CURRENT_PHASE="report"
(cd "$HARNESS_ROOT" && ./scripts/benchmark-report.sh \
  --manifest "$MANIFEST" --analysis-manifest benchmarks/analysis.json \
  --input "$OUTPUT/raw.jsonl" --output "$OUTPUT/report.json")
cp "$MANIFEST" "$OUTPUT/manifest.json"
rm -f "$MANIFEST"

CURRENT_PHASE="post_measure_validation"
verify_source "$CANDIDATE_SOURCE" "$CANDIDATE_REVISION"
verify_source "$BASE_SOURCE" "$BASE_REVISION"
verify_source "$QUICKJS_SOURCE" "$REFERENCE_REVISION"
(cd "$HARNESS_ROOT" && ./scripts/performance-policy-audit.sh)
(cd "$HARNESS_ROOT" && ./scripts/external-corpus-audit.sh)

CURRENT_PHASE="summary"
(cd "$HARNESS_ROOT" && python3 -m tools.benchmark.preview summary \
  --report "$OUTPUT/report.json" --markdown "$OUTPUT/summary.md" \
  --json-output "$OUTPUT/summary.json" --status-output "$OUTPUT/status.json" \
  --harness-mode "$HARNESS_MODE" --harness-revision "$HARNESS_REVISION")

RUN_COMPLETED=1
CURRENT_PHASE="complete"
printf 'performance preview evidence: %s\n' "$OUTPUT"
