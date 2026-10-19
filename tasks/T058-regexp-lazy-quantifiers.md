# T058: Rejected v6 unit; diagnostic-only instruction regression work

- Status: active diagnosis; v6 current-base unit rejected on its sole valid screen; no re-decision or budget reset.
- Implementation: rejected T058 changes present; diagnostic fix makes the numeric helper's existing 0-4 argument copies explicit. No optimization retained.
- Verified at: `88bec228`, 2026-10-19 UTC; fetched r2 rejection and raw K=6 evidence, original iMac exact-image instruction/profile diagnostic.
- Evidence: [v6 rejection](archive/T058-v6-rejection.md), [attempt 1](archive/T058-attempt1-incomplete.md), [attempt 2](archive/T058-attempt2-inconclusive.md). New diagnostic jobs/results recorded below.
- Unresolved: final helper-arity attribution, full gate completion and canonical fixed-candidate counts/quick triage; no acceptance.
- Next action: compare exact-image profiles and feature execution counters; inspect the helper's generated argument-copy prologue. No new plan or acceptance is authorized.

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
made about the particular LLVM heuristic until isolated; source size alone
is not established as its trigger.

Diagnostic jobs are non-timing and leave every raw counter/profile artifact:
- `20261019T225612Z-t058-exact-call-instruction-diagnose`: exact counts,
  eight-second samples and function-body comparison; exit 0.
- `20261019T225638Z-t058-execution-path-diagnose`: independent feature builds
  of both source revisions; execution counters and compact/IC traces.
- `20261019T230009Z-t058-codegen-lineage-diagnose`: canonical intermediate
  builds, counter increments and disassembly; exit 0.

Record conclusions, hashes, revision and checks here before ending. Any
new candidate remains diagnostic-only and cannot reopen the rejected unit.

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
region 9..29 (8 registers, 4 boxed). Exact images, feature builds and old
images are kept distinct throughout this diagnosis.
