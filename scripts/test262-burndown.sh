#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TEST262_DIR="$ROOT_DIR/third_party/test262"
BURNDOWN_FILE="$ROOT_DIR/docs/conformance/burndown.jsonl"

REPORT_DIR=""
ENTRY_FILE=""
COMMIT=""
SOURCE_LABEL="local-exact"
RECORDED=""
DRY_RUN=0

usage() {
  cat >&2 <<'USAGE'
usage: scripts/test262-burndown.sh --report DIR [--commit SHA] [--source LABEL] [--date YYYY-MM-DD] [--dry-run]
       scripts/test262-burndown.sh --entry FILE [--dry-run]

Appends one conformance burndown entry to docs/conformance/burndown.jsonl.

--report DIR reads both-engine shard summaries (summary*.json) produced by
full-coverage runs such as:
  scripts/test262-baseline.sh --all --engine both --shard I/N --summary-json ...
  scripts/find-qjsng-gaps.sh --exact --all
Every summary must be engine=both, limit=all, unfiltered; partial probes are
rejected so the time series only contains complete scans.

--entry FILE appends an already-built schema-1 entry, such as the
test262-burndown artifact written by the Test262 Coverage CI workflow.
USAGE
}

die() {
  echo "error: $*" >&2
  exit 1
}

while [ "$#" -gt 0 ]; do
  case "$1" in
    --report)
      [ "$#" -ge 2 ] || { usage; exit 2; }
      REPORT_DIR="$2"
      shift 2
      ;;
    --entry)
      [ "$#" -ge 2 ] || { usage; exit 2; }
      ENTRY_FILE="$2"
      shift 2
      ;;
    --commit)
      [ "$#" -ge 2 ] || { usage; exit 2; }
      COMMIT="$2"
      shift 2
      ;;
    --source)
      [ "$#" -ge 2 ] || { usage; exit 2; }
      SOURCE_LABEL="$2"
      shift 2
      ;;
    --date)
      [ "$#" -ge 2 ] || { usage; exit 2; }
      RECORDED="$2"
      shift 2
      ;;
    --dry-run)
      DRY_RUN=1
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

append_entry() {
  local entry="$1"
  local commit
  commit="$(printf '%s' "$entry" | sed -n 's/.*"commit":"\([^"]*\)".*/\1/p')"
  [ -n "$commit" ] || die "entry has no commit field"
  if [ -f "$BURNDOWN_FILE" ] && grep -F "\"commit\":\"$commit\"" "$BURNDOWN_FILE" >/dev/null; then
    die "burndown already has an entry for commit $commit; remove the stale line first if this is a re-record"
  fi
  if [ "$DRY_RUN" -eq 1 ]; then
    printf '%s\n' "$entry"
    return
  fi
  mkdir -p "$(dirname "$BURNDOWN_FILE")"
  printf '%s\n' "$entry" >>"$BURNDOWN_FILE"
  echo "recorded burndown entry for $commit in ${BURNDOWN_FILE#"$ROOT_DIR/"}"
}

if [ -n "$ENTRY_FILE" ]; then
  [ -f "$ENTRY_FILE" ] || die "missing entry file: $ENTRY_FILE"
  [ "$(wc -l <"$ENTRY_FILE" | tr -d ' ')" -le 1 ] || die "entry file must contain a single JSON line"
  entry="$(tr -d '\n' <"$ENTRY_FILE")"
  for key in '"schema":1' '"recorded":' '"commit":' '"source":' '"configured":' '"actionable_gap":'; do
    case "$entry" in
      *"$key"*) ;;
      *) die "entry file is missing $key" ;;
    esac
  done
  append_entry "$entry"
  exit 0
fi

[ -n "$REPORT_DIR" ] || { usage; exit 2; }
[ -d "$REPORT_DIR" ] || die "missing report directory: $REPORT_DIR"
summaries=("$REPORT_DIR"/summary*.json)
[ -f "${summaries[0]}" ] || die "no summary*.json files in $REPORT_DIR"

for f in "${summaries[@]}"; do
  grep -q '"engine": "both"' "$f" || die "$f is not an --engine both summary"
  grep -q '"limit": "all"' "$f" || die "$f is not an --all summary; burndown only records complete scans"
  grep -q '"filter": ""' "$f" || die "$f is a filtered summary; burndown only records unfiltered scans"
done

sums="$(awk '
  /^  "total":/ { total += $2 + 0 }
  /^  "rust_not_run":/ { not_run += $2 + 0 }
  /^    "total":/ { ng_skip += $2 + 0 }
  /^  "quickjs_rust":/ {
    n = split($0, a, /[^0-9]+/)
    rust_pass += a[2]; rust_fail += a[3]; rust_timeout += a[4]
  }
  /^  "quickjs_ng":/ {
    n = split($0, a, /[^0-9]+/)
    ng_pass += a[2]; ng_fail += a[3]; ng_timeout += a[4]
  }
  /^    "both_pass":/ { both_pass += $2 + 0 }
  /^    "quickjs_ng_pass_rust_fail":/ { gap_fail += $2 + 0 }
  /^    "quickjs_ng_pass_rust_timeout":/ { gap_timeout += $2 + 0 }
  /^    "quickjs_ng_pass_rust_not_run":/ { gap_not_run += $2 + 0 }
  /^    "rust_pass_quickjs_ng_nonpass":/ { rust_only += $2 + 0 }
  END {
    printf "total=%d ng_skip=%d not_run=%d rust_pass=%d rust_fail=%d rust_timeout=%d ", \
      total, ng_skip, not_run, rust_pass, rust_fail, rust_timeout
    printf "ng_pass=%d ng_fail=%d ng_timeout=%d both_pass=%d ", \
      ng_pass, ng_fail, ng_timeout, both_pass
    printf "gap_fail=%d gap_timeout=%d gap_not_run=%d rust_only=%d\n", \
      gap_fail, gap_timeout, gap_not_run, rust_only
  }
' "${summaries[@]}")"
eval "$sums"

upstream_total="$(find "$TEST262_DIR/test" -type f -name '*.js' | wc -l | tr -d ' ')"
[ "$total" -eq "$upstream_total" ] || \
  die "summaries cover $total cases but the pinned Test262 checkout has $upstream_total; record only complete shard sets"

configured=$((total - ng_skip))
rust_accounted=$((rust_pass + rust_fail + rust_timeout + not_run))
[ "$rust_accounted" -eq "$configured" ] || \
  die "inconsistent summaries: rust results account for $rust_accounted of $configured configured cases"
actionable=$((gap_fail + gap_timeout))

[ -n "$COMMIT" ] || COMMIT="$(git -C "$ROOT_DIR" rev-parse HEAD)"
[ -n "$RECORDED" ] || RECORDED="$(date -u +%Y-%m-%d)"

entry="$(printf '{"schema":1,"recorded":"%s","commit":"%s","source":"%s","total":%d,"ng_config_skipped":%d,"configured":%d,"rust":{"pass":%d,"fail":%d,"timeout":%d,"not_run":%d},"ng":{"pass":%d,"fail":%d,"timeout":%d},"comparison":{"both_pass":%d,"actionable_gap":%d,"ng_pass_rust_fail":%d,"ng_pass_rust_timeout":%d,"ng_pass_rust_not_run":%d,"rust_pass_ng_nonpass":%d}}' \
  "$RECORDED" "$COMMIT" "$SOURCE_LABEL" \
  "$total" "$ng_skip" "$configured" \
  "$rust_pass" "$rust_fail" "$rust_timeout" "$not_run" \
  "$ng_pass" "$ng_fail" "$ng_timeout" \
  "$both_pass" "$actionable" "$gap_fail" "$gap_timeout" "$gap_not_run" "$rust_only")"

append_entry "$entry"
