# Numeric result liveness and evaluation effects

This fix is independent of typed Context/receiver changes. Baseline `3cc12c6f`
already deletes a discarded global call or swallows its throw
([base reproducer](base-before.log)); S3b exposes the same optimizer through
additional closure/native-callback entries. A call's unread destination is not
proof that evaluating it is pure. Removing the Call also erased required callee
validation, so an ineligible/effectful caller could incorrectly become a leaf.

`typed_loop/helper_graph/numeric/effects.rs` now owns an exhaustive effect
classification. Only proven scalar operations can be deleted by result liveness.
Calls, natives and unmodeled operators retain evaluation/admission obligations;
control flow/Bail remains. Scalar arithmetic is over encoded numbers/booleans/
undefined, not boxed Values: it cannot reach getters, proxies, user coercions or
TDZ. Other operators are conservative unless specifically proven pure. New NumOp
variants require an explicit effect decision. This does not disable DCE.

## Pass audit

| Pass | Effect boundary checked |
| --- | --- |
| Shared numeric optimizer | DCE consults Effect; constant hoisting only moves Const; copy propagation keeps producers and call argument ranges; result forwarding retains the producer exactly once, in place; branch fusion only consumes encoded scalar comparisons/bitwise operations |
| Compact numeric-plan lowering | Uses the same optimizer; callees must still resolve to valid pure numeric plans before running. Unsupported property access/coercion becomes Bail or declines; unreachable code is removed only by CFG proof |
| `typed_loop/forward.rs` | Deletes Move/MoveBoxed copies or ToNumeric on the scalar Typed enum (Number/Boolean/Undefined). Boxed coercions are not that operation. Retargeting retains producers and deopt-site liveness |
| `typed_loop/hoist.rs`, `pure_calls.rs` | Invariant reads require ordinary data properties; accessor/Proxy/exotic reads decline. Call-hoisting guards allow only named pure intrinsics on scalar/primitive inputs and exclude random, user/bound functions, callbacks and writes |
| Compact register lowering | Pop emits Drop; the producer remains |
| Wide lowering and peepholes | Only unused constants and inaccessible completion copies are elided. Updates/coercions remain, aliases materialize at call/exit/write barriers, and retargeted reads/operators still execute at their original point |

Regression controls cover discarded getters, Proxy gets, valueOf/Math coercions
and throws, including 32-iteration loops. No property read, user conversion or
potential abrupt completion is declared pure from destination liveness.

## Verification

The initial **24 tests** fail **23** before the fix; the getter/coercion controls
already pass. The final **26 tests** all pass, adding positive pure-DCE and
conservative native/unmodeled-operation controls ([before](before.log),
[after](after.log)). Separate effect and throw tests cover direct, call, apply,
Reflect.apply, bound, sort, toSorted, typed sort/toSorted and Array.from routes;
local closures and Promise reactions also preserve effects/throw identity.
The permanent script fixture covers discarded calls and the module fixture
checks Promise jobs: old candidate `0:2 / false`, fixed/NG `1:2 / true`
([before jobs](jobs-before.log), [after jobs](jobs-after.log)).

For cherry-pick independence, a source archive of main **ed022fa1** receives
only the three numeric optimizer/test files, with no Context/receiver/mask
changes. Its same **26 focused tests pass** ([main proof](main-proof.log)).
[Metadata](metadata.json) binds the exact overlay and raw logs. The archive and
build outputs stay under the current worktree's target directory; main is never
modified. No full/exact requalification or timing was run for this proof.
