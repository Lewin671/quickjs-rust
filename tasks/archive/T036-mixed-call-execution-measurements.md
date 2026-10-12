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

## Lead follow-up 1: corpus-mix pricing (written before attribution/replay)

2026-10-12, production baseline still e54e33eb; no runtime candidate. The lead
withheld approval for the four-stage program and authorized this sequence:
corpus pricing -> instruction attribution -> only conditionally an ABI replay.
Validate remains a mandatory control. This section supersedes the blanket
payoff ranges as a work-selection basis; it does not edit the earlier record.

The exact five official bundles were SHA-checked against brief 01 raw rows.
Their stdout matches the saved candidate rows. Standard candidate/NG medians
below come from all 30 measurement blocks of that sealed run; diagnostics
are never timed. Temporary counters count by executed op and actual Value
kind; read/write classification includes the wide hot path, which the old
named_property counters omitted. Object allocations include ordered builders;
ObjectRef clones are whole-engine copies, compact-register drops are a narrower
inventory and are not represented as all engine drops.

The original joint micro prices were insufficient: hash has 1,260,002 wide
reference/null truthiness branches but only 19 reference/null CompareJumps;
all five have zero matched numeric read-modify-store-**reread** packets. The
307/132-instruction differential was not a price for every numeric store.
We added nonempty 4/8 atom and ordinary-vs-method call calibrations (N=600,000,
five alternating pairs); their counter traces prove the executed tier.
`numericplain` runs on the closed-form/loop helper path, not NumericPlan:
its price is explicitly excluded from pricing 990,018/584,728 NumericPlan
entries. Likewise a next-load price is not multiplied by every reference move.

### Pricing ledger and limits

For metric m, `gap_m = median(R_m)-median(NG_m)` and component share is
`100 * frequency * (price_R-price_NG) / gap_m`. The signed residual is the
whole gap minus all priced contributions. No clipping to 0/100, renormalizing,
or calling a priced overhead an achievable saving. Reused prices measure
specific source operations, not exclusive native-symbol cost. Matching by
operation/value/cache path is narrower than treating every wide op alike.

Activation-envelope price is ordinary wide call A minus one caller scalar
accumulation price S. It still includes a constant return/prologue and argument
transport; it is not entirely a storage-ABI cost. Method premium is same
reference-argument callee method A minus ordinary A, not the whole method A.
NumericPlan, constructors, generic/typed calls, raw reference moves, field
stores without the matched reread, binary-valued null tests, allocations,
other arithmetic, string/native work and startup remain unpriced residual.

| Counter-priced component | R-minus-NG instructions/event | R-minus-NG cycles/event |
| --- | ---: | ---: |
| activation | 357.801 | 139.753 |
| method_lookup | 518.997 | 127.413 |
| ref_hot | 61.023 | 12.667 |
| ref_cold | 201.437 | 22.558 |
| number_hot | 54.052 | 6.866 |
| number_cold | 183.996 | 19.719 |
| null | 156.994 | 45.281 |
| truth | 59.227 | 29.390 |
| identity | 69.050 | 31.456 |
| reread_packet | 174.810 | 13.769 |
| add | 61.817 | -5.746 |
| mul | -28.684 | 23.396 |

Truth/identity prices subtract the scalar accumulation of their nonempty
micro. These are effective differences, not physical branch-only timings.
Cold reference/numeric prices are literal/slot cold-cache proxies; cache-kind
hit counting in the next phase tests their transfer to actual OwnSlot/Exact
paths. The unrelated String(number)+length price has zero matched invocations:
validate performs 16,000 numeric formats through concat and 30,001 length
queries; it needs the separate conversion/length unit the lead requested.

### Gap shares: instructions (percent of the net R-minus-NG gap)

| Target | Activation envelope | Method lookup/receiver premium | Ref fields | Numeric fields | Null/truth/identity | Reread packet | Add/mul | Unexplained residual |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| hash-map | 29.0 | 42.1 | 19.2 | 15.9 | 2.9 | 0.0 | 1.5 | -10.7 |
| cdjs | 36.2 | 36.4 | 11.2 | 13.6 | 4.4 | 0.0 | 1.5 | -3.3 |
| 3d-raytrace | 11.3 | 11.2 | 9.1 | 20.2 | 0.0 | 0.0 | 4.0 | 44.3 |
| access-binary-trees | 45.9 | 33.3 | 5.9 | 3.5 | 10.3 | 0.0 | 0.1 | 1.1 |
| string-validate-input | 9.9 | 0.0 | 0.0 | 7.6 | 0.0 | 0.0 | -1.0 | 83.4 |

### Same ledger: cycles (percent; transfer is not a closed partition)

| Target | Activation envelope | Method premium | Ref fields | Numeric fields | Null/truth/identity | Reread packet | Add/mul | Signed residual |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| hash-map | 83.8 | 76.4 | 15.9 | 13.0 | 10.7 | 0.0 | -1.0 | -98.7 |
| cdjs | 57.4 | 36.2 | 9.1 | 6.6 | 8.7 | 0.0 | 0.0 | -18.1 |
| 3d-raytrace | 30.6 | 19.0 | 7.2 | 16.7 | 0.1 | 0.0 | 21.4 | 5.1 |
| access-binary-trees | 61.5 | 28.0 | 4.2 | 1.5 | 10.2 | 0.0 | 4.5 | -9.9 |
| string-validate-input | 17.1 | 0.0 | 0.0 | 3.6 | 0.0 | 0.0 | 3.4 | 75.9 |

**The cycle prices over-explain three cases, dramatically hash-map.** This
falsifies reading these proxies as an exclusive causal cycle partition or
summing them into a parity forecast. Instruction accounting is much closer,
but hash/CDjs still have negative residuals; omitted faster paths and
callee/cache/liveness differences must be measured, not assumed. A negative
residual is neither a negative runtime cost nor spare optimization budget.
Raytrace and validate retain large instruction residuals. No combination
priced here is verified to bring any full case to parity.

For the conditional replay gate, we use the **instruction gap**, the metric
whose call envelope is stable across kernels: activation envelope alone
prices 29.0%/36.2% in hash/CDjs (even excluding method premium). The gate's
>=10% in two targets is met as a diagnostic pricing condition, not a proven
cycle saving. If the lead intended a closed cycle partition instead, that
stronger condition is not established; the standalone replay cannot authorize
runtime integration in either interpretation.

### Exact inventory (wide/typed and whole-engine scope stated separately)

| Target | Wide plain/method calls | NumericPlan plain/method entries | Constructor inline (compact minus wide/numeric Value driver) | General frames / direct route markers | Allocated object/array |
| --- | ---: | ---: | ---: | ---: | ---: |
| hash-map | 0/2070026 | 990018/0 | 90003 | 1/15 | 90698/20 |
| cdjs | 587824/1320501 | 584709/19 | 254068 | 18038/112917 | 379686/10273 |
| 3d-raytrace | 18086/38498 | 0/0 | 0 | 1/43 | 789/10667 |
| access-binary-trees | 41264/41264 | 0/0 | 42072 | 0/1616 | 42726/2 |
| string-validate-input | 20000/0 | 0/0 | 0 | 0/1 | 4657/5 |

Direct route markers overlap standalone/compact admissions; they are not a
second set of physical frame allocations. Constructor difference also
contains admitted non-call constructions and is kept separate from the
ordinary-call A price. Full exact counters are retained below, including
field read/write kinds, reference moves, null/identity/truth tests, clear
width/live handles, numeric ops, builtin events and cache paths.

```json
{
 "3d-raytrace": {
  "authoritative_assign_local_hits": 1905,
  "authoritative_load_local_hits": 6127,
  "authoritative_store_local_hits": 1501,
  "call_env_marker_scrubs": 0,
  "closed_form_leaf_evaluations": 0,
  "compact_caller_env_calls": 0,
  "compact_direct_calls": 56584,
  "compact_function_ops": 3557887,
  "compact_standalone_activations": 42,
  "computed_property_reads": 3698,
  "computed_property_writes": 164,
  "declined_loop_plan_edges": 965,
  "direct_leaf_frames": 43,
  "dispatched_assign_local_ops": 1905,
  "dispatched_branch_return_ops": 1005,
  "dispatched_call_construct_ops": 80,
  "dispatched_computed_property_ops": 1931,
  "dispatched_general_ops": 1146,
  "dispatched_global_binding_ops": 49,
  "dispatched_load_const_ops": 2167,
  "dispatched_load_local_ops": 6173,
  "dispatched_local_binding_ops": 9581,
  "dispatched_named_property_ops": 26,
  "dispatched_numeric_ops": 4769,
  "dispatched_stack_ops": 2160,
  "dispatched_store_local_ops": 1503,
  "executed_ops": 22914,
  "generic_call_frames": 1,
  "loop_backedges": 998,
  "loop_plan_entries": 33,
  "mix_array_alloc": 10667,
  "mix_cache_hot_other": 534482,
  "mix_cache_hot_reject_exact": 27042,
  "mix_cache_hot_reject_literal": 0,
  "mix_cache_hot_reject_ownslot": 156902,
  "mix_cache_probe_calls": 157740,
  "mix_cache_read_exact": 43788,
  "mix_cache_read_literal": 0,
  "mix_cache_read_ownslot": 78451,
  "mix_cache_read_shared": 60266,
  "mix_compact_numeric_calls": 0,
  "mix_hot_number": 395992,
  "mix_hot_other": 3404,
  "mix_hot_reference": 4438,
  "mix_number_format": 2700,
  "mix_numeric_method_calls": 0,
  "mix_numeric_plain_calls": 0,
  "mix_numeric_plan_entries": 0,
  "mix_numeric_plan_nested": 0,
  "mix_object_alloc": 789,
  "mix_object_clone": 214255,
  "mix_object_drop": 281595,
  "mix_slow_number": 80549,
  "mix_slow_other": 38724,
  "mix_slow_reference": 79523,
  "mix_string_length": 2,
  "mix_string_number": 0,
  "mix_string_other": 0,
  "mix_string_wrapper_number": 0,
  "mix_string_wrapper_other": 0,
  "mix_typed_identity_compare": 0,
  "mix_typed_null_compare": 0,
  "mix_typed_null_truthy": 0,
  "mix_typed_read_number": 0,
  "mix_typed_read_other": 0,
  "mix_typed_read_reference": 0,
  "mix_typed_ref_moves": 0,
  "mix_typed_ref_truthy": 0,
  "mix_typed_write_number": 0,
  "mix_typed_write_other": 0,
  "mix_wide_argument_numbers": 55565,
  "mix_wide_argument_references": 104240,
  "mix_wide_binary_identity_compare": 0,
  "mix_wide_binary_null_compare": 35000,
  "mix_wide_call_registers": 907176,
  "mix_wide_calls": 56584,
  "mix_wide_clear_live": 551503,
  "mix_wide_clear_ref": 124462,
  "mix_wide_clear_slots": 907909,
  "mix_wide_exit_probes": 74807,
  "mix_wide_exits": 0,
  "mix_wide_fallback_calls": 2298,
  "mix_wide_identity_compare": 0,
  "mix_wide_max_depth": 5,
  "mix_wide_method_calls": 38498,
  "mix_wide_null_compare": 440,
  "mix_wide_null_local": 0,
  "mix_wide_null_temp": 0,
  "mix_wide_null_truthy": 538,
  "mix_wide_numeric_calls": 0,
  "mix_wide_other_compare": 86673,
  "mix_wide_plain_calls": 18086,
  "mix_wide_read_function": 41590,
  "mix_wide_read_hot": 403834,
  "mix_wide_read_null": 0,
  "mix_wide_read_number": 476541,
  "mix_wide_read_other": 538,
  "mix_wide_read_reference": 83961,
  "mix_wide_read_slow": 198796,
  "mix_wide_read_string": 0,
  "mix_wide_ref_moves": 200117,
  "mix_wide_ref_truthy": 0,
  "mix_wide_returns": 56626,
  "mix_wide_rmw": 0,
  "mix_wide_rmw_reread": 0,
  "mix_wide_root_entries": 42,
  "mix_wide_scalar_add": 253974,
  "mix_wide_scalar_mul": 299455,
  "mix_wide_scalar_other": 313627,
  "mix_wide_take_ref_copy": 0,
  "mix_wide_take_ref_move": 0,
  "mix_wide_write_number": 140,
  "mix_wide_write_other": 0,
  "mix_wide_write_reference": 154,
  "named_property_reads": 28,
  "named_property_writes": 24,
  "native_calls": 2796,
  "nested_vm_constructions": 9,
  "ordinary_call_attempts": 56628,
  "same_vm_frame_entries": 0
 },
 "access-binary-trees": {
  "authoritative_assign_local_hits": 0,
  "authoritative_load_local_hits": 426,
  "authoritative_store_local_hits": 2491,
  "call_env_marker_scrubs": 0,
  "closed_form_leaf_evaluations": 0,
  "compact_caller_env_calls": 0,
  "compact_direct_calls": 124600,
  "compact_function_ops": 1296152,
  "compact_standalone_activations": 1616,
  "computed_property_reads": 0,
  "computed_property_writes": 0,
  "declined_loop_plan_edges": 412,
  "direct_leaf_frames": 1616,
  "dispatched_assign_local_ops": 0,
  "dispatched_branch_return_ops": 422,
  "dispatched_call_construct_ops": 1621,
  "dispatched_computed_property_ops": 0,
  "dispatched_general_ops": 473,
  "dispatched_global_binding_ops": 1229,
  "dispatched_load_const_ops": 547,
  "dispatched_load_local_ops": 4109,
  "dispatched_local_binding_ops": 6600,
  "dispatched_named_property_ops": 814,
  "dispatched_numeric_ops": 2055,
  "dispatched_stack_ops": 3331,
  "dispatched_store_local_ops": 2491,
  "executed_ops": 17092,
  "generic_call_frames": 0,
  "loop_backedges": 412,
  "loop_plan_entries": 0,
  "mix_array_alloc": 2,
  "mix_cache_hot_other": 208740,
  "mix_cache_hot_reject_exact": 23,
  "mix_cache_hot_reject_literal": 0,
  "mix_cache_hot_reject_ownslot": 0,
  "mix_cache_probe_calls": 42091,
  "mix_cache_read_exact": 13,
  "mix_cache_read_literal": 0,
  "mix_cache_read_ownslot": 0,
  "mix_cache_read_shared": 5,
  "mix_compact_numeric_calls": 0,
  "mix_hot_number": 42066,
  "mix_hot_other": 21440,
  "mix_hot_reference": 61887,
  "mix_number_format": 0,
  "mix_numeric_method_calls": 0,
  "mix_numeric_plain_calls": 0,
  "mix_numeric_plan_entries": 0,
  "mix_numeric_plan_nested": 0,
  "mix_object_alloc": 42726,
  "mix_object_clone": 188890,
  "mix_object_drop": 103160,
  "mix_slow_number": 6,
  "mix_slow_other": 41264,
  "mix_slow_reference": 9,
  "mix_string_length": 1,
  "mix_string_number": 0,
  "mix_string_other": 0,
  "mix_string_wrapper_number": 0,
  "mix_string_wrapper_other": 0,
  "mix_typed_identity_compare": 0,
  "mix_typed_null_compare": 0,
  "mix_typed_null_truthy": 0,
  "mix_typed_read_number": 0,
  "mix_typed_read_other": 0,
  "mix_typed_read_reference": 0,
  "mix_typed_ref_moves": 0,
  "mix_typed_ref_truthy": 0,
  "mix_typed_write_number": 0,
  "mix_typed_write_other": 0,
  "mix_wide_argument_numbers": 82528,
  "mix_wide_argument_references": 0,
  "mix_wide_binary_identity_compare": 0,
  "mix_wide_binary_null_compare": 0,
  "mix_wide_call_registers": 701488,
  "mix_wide_calls": 82528,
  "mix_wide_clear_live": 399280,
  "mix_wide_clear_ref": 82528,
  "mix_wide_clear_slots": 1051800,
  "mix_wide_exit_probes": 0,
  "mix_wide_exits": 0,
  "mix_wide_fallback_calls": 0,
  "mix_wide_identity_compare": 0,
  "mix_wide_max_depth": 8,
  "mix_wide_method_calls": 41264,
  "mix_wide_null_compare": 42072,
  "mix_wide_null_local": 0,
  "mix_wide_null_temp": 42072,
  "mix_wide_null_truthy": 0,
  "mix_wide_numeric_calls": 0,
  "mix_wide_other_compare": 42072,
  "mix_wide_plain_calls": 41264,
  "mix_wide_read_function": 41264,
  "mix_wide_read_hot": 125393,
  "mix_wide_read_null": 21440,
  "mix_wide_read_number": 42072,
  "mix_wide_read_other": 0,
  "mix_wide_read_reference": 61896,
  "mix_wide_read_slow": 41279,
  "mix_wide_read_string": 0,
  "mix_wide_ref_moves": 41264,
  "mix_wide_ref_truthy": 0,
  "mix_wide_returns": 126216,
  "mix_wide_rmw": 0,
  "mix_wide_rmw_reread": 0,
  "mix_wide_root_entries": 1616,
  "mix_wide_scalar_add": 20632,
  "mix_wide_scalar_mul": 41264,
  "mix_wide_scalar_other": 82528,
  "mix_wide_take_ref_copy": 0,
  "mix_wide_take_ref_move": 0,
  "mix_wide_write_number": 42072,
  "mix_wide_write_other": 42880,
  "mix_wide_write_reference": 41264,
  "named_property_reads": 1626,
  "named_property_writes": 2,
  "native_calls": 5,
  "nested_vm_constructions": 1,
  "ordinary_call_attempts": 126216,
  "same_vm_frame_entries": 0
 },
 "cdjs": {
  "authoritative_assign_local_hits": 51021,
  "authoritative_load_local_hits": 329539,
  "authoritative_store_local_hits": 210863,
  "call_env_marker_scrubs": 0,
  "closed_form_leaf_evaluations": 0,
  "compact_caller_env_calls": 1,
  "compact_direct_calls": 2162393,
  "compact_function_ops": 48853125,
  "compact_standalone_activations": 112918,
  "computed_property_reads": 100040,
  "computed_property_writes": 0,
  "declined_loop_plan_edges": 50026,
  "direct_leaf_frames": 112917,
  "dispatched_assign_local_ops": 51021,
  "dispatched_branch_return_ops": 152528,
  "dispatched_call_construct_ops": 150274,
  "dispatched_computed_property_ops": 50020,
  "dispatched_general_ops": 73568,
  "dispatched_global_binding_ops": 18023,
  "dispatched_load_const_ops": 195023,
  "dispatched_load_local_ops": 474967,
  "dispatched_local_binding_ops": 808915,
  "dispatched_named_property_ops": 248276,
  "dispatched_numeric_ops": 185671,
  "dispatched_stack_ops": 305688,
  "dispatched_store_local_ops": 282927,
  "executed_ops": 2187986,
  "generic_call_frames": 18038,
  "loop_backedges": 50159,
  "loop_plan_entries": 133,
  "mix_array_alloc": 10273,
  "mix_cache_hot_other": 9482524,
  "mix_cache_hot_reject_exact": 1026871,
  "mix_cache_hot_reject_literal": 368095,
  "mix_cache_hot_reject_ownslot": 56063,
  "mix_cache_probe_calls": 1846658,
  "mix_cache_read_exact": 1154031,
  "mix_cache_read_literal": 303368,
  "mix_cache_read_ownslot": 28032,
  "mix_cache_read_shared": 150,
  "mix_compact_numeric_calls": 0,
  "mix_hot_number": 3046497,
  "mix_hot_other": 2416458,
  "mix_hot_reference": 3167369,
  "mix_number_format": 1,
  "mix_numeric_method_calls": 19,
  "mix_numeric_plain_calls": 584709,
  "mix_numeric_plan_entries": 584728,
  "mix_numeric_plan_nested": 0,
  "mix_object_alloc": 379686,
  "mix_object_clone": 11544360,
  "mix_object_drop": 7083095,
  "mix_slow_number": 496348,
  "mix_slow_other": 1134168,
  "mix_slow_reference": 86442,
  "mix_string_length": 1,
  "mix_string_number": 0,
  "mix_string_other": 0,
  "mix_string_wrapper_number": 0,
  "mix_string_wrapper_other": 0,
  "mix_typed_identity_compare": 126,
  "mix_typed_null_compare": 0,
  "mix_typed_null_truthy": 64,
  "mix_typed_read_number": 17,
  "mix_typed_read_other": 67,
  "mix_typed_read_reference": 321,
  "mix_typed_ref_moves": 180,
  "mix_typed_ref_truthy": 189,
  "mix_typed_write_number": 0,
  "mix_typed_write_other": 0,
  "mix_wide_argument_numbers": 4026,
  "mix_wide_argument_references": 2361869,
  "mix_wide_binary_identity_compare": 115974,
  "mix_wide_binary_null_compare": 0,
  "mix_wide_call_registers": 17834676,
  "mix_wide_calls": 1908325,
  "mix_wide_clear_live": 7786591,
  "mix_wide_clear_ref": 3430120,
  "mix_wide_clear_slots": 21341377,
  "mix_wide_exit_probes": 586428,
  "mix_wide_exits": 0,
  "mix_wide_fallback_calls": 656451,
  "mix_wide_identity_compare": 87231,
  "mix_wide_max_depth": 8,
  "mix_wide_method_calls": 1320501,
  "mix_wide_null_compare": 19074,
  "mix_wide_null_local": 9538,
  "mix_wide_null_temp": 9536,
  "mix_wide_null_truthy": 136231,
  "mix_wide_numeric_calls": 584728,
  "mix_wide_other_compare": 1498918,
  "mix_wide_plain_calls": 587824,
  "mix_wide_read_function": 1976971,
  "mix_wide_read_hot": 8630324,
  "mix_wide_read_null": 151037,
  "mix_wide_read_number": 3542845,
  "mix_wide_read_other": 112176,
  "mix_wide_read_reference": 3253811,
  "mix_wide_read_slow": 1716958,
  "mix_wide_read_string": 1310442,
  "mix_wide_ref_moves": 3971635,
  "mix_wide_ref_truthy": 989896,
  "mix_wide_returns": 2275309,
  "mix_wide_rmw": 1,
  "mix_wide_rmw_reread": 0,
  "mix_wide_root_entries": 112916,
  "mix_wide_scalar_add": 526530,
  "mix_wide_scalar_mul": 138434,
  "mix_wide_scalar_other": 2258228,
  "mix_wide_take_ref_copy": 115974,
  "mix_wide_take_ref_move": 115974,
  "mix_wide_write_number": 472469,
  "mix_wide_write_other": 448223,
  "mix_wide_write_reference": 359339,
  "named_property_reads": 496456,
  "named_property_writes": 96,
  "native_calls": 650927,
  "nested_vm_constructions": 18046,
  "ordinary_call_attempts": 2293349,
  "same_vm_frame_entries": 0
 },
 "hash-map": {
  "authoritative_assign_local_hits": 21,
  "authoritative_load_local_hits": 20,
  "authoritative_store_local_hits": 19,
  "call_env_marker_scrubs": 0,
  "closed_form_leaf_evaluations": 180000,
  "compact_caller_env_calls": 1,
  "compact_direct_calls": 2160029,
  "compact_function_ops": 42300893,
  "compact_standalone_activations": 16,
  "computed_property_reads": 0,
  "computed_property_writes": 0,
  "declined_loop_plan_edges": 8,
  "direct_leaf_frames": 15,
  "dispatched_assign_local_ops": 21,
  "dispatched_branch_return_ops": 25,
  "dispatched_call_construct_ops": 16,
  "dispatched_computed_property_ops": 0,
  "dispatched_general_ops": 90,
  "dispatched_global_binding_ops": 3,
  "dispatched_load_const_ops": 58,
  "dispatched_load_local_ops": 38,
  "dispatched_local_binding_ops": 93,
  "dispatched_named_property_ops": 25,
  "dispatched_numeric_ops": 39,
  "dispatched_stack_ops": 60,
  "dispatched_store_local_ops": 34,
  "executed_ops": 409,
  "generic_call_frames": 1,
  "loop_backedges": 39,
  "loop_plan_entries": 31,
  "mix_array_alloc": 20,
  "mix_cache_hot_other": 3960126,
  "mix_cache_hot_reject_exact": 180080,
  "mix_cache_hot_reject_literal": 0,
  "mix_cache_hot_reject_ownslot": 7020006,
  "mix_cache_probe_calls": 3780119,
  "mix_cache_read_exact": 1800017,
  "mix_cache_read_literal": 0,
  "mix_cache_read_ownslot": 3510032,
  "mix_cache_read_shared": 180001,
  "mix_compact_numeric_calls": 0,
  "mix_hot_number": 1349991,
  "mix_hot_other": 2249991,
  "mix_hot_reference": 0,
  "mix_number_format": 0,
  "mix_numeric_method_calls": 0,
  "mix_numeric_plain_calls": 990018,
  "mix_numeric_plan_entries": 990018,
  "mix_numeric_plan_nested": 0,
  "mix_object_alloc": 90698,
  "mix_object_clone": 7798677,
  "mix_object_drop": 5310133,
  "mix_slow_number": 1800067,
  "mix_slow_other": 270039,
  "mix_slow_reference": 2430043,
  "mix_string_length": 1,
  "mix_string_number": 0,
  "mix_string_other": 0,
  "mix_string_wrapper_number": 0,
  "mix_string_wrapper_other": 0,
  "mix_typed_identity_compare": 0,
  "mix_typed_null_compare": 0,
  "mix_typed_null_truthy": 131071,
  "mix_typed_read_number": 393691,
  "mix_typed_read_other": 98329,
  "mix_typed_read_reference": 475370,
  "mix_typed_ref_moves": 41071,
  "mix_typed_ref_truthy": 98318,
  "mix_typed_write_number": 139389,
  "mix_typed_write_other": 0,
  "mix_wide_argument_numbers": 2969989,
  "mix_wide_argument_references": 0,
  "mix_wide_binary_identity_compare": 0,
  "mix_wide_binary_null_compare": 0,
  "mix_wide_call_registers": 18810411,
  "mix_wide_calls": 2070026,
  "mix_wide_clear_live": 7560235,
  "mix_wide_clear_ref": 1080070,
  "mix_wide_clear_slots": 19620559,
  "mix_wide_exit_probes": 900022,
  "mix_wide_exits": 0,
  "mix_wide_fallback_calls": 1,
  "mix_wide_identity_compare": 0,
  "mix_wide_max_depth": 3,
  "mix_wide_method_calls": 2070026,
  "mix_wide_null_compare": 19,
  "mix_wide_null_local": 19,
  "mix_wide_null_temp": 0,
  "mix_wide_null_truthy": 360001,
  "mix_wide_numeric_calls": 990018,
  "mix_wide_other_compare": 900004,
  "mix_wide_plain_calls": 0,
  "mix_wide_read_function": 2070027,
  "mix_wide_read_hot": 3599982,
  "mix_wide_read_null": 90003,
  "mix_wide_read_number": 3150058,
  "mix_wide_read_other": 360000,
  "mix_wide_read_reference": 2430043,
  "mix_wide_read_slow": 4500149,
  "mix_wide_read_string": 0,
  "mix_wide_ref_moves": 2970041,
  "mix_wide_ref_truthy": 900001,
  "mix_wide_returns": 2160043,
  "mix_wide_rmw": 270001,
  "mix_wide_rmw_reread": 0,
  "mix_wide_root_entries": 14,
  "mix_wide_scalar_add": 629990,
  "mix_wide_scalar_mul": 22,
  "mix_wide_scalar_other": 1530039,
  "mix_wide_take_ref_copy": 0,
  "mix_wide_take_ref_move": 0,
  "mix_wide_write_number": 540025,
  "mix_wide_write_other": 450004,
  "mix_wide_write_reference": 90020,
  "named_property_reads": 24,
  "named_property_writes": 26,
  "native_calls": 5,
  "nested_vm_constructions": 9,
  "ordinary_call_attempts": 2340047,
  "same_vm_frame_entries": 0
 },
 "string-validate-input": {
  "authoritative_assign_local_hits": 0,
  "authoritative_load_local_hits": 1,
  "authoritative_store_local_hits": 7,
  "call_env_marker_scrubs": 0,
  "closed_form_leaf_evaluations": 0,
  "compact_caller_env_calls": 0,
  "compact_direct_calls": 20000,
  "compact_function_ops": 1388035,
  "compact_standalone_activations": 1,
  "computed_property_reads": 0,
  "computed_property_writes": 0,
  "declined_loop_plan_edges": 4,
  "direct_leaf_frames": 1,
  "dispatched_assign_local_ops": 0,
  "dispatched_branch_return_ops": 5,
  "dispatched_call_construct_ops": 5,
  "dispatched_computed_property_ops": 0,
  "dispatched_general_ops": 15,
  "dispatched_global_binding_ops": 4,
  "dispatched_load_const_ops": 77,
  "dispatched_load_local_ops": 4,
  "dispatched_local_binding_ops": 11,
  "dispatched_named_property_ops": 0,
  "dispatched_numeric_ops": 1,
  "dispatched_stack_ops": 19,
  "dispatched_store_local_ops": 7,
  "executed_ops": 137,
  "generic_call_frames": 0,
  "loop_backedges": 4004,
  "loop_plan_entries": 4000,
  "mix_array_alloc": 5,
  "mix_cache_hot_other": 8008,
  "mix_cache_hot_reject_exact": 95996,
  "mix_cache_hot_reject_literal": 0,
  "mix_cache_hot_reject_ownslot": 0,
  "mix_cache_probe_calls": 78003,
  "mix_cache_read_exact": 73996,
  "mix_cache_read_literal": 0,
  "mix_cache_read_ownslot": 0,
  "mix_cache_read_shared": 0,
  "mix_compact_numeric_calls": 0,
  "mix_hot_number": 0,
  "mix_hot_other": 0,
  "mix_hot_reference": 0,
  "mix_number_format": 16000,
  "mix_numeric_method_calls": 0,
  "mix_numeric_plain_calls": 0,
  "mix_numeric_plan_entries": 0,
  "mix_numeric_plan_nested": 0,
  "mix_object_alloc": 4657,
  "mix_object_clone": 166291,
  "mix_object_drop": 28000,
  "mix_slow_number": 30000,
  "mix_slow_other": 78003,
  "mix_slow_reference": 0,
  "mix_string_length": 30001,
  "mix_string_number": 0,
  "mix_string_other": 0,
  "mix_string_wrapper_number": 0,
  "mix_string_wrapper_other": 0,
  "mix_typed_identity_compare": 0,
  "mix_typed_null_compare": 0,
  "mix_typed_null_truthy": 0,
  "mix_typed_read_number": 0,
  "mix_typed_read_other": 47997,
  "mix_typed_read_reference": 0,
  "mix_typed_ref_moves": 31998,
  "mix_typed_ref_truthy": 0,
  "mix_typed_write_number": 0,
  "mix_typed_write_other": 0,
  "mix_wide_argument_numbers": 8000,
  "mix_wide_argument_references": 0,
  "mix_wide_binary_identity_compare": 0,
  "mix_wide_binary_null_compare": 0,
  "mix_wide_call_registers": 156000,
  "mix_wide_calls": 20000,
  "mix_wide_clear_live": 48013,
  "mix_wide_clear_ref": 1,
  "mix_wide_clear_slots": 156032,
  "mix_wide_exit_probes": 88004,
  "mix_wide_exits": 0,
  "mix_wide_fallback_calls": 78003,
  "mix_wide_identity_compare": 0,
  "mix_wide_max_depth": 1,
  "mix_wide_method_calls": 0,
  "mix_wide_null_compare": 0,
  "mix_wide_null_local": 0,
  "mix_wide_null_temp": 0,
  "mix_wide_null_truthy": 0,
  "mix_wide_numeric_calls": 0,
  "mix_wide_other_compare": 64003,
  "mix_wide_plain_calls": 20000,
  "mix_wide_read_function": 78003,
  "mix_wide_read_hot": 0,
  "mix_wide_read_null": 0,
  "mix_wide_read_number": 30000,
  "mix_wide_read_other": 0,
  "mix_wide_read_reference": 0,
  "mix_wide_read_slow": 108003,
  "mix_wide_read_string": 0,
  "mix_wide_ref_moves": 4000,
  "mix_wide_ref_truthy": 0,
  "mix_wide_returns": 20001,
  "mix_wide_rmw": 0,
  "mix_wide_rmw_reread": 0,
  "mix_wide_root_entries": 1,
  "mix_wide_scalar_add": 0,
  "mix_wide_scalar_mul": 24001,
  "mix_wide_scalar_other": 10000,
  "mix_wide_take_ref_copy": 0,
  "mix_wide_take_ref_move": 0,
  "mix_wide_write_number": 0,
  "mix_wide_write_other": 0,
  "mix_wide_write_reference": 0,
  "named_property_reads": 0,
  "named_property_writes": 0,
  "native_calls": 4,
  "nested_vm_constructions": 5,
  "ordinary_call_attempts": 20001,
  "same_vm_frame_entries": 0
 }
}
```

Commands (all counting and timing under the shared lock):
```sh
lockf /tmp/qjs-lead/host-timing.lock cargo build --release -p qjs-cli --features perf-counters --target-dir target/mixed-call-followup/build
lockf /tmp/qjs-lead/host-timing.lock env PYTHONPATH=. python3 target/mixed-call-followup/collect-corpus.py
lockf /tmp/qjs-lead/host-timing.lock env PYTHONPATH=. python3 target/mixed-call-followup/atomic/measure.py measure
lockf /tmp/qjs-lead/host-timing.lock env PYTHONPATH=. python3 target/mixed-call-followup/extra/measure.py measure
lockf /tmp/qjs-lead/host-timing.lock env PYTHONPATH=. python3 target/mixed-call-followup/plain/measure.py measure
PYTHONPATH=. python3 target/mixed-call-followup/price.py
```
- `corpus-counts.json` SHA-256 `4c63fdc88d668e859007b38df0a5bcadd68f66e36f60e6110d4d1a2f78e35d01` (local artifact, not retained).
- `pricing.json` SHA-256 `dfd95c78165a50a3d2a0058173c4177b8fec1f17c062d81e04e9715ed2f5a124` (local artifact, not retained).
- `atomic/raw.json` SHA-256 `ef8cf2b49c54810e8f82d9a7020b0de450766d0d93757d22471e0a6637655b68` (local artifact, not retained).
- `extra/raw.json` SHA-256 `37e934c2a90d36447d5bf94e9aa1a32aa0369fa9e8964e575735ed8b4de2d8ab` (local artifact, not retained).
- `plain/raw.json` SHA-256 `69316ba716fc8c1bfe72f920924ee3d855569d07bc1087675d4bfb62a8ff4acc` (local artifact, not retained).
- Counter binary SHA-256 `63e05fafe4d62979dc36a6b7af4dde48b8ae9a70b2fea79667757fa2d3c363b7`; probe patch is retained at the end of this follow-up.

## Lead follow-up 2: instruction-level attribution (written before ABI replay)

Attribution combines executed counters, matched nonempty instruction deltas,
ARM64 disassembly of the exact e54e33eb standard binary, and CPU Counters /
Time Profiler stacks of that binary. Instrumented counters are used for event
counts, never timing. Sample fractions below are **timer weights**, not
instruction fractions; we do not convert them to instructions or distribute
an unexplained residual by them. Instruments 27 guided mode exports 1 ms
weights. Full-option `FIXED_INSTRUCTIONS`/sampleByTime=false still yielded
1 ms timer weights; no exclusive per-function instruction measurement resulted.

Recording-option probes failed with exact output:
`Failed to load recording options ...: The data couldn’t be read because it is missing.`
and manual mode: `Failed to load recording options ...: The data couldn’t be read because it isn’t in the correct format.`
Native Instruments control was unavailable (`cgWindowNotFound`); no permission
or security setting was changed. A time-limited launched target is SIGKILLed
by xctrace; its exit status is not a JS correctness failure. Corpus stdout and
micro assertions were checked separately. Complete recordings, not failed
option probes, supply the following samples.

### Which steps the joint prices actually contain

| Chunk | Measured instruction excess R-minus-NG | Function/steps and measured mechanism | Classification / removable amount established now |
| --- | ---: | --- | --- |
| H: next/null group | 492.795 | An independent next load/transfer is +201.438; independent strict null branch +156.988; remaining +134.369 is joint dispatch/liveness/operand-flow cost, not an exclusive helper charge. Original H has one field-ref result/clone/drop per step, not hundreds of refcount operations. | Cache/guard and branch helpers are leaf work; whole remaining 134 is **not** proven removable. Neither ABI nor public Value rewrite is required by the measured H mechanism. |
| T: child-choice group | 718.132 | Two ref field results, one null compare, one reference Move, three ObjectRef clones and three compact overwrite drops per step. Attributing 2*next + null leaves +158.268, but numeric hot/cold and liveness differ: this is a residual, not a copy price. | Dead-copy elimination within a region is a leaf; a live borrowed handle across reentry needs ABI roots/lifetimes; reducing obligatory tagged-handle dispatch needs representation work. No blanket 718-instruction removal justified. |
| W: store + reread | 174.810 | A full read/add/store/reload step is +258.749, versus +203.853 without reread: the extra reread is +54.896. The subtraction against read/add contributes the rest. No per-step ObjectRef clone increase in W. Real corpus has **zero** matched RMW+reread packets; raw stores are not W events. | Eliding a redundant reread is a leaf with an effect/liveness proof; no target reach shown for this exact packet. Do not build it for the five cases. |
| Fixed wide method envelope | 938.615 | Matched ordinary call +419.618; method lookup/receiver premium +518.997. One caller scalar accumulation contributes +61.817, leaving an activation/arg/return/seed envelope +357.801. The envelope is not all frame allocation. | Lookup/duplicate guards and within-region transport are leaves; borrowing caller roots across a call is ABI work; necessary owned-handle copy/drop remains without a representation/ownership change. Exact removable fraction of the 358 is unmeasured. |

These identities use the retained primitive prices, not sampler percentages.
The original “~500-700 extra instructions are reference transport” was too
coarse: property resolution, comparison dispatch and source-flow differences
are part of those prices. Numeric instructions do not rank a representation
migration first; measured scalar cycles already beat NG.

### Specific fat paths supported by source and disassembly

1. **Missing hot cache cases.** `NamedPropertyCache::probe_hot` accepts
   SharedSlot and guarded prototype entries, but rejects Exact, OwnSlot and
   LiteralShape. `get_prop_named` repeats the exotic guards/hot probe before
   calling the four-entry `probe`. Exact counting of successful OwnSlot reads
   gives hash 3,510,015, CDjs 28,022, raytrace 78,441; successful LiteralShape
   gives CDjs 229,039. Tree/validate have zero of those hit families. This is
   an operation/layout general leaf, unlike the rejected proven-receiver
   prototype shortcut: preserve all identity, epoch, descriptor and exotic
   guards; just deliver an already valid own-cache answer without the cold trip.
2. **Branchless reference cross-product.** For Object-vs-Null, e54e33eb
   `fast_strict_eq` executes 46 ARM64 instructions: 8 entry/type-path,
   36 at 0x10007c354..0x10007c3e0 comparing/selecting all six heap families,
   2 return. `WideActivation::compare` adds 39 on that successful strict path,
   excluding the helper: a 160-byte frame, four saved register pairs,
   operator routing and Result writeout. Null needs none of the six pointer
   comparisons. A same-discriminant guard / common ref-null answer is a leaf,
   with Number/NaN, symbols, Proxy, HTMLDDA and coercive equality negative tests;
   it must retain generic fallbacks. A target of 20-36 fewer instructions per
   mismatched reference comparison is a testable proposal, **not a measured
   implementation saving**. This mechanism can be killed on an isolated
   same-variant/mismatched-variant micro before broadening dispatch arms.
3. **Current frame is already small.** `size_of::<WideFrame>()` in the
   diagnostic build is 64 bytes. The current wide route moves that descriptor
   into a pooled vector; it does not construct/move the T021 704-byte
   FrameState six times/call. Any ABI claim must price bank/continuation and
   owner restoration costs, not recount the reverted implementation's memmoves.
   `inline_entry` memo-hit disassembly is a small 352-byte function (88 static
   instructions total, including cold branches); facts/CallEnv comparisons
   are already cached. Timer-flat entry samples <1.1% do not support charging
   the whole envelope to frame entry. Most entry/return transport is in `run`.
4. **Native-backedge probes remain.** Hash executes 900,022 wide Exit probes,
   raytrace 74,807, CDjs 586,428, validate 88,004, tree zero. A probe is not a
   VM handover: `exit_to_interpreter` can immediately Continue at a known
   native backedge. Rebuilding a driver view / encoded call action here is a
   distinct leaf candidate; preserve accelerator/deopt/hand-back semantics.
   This is not a frame-stack migration. Probe instruction price remains
   unmeasured, so it is ranked as a cheap pricing experiment, not booked ROI.

Cold `get_prop_named`: 416-byte stack frame, six saved register pairs plus
FP/LR and six argument register moves; common return restores six pairs,
stack and ret. `NamedPropertyCache::probe`: 992 static ARM64 instructions
across all four entries and cold/error paths. Those are **static code sizes**,
not executed per-read counts; do not multiply 992 by a probe frequency.

The lead supplied historical object-reference Value clone/drop costs of
5.5/3.6 ns and the failed inlining experiments. Those are not remeasured here.
Inlining, hand-written Value cloning and packed scalar tags are not proposed.
Removing a proven dead copy is different from inlining an obligatory copy.

### Timer samples identify functions, not instruction chunks

| Micro | Samples | Wide run | Compare + strict-eq | Value clone | Value drop | Property/cache/slots/memcmp | Inline entry | Exit helper |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| null | 4080 | 52.97% | 29.88% | 1.96% | 2.82% | 11.08% | 0.66% | 0.25% |
| tree | 4131 | 39.89% | 7.84% | 8.67% | 5.16% | 37.57% | 0.24% | 0.17% |
| write | 4147 | 71.67% | 0.00% | 9.57% | 3.83% | 13.67% | 0.87% | 0.07% |
| callplain | 4137 | 48.76% | 0.00% | 6.14% | 8.32% | 35.39% | 0.94% | 0.17% |
| reference | 6078 | 45.21% | 0.00% | 6.17% | 6.99% | 40.16% | 1.05% | 0.12% |

No per-function “extra 500 instructions” total was established from timer
samples. The remainder above is explicitly unassigned. Correctly measuring
exclusive costs needs instruction-event sampling or dynamic basic-block
counts, plus matched receiver/arity/cache regimes. This limitation prevents a
closed cycle payoff forecast and is retained, not filled with invented chunks.

Commands: `lockf /tmp/qjs-lead/host-timing.lock env PYTHONPATH=. python3
 target/mixed-call-followup/attribution/profile.py` records each 4 s case;
reference used a separate 6 s recording. `xcrun xctrace export <trace> --xpath
'/trace-toc/run[@number="1"]/data/table[@schema="time-profile"]'` exports stacks.
Disassembly: `llvm-nm -n -C <exact-base>` followed by
`llvm-objdump -d --no-show-raw-insn --demangle --start-address=<symbol>
--stop-address=<next-symbol> <exact-base>`; all source/symbols are e54e33eb.
Counter packet validation has 4,800,000 RMW and 4,800,000 reread matches on
original write-4-4, proving the zero corpus result is not a dead counting hook.
- `all-flat-samples.json` SHA-256 `a0e953b42133f43b6d9508f591d3f1128526ce1a3454d856c66f589e9491ff54` (local artifact, not retained).
- `fast_strict_eq.asm` SHA-256 `95384dde95c3f070905558e4f957b4f0f7773d75f01a9c2db88cd6747505d55d` (local artifact, not retained).
- `compare.asm` SHA-256 `b5af9cb074b3a661b684bf7cab24f72ad8e6cf530fbc012a7b10799efd4ffa27` (local artifact, not retained).
- `get_prop_named.asm` SHA-256 `ee92dd5a216a959261e127aaec84ff9a0d65854461095c42a9ef597e7f7b8cd0` (local artifact, not retained).

## Lead follow-up 3: conditional standalone stage-1 kill replay

Run only after follow-up 1 and 2 were written. Instruction-gap envelope pricing
meets the stated >=10% condition in hash/CDjs; cycle transfer remains unclosed.
This is a **standalone storage-cost replay**, no runtime integration or stage
advancement. It replays measured max nested-wide depths 3/8/5/8/1 and 16/32
register capacities around the observed mean call-bank widths, not every
callee-size distribution. Final model keeps both scalar and boxed elements
16 bytes; an initial two-variant niche-sized boxed model was discarded and
remeasured after adding the safe Number variant. No packed scalar or pointer
encoding. Header is 56 bytes, current production WideFrame 64 bytes.

One Enter/one scalar operation/one Leave per simulated call, with three
owned Rc handles in the reference case, zero-based per-activation slices and
ids across boundaries. All depth banks/descriptors are warmed once. Capacity
and bank-pointer invariants stay unchanged; result checksum and parent-return
state are asserted. This does not implement JS guards, cells, handlers,
reentry or exact exits and cannot substitute for their regressions.
N=10M and 2N, five counter pairs; per-call increments normalized by depth.

| Corpus depth / bank width | Scalar cycles / instructions | Reference cycles / instructions | Gate <=80 cycles |
| --- | ---: | ---: | --- |
| hash-map 3/16 | 32.03 / 187.68 | 38.87 / 219.68 | PASS |
| cdjs 8/16 | 31.27 / 185.40 | 38.22 / 217.42 | PASS |
| 3d-raytrace 5/32 | 31.63 / 186.22 | 38.42 / 218.23 | PASS |
| access-binary-trees 8/16 | 31.36 / 185.40 | 38.15 / 217.40 | PASS |
| string-validate-input 1/16 | 31.92 / 195.01 | 39.53 / 227.01 | PASS |

**The storage shape is not falsified; the migration remains unapproved.**
This proves a warmed split bank need not cost 80 cycles. It does not prove it
beats the already pooled 64-byte production route, or removes the call's
method/property guards. No candidate/base speedup or whole-case ROI follows.
In-place warm bank construction did not move a FrameState or allocate a bank
per call. Reentry/capture support would add work absent from this bound.

Commands:
```sh
lockf /tmp/qjs-lead/host-timing.lock rustc --edition 2024 -O target/mixed-call-followup/replay/replay.rs -o target/mixed-call-followup/replay/replay
lockf /tmp/qjs-lead/host-timing.lock env PYTHONPATH=. python3 target/mixed-call-followup/replay/measure.py
```

## Re-ranked units and what would reach parity

Numbers below are **pre-build experiment budgets**, not measured removals.
The static avoidable steps are established; compiler/code-layout net savings
are not. Every implementation still needs a newly frozen plan, negative
semantics, focused tests, pinned layout, the six sentinels and full gates.
No instruction share may be read as a guaranteed cycle share.

| Rank / proposed unit | Gap share/reach from phase 1 | Instructions / cycles to test removing | Kind and likely files |
| --- | --- | --- | --- |
| 1. Basic own-cache answers without cold round trip | Hash ref+numeric fields price 35.1% instruction gap; actual OwnSlot successes 3.510M. CDjs 257,061 OwnSlot/Literal hits; raytrace 78,441; tree/validate zero. Reach is those **hits**, not all misses. | Cold frame save/restore and argument moves are 24 static instructions, plus duplicated ordinary guard/hot probe/scan. Test net 20-60 instructions, 3-10 cycles per successful unsupported-hot hit; new helper/proof costs can erase this. | Leaf: `bytecode/named_property_cache.rs`, its tests, `compact_fn/property.rs`, targeted runtime tests. Do not pre-prove prototype misses for unrelated receivers. |
| 2. Method resolution / driver-segment pricing and bounded cleanup | Method premium 42.1/36.4/11.2/33.3/0% of instruction gaps (hash/CDjs/ray/tree/validate). Activation envelope 29.0/36.2/11.3/45.9/9.9%; native exit probes 900022/586428/74807/0/88004. | Price cold-method duplicate guards and native-backedge action/view rebuild separately first. Test 20-80 instructions / 3-15 cycles per matched method resolution; 20-80 instructions / 3-12 cycles per confirmed native probe. Do not book the whole 519 method premium or 358 activation envelope. | Leaf experiments first: `compact_fn/property.rs`, `wide/activation.rs`, `wide/activation/exits.rs`, `wide/mod.rs`, `vm/wide_resume.rs`, focused tests. Shares overlap rank 1; serialize. |
| 3. Same-kind reference equality / ref-null answer | Tree null/identity group 10.3% instruction gap; CDjs 4.4%, hash 2.9% is mostly truthiness and **not** this strict-eq helper; ray's binary-valued null tests unpriced separately. | Static Object/Null helper contains 36 cross-product instructions; propose 20-36 fewer instructions, 5-15 cycles per matched comparison, plus a cheap guard. Do not claim all 157 null or 718 child-choice excess. | Leaf: `bytecode/vm_ops.rs`, `wide/activation.rs` only if needed, `typed_loop/execute.rs` callers/tests. Protect NaN, symbols, Proxy, HTMLDDA and coercion fallbacks. |
| 4. Activation root/continuation ABI for live cross-call borrows | Same activation envelope as rank 2; not a second independent share. Within-region dead-copy elimination remains a leaf, obligatory live roots do not. | Replay 31-40 cycles / 185-227 instructions, <=80 gate. Production removals **unmeasured**; cannot subtract this toy from the 358-instruction envelope. Need matched existing-driver replay before selecting integration. | Migration, not approved: `wide/activation.rs`, `compact_fn/activation.rs`, `typed_loop/frame.rs`, `vm/wide_resume.rs`, `frame_stack.rs`, `function/call.rs`, binding/capture regression tests. No name snapshot. |
| 5. Actual conversion/length and allocation pricing | Validate residual 83.4% instructions / 75.9% cycles; 16000 number formats, 30001 code-unit length requests, 4657 object allocations. Tree allocates 42726 objects; allocation and constructors unpriced in phase 1. | No actual-path removal established. String(number) micro is not concat(number); wrapper/regexp work is not all owned-reference transport. Price the real paths before a leaf/representation plan. | Pricing first: `conversion.rs`, `number/formatting.rs`, `string/prototype/sequence.rs` / relevant string dispatcher, `string.rs`, `value/object/ordered_data_builder.rs`, constructors/tests as supported by the next profile. |

A public Value / GC / handle-representation migration is **not selected**.
It would be needed only if obligatory ownership/tag/allocator work still
explains a large priced residual after the bounded leaves. The historical
clone/drop times and global clone counts are not a proof that all those copies
are redundant, and no inlining or NaN-payload retry is authorized.

| Target | Cycle gap to NG, millions | Current supported combination | <=1.0 judgment |
| --- | ---: | --- | --- |
| hash-map | 345.22 | OwnSlot leaf first; method/native-view pricing second; conditional ABI only after remaining envelope attribution. | None demonstrated. Even rank-1 optimistic 3.51M*10 =35.10M cycles is only 10.2% of the gap; the other removals are not measured. |
| cdjs | 464.26 | Method/copy/segment paths first; basic-cache leaf has only 0.257M matched hits. Allocation/geometry/call residual needs pricing. | None demonstrated; inherited method reach is large but 519 instructions is a premium, not a removable chunk. |
| 3d-raytrace | 25.86 | Numeric/reference field paths plus method/segment cleanup; preserve existing numeric tiers. | None demonstrated; 44.3% instruction residual remains and the positive numeric cycle price is codegen-sensitive. |
| access-binary-trees | 18.76 | Reference equality leaf, call/segment attribution and constructor/allocation pricing. | None demonstrated; own-cache leaf has zero target hits. Its close instruction accounting does not prove equivalent cycle removals. |
| string-validate-input | 16.37 | Mandatory control; real concat(number)/length/native/allocator pricing in a separate unit. | None demonstrated; current ABI alone cannot price/remove the 75.9% cycle residual. |

All five reaching <=1.0 requires measured **net** removals at least equal to
those cycle gaps with controls unchanged. Neither summing overlapping model
shares nor adding the replay to the leaves establishes that. Recommend rank 1
as the next narrow measured unit, with rank 2/3 cheap pricing in order; do not
freeze the four-stage program or claim a parity combination now.
- `replay/results.json` SHA-256 `13fbd9146b83f932fefcd19e989896e2a6b7dcabaca98e45148059d26092fff8` (local artifact; source retained below).
- `replay/replay.rs` SHA-256 `27f463e11610015e20485f88aae7b890e44f8f86cca89e1c82d506754ad94fda` (local artifact; source retained below).
- `replay/measure.py` SHA-256 `e22604edd67ca41b64f5fb13c4342595d4a150e513175de76abaae2912af274b` (local artifact; source retained below).

## Follow-up reproducibility and handoff checks

The temporary counter patch below is documentation only, not compiled or
committed runtime source. Apply in an isolated e54e33eb worktree, build only
with perf-counters, and restore before any candidate/timing build. Phase-1
counter hash identifies the then-current probe; later successful-hit and
64-byte-frame probes added fields without changing counted semantics.
The retained patch is the final probe version, not the literal earlier hash.
Timing always used the unchanged e54e33eb/NG executables. Replay is standalone.

13 runtime files were restored from saved originals, each checked equal to
`git show e54e33eb:<path>`. `git diff --exit-code e54e33eb -- crates` passes.
No runtime changes, probe feature, dependency, generated trace or build output
is in the commit. No plan or budget was advanced. Documentation checks:
check-context, staged check-touched, base check-touched, branch scope and
whitespace checks. No new full runtime/conformance/promotion claim.

A first diagnostic build failed with:
`error[E0509]: cannot move out of type ObjectRef, which implements the Drop trait`
at `value/object/teardown.rs:17` (`Rc::into_inner(self.0)`). The custom Drop
probe was removed; compact overwrite drops were counted instead. Later
counter builds passed. A generated atomic source initially had a Python
`SyntaxError: invalid syntax`; it was corrected before measurement. Concurrent
cargo/rustc/qjs warnings discard and repeat the affected complete form; no
polluted partial sample is used. The initial niche-sized boxed replay was
also discarded. None is a failed implementation screen or promotion attempt.

### Diagnostic counter patch

Replace each line exactly `<SPACE>` with one literal space before applying
the diff; this preserves blank context lines without Markdown trailing whitespace.

```diff
diff --git a/crates/qjs-runtime/src/bytecode/compact_fn/activation.rs b/crates/qjs-runtime/src/bytecode/compact_fn/activation.rs
index 636c3052..902134a7 100644
--- a/crates/qjs-runtime/src/bytecode/compact_fn/activation.rs
+++ b/crates/qjs-runtime/src/bytecode/compact_fn/activation.rs
@@ -649,6 +649,7 @@ fn run_frames(
                     // `direct_leaf_frames`: no frame is built.
                     crate::diagnostics::count!(ordinary_call_attempts);
                     crate::diagnostics::count!(compact_direct_calls);
+                    crate::diagnostics::count!(mix_compact_numeric_calls);
                     let callee_base = current_base + current_len;
                     let callee_len = inline.register_count;
                     reserve_registers(registers, callee_base + callee_len);
diff --git a/crates/qjs-runtime/src/bytecode/compact_fn/execute.rs b/crates/qjs-runtime/src/bytecode/compact_fn/execute.rs
index 5b1df25b..73e962ad 100644
--- a/crates/qjs-runtime/src/bytecode/compact_fn/execute.rs
+++ b/crates/qjs-runtime/src/bytecode/compact_fn/execute.rs
@@ -39,6 +39,7 @@ pub(super) enum Action {
 #[inline(always)]
 pub(super) fn store(slot: &mut Value, value: Value) {
     let previous = std::mem::replace(slot, value);
+    #[cfg(feature = "perf-counters")] if matches!(previous,Value::Object(_) | Value::Array(_)){crate::diagnostics::count!(mix_object_drop); }
     if matches!(
         previous,
         Value::Number(_) | Value::Boolean(_) | Value::Null | Value::Undefined
diff --git a/crates/qjs-runtime/src/bytecode/compact_fn/numeric_plan.rs b/crates/qjs-runtime/src/bytecode/compact_fn/numeric_plan.rs
index 96e784bf..8ea54630 100644
--- a/crates/qjs-runtime/src/bytecode/compact_fn/numeric_plan.rs
+++ b/crates/qjs-runtime/src/bytecode/compact_fn/numeric_plan.rs
@@ -251,6 +251,7 @@ pub(in crate::bytecode) fn run(
     cells: &[Upvalue],
     args: &[f64],
 ) -> Option<Value> {
+    crate::diagnostics::count!(mix_numeric_plan_entries);
     let result = if plan.leaf {
         run_leaf(plan, args)?
     } else {
@@ -513,6 +514,7 @@ fn execute(ops: &[NumOp], resume: &mut usize, window: &mut [f64]) -> Option<Exit
                 args,
                 argc,
             } => {
+                crate::diagnostics::count!(mix_numeric_plan_nested);
                 *resume = pc;
                 return Some(Exit::Call {
                     dst,
diff --git a/crates/qjs-runtime/src/bytecode/compact_fn/wide/activation.rs b/crates/qjs-runtime/src/bytecode/compact_fn/wide/activation.rs
index e20c1c34..fed1e1bd 100644
--- a/crates/qjs-runtime/src/bytecode/compact_fn/wide/activation.rs
+++ b/crates/qjs-runtime/src/bytecode/compact_fn/wide/activation.rs
@@ -425,6 +425,7 @@ fn callee_parameter_slots(callee: &Value) -> &[usize] {
 /// -- is copied, unless the operation writes its result back there.
 #[inline(always)]
 fn take_operand(window: &mut [Value], register: u16, dst: u16, locals: u16) -> Value {
+    #[cfg(feature = "perf-counters")] crate::diagnostics::update(|c| { if matches!(window[register as usize],Value::Object(_) | Value::Array(_)){if register == dst || register >= locals {c.mix_wide_take_ref_move+=1;}else{c.mix_wide_take_ref_copy+=1;}} });
     if register == dst || register >= locals {
         std::mem::replace(&mut window[register as usize], Value::Undefined)
     } else {
@@ -438,6 +439,8 @@ fn clear_window(window: &mut [Value]) {
     // consumed its operands moved them out -- and an empty register needs no
     // write at all.
     for slot in window {
+        crate::diagnostics::count!(mix_wide_clear_slots);
+        #[cfg(feature = "perf-counters")] crate::diagnostics::update(|c| {if !matches!(slot,Value::Undefined){c.mix_wide_clear_live+=1;}if matches!(slot,Value::Object(_) | Value::Array(_)){c.mix_wide_clear_ref+=1;}});
         if !matches!(slot, Value::Undefined) {
             execute::store(slot, Value::Undefined);
         }
@@ -712,6 +715,8 @@ fn run_frames(
     let root_owner = entry.upvalue_owner;
     let root_slots = entry.upvalue_slots;
     let mut current_this = root_this;
+    crate::diagnostics::count!(mix_wide_root_entries);
+    #[cfg(feature="perf-counters")] crate::diagnostics::update(|c|c.mix_wide_frame_bytes=std::mem::size_of::<WideFrame>() as u64);
     let mut current_base = 0_usize;
     let mut current_len = entry.program.register_count;
     let mut pc = 0_usize;
@@ -758,6 +763,21 @@ fn run_frames(
                 pc += 1;
                 #[cfg(feature = "perf-counters")]
                 crate::diagnostics::update(|c| c.compact_function_ops += 1);
+                #[cfg(feature = "perf-counters")]
+                crate::diagnostics::update(|c| match *op {
+                 WideOp::JumpIfFalsy {cond,..} | WideOp::JumpIfTruthy {cond,..} => match &window[cond as usize] {Value::Object(_) | Value::Array(_)=>c.mix_wide_ref_truthy+=1,Value::Null | Value::Undefined=>c.mix_wide_null_truthy+=1,_=>{}},
+                 WideOp::Move {src,..} | WideOp::Dup {src,..} if matches!(window[src as usize],Value::Object(_) | Value::Array(_)) => c.mix_wide_ref_moves+=1,
+                 WideOp::CompareJump {left,right,..} => {
+                  let (l,r)=(&window[left as usize],&window[right as usize]);
+                  if matches!(l,Value::Null | Value::Undefined) || matches!(r,Value::Null | Value::Undefined) {c.mix_wide_null_compare+=1;if matches!(l,Value::Object(_) | Value::Array(_) | Value::Null | Value::Undefined)&&matches!(r,Value::Object(_) | Value::Array(_) | Value::Null | Value::Undefined){if left < program.local_registers || right < program.local_registers {c.mix_wide_null_local+=1;}else{c.mix_wide_null_temp+=1;}}}
+                  else if matches!(l,Value::Object(_) | Value::Array(_)) || matches!(r,Value::Object(_) | Value::Array(_)) {c.mix_wide_identity_compare+=1;}
+                  else {c.mix_wide_other_compare+=1;}
+                 },
+                 WideOp::Binary {op,left,right,..} if matches!((&window[left as usize],&window[right as usize]),(Value::Number(_),Value::Number(_)))=>match op {BinaryOp::Add=>c.mix_wide_scalar_add+=1,BinaryOp::Mul=>c.mix_wide_scalar_mul+=1,_=>c.mix_wide_scalar_other+=1},
+                 WideOp::Exit {..}=>c.mix_wide_exit_probes+=1,
+                 _=>{},
+                });
+                #[cfg(feature="perf-counters")] if matches!(op,WideOp::Call {..}|WideOp::CallResolved {..}|WideOp::Return {..}|WideOp::Exit {..}|WideOp::New {..}) {crate::diagnostics::mix_forget_access();}
                 match *op {
                     WideOp::LoadConst { dst, index } => {
                         let Some(value) = activation.bytecode.constants.get(index as usize) else {
@@ -819,11 +839,17 @@ fn run_frames(
                             break Err(execute::uninitialized_local());
                         };
                         if let Some(value) = property::get_prop_named_hot(receiver, &site.cache) {
+                            #[cfg(feature = "perf-counters")] crate::diagnostics::mix_read(&value,false);
+                            #[cfg(feature="perf-counters")] crate::diagnostics::mix_access_read(&value,receiver,&site.key);
+                            crate::diagnostics::count!(mix_wide_read_hot);
+                            #[cfg(feature = "perf-counters")] crate::diagnostics::update(|c| match &value {Value::Number(_)=>c.mix_hot_number+=1,Value::Object(_) | Value::Array(_)=>c.mix_hot_reference+=1,_=>c.mix_hot_other+=1});
                             execute::store(&mut window[dst as usize], value);
                             continue;
                         }
                         match property::get_prop_named(receiver, &site.key, &site.cache, env) {
-                            Ok(value) => execute::store(&mut window[dst as usize], value),
+                            Ok(value) => { #[cfg(feature = "perf-counters")] crate::diagnostics::mix_read(&value,false);
+                            #[cfg(feature="perf-counters")] crate::diagnostics::mix_access_read(&value,receiver,&site.key); crate::diagnostics::count!(mix_wide_read_slow);
+                            #[cfg(feature = "perf-counters")] crate::diagnostics::update(|c| match &value {Value::Number(_)=>c.mix_slow_number+=1,Value::Object(_) | Value::Array(_)=>c.mix_slow_reference+=1,_=>c.mix_slow_other+=1}); execute::store(&mut window[dst as usize], value) },
                             Err(error) => break Err(error),
                         }
                     }
@@ -836,6 +862,10 @@ fn run_frames(
                         if let Some(value) =
                             property::get_prop_named_hot(&window[obj as usize], &site.cache)
                         {
+                            #[cfg(feature = "perf-counters")] crate::diagnostics::mix_read(&value,false);
+                            #[cfg(feature="perf-counters")] crate::diagnostics::mix_access_read(&value,&window[obj as usize],&site.key);
+                            crate::diagnostics::count!(mix_wide_read_hot);
+                            #[cfg(feature = "perf-counters")] crate::diagnostics::update(|c| match &value {Value::Number(_)=>c.mix_hot_number+=1,Value::Object(_) | Value::Array(_)=>c.mix_hot_reference+=1,_=>c.mix_hot_other+=1});
                             execute::store(&mut window[dst as usize], value);
                             continue;
                         }
@@ -845,7 +875,9 @@ fn run_frames(
                             &site.cache,
                             env,
                         ) {
-                            Ok(value) => execute::store(&mut window[dst as usize], value),
+                            Ok(value) => { #[cfg(feature = "perf-counters")] crate::diagnostics::mix_read(&value,false);
+                            #[cfg(feature="perf-counters")] crate::diagnostics::mix_access_read(&value,&window[obj as usize],&site.key); crate::diagnostics::count!(mix_wide_read_slow);
+                            #[cfg(feature = "perf-counters")] crate::diagnostics::update(|c| match &value {Value::Number(_)=>c.mix_slow_number+=1,Value::Object(_) | Value::Array(_)=>c.mix_slow_reference+=1,_=>c.mix_slow_other+=1}); execute::store(&mut window[dst as usize], value) },
                             Err(error) => break Err(error),
                         }
                     }
@@ -855,6 +887,8 @@ fn run_frames(
                         };
                         let assigned =
                             std::mem::replace(&mut window[value as usize], Value::Undefined);
+                        #[cfg(feature = "perf-counters")] crate::diagnostics::mix_write(&assigned,false);
+                        #[cfg(feature="perf-counters")] crate::diagnostics::mix_access_write(&assigned,&window[obj as usize],&site.key);
                         if property::set_prop_named_hot(
                             &window[obj as usize],
                             site.cache.as_ref(),
@@ -888,6 +922,8 @@ fn run_frames(
                         };
                         let assigned =
                             std::mem::replace(&mut window[value as usize], Value::Undefined);
+                        #[cfg(feature = "perf-counters")] crate::diagnostics::mix_write(&assigned,false);
+                        #[cfg(feature="perf-counters")] crate::diagnostics::mix_access_write(&assigned,receiver,&site.key);
                         if property::set_prop_named_hot(
                             receiver,
                             site.cache.as_ref(),
@@ -916,6 +952,7 @@ fn run_frames(
                         left,
                         right,
                     } => {
+                        #[cfg(feature = "perf-counters")] crate::diagnostics::update(|c| { if matches!(op,BinaryOp::Eq | BinaryOp::Ne | BinaryOp::StrictEq | BinaryOp::StrictNe){let(l,r)=(&window[left as usize],&window[right as usize]);if matches!(l,Value::Null | Value::Undefined)||matches!(r,Value::Null | Value::Undefined){c.mix_wide_binary_null_compare+=1;}else if matches!(l,Value::Object(_) | Value::Array(_))||matches!(r,Value::Object(_) | Value::Array(_)){c.mix_wide_binary_identity_compare+=1;} } });
                         if let (Value::Number(left), Value::Number(right)) =
                             (&window[left as usize], &window[right as usize])
                             && let Some(value) =
@@ -1246,6 +1283,7 @@ fn run_frames(
         }
         let (dst, base, argc, resume_pc, resolved) = match action {
             Action::Return(value) => {
+                crate::diagnostics::count!(mix_wide_returns);
                 clear_window(&mut registers[current_base..current_base + current_len]);
                 let Some(caller) = frames.pop() else {
                     return Ok(value);
@@ -1302,6 +1340,8 @@ fn run_frames(
                     &registers[argument_index..argument_index + argument_count],
                 )
             {
+                crate::diagnostics::count!(mix_wide_numeric_calls);
+                #[cfg(feature = "perf-counters")] crate::diagnostics::update(|c| {if resolved{c.mix_numeric_method_calls+=1;}else{c.mix_numeric_plain_calls+=1;}});
                 if let Some(receiver) = receiver {
                     release(receiver);
                 }
@@ -1315,6 +1355,8 @@ fn run_frames(
             }
             crate::diagnostics::count!(ordinary_call_attempts);
             crate::diagnostics::count!(compact_direct_calls);
+            crate::diagnostics::count!(mix_wide_calls);
+            #[cfg(feature = "perf-counters")] crate::diagnostics::update(|c| {if resolved{c.mix_wide_method_calls+=1;}else{c.mix_wide_plain_calls+=1;} c.mix_wide_call_registers+=inline.register_count as u64;c.mix_wide_max_depth=c.mix_wide_max_depth.max(frames.len() as u64+1);for v in &registers[argument_index..argument_index+argument_count]{match v{Value::Number(_)=>c.mix_wide_argument_numbers+=1,Value::Object(_) | Value::Array(_)=>c.mix_wide_argument_references+=1,_=>{}}}});
             let callee_base = current_base + current_len;
             let callee_len = inline.register_count;
             reserve_registers(registers, callee_base + callee_len);
@@ -1381,6 +1423,7 @@ fn run_frames(
             continue;
         }
<SPACE>
+        crate::diagnostics::count!(mix_wide_fallback_calls);
         let outcome = {
             let bytecode = running_bytecode(&current_callee, root_bytecode);
             let (upvalue_owner, upvalue_slots) = match &current_callee {
diff --git a/crates/qjs-runtime/src/bytecode/named_property_cache.rs b/crates/qjs-runtime/src/bytecode/named_property_cache.rs
index a33e609e..7c3fa4c6 100644
--- a/crates/qjs-runtime/src/bytecode/named_property_cache.rs
+++ b/crates/qjs-runtime/src/bytecode/named_property_cache.rs
@@ -168,6 +168,7 @@ impl NamedPropertyCache {
     #[inline(always)]
     pub(super) fn probe_hot(&self, object: &ObjectRef) -> Option<Value> {
         let state = self.0.borrow();
+        #[cfg(feature = "perf-counters")] crate::diagnostics::update(|c| match state.entries.get(usize::from(state.hot.get())) {Some(Some(NamedPropertyCacheEntry::Exact {..}))=>c.mix_cache_hot_reject_exact+=1,Some(Some(NamedPropertyCacheEntry::LiteralShape {..}))=>c.mix_cache_hot_reject_literal+=1,Some(Some(NamedPropertyCacheEntry::OwnSlot {..}))=>c.mix_cache_hot_reject_ownslot+=1,_=>c.mix_cache_hot_other+=1});
         match state.entries.get(usize::from(state.hot.get()))? {
             Some(NamedPropertyCacheEntry::SharedSlot { key, slot }) => {
                 object.shared_data_slot_value(key, *slot)
@@ -212,6 +213,7 @@ impl NamedPropertyCache {
     }
<SPACE>
     pub(super) fn probe(&self, object: &ObjectRef) -> CacheProbe {
+        crate::diagnostics::count!(mix_cache_probe_calls);
         let state = self.0.borrow();
         let mut candidate = None;
         // Unrolled by hand: whether LLVM unrolled the walk varied from build
@@ -298,6 +300,7 @@ impl NamedPropertyCache {
     /// on every property-heavy workload.
     #[inline(always)]
     fn read_entry(entry: &NamedPropertyCacheEntry, object: &ObjectRef) -> Option<Value> {
+        #[cfg(feature = "perf-counters")] crate::diagnostics::update(|c| match entry {NamedPropertyCacheEntry::Exact {..}=>c.mix_cache_read_exact+=1,NamedPropertyCacheEntry::LiteralShape {..}=>c.mix_cache_read_literal+=1,NamedPropertyCacheEntry::OwnSlot {..}=>c.mix_cache_read_ownslot+=1,NamedPropertyCacheEntry::SharedSlot {..}=>c.mix_cache_read_shared+=1,_=>{}});
         let value = match entry {
             NamedPropertyCacheEntry::Exact {
                 object: cached_object,
@@ -310,7 +313,7 @@ impl NamedPropertyCache {
                 value
             }
             NamedPropertyCacheEntry::LiteralShape { shape, slot } => {
-                return object.literal_data_slot_value(shape, *slot);
+                let value=object.literal_data_slot_value(shape, *slot); if value.is_some(){crate::diagnostics::count!(mix_cache_success_literal);} return value;
             }
             NamedPropertyCacheEntry::OwnSlot {
                 object: cached_object,
@@ -320,10 +323,10 @@ impl NamedPropertyCache {
                 if !cached_object.ptr_eq(object) || *layout_revision != object.layout_revision() {
                     return None;
                 }
-                return object.own_data_slot_value(*slot);
+                let value=object.own_data_slot_value(*slot); if value.is_some(){crate::diagnostics::count!(mix_cache_success_ownslot);} return value;
             }
             NamedPropertyCacheEntry::SharedSlot { key, slot } => {
-                return object.shared_data_slot_value(key, *slot);
+                let value=object.shared_data_slot_value(key, *slot); if value.is_some(){crate::diagnostics::count!(mix_cache_success_shared);} return value;
             }
             // Only valid once the receiver is known to have no own `key`, so
             // it is served by `get_from_prototype` rather than from here.
@@ -510,7 +513,8 @@ impl NamedPropertyCache {
             let Some(entry) = entry else {
                 continue;
             };
-            match entry {
+
+        match entry {
                 NamedPropertyCacheEntry::Exact {
                     object: cached,
                     revision,
diff --git a/crates/qjs-runtime/src/bytecode/typed_loop/execute.rs b/crates/qjs-runtime/src/bytecode/typed_loop/execute.rs
index 7a944908..d0e9ec1f 100644
--- a/crates/qjs-runtime/src/bytecode/typed_loop/execute.rs
+++ b/crates/qjs-runtime/src/bytecode/typed_loop/execute.rs
@@ -305,7 +305,7 @@ fn execute<F: LoopFrame>(
                     deopt_here!(op);
                 }
             }
-            TypedOp::MoveBoxed { dst, src } => boxed[dst as usize] = boxed[src as usize].clone(),
+            TypedOp::MoveBoxed { dst, src } => { #[cfg(feature = "perf-counters")] crate::diagnostics::update(|c| {if matches!(boxed[src as usize],Value::Object(_) | Value::Array(_)){c.mix_typed_ref_moves+=1;}}); boxed[dst as usize] = boxed[src as usize].clone(); },
             TypedOp::Unbox { dst, src } => {
                 let Some(value) = Typed::from_value(&boxed[src as usize]) else {
                     deopt_here!(op);
@@ -313,6 +313,7 @@ fn execute<F: LoopFrame>(
                 registers[reg(dst)] = value;
             }
             TypedOp::Truthy { dst, src } => {
+                #[cfg(feature = "perf-counters")] crate::diagnostics::update(|c| match &boxed[src as usize] {Value::Object(_) | Value::Array(_)=>c.mix_typed_ref_truthy+=1,Value::Null | Value::Undefined=>c.mix_typed_null_truthy+=1,_=>{}});
                 registers[reg(dst)] = boxed_truthiness(&boxed[src as usize]);
             }
             TypedOp::Box { dst, src } => {
@@ -333,6 +334,7 @@ fn execute<F: LoopFrame>(
                 ) else {
                     deopt_here!(op);
                 };
+                #[cfg(feature = "perf-counters")] crate::diagnostics::mix_read(&value,true);
                 boxed[dst as usize] = value;
             }
             TypedOp::SetNamed {
@@ -341,6 +343,7 @@ fn execute<F: LoopFrame>(
                 value,
                 cache,
             } => {
+                #[cfg(feature = "perf-counters")] crate::diagnostics::mix_write(&boxed[value as usize],true);
                 if !set_named(
                     &boxed[object as usize],
                     &program.names[name as usize],
@@ -365,6 +368,7 @@ fn execute<F: LoopFrame>(
                     if let Some((key, slot)) = shapes.slot.as_ref()
                         && let Some(number) = receiver.shared_data_slot_number(key, *slot)
                     {
+                        crate::diagnostics::count!(mix_typed_read_number);
                         registers[reg(dst)] = Typed::Number(number);
                         continue;
                     }
@@ -375,7 +379,8 @@ fn execute<F: LoopFrame>(
                     else {
                         deopt_here!(op);
                     };
-                    registers[reg(dst)] = value;
+                    crate::diagnostics::count!(mix_typed_read_number);
+                registers[reg(dst)] = value;
                     continue;
                 }
                 let Some(value) = get_named(
@@ -389,6 +394,7 @@ fn execute<F: LoopFrame>(
                 .and_then(Typed::from_value) else {
                     deopt_here!(op);
                 };
+                crate::diagnostics::count!(mix_typed_read_number);
                 registers[reg(dst)] = value;
             }
             TypedOp::SetNamedTyped {
@@ -397,6 +403,7 @@ fn execute<F: LoopFrame>(
                 value,
                 cache,
             } => {
+                crate::diagnostics::count!(mix_typed_write_number);
                 // A number into a field the site already shares, as the
                 // inline numeric read.
                 if let (Value::Object(receiver), Typed::Number(number)) =
@@ -602,6 +609,7 @@ fn execute<F: LoopFrame>(
                 left,
                 right,
             } => {
+                #[cfg(feature = "perf-counters")] crate::diagnostics::update(|c| { let(l,r)=(&boxed[left as usize],&boxed[right as usize]);if matches!(l,Value::Null | Value::Undefined)||matches!(r,Value::Null | Value::Undefined){c.mix_typed_null_compare+=1;}else if matches!(l,Value::Object(_) | Value::Array(_)) || matches!(r,Value::Object(_) | Value::Array(_)){c.mix_typed_identity_compare+=1;} });
                 let Some(value) = boxed_equality(op, &boxed[left as usize], &boxed[right as usize])
                 else {
                     deopt_here!(op);
diff --git a/crates/qjs-runtime/src/diagnostics.rs b/crates/qjs-runtime/src/diagnostics.rs
index 55795487..38c1abbd 100644
--- a/crates/qjs-runtime/src/diagnostics.rs
+++ b/crates/qjs-runtime/src/diagnostics.rs
@@ -49,6 +49,103 @@ macro_rules! declare_counters {
 }
<SPACE>
 declare_counters! {
+    mix_wide_frame_bytes,
+
+    mix_cache_success_literal,
+    mix_cache_success_ownslot,
+    mix_cache_success_shared,
+
+    mix_wide_plain_calls,
+    mix_wide_method_calls,
+    mix_numeric_plain_calls,
+    mix_numeric_method_calls,
+    mix_wide_argument_numbers,
+    mix_wide_argument_references,
+    mix_wide_call_registers,
+    mix_wide_max_depth,
+
+    mix_wide_rmw,
+    mix_wide_rmw_reread,
+
+    mix_number_format,
+
+    mix_cache_hot_reject_exact,
+    mix_cache_hot_reject_literal,
+    mix_cache_hot_reject_ownslot,
+    mix_cache_hot_other,
+    mix_cache_probe_calls,
+    mix_cache_read_exact,
+    mix_cache_read_literal,
+    mix_cache_read_ownslot,
+    mix_cache_read_shared,
+    mix_string_wrapper_number,
+    mix_string_wrapper_other,
+
+    mix_wide_null_local,
+    mix_wide_null_temp,
+    mix_wide_ref_truthy,
+    mix_wide_null_truthy,
+    mix_typed_ref_truthy,
+    mix_typed_null_truthy,
+
+    mix_numeric_plan_entries,
+    mix_numeric_plan_nested,
+    mix_compact_numeric_calls,
+    mix_typed_null_compare,
+    mix_typed_identity_compare,
+    mix_wide_binary_null_compare,
+    mix_wide_binary_identity_compare,
+    mix_hot_number,
+    mix_hot_reference,
+    mix_hot_other,
+    mix_slow_number,
+    mix_slow_reference,
+    mix_slow_other,
+
+    mix_wide_calls,
+    mix_wide_numeric_calls,
+    mix_wide_fallback_calls,
+    mix_wide_returns,
+    mix_wide_root_entries,
+    mix_wide_exit_probes,
+    mix_wide_exits,
+    mix_wide_read_number,
+    mix_wide_read_reference,
+    mix_wide_read_null,
+    mix_wide_read_function,
+    mix_wide_read_string,
+    mix_wide_read_other,
+    mix_wide_read_hot,
+    mix_wide_read_slow,
+    mix_wide_write_number,
+    mix_wide_write_reference,
+    mix_wide_write_other,
+    mix_wide_ref_moves,
+    mix_wide_take_ref_copy,
+    mix_wide_take_ref_move,
+    mix_wide_null_compare,
+    mix_wide_identity_compare,
+    mix_wide_other_compare,
+    mix_wide_scalar_add,
+    mix_wide_scalar_mul,
+    mix_wide_scalar_other,
+    mix_wide_clear_slots,
+    mix_wide_clear_live,
+    mix_wide_clear_ref,
+    mix_typed_read_number,
+    mix_typed_read_reference,
+    mix_typed_read_other,
+    mix_typed_write_number,
+    mix_typed_write_other,
+    mix_typed_ref_moves,
+    mix_object_alloc,
+    mix_array_alloc,
+    mix_object_clone,
+    mix_object_drop,
+    mix_string_number,
+    mix_string_other,
+    mix_string_length,
+
     /// Calls a VM call opcode dispatched to a user function with a bytecode
     /// body, before any tier decides how to run it. A workload that claims N
     /// calls must report about N here, or the workload is not measuring calls.
@@ -173,3 +270,45 @@ macro_rules! count {
 }
<SPACE>
 pub(crate) use count;
+
+#[cfg(feature = "perf-counters")]
+pub(crate) fn mix_read(value: &crate::Value, typed: bool) {
+ use crate::Value;
+ update(|c| match (typed, value) {
+  (false,Value::Number(_))=>c.mix_wide_read_number+=1,
+  (false,Value::Object(_) | Value::Array(_))=>c.mix_wide_read_reference+=1,
+  (false,Value::Null)=>c.mix_wide_read_null+=1,
+  (false,Value::Function(_))=>c.mix_wide_read_function+=1,
+  (false,Value::String(_))=>c.mix_wide_read_string+=1,
+  (false,_)=>c.mix_wide_read_other+=1,
+  (true,Value::Number(_))=>c.mix_typed_read_number+=1,
+  (true,Value::Object(_) | Value::Array(_))=>c.mix_typed_read_reference+=1,
+  (true,_)=>c.mix_typed_read_other+=1,
+ });
+}
+#[cfg(feature = "perf-counters")]
+pub(crate) fn mix_write(value: &crate::Value, typed: bool) {
+ use crate::Value;
+ update(|c| match (typed,value) {
+  (false,Value::Number(_))=>c.mix_wide_write_number+=1,
+  (false,Value::Object(_) | Value::Array(_))=>c.mix_wide_write_reference+=1,
+  (false,_)=>c.mix_wide_write_other+=1,
+  (true,Value::Number(_))=>c.mix_typed_write_number+=1,
+  (true,_)=>c.mix_typed_write_other+=1,
+ });
+}
+
+#[cfg(feature="perf-counters")]
+thread_local! { static MIX_ACCESS: std::cell::RefCell<(Option<(usize,String)>,Option<(usize,String,bool)>)> = std::cell::RefCell::new((None,None)); }
+#[cfg(feature="perf-counters")]
+pub(crate) fn mix_forget_access() { MIX_ACCESS.with(|s| *s.borrow_mut()=(None,None)); }
+#[cfg(feature="perf-counters")]
+pub(crate) fn mix_access_read(v:&crate::Value, receiver:&crate::Value, key:&str) {
+ let crate::Value::Object(o)=receiver else {return};
+ MIX_ACCESS.with(|state| {let mut state=state.borrow_mut();if matches!(v,crate::Value::Number(_)){if state.1.as_ref().is_some_and(|(a,k,rmw)|*a==o.address() && k==key && *rmw){count!(mix_wide_rmw_reread);}state.0=Some((o.address(),key.to_owned()));}else{state.0=None;}state.1=None;});
+}
+#[cfg(feature="perf-counters")]
+pub(crate) fn mix_access_write(v:&crate::Value, receiver:&crate::Value, key:&str) {
+ let crate::Value::Object(o)=receiver else {return};
+ MIX_ACCESS.with(|state| {let mut state=state.borrow_mut();let rmw=matches!(v,crate::Value::Number(_))&&state.0.as_ref().is_some_and(|(a,k)|*a==o.address()&&k==key);if rmw{count!(mix_wide_rmw);}state.0=None;state.1=Some((o.address(),key.to_owned(),rmw));});
+}
diff --git a/crates/qjs-runtime/src/number/formatting.rs b/crates/qjs-runtime/src/number/formatting.rs
index 3e12fad0..fa13d46e 100644
--- a/crates/qjs-runtime/src/number/formatting.rs
+++ b/crates/qjs-runtime/src/number/formatting.rs
@@ -1,4 +1,5 @@
 pub(crate) fn number_to_js_string(number: f64) -> String {
+    crate::diagnostics::count!(mix_number_format);
     if number.is_nan() {
         "NaN".to_owned()
     } else if number == f64::INFINITY {
diff --git a/crates/qjs-runtime/src/string.rs b/crates/qjs-runtime/src/string.rs
index 4ed0bc7a..d4317311 100644
--- a/crates/qjs-runtime/src/string.rs
+++ b/crates/qjs-runtime/src/string.rs
@@ -99,6 +99,7 @@ pub(crate) fn js_string_code_unit_at(value: &JsString, index: usize) -> Option<u
<SPACE>
 /// Returns the UTF-16 length of a shared string value, memoized on the value.
 pub(crate) fn js_string_code_unit_len(value: &JsString) -> usize {
+    crate::diagnostics::count!(mix_string_length);
     value.utf16_len_with(wide_string_code_unit_len)
 }
<SPACE>
diff --git a/crates/qjs-runtime/src/string/constructor.rs b/crates/qjs-runtime/src/string/constructor.rs
index 102fa188..75f49c67 100644
--- a/crates/qjs-runtime/src/string/constructor.rs
+++ b/crates/qjs-runtime/src/string/constructor.rs
@@ -15,6 +15,7 @@ pub(crate) fn native_string(
     is_construct: bool,
     env: &mut CallEnv,
 ) -> Result<Value, RuntimeError> {
+    #[cfg(feature = "perf-counters")] crate::diagnostics::update(|c| {if matches!(argument_values.first(),Some(Value::Number(_))){c.mix_string_number+=1;}else{c.mix_string_other+=1;}});
     let value = match argument_values.first().cloned() {
         // `String(symbol)` (non-construct) returns the descriptive string;
         // `new String(symbol)` falls through to ToString, which throws.
@@ -44,6 +45,7 @@ pub(crate) fn construct_string_wrapper(
     argument_values: &[Value],
     env: &CallEnv,
 ) -> Option<Value> {
+    #[cfg(feature = "perf-counters")] crate::diagnostics::update(|c| {if matches!(argument_values.first(),Some(Value::Number(_))){c.mix_string_wrapper_number+=1;}else{c.mix_string_wrapper_other+=1;}});
     let text = match argument_values.first()? {
         Value::String(text) => text.clone(),
         Value::Number(number) => crate::JsString::from(crate::number::number_to_js_string(*number)),
diff --git a/crates/qjs-runtime/src/value/array.rs b/crates/qjs-runtime/src/value/array.rs
index 52277c99..ddf62d8b 100644
--- a/crates/qjs-runtime/src/value/array.rs
+++ b/crates/qjs-runtime/src/value/array.rs
@@ -162,6 +162,7 @@ impl ArrayData {
 impl ArrayRef {
     pub(crate) fn new(elements: Vec<Value>) -> Self {
         let length = elements.len();
+        crate::diagnostics::count!(mix_array_alloc);
         Self(Rc::new(ArrayData {
             elements: RefCell::new(elements),
             length: Cell::new(length),
@@ -174,6 +175,7 @@ impl ArrayRef {
     }
<SPACE>
     pub(crate) fn new_with_length(length: usize) -> Self {
+        crate::diagnostics::count!(mix_array_alloc);
         Self(Rc::new(ArrayData {
             elements: RefCell::new(Vec::new()),
             length: Cell::new(length),
@@ -214,6 +216,7 @@ impl ArrayRef {
                 }))
                 .unwrap_or_else(|_| unreachable!("fresh array cold state is empty"));
         }
+        crate::diagnostics::count!(mix_array_alloc);
         Self(Rc::new(data))
     }
<SPACE>
diff --git a/crates/qjs-runtime/src/value/object.rs b/crates/qjs-runtime/src/value/object.rs
index 86b72a22..5dcd82f8 100644
--- a/crates/qjs-runtime/src/value/object.rs
+++ b/crates/qjs-runtime/src/value/object.rs
@@ -340,8 +340,9 @@ impl Prototype {
 }
<SPACE>
 /// Object storage reference.
-#[derive(Clone)]
 pub struct ObjectRef(Rc<ObjectData>);
+impl Clone for ObjectRef { fn clone(&self)->Self { crate::diagnostics::count!(mix_object_clone); Self(self.0.clone()) } }
+
<SPACE>
 #[derive(Clone)]
 pub(crate) struct ObjectWeakRef(Weak<ObjectData>);
@@ -1028,6 +1029,7 @@ impl ObjectRef {
             .iter()
             .filter(|key| is_array_index_key(key))
             .count();
+        crate::diagnostics::count!(mix_object_alloc);
         Self(Rc::new(ObjectData {
             properties: RefCell::new(PropertyStorage::dynamic(properties, property_order)),
             property_revision: Cell::new(0),
@@ -1076,6 +1078,7 @@ impl ObjectRef {
         } else {
             PropertyStorage::Shaped { shape, properties }
         };
+        crate::diagnostics::count!(mix_object_alloc);
         Self(Rc::new(ObjectData {
             properties: RefCell::new(properties),
             property_revision: Cell::new(0),
@@ -1105,6 +1108,7 @@ impl ObjectRef {
         debug_assert_eq!(shape.input_len(), 2);
         debug_assert_eq!(shape.unique_len(), 2);
         let index_property_count = shape.index_property_count;
+        crate::diagnostics::count!(mix_object_alloc);
         Self(Rc::new(ObjectData {
             properties: RefCell::new(PropertyStorage::ShapedPair { shape, values }),
             property_revision: Cell::new(0),
diff --git a/crates/qjs-runtime/src/value/object/ordered_data_builder.rs b/crates/qjs-runtime/src/value/object/ordered_data_builder.rs
index 7f2f6c95..1a24aca8 100644
--- a/crates/qjs-runtime/src/value/object/ordered_data_builder.rs
+++ b/crates/qjs-runtime/src/value/object/ordered_data_builder.rs
@@ -61,6 +61,7 @@ impl OrderedDataPropertyBuilder {
     }
<SPACE>
     pub(crate) fn finish(self, prototype: Option<ObjectRef>) -> ObjectRef {
+        crate::diagnostics::count!(mix_object_alloc);
         ObjectRef(Rc::new(ObjectData {
             properties: RefCell::new(self.properties),
             property_revision: Cell::new(0),
```

### Corpus source/hash/output verification and counter collector

Source `collect-corpus.py`, SHA-256 `3826a52e33855aebb81beaef63ef7c20451f14d15a54ecfb3a54db690edb65ea`.

```python
from pathlib import Path
import os,json,re,subprocess,hashlib
from tools.benchmark.measure_lock import wait_for_builds
ROOT=Path.cwd();OUT=ROOT/'target/mixed-call-followup';MAIN=Path('/Users/qingyingliu/Code/js-engine/quickjs-rust');B=OUT/'build/release/qjs'
CASES=[('jetstream3-js-subset','hash-map'),('jetstream3-js-subset','cdjs'),('sunspider-1.0','3d-raytrace'),('sunspider-1.0','access-binary-trees'),('sunspider-1.0','string-validate-input')]
busy=wait_for_builds(60)
if busy:raise SystemExit('RETRY: competing processes remain '+str(busy))
rows={}
paths={case:MAIN/f'target/bundles/{suite}--{case}.js' for suite,case in CASES}
for name in []:
 for c in (4,8):
  for k in (4,8):paths[f'micro/{name}-{c}-{k}']=ROOT/f'target/mixed-call-design/{name}-{c}-{k}.js'
raw=[json.loads(s) for s in (MAIN/'target/comparison/brief01-e54e33eb-20261011/external-raw.jsonl').read_text().splitlines()]
for suite,case in CASES:
 r=next(r for r in raw if r['case_id']==case and r['role']=='candidate' and r['phase']=='measurement')
 assert hashlib.sha256(paths[case].read_bytes()).hexdigest()==r['bundle_sha256'],case
for name,p in paths.items():
 r=subprocess.run([str(B),'--raw',str(p)],capture_output=True,text=True,timeout=120)
 if not name.startswith('micro/'):
  expected=next(x['stdout'] for x in raw if x['case_id']==name and x['role']=='candidate' and x['phase']=='measurement')
  assert r.stdout==expected,(name,r.stdout,expected)
 if r.returncode:raise SystemExit((name,r.returncode,r.stderr[-2000:]))
 (OUT/(name.replace('/','--')+'.counts')).write_text(r.stderr)
 counts={k:int(v) for k,v in re.findall(r'^\s+(\w+) (\d+)\s*$',r.stderr,re.M)}
 rows[name]=dict(source=str(p),source_sha256=hashlib.sha256(p.read_bytes()).hexdigest(),stdout=r.stdout,counters=counts)
 (OUT/'corpus-counts.json').write_text(json.dumps(rows,indent=2))
 print(name,'done',flush=True)
print('binary_sha256',hashlib.sha256(B.read_bytes()).hexdigest())
```

### Nonempty reference/call/branch atomic calibration

Source `atomic/measure.py`, SHA-256 `d1eb05232e2aa5270942f02d9d4b601515f5283d945c94750da21087c8710be6`.

```python
import sys, os, json, statistics, hashlib, subprocess
from pathlib import Path
from tools.benchmark.screen import counter_tool, sample
from tools.benchmark.measure_lock import wait_for_builds, busy_processes
OUT=Path(__file__).resolve().parent
MAIN=Path('/Users/qingyingliu/Code/js-engine/quickjs-rust')
BINS={'rust':MAIN/'target/perf-loop/rev-e54e33eb9371c5cb4627bd8e590a648085a7fa26/qjs','ng':MAIN/'third_party/quickjs-ng/build/qjs'}
DIAG=OUT.parent/'build/release/qjs'
N=600000
# A component is an increment in a nonempty callee, never an empty-body subtraction.
KERNELS={
 'reference': 'o=o.next;',
 'null': 'if(o===null)s=s+v;',
 'truth': 'if(o)s=s+v;',
 'identity': 'if(o===o)s=s+v;',
 'read': 's=this.x;',
 'write': 'this.x=this.x+v;',
 'call': 'o=o.next;',
}
def source(name,c,k):
 body=KERNELS[name]*k
 expected=N*c*(1+k/2) if name in ('truth','identity') else N*c
 param='v'
 arg='h' if name=='call' else '(i&1)'
 init='var s=1,o=v;' if name=='call' else 'var s=this.seed,o=this.head;'
 # Both 4/8 forms have a boxed receiver and the same lowering class.
 return ('function C(){this.seed=1;this.x=1;this.y=2;this.z=3;this.head=null;}\n'
 +'C.prototype.f=function('+param+'){'+init+body+'return s;};\n'
 +'function run(n,a,vec){var s=0;for(var i=0;i<n;i++){var r=a[i&1],h=r.head;'
 +(('s+=r.f('+arg+');')*c)+'}if(!(s>0))throw 1;return s;}\n'
 +'var a=[new C(),new C()],vec={x:1,y:2,z:3},h={value:1,item:1,next:null,left:null,right:null};h.next=h;h.left=h;a[0].head=h;a[1].head=h;var result=run('+str(N)+',a,vec);if(result!=='+str(int(expected))+')throw result;'+('if(a[1].x!=='+str(1+k*c*N//2)+')throw a[1].x;' if name=='write' else '')+'print("OK");\n')
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

### Hot/cold result-kind supplemental calibration

Source `extra/measure.py`, SHA-256 `097cf6d3e824166a211dbce4ecf0e2357522905fd8672cd67524e374099075e5`.

```python
import sys, os, json, statistics, hashlib, subprocess
from pathlib import Path
from tools.benchmark.screen import counter_tool, sample
from tools.benchmark.measure_lock import wait_for_builds, busy_processes
OUT=Path(__file__).resolve().parent
MAIN=Path('/Users/qingyingliu/Code/js-engine/quickjs-rust')
BINS={'rust':MAIN/'target/perf-loop/rev-e54e33eb9371c5cb4627bd8e590a648085a7fa26/qjs','ng':MAIN/'third_party/quickjs-ng/build/qjs'}
DIAG=OUT.parent/'build/release/qjs'
N=600000
# A component is an increment in a nonempty callee, never an empty-body subtraction.
KERNELS={
 'slowread': 's=o.value;',
 'hotreference': 'o=this.head;',
 'numericcall': 's=s+v;',
}
def source(name,c,k):
 body=KERNELS[name]*k
 expected=N*c*(1+k/2) if name in ('truth','identity','numericcall') else N*c
 param='v'
 arg='h' if name=='call' else '(i&1)'
 init='var s=1;' if name=='numericcall' else 'var s=1,o=v;' if name=='call' else 'var s=this.seed,o=this.head;'
 # Both 4/8 forms have a boxed receiver and the same lowering class.
 return ('function C(){this.seed=1;this.x=1;this.y=2;this.z=3;this.head=null;}\n'
 +'C.prototype.f=function('+param+'){'+init+body+'return s;};\n'
 +'function run(n,a,vec){var s=0;for(var i=0;i<n;i++){var r=a[i&1],h=r.head;'
 +(('s+=r.f('+arg+');')*c)+'}if(!(s>0))throw 1;return s;}\n'
 +'var a=[new C(),new C()],vec={x:1,y:2,z:3},h={value:1,item:1,next:null,left:null,right:null};h.next=h;h.left=h;a[0].head=h;a[1].head=h;var result=run('+str(N)+',a,vec);if(result!=='+str(int(expected))+')throw result;'+('if(a[1].x!=='+str(1+k*c*N//2)+')throw a[1].x;' if name=='write' else '')+'print("OK");\n')
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

### Ordinary-vs-method and admission-negative calibration

Source `plain/measure.py`, SHA-256 `55449db52f54157b1202990377f307ff8af43f6f1739a14b9c1d436da79bda76`.

```python
import sys, os, json, statistics, hashlib, subprocess
from pathlib import Path
from tools.benchmark.screen import counter_tool, sample
from tools.benchmark.measure_lock import wait_for_builds, busy_processes
OUT=Path(__file__).resolve().parent
MAIN=Path('/Users/qingyingliu/Code/js-engine/quickjs-rust')
BINS={'rust':MAIN/'target/perf-loop/rev-e54e33eb9371c5cb4627bd8e590a648085a7fa26/qjs','ng':MAIN/'third_party/quickjs-ng/build/qjs'}
DIAG=OUT.parent/'build/release/qjs'
N=600000
# A component is an increment in a nonempty callee, never an empty-body subtraction.
KERNELS={
 'callplain': 'o=o.next;',
 'numericplain': 's=s+v;',
 'guard': 'if(!o)throw v;',
}
def source(name,c,k):
 body=KERNELS[name]*k
 expected=N*c*(1+k/2) if name in ('truth','identity','numericplain') else N*c
 param='v'
 arg='h' if name in ('call','callplain') else '(i&1)'
 init='var s=1;' if name=='numericplain' else 'var s=1,o=v;' if name in ('call','callplain') else 'var s=this.seed,o=this.head;'
 # Both 4/8 forms have a boxed receiver and the same lowering class.
 return ('function C(){this.seed=1;this.x=1;this.y=2;this.z=3;this.head=null;}\n'
 +'C.prototype.f=function('+param+'){'+init+body+'return s;};\n'
 +'function run(n,a,vec,fn){var s=0;for(var i=0;i<n;i++){var r=a[i&1],h=r.head;'
 +((('s+=fn('+arg+');' if name in ('callplain','numericplain') else 's+=r.f('+arg+');'))*c)+'}if(!(s>0))throw 1;return s;}\n'
 +'var a=[new C(),new C()],vec={x:1,y:2,z:3},h={value:1,item:1,next:null,left:null,right:null};h.next=h;h.left=h;a[0].head=h;a[1].head=h;var fn=a[0].f;var result=run('+str(N)+',a,vec,fn);if(result!=='+str(int(expected))+')throw result;'+('if(a[1].x!=='+str(1+k*c*N//2)+')throw a[1].x;' if name=='write' else '')+'print("OK");\n')
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

### Gap-share accounting (NumericPlan/constructors deliberately unpriced)

Source `price.py`, SHA-256 `59823e9f9054ca69bf71c757a5ffbc77ac30bca8cb1c04eb83804b80b56a7b9b`.

```python
from pathlib import Path
import json,statistics
R=Path.cwd();D=R/'target/mixed-call-followup';M=Path('/Users/qingyingliu/Code/js-engine/quickjs-rust')
count=json.loads((D/'corpus-counts.json').read_text());old=json.loads((R/'target/mixed-call-design/costs.json').read_text());atom=json.loads((D/'atomic/costs.json').read_text());extra=json.loads((D/'extra/costs.json').read_text());plain=json.loads((D/'plain/costs.json').read_text());raw=[json.loads(s) for s in (M/'target/comparison/brief01-e54e33eb-20261011/external-raw.jsonl').read_text().splitlines()]
def delta(x,k,m,a='op'):return x[k]['rust'][m][a]-x[k]['ng'][m][a]
out={};prices={}
for metric in ['instructions','cycles']:
 s=delta(old,'scalar',metric)
 prices[metric]={
 'activation':delta(plain,'callplain',metric,'envelope')-s,
 'method_lookup':delta(atom,'call',metric,'envelope')-delta(plain,'callplain',metric,'envelope'),
 'ref_hot':delta(extra,'hotreference',metric),
 'ref_cold':delta(atom,'reference',metric),
 'number_hot':delta(atom,'read',metric),
 'number_cold':delta(extra,'slowread',metric),
 'null':delta(atom,'null',metric),
 'truth':delta(atom,'truth',metric)-s,
 'identity':delta(atom,'identity',metric)-s,
 'reread_packet':delta(old,'write',metric)-delta(old,'read',metric),
 'add':s,'mul':delta(old,'muladd',metric)-s,
 }
for name,record in count.items():
 c=record['counters'];out[name]={}
 for metric in ['instructions','cycles']:
  med={e:statistics.median(x[metric] for x in raw if x['case_id']==name and x['role']==e and x['phase']=='measurement') for e in ['candidate','quickjs-ng']};gap=med['candidate']-med['quickjs-ng'];p=prices[metric]
  groups={
   'activation':c['mix_wide_calls']*p['activation'],
   'method_lookup':c['mix_wide_method_calls']*p['method_lookup'],
   'ref_fields':c['mix_hot_reference']*p['ref_hot']+c['mix_slow_reference']*p['ref_cold'],
   'number_fields':c['mix_hot_number']*p['number_hot']+c['mix_slow_number']*p['number_cold'],
   'null_truth_identity':(c['mix_wide_null_local']+c['mix_wide_null_temp'])*p['null']+(c['mix_wide_ref_truthy']+c['mix_wide_null_truthy'])*p['truth']+(c['mix_wide_identity_compare']+c['mix_wide_binary_identity_compare'])*p['identity'],
   'reread_packet':c['mix_wide_rmw_reread']*p['reread_packet'],
   'add_mul':c['mix_wide_scalar_add']*p['add']+c['mix_wide_scalar_mul']*p['mul'],
  }
  groups['residual']=gap-sum(groups.values());out[name][metric]={'totals':med,'gap':gap,'modeled':groups,'gap_share_pct':{k:v/gap*100 for k,v in groups.items()}}
(D/'pricing.json').write_text(json.dumps({'prices':prices,'cases':out},indent=2))
for n,v in out.items():
 print(n)
 for m in ['instructions','cycles']: print(m,'gap',round(v[m]['gap']),{k:round(s,1) for k,s in v[m]['gap_share_pct'].items()})
```

### Timer-profile recording/export

Source `attribution/profile.py`, SHA-256 `41fab9838a34c0021a9deb3553be994e363c25e18ed84523d531a48b3b508daa`.

```python
from pathlib import Path
import subprocess,json,xml.etree.ElementTree as E,collections
from tools.benchmark.measure_lock import wait_for_builds
D=Path(__file__).resolve().parent;MAIN=Path('/Users/qingyingliu/Code/js-engine/quickjs-rust');B=MAIN/'target/perf-loop/rev-e54e33eb9371c5cb4627bd8e590a648085a7fa26/qjs'
if wait_for_builds(60):raise SystemExit('RETRY: competing processes remain')
results={}
for name in ['null','tree','write','callplain']:
 trace=D/f'{name}-rust.trace'
 if not trace.exists():
  r=subprocess.run(['xcrun','xctrace','record','--template','CPU Counters','--no-prompt','--time-limit','4s','--output',str(trace),'--launch','--',str(B),'--raw',str(D/f'{name}-profile.js')],capture_output=True,text=True)
  (D/f'{name}-rust.log').write_text(r.stdout+r.stderr)
  if r.returncode and not (trace.exists() and 'Recording completed' in r.stdout):raise SystemExit(r.stderr)
 path=D/f'{name}-rust-stacks.xml'
 subprocess.run(['xcrun','xctrace','export',str(trace),'--xpath','/trace-toc/run[@number="1"]/data/table[@schema="time-profile"]','--output',str(path)],check=True,capture_output=True)
 x=E.parse(path);refs={n.attrib['id']:n for n in x.iter() if 'id'in n.attrib};c=collections.Counter()
 for row in x.findall('.//row'):
  a=list(row);bt=refs.get(a[-1].attrib.get('ref'),a[-1]);fs=bt.findall('frame')
  if fs:c[refs.get(fs[0].attrib.get('ref'),fs[0]).attrib.get('name','?')]+=1
 results[name]=dict(c);(D/'flat-samples.json').write_text(json.dumps(results,indent=2));print(name,sum(c.values()),c.most_common(5),flush=True)
```

### Standalone split-bank replay

Source `replay/replay.rs`, SHA-256 `27f463e11610015e20485f88aae7b890e44f8f86cca89e1c82d506754ad94fda`.

```rust
use std::{hint::black_box, rc::Rc};
#[derive(Clone, Copy)]
enum Scalar { Number(f64), Undefined }
#[derive(Clone)]
enum Boxed { Object(Rc<usize>), Number(f64), Undefined }
struct Header { code:usize, bank:usize, pc:usize, parent:usize, result:usize, receiver:Option<Rc<usize>>, owner:Option<Rc<usize>> }
struct Bank { scalars:Box<[Scalar]>, boxed:Box<[Boxed]> }
struct Scheduler { headers:Vec<Header>, banks:Vec<Bank>, current:usize, code:Rc<usize>, receiver:Rc<usize> }
impl Scheduler {
 fn new(depth:usize,width:usize)->Self {
  let headers=(0..=depth).map(|i|Header{code:1,bank:i,pc:0,parent:0,result:0,receiver:None,owner:None}).collect();
  let banks=(0..=depth).map(|_|Bank{scalars:vec![Scalar::Undefined;width].into_boxed_slice(),boxed:vec![Boxed::Undefined;width].into_boxed_slice()}).collect();
  Self{headers,banks,current:0,code:Rc::new(1),receiver:Rc::new(42)}
 }
 #[inline(never)]
 fn enter(&mut self,arg:f64,with_ref:bool) {
  let parent=self.current;let next=parent+1;
  let h=&mut self.headers[next];h.code=black_box(1);h.bank=next;h.pc=0;h.parent=parent;h.result=0;h.owner=Some(self.code.clone());h.receiver=if with_ref{Some(self.receiver.clone())}else{None};
  self.current=next;
  // Bind a zero-based bank at the boundary; no base added per operand.
  let bank=&mut self.banks[next];bank.scalars[0]=Scalar::Number(arg);
  if with_ref { bank.boxed[0]=Boxed::Object(self.receiver.clone()); }
 }
 #[inline(never)]
 fn step(&mut self) {
  let bank=&mut self.banks[self.current];let Scalar::Number(n)=black_box(bank.scalars[0]) else{panic!("lane")};bank.scalars[0]=Scalar::Number(n+1.0);
  if let Boxed::Object(o)=&bank.boxed[0] {black_box(**o);}
 }
 #[inline(never)]
 fn leave(&mut self)->f64 {
  let here=self.current;let h=&mut self.headers[here];let parent=h.parent;let bank=&mut self.banks[here];let Scalar::Number(result)=bank.scalars[0] else{panic!("lane")};bank.scalars[0]=Scalar::Undefined;bank.boxed[0]=Boxed::Undefined;h.owner=None;h.receiver=None;self.current=parent;self.banks[parent].scalars[h.result]=Scalar::Number(result);result
 }
}
fn main() {
 let a:Vec<String>=std::env::args().collect();let n:usize=a[1].parse().unwrap();let depth:usize=a[2].parse().unwrap();let width:usize=a[3].parse().unwrap();let with_ref:bool=a[4]=="1";
 let mut s=Scheduler::new(depth,width);let cap=(s.headers.capacity(),s.banks.capacity());let ptrs:Vec<_>=s.banks.iter().map(|b|(b.scalars.as_ptr(),b.boxed.as_ptr())).collect();
 // Warm every depth; timed runs contain no fresh banks or descriptors.
 for _ in 0..10 {for d in 0..depth{s.enter(d as f64,with_ref);s.step();}for _ in 0..depth{s.leave();}}
 let mut sum=0.0;
 for i in 0..n {let input=(i&1023) as f64;for d in 0..depth{s.enter(input+d as f64,with_ref);s.step();}for _ in 0..depth {sum+=black_box(s.leave());}}
 // Each return propagates the deepest result back through the continuations.
 let periods=n/1024;let rest=n%1024;let expected=(periods*1024*1023/2+rest*rest.saturating_sub(1)/2) as f64*depth as f64+n as f64*(depth*depth) as f64;
 assert_eq!(sum,expected);assert_eq!(s.current,0);assert_eq!(cap,(s.headers.capacity(),s.banks.capacity()));
 for (b,p) in s.banks.iter().zip(ptrs){assert_eq!((b.scalars.as_ptr(),b.boxed.as_ptr()),p);}
 println!("OK header={} scalar={} boxed={} calls={}",std::mem::size_of::<Header>(),std::mem::size_of::<Scalar>(),std::mem::size_of::<Boxed>(),n*depth);
}
```

### Replay increment collection

Source `replay/measure.py`, SHA-256 `e22604edd67ca41b64f5fb13c4342595d4a150e513175de76abaae2912af274b`.

```python
from pathlib import Path
import json,statistics
from tools.benchmark.screen import sample,counter_tool
from tools.benchmark.measure_lock import wait_for_builds,busy_processes
D=Path(__file__).resolve().parent;B=D/'replay';N=10000000
cases=[('hash-map',3,16),('cdjs',8,16),('3d-raytrace',5,32),('access-binary-trees',8,16),('string-validate-input',1,16)]
rows=[];tool=counter_tool()
for name,depth,width in cases:
 for ref in (0,1):
  while True:
   if wait_for_builds(60):continue
   row={'name':name,'depth':depth,'width':width,'ref':ref,'samples':[]};dirty=False
   for pair in range(5):
    if busy_processes():dirty=True;break
    a=sample(tool,[str(B),str(N),str(depth),str(width),str(ref)],60);b=sample(tool,[str(B),str(2*N),str(depth),str(width),str(ref)],90)
    if busy_processes():dirty=True;break
    assert 'OK' in a.stdout and 'OK' in b.stdout
    row['samples'].append({m:(getattr(b,m)-getattr(a,m))/(N*depth) for m in ['cycles','instructions','wall_ns']})
   if dirty:print('RETRY',name,ref,flush=True);continue
   break
  rows.append(row);(D/'results.json').write_text(json.dumps(rows,indent=2));print(name,ref,{m:round(statistics.median(x[m] for x in row['samples']),2) for m in ['cycles','instructions','wall_ns']},flush=True)
```

All scripts are extracted from their named fences to the indicated paths
under this worktree's `target/mixed-call-followup/`. Corpus files are the
SHA-matched brief-01 bundles under the main checkout's `target/bundles/`;
regenerate them with the unchanged repository bundles helper if absent,
then verify against the retained sealed raw row hashes before counting.
The old micro source generator is already retained earlier in this record.
Timer profiles amplify N with that generator, keeping exact assertions;
ordinary profiling termination at the time limit is not an output validator.
