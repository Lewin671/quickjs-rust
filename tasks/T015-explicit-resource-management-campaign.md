# T015: Explicit Resource Management (`using` / `await using`)

## Goal

Implement Explicit Resource Management so `using` / `await using` declarations
dispose their resources (`Symbol.dispose` / `Symbol.asyncDispose`) at scope
exit. Unlocks ~113 QuickJS-NG-passing Test262 cases under
`test/language/statements/using` (47) and `test/language/statements/await-using`
(66), plus the `built-ins/DisposableStack` family edge cases.

## Status

- **Slice A — parser + AST (DONE, commit `b2c43dab`).** `using x = e;` and
  `await using x = e;` statement forms parse; `VarKind::{Using, AwaitUsing}`
  added with `is_lexical`/`is_immutable`/`is_using` helpers; bindings behave
  like `const` (block-scoped, immutable, TDZ) with **no disposal yet**.
  Contextual disambiguation, initializer-required, identifier-only, and the
  invalid-position rejections (labelled/single-statement body, redeclaration)
  are in place and covered by `crates/qjs-parser/src/tests/using_declarations.rs`.
- **Slice A.2 — for-head parsing (TODO).** `for (using x of e)`,
  `for (await using x of e)`, `for await (using x of e)`, and C-style
  `for (using x = e; ;)`. Reject `for (using x in e)` (for-in). Extend the
  for-head detection in `crates/qjs-parser/src/statement/control.rs:410` to
  recognize `using_declaration_kind()` and route to for-of (not for-in).
- **Slice B — sync block disposal (DONE, commit `fb470c14`).** A `{ }` block
  declaring sync `using` resources compiles into an implicit try/finally
  (`EnterDisposableScope` / `RegisterDisposable` / `DisposeScope` ops, VM frame
  `disposable_scopes`, logic in `bytecode/vm_dispose.rs`). Disposes LIFO on
  every completion path (normal/throw/return/break/continue), with
  `SuppressedError` chaining and the `Symbol.dispose` resolution/validation
  TypeErrors. Registration is gated on `Compiler::disposable_scope_depth`, so
  `using` in not-yet-wired scopes binds without crashing.
- **Slice B.2 — wider sync scopes (TODO).** Wrap the remaining statement-list
  scopes that can hold `using`: function/generator bodies, `for`-statement
  bodies, and `switch` CaseBlocks. Each needs the same EnterDisposableScope +
  implicit-finally(DisposeScope) wrapping; function/generator bodies are the
  highest-value (~3-5 cases: `initializer-disposed-at-end-of-functionbody`,
  `...generatorbody`, the top-level `throws-if-initializer-*` cases). Watch the
  function-body stack/return semantics (the hot compile path).
- **Slice B.3 — parser refinements (TODO).** Reject `using` at the top level of
  a Script / global eval (`using-not-allowed-at-top-level-of-script` /
  `...-of-eval` — currently accepted, a Slice A regression on 2 negative cases)
  and directly in a `switch` CaseClause; add `for`-head parsing
  (`for (using x of e)`, C-style, reject `for (using x in e)`). Reassignment to
  a `using` binding must be a TypeError (it is already, as a const slot).
- **Slice C — async runtime disposal (TODO).** `Symbol.asyncDispose` with an
  `await` at each disposal; per-iteration disposal in `for-of`.

## Slice B plan (sync `using` disposal)

When a lexical scope (block, function body, `for`-statement body) contains
`using` declarations, run their `Symbol.dispose` methods LIFO at scope exit on
every completion path (normal, `throw`, `return`, `break`, `continue`),
chaining failures via `SuppressedError`.

Recommended approach — reuse the try/finally machinery:

1. **VM frame state.** Add a `disposable_scopes: Vec<Vec<DisposeResource>>` to
   `Vm` (`crates/qjs-runtime/src/bytecode/vm.rs:51`), where
   `DisposeResource { value, method }`. Update `Vm::new` and the nested-call
   frame construction.
2. **IR ops** (`bytecode/ir.rs`): `EnterDisposableScope` (push an empty Vec),
   `RegisterDisposable` (resolve `Symbol.dispose` once on the resource on the
   stack — null/undefined skip, non-object or missing/non-callable dispose ->
   TypeError — and push to the current scope), `DisposeScope` (pop the scope and
   dispose LIFO, wrapping a dispose throw while already unwinding with
   `create_suppressed_error`).
3. **Compiler** (`bytecode/compiler.rs:803` block arm, plus the function-body
   and `for`-statement paths): when the scope has `using` decls, emit
   `EnterDisposableScope`, compile the body inside an implicit try whose
   `finally` is `DisposeScope` (model on `compiler_try.rs::compile_try` /
   `compile_finally`), and emit `RegisterDisposable` after each `using`
   initializer is bound. Route the dispose run through the existing finally
   mechanism so it fires on all abrupt exits.
4. Port method resolution / error-suppression from
   `crates/qjs-runtime/src/disposable_stack.rs` (sync: lines ~528-544; the
   `suppressed_error` chaining helper). Read `Symbol.dispose` once at register
   time (`gets-initializer-Symbol.dispose-property-once.js`).

Caveats: a `for`-statement resource is disposed once at loop end (not per
iteration); `using` bindings are immutable (treat as `const` slots — they are
already). Interaction with the known-fragile var-closure model is avoided
because `using` is lexical/const-like.

Estimated unlock: ~45 of 47 `using` cases.

## Slice C plan (async `await using`)

Extend `RegisterDisposable`/`DisposeScope` with an async hint: resolve
`Symbol.asyncDispose` first (fall back to `Symbol.dispose`), and suspend via
`Op::Await` on each disposal result. Per-iteration disposal for `for-of`
(`compiler_control.rs::compile_for_of` / `compile_for_await_of`). Hardest part:
LIFO disposals each individually awaited while preserving the pending
completion (throw/return) across suspension (extends `vm_try.rs::end_finally`).
Estimated unlock: ~66 `await-using` + 2 `using` cases.

## Verification

```sh
QJS_CLI_BIN="$PWD/target/release/qjs" ./scripts/test262-baseline.sh \
  --engine quickjs-rust --filter test/language/statements/using --all \
  --summary-json /tmp/u.json --no-fail
./scripts/check.sh
```

## Notes

The ~53 negative-parse cases (label-body, for-in head, binding patterns,
redeclaration, `static-init-await-binding-invalid`) currently pass and must
stay rejected — any parser change must preserve them. Slice A keeps them
rejected; re-run the using/await-using baseline before and after each slice.
