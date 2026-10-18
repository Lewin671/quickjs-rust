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

Frame/shape/context types stay private. Replace public Value enum patterns with documented typed views; migrate CLI
and embedding callers before changing the backend. This is an explicit Rust API migration, with unchanged JS behavior.
Frame, property and tier code adopt those views first. `CallEnv` retains per-activation context; shared services do not
globalize realm state. Independent runtime instances share no mutable process state.

## Frames: change ownership before routing calls

`FrameStore` uses reusable slots in reference-counted boxed chunks, with per-slot checked borrows released at every
driver boundary. Growing its chunk directory moves pointers, not live frames. Initialize a reserved slot's fields
directly; never return a complete `FrameState`, queue one in an `Option`, or swap it into a `current` field. Slot reuse
clears all live values, cells and pending completions before its generation advances. Retained empty buffer capacity is
not a GC root.

Each slot owns zero-based locals and operands; the executor binds their slices once per activation. There is no shared
value vector with a base addition on each access. The hot header holds code ownership, IP, buffer handles and authority
masks; try, with, disposal and suspension state remain cold. Reuse existing cold-state splitting rather than claiming it
as new. Measure chunk capacity/retained memory, initialization stores, allocations and emitted frame copies; safe Rust
alone does not prove cheap code.

Dispatch borrows an active slot and disjoint execution services, not an entire `Vm` plus aliases into it. Helpers return
`Enter(CallSeed)`, `Return`, `Throw`, `Suspend` or `NativeRequest`; a seed carries arguments/context, never a
constructed frame. Publish the resume IP and continuation, release frame/buffer borrows and the current
`FrameProgramView`, then let the driver act. Code is owned by a retained handle; no self-referential code slice survives
a switch, re-entry or stream reselection.

Ordinary calls push a `FrameId` and a typed continuation. Return transfers an owned result, drops the callee in place
and resumes the caller. Continuations also encode discarded results, construction/new.target/derived-this completion,
eval completion and native boundary results. Preparation is shared with today's callable dispatch: bound argument/this
forwarding, Proxy traps, parameter initialization, class fields and caller completion policy must run once, in their
specified order.

Throw unwinds one frame at a time through its existing catch/finally/disposal machinery. A pending return/throw/jump
belongs to that frame; finally may replace it. Never pop a frame before its cleanup finishes. Preserve thrown-value
identity, call-site spans and realm context. Account for JS depth and native bridge depth; exceeding configured limits
produces a catchable engine error before Rust stack exhaustion.

Natives receive an execution context capability after dispatch releases borrows. A synchronous callback uses `call_sync`
to drive **this same Vm** until an explicit boundary token completes; it cannot unwind through a waiting Rust native.
Rust re-entry may still nest driver invocations, but never constructs another Vm. No `RefCell`/slot/tier borrow spans
that boundary. Builtins that need suspension use explicit native continuation state; arbitrary Rust stacks cannot be
suspended. Per-activation environment state (today's `CallEnv`, its name-keyed frame-cell layer and write-back) is frame
work: the slot owns it, and a native call gets the capability, which resolves names against the slot's code metadata
only when a dynamic-scope consumer (direct eval, `with`) asks, never by eager per-call name-keyed installation.

A generator/async activation owns a detachable frame lease plus its code/cells; suspension removes it from the active
chain without retaining its caller. The lease retains its store allocation after the originating call/context returns
and can be attached to another compatible invocation. Ordinary synchronous callees finish before their parent suspends.
Preserve prologue timing, Executing/reentrant-next rejection, yield* injection, finally state, async-generator FIFO and
promise job ordering. Await returns control to the job driver, never drains jobs at each frame return.

Eval/with initially use the existing semantic preparation adapter at an explicit boundary; they share live
cells, never a locals value map. Final routing puts their prepared frames on this store too. Cross-realm calls select
the callee's realm/intrinsics and compatible atom linkage. A host starting an independent root may own a new context; a
callback from an active context must use its capability. Admission is callable kind and execution state, with no
direct-leaf prerequisite in the final interpreter route. Frames/services require **no unsafe**.

## Properties: atoms and layouts are the ordinary lookup authority

An owned `AtomDomain` interns exact JavaScript string code units (including lone surrogates) to non-reused IDs. IDs
belong to that domain, not a realm; realms that exchange objects share it. Bytecode keeps a symbolic name pool and
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
