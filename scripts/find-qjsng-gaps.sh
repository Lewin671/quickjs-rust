#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BASELINE="$ROOT_DIR/scripts/test262-baseline.sh"
TEST262_DIR="$ROOT_DIR/third_party/test262"
RUN_LIMIT="${TEST262_GAP_LIMIT:-100}"
FILTER_PREFIX=""
OUT_DIR=""
TOP_COUNT=20
AREA_COUNT=10
RECOMMEND=1
INCLUDE_TIMEOUTS=0
EXACT_SCAN=0
PROBE_LIMIT="${TEST262_GAP_PROBE_LIMIT:-100}"
PROBE_SHARD="${TEST262_GAP_PROBE_SHARD:-1/16}"

usage() {
  cat >&2 <<'USAGE'
usage: scripts/find-qjsng-gaps.sh [--limit N | --all] [--filter test/<prefix>] [--out-dir PATH] [--top N] [--areas N] [--probe-limit N] [--probe-shard I/N] [--exact] [--include-timeouts] [--no-recommend]

Runs a QuickJS-NG comparison baseline and prints the cases where QuickJS-NG
passes but quickjs-rust has an actionable feature gap. Stress timeouts are
reported separately by default; use --include-timeouts to include them in the
gap list and greedy recommendation.

For unfiltered --all recommendation runs, the default strategy is a fast greedy
probe over TEST262_GAP_PROBE_LIMIT cases from TEST262_GAP_PROBE_SHARD. Use
--exact --all when a complete audit is required, especially to prove there are
no remaining gaps.
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
    --probe-limit)
      [ "$#" -ge 2 ] || { usage; exit 2; }
      PROBE_LIMIT="$2"
      shift 2
      ;;
    --probe-shard)
      [ "$#" -ge 2 ] || { usage; exit 2; }
      PROBE_SHARD="$2"
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
case "$PROBE_SHARD" in
  */*) ;;
  *)
    echo "error: --probe-shard must use I/N form: $PROBE_SHARD" >&2
    exit 2
    ;;
esac
case "$TOP_COUNT:$AREA_COUNT" in
  *[!0-9:]*|0:*|*:0)
    echo "error: --top and --areas must be positive integers" >&2
    exit 2
    ;;
esac

if [ ! -x "$BASELINE" ]; then
  echo "error: missing executable $BASELINE" >&2
  exit 1
fi
if [ -n "$FILTER_PREFIX" ] && [ ! -f "$TEST262_DIR/$FILTER_PREFIX" ] && [ ! -d "$TEST262_DIR/$FILTER_PREFIX" ]; then
  echo "error: --filter does not match a Test262 file or directory: $FILTER_PREFIX" >&2
  echo "hint: use a path under third_party/test262/test, for example test/built-ins/String" >&2
  exit 2
fi

if [ -z "$OUT_DIR" ]; then
  stamp="$(date +%Y%m%d-%H%M%S)"
  scope="${FILTER_PREFIX:-all}"
  scope="$(printf '%s' "$scope" | tr '/ :' '---' | tr -cd '[:alnum:]_.-')"
  OUT_DIR="$ROOT_DIR/target/test262-gaps/$scope-$stamp"
fi
mkdir -p "$OUT_DIR"

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
if [ "$PROBE_SCAN" -eq 1 ]; then
  baseline_args+=(--shard "$PROBE_SHARD" --stop-after-limit)
fi
if [ -n "$FILTER_PREFIX" ]; then
  baseline_args+=(--filter "$FILTER_PREFIX")
fi

echo "Running Test262 comparison against QuickJS-NG..."
echo "  log: $BASELINE_LOG"
set +e
"$BASELINE" "${baseline_args[@]}" >"$BASELINE_LOG" 2>&1
baseline_status=$?
set -e
if [ "$baseline_status" -ne 0 ]; then
  echo "error: baseline comparison failed; last log lines:" >&2
  tail -n 40 "$BASELINE_LOG" >&2
  exit "$baseline_status"
fi

: >"$TIMEOUTS_TSV"
awk -F'"' -v include_timeouts="$INCLUDE_TIMEOUTS" -v timeouts_file="$TIMEOUTS_TSV" '
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
' "$CASES_JSONL" >"$GAPS_TSV"

awk -F'\t' '{ count[$4]++ } END { for (area in count) print count[area] "\t" area }' "$GAPS_TSV" \
  | sort -nr >"$AREAS_TSV"

awk -F'\t' '
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
  }
  END {
    for (area in total) {
      engine = fail[area] + timeout[area]
      harness = skipped[area] + 0
      score = (engine * 1000000) + (total[area] * 1000) - harness
      printf "%d\t%d\t%d\t%d\t%d\t%s\n", score, total[area], engine, fail[area] + 0, harness, area
    }
  }
' "$GAPS_TSV" | sort -nr >"$RECOMMENDATIONS_TSV"

total_gaps="$(wc -l <"$GAPS_TSV" | tr -d ' ')"
rust_fail="$(awk -F'\t' '$2 == "fail" { count++ } END { print count + 0 }' "$GAPS_TSV")"
rust_timeout="$(awk -F'\t' '$2 == "timeout" { count++ } END { print count + 0 }' "$GAPS_TSV")"
excluded_timeout="$(wc -l <"$TIMEOUTS_TSV" | tr -d ' ')"
harness_gap="$(awk -F'\t' '$2 == "skipped" { count++ } END { print count + 0 }' "$GAPS_TSV")"

echo
echo "QuickJS-NG gap report"
echo "  filter: ${FILTER_PREFIX:-test/}"
echo "  requested limit: $RUN_LIMIT"
if [ "$PROBE_SCAN" -eq 1 ]; then
  echo "  scan: greedy probe over $BASELINE_RUN_LIMIT cases from shard $PROBE_SHARD"
else
  echo "  scan: exact requested range"
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
echo "  rust harness gap: $harness_gap"

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
  recommendation="$(awk -F'\t' '$3 > 0 { print; exit }' "$RECOMMENDATIONS_TSV")"
  if [ -z "$recommendation" ]; then
    recommendation="$(sed -n '1p' "$RECOMMENDATIONS_TSV")"
  fi
  if [ -n "$recommendation" ]; then
    IFS=$'\t' read -r _score rec_total rec_engine rec_fail rec_harness rec_area <<<"$recommendation"
    rec_timeout=$((rec_engine - rec_fail))
    echo
    echo "Greedy recommendation:"
    echo "  area: $rec_area"
    echo "  gaps: $rec_total"
    echo "  engine gaps: $rec_engine"
    echo "  rust fail: $rec_fail"
    echo "  rust timeout: $rec_timeout"
    echo "  harness gaps: $rec_harness"
    echo "  rerun: ./scripts/find-qjsng-gaps.sh --filter $rec_area --all"
    if [ "$PROBE_SCAN" -eq 1 ]; then
      echo "  note: recommendation is from a greedy probe; use --exact --all for a complete audit."
    fi
    echo
    echo "Recommended cases:"
    awk -F'\t' -v area="$rec_area" '$4 == area {
      label = $2
      if ($3 != "") {
        label = label ", " $3
      }
      printf "  %s  (%s)\n", $1, label
    }' "$GAPS_TSV" \
      | head -n "$TOP_COUNT"
  fi
fi

echo
echo "Raw files:"
echo "  summary: $SUMMARY_JSON"
echo "  cases: $CASES_JSONL"
echo "  gaps: $GAPS_TSV"
echo "  excluded timeouts: $TIMEOUTS_TSV"
echo "  recommendations: $RECOMMENDATIONS_TSV"
echo "  baseline log: $BASELINE_LOG"
