# T043: Derived-constructor direct-eval context

- Status: closed; the derived-this authority migration is verified.
- Implementation: integrated at `4fd7d959` on the lead branch, including all
  three stages. T042's rejected route/counters remain absent.
- Verified at: `fbe06ea8`, 2026-10-13, full local gate with 2276 runtime,
  5170 Test262 cases and 238 script-goal NG fixtures; remote CI is green.
  The regenerated x86_64 order and 17 derived NG fixtures pass on the iMac.
- Evidence: the programs below and queue job
  `20261013T070255Z-derived-this-layout-and-regression`; raw receipts/logs are
  workspace-local under `outputs/quickjs-performance-20261012/t043-*`, not
  retained. Owner `83114d0f` is retained as diagnostic source.
- Unresolved: two separately reproduced baseline gaps below are not fixed;
  the original introducing revision was not bisected. No performance claim.
- Next action: refresh T040's comparison from the integrated source; any
  further semantic family is a separate task, not a continuation of this unit.

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
callback effects and exceptions. Rejected T042 is separate historical work;
its prepared-eval route is absent after the lead revert. This correctness
unit neither reinstates nor expands that performance admission.

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
and cell identity, not just a final allocation count. No prepared-eval
performance route belongs to this correctness unit.

## Integration and host verification

The lead retains all 18 derived semantic cases and three cell-identity cases.
Only the rejected T042 prepared-route counter assertion is removed; its
callback/throw program and expected result remain unchanged. Baseline env tests
are extracted to their own module with their bodies unchanged. T042's owned
evaluator, weak eval identity and counters are absent from the integrated tree.

The x86_64 layout stabilizes after one relink, actual VM/wide offsets
`0xc40/0x400`. Regenerated order SHA
`5b121ecb3de9b7b4672476522e90b974c013de86f643a075667e8ef8cd5ca3c9`,
release binary SHA
`b97911efb7bfc31df8bd672922b64888aa16b8c5f2cc049d60f4720e2b94a1c2`.
All 17 derived fixtures match fixed NG on that exact binary. These are address
and correctness checks, not performance qualification.

## Remaining independent baseline gaps

Both were independently reproduced at `06f0ecd3` and persist in the integrated
debug CLI. They are outside this unit's binding-identity scope:

```js
class A {constructor(){this.which='A';}}
class B {constructor(){this.which='B';}}
class D extends A {constructor(){super((Object.setPrototypeOf(D,B),0));}}
[new D().which,new D().which].join(',');
```

Rust returns `B,B`; NG script returns `A,B`. Super constructor resolution must
precede argument effects, rather than observe the prototype after arguments.

```js
var obj={this:1,method(){var before=this;eval('this.this=2');
  return eval('this')===before && this.this===2;}};
var root=globalThis;root.this=99;var ok=obj.method();
[ok,eval('this')===root,(0,eval)('this')===root,root.this].join(',');
```

Rust returns `true,false,false,99`; NG script returns `true,true,true,99`.
An ordinary global object's `this` property must not replace implicit global
this authority. The method's receiver/property mutation is correct in both.
