# Pre-existing semantic repairs: lead feedback round 2

Branch `agent/t077-preexisting-fixes/codex`, round-2 start `a2b51693`, review base
`49fba8b6`. The second review found one regression introduced by `a2b51693`
(catch UpdateEmpty on break), three older completion-value divergences, the
for-using operand-stack growth and await-using disposal order. Completion
values had been patched per statement twice, so this round replaces the model
instead of patching a third form. No origin push, main merge or timing.

## Commits

| Commit | Change |
| --- | --- |
| `d2e77d19` | One completion-register model for every statement form (A, B, C, D, F growth) |
| `d58af31d` | Debug-assert operand-stack depth at every protected-region exit; with/await-using depth fixes |
| `5d67cfc7` | `stage_abrupt` assertion, `discard_finalizers`, linear `route_jumps`, header wrap |
| `70ac7db6` | Await each async dispose before the next; SuppressedError chaining (E) |
| `7a7cbc7e` | Wide tier enters inner loops under an interpreter-only plan region |
| `e3db17fd` | Typed-loop entry remembers a plain array for its hole path |
| `754f32b3` | Typed loops never create an indexed property on a non-array object |

The last two repair the instruction-count regression the completion model
exposed; see [Instruction counts](#instruction-counts).

## Completion model (A, B, C, D)

Root cause: each block, loop, switch, label and try kept its own result slot,
and values were copied between slots. A catch shared its try's slot, a finally
block's statements wrote the enclosing loop's slot (B), and break/continue
pushed the value they carried, so a break after an `if`/`with` that had reset
its own slot picked up the stale outer value (A), and a break across a
label-only block copied in the wrong order (C). An empty labelled block wrote
`undefined` (D).

Now `Compiler::completion` is the single owner of the current completion
value (ES2025 §14 statement semantics, §6.2.4 UpdateEmpty):

- Blocks and labels share the enclosing register. `if`, `with`, `try` and
  `catch` reset it to `undefined` unless their body writes it first
  (UpdateEmpty(C, undefined)); loops and switch always reset it.
- A finally block gets its own register only when a break/continue leaves it,
  so its normal completion never reaches the enclosing value.
- break/continue carry no value; a jump whose innermost register differs from
  its target's goes through a stub that copies it.
- Statements leave nothing on the operand stack; declarations, empty blocks and
  empty statements complete empty.
- Function bodies have no register (`completion` is `None`) and emit no
  completion work. The dead-completion machinery in the string-append fast
  path, typed-loop compiler and compact tier is removed.

## For-using growth and region depth (F)

Root cause: a for-using loop pushed its completion value twice per exit, and a
break whose value a finally overrode left the old value behind; both grew the
operand stack inside one activation. With statements now stack-neutral, every
region exits at its entry depth. The VM debug-asserts that at each `ExitTry`
and for every region an abrupt jump leaves (a jump, unlike throw/return, never
truncates). The for-of rethrow handler is the one allowed +1. The assertions
found two more imbalances: the with region was opened after its object was
pushed (it now opens before; abrupt exits still truncate the with stack), and
the await-using disposal finally left the awaited value on the stack.

## Await-using disposal order (E)

Root cause: an `await using` scope called every asyncDispose up front and
awaited only the last result, so disposals interleaved and earlier rejections
were dropped. DisposeResources requires each result to be awaited before the
next resource and every throw chained into a SuppressedError. The disposal
finally is now a bytecode loop: `DisposeAsyncStep` disposes until one result
needs an await, the finally awaits it in its own catch region, and
`DisposeAsyncRejected` chains a rejection before continuing. The scope keeps
needsAwait/hasAwaited across suspensions. Sync scopes keep `DisposeScope`.
Reviewer repro: `b1,b2,a1,a2 | S(Ea,Eb)`, as node and NG.

## Code quality

`stage_abrupt` debug-asserts no staged completion; `discard_finalizers`
replaces `unwind_finalizers(usize::MAX)`; `route_jumps` is one sweep with a
stack of open regions (nesting debug-asserted) instead of jumps × regions; the
with-region depth excludes the consumed object; the long header line in
`compiler_control.rs` is wrapped.

## Instruction counts

The [round-2 replay](T077-preexisting-round2-instructions.md) of public109
(`a2b51693` vs `e3db17fd`, plain release builds, instructions only) gives a
geomean of **0.973595871** over the 107 reproducible inputs, sum
537,502,946,775 → 526,721,613,978, all stdout identical.

At `70ac7db6` the same replay gave 0.980022797 with `access-nsieve` at 1.2498.
Cause: base `if` false paths carried dead completion stores, so the outer sieve
loop did not match the predicate-scan plan (`vm_numeric_mutation_loop`). Without
them it matches (and declines at run time on boolean elements), and its region
made the wide tier refuse the inner loop as "claimed by an enclosing plan" —
a guard meant for frames that replay plans, which the wide tier never does.
`7a7cbc7e` limits the guard to frames that run plans. The outer loop then runs
typed in the interpreter frame, whose hole-path check (`array_access_is_plain`)
was uncached, unlike the wide frame's; the sieve's array keeps holes at 0 and
1, so every store paid it. `e3db17fd` remembers the last plain array per entry
(no user code runs inside a program). `access-nsieve` is now 0.9839 and
`bitops-nsieve-bits` 0.9641.

Remaining ratios above 1.005: `broad/many_locals_call` 1.0469,
`sentinel/prototype_method_call` 1.0100, `kraken-1.1/audio-dft` 1.0058. Their
hot functions emit no completion work. For `many_locals_call` the per-iteration
change flips sign with the callee's local count (plain release builds, 200,000
calls: no locals 72.72M → 65.39M, 4 locals 135.81M → 137.35M, 32 locals
796.36M → 786.29M), with identical typed regions and compact programs, which
points to code layout, not executed work. `prototype_method_call` executes
fewer interpreter ops (219 → 119) with unchanged per-iteration counters.

## Typed-loop prototype index creation

Root cause: whether an array's element accesses are plain (realm
`Array.prototype`, no own indexed property on the chain) is remembered for a
whole typed-loop entry, by the executor for its last plain array (`e3db17fd`)
and, already before this round, by the wide frame for the chain.
`ComputedWrite`'s ordinary-object arm could create an index key mid-entry:
`for (...) { t += a[3] | 0; p = os[i]; p[k] = 9; }` with `os[3] ===
Object.prototype` and `k = '3'` returns 0 instead of node/NG's 36, in both the
interpreter frame and the wide tier.

Rule (refuse, not invalidate): `ComputedWrite` hands back before it would
create an own array-index property; overwriting one stays a plain write
(`write_or_create_computed_data_key_without_index_creation`). The typed_loop
module header states the invariant those answers rely on. Audit of the
admitted op set: `SetNamed`/`SetNamedTyped` only overwrite (`write_hot` never
appends); element stores and `ArrayPush` touch arrays only, and an array on a
chain is already a hazard (`ArrayPush` also checks live); stores into
`Array.prototype` and `__proto__` decline; delete, private stores and
prototype changes are not admitted; calls run only numeric closed forms and
natives that add no property to an existing object; keys >= 2^32-1 are names.
No other operation changes the cached facts.

Evidence: unit tests for both frames (`typed_loop/tests.rs`,
`tests/diagnostics.rs`) and for the declining write fail at `c6d8bff2` (0) and
pass now; fixture `typed-loop-prototype-index-creation.js` gives
`0:0:45:0:45` at the base and `0:36:45:36:45`, as node v24 and NG, now.
Further variants (constant `'3'` and `3` keys, store before read, an array
grown by `length`, a custom chain object, `'3.0'`) match node and NG; four of
them were wrong at the base. Instructions only (plain release builds of
`git archive` exports, frozen public109 inputs, median of three, AB/BA/AB,
stdout identical): `access-nsieve` 281,072,096 → 281,045,323 (0.99990),
`bitops-nsieve-bits` 332,311,552 → 332,240,181 (0.99979).

Checks at `754f32b3`: pre-commit fast gate (2,780 runtime tests, 65 Test262
cases); `./scripts/check.sh` exit 0 with 2,780 runtime tests, 28 counter
diagnostics, 21 agent tests, 767 benchmark-tool tests, 35 script tests, all
three clippy configurations and 5,176/5,176 curated Test262; NG comparison of
339 scripts + 10 modules in all nine env modes (default, IC-off, verify-IC,
verify-bindings, force-eval, poison, activation-poison, combined-all,
combined-off) on both debug and release builds, 0 diffs. The matrix below is
unchanged from `e3db17fd`.

## Verification matrix

All at HEAD `e3db17fd` unless stated. Every code commit passed its pre-commit
fast gate (format, clippy, context/size, runtime tests, 65 selected Test262
cases): 2,774 / 2,775 / 2,776 / 2,777 / 2,777 / 2,778 runtime tests.

- `./scripts/check.sh`: exit 0; 2,778 runtime tests, 27 counter diagnostics,
  21 agent tests, 767 benchmark-tool tests, 35 script tests, all three clippy
  configurations, 5,176/5,176 curated Test262.
- NG comparison: 338 scripts + 10 modules in all ten S3c modes, exit 0
  (assertion/overflow-enabled release build).
- Stress: seven modes × 2,778 tests; combined-off 2,736 plus the 42 IC-control
  skips; exit 0.
- Public correctness: 86 cases, 258 case-modes, 351/351 invocations.
- Binding fuzz, 500 seeds per profile: 2,000 cases, MATCH 1,791, PRE 209
  (binding 395/105, binding-simple 410/90, receiver 500/0, global-store
  486/14), no new regression or mode failure, stable inputs.
- Reviewer completion fuzzer (`compl.py`, rel and dbg-all): 9,000 programs,
  9,000 ok, 0 new/changed/pre/mode-split divergences from node.
- Reviewer `using` generator: 252 async programs and 252 sync programs, 0 diffs
  in five modes. The F repro differs from node only because node lacks
  `scriptArgs`; Rust equals NG.
- Control-flow fuzzer (`fuzzdrive.py`, modes rel, rel-off, dbg, dbg-all,
  dbg-off): `fuzz.py` sync seeds 150000–150019, gen 160000–160019, async
  170000–170009; `fuzz2.py` sync 180000–180019, gen 181000–181019, async
  182000–182009. 20,000 programs, 100,000 observations, 0 mismatches.
- `find-qjsng-gaps.sh --all --filter test/language/statements/<area>`, before
  (`a2b51693`) and after (HEAD): identical, no NG-pass/Rust-nonpass gap.

| Area | Rust pass | NG pass |
| --- | ---: | ---: |
| try | 198 | 198 |
| with | 181 | 175 |
| labeled | 23 | 23 |
| break | 20 | 20 |
| continue | 24 | 24 |
| for | 381 | 381 |
| for-in | 115 | 115 |
| for-of | 751 | 751 |
| for-await-of | 1,234 | 1,234 |
| switch | 108 | 108 |
| block | 19 | 19 |
| if | 67 | 67 |
| using | 78 | 78 |
| await-using | 94 | 94 |

## Open

- Pre-existing: function declarations in a switch case are not instantiated at
  CaseBlock entry: `switch(1){case 0: function g(){} case 1: typeof g}` throws
  ReferenceError; node gives `"function"` (ES2025 §14.12.4,
  BlockDeclarationInstantiation of the CaseBlock). Same at `a2b51693`.
- Pre-existing (P2): AsyncIteratorClose does not await the `return()` result
  (ES2025 §7.4.13 step 5.d), so `break` out of `for await` runs the async
  generator's `finally` after the post-loop code.
- Pre-existing (P3): a function body's top-level expression-statement values
  stay on the operand stack until return; RSS grows with statement count, no
  wrong result.
- `using` directly in a case clause is a SyntaxError in both engines; only the
  message differs.

## Changed files

Runtime: `bytecode/{compiler,compiler_control,compiler_expr,compiler_regions,
compiler_try,compiler_values,util,ir,vm,vm_dispose,vm_try,vm_string_append}.rs`,
`bytecode/vm/{general_ops,rare_ops}.rs`, `bytecode/virtual_object/{flow,lower}.rs`,
`bytecode/typed_loop/{compile,execute,frame,mod}.rs`, `value/object/data_write.rs`,
`bytecode/compact_fn/wide/{compile,loop_frame}.rs` (all under
`crates/qjs-runtime/src/`).

Tests: `tests/abrupt_statement_completions.rs` (four completion tests),
`tests/protected_scope_exits.rs` (region depth), `tests/disposable_stacks.rs`
(sequential await-using), `tests/diagnostics.rs` (inner loop on the wide tier),
`bytecode/typed_loop/tests.rs` (plain-array memo, index creation in a loop), and a `route_jumps`
nested-region unit test. Fixtures in `tests/fixtures/compare-qjs/`:
`catch-break-updateempty-completion.js`, `finally-normal-completion-isolation.js`,
`labelled-block-break-completion.js`, `empty-statement-completion.js`,
`for-using-exit-stack-balance.js`, `await-using-sequential-disposal.js`,
`typed-loop-prototype-index-creation.js`.
