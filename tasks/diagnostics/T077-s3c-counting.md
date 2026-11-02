# T077 S3c count and instruction receipt

Design evidence, 2026-11-01, source **8322fc983da20800ce612ad148781aea30e28d08**,
local arm64, rustc 1.95.0 / LLVM 22.1.2. No runtime/IR source instrumentation edit,
local timing, iMac timing or held-out input inspection. This is not acceptance.
The [brief](../proposals/T077-s3c-brief.md) owns the proposed ABI and gates;
[inventory](../proposals/T077-s3c-inventory.md) owns source/deletion/migration cuts.

## Method and limitations

The [records](T077-s3c-counts.json) retain **109 programs**: 10 development,
45 external, 25 broad, 6 sentinel, 21 existing S3b reproducers, 2 diagnostic
fixed-clock companions. All stdout/sentinels match across counter, ordinary
release and coverage builds. The ten development workload hashes and complete
counter maps also exactly match the [S3b final maps](T077-s3b-final3-owner/candidate-counts.json).

Each original program runs twice with existing `perf-counters`; maps match for
**84/86** public programs, all 21 reproducers and both fixed-clock companions.
The two live-input exceptions are audio-beat-detection's Date-driven processing
and SunSpider crypto-aes's Date nonce. Both original maps are retained, not
silently replaced. Coverage is one separate replay per program and can follow a
different live-input branch in those two cases. The fixed-clock companions give
deterministic diagnostic counts, **not** public acceptance measurements. All
cost estimates give the two original variable-input programs zero saving credit.

Retired instructions are read from `/usr/bin/time -l` on an **uninstrumented**
release binary: one sample per corpus/companion program, three per reproducer
(use median). All wall/cycle/RSS fields from that tool are discarded. Coverage
uses the unmodified source built with `-Cinstrument-coverage`, LTO off, then
LLVM 22's llvm-profdata/llvm-cov; only function/region event counts are used,
never instructions from that build. Counter/coverage builds must never be timed.
Binary hashes, recipes, source/workload hashes, arguments and source probe lines
are in the records. The regular/count builds use checked-in release fat LTO/16
units/abort and `.cargo/config.toml`; the coverage RUSTFLAGS replace those flags.

Whole-process instructions include parse/startup/shutdown. Broad/sentinel use a
fixed **100,000 iterations** each, with checksums/operation counts verified; no
wall-based calibration. Their reach and the selection model consequently are
for this diagnostic invocation, not the formal harness's calibrated samples.
Held-out remains **unpriced**; its earlier accepted wall geomean 0.9698 is a
historical S3b result, not a count or expected S3c payoff.

## Counted reach at main

Counts below are event sums, not heap-allocation counts. External coverage sums
are one live-input replay; its two variable programs prevent a deterministic
claim for the original full lane. Per-case values and both counter maps remain
in JSON. The 43 other external programs are repeat-identical. Do not add rows
that describe the same call from different layers.

| Event / observed operation | Development 10 | External 45 | Broad 25 | Sentinel 6 |
| --- | ---: | ---: | ---: | ---: |
| ordinary call attempts | 29,298,761 | 11,051,056 | 1,000,100 | 300,222 |
| general FrameState/Vm constructions | 2,137,508 | 222,838 | 39 | 14 |
| new local-vector backing stores | 673 | 259 | 39 | 11 |
| new operand-vector backing stores | 676 | 259 | 39 | 11 |
| inline activation-slot entries (= releases) | 18,305,298 | 7,812,174 | 100,016 | 75 |
| ActivationStack.current bindings at switches/segments | 38,512,516 | 17,346,360 | 200,036 | 152 |
| ActivationContext::for_call builds | 1,626,499 | 250,851 | 73 | 22 |
| ActivationContext::new builds (separate factory) | 512,713 | 25,705 | 27 | 10 |
| reuse_for_call attempts | 24,995,766 | 7,852,557 | 100,026 | 71 |
| bind_indexed_call_owner calls | 26,621,997 | 8,103,408 | 100,099 | 93 |
| wide StoreUpvalueLocal writes | 816,965 | 2,571 | 100,000 | 0 |
| all BindingCell::assign calls (overlaps preceding row) | 1,804,423 | 480,968 | 100,000 | 0 |
| VM direct cell slow-read branch | 19,141,872 | 237,259 | 0 | 0 |
| checking-dispatch iterations | 16,905,987 | 2,259,994 | 2 | 4 |
| run_reference_event calls | 0 | 27,562 | 0 | 0 |
| snapshot exports / resume reconstructions | **0 / 0** | **0 / 0** | **0 / 0** | **0 / 0** |
| generator start/prologue entries | 0 | 0 | 0 | 0 |
| pop_arguments Vec factory calls | **0** | **90** | 0 | 0 |

The two activation factories (`new`, `for_call`) are disjoint; reuse attempts
do not construct a new context. Standalone indexed-entry preparation runs
**8,316,966 / 291,234** times in development/external; general prepared-function
entry runs **512,696 / 19,797** times. They overlap owner binds/builds above,
and are unpriced reach ceilings for the proposed S3d entry-path experiment.
A `for_call` build and a reuse attempt are not two logical activations. The
switch count includes root binding and multiple execution segments: it is not
exactly two switches per call. `StoreUpvalueLocal` includes shared global-lexical
cells; these counts do not prove the InitializedMutable policy for every write.
The direct-cell read row includes ordinary captures/own exposed cells as well
as eval locals; it is a candidate reach ceiling, not a count of eval-only loss.
Checking iterations can include a terminal fetch, not just executed opcodes.

The development pool requests remain 2,137,508 locals and operands, 457,653
cell allocations, 328,929 promotions, 5,368 leases and 1,388 lexical owners.
Fresh storage is already uncommon: saving a pool request is not saving malloc.
Scope/cell/lexical-owner identities cannot be pooled merely to reduce those counts.

`compact_slice_fallbacks` / `wide_slice_fallbacks` are **567,740 / 10,117,049**
in development and **7 / 1,913,236** in external. These helper calls already
borrow slices, often for natives. They are **not argument-vector allocations**.
The old S3d numbers (7,438,472 pop vectors, 448,836 tier vectors) from dd0905b2
are obsolete for pricing main: S3a/S3b have removed the ordinary copies. Semantic
Proxy/bound/spread/rest argument lists remain separately owned, not deletable costs.

## Reproducer instruction calibration

Current instructions are absolute median retired instructions on main, with
all input/code/counter details in JSON. The 130–150 call protocol and ~125
checked-write component come from [S3b round 4](T077-s3b-progress.md#round-4-milestone-2-causes-35),
which supersedes the earlier investigator's ~480/call estimate in
[attribution](T077-s3b-r4/attribution.md). No old candidate is described as main.

| Reproducer | Main instructions | Relevant reached event |
| --- | ---: | --- |
| upw0 | 1,024,145,145 | 1,000,000 inline entries, 0 captured writes |
| upw1 | 1,354,179,813 | same entries, 1,000,000 wide cell writes |
| upw2 | 1,668,212,228 | same entries, 2,000,000 wide cell writes |
| upw4 | 2,296,309,490 | same entries, 4,000,000 wide cell writes |
| call-no-global | 1,007,968,073 | no inline entries; folded-call control |
| call-global-var-write | 5,392,419,215 | 1,000,000 owner binds, no reference events |
| call-implicit-global-write | 6,749,497,887 | 1,000,000 owner binds, 4,000,008 events, 16,000,041 checking iterations |
| vm-ops-plain | 1,453,014,344 | 0 checking iterations |
| vm-ops-local | 4,789,198,613 | 6,000,003 direct cell reads, 17,000,014 checking iterations |
| vm-ops | 7,575,417,666 | global-routing control, 0 checking iterations |
| eval-closure | 4,069,585,520 | 300,000 direct cell reads |
| eval-parent-closure | 1,615,107,436 | 300,000 owner binds |

upw1→upw2 adds **314.03 instructions/write**; upw2→upw4 adds **314.05**,
with identical 1M entry counts and the same tier. This prices the whole extra
read/add/write sequence, not just assign. Replacing ~125 instructions of policy
work by a 25–45 instruction proven mutable-cell replace saves **80–100**, leaving
an estimated whole additional sequence around **214–234**. upw0's 1,024 inclusive
instructions/call would fall to about **904–954** if the net 70–120 call saving
is achieved. The residual call-protocol budget itself is **30–60**. These are
design budgets; no post-change binary exists.

For vm-ops-local the read budget alone is 6,000,003 × 15–30 = **90–180M** saved
instructions; removing checking polling adds 17,000,014 × 5–10 = **85–170M**.
For call-implicit-global-write, 4,000,008 × 5–15 event overhead plus
16,000,041 × 5–10 polling gives **100–220M**, before any non-inline entry work.
This does not explain away its entire historical 1.32 ratio. Global writes still
pay full observable SetMutableBinding semantics; ordinary vm-ops global routing
has no checking-loop credit. Treating every global event or slow read as removable
would violate the binding/reference contract.

## Reach times saved cost: conditional envelope, not a forecast

For case i, N is its measured uninstrumented instructions and
S = A×inline_entries + W×wide_cell_writes + E×reference_events
+ L×direct_cell_reads + O×checking_iterations. Inline entries use min(entry,release).
The two variable-input originals get S=0. Low/high credit budgets are
(A,W,E,L,O) = **(70,80,5,15,5)** / **(120,100,15,30,10)** instructions/event.
The ratio is geomean(1−S/N), **not** 1−sum(S)/sum(N). Counts overlap only as
explicitly separate cost components: e.g. polling vs event selection vs cell
routing. The budgets must be verified net of newly emitted boundary dispatch.

| Lane | Call saving | Write saving | Reference + read + polling saving | Conditional instruction ratio |
| --- | ---: | ---: | ---: | ---: |
| development | 1,281.37–2,196.64M | 65.36–81.70M | 371.66–743.32M | **0.98455–0.99118** |
| external (2 variable cases zero credit) | 545.68–935.45M | 0.106–0.132M | 14.92–29.97M | **0.99346–0.99626** |
| broad (100k iterations) | 7.001–12.002M | 8–10M | <0.001M | **0.99446–0.99631** |
| sentinel (100k iterations) | 0.0053–0.0090M | 0 | <0.001M | **0.999988–0.999993** |
| equal-case selection, 86 | 1,834.06–3,144.09M | 73.46–91.83M | 386.58–773.29M | **0.99317–0.99594** |
| held-out | unmeasured | unmeasured | unmeasured | **unpriced, acceptance only** |

Low/high are budget endpoints, not confidence intervals or guaranteed lower
bounds. They assume all counted candidate stores/reads qualify. New directory,
block/box traffic, optional exposure and root ownership conversion are unmeasured
and may erase gains. No suspension savings are booked: development and external
both have zero suspension reach. Standalone/root owner work is an additional
unpriced ceiling, not already included in inline savings.

The external budget misses **<=0.983**. With the other high credits fixed, the
same model needs about **325 saved instructions per inline entry**, more than
the whole **130–150** S3b call-binding protocol. Counts do not translate linearly
to wall time, but there is no credible instruction payer for full recovery in
ownership alone. Neither a 3% wall promise nor a cheaper argument-Vec claim is
supported. See the inventory's bounded entry-path experiment and combined-S3d
re-scope options; the lead must freeze one scope before runtime work.

## Reproduce without editing runtime source

Initialize only the two pinned top-level submodules with `./scripts/bootstrap.sh`.
Use main 8322fc98, or verify all runtime/compiler/build inputs are identical to it.
Do not run the timing collectors. The collector below writes only under target;
extract its Python fence to `target/s3c-collect.py`, then run the builds and
`PYTHONPATH=. PYTHONDONTWRITEBYTECODE=1 python3 target/s3c-collect.py`.
Keep llvm-tools-preview for rustc 1.95.0 installed (`rustup component add
llvm-tools-preview`); the host Homebrew LLVM 21 is not the matching coverage reader.

```sh
cargo build --release -p qjs-cli --features perf-counters --target-dir target/s3c-counters
cargo build --release -p qjs-cli --target-dir target/s3c-instructions
RUSTFLAGS='-Cinstrument-coverage' CARGO_PROFILE_RELEASE_LTO=off CARGO_PROFILE_RELEASE_CODEGEN_UNITS=16 cargo build --release -p qjs-cli --target-dir target/s3c-coverage
RUSTC_BOOTSTRAP=1 cargo rustc -p qjs-runtime --target-dir target/s3c-sizes -- -Zprint-type-sizes
```

The last command is inventory-only compiler type-layout output, with no runtime
patch; it reports FrameState 448, ColdFrame 288, snapshot 464, ActivationContext
120, CallOwner 48, CompactFrame 80, WideFrame 48, Value 16 bytes on arm64.
These are not release stack-depth or x86 claims. Coverage-instrumented build
scripts can leave a default*.profraw in crates/qjs-cli; keep it out of commits.
A fresh target/s3c-counts directory is required for a full independent replay;
the collector's cache resumes partial runs and must not be reused across revisions.

```python
from pathlib import Path
import os,json,subprocess,re,hashlib,sys
from tools.benchmark.screen import resolve_cases,DEFAULT_CACHE,_check_output
ROOT=Path.cwd(); out=ROOT/'target/s3c-counts'; out.mkdir(exist_ok=True)
work=out/'programs'; work.mkdir(exist_ok=True)
cs=resolve_cases(['development','external','broad','sentinel'],DEFAULT_CACHE,work)
llvm=Path(subprocess.check_output(['rustc','--print','sysroot'],text=True).strip())/'lib/rustlib/aarch64-apple-darwin/bin'
env={k:v for k,v in os.environ.items() if not k.startswith(('QJS_','LLVM_PROFILE_FILE'))}
bins={k:ROOT/f'target/s3c-{k}/release/qjs' for k in ['counters','instructions','coverage']}
probe_specs={
 'activation_plain_builds':('context/activation.rs','new'),
 'prepared_function_entries':('function/call.rs','enter_function_activation'),
 'standalone_indexed_entries':('function/call.rs','enter_indexed_function_activation'),
 'activation_slot_entries':('bytecode/compact_fn/activation_stack.rs','callee_slot'),
 'activation_slot_releases':('bytecode/compact_fn/activation_stack.rs','release'),
 'activation_slot_switches':('bytecode/compact_fn/activation_stack.rs','current'),
 'activation_builds':('context/activation.rs','for_call'),
 'activation_rebinds':('context/activation.rs','reuse_for_call'),
 'activation_release_checks':('context/activation.rs','release_for_reuse'),
 'owner_binds':('function/call.rs','bind_indexed_call_owner'),
 'wide_pushes':('bytecode/compact_fn/wide/activation/entry.rs','push_callee_frame'),
 'snapshot_exports':('bytecode/vm_generator.rs','into_snapshot'),
 'resume_reconstructions':('bytecode/vm_generator.rs','run_from_yield'),
 'generator_prologue_entries':('bytecode/vm_generator.rs','start_suspended_at_body'),
 'generator_start_entries':('bytecode/vm_generator.rs','run_from_start'),
 'argument_vec_pops':('bytecode/vm.rs','pop_arguments'),
 'compact_slice_fallbacks':('bytecode/compact_fn/activation.rs','call_from_activation'),
 'wide_slice_fallbacks':('bytecode/compact_fn/wide/activation.rs','call_from_activation'),
 'cell_assigns':('binding/cell.rs','assign'),
 'vm_local_slow_reads':('bytecode/vm_bindings.rs','load_local_slow'),
 'reference_events':('bytecode/vm_references.rs','run_reference_event'),
}
region_specs={
 'checked_dispatch_ops':('bytecode/vm.rs','if let Some(active) = self.current.binding_scope',500),
 'wide_captured_writes':('bytecode/compact_fn/wide/activation.rs','let Some(cell) = activation.upvalue_cell(slot as usize) else {',816),
 'vm_direct_cell_reads':('bytecode/vm_bindings.rs','return cell.read();',400),
}
probe_lines={}
for key,(path,fn) in probe_specs.items():
 s=(ROOT/'crates/qjs-runtime/src'/path).read_text().splitlines()
 probe_lines[key]=(path,next(i+1 for i,line in enumerate(s) if re.search(r'\bfn '+fn+r'\b',line)))
for key,(path,marker,lower) in region_specs.items():
 s=(ROOT/'crates/qjs-runtime/src'/path).read_text().splitlines()
 probe_lines[key]=(path,next(i+1 for i,line in enumerate(s) if i+1>=lower and marker in line))
rows=[]
items=[(c.id,c.workload,([c.case_id,'100000'] if c.kind=='internal' else []),c) for c in cs]
audio=next(c for c in cs if c.id=='external/kraken-1.1/audio-beat-detection')
clock=work/'audio-beat-detection-fixed-clock.js'
clock.write_text('var __s3cTimestamp = 0;\n'+audio.workload.read_text().replace('(new Date()).getTime()', '(__s3cTimestamp += 16)'))
items += [('diagnostic/audio-fixed-clock',clock,[],audio)]
aes=next(c for c in cs if c.id=='external/sunspider-1.0/crypto-aes')
aesclock=work/'crypto-aes-fixed-clock.js'
aesclock.write_text(aes.workload.read_text().replace('(new Date()).getTime()', '1234567890000'))
items += [('diagnostic/aes-fixed-clock',aesclock,[],aes)]
items += [('repro/'+p.stem,p,[],None) for p in sorted((ROOT/'tasks/diagnostics/T077-s3b-r4/repro').glob('*.js'))]
for ident,program,args,case in items:
 key=ident.replace('/','--'); dest=out/(key+'.json')
 prior=json.loads(dest.read_text()) if dest.exists() else None
 if prior and set(probe_lines)<=set(prior['coverage']):
  rows.append(prior); continue
 maps=[]; stdout=None
 if prior:
  maps=[prior['counters'], prior['counters'] if prior['counter_repeat']=='identical' else prior['counter_repeat']]
  instructions=prior['instructions']
 else:
  for repeat in range(2):
   r=subprocess.run([str(bins['counters']),'--raw',str(program),*args],env=env,text=True,capture_output=True,timeout=600)
   assert r.returncode==0,(ident,r.stderr)
   if case: _check_output(case,r.stdout)
   if stdout is None: stdout=r.stdout
   assert stdout==r.stdout,ident
   maps.append({k:int(v) for k,v in re.findall(r'^\s+(\w+) (\d+)$',r.stderr,re.M)})
  assert maps[0],ident
  if ident.startswith(('development/','repro/','diagnostic/','broad/','sentinel/')): assert maps[0]==maps[1],ident
  instructions=[]
  for repeat in range(3 if case is None else 1):
   r=subprocess.run(['/usr/bin/time','-l',str(bins['instructions']),'--raw',str(program),*args],env=env,text=True,capture_output=True,timeout=600)
   assert r.returncode==0 and r.stdout==stdout,(ident,r.returncode)
   instructions.append(int(re.search(r'(\d+)\s+instructions retired',r.stderr)[1]))
 raw=out/(key+'.profraw'); prof=out/(key+'.profdata')
 r=subprocess.run([str(bins['coverage']),'--raw',str(program),*args],env={**env,'LLVM_PROFILE_FILE':str(raw)},text=True,capture_output=True,timeout=600)
 if prior:
  assert r.returncode==0 and hashlib.sha256(r.stdout.encode()).hexdigest()==prior['stdout_sha256'],ident
  stdout=r.stdout
 else: assert r.returncode==0 and r.stdout==stdout,ident
 subprocess.run([str(llvm/'llvm-profdata'),'merge','-sparse',str(raw),'-o',str(prof)],check=True,capture_output=True)
 cov=json.loads(subprocess.check_output([str(llvm/'llvm-cov'),'export',str(bins['coverage']),'-instr-profile='+str(prof)],text=True))['data'][0]
 events={}
 for name,(path,line) in probe_lines.items():
  funcs=[f for f in cov['functions'] if f['filenames'][0].endswith('/'+path) and f['regions'][0][0]==(line if name in probe_specs else f['regions'][0][0]) and not f['name'].startswith('_RNC')]
  if name in probe_specs:
   assert funcs,(name,ident)
   events[name]=sum(f['count'] for f in funcs)
  else:
   matches=[r[4] for f in funcs for r in f['regions'] if r[0]==line and r[5]==0 and r[7]==0]
   assert matches,(name,ident)
   events[name]=max(matches)
 row={'id':ident,'sha256':hashlib.sha256(program.read_bytes()).hexdigest(),'args':args,'stdout_sha256':hashlib.sha256(stdout.encode()).hexdigest(),'instructions':instructions,'counters':maps[0],'counter_repeat':maps[1] if maps[0]!=maps[1] else 'identical','coverage':events}
 dest.write_text(json.dumps(row,indent=2)+'\n'); rows.append(row)
 print(ident,events['activation_slot_entries'],events['wide_captured_writes'],flush=True)
 raw.unlink();prof.unlink()
metadata={'base':subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip(),'rustc':subprocess.check_output(['rustc','-vV'],text=True),'host':subprocess.check_output(['uname','-m'],text=True).strip(),'binary_sha256':{k:hashlib.sha256(p.read_bytes()).hexdigest() for k,p in bins.items()},'probe_lines':probe_lines,'internal_iterations':100000,'counter_repeats':2,'instruction_repeats':{'corpus':1,'repro':3},'coverage_repeats':1,'records':rows}
(out/'counts.json').write_text(json.dumps(metadata,indent=2)+'\n')
```

## Design verification

At the design tree over 8322fc98, `check-context.py`, staged
`check-touched.sh --staged --explain` and `git diff --check` pass. Runtime,
compiler, scripts, Cargo/build configuration, benchmarks and tests are identical
to the source base. No runtime semantic file changed, so the fast gate selected
no crate tests or focused Test262 slice. Full check, NG, stress, depth and exact
scans are implementation acceptance gates and were **not run for this design**;
no new conformance, RSS or performance standing is claimed.
