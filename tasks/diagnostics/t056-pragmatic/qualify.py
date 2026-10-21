"""Owner-authorized non-timing canonical instruction snapshot, never acceptance."""
import hashlib,json,os,statistics,subprocess,sys
from pathlib import Path
from tools.benchmark import screen as s
from tools.benchmark.layout_identity import host_identity
out=Path(os.environ['BENCH_OUT']);assert 'no-timing' in (out.parent/'status').read_text()
BASE='f58c696e8379261ed726f0cea1051ed6d34f431a';BASE_HASH='b0dfc65d0c026d62aa64ef3d69a089eb73b51088c84af75b44617ea2a1737f0e'
CANDIDATE=sys.argv[1];assert subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip()==CANDIDATE
sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest()
paths={}
for name,revision in [('base',BASE),('candidate',CANDIDATE),('aligned','8f7d97c71b34e2bec52e2c6df87f4ba4ac51641a'),('previous','dfff3bb6bf3db01fb52a7612ed1144900b5f08c5')]:
 script='set -euo pipefail\n. scripts/lib.sh\nc="$(qjs_resolve_cargo)"\nqjs_build_revision "$PWD" "$c" "$1"\n'
 p=subprocess.run(['bash','-c',script,'--',revision],text=True,capture_output=True,check=True)
 (out/(name+'-build.log')).write_text(p.stderr);paths[name]=Path(p.stdout.strip());assert paths[name].is_file()
assert sha(paths['base'])==BASE_HASH
images={'candidate':CANDIDATE,'base':BASE,'binary_sha256':{n:sha(p) for n,p in paths.items()},'paths':{n:str(p) for n,p in paths.items()},'host':host_identity(),'source_sha256':{str(p):sha(p) for p in Path('crates').rglob('*.rs')}}
(out/'images.json').write_text(json.dumps(images,indent=2)+'\n')
policy=json.loads(Path('benchmarks/policies/imac-x86_64-rust1.95-fat16-layout-v6.json').read_text())
args=[sys.executable,'-m','tools.benchmark.screen','--candidate',str(paths['candidate']),'--base',str(paths['base']),'--pairs','1','--symbols','40','--json',str(out/'snapshot86.json')]
for name in policy['cases']:args+=['--case',name]
with (out/'snapshot86.txt').open('w') as log,(out/'snapshot86.err').open('w') as err:subprocess.run(args,stdout=log,stderr=err,check=True)
snapshot=json.loads((out/'snapshot86.json').read_text());assert snapshot['pairs']==1 and len(snapshot['cases'])==86 and {row['id'] for row in snapshot['cases']}==set(policy['cases'])
ratios=[{'case':row['id'],'instructions':row['ratios']['instructions'][0],'limit':max(1.03,policy['cases'][row['id']]['instruction_band'])} for row in snapshot['cases']]
work=out/'work';work.mkdir();case=s.resolve_cases(['external/sunspider-1.0/string-unpack-code'],s.DEFAULT_CACHE,work)[0]
commands={'empty':['--raw','-e','0;'],'unpack':['--raw',str(case.workload)],'cli_only':['--help']};totals={};tool=s.counter_tool(trust_cycles=False)
for name,binary in paths.items():
 totals[name]={}
 for key,argv in commands.items():
  subprocess.run([str(binary),*argv],stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL,check=True)
  totals[name][key]=[s.sample(tool,[str(binary),*argv],120).instructions for _ in range(3)]
report={'diagnostic_only':True,'timing':False,'acceptance':False,'attempts_consumed':0,'candidate':CANDIDATE,'helper_sha256':sha(__file__),'images':images['binary_sha256'],'snapshot_sha256':sha(out/'snapshot86.json'),'instruction_ratios':ratios,'blocked':[r['case'] for r in ratios if r['instructions']>r['limit']],'canonical_totals':totals,'canonical_medians':{n:{c:statistics.median(v) for c,v in rows.items()} for n,rows in totals.items()},'scope':'86-case instruction snapshot only; wall fields in raw screen JSON are never timing/acceptance evidence. Lead must still run fresh full correctness and bound86 before timing.'}
(out/'qualification.json').write_text(json.dumps(report,indent=2)+'\n');print('SNAPSHOT',len(ratios),'blocked',report['blocked'],'totals',report['canonical_medians'],flush=True)
if report['blocked']:raise SystemExit(3)
