#!/usr/bin/env bash
# T066 promotion decision (non-timing). Inputs: selection formal 20261022T041140Z (queue), formal 20261022T182921Z,
# screen attempt 2 panel 20261022T172940Z, held-out rerun 20261022T211111Z, CI Test262 Coverage run 39832778267 (commit 9420a6a5).
set -euo pipefail
Q="$BENCH_QUEUE"
SEL="$Q/20261022T041140Z-current-main-9eba7a1d-formal/out/bundle"
F="$Q/20261022T182921Z-t066-formal60/out/bundle"
test "$(cat "$Q/20261022T211111Z-t066-heldout30-r2/status")" = "done 0"
if grep -q DISTURBED "$Q/20261022T211111Z-t066-heldout30-r2/log"; then echo "held-out rerun DISTURBED" >&2; exit 2; fi
./scripts/performance-decision.sh queue --summary "$SEL/summary.json" --broad-report "$SEL/report.json" \
  --sentinel-report "$SEL/sentinel-report.json" --external-report "$SEL/external-report.json" --output "$BENCH_OUT/queue.json"
test "$(shasum -a 256 "$BENCH_OUT/queue.json" | cut -c1-64)" = f6666a68cb8e1841418d6f5012e6f7810b0c25ebf982b45ef6d5564855e0ae13
cat > "$BENCH_OUT/burndown.json" <<'JSON'
{"schema":1,"recorded":"2026-10-22","commit":"9420a6a5771fd42a157b72ac5c3980adfd2b5e16","source":"ci-aggregate","total":53572,"ng_config_skipped":10900,"configured":42672,"rust":{"pass":42672,"fail":0,"timeout":0,"not_run":0},"ng":{"pass":42602,"fail":70,"timeout":0},"comparison":{"both_pass":42602,"actionable_gap":0,"ng_pass_rust_fail":0,"ng_pass_rust_timeout":0,"ng_pass_rust_not_run":0,"rust_pass_ng_nonpass":70}}
JSON
./scripts/performance-decision.sh decide --mode promotion \
  --unit tasks/performance-units/cli-size-class-allocator.json --queue "$BENCH_OUT/queue.json" --profile-root "$Q" \
  --summary "$F/summary.json" --broad-report "$F/report.json" --sentinel-report "$F/sentinel-report.json" \
  --external-report "$F/external-report.json" \
  --layout-panel "$Q/20261022T172940Z-t066-screen-attempt2/out/attempt/screen/complete.json" \
  --held-out-panel "$Q/20261022T211111Z-t066-heldout30-r2/out/heldout/panel.json" \
  --test262-burndown "$BENCH_OUT/burndown.json" \
  ${RESOURCE_REPORT:+--resource-report "$RESOURCE_REPORT"} \
  ${RESOURCE_RAW:+--resource-raw "$RESOURCE_RAW"} \
  --output "$BENCH_OUT/decision.json" || true
cat "$BENCH_OUT/decision.json"
