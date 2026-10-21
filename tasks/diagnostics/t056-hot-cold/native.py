"""Native request/class histogram of canonical images; never timing evidence."""
import collections, gzip, hashlib, json, os, re, selectors, subprocess, time
from pathlib import Path
OUT=Path(os.environ['BENCH_OUT']);assert 'no-timing' in (OUT.parent/'status').read_text()
probe=json.loads((OUT/'probe.json').read_text());sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest()
source=Path(probe['commands']['external/sunspider-1.0/string-unpack-code'][1]).read_text()
results={}
for role,meta in probe['images'].items():
    binary=Path(meta['path']);assert sha(binary)==meta['sha256']
    for case,text in [('empty','0;'),('unpack',source)]:
        tag=role+'-'+case;env=dict(os.environ)
        for k in ['MallocStackLogging','MallocNanoZone','QJS_TL_TRACE','T056_ENV_COOKIE']:env.pop(k,None)
        env['MallocStackLoggingNoCompact']='1'
        with (OUT/(tag+'.stderr')).open('w') as err:
            p=subprocess.Popen([str(binary),'--raw','--interactive'],stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=err,env=env)
            try:
                p.stdin.write(('eval('+json.dumps(text)+'); "T056_NATIVE_DONE"\n').encode());p.stdin.flush()
                select=selectors.DefaultSelector();select.register(p.stdout,selectors.EVENT_READ);buf=b'';deadline=time.monotonic()+180
                while b'T056_NATIVE_DONE\n' not in buf:
                    assert select.select(max(0,deadline-time.monotonic())),tag
                    chunk=os.read(p.stdout.fileno(),65536);assert chunk,(tag,p.poll(),buf[-500:]);buf+=chunk
                assert b'error:' not in buf,buf[-1000:]
                (OUT/(tag+'.stdout')).write_bytes(buf)
                counts=collections.Counter();stacks=collections.Counter();frees=0
                with gzip.open(OUT/(tag+'.events.txt.gz'),'wt') as raw:
                    q=subprocess.Popen(['/usr/bin/malloc_history',str(p.pid),'-allEvents','-noContent'],stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True)
                    for line in q.stdout:
                        raw.write(line)
                        m=re.match(r'ALLOC 0x[0-9a-f]+-0x[0-9a-f]+ \[size=(\d+)\]: (.*)',line)
                        if m:
                            n=int(m[1]);counts[n]+=1
                            frames=tuple(x.strip() for x in re.findall(r'\(qjs\) ([^|]+)',m[2]));stacks[n,frames]+=1
                        elif line.startswith('FREE '):frees+=1
                    assert q.wait(timeout=30)==0
                result={'revision':probe['revisions'][role],'binary_sha256':sha(binary),'source_sha256':hashlib.sha256(text.encode()).hexdigest(),
                        'alloc_count':sum(counts.values()),'free_count':frees,'requested_bytes':sum(n*c for n,c in counts.items()),
                        'request_counts':dict(sorted(counts.items())), 'raw_sha256':sha(OUT/(tag+'.events.txt.gz'))}
                stackrows=[{'size':n,'count':c,'frames':list(f)} for (n,f),c in stacks.most_common()]
                (OUT/(tag+'.stacks.json')).write_text(json.dumps(stackrows,indent=2)+'\n')
                result['stacks_sha256']=sha(OUT/(tag+'.stacks.json'));results[tag]=result
                print('NATIVE',tag,result['alloc_count'],result['requested_bytes'],flush=True)
            finally:
                p.stdin.close();p.wait(timeout=30)
# Query the actual host API for every observed request, including large ones.
c=OUT/'classes.c';c.write_text('#include <malloc/malloc.h>\n#include <stdio.h>\nint main(void){size_t n;while(scanf("%zu",&n)==1)printf("%zu %zu\\n",n,malloc_good_size(n));return 0;}\n')
subprocess.run(['cc','-O2',str(c),'-o',str(OUT/'classes')],check=True)
sizes=sorted({int(n) for r in results.values() for n in r['request_counts']})
q=subprocess.run([str(OUT/'classes')],input='\n'.join(map(str,sizes))+'\n',capture_output=True,text=True,check=True)
classes={int(n):int(k) for n,k in map(str.split,q.stdout.splitlines())};assert set(classes)==set(sizes)
for result in results.values():
    hist=collections.Counter()
    for n,count in result['request_counts'].items():hist[classes[int(n)]]+=count
    result['class_counts']=dict(sorted(hist.items()));result['class_bytes']=sum(n*c for n,c in hist.items())
(OUT/'native-summary.json').write_text(json.dumps({'diagnostic_only':True,'timing':False,'scope':'canonical images, MallocStackLoggingNoCompact REPL eval wrapper; every ALLOC including realloc/free history; malloc_good_size by actual host, not rounded inference; no live/peak or latency claim','host':subprocess.check_output(['sw_vers'],text=True),'malloc_good_size':classes,'results':results},indent=2)+'\n')
