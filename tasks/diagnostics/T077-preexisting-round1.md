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

## Scope still open

The four reported base bugs are with-scope abrupt exits, for-using labeled
cleanup, unlabeled break through a labeled block, and catch break completion
values. Final reporting requires before/after filtered gaps, both reproducers
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
