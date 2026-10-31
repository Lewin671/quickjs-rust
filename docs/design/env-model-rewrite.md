# Environment and binding model

The runtime uses slot-indexed locals and shared cells. This is a protected
boundary: ordinary calls must not copy a name-to-value environment. Source paths
below are under `crates/qjs-runtime/src/`; tests are under `tests/` there.
The remaining service-view migration belongs to
[T077 S3b](../../tasks/proposals/T077-s3b-brief.md).

## Compiled authority

`bytecode/binding_plan.rs` owns one immutable BindingPlan per body. The existing
compiler records declarations and parameter/body/block/catch/loop/class-name/with
boundaries. Scope IDs and binding IDs belong to the retained metadata owner;
descriptors contain names, declaration attributes and routes, never live values.
The PC map identifies the active lexical chain after normal or abrupt control
flow, including future TDZ declarations and excluding exited sibling scopes.

Parameter inputs and capture inputs are positional. Capture linking follows the
callee's received-slot order, using ParentLocal or ParentUpvalue routes. Compiler
inputs and temporaries are excluded from exposed descriptors. Destructured
parameter leaf bindings are parameters too; raw pattern inputs are not bindings.

## Live locations and invariants

1. **Private locals use slots.** `FrameState.locals[slot]` is an Option<Value>;
   None is uninitialized. The authoritative-slot mask selects direct indexed
   access. A cell route takes precedence over a legacy slot projection.
2. **Captured bindings share identity.** `binding/cell.rs::BindingCell` owns
   immutable declaration metadata and Uninitialized/Initialized state. The
   declaring frame, sibling closures and descendants retain the same Rc cell.
   Init differs from Assign; failed const/TDZ writes leave state unchanged.
   Tests: `binding::cell::policy_tests`, `closure_state.rs` captured/shadowed cells.
3. **Capture order is exact.** Producers and consumers use the same received
   classification; adding one accidental input must not shift subsequent cells.
   Tests: `bytecode/binding_plan/tests.rs`, `bytecode/upvalue_resolver.rs`.
4. **Fresh lexical entry preserves metadata.** Loop/catch re-entry replaces the
   frame's cell handle without modifying escaped cells. Fresh cells retain the
   declaration's policy and TDZ state. Per-iteration copying reads the current
   cell rather than an older slot projection. Tests: `closure_state.rs` loop cells.
5. **Arguments is a lexical binding.** Observable implicit arguments belongs to
   the real function's parameter descriptor. Parameters, body lexicals and block
   lexicals can shadow it. Arrows inherit the enclosing binding; absent arguments
   is unresolvable, not a TDZ cell. Eval-only demand also requests arguments.
6. **Mapped indices alias parameter cells.** Simple sloppy parameter mappings
   use the last duplicate position. Reverse writes and descriptor/delete severance
   preserve the specified identity; strict/non-simple arguments is unmapped.
   Tests: `parameters.rs`, `function/call_activation_tests.rs`, `s3b_bindings.rs`.
7. **Parameter/body targets are distinct where needed.** Non-simple parameters
   keep their own bindings and TDZ during defaults. Same-named body vars get
   their own locations initialized from the parameters; default-created closures
   retain the parameter location. Simple parameters share the body variable target.
8. **Global lexicals are canonical cells.** Realm binding storage holds their
   metadata/state; there is no copied lexical value projection or mutability side
   set. A global property of the same name remains independent. Ordinary global
   data routes synchronize with the global object; delete/accessor definition
   invalidates a data route and restores semantic Get, without inventing lexical
   TDZ. Tests: `global.rs`, `closure_state.rs`, `direct_eval/s3b_bindings.rs`.
9. **Intrinsics are not mutable global bindings.** Set-once realm intrinsic fields
   own Object/Array/Function/String/Number prototypes and well-known symbols.
   Rebinding a public constructor does not redefine those identities.
10. **Modules retain live cells.** Import routes attach exporting cells and reject
    writes through import access policy. Module top-level slots attach canonical
    instantiation cells. Tests: `modules.rs`, direct module frame tests.
11. **Suspension retains locations.** Snapshots own the same cells, scope leases,
    holders and selected references across yield/await, not copies of their values.
    See [generator suspension](generator-suspension.md).

## Dynamic scope and declarations

`binding/scope.rs::ScopeLease` retains an immutable descriptor, positional cells,
parent leases, optional with object and optional own VarHolder. No lease points
at a VM, frame ID, recycled window or borrowed local. A function retains its
optional lexical scope in cold state; slot-only closures need no lease.

Only runtime-added declarations use a mutable name-to-cell holder. An ordinary
activation exposes a fresh own holder at its selected variable boundary. Parent
links remain live: adding an outer declaration after child entry is visible;
creating an inner declaration never selects an outer homonym. Existing own
static bindings reuse their cells and attributes. Eval-added bindings are
separately owned and deletable. Delete removes only the selected own entry,
revealing the parent on subsequent lookup; a saved cell reference keeps its base.
Structural epochs change for add/delete/remap, never for cell-value writes.
Tests: `binding/scope_tests.rs`, `direct_eval/s3b_bindings.rs`.

The resolver walks descriptor/cells, own holder, with object and parent in lexical
order. With uses HasProperty and unscopables, releasing every storage/cell borrow
before hooks. The with-reference instructions save the selected cell/property
before RHS effects, and suspension retains that reference until consumption.

Sloppy function eval declares in the caller's selected own holder after checking
active lexical/parameter conflicts. Strict eval owns a new holder and lexical
scope. Global script/eval builders explicitly select global routes; function-shaped
eval IR must not imply a function variable environment. Global setters are not
published as data cells. Host scripts install function descriptors before the
body and preserve their non-configurable attributes.

Compiled eval caches retain plans and parse context only. Every invocation uses
its current chain; the old selected-name cached execution view is deleted.
Permanent NG fixture: `s3b-scope-live-leases.js`.

## Immutable names and imports

A named function expression compiles one self-name descriptor/metadata plan.
Each function instance owns one initialized cell in a lease outside its parameter
and body scopes. Calls and escaping children retain that cell; body vars/eval
additions shadow it in their own holder. Its write policy ignores sloppy Assign
and rejects strict Assign; const/class names always reject Assign. Named calls
never snapshot or restore a same-named caller binding. Dynamic Function names
are display names and introduce no self environment. Import routes reject Assign
without reading an exporter's possibly uninitialized cell; reads still observe
that cell's TDZ. Tests: `direct_eval/self_binding.rs`, `binding/scope_tests.rs`.

## Lexical call context and remaining compatibility

`context/call_owner.rs` separates receiver, new.target, home/super, active
constructor and field/static restrictions from declarative bindings. An ordinary
activation owns this metadata inline. At lexical retention it moves into one Rc
and the declaring activation redirects to that same owner. BindingPlan propagates
owner demand through lexical arrows, including nested new.target/eval; parameter-only
arrows retain no owner. Arguments remains a lexical binding.

Derived this is one uninitialized BindingCell in the typed owner; no declaring
slot or name-keyed mirror exists. Super constructs Base before duplicate checking,
initializes the cell before fields, preserves TDZ on Base failure and preserves
binding on field failure. An object return may bypass an unbound this. Root
builders initialize script/module receivers explicitly; a public property named
this cannot change them. Fields/static blocks establish their own receiver and
restrictions, arrows inherit them, and ordinary nested calls establish new owners.
Tests: `context/call_owner.rs`, `function/call_activation_tests.rs`,
`direct_eval/derived_this.rs`, and `class_keys.rs`.

`function/env.rs::CallEnv` still carries service/call metadata during the S3b
migration. Its typed scope cursor/entry root selects canonical cells; compiler-owned
cells are excluded from value apply-back so a stale native view cannot undo a
callback. Remaining FrameBindings/deopt views, raw compatibility publishers and
legacy eval/global-selection metadata are temporary and must disappear with Context.
Receiver/home/super/constructor/field marker strings and synthetic this capture
slots are deleted. The temporary typed construct-dispatch target is separate from
lexical new.target; B5 moves its borrowed source into NativeSite.
CreationContext/ActivationContext wrapping and the borrowed service ABI remain.
Class keys execute inline between class-region operations. A cold continuation
owns the partial class/private overlay across yield/await; handlers truncate
regions to their entry depth. Heritage validation precedes keys, each method
is installed before the next key, and static public/private fields and blocks
run together in source order. The inner-name cell remains TDZ through keys and
is initialized once before static initialization; inferred display names create
no inner lexical binding. Module entry explicitly supplies initialized undefined
this for arrow capture. Field/static helpers never own implicit arguments.
The frozen brief owns the remaining migration and deletion contract.

Compact/wide/leaf tiers retain their static cell paths. A retained dynamic chain
is not a caller-environment equality proof; current admission rejects shortcuts
that could bypass live crossing names. T077 later retires these tiers separately.

## Known limits

- Rc cycles between functions, cells, objects and private state are not collected;
  the tracing collector/arena stage owns that replacement.
- Dynamic lookup is an uncached chain walk. Do not add a name/value cache or an
  inherited map fork to avoid walking it.
