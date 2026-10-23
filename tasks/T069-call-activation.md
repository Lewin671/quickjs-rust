# T069: Positional call and activation preparation

- Status: active; owner implementation/evidence complete, lead review pending.
- Implementation: broader candidate `9aac3558`; first candidate remains committed.
- Verified at: broader candidate `9aac3558`, 2026-10-23: full gate
  passes, 2,366 runtime tests (31 new), 732 benchmark-tool tests, 35 script
  tests and 5,170 curated Test262 cases. Complete candidate scan also passes
  all 42,672 configured cases with zero parity gaps at `9aac3558`.
- Evidence: [final measured record](archive/T069-final-evidence.md),
  [plan](performance-units/positional-call-activation.json),
  [first candidate](archive/T069-first-candidate.md).
- Unresolved: strict moment/diff/graphlib payoff misses require lead review
  under the prospectively frozen overall-first rule; pre-existing gap below.
- Next action: lead adjudicates promotion from the complete evidence. No final
  decide/promotion, origin push or merge was performed by this owner.

## Goal and selection

Implement T068 proposal 1 as an incremental call-preparation unit. The target
rule is fixed before candidate measurements: every development program whose
T068 calls-plus-environments exclusive self share is at least 18 percent.
This selects lodash/transform (25.878%), immutable/persistent (22.278%),
diff/compare (20.363%), graphlib/paths (18.440%), moment/dates (18.256%).
The remaining 81 policy cases are controls. Held-out never selects work.
These shares bound reach, not expected savings. The earlier T056 prototype
(`117082ed`) priced a narrower borrowed-context population; its 0.961
development result is motivation, not this unit's acceptance evidence.

T022 overall-first acceptance is frozen prospectively: development, external,
equal-case all-case portfolio and held-out are overall gates. Individual
lane ceilings, sentinels/microbenchmarks and RSS are local gates; report every
local regression. RSS: per-case median at most 1.10, development geometric
mean at most 1.05, at least 15 alternating pairs. Use T067's host-derived
resource manifest. Correctness and zero Test262 gaps cannot be waived.
The lead makes the final promotion decision; this owner does not run decide.

## Target architecture and implementation order

1. Each immutable bytecode owns its frame-initialization metadata: static
   capture-cell requirements, incoming binding routes and positional argument
   slots; parameter shape, arguments shadowing and rest-slot facts are computed
   from the existing AST at compilation. Cache transitive eval/with requirements
   through function and class bodies, including deferred keys/private elements.
   Runtime realm/module/dynamic cell identities remain runtime inputs.
2. General synchronous calls can supply argument slices directly to the
   frame initializer without first allocating a named binding for each
   parameter. The existing compiled parameter prologue still owns defaults,
   destructuring, duplicate parameters and TDZ. Mapped arguments and dynamic
   eval/with paths retain the existing compatibility setup where required.
3. Initialize shared capture cells only after positional slots are seeded;
   escaping and sibling closures must retain the identical cell. Materialize
   special call bindings only when needed; retain a services CallEnv wherever
   execution actually needs realm/module/private/host state. Do not borrow a
   caller's mutable binding view or introduce restoration obligations.
4. Existing compact/direct-leaf priority, native and callback boundaries,
   construction completion and suspended-frame ownership remain intact.
   This is neither T021's shared-frame driver nor a register-transfer rewrite.

The protected [binding model](../docs/design/env-model-rewrite.md) is the
authority: no per-call name-keyed snapshot, no copied binding authority.
Admission depends on operations and binding requirements, never program names.
No dependency, unsafe code, compiler recipe or third-party edit is planned.
Freeze mandatory K=6 selection, 30 formal blocks, 30 held-out pairs and a
two-attempt budget; the unit removes CPU work rather than changing Value or
allocator representation, and still collects RSS and host resource lanes.

## Baseline semantic observation

Before any runtime edits, fresh debug and release/agents builds of `b93e4daa`
both fail this program with `ReferenceError: undefined identifier value`;
the pinned local QuickJS-NG returns 8:

```js
function outer(value) { return function () { return eval('value'); }; }
outer(8)();
```

This is outside T069's static-call optimization. Preserve the existing route
for transitive eval/with and report the gap for separate conformance work;
do not encode the wrong result as a behavioral regression test. The current
19 static/context-focused programs agree on both engines before changes.

## Scope

- Owner: `agent/t069-call-activation/codex`, base
  `b93e4daa5d9d53f1780a66f31b31a96e2bfe1895`, this worktree.
- Runtime paths: `function/call*`, bytecode frame/binding initialization,
  immutable IR activation metadata, `bytecode/compiler*.rs` (passing existing
  parameter metadata, including synthetic constructors), focused tests and
  module/binding-model documentation.
- T070 owns executor operand/register transfer; do not restructure those paths.
- No push, merge, main-checkout edits or promotion decision.

## Evidence and current revision

The [first candidate record](archive/T069-first-candidate.md) freezes baseline,
profile, correctness and triage receipts. Candidate `74e8f8aa` passes all 42,672
configured Test262 cases, but its triage is `likely loss`; that gate deferred
without spending an attempt. The broader candidate has now spent **1/2**:
its complete K=6 screen strictly rejects three target payoff gates, while all
six development/external/portfolio aggregates improve. No budget is reset.

A broader preparation revision is committed at `9aac3558`: static callbacks
keep shared cells without caller name export, marked/dynamic/with contexts
retain their compatibility views, global refresh visits precomputed fallback
slots, and ordinary completion recycles cleared storage in place. Derived and
suspended completion routes stay intact. No caller environment is borrowed.
Full checks and new callback/storage tests passed before that commit.
Entry metadata is boxed once per body, keeping its vectors out of the hot
bytecode payload. Active frames remain independent; recycled slots are cleared
and captured cells are never recycled. Eval/with, retained scopes and marked
realm views remain on their compatibility routes. No executor register transfer
or active-frame scheduler is changed.

- `20261023T081718Z-t069-repair-correctness`: exit 0; iMac comparisons/subset.
- `20261023T081731Z-t069-repair-triage`: exit 0; diagnostic `unclear`, 105/105
  valid pairs, no invalid/heavy pairs, 329 s including quieting, within budget.
- `20261023T082004Z-t069-repair-screen-promising`: queued before seeing the
  new triage result. Starts attempt 1 only if correctness passes, triage is
  complete/in budget, and frozen-target geometric means are at most 0.97 in
  both wall and instructions. This is a diagnostic spending trigger, not an
  acceptance change: the complete K=6 screen keeps every frozen gate.
- The trigger passed: target wall geomean 0.929874, instructions 0.921258.
  Attempt 1 is collecting. Full candidate conformance passes and is recorded
  in the burndown; local release/agents image SHA-256
  `564d32f4c423a05495c81f6a161c335b20d1fbfb9b0f982fafb27a368238ec59`.
- `20261023T082854Z-t069-rss15`: exit 0, all 15-pair RSS limits pass;
  development geomean 0.998300, maximum case 1.009052.
- `20261023T082855Z-t069-resources`: exit 0, healthy/complete; size 1.000975,
  fresh 0.987491, resource RSS 1.011464 C/B; host replay queued below.
- `20261023T092154Z-t069-formal30`: exit 0, healthy/complete 30-block bundle;
  all 86 v6 case precision checks resolved without continuation. Joint wall
  C/B: development 0.957429, external 0.998279, portfolio 0.991613; intervals
  all below 1. Development C/NG 2.640021. Strict target payoff misses remain.
- `20261023T092154Z-t069-heldout30`: cancelled (130) after queue-order race;
  partial data discarded whole, no metrics read or used for work selection.
- `20261023T093959Z-t069-heldout30-ordered`: exit 0 in 4,682 s, all six
  layouts/30 pairs; combined held-out 0.929786, layout ratios 0.925992–0.932820,
  all intervals below 1. No missing held-out precision evidence.
- `20261023T095351Z-t069-resource-replay`: exit 0, all raw resource reports
  authenticate against formal engines/host manifest. Selection/held-out raw
  replay also passes. No final decide or promotion step is authorized or run.
- Full-gate log SHA-256:
  `5917288109336a9a2dc6a16236a890cb60e9c5f0fe990a1dde468574dd50b430`;
  the commit hook also passed its 117-case mapped slice.

## Acceptance and verification

- Plan validation: `validate-unit` passes against queue SHA-256
  `71bf531c39224650b54956e054e2c4f7935798ce6aa109a85b4b1813fe074221`;
  frozen plan SHA-256 `548257aa6d218fbdcf7f3b5036c770025952a78f1b7b731db8e77df8db708dd9`.
- [x] Validated frozen schema-3 plan committed before runtime changes (`982d19b5`).
- [x] Focused parameter/cell/context restoration tests and full local gate.
- [x] QuickJS-NG comparisons and zero-gap complete Test262 evidence.
- [x] Quick triage; if promising, finite-budget screen, formal, held-out,
  RSS and resource evidence on the serialized iMac queue.
- [x] Evidence and local regressions recorded; lead decision remains pending.
- [x] Final full gate, branch-scope validation and workspace cleanup.

```sh
./scripts/check-touched.sh --staged --explain
./scripts/check.sh
./scripts/compare-qjs.sh
```

Final full gate also passes on unchanged runtime `9aac3558` with the handoff
record; log SHA-256 `86abe1e1d060467e42ebe7227ecf41c7506cb09ab31fb506baa0c2e0b442435e`.
Branch descent/path-scope check against b93e4daa passes. Remote CI is unrun
because the user requested no branch publication.
Cleanup: `cargo clean` removed 69,304 files / 9.6 GiB; the tracked target file
was restored with `git checkout -- target`. Free disk is 53 GiB. Handoff
documentation is committed without rebuilding cleaned binaries.
