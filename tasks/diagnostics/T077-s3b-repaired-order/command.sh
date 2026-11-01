python3 - <<'PY_ORDER'
import datetime,json,os,signal,subprocess,hashlib
from pathlib import Path
from zoneinfo import ZoneInfo
out=Path(os.environ["BENCH_OUT"])
now=datetime.datetime.now(ZoneInfo("Asia/Shanghai"))
if 135 <= now.hour*60+now.minute < 300:
    raise SystemExit("CST timing blackout proximity: no profiles started")
cutoff=now.replace(hour=2,minute=45,second=0,microsecond=0)
if cutoff<=now:cutoff+=datetime.timedelta(days=1)
limit=min(7200,int((cutoff-now).total_seconds())-5)
binary=Path(subprocess.check_output(["python3","-m","tools.benchmark.build_cache","build-rust","--source",str(Path.cwd()),"--target-dir",str(out/"build")],text=True).strip())
command=["python3","-m","tools.benchmark.order_file","--binary",str(binary),"--output",str(out/"hot-functions-x86_64.order")]
metadata={"source":"1e91be92c2af7b876efdc68f7d8698820b44dfe1","base":"3cc12c6f","scope":"default external/sentinel/broad; no development or held-out","command":command,"binary_sha256":hashlib.sha256(binary.read_bytes()).hexdigest(),"producer_sha256":hashlib.sha256(Path("tools/benchmark/order_file.py").read_bytes()).hexdigest(),"started_cst":datetime.datetime.now(ZoneInfo("Asia/Shanghai")).isoformat()}
(out/"order-request.json").write_text(json.dumps(metadata,indent=2)+"\n")
process=subprocess.Popen(command,start_new_session=True)
try:status=process.wait(timeout=limit)
except subprocess.TimeoutExpired:
    os.killpg(process.pid,signal.SIGTERM)
    try:process.wait(timeout=3)
    except subprocess.TimeoutExpired:os.killpg(process.pid,signal.SIGKILL)
    raise SystemExit("own order profiling stopped before blackout")
metadata["exit"]=status
if status==0:metadata["order_sha256"]=hashlib.sha256((out/"hot-functions-x86_64.order").read_bytes()).hexdigest()
(out/"order-result.json").write_text(json.dumps(metadata,indent=2)+"\n")
raise SystemExit(status)
PY_ORDER

