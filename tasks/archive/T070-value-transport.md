# T070: Ownership-aware operand/register transfer

- Status: closed without promotion, 2026-10-23; broadened triage wall geomean 0.975245 exceeds the lead's <=0.97 condition.
- Implementation: present; wide and typed-loop last-use transfers, exit ownership handoff, borrowed dense reads, VM receiver/argument moves; call entry unchanged.
- Verified at: `b74a84a9`, 2026-10-23; fast/full gates, 2362 runtime tests, 5170 curated Test262 and 256 NG fixtures pass. Full exact zero-gap scan remains at `83f47dba` only.
- Evidence: [frozen plan](../performance-units/ownership-aware-register-transfer.json), [broadened triage and closure](T070-broader-transport.md); original triage/screen retained below.
- Unresolved: cohort payoff is insufficient; control regressions remain; no formal/held-out/RSS/resource or new full exact claim for the broadened candidate.
- Next action: lead reviews the retained branch and closure; this unit spends no further attempt and makes no promotion decision.

## Goal and boundary

Implement T068 proposal 3 without changing Value width, reference ownership,
binding storage, call entry, activation preparation, or dependencies. Owner:
`agent/t070-value-transport/codex`, base `b93e4daa5d9d53f1780a66f31b31a96e2bfe1895`.
T069 owns call/frame/environment preparation. Runtime edits cover compiler-proven register and operand transfer across tiers,
without call/frame preparation changes; task and
plan indexes are this owner's documentation. No origin push or merge.

## Target architecture and stages

1. Freeze the plan and verified exact-base receipts before runtime edits.
2. Analyze the final wide register IR after forwarding, peepholes and jump
   relocation. Backward fixed-point liveness joins both successors and loop
   backedges. Interpreter exits conservatively read all locals and every
   operand within their compiler-verified stack depth, preserving the state
   an interpreter or typed loop may observe on resumption. Retired operands
   above that depth are not exit inputs.
3. Replace a register copy with an ownership take only when its distinct source
   is dead on every successor. For a general binary operation, take both
   distinct operands only when each old input is dead (an overwritten result
   register also qualifies). Keep the numeric fast path. The executor replaces
   consumed sources with Undefined and publishes through the existing store.
   Live sources, checked lexical reads, received cells, and observable stores
   retain their paths. Existing aliases remain separate owning references.
4. Verify semantics, then use queue-only triage before spending a frozen screen
   attempt. A promising candidate runs complete screen/formal/held-out and
   resource/RSS evidence; the lead alone decides promotion.

The proof concerns register contents, never virtual holder aliases or source
patterns. It preserves register numbering, operation count, jump targets and
resume maps. `Drop` reads and kills a register: an explicit release keeps its
source live until that instruction. Calls read their full callee/receiver/
argument ranges. A future operation must explicitly declare its value and
control-flow effects.

Historical constraints: T021's virtual-holder experiment failed alias writes
and branch merges; this pass runs after existing alias materialization and
never changes it. T018's discarded-member experiment broke loop admission;
this pass never changes stack bytecode or its grammar. T056 M1.2's ownership/
object-layout regression is not retried: Value and heap layout stay unchanged.

## Frozen selection rule and acceptance

Select development programs whose pooled ownership-family self fraction in
T068 exceeds that family's equal-program mean (11.906%), counting every
repeat and using no held-out data: graphlib 569/3243 (17.55%), big.js 832/5447
(15.27%), immutable 1233/9296 (13.26%). These are reach observations, not predicted
speedups. New exact-base samples bind the byte-identical mechanism evidence to
the fresh queue; they do not reorder the targets. The plan will bind v6 formal
analysis and qualified workers-4 v5 collection, a two-attempt budget, mandatory
K=6, 60 formal blocks and 30 held-out pairs.

T022 overall-first applies prospectively: overall measures are development,
external, equal-case portfolio and held-out. When all improve, local lane
ceilings, individual microbenchmarks/sentinels and RSS do not block the lead's
promotion decision; every regression remains reported. Correctness and zero
Test262 gaps are never waived. RSS defaults are per-case median <=1.10,
development geomean <=1.05 with >=15 alternating pairs; resources use T067's
derived iMac manifest. No final decide/promotion command is authorized.

## Collection

- `20261023T044840Z-t070-base-queue`: 30-block `f49be95c` versus `b93e4daa`
  workers-4 comparison, exit 0 / 2085 s, no DISTURBED (two light samples).
  Both Rust binaries are SHA-256 `b3132b40cc52b9b39012089db873c25dc7d2a1802e249322c1891577edee58c1`.
  Invalid as queue evidence: base `branch_arithmetic` linearity 1.1696704 exceeds
  1.15. All 2250 broad measurements and 30 blocks are valid; 74/75 linearity
  checks pass. The whole bundle is replaced, never pooled or repaired.
- `20261023T044950Z-t070-exact-profiles`: three process-complete sample windows
  requested per target plus diff, exit 1 / 70 s before sampling: collector
  omitted creating its workload directory. Fixed in the replacement below.
- `20261023T045305Z-t070-bind-queue`: generate the fresh queue from the complete
  baseline bundle, exit 2 / 40 s: `queue requires healthy complete evidence:
  broad: incomplete or unhealthy measurement`.
- `20261023T051239Z-t070-export-bindings`: cancelled (130) after its profile
  prerequisite failed; no data used.
- `20261023T052518Z-t070-baseline-health`: exit 0; identified the linearity
  failure above without editing any measurement or threshold.
- `20261023T052723Z-t070-exact-profiles-r2`: exit 0 / 41 s; all 12 complete
  sampling receipts and workload stdout checks pass at exact `b93e4daa`.
- Shared `20261023T052725Z-t069-baseline-replacement`: exit 0 / 2016 s; all
  lanes healthy, no measurement issues, ready for unit gates; two light
  background samples, no DISTURBED. Only T070 jobs were cancelled by this owner.
- `20261023T053028Z-t070-bind-export-r2`: validate replacement queue and export
  content-verifiable bindings into this worktree, exit 0.

`validate-unit` passes with nine content-verified receipts on 2026-10-23 UTC.
Queue SHA-256 `71bf531c39224650b54956e054e2c4f7935798ce6aa109a85b4b1813fe074221`;
plan SHA-256 `9762572011ef5e380c39843ef87759c85fab7b3246406d099149ed94c0e1d04a`.
New pooled self counts are graphlib 620/3225, big.js 877/5418, immutable
1123/9251; the T068 selection rule and targets remain unchanged. No runtime
file changed before freeze commit `c411c6ce`; no screen attempt was spent at freeze.

## Candidate and attempt ledger

Candidate `83f47dba069ad3064cda49dd4ebbfbc68cd08735` implements the target
architecture above. No unsafe, dependency, public API, heap or binding-model
change. The closed effect match includes control flow and explicit Drop uses;
the analysis allocates only during first compilation, never per activation.

Triage `20261023T061259Z-t070-triage` is unclear: graphlib wall 0.946417 /
instructions 0.976696, big.js 0.997420 / 0.999196, immutable 1.016849 / 0.992150.
No obvious heuristic regression; capturing closure and moment wall intervals
cross their diagnostic bands. The complete table and stage times are in the
triage appendix. This is not a promotion or formal rejection.

`20261023T062146Z-t070-screen-attempt1` finishes all 86 cases and six layouts at
exact `83f47dba`, workers 4; exit 3 / 3240 s, no DISTURBED. First of at most two
valid attempts is spent; one remains. [Complete screen record](T070-screen-attempt1.md)
includes every case/layout, hashes, target failures and the instruction alert.
The tool reports reject/incomplete despite full physical collection. K=6 wall
point estimates: development 0.992205, external 0.996869, portfolio 0.994242.
Lead declined promotion on 2026-10-23: these ~0.6–0.8% improvements are below
the host's ~5% noise floor; more collection would not change that decision.
Candidate full local gate passes, including 732 benchmark-tool tests, 35
script tests, agent/perf-counters feature checks and all 5170 curated Test262
cases. Fast-gate slices cover eval-code, block and function statements.

[Supplemental producer attribution](T070-producer-attribution.md)
uses the same exact-base windows: big.js has 651/877 ownership self samples
under the typed-loop executor and only 41/877 under the wide driver. These
are visible callers, not removable costs; inlined work is unresolved. No
target or implementation changes follow from this diagnosis during attempt 1.

The complete `--exact --all` scan passes locally on `83f47dba` runtime content:
53572 total, 10900 NG-config skips, 42672/42672 Rust passes; fail/timeout/not-run
and actionable gaps all zero. NG passes 42602 and fails its standing 70 cases.
The release + agents binary is SHA-256 `ab0a87723a7357bdd1586d48372f810ccd7f26f660ab0afa0c0ab0f5198128b4`.
The archived harness/corpus copy lives only under target; NG/Test262 pins are
`f7830186` / `42496613`, original references unmodified. The complete scan's
entry is appended to `docs/conformance/burndown.jsonl`. Correctness only; no
arm64 timing enters the performance evidence.

## Stopped measurement jobs

- `20261023T070041Z-t070-screen-export`: exit 0, authenticated screen metadata.
- `20261023T071955Z-t070-formal60`: 60 blocks, workers 4; cancelled by lead, exit 130.
- `20261023T071956Z-t070-heldout30`: 30 pairs across all six held-out layouts; cancelled by lead, exit 130.
- `20261023T072115Z-t070-rss15`: all ten development programs, 15 alternating
  single-process pairs; cancelled by lead, exit 130; no RSS result.
- `20261023T072116Z-t070-resources`: T067-derived size/fresh/RSS lanes; cancelled by lead, exit 130.

Queue status and iMac clock checked before submission (2026-10-23 12:48 CST,
outside 02:45-05:00). Queue-only ref transport is the wrapper's mechanism,
not publication to origin. Local free disk at start: 66 GiB.

## Lead amendment: final candidate (2026-10-23)

This supersedes the original wide-only scope and continuation decision. The
frozen JSON and attempt-1 screen remain unchanged. Attempt ledger: one spent,
one available; quick triage costs zero. The lead cancelled all four follow-ups
above, so no formal, held-out or resource conclusion is claimed for attempt 1.

Extend consumption to typed-loop boxed registers and exit publication, dense
reads into scalar/register consumers, VM general operations and value transfer
into call argument buffers. Observable arrays/cells and live aliases keep owning
references. T069 retains call/frame/activation preparation ownership.

Stages: first inspect each tier's recovery and operand lifetime contracts;
then add move-only transfers where all successor/recovery uses are preserved;
verify guards, branch/backedge joins and aliases before measuring. Run the same
seven development programs as T065 round 2: big.js, immutable, js-yaml,
graphlib, lodash, diff, moment (full IDs in that record). Freeze this cohort
before implementation; require their natural-layout wall-ratio geomean <=0.97
and correctness before spending the last K=6 attempt. Otherwise close without
promotion. The lead alone decides promotion; no final decide command.

The broadened candidate adds no representation, binding or activation change.
Typed-loop moves reuse final-IR recovery-aware liveness, excluding pinned
sources; guarded producers publish only after success. Both tiers share the
existing immediate-aware register store. Dense scalar reads borrow the present
own element instead of cloning a temporary; boxed reads retain array ownership.
Exit restoration moves the last use and clones duplicate stack/publication
uses. General VM property stores borrow popped receivers; argument drains and
wide slow-call buffers move already-consumed operands in source order.

Focused coverage: dead/pinned/self sources, either branch and loop/recovery
edges, duplicate recovery plus frame publication with ownership release,
residency/guard exits, dense holes/descriptors, argument order/aliases,
bound/proxy/throwing calls and observable setters. Focused typed-loop, wide,
array and general-VM tests pass; full gate and 256 NG fixtures pass at `b74a84a9`.

## Final diagnostic and closure

`20261023T075942Z-t070-broader-triage`: exit 0 / 287 s, 115/115 valid pairs,
no DISTURBED; seven-program wall 0.975245059666, instructions 0.987032034632.
This fails the lead's <=0.97 prerequisite: close without promotion. Attempt
ledger ends at one spent / two allowed; the last attempt is unused. The
broadened code remains present on this branch. No subsequent screen or formal,
held-out, RSS or resources run. `20261023T075949Z-t070-broader-compare` passes
256 fixtures, exit 0 / 21 s; `20261023T080100Z-t070-broader-export` exits 0.
The linked closure record retains every case, interval, hash, stage and local
regression. Full exact conformance was not repeated for this stopped candidate.

## Verification

Baseline and candidate: `cargo test -p qjs-runtime bytecode::compact_fn::wide::`.
Every commit: `./scripts/check-touched.sh --staged --explain`.
Candidate: `TEST262_DIR=<shared corpus> ./scripts/check.sh`, QuickJS-NG
comparison, and zero-gap conformance evidence before any promotion claim.
Final cleanup: `cargo clean`, restore the tracked target file, check status.
