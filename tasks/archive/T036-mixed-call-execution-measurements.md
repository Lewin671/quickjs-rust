# T036 design measurements at e54e33eb

Design-only record, 2026-10-12. No runtime candidate or before/after claim.
[T036](../T036-mixed-call-execution-design.md) owns next action;
[the design](../../docs/design/mixed-call-execution.md) owns architecture.
All prices below are **effective JavaScript-level increments**, not exclusive
native-symbol cycle attribution. The fitted call envelope includes callee
entry, fixed seed reads, receiver/method resolution, argument/result transfer,
return/cleanup and caller accumulation. Hardware counters alone cannot split
those internals without an instrumented perturbation. Do not relabel a flat
sampler fraction as a dispatch or clone cycle count.

## Identity and existing corpus evidence

All new timing reuses the unchanged queue-candidate executable:
`<main>/target/perf-loop/rev-e54e33eb9371c5cb4627bd8e590a648085a7fa26/qjs`,
SHA-256 `6b40aa1944af1096657641cab5b2f80319cc35c3765a79da169cb5c698af09c4`.
NG pin `f7830186043e4488f2998759d60a514faf07cbc9`;
`<main>/third_party/quickjs-ng/build/qjs` is the timing reference.
The existing counter binary `<main>/target/profiles/brief01-e54e33eb/build/release/qjs`
is diagnostic only, never timed. No cargo builds were needed.
Bundle/queue hashes and readiness are recorded once in
[T018](../T018-broad-performance.md#standing-at-e54e33eb).

Brief 01 amplified samples (`profile-attribution.json`, local, not retained):

| Target | Wall/NG | Wide flat samples | Flat Value clone | Flat property read |
| --- | ---: | ---: | ---: | ---: |
| hash-map | 1.3581 | 729/1457 = 50.03% | 4.87% | 4.60% |
| cdjs | 1.3488 | 568/1334 = 42.58% | 4.80% | 4.20% |
| 3d-raytrace | 1.3340 | 796/1304 = 61.04% | 4.22% | 7.75% |
| access-binary-trees | 1.3058 | 438/995 = 44.02% | 5.03% | 4.02% |
| string-validate-input | 1.2447 | 237/986 = 24.04% | 2.64% | 6.09% |

Flat groups are separate sampler bins; inclusive callers overlap and are not
summed. Samples of amplified scripts are reach evidence, not measured savings.
Existing method-mutation N/2N micro: 407.86/242.97 cycles and
2269.97/967.21 instructions per iteration, Rust/NG (1.679 cycles ratio).
That one-operation shape cannot price fixed call and body separately.

## Nonempty 8-minus-4 cost model

For every kernel, hold N=300,000 outer iterations and measure C=4/8 calls per
iteration and K=4/8 copies of the nonempty operation per callee. Receivers
alternate between two same-prototype objects; values/handles are passed as
inputs. All four (C,K) forms must run the same wide callee and wide caller.
Use five warmed, alternating engine pairs per form, exact result assertions,
`/usr/bin/time -l` counters, and the same paths for both engines.

For median total cycles or instructions T(C,K):
`D4 = (T(8,4)-T(4,4))/(4*N)` and
`D8 = (T(8,8)-T(4,8))/(4*N)`.
Marginal operation `B=(D8-D4)/4`; fixed call envelope `A=2*D4-D8`.
Thus `D4=A+4*B`. Outer loop/startup/parsing cancels in C; callee work is
priced by K. **No empty function or empty loop subtraction.** Negative or
unstable residuals disprove an additive subcomponent interpretation.

Kernel bodies are retained verbatim in the reproducer below. Their roles:
scalar and muladd price scalar arithmetic in an otherwise boxed body; read
adds a numeric own-field load; write prices numeric own-field mutation;
hash prices linked-entry value read, next-handle transport and null branch;
vector prices six numeric field loads and three multiply/add steps; tree
prices item read and branch-dependent child-handle transport; string prices
numeric-to-string builtin invocation plus length and accumulation. The latter
represents validate-input's makeNumber/conversion path, not its RegExp path.
These isolate operation families, not full workload algorithms; recursive
allocation in trees and CDjs's tree/geometry mix need separate pricing.

## Retained component prices

All values rounded to two decimals for reproduction, not claimed precision.
A/B are fixed call-envelope/marginal nonempty-step costs; instructions are
retired instructions, not dispatched bytecodes. There is no implemented
candidate: these are Rust-base versus NG, not before/after migration.

| Representative target / kernel | A cycles Rust / NG | A instructions Rust / NG | B cycles Rust / NG | B instructions Rust / NG |
| --- | ---: | ---: | ---: | ---: |
| Boxed-body scalar add | 286.12 / 91.04 | 1627.02 / 628.75 | 15.30 / 21.04 | 96.92 / 35.10 |
| Boxed-body multiply/add | 182.71 / 131.76 | 1631.82 / 629.03 | 42.41 / 24.76 | 114.19 / 81.05 |
| Numeric own-field read/add | 255.79 / 101.13 | 1626.89 / 628.90 | 38.32 / 26.89 | 183.99 / 100.05 |
| CDjs / field mutation + reload | 340.34 / 193.87 | 1617.59 / 750.22 | 71.84 / 46.65 | 490.80 / 232.05 |
| hash-map / entry read + null branch + next | 347.45 / 178.65 | 1621.01 / 619.22 | 144.06 / 58.55 | 815.95 / 239.22 |
| 3d-raytrace / six loads + three multiply/adds | 314.24 / 163.12 | 1609.46 / 613.82 | 242.65 / 106.00 | 1535.14 / 532.06 |
| binary-trees / item + conditional child | 338.90 / 160.83 | 1642.67 / 629.28 | 189.74 / 76.74 | 1093.11 / 291.04 |
| validate-input / String(number) + length/add | 443.02 / 174.54 | 1644.05 / 628.83 | 284.92 / 182.23 | 1879.15 / 1191.14 |

The 8-minus-4 **differential components** below price the added source work
in the same wide lowering class. They are effective substitutions, not an
exclusive native-symbol partition. W includes the write and extra reload;
H/T include handle moves, branch and child/next load. S remains explicit so
already-fast scalar arithmetic is not mislabelled a removable boxing cost.

| Named source component | Cycles Rust / NG | Instructions Rust / NG | Formula |
| --- | ---: | ---: | --- |
| S: scalar add/accumulate | 15.30 / 21.04 | 96.92 / 35.10 | `B(scalar)` |
| M: additional multiplication | 27.11 / 3.71 | 17.27 / 45.96 | `B(muladd)-S` |
| R: own numeric field load replacing argument read | 23.02 / 5.85 | 87.07 / 64.95 | `B(read)-S` |
| W: field store/reload replacing local accumulation | 33.52 / 19.75 | 306.82 / 132.01 | `B(write)-B(read)` |
| H: null branch + next-handle transport | 105.74 / 31.66 | 631.97 / 139.17 | `B(hash)-B(read)` |
| T: child choice + reference transport | 151.42 / 49.84 | 909.12 / 190.99 | `B(tree)-B(read)` |
| G: builtin resolve/invoke + number conversion + length | 269.63 / 161.18 | 1782.24 / 1156.04 | `B(string)-S` |

The additive ledgers for one four-step call are:
hash `A_hash+4*(S+R+H)`; CDjs mutation `A_write+4*(S+R+W)`;
tree `A_tree+4*(S+R+T)`; conversion `A_string+4*(S+G)`.
These reproduce measured D4 by construction. Vector is deliberately one
joint component: `A_vector+4*B_vector`. Transferring primitive prices gives
`B_vector-3*B_muladd-6*R = -22.69 Rust / -3.37 NG cycles` (instructions
670.16 / -100.79). **That falsifies an additive independent-primitive cycle
model for vector expressions**, not negative execution cost. Do not assign
those residual cycles to a removable helper. Load ordering, liveness,
intermediate transport and codegen must be replayed jointly in stage 2.
Instruction increments are stable across the discarded short-window trial;
cycle intercept/slope attribution varies, so compare joint D4 as well.

Diagnostic proof at the same source count N=300,000: all 32 forms report
`CFOK wide params=(n,a,vec)` and `CFOK wide params=(v)`; no callee CFEXIT
or CFDECLINE. Ordinary calls are exactly 1,200,003 / 2,400,003 at C=4/8.
The caller's initial typed-loop attempt may deopt and hand back once; it
must not be confused with recurring callee exits. Timing uses the standard
binary; diagnostic numbers below prove coverage, never performance.

| Kernel, C=4 | Compact ops K=4 | Compact ops K=8 | Compact direct calls K=4 / K=8 |
| --- | ---: | ---: | ---: |
| scalar | 23100004 | 32700004 | 1199996 / 1199996 |
| muladd | 23100004 | 32700004 | 1199996 / 1199996 |
| read | 23100004 | 32700004 | 1199996 / 1199996 |
| write | 32700004 | 51900004 | 1199996 / 1199996 |
| hash | 47100004 | 80700004 | 1199996 / 1199996 |
| vector | 74700037 | 137100037 | 1200000 / 1200000 |
| tree | 47100004 | 80700004 | 1199996 / 1199996 |
| string | 37500004 | 61500004 | 1199996 / 1199996 |

## What the migration proposes to remove (unverified budget)

| Component | Proposed saving in Rust cycles | Retained work / reason for cap |
| --- | ---: | --- |
| A: call envelope | 70-110 per admitted call | Method/callee guards, two fixed seed reads and caller accumulation survive. Target the ABI transport, seeding/clearing bookkeeping; do not count the full A or existing CallEnv bypass as savings. |
| S: scalar add | 0 per step | Already 15.30 vs NG 21.04; instruction excess alone is not a cycle opportunity. |
| M: extra multiplication | 10-15 per step | Keep operands scalar; retain JS numeric rules and the multiplication. |
| R: own numeric load | 5-8 per load | Remove Value temporary/result conversion; keep shape/property guards and actual read. |
| W: mutation + reload | 8-12 per step | Scalar arithmetic/result transport; publish the actual store immediately, no transactional field cache. |
| H: entry hop group | 15-25 per step | Avoid dead-handle clones/moves; keep pointer loads, branch and live alias ownership. |
| T: child-choice group | 15-25 per step | Same; no recursion or allocation elimination. |
| Vector joint step | 35-60 per step | Mixed numeric intermediates and fewer boxed numeric transports; joint replay must justify this instead of adding primitive estimates. |
| G: conversion group | 10-20 per step | Scalar builtin boundary only; allocation, decimal formatting, boxed string result and length remain. No whole native invocation counted as removed. |

These are experiment budgets, **not measured savings**. With four steps/call,
proposed savings are hash 150-242 cycles of ~924, mutation 122-190 of ~628,
vector 210-350 of ~1285, tree 150-242 of ~1098, conversion 110-190 of ~1583.
Multiplying those fractions by the corresponding wide-flat reach gives
roughly hash 8-13%, CDjs mutation 8-13%, vector 10-17%, tree 6-10%,
conversion 2-3%; rounded scenario ranges below allow small mix/transport contributions outside
that one kernel. Those are conditional, not extra measured dispatch savings.
This is the evidence behind revising the earlier blanket 10-20% / 5-12%
planning ranges downward. Eligible-operation frequencies are still missing;
no benefit may be frozen on these sample fractions alone.

## Coverage and invalid attempts

The first generator put objects inside the caller. Virtual-object lowering
made the caller decline wide; the scalar/read/write/hash preliminary call
intercept was ~600-730 cycles, largely standalone entry. These numbers are
**excluded**, not evidence for a wide-to-wide ABI win. Pass objects into the
caller instead, then check C/K admission and counts before retaining prices.
An early generated source had a malformed repeated-call suffix; the diagnostic
returned `error: expected expression`. It was corrected before any timing.

Build-contention attempts exited rather than retain polluted samples:
`RETRY: competing processes remain ['cargo(27714)', 'cargo(28172)', 'rustc(28182)']`
and `RETRY: competing processes remain ['cargo(77450)', 'rustc(77451)', 'cargo(77901)']`.
Another partial attempt stopped with
`RETRY: concurrent process ['cargo(83099)', 'rustc(83205)']`.
A completed-form warning discards and repeats that entire form, with five fresh
pairs after builds settle. An N=60,000 trial was additionally discarded for
short increments; retained measurements use N=300,000. No partial run is a
formal comparison, screen decision or attempt on a runtime plan.

## Payoff model and parity requirements

For planning only, `new_ratio = old_ratio * (1-net_saving)`; application of a
CPU saving to a wall ratio assumes no new off-CPU/startup cost. It is not a
new NG measurement. Corpus traces must weight the eligible mix before freezing.

| Target | Planning net saving | Projected wall/NG | Saving needed for <=1 | Required reduction of wide flat bin if it alone pays |
| --- | ---: | ---: | ---: | ---: |
| hash-map | 8-14% | 1.1680-1.2495 | 26.37% | 52.7% |
| cdjs | 8-14% | 1.1600-1.2409 | 25.86% | 60.7% |
| 3d-raytrace | 10-18% | 1.0939-1.2006 | 25.04% | 41.0% |
| access-binary-trees | 6-11% | 1.1622-1.2275 | 23.42% | 53.2% |
| string-validate-input | 2-4% | 1.1949-1.2198 | 19.66% | 81.8% |

These are scenario ranges, **not confidence intervals**. First four have
43-61% wide flat reach, but only numeric transport and avoidable call copies
are removable; native helpers, pointer dereferences and valid guards remain.
Validate has only 24% wide flat reach and mixed string/native work; its lower
estimate is intentional. The broad same-host qualification remains open.

For all five to reach parity, measured eligible mix times measured per-step
savings, less entry/exit/reentry tax, must exceed each needed saving above.
Validate would need ~82% of its wide flat time removed if nothing else improves;
that is unsupported by this design. Migration alone is not a credible promise
of all-five <=1.0. The measured conversion scenario also fails a five-target
0.95/base payoff ceiling before implementation: recommend validate as a
mandatory control, with a separate conversion-pricing unit, rather than
building three stages before discovering that failure. This scope decision
is for the lead; the draft proposes it and is not frozen. A larger program would additionally need shape-slot/reference
transport and allocation improvements for map/CDjs/tree/raytrace, and pricing
then optimizing numeric-to-string/string-builder/native-boundary work for
validate. Do not resurrect a rejected cache or clone trick without a new
profile and different economics. A compact safe handle representation/GC
migration is a separate architecture decision, not hidden in this unit.

## Exact commands and retained reproducer

From the design worktree, extract the sole Python fence below to
`target/mixed-call-design/measure.py` (create the directory first). It imports
unchanged repository counter helpers; it is a measurement artifact, not engine
or tool implementation. Replace MAIN only when locating the saved binaries.

```sh
lockf /tmp/qjs-lead/host-timing.lock env PYTHONPATH=. \
  python3 target/mixed-call-design/measure.py measure
lockf /tmp/qjs-lead/host-timing.lock env PYTHONPATH=. \
  python3 target/mixed-call-design/measure.py diagnose
```

Retained N=300,000 run: 32 forms, five pairs each; minimum C increment
62.56 ms (>50 ms). Raw counters and generated scripts are local artifacts,
not retained. Conclusion and full reproducer are retained here. SHA-256:
- `raw.json`: `b669111b964f44fa6c2cb46ab75586a589d2970bb75a5ff6960bb952d8c19349`.
- `costs.json`: `ed24755f34d841b7533a4e3d285401eb18daf2c9d10bcf2cb0aee500a111ba4c`.
- `measure.py`: `274dddeeb91d09770e2c60e568f790a2e3923f4dd75bc420ff75032073cafe44`.
- NG executable: `cfd8386c3c29b1125a878b8fb82f9627820f2dcc16d2a691c5f8c16ad0b047a0`.

```python
import sys, os, json, statistics, hashlib, subprocess
from pathlib import Path
from tools.benchmark.screen import counter_tool, sample
from tools.benchmark.measure_lock import wait_for_builds, busy_processes
OUT=Path(__file__).resolve().parent
MAIN=Path('/Users/qingyingliu/Code/js-engine/quickjs-rust')
BINS={'rust':MAIN/'target/perf-loop/rev-e54e33eb9371c5cb4627bd8e590a648085a7fa26/qjs','ng':MAIN/'third_party/quickjs-ng/build/qjs'}
DIAG=MAIN/'target/profiles/brief01-e54e33eb/build/release/qjs'
N=300000
# A component is an increment in a nonempty callee, never an empty-body subtraction.
KERNELS={
 'scalar': 's=s+v;',
 'muladd': 's=s+v*v;',
 'read': 's=s+this.x;',
 'write': 'this.x=this.x+v;s=this.x;',
 'hash': 'if(o!==null){s=s+o.value;o=o.next;}',
 'vector': 's=s+this.x*v.x+this.y*v.y+this.z*v.z;',
 'tree': 's=s+o.item;o=o.left===null?o.right:o.left;',
 'string': 's=s+String(v).length;',
}
def source(name,c,k):
 body=KERNELS[name]*k
 expected=N*c*(1+k/2) if name in ('scalar','muladd') else N*c*(1+14*k) if name=='vector' else N*c+k*(N//2*c)*(N//2*c+1)//2 if name=='write' else N*c*(1+k)
 param='v'
 arg='(i&1)'
 if name=='vector':arg='vec'
 init='var s=this.seed,o=this.head;'
 # Both 4/8 forms have a boxed receiver and the same lowering class.
 return ('function C(){this.seed=1;this.x=1;this.y=2;this.z=3;this.head=null;}\n'
 +'C.prototype.f=function('+param+'){'+init+body+'return s;};\n'
 +'function run(n,a,vec){var s=0;for(var i=0;i<n;i++){var r=a[i&1];'
 +(('s+=r.f('+arg+');')*c)+'}if(!(s>0))throw 1;return s;}\n'
 +'var a=[new C(),new C()],vec={x:1,y:2,z:3},h={value:1,item:1,next:null,left:null,right:null};h.next=h;h.left=h;a[0].head=h;a[1].head=h;var result=run('+str(N)+',a,vec);if(result!=='+str(int(expected))+')throw result;print("OK");\n')
for name in KERNELS:
 for c in (4,8):
  for k in (4,8):
   (OUT/f'{name}-{c}-{k}.js').write_text(source(name,c,k))
if len(sys.argv)>1 and sys.argv[1]=='generate':sys.exit()
busy=wait_for_builds(60)
if busy:raise SystemExit('RETRY: competing processes remain '+str(busy))
mode=sys.argv[1] if len(sys.argv)>1 else 'measure'
if mode=='diagnose':
 for name in KERNELS:
  for c,k in ((4,4),(4,8),(8,4),(8,8)):
   p=OUT/f'{name}-{c}-{k}.js'
   r=subprocess.run([str(DIAG),str(p)],capture_output=True,text=True,env={**os.environ,'QJS_PERF_COUNTERS':'1','QJS_CF_TRACE':'1'})
   (OUT/f'{name}-{c}-{k}.trace').write_text(r.stdout+r.stderr)
   if r.returncode:raise SystemExit(r.stderr)
 print('diagnostics done');sys.exit()
tool=counter_tool(); rows=[]
for name in KERNELS:
 for c in (4,8):
  for k in (4,8):
   p=OUT/f'{name}-{c}-{k}.js'
   while True:
    busy=wait_for_builds(60)
    if busy:
     print('RETRY FORM',name,c,k,'competing processes remain',busy,flush=True)
     continue
    for binary in BINS.values():sample(tool,[str(binary),str(p)],30)
    row={'name':name,'calls':c,'ops':k,'n':N,'samples':{'rust':[],'ng':[]}}
    dirty=False
    for pair in range(5):
     for e in (('rust','ng') if pair%2==0 else ('ng','rust')):
      if busy_processes():dirty=True;break
      t=sample(tool,[str(BINS[e]),str(p)],30)
      if busy_processes():dirty=True;break
      if 'OK' not in t.stdout:raise SystemExit('bad output')
      row['samples'][e].append({m:getattr(t,m) for m in ('cycles','instructions','wall_ns')})
     if dirty:break
    if dirty:
     print('RETRY FORM',name,c,k,'concurrent process; discard entire form',flush=True)
     continue
    break
   rows.append(row);(OUT/'raw.json').write_text(json.dumps(rows,indent=2))
 print('finished',name,flush=True)
res={}
for name in KERNELS:
 res[name]={}
 for e in BINS:
  res[name][e]={}
  for metric in ('cycles','instructions','wall_ns'):
   med={(r['calls'],r['ops']):statistics.median(x[metric] for x in r['samples'][e]) for r in rows if r['name']==name}
   d4=(med[8,4]-med[4,4])/(4*N);d8=(med[8,8]-med[4,8])/(4*N)
   op=(d8-d4)/4; envelope=2*d4-d8
   res[name][e][metric]={'op':op,'envelope':envelope,'call4':d4,'call8':d8}
(OUT/'costs.json').write_text(json.dumps(res,indent=2))
print(json.dumps(res,indent=2))
```
