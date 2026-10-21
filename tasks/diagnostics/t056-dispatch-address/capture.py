"""Unsigned, terminal image-copy captures; canonical frames/headers unchanged."""
import os,json,subprocess,hashlib,struct,shutil,re
from pathlib import Path
from tools.benchmark.layout_variant_audit import sections
from tools.benchmark.screen import resolve_cases,DEFAULT_CACHE
ROOT=Path(__file__).resolve().parent;out=Path(os.environ['BENCH_OUT']);assert 'no-timing' in (out.parent/'status').read_text();config=json.loads((ROOT/'inputs.json').read_text());job=Path(os.environ['BENCH_QUEUE'])/config['source_job'];assert (job/'status').read_text().strip()=='done 0'
sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest()
# Type sizes/offsets come from fresh runtime compiler layout, saved with result.
layoutjob=Path(os.environ['BENCH_QUEUE'])/'20261021T120731Z-t056-header-layout-qualification/out';s=(layoutjob/'count-layout.log').read_text();types={}
for block in re.split(r'(?=^print-type-size type: )',s,flags=re.M):
 m=re.match(r'print-type-size type: `([^`]+)`: (\d+) bytes, alignment: (\d+) bytes',block)
 if m and m[1] in ("bytecode::typed_loop::TypedLoopProgram","bytecode::typed_loop::TypedLoopScratch","bytecode::compact_fn::wide::loop_frame::WideLoopFrame<'_>","bytecode::ir::Bytecode"):
  fields={};offset=0
  for line in block.splitlines()[1:]:
   f=re.match(r'print-type-size\s+field `\.(\w+)`: (\d+) bytes',line);pad=re.match(r'print-type-size\s+(?:end )?padding: (\d+) bytes',line)
   if f:fields[f[1]]={'offset':offset,'bytes':int(f[2])};offset+=int(f[2])
   elif pad:offset+=int(pad[1])
  assert offset==int(m[2]);types[m[1]]={'size':int(m[2]),'fields':fields,'compiler_text':block}
assert len(types)==4
bc_mask=types['bytecode::ir::Bytecode']['fields']['cell_received_upvalue_slots']['offset']
work=out/'work';work.mkdir();cases=resolve_cases(['broad/global_read','broad/local_read'],DEFAULT_CACHE,work);active=out/'active/qjs';active.parent.mkdir();report={'timing':False,'diagnostic_only':True,'types':types,'images':{},'snapshots':[]}
for key in ('QJS_TL_TRACE','MallocNanoZone','MallocStackLogging','T056_INITIAL_STACK_PAD','T056_STACK_COOKIE'):os.environ.pop(key,None)
os.environ.update(TZ='UTC',LC_ALL='C')
for role,meta in config['roles'].items():
 source=job/'out/binaries'/role/'qjs';original=source.read_bytes();assert sha(source)==meta['binary_sha256'];entry=meta['entry'];header=meta['header'];helper=entry+0x1000
 # Dump all addresses before write syscall clobbers r11/rcx. No resumes, logger,
 # new native frame, allocation, signature or segment/header modification.
 asm='''.text
.globl _snapshot
_snapshot:
pushq %rsp
pushq %rbp
pushq %r9
pushq %r11
pushq -0x30(%rbp)
pushq -0x48(%rbp)
leaq Lhere(%rip),%rax
Lhere:
pushq %rax
pushq 0x8(%r11)
pushq 0x10(%r11)
movq %rsp,%r15
'''
 def dump(ptr,length):return '\n'+ptr+'\n'+length+'\nmovl $2,%edi\nmovl $0x2000004,%eax\nsyscall\n'
 asm+=dump('movq %r15,%rsi','movl $72,%edx')
 for off,size in ((40,528),(24,192),(32,80),(48,128)):
  asm+=dump(f'movq {off}(%r15),%rsi',f'movl ${size},%edx')
 asm+=dump('movq 8(%r15),%rsi','movq (%r15),%rdx\nshlq $4,%rdx')
 asm+=dump('movq 24(%r15),%rsi\nmovq 144(%rsi),%rsi','movl $1280,%edx')
 asm+=dump('movq 40(%r15),%rsi\nmovq 296(%rsi),%rsi','movq 40(%r15),%rdx\nmovq 304(%rdx),%rdx\nimulq $24,%rdx,%rdx')
 asm+=dump('movq 40(%r15),%rsi\nmovq 80(%rsi),%rsi','movq 40(%r15),%rdx\nmovq 88(%rdx),%rdx\nshlq $3,%rdx')
 asm+='movq 24(%r15),%r13\nmovq 64(%r13),%r12\nmovq 72(%r13),%r14\ncmpq $32,%r14\nja Lbad\n'
 asm+=dump('movq %r12,%rsi','movq %r14,%rdx\nshlq $3,%rdx')
 asm+='testq %r14,%r14\nje Ldone\nLcell:\n'
 asm+=dump('movq (%r12),%rsi','movl $40,%edx')
 asm+='addq $8,%r12\ndecq %r14\njne Lcell\nLdone:\nxorl %edi,%edi\nmovl $0x2000001,%eax\nsyscall\nud2\nLbad:\nmovl $7,%edi\nmovl $0x2000001,%eax\nsyscall\nud2\n'
 (out/'bridge.s').write_text(asm);subprocess.run(['cc','-arch','x86_64','-c',str(out/'bridge.s'),'-o',str(out/'bridge.o')],check=True);assert 'Relocation information' not in subprocess.check_output(['otool','-r',str(out/'bridge.o')],text=True)
 obj=(out/'bridge.o').read_bytes();sec=next(x for x in sections(out/'bridge.o') if x[2]=='__TEXT/__text');bridge=obj[sec[3]:sec[3]+sec[1]];here=bridge.index(bytes.fromhex('488d0500000000'))+7
 image=out/(role+'-snapshot');image.write_bytes(original);image.chmod(0o755);segs=sections(image);assert not subprocess.check_output(['otool','-l',str(image)],text=True).count('LC_CODE_SIGNATURE');text=next(x for x in segs if x[2]=='__TEXT/__text');h=text[3]+header-text[0];v=text[3]+helper-text[0];assert original[h:h+6]==bytes.fromhex('493b7b1073e4')
 patched=bytearray(original);patched[h:h+6]=b'\xe9'+struct.pack('<i',helper-header-5)+b'\x90';patched[v:v+len(bridge)]=bridge;image.write_bytes(patched);assert len(patched)==len(original) and sections(image)==segs and patched[:h]==original[:h] and patched[h+6:v]==original[h+6:v] and patched[v+len(bridge):]==original[v+len(bridge):]
 report['images'][role]={'canonical_sha256':meta['binary_sha256'],'snapshot_sha256':sha(image),'header':header,'helper':helper,'helper_bytes':len(bridge),'here_offset':here,'unsigned_headers_and_all_other_bytes_preserved':True,'changed_ranges':[[header,6],[helper,len(bridge)]]};shutil.copy2(image,active)
 for case in cases:
  for n in (23068672,46137344):
   for rep in range(3):
    result=subprocess.run([str(active),'--raw',str(case.workload),case.case_id,str(n)],capture_output=True,check=True,timeout=30);assert not result.stdout;data=result.stderr;pos=0
    def take(size):
     global pos
     chunk=data[pos:pos+size];assert len(chunk)==size;pos+=size;return chunk
    values=dict(zip(('ops_count','ops_base','runtime_here','frame','scratch','program','bank','rbp','rsp'),struct.unpack('<9Q',take(72))));program=take(528);frame=take(192);scratch=take(80);bank=take(128);ops=take(values['ops_count']*16);bytecode=take(1280);u64=lambda b,i:struct.unpack_from('<Q',b,i)[0];constptr=u64(program,296);constlen=u64(program,304);constants=take(constlen*24);localptr=u64(program,80);locallen=u64(program,88);localdata=take(locallen*8);uptr=u64(frame,64);ulen=u64(frame,72);upvalues=take(ulen*8);cells=[take(40) for _ in range(ulen)];assert pos==len(data)
    slide=values['runtime_here']-(helper+here);assert slide%4096==0 and values['ops_count']==(4 if case.case_id=='global_read' else 5)
    slots=[{'reg':struct.unpack_from('<H',localdata,i*8)[0],'slot':struct.unpack_from('<I',localdata,i*8+4)[0]} for i in range(locallen)];mask=int.from_bytes(bytecode[bc_mask:bc_mask+16],'little');received=[]
    for row in slots:
     slot=row['slot']
     if slot<128 and mask&(1<<slot):
      index=(mask&((1<<slot)-1)).bit_count();assert index<ulen;raw=u64(upvalues,index*8);received.append({**row,'upvalue_index':index,'RcInner':raw,'cell':raw+16,'payload':raw+24,'cell_raw_hex':cells[index].hex()})
    report['snapshots'].append({'role':role,'case':case.id,'N':n,'rep':rep,**values,'image_slide':slide,'constants_ptr':constptr,'constants_len':constlen,'constants_hex':constants.hex(),'local_slots_ptr':localptr,'local_slots':slots,'upvalues_ptr':uptr,'upvalues_len':ulen,'received_cells':received,'program_hex':program.hex(),'frame_hex':frame.hex(),'scratch_hex':scratch.hex(),'bank_hex':bank.hex(),'ops_hex':ops.hex(),'bytecode_cell_mask':mask,'deferred_publication':bool(program[520]),'sloppy_writes_len':u64(program,184),'raw_sha256':hashlib.sha256(data).hexdigest()})
 print('CAPTURED',role,flush=True)
(out/'captures.json').write_text(json.dumps(report,indent=2)+'\n');print('READY',len(report['snapshots']),flush=True)
