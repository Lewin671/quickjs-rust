# T038: Tofte operation prices at af4d0d3c

Brief 06 pricing/selection record, 2026-10-12. No production unit was frozen,
implemented or screened. [T038](../T038-tofte-operation-pricing.md) owns next action.

## Identity and method

Branch base `8e208b8a8f19764e99f2a003d6114fbca9d163db` has exactly the runtime
of queue candidate `af4d0d3c81aad98721661689cf7224d2c17b9781`.
Queue SHA-256 `2882db837308dca8db304e3d0de0b32efcb4326332fe218393020c2f9e275f26`;
source standing: T018/T037. This is diagnostic pricing, not a new formal run.

| Asset | SHA-256 |
| --- | --- |
| Exact Rust executable | `6878520ff5fc8fc480e9fd8086c2b5caeb4ba609236ea423cd5a6d573a6fc54b` |
| NG executable, pin f7830186 | `cfd8386c3c29b1125a878b8fb82f9627820f2dcc16d2a691c5f8c16ad0b047a0` |
| Hash-verified neutral-shell bundle | `fabd80e6d4c685f5723de063ba77f7d8e0bca4af6bdecd30dd87611b7f758032` |
| Exact Rust amplified sample receipt | `888b37ac6a4acfda919c10b05a4b272c50dff00551578c98211f9ad8088963d9` |

Scratch: `../wt-quickjs-rust-tofte-pricing-codex`, branch
`agent/tofte-pricing/codex`, base 8e208b8a. Raw `target/tofte-pricing/`
artifacts are local, not retained. The prices/counts/conclusions below and
[reproducer](../measurements/tofte_measure.py) are the durable evidence.
The main checkout and excluded owner files were never edited.

Each cell has five warmed, alternating Rust/NG pairs, equal-length engine
paths `r/qjs` and `n/qjs`, `/usr/bin/time -l` hardware counters. Quiet checks
before and after pairs found no remaining competing compiler/engine process;
no timing warning was accepted. Every benchmark, counter run and sampler was
wrapped in `lockf /tmp/qjs-lead/host-timing.lock`. Counter builds never timed.

C=4/8 nonempty calls per outer iteration; K=4/8 nonempty steps in each
callee (or 4/8 array elements for scan kernels). For median T(C,K),
`D4=(T(8,4)-T(4,4))/(4*N)`, `D8=(T(8,8)-T(4,8))/(4*N)`,
`B=(D8-D4)/4`, `A=2*D4-D8`. N=30,000 for ordinary components, 2,000 for
closure/eval components, 300 for original formatDate bodies; matched kernels
use 1,000. No empty-body subtraction. A/B are effective source increments,
including transport/accumulation, **not exclusive native-symbol prices**.

All 176 initial generated forms match measured bytes. Final generation
verified all 188 measured forms, including getYear/strict/native search. C/K admission was checked with diagnostic counters/traces.
The scalar-only kernel folds; it is NOT subtracted from wide kernels.
Boolean-to-number `string-eq` and `string-concat` with substring are composite
probes, not prices for a bare string comparison/concatenation. Source-character
prices below replay the original function, preserving its generic/wide mix.

## Whole-case gap and disjoint source-character ledger

| Metric | Rust | NG | Rust minus NG | Rust / NG |
| --- | ---: | ---: | ---: | ---: |
| Cycles | 198,246,384 | 138,182,396 | 60,063,988 | 1.434671 |
| Instructions | 877,625,530 | 696,636,154 | 180,989,376 | 1.259803 |
| Wall ns (beside counters) | 68,106,083 | 48,048,041 | 20,058,042 | 1.417458 |

These medians are separate diagnostics; formal standing remains 1.422100
wall / 1.435574 cycles. Parity needs 30.30% of Rust cycles removed.

Use literal `-` A for the common activation envelope; B for each actual
format character. Per-case counts come from source event counters, identical
on both engines. Shares below divide signed Rust-minus-NG contributions by
the whole-case gap. They are **not expected savings**.

| Family / nonempty unit | Count | Cycles R / NG | Instructions R / NG | Cycle gap share | Instruction gap share |
| --- | ---: | ---: | ---: | ---: | ---: |
| Fixed formatDate activation (A) | 1,000 | 62,430.54 / 30,305.56 | 289,521.55 / 186,201.37 | 53.48% | 57.09% |
| Y | 1,000 | 17,838.06 / 14,463.15 | 89,289.32 / 78,818.43 | 5.62% | 5.79% |
| m | 500 | 13,591.78 / 12,816.13 | 65,425.93 / 69,656.43 | 0.65% | -1.17% |
| d | 1,000 | 12,860.17 / 12,945.15 | 59,935.54 / 68,801.20 | -0.14% | -4.90% |
| l | 500 | 12,389.60 / 11,986.28 | 62,323.01 / 64,805.16 | 0.34% | -0.69% |
| F | 500 | 11,419.98 / 11,284.19 | 57,072.91 / 61,475.09 | 0.11% | -1.22% |
| g | 500 | 12,233.59 / 12,358.51 | 58,057.68 / 65,421.31 | -0.10% | -2.03% |
| i | 500 | 13,370.43 / 12,946.53 | 64,491.27 / 70,785.86 | 0.35% | -1.74% |
| s | 500 | 13,418.85 / 13,884.14 | 71,454.94 / 75,195.17 | -0.39% | -1.03% |
| A | 500 | 11,225.14 / 10,325.23 | 53,817.43 / 59,337.40 | 0.75% | -1.52% |
| Literal character (B) | 5,500 | 5,676.42 / 3,894.19 | 29,212.36 / 15,527.18 | 16.32% | 41.59% |

| Residual | Rust | NG | Unexplained gap | Gap share |
| --- | ---: | ---: | ---: | ---: |
| Cycles | 30,072,638.17 | 16,250,009.65 | 13,822,628.52 | 23.0132% |
| Instructions | 61,889,580.83 | 44,077,423.27 | 17,812,157.56 | 9.8415% |

The model totals 168,173,745.83 / 121,932,386.35 cycles and
815,735,949.17 / 652,558,730.73 instructions. Residual includes startup,
outer date advance, real date/branch/result-length mix, and transfer failure
of additive prices. Do not assign it to an invented optimization. A from
different format characters ranges 62.4-72.4k Rust cycles: cycle intercepts
are not a uniquely closed partition. Fixed-A instructions are much steadier. Separately priced setTime plus
getTime outer advance is only 0.24% of cycle gap; it does not close residual.

## Operation-family counters and component prices

Counters on the unmodified bundle: 28,002 NewFunction events (28,000 internal
plus two outer functions), 6,001 NewArray events (6,000 formatter literals
plus one startup event), 2,000 fast String-wrapper constructions, 1,001 Date
constructions, 6,371 primitive concatenations of which 871 have a Number
operand. Of the concatenations, 5,500 build eval sources; 871 are zero prefixes.
Source return-type counters independently give m=405, d=298, i=85, s=83
zero prefixes, identical on NG. Wrapper lengths are read 2,000 times.

| Event | Execution count Rust / NG |
| --- | ---: |
| formatDate (general ordinary frame) | 1,000 / 1,000 |
| arrayExists (wide compact after initial probe) | 11,000 / 11,000 |
| formatter calls (wide compact) | 5,500 / 5,500 |
| Array membership comparisons | 222,000 / 222,000 |
| Character body iterations | 11,000 / 11,000 |
| Date getDate / getMonth / getHours | 2,000 / 1,500 / 1,500 on each |
| Date getMinutes / getSeconds / getDay / getTime | 1,000 / 1,000 / 500 / 500 on each |
| Date getFullYear | 2,000 / 1,000 |
| Date getYear | 0 / 1,000 |
| Date setTime; split; join | 500; 1,000; 1,000 on each |
| Direct eval; cache hits / initial distinct-source misses | 5,500; 5,491 / 9 on Rust |
| JS switch; function-array calls; Function constructor | 0 / 0 |

NG uses the host local time zone; Rust Date uses UTC. Y's validation Date is
UTC midnight 2001-01-01, which is local year 2000 on this host. Thus NG takes
getYear fallback; Rust trusts getFullYear. Original-character pricing above
includes this difference. These are measured paths, not identical calendar
internals or a Date correctness/parity claim.

The stock counter reports 17,500 ordinary attempts, 16,500 direct-leaf frames,
1,000 generic call frames, 16,500 compact standalone activations, no compact
inline/caller-env calls and no closed-form calls. Direct-leaf plus standalone
counts overlap; they must not be added. 6,502 nested VMs include 5,500 eval
frames, 1,000 general formatters and startup/initial continuation. 805,065
interpreter ops coexist with 2,322,563 compact ops. All 11,501 counted ordinary
backedges decline accelerators; compact loop iterations are not ordinary edges.
Stock property counters (79,000 computed reads, 8,002 named reads) likewise
omit some wide fast paths; the 222,000 comparison count is supplemental source
instrumentation, not a relabeling of the stock property counter. Native_calls
8,502 omits fast Date methods/String construction; use the explicit events.

Selected-eval preparation performs 549,100 local/upvalue slot visits over
5,491 cache hits (50 slots twice). Overlay instrumentation reports 1,400,450
visits; it also counted the persistence enumeration, so only 1,400,000
(28,000 closures times 50) are attributed to closure overlay enumeration.

Prices below are joined component probes; they are nested inside the first
ledger and **must not be added to it**. A source-family instruction/cycle
projection can be computed as `(count_R*price_R-count_NG*price_NG)/gap`, but
these joined probes include the named extra operation, and differ in types,
loop guards and body lowering. They are reach/pricing evidence, not a second
exclusive partition. The quantified search/closure projections follow below.

| Nonempty component B | Cycles R / NG | Instructions R / NG | Included work / reach |
| --- | ---: | ---: | --- |
| Small ordinary closure | 742.23 / 724.71 | 4,044.38 / 4,892.52 | construction/capture + scalar step |
| 44-local ordinary closure | 711.19 / 741.75 | 3,946.50 / 4,896.77 | construction/capture + scalar step |
| 44-local direct-eval closure | 1,462.43 / 857.98 | 7,906.84 / 5,691.72 | construction/scope/overlay + scalar step |
| String-array scan step | 162.20 / 109.74 | 845.25 / 466.19 | array read + string compare + induction/branch |
| Existing typed strict-string scan | 84.50 / 103.48 | 445.41 / 443.90 | same scan body with ===; typed counters prove it runs |
| Existing native string search | 20.35 / 21.62 | 72.09 / 84.00 | borrowed indexOf scan; different activation/guard contract |
| arrayExists invocation, target at index 14 | 2,574.67 / 1,758.04 | 12,355.00 / 7,012.05 | 15 comparisons + call + return |
| Date getHours | 186.96 / 608.88 | 924.09 / 2,707.05 | method resolution + Date value check + components + accumulation |
| Date getDate | 212.48 / 603.32 | 997.93 / 2,670.30 | same |
| Date getMonth | 223.61 / 607.96 | 997.75 / 2,711.46 | same |
| Date getFullYear | 218.18 / 601.17 | 998.58 / 2,701.46 | same; environment-specific local calendar |
| Date getYear | 221.12 / 626.24 | 1,000.60 / 2,716.72 | Date value read + local calendar + year offset |
| Date getDay | 170.36 / 579.48 | 924.58 / 2,711.93 | same |
| Date getMinutes | 222.53 / 574.41 | 1,001.19 / 2,697.40 | same |
| Date getSeconds | 245.69 / 581.69 | 1,034.14 / 2,717.02 | same |
| Date getTime | 168.76 / 84.60 | 902.68 / 317.02 | method resolution + internal Date value read + accumulation |
| Date setTime | 290.64 / 82.93 | 1,769.57 / 386.64 | native call, ToNumber/TimeClip/internal store + scalar step |
| Date string construction + validation getter | 3,144.74 / 1,221.19 | 16,602.05 / 6,351.06 | prototype/object allocation + ISO/formatted/legacy parse + getFullYear |
| new String(number).length | 479.30 / 604.25 | 2,984.68 / 3,947.76 | number formatting + lazy wrapper + length/read + accumulation |
| String wrapper length | 34.91 / 21.42 | 281.69 / 93.96 | property read + accumulation |
| "0" + number, length | 202.15 / 281.04 | 1,346.08 / 1,857.55 | number formatting + concat + length/read + accumulation |
| "a" + string, length | 200.83 / 171.96 | 1,305.99 / 1,065.14 | concat + length/read + accumulation |
| Escaping 7-element array | 377.13 / 485.86 | 2,579.87 / 3,005.91 | literal + property store/reload/length + accumulation |
| Captured wide call returning getTime | 297.57 / 124.82 | 1,441.66 / 547.44 | call boundary + getter + accumulation |
| Direct eval of v, small frame | 5,960.42 / 4,929.92 | 31,726.11 / 29,928.65 | cache, selected env, eval VM, lookup + accumulation |
| Direct eval of v, 44-local frame | 6,404.47 / 10,833.77 | 34,381.80 / 65,183.61 | same, different lexical topology |
| Eval g0() in 28-closure frame | 8,572.82 / 11,619.16 | 40,234.29 / 72,921.86 | eval VM + captured getTime call + accumulation |
| Direct g0() in same eval-capable frame | 478.73 / -13.81 | 2,832.84 / 461.20 | captured getTime call + accumulation |

Matched direct-call NG cycle slope is negative (-13.81); that falsifies an
exclusive additive cycle interpretation for that small increment. Do not use
matched eval-minus-direct as a closed eval cycle budget. Its instruction
prices remain positive. The simpler eval-call probes return a closed-form
captured value, unlike tofte's wide formatter, so they are not transferred.

Date component getters call date_value (one internal numeric-property read),
then utc_date_time/civil_from_days; getTime performs only the value read.
Effective Rust component-minus-getTime increments are roughly 2-77 cycles;
NG local getters include local-time conversion and are hundreds of cycles
more expensive. Date internals cannot explain the main positive gap.
Date method reads and the additional 1,000 Y getFullYear presence checks are
inside the joined method/Y prices; no exclusive property-lookup price is claimed.
Array/string result transport and Number conversion are likewise retained work.

## Admission and terminal blockers

- **Outer loop:** global code stays generic. The typed builder ultimately
  declines at DefineGlobalVar(shortFormat), ip 58; formatting calls/eval also
  prevent a pure numeric sink region. Batch-1 global stores do not admit it.
- **formatDate:** contains direct eval and creates capturing closures; it
  cannot take direct-leaf/whole-body wide admission. The character loop
  280..342 ultimately fails helper Call(2) at ip 307 (arrayExists is not a
  closed-form helper); CallDirectEval is also outside the tier. Generic
  frame/dynamic binding authority is the terminal boundary, not switch cost.
- **arrayExists:** the wide body is admitted. A typed program also compiles
  (7..33, seven scalar registers / one boxed register), but DenseRead and Eq
  remain scalar. x is a String; scalar-register seeding rejects it before
  executing. QJS_TL_TRACE=4 reports TLRUN Declined at both initial probes.
  Existing BoxedEquality supports strict string equality; loose Eq/Ne only
  compare object identities. A boxed literal-string probe reaches Eq and
  deoptimizes there. Both operand-class inference/entry and loose string
  equality admission must change; merely teaching seeding about String fails. The special
  numeric predicate scan also does not admit the property-derived length
  header (it requires local/local bounds). The loop continues wide.
- **Eval bodies:** nine cached global/eval bytecodes, typically length seven,
  perform dynamic LoadGlobal(formatter) + Call; they stay generic. Cache hits
  avoid parsing/full writeback but still fork/project context and create an
  eval VM. Each called formatter bypasses unchanged dynamic scope and enters
  standalone wide; named methods/new/String and received cells prevent scalar
  closed-form evaluation. No formatter creates another ordinary closure.

The terminal broad blocker is a dynamic activation with live shared cells,
not a shortage of narrow native Date fast paths. Admitting all of formatDate
requires a staged activation/binding protocol and exact eval/closure exits.

## Prior art, reach and step-2 decision

Fresh exact-Rust sample: 3,448 main-thread samples. Inclusive / flat:
overlay 9.02% / 5.34%, scope checks 2.49% / 2.29%, captures 2.44% / 0.90%,
selected eval projection 5.48% / 2.20%, frame construction 7.31% / 1.02%,
upvalue resolution 2.18% / 0.99%, wide execution 23.38% / 11.05%.
Inclusive groups overlap; there is no exclusive instruction profile.

| Candidate boundary | Reach priced against whole Rust case | Decision |
| --- | --- | --- |
| Activation/binding representation across direct eval | Fixed-A excess 32.12M cycles = 16.20% case, 53.48% gap | Largest packet, but frame/cell/closure/array phases are different lifetimes; staged migration, not one leaf |
| Static overlay topology proof | Inclusive reach 9.02%; ordinary/eval closure increment difference is nonexclusive | A guarded proof could remove enumeration; clearly reachable standalone saving below 10% |
| Typed string equality admission | Existing typed strict route B=84.50 vs current 162.20; step saving 8.70% case, 7.51% including joined A | Operand boxing plus non-coercing String/String loose equality is a leaf, but even this admission analogue misses 10% |
| Borrowed primitive-string predicate execution | Native 20.35 vs wide 162.20 gives ideal 31.49M cycles = 15.88% case / 52.43% gap | Structural successor: general effect/data-flow lowering and borrow/entry/replay contract; native builtin price is not a sound leaf savings proof |
| Selected-eval slot projection | 5.48% inclusive reach; 549,100 slot visits | Invert selection to indexed slots with preserved ordering/dynamic shadowing; below 10% independently |
| Cached capture/scope metadata | 2.44% / 2.49% inclusive reach; must retain cells and live name guards | Below 10%; cannot sum them blindly with overlay/frame fractions |
| Static upvalue/frame-entry plan | 2.18% resolution, 7.31% inclusive frame construction | Immutable plan caching reaches only part of those groups; below 10% |
| Date parsing/construction or String wrapper | 1,001 / 2,000 events; String probe already cheaper than NG | Small; no >=10% case packet |

The 28,000-closure joined-price excess is 16.92M cycles (8.54% case,
28.18% gap) and 62.02M instructions (34.27% instruction gap). It is not a
hard theoretical upper bound or entirely removable metadata work. Search
wide-vs-NG numbers likewise price excess, not the entire 36.01M-cycle scan path.
A sharper nonempty bound uses existing strict-string typed execution (TLRUN
Ran / CFLOOP; loose-string boxed execution reports TLDEOPT at Eq). Its A is
498.64 vs the current scan's 284.15 cycles, B 84.50 vs 162.20. At 222,000
comparisons/11,000 calls, saving is 17.25M cycles before entry, 14.89M after
entry = **7.51% whole case / 24.79% gap**. Instructions save 74.81M = 41.33%
gap after entry. This is an admission analogue, not a runtime candidate:
original variable x still requires boxing and an extra live register.

The borrowed native indexOf analogue is a useful larger ceiling, not an
admissible replacement of this loop. Its strict comparison is equivalent only
for actual primitive String/String pairs; loose coercions, holes, inherited
indices, getters, aliasing and local-cell observers still require guarded
entry and exact continuation. Existing numeric predicate admission requires
local/local bounds and numeric predicates. Reusing it by recognizing this
specific header/tail would violate the brief's no-template rule. A general
successor needs operation/effect/data-flow lowering plus the borrow/activation
and replay protocol, with stages and negative cases before a payoff gate.
The wide activation bridge is also outside this brief's committed boundary.
Do not freeze the 15.88% ideal ceiling as a leaf promise.

Prior art was searched across tasks/, including benchmark/shape/template
terms. The July incremental overlay (T021 log, 60e28ecf) retained active-cell
snapshots/revision checks and achieved only 0.980412 against its <=0.90 gate;
September's prefix-difference overlay (T033 screen log) was 1.019. Both left
per-closure enumeration/bookkeeping. A successor must eliminate enumeration
through a proven topology invariant or compile-time indexed metadata, not
skip more prefix hashes or rename the same snapshot cache. Such a successor
was not implemented/frozen here. T033 already names eval environment creation
and closure/frame cost; T036's four-stage mixed-call draft remains unapproved.

No candidate may inspect format names, switch-token strings, the 28/50 counts,
source paths, literal tables, iteration counts, or benchmark bytecode layouts.
Replacing eval("X()") with a dispatch table, interning this Date input, sharing
these particular arrays or recognizing arrayExists's source is inadmissible.
Future admission must follow operations, operand types, cell identity and
observable data flow, with ordinary guard failures/side effects preserved.

Step 3 was not entered; attempts used **0/2**, no plan/candidate/placement
screen and no order-file change. If a later control's cycles alone regress,
report nm -n addresses/sizes of wide/VM executors, call_leaf and cache probe;
do not consume an attempt or re-pin to hide it. Required future controls remain
all six sentinels, xparb, tagcloud, unpack-code, ai-astar and changed-path peers.

## Reproduction and verification

Historical actual commands (all from pricing scratch, with shared lock):
`env PYTHONPATH=. PYTHONDONTWRITEBYTECODE=1 python3 target/tofte-pricing/measure.py`
(component); corresponding `measure-extra.py`, `measure-format.py`,
`measure-matched.py`, `measure-legacy.py`, `measure-search.py`, `measure-strict.py`;
`collect.py`, `final-counts.py`, QJS_TL_TRACE=4 original/scan/strict traces;
`profile-run.py` ran sample <pid> 5 1 -mayDie -file .../tofte.sample.
Each command above was prefixed by `lockf /tmp/qjs-lead/host-timing.lock`.

Portable equivalent, after supplying the exact cached executables/bundle:

```sh
python3 tasks/measurements/tofte_measure.py generate --bundle <bundle>
lockf /tmp/qjs-lead/host-timing.lock env PYTHONPATH=. PYTHONDONTWRITEBYTECODE=1 \
  python3 tasks/measurements/tofte_measure.py price --bundle <bundle> \
  --rust <af4d0d3c-qjs> --ng <NG-qjs> --group all --output-dir target/tofte-replay
```

For augmented events, in a **linked scratch worktree**, run
`tofte_diagnostics.py install --root <scratch> --backups <fresh-target-dir>`,
then locked `cargo build --release -p qjs-cli --features perf-counters` there.
Run the measure helper's `counter` mode with --counter <diagnostic-qjs>;
`profile` mode samples only the hash-checked production binary. Restore with
`tofte_diagnostics.py restore --root <scratch> --backups <same-dir>` before
any commit. Restore refuses intervening edits. No excluded owner file is touched.

Validation: diagnostic install/restore round trip passed; git diff --exit-code
-- crates clean; original stock counters and stdout all identical to augmented
run. Source event counts agree on both engines apart from the documented year
branch. Python compile/help/generation passed. Full gate passed at runtime base 8e208b8a (runtime af4d0d3c), with pricing
documents/scripts present; final helper extensions then passed compile/help,
188-input byte equality, file-size/context and staged gates:
2246 runtime tests, 23 perf-counter tests, 21 agents tests, 5170 Test262
subset cases, 375 benchmark-tool tests and 35 script tests; fmt/clippy/context,
all plan validations, hook isolation and file-size gates passed. Staged gate
passed (documentation/measurement paths select context only; no focused
Test262 slice). Remote CI is recorded after commit in the handoff; none is new runtime
promotion evidence.
No runtime before/after, promotion/burndown, formal 30-block comparison,
performance control screen or fixed-hardware qualification was performed.

Initial harness setup errors were `qjs: unknown option '--raw'` and
`ReferenceError: i is not defined` (NG auto-selected module mode). Both were
resolved by NG `--script`, matching external_preview.py. They produced no
retained sample cells. An intermediate context check reported a not-yet-written
archive link; final context checks pass. No correctness gate failed.

Out-of-scope stale document: T033 still says d1313d52 is the only formal
standing since loop-template removal; T018/T037 now record af4d0d3c. T033's
historical unit rejection is unaffected. This record does not edit its owner.
