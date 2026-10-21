"""Exact operand/code-window comparison from five real image captures."""
import json,struct,hashlib,collections,sys,copy
from pathlib import Path
ROOT=Path(__file__).resolve().parent;REPO=ROOT.parents[2];sys.path[:0]=[str(ROOT),str(REPO)]
from walk import Walker
from evidence_io import write
Q=REPO.parents[2]/'outputs/imac-bench';capture=Q/'20261021T124843Z-t056-dispatch-heap-capture/out/captures.json';probe_capture=Q/'20261021T130515Z-t056-ops-address-capture/out/captures.json';cap=json.loads(capture.read_text());pcap=json.loads(probe_capture.read_text());cfg=json.loads((ROOT/'inputs.json').read_text());orig=Q/cfg['source_job']/'out';asmroot=Q/'20261021T115238Z-t056-header-subparts-build/out/audits';prior=Q/'20261021T112221Z-t056-source-bisect-build/out/audits';results={};allreplays={}
# Original producer emitted raw data; compact records preserve its source hash.
def capture_index(name,data,raw):
 refs={}
 for role in data['images']:
  target='captures/'+name+'-'+role+'.json';write(ROOT/target,{'role':role,'snapshots':[r for r in data['snapshots'] if r['role']==role]});refs[role]=target
 write(ROOT/(name+'.json'),{**{k:v for k,v in data.items() if k!='snapshots'},'raw_job_sha256':hashlib.sha256(raw.read_bytes()).hexdigest(),'snapshot_files':refs})
capture_index('captures',cap,capture);capture_index('probe-captures',pcap,probe_capture)
sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest()
from decode import decode

for role,m in cfg['roles'].items():
 binary=orig/'binaries'/role/'qjs';assert sha(binary)==m['binary_sha256'];asm=(prior if role in ('aligned','candidate') else asmroot)/role/'final-path.asm';allreplays[role]={};results[role]={}
 for case in ('broad/global_read','broad/local_read'):
  rows=[r for r in cap['snapshots'] if r['role']==role and r['case']==case];replays=[]
  for row in rows:
   r=Walker(binary,asm,m['entry'],m['header'],decode(row)).run();assert r['instructions_per_iteration']==(152 if case.endswith('global_read') else 197);replays.append(r)
  allreplays[role][case]=replays[0];first=rows[0];slide=first['image_slide'];code=[]
  for index,r in enumerate(replays[0]['trace']):
   pc=int(r['pc'],16);length=r['length'];code.append({**r,'sequence':index,'low12':pc%4096,'runtime_pc':hex(pc+slide),'offset32':pc%32,'offset64':pc%64,'window32':hex(pc//32*32),'last_byte_window32':hex((pc+length-1)//32*32),'window64':hex(pc//64*64),'last_byte_window64':hex((pc+length-1)//64*64),'crosses32':pc//32!=(pc+length-1)//32,'ends32':(pc+length)%32==0,'crosses64':pc//64!=(pc+length-1)//64,'ends64':(pc+length)%64==0,'next_pc_VM':replays[0]['trace'][index+1]['pc'] if index+1<len(replays[0]['trace']) else hex(m['header'])})
  memory=[]
  for a in replays[0]['accesses']:
   pc=int(a['pc'],16);inst=next(r for r in replays[0]['trace'] if int(r['pc'],16)==pc);addr=int(a['address'],16);region=a['region'];label=region+'+'+str(a['region_offset'])
   if region=='program':label='program+'+str(a['region_offset']+8)
   if region=='stack_scratch_pointer':label='RBP-48 / RSP+'+str(addr-first['rsp'])
   image=region=='image_constant_or_jump_table'
   if image:
    primary=next(t for r in m['paths']['Dispatch'] for t in r['relocations'] if t['kind']=='rip');binary_table=next(t for r in m['paths']['Lt'] for t in r['relocations'] if t['kind']=='rip');lo,hi=int(primary['target'],16),int(binary_table['target'],16)
    label=('TypedOp_jump_table+'+hex(addr-lo)) if lo<=addr<hi else ('BinaryOp_jump_table+'+hex(addr-hi)) if hi<=addr<hi+80 else 'RIP_FP_constant_1.0'
   memory.append({**a,'instruction':inst['op']+' '+inst['args'],'effective_class':label,'absolute_VM_or_data_address':hex(addr),'actual_process_address':hex(addr+slide) if image else hex(addr),'low12':addr%4096,'image_address_is_unslid_VM':image})
  rip=[]
  for name,path in m['paths'].items():
   for r in path:
    for target in r['relocations']:
     if target['kind']=='rip':rip.append({'path':name,'load_or_lea_pc':hex(r['pc']),'relative_PC':r['offset'],'operation':r['op'],'target_VM':target['target'],'target_process':hex(int(target['target'],16)+slide),'target_low12':int(target['target'],16)%4096,'target_mod64':int(target['target'],16)%64,'referent8':target.get('referent8')})
  key=role+'-'+case.rsplit('/',1)[-1];write(ROOT/'loops'/f'{key}.json',{'role':role,'case':case,'binary_sha256':sha(binary),'asm_sha256':sha(asm),'instructions_per_iteration':len(code),'all_six_capture_paths_equal':len(replays)==6 and all(x['trace']==replays[0]['trace'] for x in replays),'code':code,'memory':memory,'branches':replays[0]['branches'],'RIP_targets':rip,'no_heap_access_outside_bank_program_ops':all(a['region'] in ('bank','ops','program','stack_scratch_pointer','image_constant_or_jump_table') for a in memory)})
  def values(key):return sorted({hex(r[key]) for r in rows})
  hist={k:dict(collections.Counter(hex(r[k]%4096) for r in rows)) for k in ('bank','program','ops_base','constants_ptr','rbp','rsp')};cellhist=dict(collections.Counter(hex(c['cell']%4096) for r in rows for c in r['received_cells'] if c['reg']==5));results[role][case]={'loop_record':'loops/'+key+'.json','samples':len(rows),'heap_low12_histograms':hist,'r5_cell_low12_histogram':cellhist,'first_absolute_addresses':{k:hex(first[k]) for k in ('bank','program','ops_base','constants_ptr','rbp','rsp')},'first_received_cells':first['received_cells'],'publication_flag':first['deferred_publication'],'publication_sinks_len':first['sloppy_writes_len'],'r5_received_present':any(c['reg']==5 for r in rows for c in r['received_cells']),'all_received_r5_payloads_one':all(bytes.fromhex(c['cell_raw_hex'])[32:40]==struct.pack('<d',1.) for r in rows for c in r['received_cells'] if c['reg']==5)}
# Compare ordered PCs, not merely instruction strings with relocations erased.
reference=allreplays['count']['broad/global_read']['trace'];deltas={}
for role in cfg['roles']:
 t=allreplays[role]['broad/global_read']['trace'];assert len(t)==len(reference)==152
 deltas[role]=dict(collections.Counter(int(b['offset'],16)-int(a['offset'],16) for a,b in zip(reference,t)))
 assert [x['op'] for x in reference]==[x['op'] for x in t]
probe_summary={}
for role in pcap['images']:
 rows=[r for r in pcap['snapshots'] if r['role']==role];mode=role.rsplit('_',1)[-1];wanted=0x600 if mode=='phase600' else 0x900 if mode=='phase900' else 0x900 if role.startswith('count_flags') else 0x600
 assert len(rows)==12
 if mode!='sham':assert all(r['ops_base']%4096==wanted for r in rows)
 assert all(r['bank']%4096==0 for r in rows) and all(r['rbp']-r['rsp']==0x1b0 for r in rows)
 assert all(r['sloppy_writes_len']==0 for r in rows);probe_summary[role]={'rows':len(rows),'ops_low12_histogram':dict(collections.Counter(hex(r['ops_base']%4096) for r in rows)),'bank_low12':'0x0','program_low12':{c:dict(collections.Counter(hex(r['program']%4096) for r in rows if r['case']==c)) for c in ('broad/global_read','broad/local_read')},'RBP_minus_RSP':0x1b0,'normal_loop_bytes_match_parent':True}
write(ROOT/'comparison.json',{'diagnostic_only':True,'timing':False,'capture_source':'20261021T124843Z-t056-dispatch-heap-capture','raw_capture_sha256':sha(capture),'probe_capture_source':'20261021T130515Z-t056-ops-address-capture','raw_probe_capture_sha256':sha(probe_capture),'roles':results,'global_PC_deltas_from_count':deltas,'correlated_observations':['fast count/flags/8f have identical relative-PC vectors; slow count_flags/dfff share another vector, with -3/-6/-12B internal shifts','ops low12 fast600/slow900 for BOTH global and local; local remains fast900, so ops phase alone is not sufficient','global program+8 low12=8 overlaps bank payload low12=8 in ALL five originals; local program low12=600; no actual physical overlap','FP literal low12 differs across individual images and does not separate both classes; all8B aligned and do not cross64B'],'unique_hardware_cause_identified':False,'reason':'two coupled class-separating factors (ops phase and relative code map), with additional literal differences. Correlation alone cannot select a single cause. Confirmation isolates ops address holding all hot code PCs/bytes constant.','probe_qualification':probe_summary})
print('COMPARED',deltas,'probe snapshots',len(pcap['snapshots']))
