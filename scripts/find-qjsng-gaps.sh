#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BASELINE="$ROOT_DIR/scripts/test262-baseline.sh"
TEST262_DIR="$ROOT_DIR/third_party/test262"
QUICKJS_NG_DIR="$ROOT_DIR/third_party/quickjs-ng"
RUN_LIMIT="${TEST262_GAP_LIMIT:-100}"
FILTER_PREFIX=""
OUT_DIR=""
TOP_COUNT=20
AREA_COUNT=10
RECOMMEND=1
RECOMMEND_STRATEGY="${TEST262_GAP_RECOMMEND_STRATEGY:-quickwins}"
RECOMMEND_BATCH_CAP="${TEST262_GAP_RECOMMEND_BATCH_CAP:-5}"
VERIFY_CANDIDATES="${TEST262_GAP_VERIFY_CANDIDATES:-5}"
INCLUDE_TIMEOUTS=0
EXACT_SCAN=0
REPORT_SOURCE=""
FROM_LATEST_REPORT=0
SKIP_AREAS=()
PROBE_LIMIT="${TEST262_GAP_PROBE_LIMIT:-100}"
PROBE_SHARDS="${TEST262_GAP_PROBE_SHARDS:-${TEST262_GAP_PROBE_SHARD:-1/16,5/16,9/16,13/16}}"

usage() {
  cat >&2 <<'USAGE'
usage: scripts/find-qjsng-gaps.sh [--limit N | --all] [--filter test/<prefix>] [--out-dir PATH] [--top N] [--areas N] [--strategy quickwins|fast|largest] [--recommend-batch-cap N] [--verify-candidates N] [--probe-limit N] [--probe-shard I/N] [--probe-shards I/N[,I/N...]] [--from-report PATH | --from-latest-report] [--skip-area test/<prefix>] [--exact] [--include-timeouts] [--no-recommend]

Runs a QuickJS-NG comparison baseline and prints the cases where QuickJS-NG
passes but quickjs-rust has an actionable feature gap. Stress timeouts are
reported separately by default; use --include-timeouts to include them in the
gap list and greedy recommendation.

For unfiltered --all recommendation runs, the default strategy is a fast greedy
probe over TEST262_GAP_PROBE_LIMIT cases from each TEST262_GAP_PROBE_SHARDS
shard, run concurrently. Use --exact --all when a complete audit is required,
especially to prove there are no remaining gaps.
The default recommendation strategy is quickwins greedy: prefer reviewable
engine-gap batches, then small harness-only batches that include at least one
case without broad-feature hints and may only need metadata confirmation, and
de-prioritize areas whose paths or skip metadata point at broad missing features
such as async, destructuring, class, yield, proxy, realm, species, resizable
buffers, or Annex B global-code semantics. Use --strategy fast for the older
small-batch-first behavior or --strategy largest for largest-gap-first.
For default global probes, the script exact-checks the top quickwins candidate
areas before printing a recommendation, so sampled areas that expand into broad
work do not outrank smaller exact wins. Use --verify-candidates 0 to disable
that follow-up, or TEST262_GAP_VERIFY_CANDIDATES to change the default.
Use --from-report with a previous output directory or cases.jsonl to recompute
the greedy recommendation without executing Test262 again. Use --skip-area to
ignore an area already being worked or already rechecked.
Raw summary and case JSONL files are written under target/test262-gaps/ unless
--out-dir is supplied.
USAGE
}

while [ "$#" -gt 0 ]; do
  case "$1" in
    --all)
      RUN_LIMIT="all"
      shift
      ;;
    --limit)
      [ "$#" -ge 2 ] || { usage; exit 2; }
      RUN_LIMIT="$2"
      shift 2
      ;;
    --filter)
      [ "$#" -ge 2 ] || { usage; exit 2; }
      FILTER_PREFIX="$2"
      shift 2
      ;;
    --out-dir)
      [ "$#" -ge 2 ] || { usage; exit 2; }
      OUT_DIR="$2"
      shift 2
      ;;
    --top)
      [ "$#" -ge 2 ] || { usage; exit 2; }
      TOP_COUNT="$2"
      shift 2
      ;;
    --areas)
      [ "$#" -ge 2 ] || { usage; exit 2; }
      AREA_COUNT="$2"
      shift 2
      ;;
    --strategy)
      [ "$#" -ge 2 ] || { usage; exit 2; }
      RECOMMEND_STRATEGY="$2"
      shift 2
      ;;
    --recommend-batch-cap)
      [ "$#" -ge 2 ] || { usage; exit 2; }
      RECOMMEND_BATCH_CAP="$2"
      shift 2
      ;;
    --verify-candidates)
      [ "$#" -ge 2 ] || { usage; exit 2; }
      VERIFY_CANDIDATES="$2"
      shift 2
      ;;
    --probe-limit)
      [ "$#" -ge 2 ] || { usage; exit 2; }
      PROBE_LIMIT="$2"
      shift 2
      ;;
    --probe-shard)
      [ "$#" -ge 2 ] || { usage; exit 2; }
      PROBE_SHARDS="$2"
      shift 2
      ;;
    --probe-shards)
      [ "$#" -ge 2 ] || { usage; exit 2; }
      PROBE_SHARDS="$2"
      shift 2
      ;;
    --from-report)
      [ "$#" -ge 2 ] || { usage; exit 2; }
      REPORT_SOURCE="$2"
      shift 2
      ;;
    --from-latest-report)
      FROM_LATEST_REPORT=1
      shift
      ;;
    --skip-area)
      [ "$#" -ge 2 ] || { usage; exit 2; }
      SKIP_AREAS+=("$2")
      shift 2
      ;;
    --exact)
      EXACT_SCAN=1
      shift
      ;;
    --recommend)
      RECOMMEND=1
      shift
      ;;
    --include-timeouts)
      INCLUDE_TIMEOUTS=1
      shift
      ;;
    --no-recommend)
      RECOMMEND=0
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      usage
      exit 2
      ;;
  esac
done

if [ "$FROM_LATEST_REPORT" -eq 1 ] && [ -n "$REPORT_SOURCE" ]; then
  echo "error: use only one of --from-report or --from-latest-report" >&2
  exit 2
fi

case "$RUN_LIMIT" in
  all) ;;
  ''|*[!0-9]*)
    echo "error: --limit must be a non-negative integer or --all: $RUN_LIMIT" >&2
    exit 2
    ;;
esac
case "$PROBE_LIMIT" in
  ''|*[!0-9]*|0)
    echo "error: --probe-limit must be a positive integer: $PROBE_LIMIT" >&2
    exit 2
    ;;
esac
IFS=',' read -r -a PROBE_SHARD_LIST <<<"$PROBE_SHARDS"
if [ "${#PROBE_SHARD_LIST[@]}" -eq 0 ]; then
  echo "error: --probe-shards must not be empty" >&2
  exit 2
fi
for shard in "${PROBE_SHARD_LIST[@]}"; do
  case "$shard" in
    */*) ;;
    *)
      echo "error: probe shards must use I/N form: $shard" >&2
      exit 2
      ;;
  esac
done
case "$TOP_COUNT:$AREA_COUNT" in
  *[!0-9:]*|0:*|*:0)
    echo "error: --top and --areas must be positive integers" >&2
    exit 2
    ;;
esac
case "$RECOMMEND_STRATEGY" in
  quickwins|fast|largest) ;;
  *)
    echo "error: --strategy must be quickwins, fast, or largest: $RECOMMEND_STRATEGY" >&2
    exit 2
    ;;
esac
case "$RECOMMEND_BATCH_CAP" in
  ''|*[!0-9]*|0)
    echo "error: --recommend-batch-cap must be a positive integer: $RECOMMEND_BATCH_CAP" >&2
    exit 2
    ;;
esac
case "$VERIFY_CANDIDATES" in
  ''|*[!0-9]*)
    echo "error: --verify-candidates must be a non-negative integer: $VERIFY_CANDIDATES" >&2
    exit 2
    ;;
esac

if [ ! -x "$BASELINE" ]; then
  echo "error: missing executable $BASELINE" >&2
  exit 1
fi
if [ "$FROM_LATEST_REPORT" -eq 1 ]; then
  latest_root="$ROOT_DIR/target/test262-gaps"
  if [ ! -d "$latest_root" ]; then
    echo "error: no previous gap reports found under $latest_root" >&2
    exit 2
  fi
  REPORT_SOURCE="$(find "$latest_root" -mindepth 1 -maxdepth 1 -type d -name '*-*' -exec test -f '{}/cases.jsonl' ';' -print | sort | tail -n 1)"
  if [ -z "$REPORT_SOURCE" ]; then
    echo "error: no previous gap report with cases.jsonl found under $latest_root" >&2
    exit 2
  fi
fi
REPORT_CASES_SOURCE=""
if [ -n "$REPORT_SOURCE" ]; then
  if [ -d "$REPORT_SOURCE" ] && [ -f "$REPORT_SOURCE/cases.jsonl" ]; then
    REPORT_CASES_SOURCE="$REPORT_SOURCE/cases.jsonl"
  elif [ -f "$REPORT_SOURCE" ]; then
    REPORT_CASES_SOURCE="$REPORT_SOURCE"
  else
    echo "error: --from-report must point to a gap output directory or cases.jsonl: $REPORT_SOURCE" >&2
    exit 2
  fi
fi
if [ -n "$FILTER_PREFIX" ] && [ ! -f "$TEST262_DIR/$FILTER_PREFIX" ] && [ ! -d "$TEST262_DIR/$FILTER_PREFIX" ]; then
  echo "error: --filter does not match a Test262 file or directory: $FILTER_PREFIX" >&2
  echo "hint: use a path under third_party/test262/test, for example test/built-ins/String" >&2
  exit 2
fi

if [ -z "$OUT_DIR" ]; then
  stamp="$(date +%Y%m%d-%H%M%S)-$$"
  scope="${FILTER_PREFIX:-all}"
  scope="$(printf '%s' "$scope" | tr '/ :' '---' | tr -cd '[:alnum:]_.-')"
  OUT_DIR="$ROOT_DIR/target/test262-gaps/$scope-$stamp"
fi
mkdir -p "$OUT_DIR"

write_gap_files() {
  local cases_file="$1"
  local gaps_file="$2"
  local timeouts_file="$3"
  : >"$timeouts_file"
  awk -F'"' -v include_timeouts="$INCLUDE_TIMEOUTS" -v timeouts_file="$timeouts_file" '
    function area_for(path, parts, n, limit, i, area) {
      n = split(path, parts, "/")
      if (n < 2) {
        return path
      }
      if (parts[2] == "built-ins" && n >= 6) {
        limit = 5
      } else if (n >= 4) {
        limit = n == 4 ? 3 : 4
      } else {
        limit = n - 1
      }
      area = parts[1]
      for (i = 2; i <= limit; i++) {
        area = area "/" parts[i]
      }
      return area
    }
    {
      path = $4
      rust_result = $12
      rust_skip = $16
      qjsng_result = $24
      if (qjsng_result == "pass" && rust_result != "pass") {
        area = area_for(path)
        line = path "\t" rust_result "\t" rust_skip "\t" area
        if (rust_result == "timeout") {
          print line > timeouts_file
          if (include_timeouts != 1) {
            next
          }
        }
        print line
      }
    }
  ' "$cases_file" >"$gaps_file"
}

apply_skipped_areas() {
  local gaps_file="$1"
  if [ "${#SKIP_AREAS[@]}" -eq 0 ]; then
    return
  fi
  local skip_file="$OUT_DIR/skip-areas.txt"
  printf '%s\n' "${SKIP_AREAS[@]}" >"$skip_file"
  awk -F'\t' -v skip_file="$skip_file" '
    BEGIN {
      while ((getline area < skip_file) > 0) {
        skip[area] = 1
      }
      close(skip_file)
    }
    !($4 in skip)
  ' "$gaps_file" >"$gaps_file.tmp"
  mv "$gaps_file.tmp" "$gaps_file"
}

write_recommendations() {
  local gaps_file="$1"
  local recommendations_file="$2"
  awk -F'\t' -v strategy="$RECOMMEND_STRATEGY" -v batch_cap="$RECOMMEND_BATCH_CAP" '
    function has_hard_hint(path, skip, area) {
      if (area ~ /annexB\/language\/global-code$/) {
        return 1
      }
      if (skip ~ /async/) {
        return 1
      }
      if (path ~ /(^|\/)(class|dstr|for-await-of|yield)(\/|$)/) {
        return 1
      }
      if (path ~ /(destructur|proxy|Proxy|realm|Realm|species|superclass|resizable-buffer|growable-buffer)/) {
        return 1
      }
      return 0
    }
    {
      area = $4
      total[area]++
      if ($2 == "fail") {
        fail[area]++
      } else if ($2 == "timeout") {
        timeout[area]++
      } else if ($2 == "skipped") {
        skipped[area]++
      }
      if (has_hard_hint($1, $3, area)) {
        hard[area]++
      }
    }
    END {
      for (area in total) {
        engine = fail[area] + timeout[area]
        harness = skipped[area] + 0
        hard_hints = hard[area] + 0
        if (strategy == "largest") {
          score = (engine * 1000000) + (total[area] * 1000) - harness
        } else if (strategy == "quickwins") {
          if (engine > 0 && engine <= batch_cap && hard_hints == 0) {
            score = 1000000000 + (engine * 1000000) + (total[area] * 1000) - harness
          } else if (engine > 0 && hard_hints == 0) {
            over = engine - batch_cap
            score = 900000000 - (over * 1000000) + (engine * 1000) + total[area] - harness
          } else if (engine == 0 && hard_hints == 0 && total[area] <= batch_cap) {
            score = 800000000 + (total[area] * 1000) - harness
          } else if (engine == 0 && hard_hints == 0) {
            score = 700000000 + (total[area] * 1000) - harness
          } else if (engine == 0 && total[area] <= batch_cap && hard_hints < total[area]) {
            score = 650000000 + (total[area] * 1000) - (hard_hints * 10000) - harness
          } else if (engine == 0 && hard_hints < total[area]) {
            over = total[area] - batch_cap
            score = 625000000 - (over * 1000000) + total[area] - (hard_hints * 10000) - harness
          } else if (engine == 0 && total[area] <= batch_cap) {
            score = 400000000 + (total[area] * 1000) - (hard_hints * 10000) - harness
          } else if (engine == 0) {
            over = total[area] - batch_cap
            score = 300000000 - (over * 1000000) + total[area] - (hard_hints * 10000) - harness
          } else if (engine > 0 && engine <= batch_cap) {
            score = 600000000 + (engine * 1000000) + (total[area] * 1000) - (hard_hints * 10000) - harness
          } else if (engine > 0) {
            over = engine - batch_cap
            score = 500000000 - (over * 1000000) + (engine * 1000) + total[area] - (hard_hints * 10000) - harness
          } else {
            score = (total[area] * 1000) - (hard_hints * 10000) - harness
          }
        } else if (engine > 0 && engine <= batch_cap) {
          score = 1000000000 + (engine * 1000000) + (total[area] * 1000) - harness
        } else if (engine == 0 && total[area] <= batch_cap) {
          score = 900000000 + (total[area] * 1000) - harness
        } else if (engine > 0) {
          over = engine - batch_cap
          score = 100000000 - (over * 1000000) + (engine * 1000) + total[area] - harness
        } else {
          score = (total[area] * 1000) - harness
        }
        printf "%d\t%d\t%d\t%d\t%d\t%d\t%s\n", score, total[area], engine, fail[area] + 0, harness, hard_hints, area
      }
    }
  ' "$gaps_file" | sort -nr >"$recommendations_file"
}

if [ -z "$REPORT_CASES_SOURCE" ] && [ -z "${QJS_CLI_BIN:-}" ] && [ "$RUN_LIMIT" = "all" ] && [ -z "$FILTER_PREFIX" ] && [ "$RECOMMEND" -eq 1 ] && [ "$EXACT_SCAN" -eq 0 ]; then
  if command -v cargo >/dev/null 2>&1; then
    CARGO_BIN="cargo"
  elif [ -x "$HOME/.cargo/bin/cargo" ]; then
    CARGO_BIN="$HOME/.cargo/bin/cargo"
  else
    echo "error: cargo not found; install Rust with rustup before running gap probes" >&2
    exit 127
  fi
  build_output="$(mktemp "${TMPDIR:-/tmp}/qjs-gap-cargo-build-XXXXXX")"
  set +e
  "$CARGO_BIN" build -q --message-format=json-render-diagnostics -p qjs-cli >"$build_output"
  build_status=$?
  set -e
  if [ "$build_status" -ne 0 ]; then
    cat "$build_output" >&2
    rm -f "$build_output"
    exit "$build_status"
  fi
  QJS_CLI_BIN="$(sed -n 's/.*"executable":"\([^"]*\)".*/\1/p' "$build_output" | tail -n 1)"
  rm -f "$build_output"
  if [ -z "$QJS_CLI_BIN" ]; then
    QJS_CLI_BIN="$ROOT_DIR/target/debug/qjs"
  fi
  export QJS_CLI_BIN
fi
if [ -z "$REPORT_CASES_SOURCE" ] && [ "$RUN_LIMIT" = "all" ] && [ -z "$FILTER_PREFIX" ] && [ "$RECOMMEND" -eq 1 ] && [ "$EXACT_SCAN" -eq 0 ]; then
  if [ ! -d "$QUICKJS_NG_DIR" ]; then
    echo "error: missing $QUICKJS_NG_DIR; run ./scripts/bootstrap.sh first" >&2
    exit 1
  fi
  if [ ! -x "$QUICKJS_NG_DIR/build/qjs" ] || [ ! -x "$QUICKJS_NG_DIR/build/run-test262" ]; then
    make -C "$QUICKJS_NG_DIR" all
  fi
fi

SUMMARY_JSON="$OUT_DIR/summary.json"
CASES_JSONL="$OUT_DIR/cases.jsonl"
GAPS_TSV="$OUT_DIR/qjsng-pass-rust-nonpass.tsv"
TIMEOUTS_TSV="$OUT_DIR/qjsng-pass-rust-timeout.tsv"
AREAS_TSV="$OUT_DIR/gap-areas.tsv"
RECOMMENDATIONS_TSV="$OUT_DIR/recommendations.tsv"
BASELINE_LOG="$OUT_DIR/baseline.log"

PROBE_SCAN=0
BASELINE_RUN_LIMIT="$RUN_LIMIT"
if [ "$RUN_LIMIT" = "all" ] && [ -z "$FILTER_PREFIX" ] && [ "$RECOMMEND" -eq 1 ] && [ "$EXACT_SCAN" -eq 0 ]; then
  PROBE_SCAN=1
  BASELINE_RUN_LIMIT="$PROBE_LIMIT"
fi

baseline_args=(--engine both --summary-json "$SUMMARY_JSON" --case-results-jsonl "$CASES_JSONL" --no-fail)
if [ "$BASELINE_RUN_LIMIT" = "all" ]; then
  baseline_args+=(--all)
else
  baseline_args+=(--limit "$BASELINE_RUN_LIMIT")
fi
if [ -n "$FILTER_PREFIX" ]; then
  baseline_args+=(--filter "$FILTER_PREFIX")
fi

if [ -n "$REPORT_CASES_SOURCE" ]; then
  echo "Recomputing QuickJS-NG gap recommendation from an existing report..."
  echo "  source: $REPORT_CASES_SOURCE"
  echo "  log: $BASELINE_LOG"
  cp "$REPORT_CASES_SOURCE" "$CASES_JSONL"
  {
    printf '{\n'
    printf '  "engine": "both",\n'
    printf '  "filter": "%s",\n' "$FILTER_PREFIX"
    printf '  "mode": "replayed-report",\n'
    printf '  "source": "%s"\n' "$REPORT_CASES_SOURCE"
    printf '}\n'
  } >"$SUMMARY_JSON"
  printf 'replayed cases from %s\n' "$REPORT_CASES_SOURCE" >"$BASELINE_LOG"
else
  echo "Running Test262 comparison against QuickJS-NG..."
  echo "  log: $BASELINE_LOG"
fi
if [ -z "$REPORT_CASES_SOURCE" ] && [ "$PROBE_SCAN" -eq 1 ]; then
  : >"$CASES_JSONL"
  : >"$BASELINE_LOG"
  probe_dir="$OUT_DIR/probes"
  mkdir -p "$probe_dir"
  pids=()
  logs=()
  cases=()
  summaries=()
  for shard in "${PROBE_SHARD_LIST[@]}"; do
    safe_shard="$(printf '%s' "$shard" | tr '/' '-')"
    shard_dir="$probe_dir/$safe_shard"
    mkdir -p "$shard_dir"
    shard_log="$shard_dir/baseline.log"
    shard_cases="$shard_dir/cases.jsonl"
    shard_summary="$shard_dir/summary.json"
    logs+=("$shard_log")
    cases+=("$shard_cases")
    summaries+=("$shard_summary")
    shard_args=(--engine both --summary-json "$shard_summary" --case-results-jsonl "$shard_cases" --no-fail --limit "$PROBE_LIMIT" --shard "$shard" --stop-after-limit)
    "$BASELINE" "${shard_args[@]}" >"$shard_log" 2>&1 &
    pids+=("$!")
  done

  baseline_status=0
  for index in "${!pids[@]}"; do
    if ! wait "${pids[$index]}"; then
      baseline_status=1
      echo "error: probe shard ${PROBE_SHARD_LIST[$index]} failed; last log lines:" >&2
      tail -n 40 "${logs[$index]}" >&2
    fi
  done
  for index in "${!logs[@]}"; do
    {
      printf '== probe shard %s ==\n' "${PROBE_SHARD_LIST[$index]}"
      cat "${logs[$index]}"
      printf '\n'
    } >>"$BASELINE_LOG"
    if [ -f "${cases[$index]}" ]; then
      cat "${cases[$index]}" >>"$CASES_JSONL"
    fi
  done
  {
    printf '{\n'
    printf '  "engine": "both",\n'
    printf '  "filter": "",\n'
    printf '  "mode": "greedy-probe",\n'
    printf '  "probe_limit_per_shard": %s,\n' "$PROBE_LIMIT"
    printf '  "probe_shards": ['
    for index in "${!PROBE_SHARD_LIST[@]}"; do
      [ "$index" -eq 0 ] || printf ', '
      printf '"%s"' "${PROBE_SHARD_LIST[$index]}"
    done
    printf '],\n'
    printf '  "probe_summaries": ['
    for index in "${!summaries[@]}"; do
      [ "$index" -eq 0 ] || printf ', '
      printf '"%s"' "${summaries[$index]#"$OUT_DIR/"}"
    done
    printf ']\n'
    printf '}\n'
  } >"$SUMMARY_JSON"
  if [ "$baseline_status" -ne 0 ]; then
    exit "$baseline_status"
  fi
elif [ -z "$REPORT_CASES_SOURCE" ]; then
  set +e
  "$BASELINE" "${baseline_args[@]}" >"$BASELINE_LOG" 2>&1
  baseline_status=$?
  set -e
  if [ "$baseline_status" -ne 0 ]; then
    echo "error: baseline comparison failed; last log lines:" >&2
    tail -n 40 "$BASELINE_LOG" >&2
    exit "$baseline_status"
  fi
fi

write_gap_files "$CASES_JSONL" "$GAPS_TSV" "$TIMEOUTS_TSV"
apply_skipped_areas "$GAPS_TSV"

awk -F'\t' '{ count[$4]++ } END { for (area in count) print count[area] "\t" area }' "$GAPS_TSV" \
  | sort -nr >"$AREAS_TSV"

write_recommendations "$GAPS_TSV" "$RECOMMENDATIONS_TSV"

VERIFIED_CANDIDATES=0
if [ "$PROBE_SCAN" -eq 1 ] && [ "$RECOMMEND" -eq 1 ] && [ "$RECOMMEND_STRATEGY" = "quickwins" ] && [ "$VERIFY_CANDIDATES" -gt 0 ] && [ -s "$RECOMMENDATIONS_TSV" ]; then
  PROBE_RECOMMENDATIONS_TSV="$OUT_DIR/probe-recommendations.tsv"
  VERIFIED_CASES_JSONL="$OUT_DIR/exact-candidate-cases.jsonl"
  VERIFIED_GAPS_TSV="$OUT_DIR/exact-candidate-gaps.tsv"
  VERIFIED_TIMEOUTS_TSV="$OUT_DIR/exact-candidate-timeouts.tsv"
  cp "$RECOMMENDATIONS_TSV" "$PROBE_RECOMMENDATIONS_TSV"
  : >"$VERIFIED_CASES_JSONL"
  verify_dir="$OUT_DIR/exact-candidates"
  mkdir -p "$verify_dir"
  candidate_areas=()
  while IFS= read -r area; do
    candidate_areas+=("$area")
  done < <(awk -F'\t' -v limit="$VERIFY_CANDIDATES" '{ print $7; shown++ } shown >= limit { exit }' "$PROBE_RECOMMENDATIONS_TSV")
  if [ "${#candidate_areas[@]}" -gt 0 ]; then
    echo "Verifying top greedy candidate areas exactly..."
    for area in "${candidate_areas[@]}"; do
      safe_area="$(printf '%s' "$area" | tr '/ :' '---' | tr -cd '[:alnum:]_.-')"
      area_dir="$verify_dir/$safe_area"
      mkdir -p "$area_dir"
      area_summary="$area_dir/summary.json"
      area_cases="$area_dir/cases.jsonl"
      area_log="$area_dir/baseline.log"
      echo "  exact: $area"
      set +e
      "$BASELINE" --engine both --summary-json "$area_summary" --case-results-jsonl "$area_cases" --no-fail --all --filter "$area" >"$area_log" 2>&1
      area_status=$?
      set -e
      if [ "$area_status" -ne 0 ]; then
        echo "error: exact candidate verification failed for $area; last log lines:" >&2
        tail -n 40 "$area_log" >&2
        exit "$area_status"
      fi
      cat "$area_cases" >>"$VERIFIED_CASES_JSONL"
      VERIFIED_CANDIDATES=$((VERIFIED_CANDIDATES + 1))
    done
    write_gap_files "$VERIFIED_CASES_JSONL" "$VERIFIED_GAPS_TSV" "$VERIFIED_TIMEOUTS_TSV"
    apply_skipped_areas "$VERIFIED_GAPS_TSV"
    write_recommendations "$VERIFIED_GAPS_TSV" "$RECOMMENDATIONS_TSV"
  fi
fi
RECOMMENDED_CASES_TSV="$GAPS_TSV"
if [ "$VERIFIED_CANDIDATES" -gt 0 ] && [ -s "${VERIFIED_GAPS_TSV:-}" ]; then
  RECOMMENDED_CASES_TSV="$VERIFIED_GAPS_TSV"
fi

total_gaps="$(wc -l <"$GAPS_TSV" | tr -d ' ')"
rust_fail="$(awk -F'\t' '$2 == "fail" { count++ } END { print count + 0 }' "$GAPS_TSV")"
rust_timeout="$(awk -F'\t' '$2 == "timeout" { count++ } END { print count + 0 }' "$GAPS_TSV")"
excluded_timeout="$(wc -l <"$TIMEOUTS_TSV" | tr -d ' ')"
rust_not_run="$(awk -F'\t' '$2 == "skipped" { count++ } END { print count + 0 }' "$GAPS_TSV")"

echo
echo "QuickJS-NG gap report"
echo "  filter: ${FILTER_PREFIX:-test/}"
echo "  requested limit: $RUN_LIMIT"
if [ "$PROBE_SCAN" -eq 1 ]; then
  echo "  scan: greedy probe over $BASELINE_RUN_LIMIT cases per shard from shards $PROBE_SHARDS"
  if [ "$VERIFIED_CANDIDATES" -gt 0 ]; then
    echo "  recommendation verification: exact scan of $VERIFIED_CANDIDATES candidate areas"
  fi
elif [ -n "$REPORT_CASES_SOURCE" ]; then
  echo "  scan: replayed report"
else
  echo "  scan: exact requested range"
fi
if [ "${#SKIP_AREAS[@]}" -gt 0 ]; then
  echo "  skipped areas: ${SKIP_AREAS[*]}"
fi
echo "  output: $OUT_DIR"
echo
if [ "$INCLUDE_TIMEOUTS" -eq 1 ]; then
  echo "QuickJS-NG passes, quickjs-rust does not: $total_gaps"
else
  echo "QuickJS-NG passes, quickjs-rust actionable gaps: $total_gaps"
fi
echo "  rust fail: $rust_fail"
echo "  rust timeout: $rust_timeout"
if [ "$INCLUDE_TIMEOUTS" -eq 0 ]; then
  echo "  rust timeout excluded: $excluded_timeout"
fi
echo "  rust not run: $rust_not_run"

if [ "$total_gaps" -eq 0 ]; then
  echo
  if [ "$PROBE_SCAN" -eq 1 ]; then
    echo "No gaps found in the greedy probe. Run with --exact --all to confirm the full Test262 range."
  elif [ "$INCLUDE_TIMEOUTS" -eq 1 ]; then
    echo "No QuickJS-NG pass / quickjs-rust non-pass cases found in this run."
  else
    echo "No QuickJS-NG pass / quickjs-rust actionable gaps found in this run."
  fi
  exit 0
fi

echo
echo "Top gap areas:"
head -n "$AREA_COUNT" "$AREAS_TSV" | awk -F'\t' '{ printf "  %s  %s\n", $1, $2 }'

echo
echo "First actionable cases:"
head -n "$TOP_COUNT" "$GAPS_TSV" | awk -F'\t' '{
  label = $2
  if ($3 != "") {
    label = label ", " $3
  }
  printf "  %s  (%s)\n", $1, label
}'

if [ "$RECOMMEND" -eq 1 ]; then
  if [ "$RECOMMEND_STRATEGY" = "quickwins" ] || [ "$RECOMMEND_STRATEGY" = "fast" ]; then
    recommendation="$(sed -n '1p' "$RECOMMENDATIONS_TSV")"
  else
    recommendation="$(awk -F'\t' '$3 > 0 { print; exit }' "$RECOMMENDATIONS_TSV")"
    if [ -z "$recommendation" ]; then
      recommendation="$(sed -n '1p' "$RECOMMENDATIONS_TSV")"
    fi
  fi
  if [ -n "$recommendation" ]; then
    IFS=$'\t' read -r _score rec_total rec_engine rec_fail rec_harness rec_hard rec_area <<<"$recommendation"
    rec_timeout=$((rec_engine - rec_fail))
    echo
    echo "Greedy recommendation:"
    echo "  strategy: $RECOMMEND_STRATEGY"
    if [ "$RECOMMEND_STRATEGY" = "quickwins" ] || [ "$RECOMMEND_STRATEGY" = "fast" ]; then
      echo "  batch cap: $RECOMMEND_BATCH_CAP engine gaps"
    fi
    echo "  area: $rec_area"
    echo "  gaps: $rec_total"
    echo "  engine gaps: $rec_engine"
    echo "  rust fail: $rec_fail"
    echo "  rust timeout: $rec_timeout"
    echo "  rust not run: $rec_harness"
    if [ "$RECOMMEND_STRATEGY" = "quickwins" ]; then
      echo "  hard hints: $rec_hard"
    fi
    echo "  rerun: ./scripts/find-qjsng-gaps.sh --filter $rec_area --all"
    if [ "$PROBE_SCAN" -eq 1 ]; then
      if [ "$VERIFIED_CANDIDATES" -gt 0 ]; then
        echo "  note: recommendation is exact-verified from the top greedy probe candidates; use --exact --all for a complete audit."
      else
        echo "  note: recommendation is from a greedy probe; use --exact --all for a complete audit."
      fi
    fi
    echo
    echo "Recommended cases:"
    awk -F'\t' -v area="$rec_area" -v top="$TOP_COUNT" '$4 == area {
      label = $2
      if ($3 != "") {
        label = label ", " $3
      }
      priority = 3
      if ($2 == "fail") {
        priority = 0
      } else if ($2 == "timeout") {
        priority = 1
      } else if ($2 == "skipped") {
        priority = 2
      }
      printf "%d\t%s\t%s\n", priority, $1, label
    }' "$RECOMMENDED_CASES_TSV" | sort -n | head -n "$TOP_COUNT" | awk -F'\t' '{ printf "  %s  (%s)\n", $2, $3 }'
    echo
    echo "Next candidate areas:"
    awk -F'\t' '{ printf "  %s  engine=%s gaps=%s harness=%s hard=%s\n", $7, $3, $2, $5, $6; shown++ } shown >= 5 { exit }' "$RECOMMENDATIONS_TSV"
  fi
fi

echo
echo "Raw files:"
echo "  summary: $SUMMARY_JSON"
echo "  cases: $CASES_JSONL"
echo "  gaps: $GAPS_TSV"
echo "  excluded timeouts: $TIMEOUTS_TSV"
echo "  recommendations: $RECOMMENDATIONS_TSV"
if [ "$PROBE_SCAN" -eq 1 ] && [ "$VERIFIED_CANDIDATES" -gt 0 ]; then
  echo "  probe recommendations: $PROBE_RECOMMENDATIONS_TSV"
  echo "  exact candidate cases: $VERIFIED_CASES_JSONL"
fi
echo "  baseline log: $BASELINE_LOG"
