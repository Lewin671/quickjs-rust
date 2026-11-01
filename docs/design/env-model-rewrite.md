# Environment and binding model

The runtime uses slot-indexed locals and shared cells, a protected boundary:
ordinary calls must not copy a name-to-value environment. Paths are under
`crates/qjs-runtime/src/`, tests under `tests/` there ([T077 S3b](../../tasks/proposals/T077-s3b-brief.md)).
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
   access. Capture/eval exposure moves value or TDZ into its cell and clears the slot.
2. **Captured bindings share identity.** `binding/cell.rs::BindingCell` owns
   immutable declaration metadata and Uninitialized/Initialized state. The
   declaring frame, sibling closures and descendants retain the same Rc cell.
   Init differs from Assign; Delete checks presence without reading TDZ values.
   Tests: `binding::cell::policy_tests`, `closure_state.rs` captured/shadowed cells.
3. **Capture order is exact.** Producers and consumers use the same received
   classification; adding one accidental input must not shift subsequent cells.
   Tests: `bytecode/binding_plan/tests.rs`, `bytecode/upvalue_resolver.rs`.
4. **Fresh lexical entry preserves metadata.** Loop/catch re-entry replaces the
   frame's cell handle without modifying escaped cells. Fresh cells retain the
   declaration's policy and TDZ state. Per-iteration copying reads the current
   cell, never a slot copy. Scope re-entry also separates escaped TDZ cells. Tests: `closure_state.rs` loop cells.
5. **Arguments is a lexical binding.** Its parameter descriptor owns an initialized
   lazy recipe, including eval demand; first read creates the object, assignment
   can discard it unread. Arrows inherit the binding; parameters/lexicals shadow
   it. Absent arguments is unresolvable, not a TDZ cell.
6. **Mapped indices alias parameter cells.** Supplied simple-sloppy indices use
   the last duplicate position; descriptors/delete/freeze sever mappings; seal retains them.
   Strict/non-simple is unmapped. Parameter/body cells can share one lazy object.
   Tests: `parameters.rs`, `binding/cell.rs`, `function/call_activation_tests.rs`.
7. **Parameter/body targets are distinct where needed.** Parameter expressions
   require a separate body variable target; same-named body vars copy parameters.
   Parameter-created closures retain parameter/enclosing cells outside body declarations.
   Expression-free rest/pattern lists share the parameter/body variable target.
8. **Global lexicals are canonical cells.** Realm binding storage holds their
   metadata/state; there is no copied lexical value projection or mutability side
   set. A global property of the same name remains independent. Ordinary global
   data routes synchronize with the global object; delete/accessor definition
   invalidates a data route and restores semantic Get, without inventing lexical
   TDZ. ObjectStorage publishers cannot mutate declaratives; invalidation has a
   separate typed state. Tests: `global.rs`, `closure_state.rs`, `binding/cell.rs`.
9. **Intrinsics are not mutable global bindings.** Set-once realm intrinsic fields
   own Object/Array/Function, primitive/error prototypes and well-known symbols.
   Internal errors capture their originating callee realm before unwinding.
10. **Modules retain live cells.** Import routes attach exporting cells and reject
    writes through import access policy. Top-level slots, hoists and namespaces
    share cells established before linking; no settled import/export copies.
11. **Suspension retains locations.** Snapshots own the same cells, scope leases,
    holders and selected references across yield/await, not copies of their values.
    See [generator suspension](generator-suspension.md).

## Dynamic scope and declarations

`binding/scope.rs::ScopeLease` retains an immutable descriptor, positional cells,
parent leases, optional with object and optional own VarHolder. No lease points
at a VM, frame ID, recycled window or borrowed local. A function retains its
optional lexical scope only for compiled name demand or mutable mappings.

Only runtime-added declarations use a mutable name-to-cell holder. An ordinary
activation exposes a fresh own holder at its selected variable boundary. Parent links
remain live: adding an outer declaration after child entry is visible; creating an inner
declaration never selects an outer homonym. Existing own static bindings reuse their
cells and attributes. Eval-added bindings are separately owned and deletable. Delete
reveals the parent on fresh lookup. Saved holder references recreate only their own
binding (sloppy) or throw (strict). Structural epochs change for add/delete/remap, never
for cell-value writes. Tests: `binding/scope_tests.rs`, `direct_eval/s3b_bindings.rs`.

The resolver walks descriptor/cells, own holder, with object and parent in lexical
order; with uses HasProperty and unscopables, releasing every borrow before hooks.
Reference intervals save the selected cell/holder/global base before RHS effects;
consumption, abrupt exit or short-circuit releases it. A global start bound without
observable work (lexical, import, own property) or read at its start saves nothing
(`reference_admission.rs`); every tier stores it by `binding::set_bound_global`.

Sloppy function eval declares in the caller's selected own holder after checking
active lexical/parameter conflicts. Strict eval owns a new holder and lexical scope.
Global script/eval builders explicitly select global routes; function-shaped eval
IR must not imply a function variable environment. Global setters are not published
as data cells. Host scripts install function descriptors before the body and
preserve their non-configurable attributes. Compiled eval caches retain plans and
parse context only; every invocation uses its current chain.

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

## Lexical call context

`context/call_owner.rs` owns receiver/new.target, home/super, active constructor and
field/static restrictions inline. Lexical retention moves the owner into one Rc,
redirecting the declaring activation. BindingPlan propagates owner demand through
lexical arrows (including nested new.target/eval); parameter-only arrows retain none.
Inline calls/constructors and standalone callbacks bind this owner before execution;
wide/loop/VM execution reads it. Inputs/drivers carry no receiver/target/home copy.

Derived this is one uninitialized BindingCell in the typed owner; no declaring
slot or name-keyed mirror exists. Super constructs Base before duplicate checking,
initializes the cell before fields, preserves TDZ on Base failure and preserves
binding on field failure. An object return may bypass an unbound this. Root
builders initialize script/module receivers explicitly; a public property named
this cannot change them. Fields/static blocks establish their own receiver and
restrictions, arrows inherit them, and ordinary nested calls establish new owners.
Tests: `context/call_owner.rs`, `function/call_activation_tests.rs`, `class_keys.rs`.

`context/creation.rs` owns shared realm, referrer, module routes, private context and
explicit global selection. `context/activation.rs` owns logical variable identity
(minted on first exposure), call owner, scope roots, mode and strictness. **Reuse:**
only a returned, never-exposed activation (unminted identity, inline owner, no
scopes/holders/saved references) enters a pool bounded in count and bytes; no lease,
cell, closure, arguments object, continuation, generator or eval holder can reference
it, and an identical later entry rebinds it. Related realms mint non-wrapping tokens in
their AtomDomain boundary. Parameter/body entry changes only its scope boundary.
Module-root routing stays a scope fact; creation/module metadata holds cell routes only.

`Context<'a>` borrows services/activation; it is never cloned or retained. NativeSite
borrows call info, target and eval sources; callee realm is lazy. Vm/tiers own callee
state; no caller-equality proof or empty-env factory remains. RuntimeGroup shares
AtomDomain/weak realm registry; host/agent services stay canonical; Dynamic Function
reuses its callee Realm/live globals. Debug services capture QJS_VERIFY_BINDINGS,
QJS_FORCE_EVAL_EXPOSURE, QJS_POISON_RELEASED_SCOPES and QJS_POISON_RELEASED_ACTIVATIONS
(pooled activations hold a checked poison) once; production ignores them. Forcing
applies to materialized declarations; nonexposing admitted tiers keep their storage.
Class keys execute inline; a cold continuation owns partial class/private context across
yield/await, handlers restore regions and mode. Heritage validation precedes keys; each
method installs before the next; static fields/blocks run in source order; the inner
name stays TDZ through keys, initializes before static fields; inferred names bind none.

Compact/wide/leaf tiers keep static cell paths; retained chains prove nothing of callers.
A slot-backed body that cannot observe its chain (no eval/with/closure creation) enters
without it only while the chain resolves free names globally and captured names to their
received cells: no with object, holder of a suspendable owner, holder binding either name
kind, or other fixed cell of one. `function/call.rs` caches this per (function, holder
epoch), which holder add/delete bumps. T077 retires these tiers later.

## Known limits

- Rc cycles among functions, cells, objects and private state are not collected (collector stage).
- Dynamic lookup is an uncached chain walk; no name/value cache or inherited map fork.
