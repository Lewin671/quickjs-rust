# T045: Boxed representation demand through identity-flow joins

- Status: active.
- Implementation: candidate present only on `agent/typed-boxed-demand/owner`
  at `dae07218`; main and the lead runtime remain unchanged. No retention yet.
- Verified at: candidate `dae07218`, 2026-10-13; full local gate, 5170 Test262
  subset, UTC NG247, CI38407657103 and complete configured zero-gap coverage.
- Evidence: workspace-local jobs `20261013T130216Z-dynamic-method-main-exact-profiles`
  and `20261013T130253Z-dynamic-method-main-route`; hashes and conclusions below.
  Raw artifacts are local, not retained in this repository.
- Unresolved: performance. Attempt-one dispatch is waiting for an idle host;
  no measurements or attempt outcome exist yet.
- Next action: collect the frozen screen `20261013T135302Z-t045-screen-attempt1`.
  Only a passing screen permits a formal run and retention decision.

## Evidence and boundary

The starting runtime is `27ca5140342df9050911dc5c76b5eb28a112266a`.
Main `d2bcf2c2` adds tools, tests and history; its runtime, architecture,
benchmark inputs and layout inputs are identical. The fresh queue SHA-256 is
`4d458d5657079cdb4e11478b743cb68788d1c1ad646fc55f0b85d5b9a949b712`.
`dynamic_method_call` ranks fourth at 3.3377 times NG. The frozen plan records
the explicit priority override; unresolved higher-ranked writes remain controls.

Three profiles use exact default binary
`b97911efb7bfc31df8bd672922b64888aa16b8c5f2cc049d60f4720e2b94a1c2`, unchanged
workload SHA-256 `e4c79e512f11bd3716cba0f2a85d650cc120594b6f06370acd92f74952c3248c`,
and the original CLI case at 16777216 diagnostic iterations. They are not timing
results. Ordinary VM exclusive self accounts for 810/1665, 821/1669 and
867/1669 samples. Named-read subtrees account for 165/1665, 170/1669 and
140/1669; resolved-call subtrees for 202/1665, 213/1669 and 231/1669.
These non-overlapping ranges describe costs, not promised removable fractions.
No Wide execution was observed in this case.

The feature-instrumented route compiles region 15..48 to 16 operations, then
reports `TLRUN Declined`, 10000 interpreter backedges and no `TLCLAIM`.
Successful closed-form callee execution does not prove typed-loop entry.
Existing named-property caches, fixed-argument direct calls and numeric leaves
are active. This is not another property-cache or argument-vector proposal.

The compiler joins different local origins as `Computed`. When the resulting
receiver local demands a boxed value, its incoming object locals remain scalar
sources, and scalar seeding rejects objects. Scalar register 48 and boxed
register 48 are separate namespaces; the equal number is not corruption.
The incoming receivers escape to ordinary heap values, not a virtual bank.

Job `20261013T131539Z-dynamic-method-seed-decline` confirms the exact rejection:
region 15..48 seeds scalar register 6 from local slot 2 holding `Object`.
Scalar locals include slots 2 and 3; the boxed map includes only receiver slot 4.
The feature-only diagnostic patch SHA-256 is
`7ec023d8074283187eb053e0b173775d5171064b3a616faa0b393008f76f3fe4`;
its binary is `45abdeeb2152e3157c70470d3ef62d9d7513889ff0e43d40e80308369e85a767`.
This is supplemental diagnosis, not exact release profiling or timing.
The diagnostic worktree was restored and removed; no production change remains.

Historical T021 already added scalar-to-boxed join widening and boxed-string
seeding; it did not preserve the union of incoming identity origins. T041's
hot-answer cache and T044's selected-stream/virtual-bank attempts are closed.
This unit changes compiler representation inference only.

## Scope and proof obligations

Follow [the frozen unit plan](performance-units/typed-loop-boxed-demand-provenance.json)
and [the performance workflow](../docs/performance-workflow.md). The lead owns
the plan, docs, integration and all serialized iMac measurements. The coding
owner changes only typed-loop compiler support and its existing test families;
an independent reviewer checks representation, ownership and deoptimization.

Carry bounded identity provenance through conditional joins and copies, then
propagate existing boxed demand monotonically. Arithmetic results retain their
computed semantics. Unknown sources, unsupported joins and resource limits stay
conservative. Use existing operations, register classes and runtime guards.
Do not modify selected-stream caches, virtual banks, normal VM dispatch, AST,
Value/Function ABI, environment snapshots, dependencies or unsafe policy.

Tests must prove actual tier entry for parameter receivers and arbitrary field
names, nested joins and copy chains. Cover mixed values, numeric-only control
flow, zero-trip/TDZ, getter/Proxy/null fallback, observable coercion and callback
mutation, receiver identity, early exits, residency and exact deopt stack/IP.
Every loop regression probe is finite. Only a passing frozen screen permits a
formal full-portfolio run; configured exact-candidate zero-gap conformance is
also required before retention.

## Candidate verification

Owner commit `dae072185b1bacc81fe7f3829a918bdb6a30766a` changes only five typed-loop
compiler/test files; scope validation passes. The compiler-only canonical union
is bounded to 64 sources and 512 unions. Copy demands grow the existing boxed
sets; computed operands stay opaque. Already-emitted opaque scalar predecessors
and new backward-edge sources remain conservative rejection boundaries.

Two regression tests fail on baseline: parameter sources are not boxed and
checksum-correct execution has zero loop entries. Twelve new tests pass on the
candidate, including actual entry, exact getter/Proxy/null guard continuation,
method prefix stack `[21, receiver, receiver]`, local writeback, callback changes,
coercion, TDZ, residency and bounded deterministic inference. Full local gate
passes 2274 runtime tests, feature/clippy checks, tool/script gates and 5170
curated cases; normal fast gate passes its 65 selected cases. CI38407657103 is
green. The 247 UTC NG fixtures and 13 additional finite owner probes match.

Independent review runs 13 public probes through raw, direct eval and arrow-IIFE
goals: 39/39 baseline/candidate/NG match. Separate raw feature traces prove
actual `Ran` or exact guard `Deoptimized`; opaque mixed and TDZ cases are only
fallback coverage. Production/build inputs and immutable binaries remain
unchanged across checks. Review finds no confirmed blocker. Raw receipts are
workspace-local `outputs/quickjs-performance-20261012/t045-independent-probes/`.

Exact coverage38408258713 records candidate `dae07218`: all 42672 configured
cases pass, with zero fail/timeout/not-run/actionable gap; NG passes42602 and the
remaining70 pass only in Rust. The full53572 comparison rows have SHA-256
`81dc4818eab013bf4223fa66c301986384cf73ade27a0b6fddfb059d43936b03`, identical to
the prior verified scan including its NG-excluded cohort. This is not a claim
that all53572 pass. The append-only conformance ledger records this candidate.

iMac job `20261013T134519Z-t045-fixed-layout-and-utc247` stabilizes in one link
with no order change. Default executable SHA-256 is
`4cc54bd1516d0bf8bf1e0f0eac9c23e2139e42578175b4fcc825c331b9138316`;
order SHA-256 `5b121ecb3de9b7b4672476522e90b974c013de86f643a075667e8ef8cd5ca3c9`.
Actual page offsets are VM`0xc40` and typed Wide`0x400`; there is no offset search
or timing result. All247 UTC fixtures pass again. Job
`20261013T135021Z-t045-dynamic-method-route` observes region15..48 `Ran` and
`TLCLAIM typed` on the unchanged original target, unlike baseline's seed decline.
Both jobs have empty disturbance logs. Feature traces remain supplemental.
