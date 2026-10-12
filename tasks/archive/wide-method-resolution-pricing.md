# Wide method resolution: pricing before implementation

Revision e54e33eb9371c5cb4627bd8e590a648085a7fa26, 2026-10-12.
R binary SHA-256 6b40aa1944af1096657641cab5b2f80319cc35c3765a79da169cb5c698af09c4;
NG f7830186, SHA-256 cfd8386c3c29b1125a878b8fb82f9627820f2dcc16d2a691c5f8c16ad0b047a0.
No runtime source changes preceded this measurement or the frozen plan.
A was closed/reverted in 86e1a462 before B began; B starts independently at e54.

## Result and decision

| Packet (8-minus-4 increment, never empty-body subtraction) | R instructions | NG instructions | R cycles | NG cycles |
| --- | ---: | ---: | ---: | ---: |
| Cold alternating-receiver method read | 737.621 | 231.255 | 121.107 | 53.975 |
| Matched function-reference move | 338.088 | 170.258 | 69.530 | 41.202 |
| Read minus move: lookup/guards/resolution | 399.533 | 60.997 | 51.578 | 12.774 |
| Known-native loop packet | 1497.503 | 605.954 | 322.074 | 150.334 |
| Same packet unrolled | 1195.691 | 514.163 | 246.491 | 109.432 |
| Loop minus unrolled: control + exit/action/view | 301.812 | 91.791 | 75.583 | 40.902 |

Lookup proxy excess: 338.536 instructions / 38.804 cycles; native-loop overhead
excess: 210.021 instructions / 34.681 cycles. The latter includes loop condition,
induction and branch work as well as native action/view; it is an upper bound
on the action/view component, not a claim that all 210 instructions are removable.
The first reference-only loop fixture was rejected as calibration: it entered
the typed tier and had the same 66 probes at 4/8 steps. No price from that
fixture is used. The final loop uses a reference-carrying dynamic call.

| Target | Method calls | Native probes (upper-bound reach) | Lookup proxy share of instruction / cycle gap | Native-loop proxy share of instruction / cycle gap |
| --- | ---: | ---: | ---: | ---: |
| hash-map | 2070026 | 900022 | 27.5% / 23.3% | 7.4% / 9.0% |
| cdjs | 1320501 | 586428 | 23.7% / 11.0% | 6.5% / 4.4% |
| access-binary-trees | 41264 | 0 | 21.7% / 8.5% | 0.0% / 0.0% |

These are calibration proxies applied to T036 corpus counters, not an additive
exclusive partition of the gap. Receiver layouts and read/call fusion differ.
The old +518.997-instruction / +127.413-cycle method-call premium has precisely
zero native-probe contribution in the call-count increment: every plain/method
4/8-call and 4/8-operation form counted 600000 exit probes (N=600000), while
method function reads scaled with call count; callees contain no loop.
The lookup excess is numerically 65.2% of that instruction premium, but the
remainder is not a measured exclusive receiver-transfer price.

Method lookup carries the larger priced corpus burden and is the only one of
the two components with reach in trees. Select its repeated hot-miss probe,
not native-loop integration. The second probe repeats a failed receiver/layout
proof with no intervening JS or writes. Preserve cold exotic guards, cache scan,
own shadow lookup, prototype validation and accessors. Estimated removable
15-40 instructions / 3-10 cycles per cold read still needs measurement; the
338.5-instruction gap cannot be booked as removable. Native readiness, hand-back
and deopt protocols remain untouched.

## Coverage and protocol

N=200000, five AB/BA pairs; each per-operation price is the median of paired
(T8-T4)/(4*N). Both variants are nonempty, admitted wide bodies. Hardware counters
use uninstrumented exact e54 and NG; the T036 e54 diagnostic binary is reused
only for tier/reach counts, never timed. Its binary SHA-256:
09600e197a629837d874dd40b445dbf295087f4e1c8817a42ef00d3c9626aeec.

With 32 diagnostic iterations: lookup4/8 =128/256 cold function reads, both
33 probes; moves =0 function reads, both33 probes. Final native loop4/8 =
193/321 probes, unrolled =33/33; both =160/288 wide calls, no VM completion
exits and zero typed-loop plan entries. Thus one additional probe per added
loop step, with matched call and body work. Checksum asserts N*(k+1), stdout
contains OK, and every process succeeds on both engines.

One contaminated move pair was discarded and restarted after cargo(57456);
final native measurement waited for cargo(74533). No retained sample overlaps
a detected competing process. Exact commands from this worktree:

```sh
lockf /tmp/qjs-lead/host-timing.lock env PYTHONPATH=. python3 target/method-pricing/measure.py
lockf /tmp/qjs-lead/host-timing.lock env PYTHONPATH=. python3 target/method-pricing/native/measure.py
```

## Reproduction source

The first script also records the invalid typed-loop control above; only lookup
and move prices are used from it. Raw files live in target and are not retained.

```python
from pathlib import Path
import os,sys,json,re,hashlib,statistics,subprocess
from tools.benchmark.screen import sample,counter_tool
from tools.benchmark.measure_lock import wait_for_builds,busy_processes
D=Path(__file__).resolve().parent;N=200000
M=Path('/Users/qingyingliu/Code/js-engine/quickjs-rust')
B={'rust':M/'target/perf-loop/rev-e54e33eb9371c5cb4627bd8e590a648085a7fa26/qjs','ng':M/'third_party/quickjs-ng/build/qjs'}
DIAG=Path('/Users/qingyingliu/Code/js-engine/quickjs-rust-worktrees/wt-quickjs-rust-mixed-call-execution-design-codex/target/mixed-call-followup/build/release/qjs')
def source(name,k,n):
 if name in ('lookup','move'):
  step='g='+('r.f' if name=='lookup' else 'f')+';if(g!==f)throw 1;s=s+1;'
  body='var s=1,g=f;'+step*k+'return s;'
 else:
  step='o=o.next;s=s+1;'
  body='var s=1,o=r;'+('for(var j=0;j<'+str(k)+';j++){'+step+'}' if name=='loop' else step*k)+'return s;'
 return 'function C(){this.next=this;} function marker(){return 1;} C.prototype.f=marker;function kernel(r,f){'+body+'}function run(n,a,f,fn){var s=0;for(var i=0;i<n;i++){s+=fn(a[i&1],f);}return s;}var a=[new C(),new C()];var answer=run('+str(n)+',a,marker,kernel);if(answer!=='+str(n*(k+1))+')throw answer;print("OK");'
for name in ('lookup','move','loop','straight'):
 for k in (4,8):
  (D/f'{name}-{k}.js').write_text(source(name,k,N))
  (D/f'{name}-{k}-diag.js').write_text(source(name,k,32))
if len(sys.argv)>1 and sys.argv[1]=='generate':sys.exit()
while wait_for_builds(60):pass
counts={}
for name in ('lookup','move','loop','straight'):
 for k in (4,8):
  p=D/f'{name}-{k}-diag.js';r=subprocess.run([str(DIAG),'--raw',str(p)],capture_output=True,text=True,env={**os.environ,'QJS_PERF_COUNTERS':'1','QJS_CF_TRACE':'1'},timeout=30)
  assert r.returncode==0,(name,k,r.stderr[-500:]);assert r.stdout=='OK\nundefined\n'
  (D/f'{name}-{k}.trace').write_text(r.stdout+r.stderr)
  counts[f'{name}-{k}']={a:int(b) for a,b in re.findall(r'^\s+(\w+) (\d+)\s*$',r.stderr,re.M)}
(D/'counts.json').write_text(json.dumps(counts,indent=2))
print('diagnostics', {n:{k:v for k,v in c.items() if k in ('mix_wide_exit_probes','mix_wide_exits','mix_wide_read_function','mix_wide_read_slow','mix_wide_calls','mix_wide_root_entries')} for n,c in counts.items()},flush=True)
tool=counter_tool();raw=[]
for name in ('lookup','move','loop','straight'):
 rows=[]
 for pair in range(5):
  while True:
   while wait_for_builds(60):pass
   row={'name':name,'pair':pair,'samples':{}};dirty=False
   for k in ((4,8) if pair%2==0 else (8,4)):
    row['samples'][str(k)]={}
    for role in (('rust','ng') if pair%2==0 else ('ng','rust')):
     if busy_processes():dirty=True;break
     x=sample(tool,[str(B[role]),str(D/f'{name}-{k}.js')],60)
     if busy_processes():dirty=True;break
     assert 'OK\n' in x.stdout;row['samples'][str(k)][role]={m:getattr(x,m) for m in ('instructions','cycles','wall_ns')}
    if dirty:break
   if not dirty:break
   print('discard contaminated pair',name,pair,flush=True)
  raw.append(row);(D/'raw.json').write_text(json.dumps(raw,indent=2))
 print('measured',name,flush=True)
prices={}
for name in ('lookup','move','loop','straight'):
 prices[name]={role:{m:statistics.median((r['samples']['8'][role][m]-r['samples']['4'][role][m])/(4*N) for r in raw if r['name']==name) for m in ('instructions','cycles')} for role in B}
(D/'prices.json').write_text(json.dumps(prices,indent=2));print(json.dumps(prices,indent=2))
```

The native script is the same source, with all four name lists replaced by
`('loop','straight')`, body step replaced with `o=o.next;s=s+f(o);`,
and marker body replaced with `return o.next===null?0:1;`. It writes under
`target/method-pricing/native/` and uses the same N, pairs, checks and engines.
