# T065 gap anatomy: source comparison and non-timing instruction baseline

Design evidence collected 2026-10-22 UTC at `0ab98e3920629da1d7d6efae46f24f0b43bfa018`.
No runtime implementation, performance acceptance or timing job is part of T065.
The branch's runtime/build inputs have an empty diff from measured main
`9eba7a1d6ae8b67397d20e0d6c93ac79037f5e30` (M). NG is pinned
`f7830186043e4488f2998759d60a514faf07cbc9`. Top-level NG was initialized for
read-only source inspection; no nested submodule or reference content changed.

## What “about 3x” establishes

[T040](../T040-imac-general-performance.md#current-measured-standing-2026-10-21)
records all ten development candidate/NG wall ratios, geomean **3.071**; js-yaml
4.3374, immutable 4.0096, moment 3.6504, diff 3.6178, meriyah 3.3434. Broad
property_read/write/array_read are 5.1528/3.78/3.26. External SunSpider/JetStream
subset/Kraken geomeans are 1.084/1.163/.780. The source main here is newer, but
runtime/build inputs match M; this is not a new wall measurement at this branch.
The formal's base 9a0eeeca lacks lodash, so candidate/base readiness is incomplete;
that does not erase its complete current candidate/NG development standing.

Real programs encounter ordinary calls, mutable objects, computed keys and
coercions together. Fast scalar kernels already mask those costs in many suite
cases. Broad property_read's exact-M samples are **64.60% virtual-object owner**
and 35.40% loop, with zero observed named-owner samples. Its 5.15x gap is not
proof that ordinary property lookup alone is 5.15x too expensive.

## Fresh instruction collection, both engines, all ten workloads

After wrapper status showed every prior job done, submitted non-timing job
**20261022T093517Z-t065-core-v3-instructions-v2**, exit **0**. Used only
`imac-bench.sh run --name ... -- '<collector>'`, then `log`/`fetch`; no direct
SSH, timing flag, cancellation, local/release build or origin branch push.
The wrapper's internal runner ref transfer is measurement transport only.
Reused exact canonical binaries, hash-checked before any workload:

- Rust M: `f5822aa32e4c7a3def2aaf5b8eff2107ba005a76252afb25f06077fe25f2e8eb`.
- NG: `919f7235f61412ba69faf6aa05b534869aa7d69d1583b3d7aab3cd5b67cd6f40`.
- Host: macOS 12.6 x86_64; harness source 0ab98e39; no build or recipe change.

Resolve `development` with `tools.benchmark.screen.resolve_cases`; its npm loader
verifies tarball/file/driver hashes and creates the existing complete script
bundles. For each program run three pairs, Rust→NG, NG→Rust, Rust→NG, each as a
fresh process through `/usr/bin/time -l`. Rust argv is `qjs --raw <bundle>`; NG
is `qjs --script <bundle>`. Validate checksum and the role-specific external
sentinel lines, exit 0 and positive retired instructions. Save stdout and raw
rusage for every run; ignore elapsed/user/system/cycle fields. No held-out input
was read or profiled. All **60** instruction records passed checksum validation.

Fetched raw outputs remain workspace-local at
`outputs/imac-bench/20261022T093517Z-t065-core-v3-instructions-v2/`.
Recomputed every raw/stdout/workload SHA-256 against the 60 records after fetch;
all match. Git retains the numerical conclusion and provenance below, not npm
source bundles or claims that local artifacts are durable repository evidence.

| Program | Rust instructions (median) | NG instructions (median) | Median paired Rust/NG | All three paired range |
| --- | ---: | ---: | ---: | --- |
| lodash/transform | 17,512,939,642 | 8,121,183,513 | 2.1566 | 2.1559–2.1567 |
| meriyah/parse | 19,137,184,322 | 7,341,190,253 | 2.6068 | 2.5800–2.6079 |
| immutable/persistent | 28,271,748,328 | 10,679,729,426 | 2.6480 | 2.6471–2.6519 |
| moment/dates | 65,807,110,534 | 25,552,138,085 | 2.5754 | 2.5746–2.5761 |
| diff/compare | 59,095,506,257 | 20,274,143,573 | 2.9149 | 2.9142–2.9154 |
| papaparse/csv | 24,299,800,134 | 11,751,626,302 | 2.0664 | 2.0630–2.0707 |
| js-yaml/document | 32,005,991,445 | 9,221,679,369 | 3.4708 | 3.4661–3.4722 |
| big.js/arith | 24,387,400,827 | 12,918,669,098 | 1.8877 | 1.8876–1.8880 |
| matter-js/simulate | 30,909,835,003 | 17,659,546,435 | 1.7503 | 1.7503–1.7504 |
| graphlib/paths | 10,642,593,386 | 4,909,427,777 | 2.1678 | 2.1677–2.1682 |

Equal-case geomean of the ten paired medians = **2.3762824**. Whole-process
counts include parse/compile/startup/teardown and engine work, not just dispatch.
The NG count contains its own parsing and builtin costs; it is not a minimum
instruction budget for a JS program. Variability is reported, not trimmed away.

At the aggregate level, 3.071/2.3763 = **1.2924** is an indicative relative
wall-per-instruction factor. In log-ratio units, instruction inflation accounts
for about **77%**, this residual about **23%**, of the observed slowdown. This is
an inference combining separate collections, not synchronized per-symbol CPI,
a causal attribution or a memory-latency measurement. Instruction reduction is
a substantial opportunity; flat instructions cannot rule out cache/code-layout,
allocation/GC or other latency changes. Use formal wall evidence at acceptance.

| Artifact | SHA-256 |
| --- | --- |
| command.sh | `85e8ef688ae6baf014c4ff504d96d7b59364d90b8426149225fc9ecf32fd6c5a` |
| out/identity.json | `0a2e3d6ca07716de64dc418728ddb5979d7659f8e57fa2229a8b1cf674d4581f` |
| out/instructions.json | `90071c5972e18f15c5b6d8d4b90bdb8cb0a88cdec4b248d51f9d332909bb6668` |

Collector failure retained: first job **20261022T093340Z-t065-core-v3-instructions**
exited 1 on the first program's stdout contract. It incorrectly applied the
Rust-only double sentinel expectation to NG and omitted NG's canonical --script
argument. No complete records or engine-performance result came from that job.
The v2 collector uses canonical per-role argv/expectations and saves raw stdout
before validating it. This was a collector correction, not a discarded adverse
candidate. Both jobs remain in the runner history.

## Mechanism shares: what can actually be estimated

An exact percentage of the *Rust-minus-NG gap* per mechanism is not identified:
we do not have matched causal NG/Rust per-mechanism costs. Profiles report Rust
time shares, counters report source events, and neither directly partitions
excess time. Inlined handlers, ownership and allocation overlap. Reporting
additive percentages summing to 100 would manufacture evidence.

For a reviewable estimate use **visible gap-equivalent reach**: normalize a
Rust profile fraction f by `1 - 1/3.071 = .67437`. It estimates the fraction of
the lane's excess time that deleting *all* visible f could cover if the rest of
the process were unchanged. Zero to that number is a conditional removable-reach
range, not a bound on the full mechanism's cost or a predicted speedup: NG pays
some of the same work, hidden/inlined work can be larger, and f is sampled.

The following common denominator deliberately uses the historical nine-program
non-papaparse equal-case means from [T056 part 2](T056-unit0-part2-evidence.md).
It is a proxy for development reach, **not a measured all-ten current-lane
partition**. Papaparse's old 91.7% string profile predates the retained T058
RegExp change and cannot be carried forward. Current exact-M five-program
profiles corroborate property/call priority, but leave four current programs
without equivalent profiles. Fresh counts above fill the all-ten instruction
gap; they cannot repair this mechanism-attribution limit.

| Mechanism | Estimated visible share of development gap (conditional proxy) | Evidence / limit |
| --- | --- | --- |
| Property names, storage, probes | **0–23%** (15.3% historical Rust owner reach / .674) | 0–29% using exact-M top-five named owner mean 19.33%; moment/diff 4.0/5.7 comparisons per attempt historically, 46–93% cache rejects. Required lookup semantics remain. |
| Calls, frames, entry/env plumbing | **0–27%** (18.3% / .674) | Exact-M top-five broad owner mean 21.06% gives 0–31%; narrow E+V is 16.32% graphlib and 20.14% immutable. Not all ordinary calls use the same route. |
| Value clone/drop and refcount transport | **0–15% directly visible**, overlaps both rows above (9.82% / .674) | 29–139M Value clones/program; 42–91% heap-owning except big.js 15%; direct glue misses inlining. An 8-byte word with RC keeps most ownership work. |
| Allocation/heap/collection | **Not separately identifiable** | Allocator self 10–25% in seven historical development programs, already charged to semantic owners. Current Rust does not collect cycles; eliminating RC adds tracing/root work. No separate additive gap share is defensible. |
| Strings / RegExp | **0–8%** on the nine-program historical proxy (5.27% / .674); all-ten current share unknown | Exact-M js-yaml 20.83%, moment 9.26%, diff 8.38%; normalized individual gap reach is much larger there. Old papaparse profile is superseded. |
| Bytecode format / pure dispatch | **Unknown**, not the 53.4% loop owner (79% normalized reach) | Loop self includes arithmetic, inline property/clone work, tiers and spills. T024 preamble split gave only external .9906 / sentinel .9989, so dispatch alone has weak support. |
| Exceptions / suspension / native ABI | **Unknown as separate gap shares** | Call-owner buckets include natives; no isolated exception counts. Source supports colder completion/ABI design, not an invented steady-state percentage. |

These are overlapping opportunity envelopes, never sum them. The supported
judgment is **ownership/value transport + property authority + call/context
construction together**, not “dispatch explains 3x”. The new collector is also a
correctness/retention change; its speed benefit is unmeasured. M0 must determine
whether their combination removes work after rooting, GC and builtin costs.

Current exact-M corroboration, from [T064 inputs](T064-successor-plan-inputs.md):

| Program | Named owner | Vm+env+call owner | String/RegExp owner | Loop owner |
| --- | ---: | ---: | ---: | ---: |
| moment | 19.96% | 17.58% | 9.26% | 37.68% |
| diff | 23.73% | 26.78% | 8.38% | 32.19% |
| meriyah | 28.10% | 14.66% | 2.93% | 49.81% |
| js-yaml | 13.11% | 17.29% | 20.83% | 43.43% |
| immutable | 11.74% | 28.99% | 1.06% | 47.42% |

Narrow [M2 pricing](T056-m2-plan-inputs.md#fresh-candidate-reach-and-price)
separates setup from handlers. Its independent prototype removes **15.39% whole
instructions on immutable**, but only **0.307% on graphlib**; credit limited to
sampled E+V is 9.448% and .307%. Only 3.46%/0.085% of total activations entered
the narrow prototype respectively. Do not scale those savings by total ordinary
call counts or forecast a whole-lane 20% gain from a single frame slice.

## Pinned source comparison, per mechanism

Paths below are under `crates/qjs-runtime/src/` for Rust and
`third_party/quickjs-ng/` for NG, inspected at the identities above. They are
source evidence, not a new benchmark or a claim about another NG release.

| Mechanism | Rust today | Pinned QuickJS-NG | Consequence for v3 |
| --- | --- | --- | --- |
| Value and clone/drop | `value/mod.rs` asserts 16 bytes; enum variants wrap Rc-backed handles. Function/array/object/map/set/proxy/string paths retain/drop owners. | `quickjs.h:155–159,299–309`: default 64-bit JSValue is **16 bytes** (union + int64 tag); NaN boxing defaults to 32-bit builds. `quickjs.c:1564` js_dup increments a common count only on refcounted tags; interpreter handlers transfer stack ownership explicitly. | Do not claim NG wins by an 8-byte Value. Word width helps footprint; tracing removes RC copies, at the price of explicit roots and collection. |
| Heap and cycles | `value/mod.rs`, `value/teardown.rs`, Upvalue and suspension headers: Rc/RefCell, iterative long drops, no cycle collection. | `quickjs.c:1583` allocation threshold, JSRefCountHeader/JSGCObjectHeader; `JS_RunGC:7078` decref/scan/free-cycles over counted objects. | Arena + nonmoving tracing is different from T023's failed intrusive-RC arena. Measure live/retained bytes and root scanning, not just allocation count. |
| Objects / atoms | `value/object.rs`: Small vector, Dynamic index, Shaped and ShapedPair; property_revision/layout_revision; `bytecode/linked_code.rs` links names once but ordinary stores still use their own key authorities. | `quickjs.h` JSAtom is u32; `quickjs.c:1020–1040` JSShapeProperty stores atom/flags/hash chain, JSShape shares names/prototype, JSObject separates property payload. `find_own_property:6466` hashes integer atom then compares atoms. | Uniform runtime atom/shape slots and data ICs across all creation paths, bounded churn; retain full descriptor/exotic algorithms. NG's hash lookup is not evidence it has the proposed v3 IC design. |
| Bytecode / dispatch | `bytecode/ir.rs` Op enum holds usize/String-rich operands; general stack dispatch plus numeric/compact/wide/typed/virtual streams. vm.rs header describes helper calls and authoritative ip. | `quickjs-opcode.h` defines short variable-length opcodes; `JS_CallInternal:17550` has byte PC and local sp; DIRECT_DISPATCH branch supports computed goto, otherwise switch. | Compact verified register encoding and one common frame ABI; use Rust match initially. Do not assume computed goto can be copied to stable Rust or promise dispatch-only gain. |
| Calls / frames / env | `function/call.rs`, `bytecode/vm.rs`: Vec args, CallEnv preparation, nested Vm for frame-building calls; compact has separate windows. Slot/cell identity already exists and stays protected. | `quickjs.c:369` JSStackFrame has arg_buf/var_buf/var_refs; JS_CallInternal borrows argv where possible and allocates local/stack storage together with alloca. Captures use JSVarRef, closed on return. NG itself can recurse in C. | Typed context plus one driver/reused windows; no per-call name assembly. Preserve our canonical cells, not NG's exact C stack lifetime strategy. |
| Strings | `value/js_string.rs`: canonical sentinel UTF-8 StringData behind Rc, ASCII/UTF-16 length and matcher caches; String/RegExp still translate some coordinates. | `quickjs.c:618` cached unit len + narrow/wide flag; string kinds include slice/indirect, and JSStringRope has depth/left/right. Narrow is 8-bit, wide 16-bit. | Primary Latin-1/UTF-16, shared subjects and explicit host conversion; bounded ropes optional, not an M0 prerequisite. |
| Exceptions | `lib.rs:133` RuntimeError contains optional boxed thrown Value and String message; vm_result/vm_try own structured completion and cleanup. | JS_EXCEPTION sentinel + runtime current_exception; `quickjs.c:20321` common exception path unwinds operand stack/catch offsets, closes iterators, builds backtrace when needed. | Root thrown values, cold structured error metadata and shared unwind; preserve finally/disposal/re-entry. No evidence that exception overhead dominates normal development execution. |
| Native ABI / suspension | `native.rs` family dispatch cache and &mut CallEnv; `vm_generator.rs` snapshots owned vectors/env then rebuilds Vm. | `quickjs.h:446` context/this/argc/argv function ABI; `quickjs.c:17550` resumes saved generator frame PC/sp/buffers. | Explicit Context/ArgRange/root scopes; heap suspension records retain cell/window identity and existing JS job semantics. No FFI or Rust async. |

## Architectural risks carried forward

- T021 moved whole frames / added window indirections and regressed controls;
  reusable storage must remove entry work and use fieldwise initialization.
- T024's dispatch-only thesis was not the combined value/GC/property rewrite.
  T023's RC arena rejection is not evidence that tracing is already priced.
- T056/T064 object-layout edits changed typed/wide generated instructions; K1 and
  Repair 1 still blocked 46/86 cases, and outlining was abandoned. A new crate or
  new module alone will not establish machine-code isolation. Port accelerators
  deliberately and enforce complete final lane/portfolio ceilings.
- Eight-byte packed pointers require checked ABI/provenance/NaN handling; root
  omissions and references across allocation/re-entry are the primary safety risk.
- Roughly 189k runtime lines expose legacy Value/object internals. A universal
  graph adapter preserves that cost and complicates cross-heap identity/cycles.
  The recommended isolated-backend strangler pays the port cost explicitly.

The architecture and migration documents contain decisions and invariants;
this record alone owns the dated counts, estimates, uncertainties and source
receipts. New implementation requires lead review and a supported frozen plan.
