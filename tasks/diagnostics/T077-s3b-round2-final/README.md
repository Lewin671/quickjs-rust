# S3b round-2 handoff

All requested round-2 repairs/checks are complete at **a6de336d**. Stop for the
remaining lead review. **No full/exact requalification or performance acceptance
is claimed or started in this round.** Main's generic hostname fix is integrated
through `55f5d1d6`; the old alias proposal remains an unapplied historical record.
No origin push or merge into main.

## Commits and root causes

- **28228ff1 — R1 canonical owner.** Wide inline calls/constructors kept actual
  this only in driver storage, leaving the activation owner undefined. Typed-loop
  deopts and other exits resumed the VM with that owner. The fix deletes the
  receiver side channels from positional inputs, VM frames, wide frames and
  resume arguments; all operations read the call owner. Constructor target,
  home/super and inherited owners remain canonical. Sibling binding admissions,
  native standalone entry and every resume route are covered. [R1 record](../T077-s3b-round2-r1/README.md).
- **78e8ebc3 — capture-mask bound.** Public IC verification exposed a pre-existing
  panic in lodash/moment: guarded `then_some(1_u128 << slot)` evaluated the shift
  for slots >=128. Filter before mapping to bits; retain general cell routes for
  large captures. Boundary tests span 128/129/130 captures, with a before failure
  on both candidate and baseline. [Mask record](../T077-s3b-round2-mask/README.md).
- **a6de336d — R2 effect model, independent commit.** Numeric DCE treated unread
  call results as pure, erasing callee validation/effects/throws and incorrectly
  creating leaf plans. Exhaustive effects permit DCE only for proven scalar
  operations; calls/native/unmodeled operations remain. Every relevant numeric,
  compact, wide and typed-loop elimination/forwarding path is audited.
  [R2 record](../T077-numeric-call-effects/README.md). Only its three numeric files
  overlaid on main ed022fa1 independently pass all 26 focused tests, without the
  Context/receiver/mask repairs, supporting a standalone cherry-pick.

No new dependency, unsafe, FFI, Rust async, executor or name-keyed execution
snapshot is introduced. No source/bytecode shape is recognized to evade a bug.

## Verification

- Final staged fast gate: **2,670 runtime tests**, **65 selected curated Test262
  cases**, formatting, strict workspace Clippy and context checks pass.
- R1: two low-layer owner tests fail before repair; six owner tests pass after,
  including combined binding/IC stress. The equality matrix covers 108 programs
  across four compares, three loops, function/shorthand/class methods and three
  JS caller forms. Wide **84** and dense **198** tests pass.
- R2: initial 24 focused tests fail 23 before repair; final **26** pass, including
  pure-DCE controls and getter/Proxy/coercion/throw loops. All **132 typed-loop**
  and **125 compact-plan** tests pass. Direct, local closure, call/apply/Reflect,
  bound, sort/toSorted, typed sort/toSorted, Array.from and Promise routes retain
  effects and exact thrown identity.
- **302 NG script fixtures** pass independently in default, IC-off and debug
  IC/binding-verification modes. All **four module fixtures** match NG exit/stdout
  across those three modes ([module matrix](modules.json)); Promise jobs change
  from old `0:2 / false` to fixed/NG `1:2 / true`.
- The new [public correctness sweep](../../../docs/public-workload-correctness.md)
  completes **86 cases × three modes**, **351/351 invocations pass**, zero failures
  ([summary](summary.json), [prospective request](request.json), [all rows](cases.jsonl)).
  Broad/sentinels run manifest initial and capped 2N counts; whole external and
  all development programs run intact. Expected stdout/checksums are unchanged.
  Audio-beat-detection passes all modes. No held-out workload is resolved or run.

The first full sweep is retained as a failed attempt: 348/351, with an unoptimized
verification deadline on gaussian-blur and real eager-shift panics in lodash and
moment. The fresh full sweep uses an optimized verification build with **both
debug assertions and overflow checks enabled**, keeping the same 120-second
process deadline and actual IC verification. No case, flag, output or failure is
waived or selected away. This is correctness testing, with no timing/counters or
performance verdict. [Metadata](metadata.json) records binaries, raw hashes and
the assertion-enabled build; all Rust input hashes match the committed tree.

## Next action

Receive the remaining lead fuzz findings, fix any confirmed issues, then perform
one fresh frozen requalification as directed: full/exact, stress/build/depth/
counters/order and healthy public86. Earlier exact/counter/depth receipts name
older source and cannot authorize this candidate. Lead owns formal/held-out/RSS
and stage acceptance; no payoff or S3b acceptance is inferred from these checks.
