# T058: Rejected v6 unit; diagnostic-only instruction regression work

- Status: active; diagnostic follow-up complete; v6 unit remains rejected, no re-decision/reset; fix `736200aa` passes correctness, quick triage unclear.
- Implementation: rejected T058 changes present; diagnostic fix makes the numeric helper's existing 0-4 argument copies explicit. No optimization retained.
- Verified at: fix `736200aa`, full/fast gates and selected semantic slice passed; x86 exact-image, counter/trace and lineage diagnostics below (2026-10-19 UTC).
- Evidence: [v6 rejection](archive/T058-v6-rejection.md), [attempt 1](archive/T058-attempt1-incomplete.md), [attempt 2](archive/T058-attempt2-inconclusive.md). New diagnostic jobs/results recorded below.
- Unresolved: four triage control wall intervals cross their bands; exact LLVM cost-model trigger within T057 is not isolated. No acceptance authorized.
- Next action: coordinator reviews the fixed candidate and diagnostic limits; keep the v6 rejection and do not start a new plan/acceptance without a new task.

## Current evidence and scope

The rejected source pair is base `a68a9f46` versus candidate `9e415ff4`.
Both contain T057; the old pair `351004f3`/`392ca717` did not show the
recursive instruction excess. This comparison does not imply T057's call
semantics caused it. New non-timing diagnostics run on the iMac only; local
M1 instruction counts would be host-incomparable and are not used as x86
instruction evidence. Wall durations from non-timing jobs are not results.

The user authorizes diagnosis and a general fix, full `check.sh` and quick
triage against `a68a9f46` including recursive_call_tree, then a report. No
new plan, formal acceptance, re-decision or budget reset. Avoid timing jobs
in the iMac's 03:00-05:00 local window; initial clock was 06:54 CST (UTC+8).

Exact-image diagnostic `20261019T225612Z-t058-exact-call-instruction-diagnose`
uses the immutable r2 natural binaries (hashes in the rejection archive).
N=65536/131072, three retained increments per role: base
363.3551/363.3962/363.3429 retired instructions per declared call; candidate
381.3429/381.3499/381.3529, median ratio 1.049524. Thus about 18 extra
instructions per physical numeric helper call, independent of startup.
Whole darkroom instruction medians: 15,524,633,752 versus 15,699,241,916,
ratio 1.011247. These are diagnostic counts, not new acceptance samples.

Exact profiles show recursive_call_tree runs in
`typed_loop::helper_graph::numeric::NumProgram::run`, with new memcpy /
platform memmove samples only in the candidate. The register argument
initialization differs in the audited assembly: base has up to four direct
scalar copies, candidate calls dynamic-length memcpy. Source of that helper
is unchanged between the two revisions. The older base/candidate both have
the direct-copy prologue. Feature counters and intermediate builds below confirm a code-generation
interaction, rather than a changed JavaScript call/tier/IC path. No claim is
made about the particular LLVM heuristic; source size alone is not established
as its trigger, and the new base also includes T056 M1, not only T057.

Diagnostic jobs are non-timing and leave every raw counter/profile artifact:

- `20261019T225612Z-t058-exact-call-instruction-diagnose`: exact counts,
  eight-second samples and function-body comparison; exit 0.
- `20261019T230009Z-t058-codegen-lineage-diagnose`: canonical intermediate
  builds, counter increments and disassembly; exit 0.


## Code-generation attribution and general fix

`20261019T230009Z-t058-codegen-lineage-diagnose` built exact canonical
intermediates: lazy-only `f92448f0` already emits the memcpy prologue and
381.3442 instructions/call (image `d0a6b44b...0ef70ea`, 9,008-byte helper).
Adding replace-shortcut removal (`7204b36a`) leaves it at 381.2922 and
9,008 bytes (image `7e46f526...699965c`). Thus the first T058 runtime change
is sufficient in the T057 build context; removing the literal shortcuts is
not needed to produce the regression. The unchanged helper source has direct
copies in old base/candidate and current base, dynamic memcpy only in the
new candidate and these intermediates. Source-level admission and call-cache
code are unchanged; a placement-only shift cannot add the observed memcpy.

`20261019T230053Z-t058-execution-counter-replay` confirms **every original
execution counter is equal** between both feature builds on both workloads.
Recursive at N=10,000: executed_ops 192, ordinary_call_attempts 4,
compact_function_ops 58, loop_plan_entries 1. These legacy counters do not
count calls inside a prepared numeric helper graph; supplemental per-arity
instrumentation measures those separately. Darkroom: executed_ops 428,158,
compact_function_ops 1,147, ordinary_call_attempts 22, loop_plan_entries 18,
declined_loop_plan_edges 18. No extra interpreter frame, property operation,
loop fallback or inline-cache update explains the instruction increase.
Exact samples put recursive execution in NumProgram::run (base 6,022/6,697,
candidate 5,587/6,668 top-of-stack samples), with candidate memcpy/platform
memmove scopes absent from the base's collapsed hot-stack list. Sample
fractions are not dynamic instruction counts or removable-time percentages.

The fix in `typed_loop/helper_graph/numeric.rs` makes the pre-existing
MAX_HELPER_ARITY=4 bound explicit at register initialization: fixed scalar
assignments for 0/1/2/3/4 arguments, and None for unsupported internal arity.
A compile-time assertion binds the cases to that admission limit. It avoids
a per-call dynamic-length library copy without depending on whole-crate
unrolling decisions; it recognizes no program or source shape. The register
file, NaN undefined encoding, helper dispatch and environment model stay.
The focused unit test checks argument order and exact negative-zero / NaN
payload / infinity bits at every supported arity, plus untouched undefined
locals; it passes. Existing recursive/numeric helper tests run in the full gate.

Failed diagnostic jobs are retained: `20261019T225638Z-t058-execution-path-diagnose`
built both feature binaries but the parser omitted indentation and incorrectly
reported absent counters. The replay above reads their actual output.
`20261019T230248Z-t058-helper-arity-counters` failed compilation because
count! is a statement macro used as an expression; corrected instrumented
job `20261019T230413Z-t058-helper-arity-counters-r2` is independent supplemental
evidence. These jobs are not timing screens and spend no acceptance attempt.
Local tests are on M1; no M1 hardware instruction count is compared to x86.

Verification before the new candidate commit: full
`PYTHONDONTWRITEBYTECODE=1 CARGO_INCREMENTAL=0 ./scripts/check.sh` passed
(format/clippy all features, workspace runtime 2,313, 659 benchmark-tool,
35 script tests, all plans/context/size/hook/syntax gates and all 5,170
curated Test262). Staged fast gate passed, including the selected 65-case
eval-code/block/function semantic slice and runtime crate tests. No new
dependency, public engine API or environment/binding change.
The independent perf-counter traces also match: four ICUPDATE lines with
identical keys/storage, four CFOK lines, one CFVM and the same typed-loop
region 9..29 (8 registers, 4 boxed); exact and feature images stay distinct.

## Per-arity and fixed-image evidence

Supplemental per-arity job `20261019T230413Z-t058-helper-arity-counters-r2`
exited 0; every original counter and every added count is identical between
roles. At recursive N=10,000, both enter the two-argument numeric helper
1,269,873 times = 127*(10,000-1): the first outer iteration precedes the
prepared loop. Darkroom enters one- and two-argument helpers 3,844,798 times
each, total 7,689,596. These instrumented binaries are used for event counts
only, never hardware-cost/timing conclusions.

`20261019T230513Z-t058-helper-arity-costs` used a top-level synthetic loop
whose nominal-call deltas did not reproduce the helper cost; it was not
path-qualified and is not used for attribution. The independent, wrapped
function probe `20261019T231054Z-t058-qualified-helper-arity-costs` verifies
16,129 prepared helper entries at N=128 for each arity, then measures exact
old base/candidate increments. Extra instructions/call: arity 1 = 22.0426,
2 = 18.0357, 3 = 19.0185, 4 = 17.0892. The one-/two-argument multiplicities
predict about 154.1 million extra instructions in darkroom, most of the
174.6 million whole-process median difference. The remaining ~20.5 million
(0.13% of the whole process) is not uniquely apportioned; those separate
cohorts also vary in compilation/startup/other generated code. No claim that
sampling or this estimate explains every retired instruction is made.

Fixed-image diagnostic `20261019T230938Z-t058-bounded-helper-exact-diagnose`
exited 0. Base is the same 897eaa88 image; fixed candidate `736200aa` image
SHA-256 `1f35cff0d474007b4c51bcd5fbbcba043642119a22e5212fed1e3fbc5ed87339`.
Disassembly now uses fixed scalar/SSE loads/stores and no dynamic memcpy in
argument setup; the helper occupies 9,136 bytes (adjacent-symbol range).
Three N-to-2N increments give base 363.3028/363.3591/363.2807 instructions
per call, fix 358.3279/358.2924/358.3255, median ratio 0.9863001. Darkroom
whole instruction medians are 15,544,772,115 versus 15,511,280,453, ratio
0.9978455. Different diagnostic cohorts are retained separately, not pooled
with r2, and no formal readiness/re-decision is inferred from these counts.

Diagnostic SHA-256: per-arity events `d69ab9c227aa9a6b8428c8f04dc09e9298c5a2cb193def3fde91b90b850a249e`;
qualified costs `6a683c91e609f130d30e2ad6e79abe677edc1d3950baa89b56d4ae7d79dd29d2`;
fixed count summary `cc7eba7636eed6a560b8c8c0237d48c242351625c7f03db3f11ec0a6bc6fcd0b`;
fixed raw counts `5107004cee5b31c93bd7cae673042b95274ba2011cd569ccad3009d1c1815721`;
fixed prologue `14399942d484189007f20102dc9a0d92f001e7e3a6c6ed2d70d5433af16fece6`.

Quick triage `20261019T231115Z-t058-bounded-helper-quick-triage`: base
`a68a9f46`, candidate `736200aa`, sole target papaparse/csv and all fixed
controls, including recursive_call_tree. Queue was idle; iMac pre-triage
clock 2026-10-20 07:12 CST, outside 03:00-05:00. The fixed candidate cache
was warmed by the preceding diagnostic build, not hidden in reported triage
stage times. No new plan or acceptance job was submitted. Result: **unclear**, exit 0,
100/100 valid pairs, zero invalid/heavy samples, no DISTURBED. Total
424.614 s (7.08 min): queue quieting 60.807, base build 69.749, candidate
cache hit 0.785, collection 281.287, replay 8.049, analysis 2.423 s.
Papaparse wall 0.0805106 [0.0796824, 0.0810696], instructions 0.0490336
[0.0489010, 0.0490793]. Recursive control instructions 0.9861834
[0.9861348, 0.9863017], wall 0.9549595 [0.9330278, 1.0095361]. These are
five-pair exploratory 95% intervals, not formal precision/acceptance.
No obvious regression under the heuristic. Four control wall upper endpoints
cross their frozen bands: polymorphic_call_site [0.9425, 1.0958],
object_allocation [0.9847, 1.0763], string_slice [0.9930, 1.1208],
top_level_function_call [0.9106, 1.1792]. Their instruction medians are
1.0000. These data cannot rule out their wall regressions or claim portfolio
neutrality. Imaging-darkroom is not in this fixed triage inventory; its
restored instruction cost is supported by the independent diagnostic above.
Request SHA-256 `6d397e6c57f73018be6a146e0cb3f20032b528598ac7e42b0791c1706ef65335`;
result `8c93adf0d9d9ee22999bad9cee1f4a12177f3ffbf8ce26106d235bb3baf8a092`;
Markdown `b5c98654a19c78a49819f5f365e906c426663213f99996c047c3bad93cb1abf8`.
Both triage images reproduce the diagnostic base/fix hashes exactly.

## T057 counterfactual and handoff

`20261019T232145Z-t058-pre-t057-codegen-context` exited 0. It builds
T057's pre-merge main `62ff12ba` and a counterfactual tree based on exactly
that commit with only matcher/first.rs replaced by lazy-only `f92448f0`.
The pre-merge and current base versions of that file are byte-identical.
Both pre-T057 images retain the 9,056-byte direct-copy helper, with
363.3158 and 363.3000 instructions/call. T056 M1 is already present there,
so it alone is insufficient. In this controlled context the T057 runtime
changes are necessary for the lazy change to switch argument initialization
to memcpy. This isolates a compile-time interaction, not a runtime String,
IC or call-tier path; no specific T057 function or LLVM heuristic is claimed.
Counterfactual JSON SHA-256 `e232b86dfd3458e4324ad5b58e258a1dd76e33b56ce412bdcf5ad7c2419a8484`.

The earlier top-level synthetic probe was checked independently by
`20261019T231954Z-t058-probe-path-check-cleanup`: all four arities execute
zero prepared numeric helpers. Its near-zero nominal-call deltas therefore
say nothing about this helper. The qualified wrapped probe executes the
recorded 16,129 entries per arity and is used above. Both probe results and
both failed diagnostic jobs remain retained, with no favorable selection.
Probe-path JSON SHA-256 `ab476a9c1c2ede75f7532c55663ed161d742561d0c1825e14bedd72b8eab26b9`.
The cleanup removed only a failed diagnostic Cargo build cache. Local
`target/debug` (~2.0 GiB) and fetched Cargo build copies (~154 MiB) were
removed after checks; canonical binaries, profiles, raw panels and counters
remain. No origin push or merge, no new plan, no new acceptance.
