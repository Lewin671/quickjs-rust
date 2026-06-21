# T014: Stale `var` binding across sibling-function mutation

> **Status (2026-06-21): subsumed by `T016-environment-model-rewrite.md`.**
> The leaf-call slot-refresh fix landed and stays correct, but the root is the
> snapshot capture model. Do not extend the heuristic further — the full fix is
> T016 S2+ (shared upvalue cells). This file is kept for the repro corpus below.

## Goal

Fix the bytecode VM so that a `var`-declared binding mutated by one function is
observed by another function in the same scope, even after a sibling function
has assigned to it. Only `var` is affected; `let`/`const` (lexical captures)
are already correct.

## Minimal repro

```js
var c = 9;
function inc() { c++; }
function f() { c = 0; inc(); }
f();
// c is 0; the spec result is 1.
```

Variants confirming the trigger (all observed via
`cargo run -p qjs-cli -- -e '<src>'`):

- `let c` instead of `var c` → correct (1) in the simple sibling-call shape.
  But a related shape breaks `let` too: an outer `let values` reassigned each
  loop iteration and read by a sibling callback invoked through a native method
  desyncs — `let values; function cb(v){values.push(v);} for(...){ values=[];
  [10,20].forEach(cb); }` collects into the previous iteration's array. Same
  snapshot-staleness root; both binding kinds are affected depending on shape.
- A nested function scope instead of global scope → still wrong (0).
- `f` that only does `c = 0` (no sibling call) → correct: the realm sees 0.
- `[1,2].forEach(inc)` with no reassignment in the callback → correct (sum).
- The trigger is: a function **assigns** an outer `var`, **then** another
  function mutates the same `var`; the second mutation is lost.

This surfaced in Test262
`test/built-ins/TypedArray/prototype/{filter,map}/calls-tostring-from-each-value.js`
and `calls-valueof-from-each-value.js` (the common `var calls; ...; calls = 0`
counter idiom inside a test callback), and likely many other
counter/observer-callback cases.

It also blocks the whole `resizable-buffer-{grow,shrink}-mid-iteration` cluster
across `test/built-ins/TypedArray/prototype/*` and `test/built-ins/Array/prototype/*`
(~40 cases): those use `resizableArrayBufferUtils.js`'s `let values; ...;
values = []; view.forEach(ResizeMidIteration)` pattern, where the sibling
`ResizeMidIteration` reads the stale `values`. The element-read prerequisite for
that cluster is already done — `get_view_element` now returns `undefined` for an
out-of-bounds/detached index (IntegerIndexedElementGet) — so those cases should
fall out once this binding bug is fixed.

## Root cause

Closures use a snapshot + write-back model rather than shared cells
(`crates/qjs-runtime/src/bytecode/vm_capture.rs`,
`crates/qjs-runtime/src/function/call.rs`). Global / outer `var`s flow through
the **realm** channel, while lexical bindings flow through the shared
`captured_env` (`Rc<RefCell<HashMap>>`).

When `f` assigns `c = 0`, `store_local_or_global_sloppy`
(`crates/qjs-runtime/src/bytecode/vm_bindings.rs:511`) writes the realm **and
caches** the value into `f`'s frame-local slot. `f` then calls `inc`, which
mutates the realm cell (`0 -> 1`). `f`'s cached local slot stays `0` — the
post-call refresh (`apply_selected_env`,
`refresh_*_from_captured_env`) only reconciles the `captured_env` channel, not
the realm. On `f`'s return, `propagate_caller_bindings`
(`crates/qjs-runtime/src/function/call.rs:891`) writes `f`'s stale `0` back over
the caller's binding, clobbering `inc`'s `1`.

The fix refreshes/write-backs selected caller bindings through the same
channel that owns them: frame locals are updated after callee return,
environment-local caches are kept in sync, captured lexical bindings continue
through `captured_env`, and realm-backed sloppy/global assignments write their
final value through the shared realm only when no captured binding shadows the
name.

## Implemented direction

- Reconcile frame-local slots and environment-local caches in
  `Vm::apply_selected_env` after sub-call return.
- Register sloppy/global assignment names for caller write-back only when the
  binding is realm-backed and not captured by an intervening environment.
- Preserve captured lexical binding behavior by checking the activation and
  captured binding-source environments before using the realm write-back path.

## Scope

- Allowed paths: `crates/qjs-runtime/src/bytecode/**`,
  `crates/qjs-runtime/src/function/**`.
- Forbidden paths: `third_party/**`.
- Owner boundary: serialize on one branch — this touches shared VM binding
  code; do not run it in parallel with other runtime work.

## Acceptance Criteria

- The minimal repro and its variants return the spec results.
- A focused runtime unit test covers sibling-mutation-after-assignment for both
  global and nested-function `var`.
- No regression in `./scripts/check.sh` (5007-case subset + unit tests) and a
  broad `./scripts/find-qjsng-gaps.sh --exact --all --filter test/language`
  scan does not lose previously-passing cases.
- The four TypedArray `calls-tostring`/`calls-valueof` gap cases pass.

## Verification

```sh
cargo run -p qjs-cli -- -e 'var c=9; function inc(){c++;} function f(){c=0;inc();} f(); c;'  # expect 1
./scripts/check.sh
./scripts/find-qjsng-gaps.sh --exact --all --filter test/language
```

## Notes

Discovered 2026-06-16 while routing `%TypedArray%.prototype.toLocaleString`
through per-element `toLocaleString`. The toLocaleString change itself is
correct and landed; these residual gaps are this binding bug, not the
typed-array surface.
