#!/usr/bin/env bash
# Shared helpers for repository scripts. Source after defining ROOT_DIR:
#
#   ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
#   . "$ROOT_DIR/scripts/lib.sh"
#
# Helpers return non-zero instead of exiting so each caller keeps its own
# failure policy.

# Echoes the number of online CPUs, falling back to sysctl and then to 1.
# Used as the default parallelism for the Test262 runners.
qjs_detect_jobs() {
  local jobs=""
  if command -v getconf >/dev/null 2>&1; then
    jobs="$(getconf _NPROCESSORS_ONLN 2>/dev/null || true)"
  fi
  case "$jobs" in
    ''|*[!0-9]*|0)
      if command -v sysctl >/dev/null 2>&1; then
        jobs="$(sysctl -n hw.ncpu 2>/dev/null || true)"
      fi
      ;;
  esac
  case "$jobs" in
    ''|*[!0-9]*|0) jobs=1 ;;
  esac
  echo "$jobs"
}

# Echoes a usable cargo binary, honoring $CARGO and the rustup default
# install location. Returns 1 when no cargo is available.
qjs_resolve_cargo() {
  local cargo_bin="${CARGO:-cargo}"
  if command -v "$cargo_bin" >/dev/null 2>&1; then
    printf '%s\n' "$cargo_bin"
  elif [ -x "$HOME/.cargo/bin/cargo" ]; then
    printf '%s\n' "$HOME/.cargo/bin/cargo"
  else
    return 1
  fi
}

# Ensures RUN_WITH_TIMEOUT points at the executable timeout wrapper,
# defaulting to scripts/run-with-timeout.sh.
qjs_require_run_with_timeout() {
  RUN_WITH_TIMEOUT="${RUN_WITH_TIMEOUT:-$ROOT_DIR/scripts/run-with-timeout.sh}"
  if [ ! -x "$RUN_WITH_TIMEOUT" ]; then
    echo "error: missing executable $RUN_WITH_TIMEOUT" >&2
    return 1
  fi
}

# Builds the pinned QuickJS-NG reference when its qjs binary, or any extra
# build artifact passed as an argument, is missing.
qjs_ensure_quickjs_ng() {
  local ng_dir="$ROOT_DIR/third_party/quickjs-ng"
  if [ ! -d "$ng_dir" ]; then
    echo "error: missing $ng_dir; run ./scripts/bootstrap.sh first" >&2
    return 1
  fi
  local artifact missing=0
  for artifact in "$ng_dir/build/qjs" "$@"; do
    [ -x "$artifact" ] || missing=1
  done
  if [ "$missing" -eq 1 ]; then
    make -C "$ng_dir" all
  fi
}

# Echoes the qjs-cli debug binary path, honoring a prebuilt $QJS_CLI_BIN and
# otherwise building it with the given cargo binary.
qjs_build_cli_bin() {
  local cargo_bin="$1"
  if [ -n "${QJS_CLI_BIN:-}" ]; then
    if [ ! -x "$QJS_CLI_BIN" ]; then
      echo "error: QJS_CLI_BIN is not executable: $QJS_CLI_BIN" >&2
      return 1
    fi
    printf '%s\n' "$QJS_CLI_BIN"
    return 0
  fi
  "$cargo_bin" build -q -p qjs-cli >&2
  local target_dir
  target_dir="$("$cargo_bin" metadata --format-version=1 --no-deps \
    | sed -n 's/.*"target_directory":"\([^"]*\)".*/\1/p' \
    | head -n 1)"
  target_dir="${target_dir:-$ROOT_DIR/target}"
  local bin="$target_dir/debug/qjs"
  if [ ! -x "$bin" ]; then
    echo "error: built qjs-cli binary is missing or not executable: $bin" >&2
    return 1
  fi
  printf '%s\n' "$bin"
}
