"""Canonical non-timing instruction attribution and relative-PC family audit."""
import hashlib, json, os, statistics, subprocess, sys
from pathlib import Path
from tools.benchmark import screen as s
OUT = Path(os.environ['BENCH_OUT'])
assert 'no-timing' in (OUT.parent/'status').read_text()
sha = lambda path: hashlib.sha256(Path(path).read_bytes()).hexdigest()
revs = {'base': 'f58c696e8379261ed726f0cea1051ed6d34f431a',
        'parent': '8f898dd96239d392c168bdd20049d073b3b47ff4',
        'candidate': sys.argv[1]}
assert subprocess.check_output(['git','rev-parse','HEAD'], text=True).strip() == revs['candidate']
paths = {}
for role, rev in revs.items():
    code = 'set -euo pipefail\n. scripts/lib.sh\nc="$(qjs_resolve_cargo)"\nqjs_build_revision "$PWD" "$c" "$1"\n'
    p = subprocess.run(['bash','-c',code,'--',rev],capture_output=True,text=True,check=True)
    (OUT/(role+'-build.log')).write_text(p.stderr)
    paths[role] = Path(p.stdout.strip())
    print('BUILT',role,sha(paths[role]),flush=True)
work=OUT/'work';work.mkdir()
ids=['external/sunspider-1.0/string-unpack-code','external/jetstream3-js-subset/raytrace-public-class-fields']
cases=s.resolve_cases(ids,s.DEFAULT_CACHE,work)
commands={'empty':['--raw','-e','0;'],'cli_only':['--help']}
commands.update({c.id:['--raw',str(c.workload)] for c in cases})
tool=s.counter_tool(trust_cycles=False);rows={}
for role,binary in paths.items():
    rows[role]={}
    for case,args in commands.items():
        subprocess.run([str(binary),*args],stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL,check=True,timeout=180)
        rows[role][case]=[s.sample(tool,[str(binary),*args],180).instructions for _ in range(3)]
    print('COUNTS',role,{k:statistics.median(v) for k,v in rows[role].items()},flush=True)
result={'diagnostic_only':True,'timing':False,'acceptance':False,'attempts_consumed':0,
        'revisions':revs,'images':{r:{'path':str(p),'sha256':sha(p)} for r,p in paths.items()},
        'sources':{str(p):sha(p) for p in Path('crates').rglob('*.rs')},
        'commands':commands,'instructions':rows,'medians':{r:{c:statistics.median(v) for c,v in d.items()} for r,d in rows.items()}}
(OUT/'probe.json').write_text(json.dumps(result,indent=2)+'\n')
# Existing static decoder walks actual emitted jump tables and finite numeric paths.
audit=Path('tasks/diagnostics/t056-header/audit.py')
with (OUT/'pc-audit.log').open('w') as log:
    subprocess.run([sys.executable,str(audit),'--base',str(paths['parent']),'--candidate',str(paths['candidate']),'--output-dir',str(OUT/'pc-audit')],stdout=log,stderr=subprocess.STDOUT,check=True)
report=json.loads((OUT/'pc-audit/byte-audit.json').read_text())
family={}
for path,left in report['records']['base']['paths'].items():
    right=report['records']['candidate']['paths'][path]
    family[path]={'instruction_counts':[len(left),len(right)],'relative_pcs_equal':[r['offset'] for r in left]==[r['offset'] for r in right],
                  'parent_offsets':[r['offset'] for r in left],'candidate_offsets':[r['offset'] for r in right]}
# Include every decoded numeric path, not only audit.py's local-read subset.
for frame in ('WideLoopFrame','vm::Vm'):
    old=json.loads((OUT/'pc-audit/base-path.hot-paths.json').read_text())['bodies'][frame]
    new=json.loads((OUT/'pc-audit/final-path.hot-paths.json').read_text())['bodies'][frame]
    family[frame]={'dispatch_offsets':[old['dispatch'],new['dispatch']], 'paths':{k:{'relative_pcs_equal':[r['offset'] for r in p['rows']]==[r['offset'] for r in new['paths'][k]['rows']], 'instruction_counts':[p['instructions'],new['paths'][k]['instructions']], 'parent_offsets':[r['offset'] for r in p['rows']], 'candidate_offsets':[r['offset'] for r in new['paths'][k]['rows']]} for k,p in old['paths'].items()}}
(OUT/'relative-pcs.json').write_text(json.dumps({'images':result['images'],'normalized_paths':report['comparison'],'relative_paths':family,'scope':'static numeric paths, no timing or hardware-cause inference'},indent=2)+'\n')
print('PC_AUDIT',report['all_equivalent'],flush=True)
