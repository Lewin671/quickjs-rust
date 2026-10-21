"""Enumerate all five exact 152 paths at K=6 displacements, retaining provenance."""
import hashlib,json,sys
from pathlib import Path
from geometry import branches,summary
ROOT=Path(__file__).resolve().parent;OLD=ROOT.parent/'t056-dispatch-address'
sys.path.insert(0,str(OLD))
from evidence_io import write
sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest()
queue=ROOT.parents[5]/'outputs/imac-bench'
qualified=queue/'20261021T082252Z-t056-code-layout-qualification/out/qualification.json'
q=json.loads(qualified.read_text());results={}
for role in ('aligned','count','flags','count_flags','candidate'):
 path=OLD/'loops'/(role+'-global_read.json');data=json.loads(path.read_text());assert len(data['code'])==152
 results[role]={'original_sha256':data['binary_sha256'],'path_sha256':sha(path),'offsets':{}}
 for index,offset in enumerate((0,16,64,240,512,1008)):
  # K=6 link audits prove uniform text shifts, possibly plus whole pages.
  # Count/flags/8f were not in that old timing panel: these are arithmetic
  # translations of their exact decoded PCs, not invented measured images.
  variant=q['images']['c'+str(index)] if role=='candidate' else None
  actual=(variant['audit']['actual_uniform_shift'] if index else 0) if variant else None
  if variant and index:assert actual%32==offset%32 and variant['audit']['body_drift']==variant['audit']['address_drift']==0
  rows=branches(data['code'],actual if actual is not None else offset)
  rel=f'originals/{role}-{offset}.json'
  write(ROOT/rel,{'role':role,'offset':offset,'scope':'actual audited K=6 image' if variant else 'uniform-PC translation geometry; no image built or timed','actual_uniform_shift':actual,'binary_sha256':variant['sha256'] if variant else None,'summary':summary(rows),'branches':rows})
  results[role]['offsets'][str(offset)]={'record':rel,**summary(rows)}
write(ROOT/'original-geometry.json',{'diagnostic_only':True,'timing':False,'fusion_scope':'adjacent CMP/TEST + supported Jcc candidate; no claim of actual uop fusion or DSB residency','end_rule':'end-exclusive floor(start/32) != floor((branch_pc+length)/32); includes exact end on boundary','K6_job':qualified.parts[-3],'K6_qualification_sha256':sha(qualified),'roles':results,'cross_2x2':{'job':'20261021T085751Z-t056-code-layout-cross','candidate_code0':'candidate-0','candidate_code1':'candidate-16','zone0_and_zone1':'same image PCs/bytes; environment does not alter static geometry','base':'f58 is a different body; cross-geometry.json lists all29 branches for all four actual code images, both zone cells'},'prediction_supported':False,'reason':'fast family has5 affected dynamic branches at either phase, slow family4/6; fast has more than one slow phase, and shares numeric-tag crossing with dfff at natural phase'})
print({r:{k:v['affected_dynamic'] for k,v in d['offsets'].items()} for r,d in results.items()})

# The original 2x2 used f58/dfff. Current flagged decodes have byte-identical
# __text, so their rows also decode the unflagged code0 image exactly.
cross={}
for side,role in [('b','base'),('c','candidate')]:
 code=json.loads((ROOT/'audits'/(role+'_jcc.json')).read_text())['code']
 for index in (0,1):
  image=q['images'][side+str(index)];shift=0 if index==0 else image['audit']['actual_uniform_shift'];rows=branches(code,shift)
  cross[side+str(index)]={'canonical_revision':image['revision'],'actual_image_sha256':image['sha256'],'MallocNanoZone_cells':['0','1'],'uniform_shift':shift,'basis':'decoded requested-flag copy has identical complete __text to canonical original; original+16 link audit proves uniform shift','summary':summary(rows),'branches':rows}
write(ROOT/'cross-geometry.json',{'job':'20261021T085751Z-t056-code-layout-cross','qualification_sha256':sha(qualified),'scope':'actual f58/dfff code0/+16 images, both malloc zone cells; not8f/count/flags timing images','images':cross})
