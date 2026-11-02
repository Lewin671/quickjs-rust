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
