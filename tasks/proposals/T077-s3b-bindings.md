# S3b binding plans, leases and exposure

Part of the [frozen brief](T077-s3b-brief.md); implements all approved
[T074 decisions](T077-s3-eval-scope.md) within the existing executable IR.
These are target internal types, not claims about current code.

## Static authority and live locations

```rust
struct BindingPlan {
    scopes: Box<[ScopeDescriptor]>, bindings: Box<[BindingMeta]>,
    params: ParameterPlan,               // slot routes, defaults, duplicate map, body split
    received: Box<[CaptureRoute]>,       // ParentLocal / ParentUpvalue, in received order
    locals: Box<[LocalRoute]>,           // slot/cell/realm/import/live-name
    arguments: ArgumentsPlan,            // None / Mapped / Unmapped, demand/extras
    exposure: EvalExposurePlan,
}
struct BindingMeta {
    id: BindingId, name: Name, kind: BindingKind,
    mutable: bool, strict_write: WritePolicy, deletable: bool,
    initialization: InitPolicy,          // TDZ / initialized undefined / parameter prologue
}
struct ScopeDescriptor {
    id: ScopeId, parent: ScopeBoundary, kind: ScopeKind,
    declarations: Box<[BindingId]>,       // source declaration order
    lookup: NameIndex<BindingId>,         // immutable name->route metadata, NO values
    routes: Box<[DescriptorRoute]>,       // binding->lease cell ordinal / parent boundary
    variable_target: VariableBoundary,   // separate parameter/body/eval boundaries
}
struct BindingCell(Rc<CellStorage>);
struct CellStorage { meta: BindingMetaRef, state: RefCell<BindingState> }
enum BindingState { Uninitialized, Initialized(Value), LazyArguments(Box<ArgumentsRecipe>) }
struct BindingRef { location: BindingLocation, meta: BindingMetaRef, access: AccessPolicy }
enum BindingLocation { Slot(LocalSlot), Cell(BindingCell), Global(GlobalRoute) }
struct ScopeLease {
    descriptor: ScopeDescriptorRef, parent: Option<Rc<ScopeLease>>,
    cells: Box<[BindingCell]>,            // positional exposed bindings only
    object: Option<Value>,              // with object; live identity
    own_vars: Option<Rc<VarHolder>>,     // only this variable environment
}
struct VarHolder {
    identity: VariableEnvIdentity,
    entries: RefCell<NameMap<BindingCell>>, structure: Cell<StructuralEpoch>,
}
struct EvalExposurePlan {
    scopes: Box<[ExposureScope]>,         // visible BindingIds incl. future TDZ declarations
    closure_sites: Box<[ExposureSite]>,  // scope IDs to expose at closure/eval creation
    crossing_names: Box<[LiveNameRoute]>, // stop static routing at extensible/with boundary
    arguments_demand: ArgumentsDemand,
}
```

IDs are plan-local typed indices; references retain their immutable plan owner,
not a bare recycled integer. Dynamic declaration metadata is separately owned
and immutable. `Upvalue` becomes a BindingCell handle; its identity remains Rc
identity. Unboxed slot uses the same BindingMeta and Option<Value> TDZ state.
Init/Assign/Read/Delete are separate operations; no unchecked raw `set` remains
on declarative cell paths. Import access policy rejects writes even when its
exporting target is mutable. Named-function sloppy no-op versus strict throw,
class/const/import writes and derived-this single initialization stay distinct.
Internal non-binding cells (jobs/private state) use their own typed storage.

`BindingLocation::Slot` is an ephemeral route usable only by its live frame;
it cannot escape into leases, closures or saved dynamic references. Dynamic
resolution first exposes the selected slot, then returns a Cell reference.
Realm object-property references are handled by GlobalRoute with descriptor
semantics; a deleted/redefined property invalidates its binding route, not an
uninitialized-lexical marker that would invent TDZ on a subsequent global read.
Native Context has no local-storage borrow. The executing VM/tier performs
`prepare_exposure(site, locals, cell_routes)` before constructing Context for a
direct eval or escaping closure; every dynamically reachable slot is then a cell.

## Compile once, instantiate only required identity

Build BindingPlan alongside current bytecode locals/upvalue resolution, once per
compiled body. No second parser/name analysis or executable bytecode authority.
Retain separate parameter, function-name, body, block, catch, loop, eval and with
boundaries. Compiler temporaries never enter descriptors/exposure. Reuse current
slot indices and capture order; do not reorder received upvalues accidentally.

Potential direct eval means a syntactic eval-call site that could invoke intrinsic
eval; a shadowable `eval` name is conservatively included. Propagate exposure
requirements through lexical descendants (functions/arrows/methods/keys/fields),
across intervening closures even with no static free read. Each closure site
lists its **active** enclosing scopes; retain every potentially visible binding
in those scopes, including not-yet-initialized lexicals declared later in the
same scope. Do not expose unrelated sibling scopes or inactive future blocks.
Runtime-created eval code compiles its own plan against the leased chain.

At an exposure site `expose(plan, frame, scope)` moves each private slot value
or TDZ state into its unique cell, clears slot ownership and redirects the
authoritative-slot mask. Repeated exposure reuses the cell. Already received,
realm, module and mapped-parameter cells are never copied. Ordinary accesses
stay positional and consult the cell route after promotion; no name lookup is
added to every local. Fresh loop/catch entry creates new cells; exit restores
the prior ScopeCursor, not a mutated flattened map. Descriptor name indices
are compile-time shared data, not per-call name/value maps.

Slot-only entry seeds parameters/captures and typed context without allocating
a scope map/lease. Fresh logical variable identity is promoted to a VarHolder
only on demand. A retained ScopeLease owns cells, parent leases and with objects,
never a FrameId, Vm pointer, pool window or borrowed local. Existing Rc-cycle
limits remain; no new global retention cache or GC is introduced.

## Resolution and declarations

`resolve(ctx, name, intent) -> Result<ResolvedReference>` walks the ordered
ScopeCursor: declarative descriptor/cells, own VarHolder, with HasProperty and
unscopables, then parent/global. It returns Cell+metadata, Property(base,key,
receiver,strict), or Unresolvable; release chain borrows before JS hooks.
Read/typeof/Assign/Delete consume that reference. Capture assignment references
before RHS effects; callback mutation does not trigger a second selection.

Names crossing an intervening eval-extensible variable environment or with use
LiveNameRoute, even if their child body has no eval. Bindings statically closer
than that boundary keep positional routes. S3b uses an uncached chain walk for
crossing names: no route-cache implementation is required. StructuralEpoch
changes on add/delete/remap for assertions/future caches, never on cell writes;
epochs must not wrap into a valid stale proof. Delete old scope-bypass proofs.

Sloppy EvalDeclarationInstantiation checks lexical conflicts along the specified
chain, then declares var/function in the caller's selected **own** holder; an
existing own binding reuses its cell/attributes. Finding an outer homonym never
chooses it as the declaration target. Parent holders stay live, so new outer
declarations can appear after child entry while new inner declarations shadow
them. Static var/function cells remain nondeletable; eval-added bindings carry
deletable metadata. Deletion removes only the selected own entry, revealing a
parent on later resolution; an already selected reference keeps its cell/base.

Strict eval creates its own lexical and variable environments, with declarations
confined there; indirect eval selects the realm global environment. Parameter
eval and body eval select distinct variable targets for non-simple parameters.
With is never a declaration holder: `eval('var x=2')` declares at the variable
target but independently resolves its initializer assignment through with.
Preserve Annex B, catch conflicts, abrupt order and global object descriptors.

## Arguments, lexical context and suspension

ArgumentsPlan creates the canonical lexical binding only in a real arguments
owner when observable, including transitive eval demand. Absent is failed
resolution; Present holds a value/lazy object recipe; TDZ is BindingState, never
absence. A body `let arguments` shadows implicit/parameter bindings by scope.
Simple sloppy parameters share exact cells with mapped indices (last duplicate
mapping per specification); reverse writes, accessor/redefinition/delete and
nonwritable severance preserve identity. Strict/non-simple lists are unmapped.
Keep actual argc/extras only when arguments/rest/eval or the prologue needs them.
LazyArguments is initialized, not TDZ: its recipe owns the required actuals and
parameter-cell mapping. First observable read replaces it with one object in the
same cell; Assign before first read drops the recipe. Never retain a slot mirror.

Deferred computed keys lower inline in the enclosing IR, surrounded by class
region enter/exit operations. A cold `ClassKeyContinuation` owns the partial
class, element index, saved scope/private/lexical cursor, operand depth, result
slot and resume PC. Yield/await keep that region and pending completion on the
enclosing activation, never a thunk call. Abrupt handlers restore overlays once;
arrows created there retain the class-private owner. Field/static regions apply
their own restrictions. Existing snapshots save this state; S3c later reuses the
whole owning block. Eval caches retain plans/parse context, never activation
routes; cached free names resolve through each invocation's live chain.
