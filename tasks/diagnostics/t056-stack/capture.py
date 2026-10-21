"""Non-timing image-copy snapshot: original callers/prologue, stop at first header.

No engine source edits, call frames, runtime allocation or logger calls are
introduced. The terminal bridge pushes snapshots below the original RSP and
writes them by syscall, then exits. Never use this image for timing/semantics.
"""
import os,subprocess,hashlib,json,struct,shutil,re
from pathlib import Path
from tools.benchmark.layout_variant_audit import sections
from tools.benchmark.screen import resolve_cases,DEFAULT_CACHE
out=Path(os.environ['BENCH_OUT']);assert 'no-timing' in (out.parent/'status').read_text()
roles={'base':('f58c696e8379261ed726f0cea1051ed6d34f431a','b0dfc65d0c026d62aa64ef3d69a089eb73b51088c84af75b44617ea2a1737f0e',0x10025bd90),'aligned':('8f7d97c71b34e2bec52e2c6df87f4ba4ac51641a','b0b84489a3f6fd673b06dc4bdc71c8f176387a231ca19fe1da5ea48ca4ee0fd5',0x100007d90),'allocation':('dfff3bb6bf3db01fb52a7612ed1144900b5f08c5','3466c37a62f2fb6d764d0123212749ff810138ca05fcad336ce1afa05bdb1e3f',0x100007d90)}
assembly='''.text
.globl _snapshot
_snapshot:
pushq %rsp
pushq %rbp
pushq %r9
pushq %r11
pushq -0x30(%rbp)
pushq 0x8(%r11)
pushq 0x10(%r11)
movq %rsp,%rsi
movl $2,%edi
movl $56,%edx
movl $0x2000004,%eax
syscall
xorl %edi,%edi
movl $0x2000001,%eax
syscall
ud2
'''
(out/'snapshot.s').write_text(assembly)
subprocess.run(['cc','-arch','x86_64','-c',str(out/'snapshot.s'),'-o',str(out/'snapshot.o')],check=True)
obj=(out/'snapshot.o').read_bytes();sect=next(s for s in sections(out/'snapshot.o') if s[2]=='__TEXT/__text');bridge=obj[sect[3]:sect[3]+sect[1]]
results={'diagnostic_only':True,'timing':False,'snapshot_fields':['ops_count','ops_base','stack_slot_value','program','bank','rbp','rsp'],'images':{},'runs':[],'bridge_hex':bridge.hex(),'caveat':'terminal binary-copy patch, not canonical byte identity; original entry/caller/prologue and original first-header RSP preserved; no statement about untouched timing-run environment phase'}
active=out/'active/qjs';active.parent.mkdir();work=out/'work';work.mkdir();cases=resolve_cases(['broad/global_read','broad/local_read'],DEFAULT_CACHE,work)
# Fine/coarse grid is fixed before any timing; captures can separately select a
# prespecified subset with T056_CAPTURE_GRID=coarse for bounded qualification.
grid=[0,16,48,80,112]+list(range(256,4096,256)) if os.environ.get('T056_CAPTURE_GRID')=='coarse' else [0]
for role,(rev,expected,entry) in roles.items():
 source=Path('target/perf-loop/compiler-fat16-abort-v1')/('rev-'+rev)/'qjs';original=source.read_bytes();assert hashlib.sha256(original).hexdigest()==expected
 image=out/(role+'-snapshot');image.write_bytes(original);image.chmod(0o755)
 segs=sections(image);text=next(s for s in segs if s[2]=='__TEXT/__text');header=entry+0x86;helper=entry+0x1000
 def offset(pc):return text[3]+pc-text[0]
 h=offset(header);v=offset(helper)
 # The natural header begins with cmpq [r11+0x10],rdi; jae rel8. No
 # instruction before the first header lies in the terminal bridge region.
 assert original[h:h+6]==bytes.fromhex('493b7b1073e4')
 patched=bytearray(original);patched[h:h+10]=b'\xe9'+struct.pack('<i',helper-header-5)+b'\x90'*5;patched[v:v+len(bridge)]=bridge;image.write_bytes(patched)
 status=subprocess.run(['codesign','-d',str(source)],stdout=subprocess.PIPE,stderr=subprocess.PIPE)
 assert status.returncode!=0 and b'not signed' in status.stderr,'this probe requires the original unsigned x86 image'
 signed=image.read_bytes();newsegs=sections(image)
 assert newsegs==segs and len(signed)==len(original),'mapped sections or file length changed'
 assert signed[:h]==original[:h] and signed[h+10:v]==original[h+10:v]
 assert signed[v+len(bridge):]==original[v+len(bridge):]
 # Callers and every original text byte except these two regions are exact.
 results['images'][role]={'revision':rev,'canonical_sha256':expected,'snapshot_sha256':hashlib.sha256(signed).hexdigest(),'entry':entry,'header':header,'bridge':helper,'header_original_hex':original[h:h+10].hex(),'text_bytes_changed_only_in_ranges':[[header,10],[helper,len(bridge)]],'mapped_sections_unchanged':True,'caller_and_prologue_bytes_identical':True,'unsigned_header_and_all_other_file_bytes_identical':True}
 shutil.copy2(image,active)
 for case in cases:
  for pad in grid:
   for rep in range(2):
    n=23068672 if rep==0 else 46137344
    argv=[str(active),'--raw',str(case.workload),case.case_id,str(n)]
    env=dict(os.environ)
    for key in ('T056_CAPTURE_GRID','MallocNanoZone','MallocStackLogging','MallocStackLoggingNoCompact','T056_STACK_COOKIE','QJS_ARCH_PAD_MODE','QJS_DIAGNOSTIC_BANK_POLICY','QJS_DIAGNOSTIC_BANK_TRACE'):env.pop(key,None)
    env.update(TZ='UTC',LC_ALL='C',T056_INITIAL_STACK_PAD='x'*pad)
    result=subprocess.run(['/usr/bin/time','-l',*argv],env=env,stdout=subprocess.PIPE,stderr=subprocess.PIPE,timeout=30)
    assert result.returncode==0 and result.stdout==b'' and len(result.stderr)>=56,(role,case.id,pad,rep,result.returncode,result.stderr[:1000])
    values=dict(zip(results['snapshot_fields'],struct.unpack('<7Q',result.stderr[:56])))
    assert values['ops_count']==(4 if case.id=='broad/global_read' else 5),values
    assert values['rsp']<values['rbp'] and values['bank']>0x100000000
    row={'role':role,'case':case.id,'pad_bytes':pad,'repeat':rep,'iterations':n,'argv':argv,'argv_bytes':sum(len(os.fsencode(x))+1 for x in argv),'env_count':len(env),'env_bytes':sum(len(os.fsencode(k+'='+x))+1 for k,x in env.items()),'env_entry_bytes':{k:len(os.fsencode(k+'='+x))+1 for k,x in env.items()},'env_order_sha256':hashlib.sha256(b'\0'.join(os.fsencode(k+'='+x) for k,x in env.items())).hexdigest(),**values}
    row['stack_slot']=values['rbp']-0x30
    results['runs'].append(row)
    print('SNAPSHOT',role,case.id,pad,rep,'rsp',hex(values['rsp']),'rbp',hex(values['rbp']),'bank',hex(values['bank']),'slot',hex(row['stack_slot']),flush=True)
if os.environ.get('T056_CAPTURE_GRID')=='geometry':
 import protocol
 seeds=[next(r for r in results['runs'] if r['role']==role and r['case']=='broad/global_read' and r['repeat']==0) for role in ('base','allocation')]
 pads=protocol.grid(seeds);assert len(pads)<=30
 results['geometry_grid']=pads
 from phase import take
 for role in roles:
  shutil.copy2(out/(role+'-snapshot'),active)
  for case in cases:
   for pad in pads:
    if pad==0:continue
    for rep in range(2):
     n=23068672 if rep==0 else 46137344
     env=dict(os.environ)
     for key in ('T056_CAPTURE_GRID',*protocol.ENV_KEYS):env.pop(key,None)
     env.update(TZ='UTC',LC_ALL='C',T056_INITIAL_STACK_PAD='x'*pad)
     argv=['--raw',str(case.workload),case.case_id,str(n)]
     values=take(active,argv,env);assert values['ops_count']==(4 if case.id=='broad/global_read' else 5)
     results['runs'].append({'role':role,'case':case.id,'pad_bytes':pad,'repeat':rep,'iterations':n,'argv':[str(active),*argv],**values})
(out/'snapshots.json').write_text(json.dumps(results,indent=2)+'\n')
