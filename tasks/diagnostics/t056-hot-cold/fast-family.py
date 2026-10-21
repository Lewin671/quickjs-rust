"""Fail-closed static prerequisite, separate from equal normalized bytes."""
import argparse,hashlib,json
from pathlib import Path
p=argparse.ArgumentParser(description=__doc__);p.add_argument('relative_pcs',type=Path);p.add_argument('output',type=Path);a=p.parse_args()
r=json.loads(a.relative_pcs.read_text());paths=r['relative_paths'];checks={}
for name in ('Dispatch','Backedge','Add','Lt','Update','Exit'):
    checks[name]=paths[name]['relative_pcs_equal'] and r['normalized_paths'][name]['equivalent_modulo_addresses']
for frame in ('WideLoopFrame','vm::Vm'):
    checks[frame]=all(v['relative_pcs_equal'] for v in paths[frame]['paths'].values())
result={'candidate_image_sha256':r['images']['candidate']['sha256'],'reference_image_sha256':r['images']['parent']['sha256'],
        'relative_pcs_sha256':hashlib.sha256(a.relative_pcs.read_bytes()).hexdigest(),
        'checks':checks,'fast_family_static_pass':all(checks.values()),'timing_eligible':all(checks.values()),
        'scope':'Exact relative-PC family of the qualified120B parent; normalized equality alone is insufficient. A failure blocks prepared timing jobs and the last frozen attempt; no latency/hardware-cause inference.'}
a.output.write_text(json.dumps(result,indent=2)+'\n');print(json.dumps(result,indent=2))
