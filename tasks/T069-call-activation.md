# T069: Positional call and activation preparation

- Status: active; frozen plan prepared before runtime changes.
- Implementation: none.
- Verified at: base `b93e4daa`, full local gate 2026-10-23: 2,335 runtime
  tests, 732 benchmark-tool tests, 35 script tests and 5,170 curated Test262
  cases pass; runtime is still unchanged.
- Evidence: this task, [plan](performance-units/positional-call-activation.json),
  [T068 profiles](archive/T068-profile-evidence.md).
- Unresolved: implementation/measurements not started; pre-existing dynamic-closure gap below.
- Next action: commit the validated schema-3 plan, then implement the architecture below.

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
  immutable IR activation metadata, `bytecode/compiler.rs` (passing existing
  parameter AST metadata only), focused tests and module documentation.
- T070 owns executor operand/register transfer; do not restructure those paths.
- No push, merge, main-checkout edits or promotion decision.

## Job ledger

- `20261023T044841Z-t069-baseline-queue`: cancelled while queued (130), no
  measurements. The already-running T070 baseline job has exactly the same
  base/candidate, 30 blocks and workers-4 command; reuse its sealed bundle
  `20261023T044840Z-t070-base-queue` rather than duplicate host work.
- `20261023T045032Z-t069-exact-profiles`: exit 0; three complete-process
  1ms sample runs for each of the five selected development programs at b93e4daa.
  All 15 receipts and copied assets verify against exact image
  `b3132b40cc52b9b39012089db873c25dc7d2a1802e249322c1891577edee58c1`.
- Shared baseline `20261023T044840Z-t070-base-queue`: exit 0 in 2,085 s,
  but unhealthy: base `branch_arithmetic` linearity 1.1696704 exceeds 1.15.
  All 30 blocks and samples are retained; no queue may use it. The Rust roles
  are byte-identical (`b3132b40...`), so this is not a candidate effect.
- `20261023T052725Z-t069-baseline-replacement`: exit 0 in 2,016 s; healthy
  whole-bundle replacement with `ready_for_unit_gates`, no measurement issues,
  same refs, 30 blocks and workers-4. No samples or lanes are pooled with the
  invalid bundle. This is a pre-implementation baseline, not a unit attempt.
- `20261023T052759Z-t069-bind-replacement`: exit 0; queue generated only
  from that replacement's complete healthy sealed bundle.

Fresh setup-frame inclusive unions (all three repetitions, recursion counted
once) bind the preselected programs; they do not retarget the unit:

| Development program | Setup union / main-thread samples |
| --- | ---: |
| lodash/transform | 368 / 4,439 |
| immutable/persistent | 1,278 / 9,249 |
| diff/compare | 1,015 / 14,787 |
| graphlib/paths | 386 / 3,248 |
| moment/dates | 997 / 26,068 |

The union recognizes `function_env`, FrameBindings, initial slot/cell/mask
routes, function-frame/VM constructors, frame_call_env/callee_env, seeding
eligibility and captured-cell indexing. It excludes body-level slow binding
access that positional authority can also remove, but may include coercion
called during setup. It is reach, not a savings forecast. Receipt hashes and
per-repetition fractions will be bound in the frozen plan.

Base full-gate log SHA-256:
`ed6fc08f35cbf3afec8ccfe841e6c00e0651f4f24821af2c27342ccbc5b04015`.
The log under `target/t069/base-check.log` is local and will be cleaned;
the counts and revision above are the durable result.

Before runtime changes, a complete local release/agents scan of b93e4daa also
passes all **42,672 configured cases**, with zero failures/timeouts/not-run
and zero actionable gaps (NG 42,602 pass / 70 fail). The complete entry is
appended to `docs/conformance/burndown.jsonl`. Exact unchanged harness scripts
and copies of the pinned test/harness directories ran under
`target/t069/conformance`; the reference executables were read-only links to
the main checkout. No checkout's `third_party` was modified. Eight workers,
30-second case timeout, `QJS_AGENTS=1`; unsupported NG-excluded cases are not
included in the configured-case claim. Raw evidence is local, not retained.

- Rust binary SHA-256: `557c0dd0a571d2ddb1b152c162bbf5f573f0e37fcfb7e25043427beb9e9c514d`.
- Rust case JSONL: `18f6a9a9db9a442f52f9b05ca9ffa618a877488360972eb6ef7a72f9ccc5c549`.
- NG case JSONL: `aae13b897e579f5f35ba7226d9e0c4a78d7105bb18cc33c7275986aa02e775cb`.

## Acceptance and verification

- Plan validation: `validate-unit` passes against queue SHA-256
  `71bf531c39224650b54956e054e2c4f7935798ce6aa109a85b4b1813fe074221`;
  frozen plan SHA-256 `548257aa6d218fbdcf7f3b5036c770025952a78f1b7b731db8e77df8db708dd9`.
- [ ] Validated frozen schema-3 plan committed before runtime changes.
- [ ] Focused parameter/cell/context restoration tests and full local gate.
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
