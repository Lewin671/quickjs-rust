python3 - <<'PY_IDENTITY'
import json,os,platform
from pathlib import Path
from tools.benchmark.layout_identity import host_identity
policy=json.loads(Path('benchmarks/policies/imac-x86_64-rust1.95-fat16-layout-v5.json').read_text())
actual=host_identity();expected=policy['host']
diff={key:{'expected':expected.get(key),'actual':actual.get(key)} for key in set(expected)|set(actual) if expected.get(key)!=actual.get(key)}
row={'source':'73c93d8f','expected_node':policy['host_node'],'actual_node':platform.node(),'differences':diff,'actual':actual}
Path(os.environ['BENCH_OUT'],'identity.json').write_text(json.dumps(row,indent=2))
print(json.dumps({'expected_node':row['expected_node'],'actual_node':row['actual_node'],'differences':diff},indent=2))
PY_IDENTITY

