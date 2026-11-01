python3 - <<'PY_PUBLIC86'
import datetime,json,os,signal,subprocess
from zoneinfo import ZoneInfo
from pathlib import Path
now=datetime.datetime.now(ZoneInfo("Asia/Shanghai"))
minutes=now.hour*60+now.minute
if 135 <= minutes < 300:
    raise SystemExit("iMac timing blackout proximity: no measurements started")
cutoff=now.replace(hour=2,minute=45,second=0,microsecond=0)
if cutoff <= now: cutoff += datetime.timedelta(days=1)
seconds=min(7200,int((cutoff-now).total_seconds())-5)
policy=json.loads(Path("benchmarks/policies/imac-x86_64-rust1.95-fat16-layout-v5.json").read_text())
targets=sorted(policy["cases"])
assert len(targets)==86
argv=["./scripts/quick-triage.sh","--base","3cc12c6f8d93ffd6eae91d1bd44efe74134ae195","--candidate","73c93d8feb3b0e46b8aee581cf293ed2a6d953c8","--output",str(Path(os.environ["BENCH_OUT"])/"triage")]
for target in targets: argv += ["--target",target]
process=subprocess.Popen(argv,start_new_session=True)
try:
    raise SystemExit(process.wait(timeout=seconds))
except subprocess.TimeoutExpired:
    os.killpg(process.pid,signal.SIGTERM)
    try: process.wait(timeout=3)
    except subprocess.TimeoutExpired: os.killpg(process.pid,signal.SIGKILL)
    raise SystemExit("own triage stopped before timing deadline")
PY_PUBLIC86

