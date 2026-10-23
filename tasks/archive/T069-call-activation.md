# T069: Positional call and activation preparation

- Status: closed 2026-10-23 — promoted and integrated under the [lead decision](#lead-promotion-decision-2026-10-23).
- Implementation: receiver repair `e38e85fc`; corrected comparison fixture
  `6c512032`; broader preparation remains present (rebased `66e31330`).
- Verified at: runtime `6c512032`, 2026-10-23: full gate passes, 2,382 runtime
  tests (16 repair tests), 5,170 curated cases, iMac 257-fixture comparison and
  diagnostic triage; complete exact scan passes 42,672 configured cases, zero gaps.
- Evidence: [receiver repair and binding audit](T069-call-context-fix.md),
  [original measurements](T069-final-evidence.md),
  [plan](../performance-units/positional-call-activation.json),
  [first candidate](T069-first-candidate.md).
- Unresolved: pre-existing deferred-thunk/eval context gaps (main too) move to a conformance follow-up task.
- Next action: none for T069.

## Lead promotion decision (2026-10-23)

Overall gates, frozen prospectively under the T022 overall-first rule, all pass on `9aac3558`: formal development 0.957
(joint 95% 0.956–0.959), external 0.998, equal-case portfolio 0.992, held-out K=6 0.930; RSS development geomean 0.998;
complete exact Test262 42,672/42,672 with zero gaps. Local failures, recorded not hidden: moment/diff/graphlib miss their
strict payoff gates (none is slower), one sentinel layout 1.042 over its 1.019 ceiling, and K=6 microbenchmarks up to
1.029 (array_index_of, plain_function_call) with flat instructions. An independent dsh review found a real regression
(receiver elided for deferred class computed keys), reproduced by the lead; repair `e38e85fc` adds 16 context tests and
re-passes the full gate, 257 reference fixtures and the complete scan. Its diagnostic triage keeps the five targets
(lodash 0.797, immutable 0.927, diff 0.981, graphlib 0.973, moment 1.007) with control instructions within limits, so per
the lead's prospective condition the full chain is not rerun. The lead promotes T069; no decide command was run, the frozen
plan is unchanged.

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

The protected [binding model](../../docs/design/env-model-rewrite.md) is the
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

Lead review reproduced a receiver-elision regression after rebase onto
`311bff0e` (documentation-only difference from the measured runtime). A class
computed key that mentions a private name is compiled separately: its thunk
reads the enclosing frame's `this`, invisible to an own-body opcode scan.
The repair propagates immutable receiver requirements through deferred keys
and lexical arrows, and makes arrow compilation allocate/capture that receiver
cell. Ordinary functions, methods, fields and static blocks own their receiver
and stop propagation; each still prepares its own nested code. Focused tests
cover these boundaries, escaping arrows, defaults, modules and other context.

## Original measured revisions

The [first candidate record](T069-first-candidate.md) owns the baseline,
profile, correctness and rejected initial triage. The broader `9aac3558`
(rebased `66e31330`) completed the full performance chain recorded in the
[original frozen measurements](T069-final-evidence.md). Its screen
strictly rejects the moment/diff/graphlib payoff gates while overall aggregates
improve; one of two screen attempts is spent, with no reset or promotion.
Those formal/held-out/resource results describe that original binary.

The lead requested a receiver repair, full correctness gates and one quick
triage on the same five targets against original base `b93e4daa`. The repaired
runtime is `e38e85fc`; `6c512032` corrects only the new comparison fixture.
The [repair record](T069-call-context-fix.md) owns its binding audit,
three pre-existing lexical-thunk gaps, correctness results, job ledger, full
triage table and raw identities. The fixed diagnostic rule finds no obvious
regression; four possible wall-control flags remain, notably prototype-method
calls at 1.050003 C/B. No new formal chain was run under the lead's diagnostic
scope, and the old formal intervals are not claims about this repaired binary.

## Acceptance and verification

- Plan validation: `validate-unit` passes against queue SHA-256
  `71bf531c39224650b54956e054e2c4f7935798ce6aa109a85b4b1813fe074221`;
  frozen plan SHA-256 `548257aa6d218fbdcf7f3b5036c770025952a78f1b7b731db8e77df8db708dd9`.
- [x] Validated frozen schema-3 plan committed before runtime changes (`982d19b5`).
- [x] Focused parameter/cell/context restoration tests and full local gate.
- [x] QuickJS-NG comparisons and zero-gap complete Test262 evidence.
- [x] Original finite-budget screen, formal, held-out, RSS and resource chain;
  repaired candidate has its separately identified diagnostic triage.
- [x] Evidence and local regressions recorded; lead decision remains pending.
- [x] Final full gate, branch-scope validation and workspace cleanup.

```sh
./scripts/check-touched.sh --staged --explain
./scripts/check.sh
./scripts/compare-qjs.sh
```

Final full gate passes on runtime `6c512032` with the handoff documentation;
the [repair record](T069-call-context-fix.md) retains its checksum.
Branch descent/path-scope validation uses the lead's rebased base `311bff0e`.
Remote CI is unrun because the user requested no origin branch publication.
Cleanup and complete-scan results are recorded with the repair evidence.
