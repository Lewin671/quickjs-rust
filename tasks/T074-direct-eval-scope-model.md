# T074: Direct eval scope model

- Status: active; approved design, implementation in progress on the T077 S3b owner branch.
- Implementation: S3b owner has canonical cell metadata, live leases/own holders and parameter/body routes; this is not integrated main. T071 runtime experiments remain removed.
- Verified at: narrowed T071 `dd312dac` constraint tests and comparisons, 2026-10-23; other main gaps are lead-reported at `6e899d24`/`2fc8efb8` (2026-10-24). Evidence is in [T071](archive/T071-lexical-context-review.md).
- Evidence: [S3b progress](diagnostics/T077-s3b-progress.md), [approved design](proposals/T077-s3-eval-scope.md), [T071 prior review](archive/T071-review-verification.md); reproductions below, lead narrow decision of 2026-10-24.
- Unresolved: eval-only visibility, declaration ownership/liveness, mutability/TDZ, deletion, mapped arguments, with resolution and lexical-context suspension.
- Next action: finish the frozen S3b lexical-owner/computed-key continuation and Context cuts, reference/policy audit and full stress/verification matrix.

## Design boundary

Preserve [binding invariants](../docs/design/env-model-rewrite.md): static names
use frame slots and indexed shared cells; dynamic maps contain cells only where
runtime name resolution requires them. No per-call name-to-value snapshots.
Specify environment-chain identity, declaration targets, lifetime, shadowing,
mutability, TDZ, deletion, parameters/body scopes, with ordering and suspension.
Define migration stages and tests before implementation. T071 owns only deferred
class-key call context; its removed eval experiment is not a starting implementation.

## Known gaps and constraints

Each snippet is independent. Main means `6e899d24`; NG is the reference. Lead
reported the results unless the linked T071 record identifies a new verification.

1. Eval-only outer capture (original T071 bug 4):
   `function outer(value){return function(){return eval('value')}} outer(8)();`
   Main ReferenceError; NG `8`. Do not pin the error as expected behavior.
2. Inherited eval map must remain live after call entry (P1-a):
   `function outer(){function inner(){eval('');return()=>b}var g=inner();eval('var b=5');return g()} outer();`
   Main/NG `5`; removed T071 fork-at-entry code throws ReferenceError.
   Function variant: replace declaration with `function b(){return 5}` and
   `return g()` with `return g()()`; main/NG `5`.
   Generator variant: `function outer(){function* inner(){eval('');yield;return b}var g=inner();g.next();eval('var b=5');return g.next().value}outer();`
   Main/NG `5`; also substitute a function declaration and call the returned function.
   Nested rebinding: `function a(){var x=1;return function b(){eval('');function c(){eval('');return()=>x}var f=c();eval('var x=2');return[f(),x]}}a()();`
   Main/NG `[2,2]`; removed T071 code `[1,2]`.
3. Capturing eval must preserve const and TDZ (P1-b):
   `function o(){const t=3;var f=function(){eval('t=4')};try{f()}catch(e){return e.name}return t}o();`
   Main `3`, NG `TypeError`, removed T071 capture code `4`.
   TDZ constraint: `function o(){var f=function(){eval('t=4')};try{f()}catch(e){return e.name}let t=3;return t}o();`
   NG `ReferenceError`; eval must not initialize the outer unentered cell.
   T071 tests only the shared invariant that the outer cell is not modified;
   the missing error on main remains open here.
4. Module call-context boundary (P1-c, fixed by narrowed T071):
   module `class D{#p;[(#p in {},String(this))]() {}}` must complete normally;
   top-level `this` is undefined. Removed T071 code created an unfilled slot
   and threw the derived-this ReferenceError. Preserve the corresponding
   `typeof arguments` absence and global receiver behavior.
5. Outer eval-declared var/function shadowing:
   `function outer(){eval('var b=1');function inner(){eval('var b=2');return b}var r=inner();return[r,b]}outer();`
   Main `[2,2]`, NG `[2,1]`. Function variant: outer eval declares
   `function b(){return 1}`, inner eval declares `function b(){return 2}`,
   and both returns call `b()`; NG `[2,1]`.
   Also cover captured ordinary parameters, prior eval reads, preexisting
   closures and strict eval; rejected fixes/evidence are in the prior review.
6. Eval var deletion:
   `function f(){eval('var x=1');return eval('delete x')}f();`
   Main does not delete the binding; NG `true` (subsequent typeof x undefined).
7. Mapped parameters through eval:
   `function f(x){eval('arguments[0]=2');return x}f(1);`
   Main `1`, NG `2`.
8. With declaration initialization target:
   `function f(){var x=1,o={x:0};with(o){eval('var x=2')}return[x,o.x]}f();`
   Main writes the var; NG `[1,2]`.
9. Same-frame const assignment:
   `function f(){const t=3;eval('t=4');return t}f();`
   Main `4`; NG TypeError.
10. Deferred-key suspension (must suspend the enclosing activation):
    `function* f(){class D{#p;[(#p in {},yield 'key')](){}}return typeof new D().m}var g=f();[g.next().value,g.next('m').value];`
    NG `['key','function']`. Async equivalent:
    `async function f(){class D{#p;[(#p in {},await Promise.resolve('m'))](){}}return typeof new D().m}f().then(print);`
    NG prints `function`. Do not run yield/await as a separate ordinary call.
11. Arrow created before lexical arguments:
    `function f(){let read=()=>arguments;let arguments=3;return read()}f();`
    NG `3`; a call-context slot must not hide the later lexical binding. Also
    call the arrow before initialization and require ReferenceError.

12. Arguments absence through nested global arrows:
    `(()=>{const read=()=>typeof arguments;return read()})()` must return
    `undefined` in an otherwise empty script realm. The branch still throws
    ReferenceError on both `6e899d24` and the narrowed branch (baseline binary
    independently verified by T071 queue `20261023T165631Z-t071-narrow-reference-corrected`).
    This requires distinguishing an absent lexical binding from an unfilled
    capture, including arrows with parameter-eval declarations.

13. Ordinary eval after a block arguments shadow:
    `function f(){ {let arguments='block';eval('arguments')} return eval('typeof arguments')} f();`
    Main returns `undefined`, NG `object`. The broad T071 arguments pre-pass
    incidentally masked this no-key gap. T071-flat restores baseline ordinary
    bytecode; this case remains open here rather than pinning the wrong result.

## Acceptance for the design

Explain why live inherited declarations and own eval declarations can coexist
without freezing the parent map or mutating an outer cell. Account for const/TDZ
checks even when a name exists only in parsed eval source. Include every case
above and the T071 regression fixtures in the proposed verification matrix.
