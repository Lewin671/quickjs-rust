# T069: Positional call and activation preparation

- Status: active; broader preparation revision after the first diagnostic.
- Implementation: repair working tree over `74e8f8aa`; first candidate remains committed.
- Verified at: broader working tree over `74e8f8aa`, 2026-10-23: full gate
  passes, 2,366 runtime tests (31 new), 732 benchmark-tool tests, 35 script
  tests and 5,170 curated Test262 cases. Complete 42,672-case conformance
  currently applies to first candidate `74e8f8aa`, not yet this revision.
- Evidence: this task, [plan](performance-units/positional-call-activation.json),
  [T068 profiles](archive/T068-profile-evidence.md).
- Unresolved: candidate measurement and lead decision;
  pre-existing dynamic-closure gap below.
- Next action: finish the broader revision gates, commit it, and queue its correctness and triage. Do not change the frozen targets.

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
configured Test262 cases, but its triage is `likely loss`; the prospective gate
correctly deferred. **No screen attempt has been spent (0/2).**

A broader preparation revision is present over `74e8f8aa`: static callbacks
keep shared cells without caller name export, marked/dynamic/with contexts
retain their compatibility views, global refresh visits precomputed fallback
slots, and ordinary completion recycles cleared storage in place. Derived and
suspended completion routes stay intact. No caller environment is borrowed.
Full checks and new callback/storage tests precede its next commit.
Entry metadata is boxed once per body, keeping its vectors out of the hot
bytecode payload. Active frames remain independent; recycled slots are cleared
and captured cells are never recycled. Eval/with, retained scopes and marked
realm views remain on their compatibility routes. No executor register transfer
or active-frame scheduler is changed.

## Acceptance and verification

- Plan validation: `validate-unit` passes against queue SHA-256
  `71bf531c39224650b54956e054e2c4f7935798ce6aa109a85b4b1813fe074221`;
  frozen plan SHA-256 `548257aa6d218fbdcf7f3b5036c770025952a78f1b7b731db8e77df8db708dd9`.
- [x] Validated frozen schema-3 plan committed before runtime changes (`982d19b5`).
- [x] Focused parameter/cell/context restoration tests and full local gate.
- [ ] QuickJS-NG comparisons and zero-gap complete Test262 evidence.
- [ ] Quick triage; if promising, finite-budget screen, formal, held-out,
  RSS and resource evidence on the serialized iMac queue.
- [ ] Evidence and local regressions recorded; lead decision remains pending.
- [ ] Final full gate, commit, cargo clean and tracked target-file restoration.

```sh
./scripts/check-touched.sh --staged --explain
./scripts/check.sh
./scripts/compare-qjs.sh
```
