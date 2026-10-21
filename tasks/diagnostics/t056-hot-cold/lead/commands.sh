#!/usr/bin/env bash
# Lead-only prospective chain. Current image fails before submitting any job.
set -euo pipefail
BUNDLE=tasks/diagnostics/t056-hot-cold
RUNNER=/Users/qingyingliu/Code/co-work-space/scripts/imac-bench.sh
CAND=5dd3a810cac6a88f6a1f86c0ef46dc68dfac47ee
BASE=f58c696e8379261ed726f0cea1051ed6d34f431a
PYTHONDONTWRITEBYTECODE=1 python3 - <<'GUARD'
import sys
sys.path.insert(0, 'tasks/diagnostics/t056-hot-cold/lead')
from common import fast_family
fast_family()
GUARD
finish() {
  local result=0
  "$RUNNER" wait "$1" || result=$?
  "$RUNNER" fetch "$1"
  return "$result"
}
python3 "$BUNDLE/lead/validate-local.py"
python3 "$BUNDLE/panel/validate.py"
CORRECT=$("$RUNNER" run --repo "$PWD" --ref "$CAND" --with "$BASE" \
  --name t056-hot-cold-correctness -- bash -c "$(python3 "$BUNDLE/lead/render.py" 01)")
finish "$CORRECT"
PREGATE=$("$RUNNER" run --repo "$PWD" --ref "$CAND" --with "$BASE" \
  --name t056-hot-cold-pregate86 -- bash -c "$(python3 "$BUNDLE/lead/render.py" 02)" -- "$CORRECT")
finish "$PREGATE"
TRIAGE=$("$RUNNER" run --repo "$PWD" --ref "$CAND" --with "$BASE" --timing \
  --name t056-hot-cold-triage -- bash -c "$(python3 "$BUNDLE/lead/render.py" 03)" -- "$CORRECT" "$PREGATE")
finish "$TRIAGE"
MINI=$("$RUNNER" run --repo "$PWD" --ref "$CAND" --with "$BASE" --timing \
  --name t056-hot-cold-mini -- bash -c "$(python3 "$BUNDLE/panel/render.py" mini)" -- "$CORRECT" "$PREGATE" "$TRIAGE")
watch_status=0
python3 "$BUNDLE/panel/watch.py" "$MINI" || watch_status=$?
"$RUNNER" fetch "$MINI"
test "$watch_status" -eq 0
