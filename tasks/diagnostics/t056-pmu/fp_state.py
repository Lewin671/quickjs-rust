"""Non-timing scalar/ops snapshots on frame/metadata-preserving image copies.

A header bridge checks integer checkpoint registers, then resumes the two
stolen header instructions. At a checkpoint it dumps and exits. Never time.
"""
import os,subprocess,struct,json,hashlib,shutil,re,math
from pathlib import Path
from tools.benchmark.layout_variant_audit import sections
from tools.benchmark.screen import resolve_cases,DEFAULT_CACHE
out=Path(os.environ['BENCH_OUT']);assert 'no-timing' in (out.parent/'status').read_text()
roles={'base':('f58c696e8379261ed726f0cea1051ed6d34f431a','b0dfc65d0c026d62aa64ef3d69a089eb73b51088c84af75b44617ea2a1737f0e',0x10025bd90),'aligned':('8f7d97c71b34e2bec52e2c6df87f4ba4ac51641a','b0b84489a3f6fd673b06dc4bdc71c8f176387a231ca19fe1da5ea48ca4ee0fd5',0x100007d90),'allocation':('dfff3bb6bf3db01fb52a7612ed1144900b5f08c5','3466c37a62f2fb6d764d0123212749ff810138ca05fcad336ce1afa05bdb1e3f',0x100007d90)}
N=23068672;points=(0,1,1024,N//2);work=out/'work';work.mkdir();cases=resolve_cases(['broad/global_read','broad/local_read'],DEFAULT_CACHE,work);active=out/'active/qjs';active.parent.mkdir()
report={'diagnostic_only':True,'timing':False,'images':[],'snapshots':[],'full_runs':[],'iteration_count':N,'caveat':'terminal copy with integer-only checkpoint bridge changes executed instruction stream; raw operands/control proof only, never PMU/wall evidence'}
def source(rev,path):return subprocess.check_output(['git','show',rev+':'+path],text=True)
for role,(rev,expected,entry) in roles.items():
 original=(Path('target/perf-loop/compiler-fat16-abort-v1')/('rev-'+rev)/'qjs').read_bytes();assert hashlib.sha256(original).hexdigest()==expected
 mod=source(rev,'crates/qjs-runtime/src/bytecode/typed_loop/mod.rs');variants=re.findall(r'^    ([A-Z]\w*)(?: \{|,)',mod.split('enum TypedOp {')[1].split('\n}')[0],re.M)
 ast=source(rev,'crates/qjs-ast/src/expression/operator.rs');binaryops=re.findall(r'^    ([A-Z]\w*),',ast.split('pub enum BinaryOp {')[1].split('\n}')[0],re.M)
 for point in points:
  asm=f'''.text
.globl _snapshot
_snapshot:
cmpq ${point},%rbx
jne Lresume
cmpq $0,%rdi
jne Lresume
pushq %rsp
pushq %rbp
pushq %r9
pushq %r11
pushq %rbx
pushq 0x8(%r11)
pushq 0x10(%r11)
movq %rsp,%rsi
movl $2,%edi
movl $56,%edx
movl $0x2000004,%eax
syscall
movq 0x20(%rsp),%rsi
movl $128,%edx
movl $0x2000004,%eax
syscall
movq 0x8(%rsp),%rsi
movq (%rsp),%rdx
shlq $4,%rdx
movl $0x2000004,%eax
syscall
subq $8,%rsp
stmxcsr (%rsp)
movq %rsp,%rsi
movl $4,%edx
movl $0x2000004,%eax
syscall
xorl %edi,%edi
movl $0x2000001,%eax
syscall
ud2
Lresume:
.byte 0x49,0x3b,0x7b,0x10
jae Lback
.byte 0xe9
.long 0
Lback:
.byte 0xe9
.long 0
'''
  (out/'bridge.s').write_text(asm);subprocess.run(['cc','-arch','x86_64','-c',str(out/'bridge.s'),'-o',str(out/'bridge.o')],check=True)
  assert 'Relocation information' not in subprocess.check_output(['otool','-r',str(out/'bridge.o')],text=True)
  obj=(out/'bridge.o').read_bytes();sec=next(s for s in sections(out/'bridge.o') if s[2]=='__TEXT/__text');bridge=bytearray(obj[sec[3]:sec[3]+sec[1]])
  image=out/f'{role}-{point}';image.write_bytes(original);image.chmod(0o755);segs=sections(image);text=next(s for s in segs if s[2]=='__TEXT/__text');header=entry+0x86;helper=entry+0x1000;h=text[3]+header-text[0];v=text[3]+helper-text[0]
  positions=[m.start() for m in re.finditer(re.escape(b'\xe9\0\0\0\0'),bridge)];assert len(positions)==2
  for where,target in zip(positions,[header+6,entry+0x70]):bridge[where+1:where+5]=struct.pack('<i',target-(helper+where+5))
  assert original[h:h+6]==bytes.fromhex('493b7b1073e4')
  patched=bytearray(original);patched[h:h+6]=b'\xe9'+struct.pack('<i',helper-header-5)+b'\x90';patched[v:v+len(bridge)]=bridge;image.write_bytes(patched)
  assert len(patched)==len(original) and sections(image)==segs and patched[:h]==original[:h] and patched[h+6:v]==original[h+6:v] and patched[v+len(bridge):]==original[v+len(bridge):]
  report['images'].append({'role':role,'checkpoint':point,'canonical_sha256':expected,'snapshot_sha256':hashlib.sha256(patched).hexdigest(),'entry':entry,'changed_ranges':[[header,6],[helper,len(bridge)]],'headers_and_other_file_bytes_preserved':True,'bridge_hex':bridge.hex()})
  shutil.copy2(image,active)
  for case in cases:
   result=subprocess.run([str(active),'--raw',str(case.workload),case.case_id,str(N)],stdout=subprocess.PIPE,stderr=subprocess.PIPE,timeout=60)
   assert result.returncode==0 and not result.stdout,(role,point,case.id,result.returncode,result.stderr[:300])
   values=dict(zip(('ops_count','ops_base','backedge_iterations','program','bank','rbp','rsp'),struct.unpack('<7Q',result.stderr[:56])));count=values['ops_count'];assert count==(4 if case.id=='broad/global_read' else 5) and len(result.stderr)==56+128+count*16+4
   bank=result.stderr[56:184];opsraw=result.stderr[184:184+count*16];mxcsr=struct.unpack('<I',result.stderr[-4:])[0];registers=[]
   for i in range(8):
    raw=bank[i*16:i*16+16];tag=raw[0];bits=struct.unpack('<Q',raw[8:])[0];value=struct.unpack('<d',raw[8:])[0];exponent=(bits>>52)&2047;fraction=bits&((1<<52)-1);category='zero' if exponent==fraction==0 else 'subnormal' if exponent==0 else 'nan' if exponent==2047 and fraction else 'infinite' if exponent==2047 else 'normal'
    registers.append({'index':i,'tag':tag,'variant':{0:'Number',1:'Boolean',2:'Undefined'}.get(tag,'unknown'),'payload_bits':f'{bits:016x}','value':value if tag==0 and math.isfinite(value) else None,'fp_class':category if tag==0 else None,'raw_hex':raw.hex()})
   decoded=[]
   for i in range(count):
    raw=opsraw[i*16:i*16+16];kind=variants[raw[0]];row={'kind':kind,'raw_hex':raw.hex()}
    if kind=='Binary':row.update(operator=binaryops[raw[1]],dst=struct.unpack_from('<H',raw,2)[0],left=struct.unpack_from('<H',raw,4)[0],right=struct.unpack_from('<H',raw,6)[0])
    elif kind=='Update':row.update(operator=('Increment','Decrement')[raw[1]],dst=struct.unpack_from('<H',raw,2)[0],src=struct.unpack_from('<H',raw,4)[0])
    elif kind=='Exit':row.update(cond=struct.unpack_from('<H',raw,2)[0],exit_ip=struct.unpack_from('<I',raw,4)[0])
    else:raise AssertionError(kind)
    decoded.append(row)
   print('OBSERVED',role,case.id,point,values['backedge_iterations'],[(i,registers[i]['value'],registers[i]['payload_bits']) for i in range(2,6)],flush=True)
   assert values['backedge_iterations']==point and registers[2]['value']==point+1 and registers[3]['value']==N and registers[4]['value']==(point+1)*(1 if case.id=='broad/global_read' else 3) and registers[5]['value']==1
   assert all(registers[i]['tag']==0 and registers[i]['fp_class'] in ('normal','zero') for i in range(2,6))
   report['snapshots'].append({'role':role,'case':case.id,'checkpoint':point,**values,'registers':registers,'ops':decoded,'mxcsr':f'{mxcsr:08x}','denormal_flag':bool(mxcsr&2),'ftz':bool(mxcsr&(1<<15)),'daz':bool(mxcsr&(1<<6))})
  print('BITS',role,point,flush=True)
 # Existing diagnostic image is used only for path outcomes/counts. Canonical
 # file is separately run to completion; neither duration is interpreted.
 diagnostic=Path(os.environ['BENCH_QUEUE'])/'20261021T092319Z-t056-startup-event-attribution/out/binaries'/role/'qjs'
 canonical=Path('target/perf-loop/compiler-fat16-abort-v1')/('rev-'+rev)/'qjs'
 for case in cases:
  args=['--raw',str(case.workload),case.case_id,str(N)];trace=subprocess.run([str(diagnostic),*args],env={**os.environ,'QJS_TL_TRACE':'1'},stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True,check=True,timeout=90)
  normal=subprocess.run([str(canonical),*args],stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True,check=True,timeout=90)
  assert trace.stdout==normal.stdout
  (out/(role+'-'+case.case_id+'-trace.txt')).write_text(trace.stderr)
  entries=[x for x in trace.stderr.splitlines() if x.startswith('TLRUN ')]
  assert len(entries)==1 and entries[0].endswith(' Ran') and 'TLDEOPT ' not in trace.stderr,(role,case.id,entries)
  report['full_runs'].append({'role':role,'case':case.id,'diagnostic_image_sha256':hashlib.sha256(diagnostic.read_bytes()).hexdigest(),'canonical_sha256':expected,'entry_count':len(entries),'outcome_lines':entries,'guard_deopts':0,'stdout':normal.stdout,'trace_sha256':hashlib.sha256(trace.stderr.encode()).hexdigest()})
(out/'fp-state.json').write_text(json.dumps(report,indent=2,allow_nan=False)+'\n')
