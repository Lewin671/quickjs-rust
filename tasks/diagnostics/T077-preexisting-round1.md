# Pre-existing semantic repairs: lead feedback round 1

Base: `4596f149`, branch `agent/t077-preexisting-fixes/codex`. The lead's
independent review found no incomplete repair/regression in the initial two
commits. This follow-up owns four additional base bugs and the requested
code-quality changes. No origin push, main merge or timing qualification.

## Pending completion and retention

The handoff to `BeginFinally` is one `Option<PendingAbrupt>`; active finalizers
continue owning independent completions. Removed the three parallel options and
unnecessary clears. Suspension tests retain the actual finalizer stack, and
pool tests charge its capacity as well as checking released values. Both
region-entry operations assert their ends were patched. Try/finalizer containment
shares `CodeRegion::contains`; finalizer-end patches have a separate named API.
Disposal uses `take_if`; mapped-parameter plans no longer rescan duplicate slots.

Initial follow-up runtime tests and the quality commit's fast gate qualify this
refactor; semantic repairs and final qualification are recorded below.

## Qualification scope

All four semantic repairs and the required qualification are complete below.
The matrix includes before/after filtered gaps, both reproducers
and focused fixtures, the S3c stress/NG/public/binding-fuzz matrix, the reviewer
control-flow fuzzer, and public-receipt instruction-only replay.

For-await awaiting iterator return results and native-stack overflow remain
outside this follow-up, owned by the existing scope/S3d records. NG differences
on let-loop continue/finally and duplicate-parameter direct eval must not be
copied into the implementation or expected fixture output.

## Region-owned with exits and using-loop cleanup

Quality refactor `ec4f7b59`: runtime 2,763 passes; commit fast gate passes,
including 65 selected Test262 cases.

With root cause: compiler-emitted ExitWith ran before intervening finalizers,
so their binding checkpoints tried to materialize a with lease without its
object. A synthetic protected region now surrounds each own with body; its
unwinding restores the entering with depth after all inner finalizers. Normal
flow still executes ExitWith. Captured with objects are never exited by a
callee's return.

For-using root cause: abrupt routing depended on the separately maintained
try-result stack, which the head-disposal wrapper never registered. All
break/continue sites now start as AbruptJump; a compiler finalization pass
selects ordinary Jump only when emitted EnterTry/BeginFinally ranges prove no
region is exited. Completion-value tracking no longer controls cleanup. The
obsolete iterator/with exit bookkeeping is removed from loop contexts. This
single region correction fixes both reported bugs as one semantic family.

Coverage: region-routing unit test; four runtime tests for all with abrupt
forms, nested scopes, suspension, immediate labeled disposal and 100,000
repeated using exits; two NG fixtures. Review run.py passes all three original
with/using reproducers in debug, combined stress and combined-off (nine runs),
including growth reporting 100,000 disposals before return. No RSS/timing
measurement was used to establish the repair.

## Unlabeled break targets

Root cause: an unlabeled break selected the last loop-stack entry even when
that entry represented a label-only block. Loop contexts now explicitly record
whether they accept unlabeled breaks: iterations/switch do; label-only blocks
do not. Explicit labeled break and continue retain their existing lookup.
A runtime test/NG fixture covers while/for/switch, intervening finally, nested
labels, and SyntaxError when no iteration/switch can receive the break.

The initial label test incorrectly expected the Rust parser diagnostic string
to contain the JS error class name. The focused test and first commit gate
rejected that assertion (2,768 other tests passed). It now checks the structured
parse rejection directly; the JS routing assertion already passed.

## Catch statement-list completion

Root cause: catch statements accumulated their completion into an isolated
block temporary and copied it into the try result only on normal block exit.
A break/continue read the still-undefined try result first. Catch now shares
its try's live statement-list accumulator through the existing block compiler,
including empty/declaration UpdateEmpty handling and lexical instantiation.
No finalizer completion is folded into the catch value.

A runtime test/NG fixture covers the three original 6|6|6 cases, continue,
empty statements/declarations, optional catch binding and object/array catch
patterns, including a labeled exit through an intervening label-only block.

## Verification matrix

The candidate runtime-source manifest SHA256 is
`dbfedca78487da4957aed2ad40b7b7d2a12449cb8d7301de74f90c1d3020d0f5`
(sorted JSON mapping every runtime `.rs` path to its SHA256). The source and
all copied binaries remain unchanged through qualification.

Before/after gaps use the preserved `4596f149` debug binary and the candidate
debug binary, with `find-qjsng-gaps.sh --filter <area> --all`. All configured
Rust cases pass both times; there are no NG-pass/Rust-nonpass gaps or timeouts.
The six NG with-area failures predate the repair and remain unchanged.

| Statement area | Rust before/after | NG before/after passes |
| --- | ---: | ---: |
| try | 198 / 198 | 198 / 198 |
| with | 181 / 181 | 175 / 175 |
| labeled | 23 / 23 | 23 / 23 |
| break | 20 / 20 | 20 / 20 |
| continue | 24 / 24 | 24 / 24 |
| for | 381 / 381 | 381 / 381 |
| using | 78 / 78 | 78 / 78 |
| await-using | 94 / 94 | 94 / 94 |
| for-await-of | 1234 / 1234 | 1234 / 1234 |
| for-of | 751 / 751 | 751 / 751 |
| for-in | 115 / 115 | 115 / 115 |

The review's unchanged `fuzz.py` generator and `run.py` execution/normalization
helpers, following `fuzzrun.sh`'s generate-and-compare flow, produce 100 batches
of 200 programs: sync seeds 10000–10039, generator 20000–20039, async 30000–30019.
Each batch runs under Node v22.23.3, NG and all five reviewer candidate modes
(release, release IC-off, debug, debug combined-all, debug combined-off). All
20,000 programs yield all 200 observations/batch; 100,000 candidate observations
agree with Node, NG also agrees, with stable sources/binaries and zero crashes,
timeouts or divergent batches. Original inputs/outputs remain local artifacts;
these counts, seed ranges and producer identities are the retained conclusions.

Reviewer producer SHA256:

- `fuzz.py`: `ccb6a3f18fe17914b4916b7c9cb7d62611197a9129718446006cc4e8f7abd81c`
- `run.py`: `6515705bfddae032763fd358025caeb6406e4ab837bc99ccc778e55fe06253e2`
- `node-run.js`: `9b87c6e8000dbcacdba6d72727550ed0f621e1690f0e28120b20c460a7dca852`

All five original reproducers pass review `run.py` in its five modes: 25 runs,
0 diff files versus NG. This includes the 100,000-iteration growth reproducer.
Binding fuzz (`python3 -m tools.binding_fuzz.run --count 500 --jobs 4`, all four
profiles, base binary `4596f149`) completes 2,000 seeds: 1,791 MATCH / 209 PRE;
no regression, changed result, inconsistency, panic, crash or timeout. Profile
counts are binding 395/105, binding-simple 410/90, receiver 500/0, global-store
486/14 (MATCH/PRE). Existing differences are not counted as engine agreement.

NG matrix: 332 scripts plus 10 modules in each of all ten S3c modes, using an
optimized assertion/overflow-enabled build for effective debug checks. The
standalone public correctness sweep completes 351/351 invocations (production
default, production IC-off, assertion-enabled IC-verify), with no new exclusion.
Full perf-counters runtime tests complete 2,822/2,822.

[Public109 instruction-only and semantic counters](T077-preexisting-round1-instructions.md)
record both builds, all input medians and the two clock-dependent original
inputs. No elapsed-time data or performance acceptance is claimed.

## Final gates and handoff

All four code commits pass their commit-hook fast gates (format, all-target
workspace clippy, context/size, full runtime tests and 65 selected Test262 cases
each): quality 2,763 tests; protected exits 2,768; break targets 2,769; catch
completion 2,770. The failed initial label assertion and its correction are
recorded above; no failing code commit was created. No allowlist, exclusion or
expected-failure change was needed for the four new fixtures.

`./scripts/check.sh` passes: all three clippy configurations; 2,770 default
runtime tests plus the other workspace tests/doctests; 21 agent tests; 26
counter diagnostics; 767 benchmark-tool tests; 35 script tests; context, file
size, git-hook isolation, shell syntax and plan validation; all 5,176 curated
Test262 cases. The full gate is repeated on the final documentation-amended
HEAD. No runtime source or binary changes follow qualification.

The S3c stress matrix passes seven modes with 2,770 tests each and combined-off
with 2,728 tests plus the same 42 IC-control skips. All command exits in the
NG/public/fuzz/stress/counter matrix are zero. `git diff --check` and context
validation pass. There is no new dependency, public API, driver architecture
change, push or main merge. For-await iterator-return awaiting, native-stack
overflow and the two known NG/spec disagreements remain outside the repair.

## Changed files

Paths below are relative to the repository root. Seven new Rust tests (six
runtime and one compiler), four NG fixtures, and stronger pool/suspension
assertions cover the changes.

- `crates/qjs-runtime/src/bytecode/binding_metadata.rs`
- `crates/qjs-runtime/src/bytecode/binding_plan.rs`
- `crates/qjs-runtime/src/bytecode/compiler.rs`
- `crates/qjs-runtime/src/bytecode/compiler_binding.rs`
- `crates/qjs-runtime/src/bytecode/compiler_control.rs`
- `crates/qjs-runtime/src/bytecode/compiler_regions.rs`
- `crates/qjs-runtime/src/bytecode/compiler_try.rs`
- `crates/qjs-runtime/src/bytecode/frame_pool/tests.rs`
- `crates/qjs-runtime/src/bytecode/frame_stack.rs`
- `crates/qjs-runtime/src/bytecode/mod.rs`
- `crates/qjs-runtime/src/bytecode/vm.rs`
- `crates/qjs-runtime/src/bytecode/vm_dispose.rs`
- `crates/qjs-runtime/src/bytecode/vm_generator/owner_tests.rs`
- `crates/qjs-runtime/src/bytecode/vm_try.rs`
- `crates/qjs-runtime/src/tests.rs`
- `crates/qjs-runtime/src/tests/abrupt_statement_completions.rs`
- `crates/qjs-runtime/src/tests/protected_scope_exits.rs`
- `tasks/T077-core-rebuild.md`
- `tasks/diagnostics/T077-preexisting-round1-instructions.md`
- `tasks/diagnostics/T077-preexisting-round1.md`
- `tests/fixtures/compare-qjs/catch-abrupt-completion-values.js`
- `tests/fixtures/compare-qjs/for-using-labeled-cleanup.js`
- `tests/fixtures/compare-qjs/unlabeled-break-through-labels.js`
- `tests/fixtures/compare-qjs/with-abrupt-finalizer-scope.js`
