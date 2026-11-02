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
