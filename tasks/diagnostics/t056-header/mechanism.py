"""Read-only full-loop replay and accounting from already fetched lead results."""
import json,hashlib,statistics,sys
from pathlib import Path
ROOT=Path(__file__).resolve().parent;REPO=ROOT.parents[2];sys.path[:0]=[str(ROOT),str(REPO)]
from walk import Walker
from evidence_io import write
queue=Path('/Users/qingyingliu/Code/co-work-space/outputs/imac-bench');build=queue/'20261021T112221Z-t056-source-bisect-build/out';lead=queue/'20261021T114227Z-t056-source-bisect/out';fp=json.loads((ROOT.parent/'t056-pmu/fp-state.json').read_text());audit=json.loads((ROOT.parent/'t056-bisect/images.json').read_text());results={};counts={}
for role,fprole in [('base','base'),('aligned','aligned'),('candidate','allocation')]:
 source=build/'binaries'/role/'qjs';assert hashlib.sha256(source.read_bytes()).hexdigest()==audit['images'][role]['binary_sha256'];meta=audit['audits'][role]['placement']['candidate'];entry=int(meta['entry'],16);header=int(meta['header'],16)
 results[role]={}
 for case in ('broad/global_read','broad/local_read'):
  samples=[r for r in fp['snapshots'] if r['role']==fprole and r['case']==case];replays=[]
  for snap in samples:
   replay=Walker(source,build/'audits'/role/'final-path.asm',entry,header,snap).run();assert replay['instructions_per_iteration']==(152 if case.endswith('global_read') else 197);replays.append(replay)
  result=replays[0];result['all_four_real_snapshots_replayed']=len(replays);result['all_snapshot_instruction_counts']=[r['instructions_per_iteration'] for r in replays];results[role][case]=result
 for case in ('broad/global_read','broad/local_read'):
  rows=json.loads((lead/(case.replace('/','--')+'.json')).read_text())['rows'];per=results[role][case]['instructions_per_iteration'];N=23068672;deltas=[r['samples'][role]['2N']['instructions']-r['samples'][role]['N']['instructions'] for r in rows];whole=[r['samples'][role]['N']['instructions'] for r in rows]
  counts[role+':'+case]={'loop_instructions_per_iteration':per,'N':N,'predicted_increment':per*N,'measured_increment_by_block':deltas,'median_measured_increment':statistics.median(deltas),'model_over_measured_increment':per*N/statistics.median(deltas),'median_N_whole_instructions':statistics.median(whole),'known_repeated_loop_over_whole_N':per*(N-1)/statistics.median(whole),'caveat':'coverage estimate from exact taken-path model and process retired counts; residual includes outside work, OS/counter scope/noise. Instruction fraction is not time fraction; cannot rule out rare long outside stalls without hardware/time attribution.'}
report={'diagnostic_only':True,'timing':False,'canonical_binary_sha256':{r:audit['images'][r]['binary_sha256'] for r in results},'fp_snapshot_source':'20261021T104647Z-t056-fp-state-v3','lead_source':'20261021T114227Z-t056-source-bisect','producer_sha256':{n:hashlib.sha256((ROOT/n).read_bytes()).hexdigest() for n in ('walk.py','mechanism.py','evidence_io.py')},'input_assembly_sha256':{r:hashlib.sha256((build/'audits'/r/'final-path.asm').read_bytes()).hexdigest() for r in results},'loops':results,'accounting':counts}
refs={};hashes={}
for role,cases in results.items():
 refs[role]={}
 for case,data in cases.items():
  name='loops/'+role+'-'+case.rsplit('/',1)[-1]+'.json';write(ROOT/name,data);refs[role][case]=name;hashes[name]=hashlib.sha256((ROOT/name).read_bytes()).hexdigest()
report['loops']=refs;report['loop_files_sha256']=hashes;write(ROOT/'mechanism.json',report)
print('whole-loop indexes written',len(hashes))
