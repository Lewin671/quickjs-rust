# Interpreter core v2: target and migration

Proposed architecture, **not implemented or a frozen performance plan**. The ordinary interpreter must become cheaper
even when every accelerator declines. Calls, names, layouts and values share the contracts below; none depends on a
benchmark or a literal's source form. [T056](../../tasks/T056-interpreter-core-v2.md) owns evidence, source checks and
next work; [architecture](../architecture.md) describes today's code. The protected [binding](env-model-rewrite.md) and
[suspension](generator-suspension.md) invariants remain requirements. No new crate, dependency, FFI or Rust async.

## Contracts shared by the three changes

```text
ExecutionContext = one Vm + FrameStore + native boundaries + job driver
FrameId          = store identity + reusable slot index + generation
FrameSlot        = active header + zero-based local/operand buffers + optional cold state
LinkedCode       = owned bytecode + property atoms linked to an AtomDomain
PropertyLayout   = atom -> (slot, descriptor kind, attributes), ordering, storage encoding
Value            = owning opaque word; payload access through typed methods
```

Frame/shape/context types stay private. Replace public Value enum patterns with documented typed views; migrate CLI and
embedding callers before backend changes (explicit Rust API migration, unchanged JS). Frame/property/tier code adopt those
views first. Typed activation context replaces hot `CallEnv` copies; shared services preserve realm isolation and no mutable process state.

## Frames: storage, activation context and lazy names

M2 removes both nested Vms and ordinary per-call environment assembly. It preserves one authoritative slot/cell per binding; an ordinary call, including one creating closures or invoking natives,
installs **no name-keyed entries**. General admission uses callable kind and execution state, never a benchmark identity or direct-leaf prerequisite. No unsafe is required.

Source paths are relative to `crates/qjs-runtime/src/`. `function/call.rs::function_env` inserts general-call parameters, this, arguments and internal context into `CallEnv.frame_bindings`;
`initial_slots`/`initial_local_upvalues` in `bytecode/vm_bindings.rs` import them. `FrameBindings` is a name-keyed **vector**. `vm_frame_init.rs::frame_call_env` forks entries as values, exports live slots,
then overlays cells, catch/conflict sets and private state; `frame_deopt_bindings` overlays the dynamic cell map. `vm_call_env.rs::callee_env` selects this view for direct-eval/with/deopt callers **or
closure creators**: a native may invoke a passed closure whose eval needs outer names absent from static upvalues. Legacy APIs take only `CallEnv`; `apply_env` reconciles values/declarations and
`apply_call_env` scans realm fallbacks. This compatibility view is not binding authority. `vm_call.rs` already sends ordinary native Call opcodes a realm-only env; helper calls through
`callee_env`/`current_env` still export it. Deleting the closure predicate or pooling the vector neither migrates consumers nor removes the work.

Owned code gets an immutable `BindingPlan`: parameter/input slot indices, received-cell order, call-context indices, scope parent/ranges, declaration kind/mutability and parameter/body/catch
relationships. Runtime scope-instance tokens distinguish shadowing, TDZ, exited scopes and fresh loop iterations; source names live in code metadata, never copied at entry. `CallSeed` carries owned
arguments (or a slice consumed before switching), this/new.target, callee/code/cells and realm handles. Initialize slots by index; compiled default/destructuring/rest prologues retain their order.
Mapped arguments uses those same parameter cells, including duplicate-parameter rules. `ActivationContext` stores typed home/super/active-constructor, derived-this cell, function/field/eval flags,
immutable-self identity, private environment, module/host/agent handles and realm. Lexical arrows retain the declaring context/cells, never inherit these from whichever native happens to invoke them.

A dynamic request resolves `(scope instance, name)` through code metadata to a slot/cell, then parent scopes and realm. Promoting a slot moves authority to its existing/new cell and clears its
authority bit; it never copies into a second binding. Direct eval creates its own lexical scope; strict vars stay there, sloppy vars enter the correct caller variable scope, and parameter-eval vars
remain distinct from body vars. Only genuinely new eval names need a lazy name-to-cell extension. Conflict/deletability checks use scope metadata. With-object lookup preserves own-local precedence,
unscopables and Proxy effects. Lookup caches include scope-instance/mapping generations; addition, deletion, remapping and iteration invalidate bypasses.

An eval-capable closure must retain arbitrary visible names, even absent from static free-name lists. Propagate that capability through nested closures/eval-created functions. At creation, retain an
owned `ScopeLease` of code + positional cells + parent scope descriptors (including TDZ locations and with objects); promote potentially observable locals by slot plan, with no name installation.
Static closures capture only their indexed cells. Escaping leases retain lexical scopes, never a caller frame; future initialization/writes hit the promoted cells. A dynamic consumer may build a lazy
cell index on first request, never a value snapshot. Leases share a variable-scope extension holder before eval adds names, never reference reusable FrameIds, and retain escaped cells across scope exit.

| Current consumer / source | Target service |
| --- | --- |
| General entry and closure capture: `function/call.rs`, `function/arguments.rs`, `vm_bindings.rs`, `vm_capture.rs`, `vm/rare_ops.rs`, `vm_class.rs`, `vm_private.rs` | Indexed seeds/received cells and typed context above; mapped arguments and immutable class/self cells share identity. Ordinary closure creation exports no names. |
| Direct eval: `vm_call.rs` cached/selected/full views; `global.rs` parse context, declaration checks, written-name reconciliation and captured-function repair | Resolve against the eval caller's scope lease; existing writes hit cells, new declarations extend only its variable scope. Parameter/body separation and strict lexical isolation replace repair/write-back, not syntax caching. Indirect eval/Function stay realm-only. |
| With and retained dynamic scope: `vm_with.rs`, deopt overlays and `closure_scope_use` in `vm_frame_init.rs` | Ordered object/scope descriptors plus generation-checked lazy name resolution; no eager overlays at every call/closure. Preserve bypass invalidation. |
| Native forwarding/callbacks (call/apply/bind), accessors, Proxy traps, coercion, iterators and disposal: `vm_call_env.rs`, `vm_props.rs`, `vm_set.rs`, `vm_iter.rs`, `vm_ops.rs`, `vm_literals.rs`, `vm_dispose.rs`, `vm_string_append.rs` | Realm/services capability and same-context callback boundary; each JS callback uses its own lexical lease/upvalues. Helper `current_env` calls for heritage/private/import paths use typed context too. Native invocation alone requests no caller names. |
| Coercion filtering/sync: `conversion.rs::to_primitive_method_env` / `sync_to_primitive_method_env` | Method's own captures/native context and shared realm cells; remove name filtering/value write-back once reads/writes use those authorities. Explicit native payloads remain function-owned. |
| Call completion, derived this/super, realm overrides: `function/call_frame.rs`, `vm_result.rs`, `vm_call.rs` marked-realm save/restore | Typed continuation and shared derived-this cell; isolated self binding cannot mutate caller names, so no restore-by-name. Callee realm/intrinsics selected by handle; preserve explicit cross-realm override semantics without global-value snapshots. |
| Realm/global/module refresh: `apply_env`, `refresh_realm_backed_locals_from_realm`, `vm_props.rs`, `vm_module.rs`, `module/link.rs` | Slot source is local/cell/realm reference; imports/exports use canonical cells. Global delete/accessor/recreate publishes a shared invalidation marker/generation; loads re-resolve through full property semantics, never a post-call scan. Eval shadowing retargets the affected slot to its new local cell. |
| Result enumeration / diagnostics: `module/link.rs::binding_snapshot`, `vm_jobs.rs`, `vm_import.rs`, `CallEnv` Debug | Export/result handles retain requested cells and services after frame release; enumerate only at explicit result/diagnostic request. An owned value snapshot is output only, never execution storage. `debugger` currently compiles as an empty statement (`compiler.rs`); no debugger-reflection API is assumed. |

`FrameStore` has reusable slots in reference-counted boxed chunks with checked borrows. Directory growth moves pointers, never frames. Initialize fields in place; never return/queue/swap a FrameState.
Reuse clears values/cells/completions before advancing generation; empty capacity is not a GC root. Keep existing cold try/with/disposal state. Bind each slot's zero-based buffers once per activation;
no shared-vector base arithmetic per op. Measure allocations, retained capacity, initialization stores and emitted copies; stable addresses alone do not prove cheap code.

Dispatch borrows a slot and disjoint services. Helpers yield `Enter(CallSeed)`, `Return`, `Throw`, `Suspend` or `NativeRequest`. Publish IP/typed continuation and release slot, buffer, tier and
`FrameProgramView` borrows before switching/re-entry; retain owned code handles and reacquire views after stream selection. Calls push IDs; return moves only the result and drops in place.
Continuations cover discarded results, construction/new.target, derived completion, eval and native boundary results. Preparation preserves bound/Proxy forwarding, parameter and class-field effects
exactly once. Throw unwinds catch/finally/using before popping; pending completions stay frame-local and finally may replace them. Preserve thrown identity, spans and realm.

Natives receive a capability, not a borrowed frame. `call_sync` drives **this same Vm** to a boundary token; throws return to the waiting Rust native rather than unwinding through it. Rust driver
re-entry may nest, never another Vm. Enforce JS/native depth limits before Rust stack exhaustion. Suspending builtins need explicit native continuations, never suspended Rust stacks. A generator/async
body detaches a lease retaining its store/code/cells after its creator returns, without retaining its caller; reattach to a compatible invocation. Preserve synchronous prologues, Executing rejection,
yield*, finally, async-generator FIFO and promise job order. Await yields to the job driver; ordinary returns do not drain jobs. Host independent roots may create contexts; callbacks from active
contexts must reuse them. Cross-realm frames choose callee intrinsics and compatible atom linkage.

## Properties: atoms and layouts are the ordinary lookup authority

An owned `AtomDomain` interns exact JavaScript code units (including lone surrogates) to owned identities; storage pins entries.
Linking requests checked, non-reused numeric IDs, with explicit exhaustion errors. Exchanging realms share a domain. Bytecode keeps a symbolic name pool and
obtains a cached per-domain `LinkedCode` relocation, so reused eval/Function/module code cannot carry another domain's
integers. Foreign inputs are re-interned by content at a boundary. Objects/layouts retain their domain; escaping values
keep it alive. Reclaim unused atoms with owned atom references without reusing IDs; transitions/link caches must not
retain arbitrary churn forever. Exhaustion fails explicitly, never wraps.

`ToPropertyKey` runs once, preserving effects; the result is StringAtom or Symbol identity. Numeric-index classification
preserves distinctions such as `"01"` versus `"1"`. Symbols/private names are separate identities, not string interning
aliases. Host string APIs intern at entry. Repeated named bytecodes do no string hashing or comparison; computed keys
still pay conversion/interning when not already atomized.

Every ordinary named-property store has a layout authority and value slots. `Small`, `Dynamic`, `Shaped` and
`ShapedPair` initially keep their payload layouts behind slot adapters; each receives the same atom lookup contract. The
layout's storage encoding makes its slot interpretation unambiguous. Later consolidation may remove adapters only if
measured; literals get no privileged lookup admission. Arrays, functions, Map/Set wrappers and boxed primitives use that
contract for their ordinary named parts, including lazily installed properties after their semantic hook.

Shared immutable shapes describe keys, slots, descriptor kinds and attributes. Structural edits publish a new shape;
stable value assignment changes only the slot. High-churn objects use a dictionary layout with its own identity/version
and atom index, avoiding an unbounded transition tree. Shape transitions are weakly retained; dictionary
insert/delete/reconfigure/reorder changes its version before any observer. Enumeration retains numeric-index sorting and
string/symbol insertion order; delete/reinsert and duplicate definitions preserve their language-defined order.

Baseline `Get` classifies the receiver, probes its layout once, then reads the live data slot or follows the prototype
chain. Accessors retain the original receiver; clone the callable/descriptor and release all storage borrows before
invoking it. Proxy, module namespace live exports, primitive virtual fields, array indices/length, string indices and
TypedArray integer-indexed semantics keep their full algorithms. Ordinary named portions remain covered; a shape match
never overrides exotic behavior. Missing/index/hole paths preserve prototype getters and conversion/detachment effects.

`Set` uses the same lookup and descriptor authority. Direct own writes require the actual receiver and a writable data
slot; additions, inherited setters/non-writable properties, super/Reflect receivers, extensibility, strict errors and
global-cell synchronization go through complete semantics. Define/delete/freeze/seal and lazy materialization publish
layout changes atomically with slot changes. Keep existing revision consumers valid until migrated; value-sensitive
caches still need value revisions.

No per-site cache is needed for correctness or baseline admission. Remove the old rejecting chain only after every
consumer uses the new lookup. A later **leaf** IC may memoize `(domain, layout token, atom, slot)` without storing
values. Inherited answers additionally validate every missing-own shape and prototype link to the holder, including
dictionary versions. No recycled address alone is an identity; no cache retains receivers strongly or holds a borrow
across JS. This layer is safe Rust; unchecked slots need separate bottleneck evidence and an unsafe audit.

## Value word and lifetime

Target an opaque, non-Copy, 8-byte NaN-boxed `Value` on qualified 64-bit targets. Reserve high words `0xfff8` for
immediates and `0xfff9` for a heap pointer in the low 48 bits; header kind selects the heap family. Other negative
quiet-NaN tags remain reserved. Canonicalize numeric NaNs to `0x7ff8000000000000`; preserve infinities, signed zero, BigInt and
Symbol identity. Do not rewrite raw ArrayBuffer bytes when a load produces a Number. Reserved internal TDZ/empty states
never escape as JS values. Slot-state bitmaps avoid widening every slot back to `Option<Value>`; capture still transfers
authority to the existing cell.

One allocation holds `HeapHeader { strong, weak, kind, destroy }` and the aligned payload, replacing the corresponding
Rc allocation, not wrapping an Rc in a box. Strings, BigInts, functions and object families share this ownership
protocol; header-kind load and count overhead must pass M3's payoff gate. Cloning bumps a heap count; moves transfer
ownership; borrowed views do neither. Last strong drop destroys
exactly once; weak observers retain only a tombstone until last weak drop. Escaping values outlive Vm/realm; weak
upgrades cannot observe a reused allocation. Preserve string uniqueness/COW and iterative deep teardown. Cells remain
shared `Upvalue` identities; no cell refresh/writeback is introduced. Non-atomic counts belong to the single-thread
core; feature-gated shared buffers keep their explicit sharing protocol, never make arbitrary Values Send/Sync.

Pointer payload width and Rust provenance are release blockers, not assumptions. Qualify each supported ABI/address
range before selecting this backend; never truncate an allocation pointer. Use an explicit provenance-preserving
reconstruction protocol (including exposed-provenance APIs where integer packing requires them). Unsupported targets
retain the enum backend; if a required target cannot support 8 bytes, stop and revise the encoding before rollout. No
process-global handle table or guessed Rc layout. Unsafe is confined to word/header modules, with module-level
`#[allow(unsafe_code)]` and a `// SAFETY:` proof on every block: valid tag, alignment, provenance, live count, exact
payload kind, unique destruction and weak lifetime. Safe typed views retain RefCell checks. A safe enum is the
comparison baseline; measured transport/count cost must justify unsafe. Miri and ownership tests supplement, never
replace, conformance. This retains today's cycle limitation; cycle collection/block allocation is a separate subsystem,
not a hidden prerequisite or claimed allocation win over T023.

## Tiers and exits

Compact/numeric/typed-loop tiers retain scalar banks and admission order. Their boxed banks use `Value`; shared property
operations use atoms/layouts. Exit records name FrameId, exact IP, operands, slot/cell authority and pending completion.
Publish them before a generic call/re-entry; reacquire views after it. Virtual objects retain their field bank and
identity-flow proof; do not invent a heap receiver for a placeholder. Only a later tier unit maps proven virtual fields
into registers or materializes at a proved escape. Any side exit reconstructs state once, without replaying effects.

Stage order, effects, stops and gates: [T056](../../tasks/T056-interpreter-core-v2.md#ordered-migration); its
[prior attempts](../../tasks/T056-interpreter-core-v2.md#prior-attempts) explain how this differs from rejected work.
