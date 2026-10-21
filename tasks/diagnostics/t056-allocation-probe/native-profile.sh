set -euo pipefail
export PYTHONDONTWRITEBYTECODE=1
python3 - <<'PY'
import os,subprocess,selectors,json,gzip,re,hashlib,time,collections
from pathlib import Path
from tools.benchmark.screen import resolve_cases,DEFAULT_CACHE
out=Path(os.environ['BENCH_OUT']);work=out/'work';work.mkdir()
case=resolve_cases(['external/sunspider-1.0/string-unpack-code'],DEFAULT_CACHE,work)[0]
sources={'empty':'1+2','unpack':case.workload.read_text(),'string_slice':'scriptArgs=["allocation-profile","string_slice","100000"];\n'+Path('benchmarks/workloads/broad-micro.js').read_text()}
roles={'base':'f58c696e8379261ed726f0cea1051ed6d34f431a','candidate':'8f7d97c71b34e2bec52e2c6df87f4ba4ac51641a'}
results={}
for role,rev in roles.items():
 binary=Path('target/perf-loop/compiler-fat16-abort-v1')/('rev-'+rev)/'qjs';binary=binary.resolve()
 for name,source in sources.items():
  tag=role+'-'+name;env={**os.environ,'MallocStackLoggingNoCompact':'1'}
  for key in ['MallocStackLogging','MallocNanoZone','QJS_BANK_WINDOW','T056_ENV_COOKIE']:env.pop(key,None)
  with (out/(tag+'.stderr')).open('w') as err:
   p=subprocess.Popen([str(binary),'--raw','--interactive'],stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=err,env=env)
   try:
    payload=('eval('+json.dumps(source)+'); "T056_ALLOCATION_DONE"\n').encode();p.stdin.write(payload);p.stdin.flush()
    select=selectors.DefaultSelector();select.register(p.stdout,selectors.EVENT_READ);buf=b'';deadline=time.monotonic()+180
    while b'T056_ALLOCATION_DONE\n' not in buf:
     assert select.select(max(0,deadline-time.monotonic())),('workload timeout',tag,buf[-500:])
     chunk=os.read(p.stdout.fileno(),65536);assert chunk,('child EOF',tag,p.poll(),buf[-500:]);buf+=chunk
    (out/(tag+'.stdout')).write_bytes(buf)
    assert b'error:' not in buf,buf[-1000:]
    counts=collections.Counter();stacks=collections.Counter();frees=0;events=0
    with gzip.open(out/(tag+'.events.txt.gz'),'wt') as raw:
     q=subprocess.Popen(['/usr/bin/malloc_history',str(p.pid),'-allEvents','-noContent'],stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True)
     for line in q.stdout:
      raw.write(line);events+=1
      m=re.match(r'ALLOC 0x[0-9a-f]+-0x[0-9a-f]+ \[size=(\d+)\]: (.*)',line)
      if m:
       size=int(m[1]);counts[size]+=1
       frames=re.findall(r'\(qjs\) ([^|]+)',m[2]);key=tuple(x.strip() for x in frames)
       stacks[(size,key)]+=1
      elif line.startswith('FREE '):frees+=1
     assert q.wait(timeout=30)==0,(tag,'malloc_history failed')
    r={'revision':rev,'binary_sha256':hashlib.sha256(binary.read_bytes()).hexdigest(),'source_sha256':hashlib.sha256(source.encode()).hexdigest(),'scope':'native MallocStackLoggingNoCompact, original canonical binary under one-line REPL eval wrapper; allocation requests, no timing','alloc_count':sum(counts.values()),'free_count':frees,'event_rows':events,'requested_bytes':sum(k*v for k,v in counts.items()),'request_counts':dict(sorted(counts.items())),'rounded16_counts':dict(sorted(collections.Counter({}).items())),'top_stacks':[]}
    rounded=collections.Counter()
    for size,count in counts.items():rounded[(size+15)//16*16]+=count
    r['rounded16_counts']=dict(sorted(rounded.items()))
    # Keep every grouped stack to identify changed requests, not just a top-N view.
    r['stacks']=[{'size':size,'count':count,'frames':list(frames)} for (size,frames),count in stacks.most_common()]
    r['raw_sha256']=hashlib.sha256((out/(tag+'.events.txt.gz')).read_bytes()).hexdigest()
    (out/(tag+'.json')).write_text(json.dumps(r,indent=2)+'\n');results[tag]={k:v for k,v in r.items() if k not in ['stacks','top_stacks']}
    print(tag,'allocs',r['alloc_count'],'bytes',r['requested_bytes'],'128',counts[128],'136',counts[136],flush=True)
   finally:
    p.stdin.close();p.wait(timeout=30)
(out/'allocation-summary.json').write_text(json.dumps(results,indent=2)+'\n')
PY
