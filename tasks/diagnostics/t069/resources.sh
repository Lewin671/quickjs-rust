#!/usr/bin/env bash
# T069 resource lanes using T067's measuring-host manifest. Run only through
# imac-bench.sh run --timing; the runner must have the frozen plan base ref.
set -euo pipefail
export PYTHONDONTWRITEBYTECODE=1 PYTHONPATH="$PWD"
. scripts/lib.sh
cargo_bin="$(qjs_resolve_cargo)"
base_sha="$(python3 -c 'import json; print(json.load(open("tasks/performance-units/positional-call-activation.json"))["base_sha"])')"
candidate_sha="$(git rev-parse HEAD)"
manifest="$(python3 -m tools.benchmark.host_manifest --resources)"
base="$(qjs_build_revision "$PWD" "$cargo_bin" "$base_sha")"
candidate="$(qjs_build_revision "$PWD" "$cargo_bin" "$candidate_sha")"
python3 -m tools.benchmark.local_receipt --identity qjs-rust --revision "$base_sha" --binary "$base" --manifest "$manifest" --output "$BENCH_OUT/base-receipt.json"
python3 -m tools.benchmark.local_receipt --identity qjs-rust --revision "$candidate_sha" --binary "$candidate" --manifest "$manifest" --output "$BENCH_OUT/candidate-receipt.json"
ng_sha="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["reference_engine"]["revision"])' "$manifest")"
python3 -m tools.benchmark.local_receipt --identity quickjs-ng --revision "$ng_sha" --binary third_party/quickjs-ng/build/qjs --manifest "$manifest" --output "$BENCH_OUT/quickjs-ng-receipt.json"
for lane in size fresh rss; do
  ./scripts/resource-benchmark.sh --lane "$lane" --manifest "$manifest" --candidate "$candidate" --candidate-receipt "$BENCH_OUT/candidate-receipt.json" --base "$base" --base-receipt "$BENCH_OUT/base-receipt.json" --quickjs-ng third_party/quickjs-ng/build/qjs --quickjs-ng-receipt "$BENCH_OUT/quickjs-ng-receipt.json" --output "$BENCH_OUT/$lane.jsonl"
  ./scripts/resource-benchmark-report.sh --manifest "$manifest" --input "$BENCH_OUT/$lane.jsonl" --output "$BENCH_OUT/$lane-report.json"
done
