"""Non-timing data-only ops relocation copies, same-frame entry/exit shims.

Only numeric global/local cases are admitted. Arena is one RW anonymous page,
freed by process exit; original Vec pointer is restored before normal Exit.
The last entry spill/jump and normal-exit jump are redirected. Entry replays
the spill and reserves32B; RBP-relative addresses, every repeat-path byte/PC,
and mapped headers stay unchanged. Sham uses the identical setup/cleanup.
"""
import os,json,hashlib,struct,subprocess,shutil,re
from pathlib import Path
from tools.benchmark.layout_variant_audit import sections
from tools.benchmark.layout_tables import text_symbols
from tools.benchmark.layout_symbols import parse_symbol
from tools.benchmark.screen import resolve_cases,DEFAULT_CACHE,_check_output
ROOT=Path(__file__).resolve().parent;out=Path(os.environ['BENCH_OUT']);assert 'no-timing' in (out.parent/'status').read_text();cfg=json.loads((ROOT/'inputs.json').read_text());job=Path(os.environ['BENCH_QUEUE'])/cfg['source_job'];assert (job/'status').read_text().strip()=='done 0';sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest()
work=out/'work';work.mkdir();cases=resolve_cases(['broad/global_read','broad/local_read'],DEFAULT_CACHE,work);results={};paths={};checks={};page=4096
for role in ('count','flags','count_flags','aligned','candidate'):
 original=job/'out/binaries'/role/'qjs';assert sha(original)==cfg['roles'][role]['binary_sha256'];target=out/'binaries'/role/'qjs';target.parent.mkdir(parents=True);shutil.copy2(original,target);paths[role]=target;results[role]={'kind':'original','source':role,'binary_sha256':sha(target)}
 if role in ('aligned','candidate'):continue
 data=original.read_bytes();ss=sections(original);text=next(s for s in ss if s[2]=='__TEXT/__text');at=lambda pc,n:data[text[3]+pc-text[0]:text[3]+pc-text[0]+n];entry=cfg['roles'][role]['entry'];header=cfg['roles'][role]['header'];symbols=text_symbols(original)
 vm=next(n for n in symbols.names if (sym:=parse_symbol(n)) and 'typed_loop::execute::' in sym.canonical and 'vm::Vm' in sym.canonical and symbols.sizes[n]>10000);helper=symbols.addresses[vm]+0x1000;cleanup=helper+0x300
 assert at(entry+0x68,8)==bytes.fromhex('4c8945d0eb186690')
 first=entry+0x68
 last=cfg['roles'][role]['paths']['Exit'][-1];exitjmp=last['pc']+last['length'];assert at(exitjmp,1)==b'\xe9';normal_exit=exitjmp+5+int.from_bytes(at(exitjmp+1,4),'little',signed=True)
 # Shim reserves32B after the original prologue; all RBP-relative accesses keep
 # their addresses. Every modified image maps and copies BOTH phases, including
 # sham; only an entry-time selection differs. Full GPR/RFLAGS preservation.
 regs=('rax','rcx','rdx','rsi','rdi','r8','r9','r10','r11','r12','r13','r14','r15')
 for mode in ('sham','phase600','phase900'):
  select='' if mode=='sham' else 'leaq '+('0x600' if mode=='phase600' else '0x900')+'(%r14),%rax\nmovq %rax,8(%r11)\n'
  asm='.text\n.globl _entry\n_entry:\nmovq %r8,-0x30(%rbp)\nleaq -32(%rsp),%rsp\nmovq %r11,16(%rsp)\nmovq %rax,24(%rsp)\nmovq 8(%r11),%rax\nmovq %rax,8(%rsp)\npushfq\n'+''.join('pushq %'+r+'\n' for r in regs)
  # r15 points to saved original registers (r15,r14,r13,r12,r11 at+32).
  asm+='movq %rsp,%r15\nmovq $0,%rdi\nmovq $4096,%rsi\nmovq $3,%rdx\nmovq $0x1002,%r10\nmovq $-1,%r8\nxorl %r9d,%r9d\nmovl $0x20000c5,%eax\nsyscall\njb Lfail\nmovq %rax,%r14\nmovq %rax,112(%r15)\nmovq 32(%r15),%r11\ncld\n'
  for offset in ('0x600','0x900'):
   asm+=f'leaq {offset}(%r14),%rdi\nmovq 8(%r11),%rsi\nmovq 16(%r11),%rcx\nshlq $4,%rcx\nrep movsb\n'
  asm+=select+''.join('popq %'+r+'\n' for r in reversed(regs))+'popfq\nmovq 24(%rsp),%rax\n.byte 0xe9\n.long 0\nLfail:\nmovl $9,%edi\nmovl $0x2000001,%eax\nsyscall\nud2\n'
  epilogue='.text\n.globl _cleanup\n_cleanup:\npushfq\npushq %rax\nmovq 24(%rsp),%rax\nmovq %rax,8(%r11)\npopq %rax\npopfq\nleaq 32(%rsp),%rsp\n.byte 0xe9\n.long 0\n'
  chunks=[]
  for label,source,pc,targetpc in [('entry',asm,helper,header),('cleanup',epilogue,cleanup,normal_exit)]:
   (out/(label+'.s')).write_text(source);subprocess.run(['cc','-arch','x86_64','-c',str(out/(label+'.s')),'-o',str(out/(label+'.o'))],check=True);assert 'Relocation information' not in subprocess.check_output(['otool','-r',str(out/(label+'.o'))],text=True)
   obj=(out/(label+'.o')).read_bytes();sec=next(s for s in sections(out/(label+'.o')) if s[2]=='__TEXT/__text');b=bytearray(obj[sec[3]:sec[3]+sec[1]]);j=[m.start() for m in re.finditer(re.escape(b'\xe9\0\0\0\0'),b)];assert len(j)==1;b[j[0]+1:j[0]+5]=struct.pack('<i',targetpc-(pc+j[0]+5));chunks.append((pc,bytes(b)))
  chunks+=[(first,b'\xe9'+struct.pack('<i',helper-first-5)+b'\x90'*3),(exitjmp,b'\xe9'+struct.pack('<i',cleanup-exitjmp-5))];chunks.sort();patched=bytearray(data);previous=0
  for pc,b in chunks:
   pos=text[3]+pc-text[0];assert pos>=previous;patched[pos:pos+len(b)]=b;previous=pos+len(b)
  name=role+'_'+mode;image=out/'binaries'/name/'qjs';image.parent.mkdir(parents=True);image.write_bytes(patched);image.chmod(0o755);assert sections(image)==ss and len(patched)==len(data) and 'LC_CODE_SIGNATURE' not in subprocess.check_output(['otool','-l',str(image)],text=True)
  cursor=0
  for pc,b in chunks:
   pos=text[3]+pc-text[0];assert patched[cursor:pos]==data[cursor:pos];cursor=pos+len(b)
  assert patched[cursor:]==data[cursor:]
  hotpcs={r['pc']:r['length'] for path in cfg['roles'][role]['paths'].values() for r in path}
  assert all(at(pc,n)==bytes(patched[text[3]+pc-text[0]:text[3]+pc-text[0]+n]) for pc,n in hotpcs.items())
  results[name]={'kind':mode,'source':role,'binary_sha256':sha(image),'canonical_sha256':sha(original),'first_entry_jmp':hex(first),'normal_exit_jmp':hex(exitjmp),'entry_helper':hex(helper),'cleanup_helper':hex(cleanup),'patches':[{'address':hex(pc),'bytes':b.hex()} for pc,b in chunks],'unsigned_headers_and_all_other_bytes_preserved':True,'all_repeated_path_bytes_and_PCs_preserved':True,'arena':'one RW anonymous page, both600/900 copied identically, bounded until child exit; original program.ops pointer restored before normal Exit','numeric_global_local_only':True};paths[name]=image;print('BUILT',name,flush=True)
for role,image in paths.items():
 for case in cases:
  for n in (23068672,46137344):
   r=subprocess.run([str(image),'--raw',str(case.workload),case.case_id,str(n)],capture_output=True,text=True,timeout=120);assert r.returncode==0,(role,case.id,r.returncode,r.stderr[-1000:]);check=_check_output(case,r.stdout);key=case.id+':'+str(n)
   if key in checks:assert check==checks[key]
   else:checks[key]=check
 print('QUALIFIED',role,flush=True)
(out/'probe-images.json').write_text(json.dumps({'diagnostic_only':True,'timing':False,'images':results,'all_outputs_match':True,'repeated_instruction_count':152,'input_sha256':sha(ROOT/'inputs.json'),'producer_sha256':sha(ROOT/'probe_build.py')},indent=2)+'\n');print('READY',len(results),flush=True)
