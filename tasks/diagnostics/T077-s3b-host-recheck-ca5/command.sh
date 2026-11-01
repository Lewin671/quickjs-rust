python3 - <<'PY_IDENTITY'
import json,os,platform,hashlib
from datetime import datetime,timezone
from zoneinfo import ZoneInfo
from pathlib import Path
from tools.benchmark.layout_identity import host_identity
policy=json.loads(Path('benchmarks/policies/imac-x86_64-rust1.95-fat16-layout-v5.json').read_text())
assert hashlib.sha256(Path('benchmarks/policies/imac-x86_64-rust1.95-fat16-layout-v5.json').read_bytes()).hexdigest()=='f3e8ef40b8b45601b5b86edaf4b47c2d9d904d0045eb024e837f1c447a52bec4'
actual=host_identity();expected=policy['host']
diff={key:{'expected':expected.get(key),'actual':actual.get(key)} for key in set(expected)|set(actual) if expected.get(key)!=actual.get(key)}
row={'source':'ca5ab643c2de99d7268df7022c3370596f7b8b41','expected_node':policy['host_node'],'actual_node':platform.node(),'differences':diff,'actual':actual,'checked_utc':datetime.now(timezone.utc).isoformat(),'checked_cst':datetime.now(ZoneInfo('Asia/Shanghai')).isoformat()}
Path(os.environ['BENCH_OUT'],'identity.json').write_text(json.dumps(row,indent=2))
print(json.dumps({'expected_node':row['expected_node'],'actual_node':row['actual_node'],'differences':diff},indent=2))
PY_IDENTITY

