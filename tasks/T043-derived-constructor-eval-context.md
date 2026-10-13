# T043: Derived-constructor direct-eval context

- Status: active.
- Implementation: none; two baseline semantic gaps are reproduced.
- Verified at: exact `06f0ecd3`, 2026-10-12, freshly compiled local debug CLI
  versus clean fixed NG `f7830186`. These are correctness runs, not timings.
- Evidence: minimal programs and outputs below. Raw scripts/JSON are workspace-
  local under `outputs/quickjs-performance-20261012/base06-correctness`, not
  retained in this repository. Base executable SHA
  `622ec1add4f750cd14b7ef762b8072eaaf11b3260c2ea84afd8f5eddbb62dbed`.
- Unresolved: propagation of implicit `this` initialization and the derived
  `this` TDZ through the existing native eval route; introduction revision.
- Next action: trace the original eval/frame/constructor context, add focused
  differential coverage and fix separately from T042's read-only ownership unit.

## Reproductions

```js
class B { constructor() { this.x = 4; } }
class D extends B { constructor() { eval('super()'); } }
[new D().x, new D().x].join(',');
```

NG returns `4,4`; the exact baseline throws
`ReferenceError: super constructor may only be called once`.

```js
class B {}
class D extends B {
  constructor() {
    var ok = [];
    try { eval('this'); } catch (e) { ok.push(e instanceof ReferenceError); }
    try { eval('this'); } catch (e) { ok.push(e instanceof ReferenceError); }
    super();
    this.ok = ok.join(',');
  }
}
[new D().ok, new D().ok].join('|');
```

NG returns `true,true|true,true`; the exact baseline returns `|`. Thus both
cold and warmed eval fail to preserve the pre-super `this` TDZ. A fuller
two-instance event probe records first and repeated `super()` completions:
baseline `first:ReferenceError|second:ReferenceError` per instance, NG
`first:ok|second:ReferenceError`. Counting only allocations or constructor
side effects hides this difference and is not adequate coverage.

These failures occur before T042's new implementation. That exact-base
reproduction does not identify their original introducing commit; do not
ascribe them to an earlier architectural change without a bisect.

## Scope and boundary

Trace `global.rs` native eval, `bytecode/vm_call.rs` caller projection and
writeback, `vm_frame_init`/binding/`this` handling and derived construction.
Preserve per-activation `this` identity, shared cells, single-super rules,
callback effects and exceptions. T042 conservatively excludes direct
SuperCall/Spread effects and uninitialized derived-this contexts; its ordinary
read-only callback coverage must still preserve shared-cell effects.

The configured Test262 snapshot's zero-gap result does not establish these
untested programs are correct. Add cases that assert each completion and `this`
state, not merely a final allocation count, then run focused Test262 and NG
comparisons plus the full correctness gate.

## Target architecture and migration stages

The existing `FrameBindings["this"]` Upvalue is the authority for each derived
constructor activation. It starts with the existing uninitialized marker and
is shared by the constructor, its eval projections and lexical arrows. No
`CallEnv` hot field or per-call name/value snapshot is introduced.

1. Create the cell at derived activation; attach the declaring VM slot and
   captures to it. Forks/projections preserve only this implicit derived cell's
   identity, and stale value refresh/writeback cannot replace it.
2. Reads of the marker throw rather than fall back to realm this. Native eval
   preserves the same cell even in strict mode. Super executes arguments and
   Base first, then checks uninitialized state, binds, and initializes fields.
   Base failure leaves TDZ; field failure preserves the successful binding.
   Completion rejects an uninitialized marker, while an explicit object return
   remains legal.
3. An optional FunctionColdState lexical context retains the cell and original
   constructor owner for arrows, including nested/escaping arrows with no this
   upvalue slot. Calling such an arrow restores that lexical authority; live
   super lookup and field ownership cannot be replaced with new.target.

Owner runtime paths: `function/{env,value,call}.rs`, `global.rs`,
`bytecode/{vm_bindings,vm_capture,vm_frame_init,vm_call,vm_class,vm_result}.rs`
and `bytecode/vm/rare_ops.rs`, plus focused helper/test modules. Shared design,
allowlists and benchmark plans remain lead-owned. This is a correctness fix,
not a new T042 optimization or a performance admission expansion.

Coverage must check cold/warm eval, captured/nested/escaping arrows, multiple
instances and reentry; pre-super this/typeof/super-property TDZ; duplicate
super argument/Base effects with no repeated fields; Base and field throws;
callback-super through read-only eval on success and throw; strict declaration
isolation, indirect/global/module this and legitimate undefined this; and
Reflect.construct with a distinct new.target. Assert completion, event order
and cell identity, not just a final allocation count. T042 remains ineligible
before binding and may admit ordinary read-only eval after binding.
