# T056 stage A attempt 1: diagnosis and repair contract

The valid rejection and attempt ledger live in the [stage A record](T056-m1-stageA-record.md#attempt-1-of-2-valid-k6-rejection).
Frozen unit `d184dbcc` is unchanged; one attempt remains across A+B. No timing jobs may be submitted by
the implementation owner. Any new timing or quick triage is submitted by the lead. No acceptance or wall
speedup is established by the static analysis or instruction diagnostics below.

## Evidence at `0a9af81b`

The six layout wall medians all exceed 1.025 for each of the five rejected cases; the rejection is not
dismissed as noise. The two scalar workloads in `benchmarks/workloads/broad-micro.js` read local numbers
and branch on integer parity; their hot loops touch no object. The fresh CPU profiles
(`20261020T220825Z-t056-m1a-reject-cpu-profiles`, non-timing, done 0) identify the executing bodies:

- local_read: base 952/952 main-thread samples in typed `run<WideLoopFrame>`; candidate 1025/1026
  in typed `execute<WideLoopFrame>`. branch_arithmetic: 1569/1570 versus 1668/1669 in those same bodies.
- property_dynamic_read: wide activation, computed property loads and ordinary own-data lookup, not the
  typed scalar executor. Candidate `Shape::lookup`'s separately emitted iterator helper has 486 self
  samples out of 6691 main-thread samples. It is absent from the base's corresponding top list.
- Tree/unpack profiles also expose that iterator helper (29/1159 and 39/3280 candidate self samples),
  and tree property creation/set paths. These are sampling envelopes, not instruction attribution.

Canonical binary SHA-256: base `b0dfc65d0c026d62aa64ef3d69a089eb73b51088c84af75b44617ea2a1737f0e`,
candidate `1970bb690fd9445e94036df51eb29ff7580c10ef16f0f298d0b0cee2112b7491`. They were copied directly
from the revision caches and match the rejected screen's natural images. `layout_audit` and Mach-O
disassembly were run on these exact copies, not on a new Cargo build.

| Body | Base bytes / page offset | Candidate bytes / page offset |
| --- | --- | --- |
| wide activation `run` | 25264 / 0xc30 | 23504 / 0x070 |
| interpreter typed dispatch (`run<Vm>` closure) | 13792 / 0x110 | 13952 / 0xe50 |
| wide typed dispatch | `run` closure: 13696 / 0xd90 | `execute`: 14896 / 0x630 |

The wide typed body's stack reservation changes 0x178 to 0x268. This is static frame size, not a count
of executed spills. The actual `execute<WideLoopFrame>` lives at 0x10046e630, versus the old dispatch at
0x10025bd90. The production build uses a linker order and `__text` page alignment, but has no active
fixed-slot pin header. Its order was generated at `9a0eeeca`. Against base it has 220 stale entries
(120 engine); against candidate 315 stale (201 engine). The old wide dispatch spelling no longer exists
and its replacement is unlisted. Thus the intended pin is concretely lost; this does not by itself
prove placement accounts for every wall regression.

## Repair boundary, fixed before any new diagnostic timing

1. Keep numeric shared-slot read/write guards out of typed dispatch, whose scalar arms do not use them.
   Preserve every existing receiver/encoding/descriptor guard and cache coverage. Do not add a recognizer
   for any workload or a new final-shape/add-transition replay.
2. Keep the bounded small-layout scan in a direct loop rather than an outlined Iterator::position
   instantiation; inline the existing append-edge hit into its caller. Verify real instruction counts
   on tree/unpack, aiming at <=1.01 against the original base, not fewer source lines or a smaller binary.
3. Once runtime boundaries settle, regenerate the native x86 order from the actual emitted symbols.
   Select the actual dispatch body by canonical Rust path, including an independently outlined execute
   when that is what the compiler emits. Preserve reference executor residues from f58c696e, not historical
   ARM constants or a favorable timing scan; audit stale entries, slots and linked offsets after relinking.
   This uses the standing maintenance workflow; it does not adopt T054's rejected global layout regions.
4. Declare all diagnostic instruction cases before building: the five rejected cases plus moment/dates,
   diff/compare and meriyah/parse. Raw job/patch/image identities and every result are retained. These
   non-timing counters cannot accept a migration stage or consume/reset a timing attempt.
5. The lead then runs one prospective T061 diagnostic quick triage retaining its fixed controls and all
   eight requested focus cases, and a fresh complete 86-case pre-gate on the final committed image.
   Correctness must be fresh for that image before any last-attempt screen. Triage cannot promote or
   cancel the recorded rejection, and natural-layout improvement does not waive a K=6 screen.

The standing codegen rules are [performance knowledge](../../docs/performance-knowledge.md#codegen);
repinning procedure is [performance workflow](../../docs/performance-workflow.md#re-pinning-the-code-layout).
T037, T050 and T054 were read before this repair; no historical favorable offset or rejected padding
strategy is taken as permission to tune a benchmark's shape or select a passing layout.


## Invalid temporary-build diagnostics (not acceptance attempts)

`20261020T221146Z-t056-m1a-boundary-instructions`, done 0, source `e87fa8c5` plus patch SHA-256
`93be399246ea0d258f44cef56be050a824ccc949d347d0720731a7e74205a995`. This and the subsequent storage
job **omitted the explicit CARGO_BUILD_TARGET** of the canonical producer: they used matching profile/flags
but changed the crate disambiguator, leaving 3641 order entries stale. All following numbers in this
section are retained as invalid build diagnostics, not canonical ratios or evidence of the repair effect.
Diagnostic image `dfc7beaeb81eb1b8077cf3b8c6cffca9823f89089978754500815a17bf31f2d5`.
One instruction pair on the eight predeclared cases: tree **0.98642348**, unpack **1.00908158**,
property_dynamic_read **0.92519879**, targets moment/diff/meriyah **0.96646024 / 0.96649988 / 0.96600278**.
However scalar local_read **1.09149004** and branch_arithmetic **1.07845291** regress in instructions, so
this image is **not eligible for a screen**. No wall result is adopted. The general small-layout repair
removes real work, but outlining guards alone does not stabilize dispatch: typed bodies now grow to
17,344 / 17,568 bytes.

The next structural boundary follows the standing rule to pass an executor its storage rather than its
owner: entry keeps allocation/seeding/recycling, while a non-inlined dispatch receives the scalar bank,
receiver slice, boxed slice and prepared write targets as disjoint field borrows. Its generic operation
set, guard/deopt/writeback and admission stay unchanged. Measure the same eight instruction cases, then
re-pin the final emitted dispatcher; no timing-based offset search or scalar-only special executor.

The second invalid temporary job `20261020T221618Z-t056-m1a-storage-instructions` (done 0, patch
`e0fabeb6f889812b60d5ecbef7bd11ca4ced04dcec4c5e683b7d1c6aba25235a`, image
`f68eeab660f4cc92bca2014f7eeb45ed11fe582a99fdbebc4839b8c3af6d60e5`) reported local/branch
1.01537739/1.01303681, dynamic 0.90571642, tree/unpack 0.98657758/1.00711728, targets
0.96373432/0.96727080/0.96344677. Those ratios cannot justify either storage or layout changes.
The replacement diagnostic uses the repository's build_cache build-rust producer verbatim and must first
reproduce the original 0a9af81b SHA-256 before building the repaired snapshot. The original rejection,
canonical symbol audit and CPU profiles remain valid; they used the known hash-matched cache images.


## Canonical repair and reference-derived pin

Replacement `20261020T222107Z-t056-m1a-canonical-storage-instructions` finished done 0 in 194 seconds.
Before its repaired build, `build_cache build-rust` reproduced the original canonical 0a9af81b image
**1970bb690fd9445e94036df51eb29ff7580c10ef16f0f298d0b0cee2112b7491** exactly. It sets
CARGO_BUILD_TARGET from rustc's host and the complete frozen profile/flags itself. Repaired snapshot:
source e87fa8c5 plus patch **72922f5eceee0ccd5b7c06803de7f83f5dff176de437f4f72b40576d9f32d27f**,
image **42c0984ad0b73f20c9b4f9323828bb702cc029702772ca327fa261ea4b732540**. Eight instruction pairs:
local 1.01533423, branch 1.01308084, dynamic 0.90558223, trees **0.98671078**, unpack **1.00759333**,
targets moment/diff/meriyah 0.96418189/0.96729784/0.96222644. No wall evidence is adopted.

The native x86 order is regenerated from every actual canonical text symbol (not a benchmark profile
ranking), then pinned with the existing exact std-function filler and slot machinery. Reference offsets
come only from the immutable f58c696e binary: interpreter **0x110**, wide typed dispatch **0xd90**, plus
all 15 uniquely matched existing callees (wide activation included). New numeric cached-slot helpers get
slots after existing roles. Slots initially use exact emitted extents: any growth requires re-pinning,
rather than assuming unused headroom. This avoids exhausting the concrete filler pool while retaining
fixed budgets; no new padding atoms, runtime branch, compiler flag or T054 layout-region mechanism is added.

Layout role selection now recognizes the direct execute declaration before legacy run/try_run wrappers;
it excludes nested deopt closures of execute from that declaration match. Regression tests prove it
selects each frame instantiation's actual body, retains old closure fallback and does not silently choose
an ambiguous body. Newly outlined numeric slot helpers and wide activation are explicit callee targets.

`20261020T222950Z-t056-m1a-pinned-instructions` finished done 0 in 116 seconds. Source e87fa8c5 plus
patch **411dbe755b654f4b2acac1cd716256dc6ce5db420697ded1e149fbcb32aeae67** identifies the repair runtime/tool diff at that diagnostic
(test documentation may subsequently change without changing engine inputs). Image **b2eaa2124bc9db15e84b05f0a78dd7f490ffb0405ca09bda1bdc734463dd830a**.
Static strict audit: **zero stale entries**, both executor residues and all **15 reference callee residues**
match. Typed dispatch sizes: interpreter **14320 bytes**, wide **14304 bytes**. The order SHA-256 is
**3c1dbffa85d345a7df324856b9b750bc5a3df85af26f0598b7383545d4fb5c58**.

| Case | Repaired canonical instructions / frozen base |
| --- | ---: |
| broad/local_read | 1.01523842 |
| broad/branch_arithmetic | 1.01309006 |
| broad/property_dynamic_read | 0.90544464 |
| external/sunspider-1.0/access-binary-trees | 0.98667947 |
| external/sunspider-1.0/string-unpack-code | 1.00697611 |
| development/moment/dates | 0.96441169 |
| development/diff/compare | 0.96745905 |
| development/meriyah/parse | 0.96312736 |

Trees and unpack meet the engineering <=1.01 instruction aim. Scalar local/branch now execute about
1.5%/1.3% more instructions than base (and more than rejected 0a9af81b); this is a material limit, not a
claimed wall fix. Loss of pinning and changed dispatch ownership/codegen are confirmed mechanisms, but
only the lead's prospective diagnostic can test whether the repaired code is fast enough. No result here
waives the recorded K=6 rejection, the stage budget, full conformance or the sole remaining attempt.

The local producer and static pin commands are retained in workspace `tmp/t056-m1a-reject/`; each job
also retains its exact patch, recipe, image identities, raw instruction records and audits under
`outputs/imac-bench/<job>/out/`. The stable conclusions above do not depend on opening those local files.

Local full gate passed for the repair runtime: 2345 runtime tests, agents 21, diagnostics 29,
663 benchmark-tool tests, 35 script tests, all context/size/format/clippy checks and the complete 5170-case
Test262 subset. All 255 QuickJS-NG comparison fixtures pass. Focused layout/typed/cache/counter tests pass
137 cases; explicit layout pin/symbol tests pass, including direct dispatch versus deopt closure selection.
No dependency, unsafe block, environment snapshot, new opcode or new tier admission was introduced.
The full configured exact scan at 0a9af81b does not establish conformance for the repair image; the lead's
fresh correctness job must complete before the final attempt.


## Committed repair and proposed lead verification

Repair commit **`94408090e3fdea93cb1904bda445b14192112bb0`** passed the staged fast gate, including
2345 runtime tests, 663 benchmark-tool tests and the 65 selected Test262 eval/function cases. Manual full
gate and NG fixture results above precede the commit; final edits after that full run were task/runbook
and test documentation. The pre-push duplicate gate was skipped after those completed local gates.

Non-timing identity job **`20261020T224241Z-t056-m1a-repair-committed-image`**, done 0 in 72 seconds,
rebuilt the exact commit through qjs_build_revision. Its SHA-256 is **b2eaa2124bc9db15e84b05f0a78dd7f490ffb0405ca09bda1bdc734463dd830a**,
identical to the canonical instruction diagnostic; strict pin audit passes again. Thus the eight final
instruction ratios describe the committed image, not a temporary source variant.

Candidate isolation: `50b42547` remains an ancestor and no merge was introduced.
`git diff f58c696e 94408090 -- crates` is the same 122 M1 runtime files plus the explicitly authorized
native x86 CLI order file (123 files total). No main runtime, diagnosis-branch replay, AST/parser or
compiler-recipe change was imported. Frozen plan d184dbcc remains unchanged and exactly one attempt remains.

The prepared, unsubmitted commands are in workspace **`tmp/t056-m1a-last-attempt-verify/`**:

1. New full correctness receipt: full gate/255 NG/5170 subset and eight disjoint complete exact shards,
   canonical candidate/base hash checks, no timing. The old 0a9af81b exact receipt is not reused.
2. Fresh complete 86-case instruction pre-gate, one pair, fixed max(1.03,I_i), no timing, dependent on
   the new correctness receipt and its exact image. Record the complete entry through the standard ledger.
3. One T061 diagnostic triage, submitted **only by the lead with --timing**, five pairs and workers 4;
   all 19 fixed controls remain and the eight requested diagnostic focus cases yield 23 unique cases.
   The existing --target flags label diagnostic focus only and never edit the migration's three targets.

All commands freeze candidate=94408090, require prerequisite done0/no-DISTURBED and matching helper/unit/
image bindings, and preserve the lead's ZoneInfo-based iMac window guard. No prepared verification job or
new timing job was submitted by the implementation owner. Local static/render/window/guard/inventory
validation passes; old acceptance scripts are disabled after the valid rejection.

T061 uses its existing v5 exploratory heuristic, not the v6 stage gate. Even a likely win cannot advance
stage A. The lead must inspect every result, especially the five previous failures; if a focus case's wall
CI upper endpoint still exceeds 1.025, deferring the last screen is the conservative diagnostic choice,
not a change of the frozen acceptance rule. No favorable automatic retry is proposed. Only after fresh
correctness/pre-gate and manual review may the lead consider one complete K=6 attempt; all ordinary
formal/stage requirements remain if that screen passes. Repair wall time, full configured Test262 and
the complete 86-case pre-gate are **not yet verified** by this handoff.
