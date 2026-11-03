# T077 pre-existing semantic repairs

Scope: `agent/t077-preexisting-fixes/codex`, base `49fba8b6`. No driver/frame
architecture change, timing, origin push or main merge.

## Nested finalizers

Root cause: `EndFinally` assigned a pending jump directly to the PC instead
of routing it through the remaining exited try regions. `abrupt_jump` popped
only one region and had no destination containment check. A single set of
pending fields also let nested normal finalizers and locally caught throws
consume an outer completion. Eager for-of close/ExitTry lowering ran before
inner finalizers, closing too soon and potentially popping the wrong region.

Protected regions now carry an exclusive end; jumps unwind only exited regions.
`BeginFinally`/`EndFinally` bracket an independently retained completion per
active finalizer in the existing cold state, including across yield/await.
For-of close is a synthetic finalizer; its throw handler retains original-throw
priority. Compact/numeric/wide/typed executors decline unsupported try/finally
operations before effects; virtual analysis explicitly rejects the new marker.
No private finalizer implementation exists outside the shared VM.

Coverage: runtime `finalizer_completions` tests and five NG fixtures cover two
and three nested finalizers, break/continue and labels, return, overriding
break/return/throw, locally caught throws and contained jumps, for/in/of,
while/switch, iterator-close ordering/cancelled break, generators, async and
hot callers. The derived curated case adds the missing nested abrupt-jump
combination to the pinned upstream try-area coverage.

Pre-change try scan: 201 paths, 198 configured, Rust/NG 198 passes, 0 gaps,
failures or timeouts. Function-code pre-scan: 217 paths/configured, both 217
passes, 0 gaps/failures/timeouts. Initial concurrent NG setup failed during
CMake configuration; sequential reconfiguration/build and both scans succeeded.

Initial runtime suite: 2,759 passed (before the additional hot-caller test).
The first unrestricted-concurrency NG run hit its existing 10s timeout on
`named-property-iterative-release.js`; retry uses four jobs and a 60s case
budget. Final checks and second repair are recorded below when complete.

First-fix qualification: post-change try scan again 198/198 in both engines,
0 gaps/failures/timeouts. NG retry passes 327 scripts. Runtime 2,760/2,760.
The first fast gate exposed a test-authoring mistake: local derived cases do
not inject `assert`; replacing it with direct throws fixes the new case. The
commit hook reruns the full fast gate, including all 5,175 curated cases.

## Duplicate parameter inputs

Root cause: compact/wide entry setup seeds only supplied arguments. The old
shared parameter vector routed every duplicate position to one visible slot,
so an earlier supplied argument survived when the last occurrence was missing.
Compiler-created input routes now send shadowed occurrences to private scratch
slots and only the final occurrence to the real binding. All existing tiers
consume those routes; none has a new executor-specific binding fix. BindingPlan
excludes scratch inputs from mapped arguments and exposed declarations.

Coverage: three runtime tests, a binding-plan identity/mapping assertion, one NG
fixture and a derived hot-call case based on `S10.2.1_A3.js`. They cover missing
and supplied inputs, hot direct calls/methods/map callbacks, captured reads and
writes, mapped/unmapped argument indices, delete/descriptor severing, direct
eval, and strict/arrow/non-simple/method duplicate rejection. Seven duplicate-
parameter focused tests pass, including existing numeric/property/capture tests.
Function-code post-scan: both engines 217/217, 0 gaps/failures/timeouts.

The pinned NG has a separate direct-eval duplicate-parameter defect:
`function f(a,a){var b=eval('a');eval('a=8');return String(b)+':'+a;}`
produces `1:undefined|1:2` for `f(1)+'|'+f(1,2)`; this engine correctly gives
`undefined:8|2:8` before and after these repairs. The spec-facing runtime test
pins that behavior. The NG fixture compares eval of `arguments[1]`, for which
the oracle agrees; no runtime behavior is weakened to match the NG defect.

## Branch qualification

Source for the first repair is commit `1fabac15`; its commit-hook fast gate
passes formatting, all-target workspace clippy, size/context, runtime 2,760
and curated Test262 5,175. The second repair's final qualification follows.

NG matrix: 328 scripts plus 10 modules pass in each of default, IC-off,
IC-verify, binding-verify, forced exposure, scope poison, activation poison,
combined, combined-all and combined-off (10 modes). Assertion-enabled optimized
CLI makes verification/poison flags effective. Four jobs, 60s per script.

Deterministic semantic counters: frozen S3c public 109-input receipt, same
`--release --features perf-counters` recipe at base `49fba8b6` and the candidate,
two repetitions each, no timing. All 107 reproducible inputs have identical
stdout and every counter before/after. Totals: `executed_ops` 516,234,233 →
516,234,233; `compact_function_ops` 1,126,742,759 → 1,126,742,759. Original
clock-dependent audio-beat-detection and crypto-aes are explicitly nonrepeatable;
the receipt's fixed-clock diagnostic counterparts are among the 107. They are
retained as nonrepeatable rows, never substituted. Hot finalizer fixture records
24,436 VM ops and 1,612 compact ops, demonstrating specialized caller execution.

Verification-tool corrections: a missing qualified AST type in the first
parameter compile attempt was fixed before tests. Initial fuzz mistakenly used
the perf-counters binary: diagnostic stderr was classified as observations,
creating artificial INCONS/REGR/CHANGED rows. Its 2,000-row result is retained as
invalid qualification, with no source changes made in response. Production
fuzz reruns all 500 seeds/profile. Initial counter collector rejected the two
clock-dependent original inputs; the complete replay retains their differing
repetitions and reports reproducible inputs separately. Initial native counter
resolver omitted repository-owned internal workloads; corrected resolution uses
the same public corpus resolver and checks all frozen source hashes.

Production fuzz completes 2,000 seeds with stable source/binary inputs, no
INCONS/REGR/CHANGED/PANIC/TIMEOUT/CRASH/REFERENCE_FAILURE. Per profile:

| Profile | Seeds | MATCH | Pre-existing NG difference |
| --- | ---: | ---: | ---: |
| binding | 500 | 395 | 105 |
| binding-simple | 500 | 410 | 90 |
| receiver | 500 | 500 | 0 |
| global-store | 500 | 486 | 14 |

Candidate runtime-source manifest SHA256: `f000c96b56ba1b814e0265edc61be85a359d34a1f8bbdff915d7a7d51e929aa8`
(sorted JSON mapping each runtime `.rs` path to its SHA256). This binds
qualification to source content independently of documentation-only amendments.

The second fast-gate context check initially rejected the environment-model
document at 163 lines (budget 160); the positional-input paragraph was rewritten
in place within that budget, then the unchanged runtime was rechecked.

The unchanged original reproducer files both differ on base `49fba8b6` and
match NG exactly on the production candidate (including all four duplicate-
parameter output groups). No dependency or public API is added. The native
stack-overflow and S3d architecture obligations remain in T077.

All eight S3c stress modes pass: seven modes each 2,763 runtime tests;
combined-off 2,721 with the existing 42 IC-control skips. Full perf-counters
runtime tests pass 2,815/2,815. Production NG default and IC-off each pass
328 scripts in addition to the assertion-enabled ten-mode matrix.

The native instruction-only 109-input replay completes all three pairs per
input; [full per-input counts and recipe](T077-preexisting-instructions.md)
retain the results. This does not make a timing or performance acceptance claim.

## Changed files

Paths are relative to the repository root.

- `crates/qjs-runtime/src/bytecode/binding_boundaries.rs`
- `crates/qjs-runtime/src/bytecode/binding_plan.rs`
- `crates/qjs-runtime/src/bytecode/binding_plan/tests.rs`
- `crates/qjs-runtime/src/bytecode/compiler.rs`
- `crates/qjs-runtime/src/bytecode/compiler_binding.rs`
- `crates/qjs-runtime/src/bytecode/compiler_control.rs`
- `crates/qjs-runtime/src/bytecode/compiler_expr.rs`
- `crates/qjs-runtime/src/bytecode/compiler_try.rs`
- `crates/qjs-runtime/src/bytecode/frame_stack.rs`
- `crates/qjs-runtime/src/bytecode/ir.rs`
- `crates/qjs-runtime/src/bytecode/virtual_object/flow.rs`
- `crates/qjs-runtime/src/bytecode/vm.rs`
- `crates/qjs-runtime/src/bytecode/vm/general_ops.rs`
- `crates/qjs-runtime/src/bytecode/vm/rare_ops.rs`
- `crates/qjs-runtime/src/bytecode/vm_dispose.rs`
- `crates/qjs-runtime/src/bytecode/vm_try.rs`
- `crates/qjs-runtime/src/tests.rs`
- `crates/qjs-runtime/src/tests/duplicate_parameter_bindings.rs`
- `crates/qjs-runtime/src/tests/finalizer_completions.rs`
- `docs/design/env-model-rewrite.md`
- `docs/design/generator-suspension.md`
- `tasks/T077-core-rebuild.md`
- `tasks/diagnostics/T077-preexisting-fixes.md`
- `tasks/diagnostics/T077-preexisting-instructions.md`
- `tests/fixtures/compare-qjs/duplicate-parameter-inputs.js`
- `tests/fixtures/compare-qjs/finally-completion-overrides.js`
- `tests/fixtures/compare-qjs/finally-hot-calls.js`
- `tests/fixtures/compare-qjs/finally-iterator-close-order.js`
- `tests/fixtures/compare-qjs/finally-suspended-completions.js`
- `tests/fixtures/compare-qjs/nested-finally-jumps.js`
- `tests/test262/allowlist.txt`
- `tests/test262/cases/duplicate-parameter-hot-missing-input.js`
- `tests/test262/cases/try-nested-finally-abrupt-jumps.js`

## Final check record

All rows below qualify the runtime-source fingerprint above. The final HEAD
full gate is rerun after the documentation-only amendment; no runtime source
or binary changes follow qualification.

| Check | Result |
| --- | --- |
| Bootstrap; sequential NG configure/build | Pass after the setup race described above |
| `cargo check -p qjs-runtime`; formatting | Pass after correcting interim compile/test-authoring errors |
| Focused finalizer / duplicate-parameter tests | Pass; nine new runtime tests plus binding-plan assertions |
| Original two reproducers, production vs NG | Exact match; base differs on both |
| Both commit-hook `check-touched.sh --staged --explain` gates | Pass; runtime 2,760 / 2,763, curated 5,175 / 5,176 |
| `./scripts/check.sh` | Pass: all three clippy configurations, workspace/runtime 2,763, agents 21, counter diagnostics 26, benchmark tools 767, script/context/size/hooks/plans and curated 5,176 |
| `cargo test -p qjs-runtime --features perf-counters` | 2,815 passed |
| Try/function-code gap scans before and after | Both engines 198/198 and 217/217; zero gaps/failures/timeouts |
| NG ten-mode matrix; production default/IC-off | 328 scripts + 10 modules per matrix mode; production 328 scripts per mode |
| S3c seven stress modes; combined-off | 2,763 each; 2,721 with existing 42 skips; zero failures |
| Public correctness, production and counter builds | Each 351/351 across default/IC-off/assertion-enabled IC-verify |
| Binding fuzz, production six-mode comparisons | 500/profile, 2,000 total; 1,791 MATCH / 209 PRE, no new regression/mode failure |
| Frozen public109 semantic counters, two repeats | 107 reproducible inputs: every counter and stdout unchanged; two clock-dependent originals retained |
| Frozen public109 native instructions, three pairs | Complete; every source hash matches; per-input counts linked above; no timing metrics retained |
| Context/link/size checks and `git diff --check` | Pass; no new dependency, public API or architecture shift |

Interim compile failures were field-pattern/signature/test-value construction
mistakes during the local implementation, corrected before qualification.
The final status label was also corrected to the required `active` prefix.
Failed setup, timeout, context and instrumentation attempts remain documented
above; they are not counted as passes or new engine exclusions.
