export BASE_SHA=8322fc98 CAND_SHA=bc17ccea4392a5392fd3d713129806df18f379e6
# Quick triage of all 86 v5-policy cases: BASE_SHA vs CAND_SHA (env). Diagnostic only.
set -euo pipefail
export PYTHONDONTWRITEBYTECODE=1
python3 - <<'PY'
import json, os, subprocess
from pathlib import Path
out = Path(os.environ['BENCH_OUT'])
policy = json.load(open('benchmarks/policies/imac-x86_64-rust1.95-fat16-layout-v5.json'))
ids = list(policy['cases']); assert len(ids) == 86
a = ['./scripts/quick-triage.sh', '--base', os.environ['BASE_SHA'], '--candidate', os.environ['CAND_SHA']]
for i in ids: a += ['--target', i]
a += ['--output', str(out / 'triage')]
subprocess.run(a, check=True)
PY

