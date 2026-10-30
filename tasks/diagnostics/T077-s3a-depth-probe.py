#!/usr/bin/env python3
"""Probe CLI recursion depth and semantic output, never benchmark timing.

The host stack is unchanged. A 4096 probe cap means >=4096, not an engine limit.
Child core dumps are disabled; engine behavior and stack limits are unmodified.
"""
from pathlib import Path
import argparse,hashlib,json,os,resource,subprocess
CASES={
 'forEach':('function walk(d){let s=1;[d].forEach(function(k){if(k>0)s+=walk(k-1);});return s;} walk(DEPTH);',lambda n:str(n+1)),
 'map':('function walk(d){let s=1;[d].map(function(k){if(k>0)s+=walk(k-1);return k;});return s;} walk(DEPTH);',lambda n:str(n+1)),
 'reduce':('function walk(d){return [d].reduce(function(s,k){return s+(k>0?walk(k-1):0);},1);} walk(DEPTH);',lambda n:str(n+1)),
 'call':('function f(n){return n?1+f.call(null,n-1):0;} f(DEPTH);',str),
 'apply':('function f(n){return n?1+f.apply(null,[n-1]):0;} f(DEPTH);',str),
 'reflect_apply':('function f(n){return n?1+Reflect.apply(f,null,[n-1]):0;} f(DEPTH);',str),
 'yield_star':('function* walk(n){if(n)return yield* walk(n-1);return 42;} walk(DEPTH).next().value;',lambda n:'42'),
 'plain_in_callback':('function f(n){return n?1+f(n-1):0;} var result;[0].forEach(function(){result=f(DEPTH);});result;',str),
}
def no_core():resource.setrlimit(resource.RLIMIT_CORE,(0,0))
def probe(binary,program,expected,n):
 source=program.replace('DEPTH',str(n))
 try:
  r=subprocess.run([str(binary),'--raw','-e',source],text=True,capture_output=True,timeout=30,preexec_fn=no_core,env={k:v for k,v in os.environ.items() if not k.startswith('QJS_')})
  ok=r.returncode==0 and r.stdout.strip()==expected(n)
  kind='pass' if ok else ('stack-abort' if 'overflowed its stack' in r.stderr else ('RangeError' if 'RangeError' in r.stderr else 'other-error'))
  return dict(depth=n,ok=ok,kind=kind,exit=r.returncode,stdout=r.stdout,stderr=r.stderr[:500])
 except subprocess.TimeoutExpired:return dict(depth=n,ok=False,kind='timeout')
def measure(binary,case,cap):
 source,expected=CASES[case];trace=[]
 def test(n):
  r=probe(binary,source,expected,n);trace.append(r);return r['ok']
 assert test(0), (case,trace)
 lo=0;hi=16
 while hi<=cap and test(hi):lo=hi;hi*=2
 if lo==cap:return dict(largest_pass=lo,ceiling=False,trace=trace)
 hi=min(hi,cap)
 if hi==cap and lo<hi and test(hi):return dict(largest_pass=hi,ceiling=False,trace=trace)
 while hi-lo>1:
  mid=(lo+hi)//2
  if test(mid):lo=mid
  else:hi=mid
 return dict(largest_pass=lo,first_failure=hi,failure_kind=trace[-1]['kind'] if not trace[-1]['ok'] else next(r['kind'] for r in reversed(trace) if not r['ok']),ceiling=True,trace=trace)
if __name__=='__main__':
 p=argparse.ArgumentParser();p.add_argument('--root',required=True);p.add_argument('--label',required=True);p.add_argument('--output',required=True);p.add_argument('--cap',type=int,default=4096);p.add_argument('--cases',nargs='*',default=list(CASES));args=p.parse_args()
 results=dict(label=args.label,features='default',host_stack='ordinary CLI main thread; no stack override',measure='depth/exit/semantic-output only; no timing',profiles={})
 for profile in ['debug','release']:
  binary=Path(args.root)/profile/'qjs';rows={};results['profiles'][profile]=dict(binary_sha256=hashlib.sha256(binary.read_bytes()).hexdigest(),cases=rows)
  for case in args.cases:
   row=measure(binary,case,args.cap);rows[case]=row
   Path(args.output).write_text(json.dumps(results,indent=2)+'\n')
   print(args.label,profile,case,row['largest_pass'],row.get('failure_kind','cap'),flush=True)
