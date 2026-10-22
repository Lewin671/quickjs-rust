# Interpreter core v3 (proposed)

Target architecture for [T065](../../tasks/T065-interpreter-core-v3.md), pending
lead review; no part of this backend is implemented. Current architecture stays
in [architecture.md](../architecture.md). [Migration](interpreter-core-v3-migration.md)
owns sequencing; [gap anatomy](../../tasks/archive/T065-gap-anatomy.md) owns
revision-bound measurements. Reuse qjs-lexer, qjs-parser and qjs-ast unchanged.

## Representation and ownership

Use a private, Copy, 8-byte `ValueWord`: f64 bits or reserved quiet-NaN tags for
immediates and nonmoving heap pointers. Canonicalize arithmetic Number NaNs;
retain signed zero and infinities. Raw ArrayBuffer loads/stores preserve bytes;
boxing an observed Number may canonicalize it. Small int32 immediates are an
optional tag fast path, with exact overflow/negative-zero fallback to Number.
TDZ, holes and internal completion markers have distinct inaccessible tags.

Unlike v2's owning word, copying a v3 word changes no reference count. The
collector owns payloads. A word alone is never a host root. Embedders receive
`RootedValue`/persistent root handles tied to a runtime; transient words stay
private. Root cloning registers a root, never recursively clones a JS value.
A runtime group owns one heap and atom domain; its realms share them. No
process-global handle table, implicit default heap or cross-runtime raw words.

Start with a safe tagged-enum reference backend for differential/GC tests and
an alternate packed backend in the same vertical slice. The recommended packed
backend uses low-48-bit addresses on qualified macOS x86_64/aarch64; check every
allocation's full address before encoding. Unsupported targets use the enum
backend selected before creating the runtime. Address rejection fails allocation
with a structured error; never mask bits or switch representation mid-execution.

All pointer encoding, allocation, dereference and sweep code lives in small
`core_v3/{word,heap}` modules with local `allow(unsafe_code)`, documented ownership
proofs and a `SAFETY` comment at every block; admission requires a measured
bottleneck versus safe alternatives. Expose provenance from the original live allocation
before integer encoding and reconstruct only that allocation;
never infer Rc headers or manufacture Rust references from arbitrary bits. A heap/lifetime token brands internal access; no reference survives an allocating
or re-entrant operation. Miri on the safe/reference path plus applicable packed
operations, sanitizers and both-target tests precede default selection. If this
proof or M0 fails, evaluate indexed handles as a separately priced alternative;
extra handle-table loads are a cost to measure, not a presumed free solution.

## Heap and collection

Replace Rc/RefCell JS ownership with runtime-owned, typed, nonmoving arena pages
and a stop-the-world mark/sweep collector. Common header: kind, mark state,
allocation size/class; payloads have stable addresses. Size-class bump/free-list
allocation handles small cells; large buffers use separate owned allocations.
First collector is non-generational: no write barrier or moving-pointer protocol.
No RC inside the arena; Rust-owned immutable compiler metadata may keep Rc.

Collect at explicit allocation/call/backedge safepoints after publishing all live
words. Roots include active frame windows, pending arguments/results/exceptions,
native root scopes, host persistent roots, realm intrinsics/bindings, modules,
jobs and suspended frames. Code constants and caches are traced or explicitly
weak. Trace every payload family; debug stress collects at every allocation.
No Rust object/property borrow may span allocation, coercion or a JS callback.
Scope APIs enforce that separation; merely replacing RefCell with raw pointers
is unsound. No runtime-global mutable state and no conservative Rust-stack scan.

WeakMap uses ephemeron fixed-point marking; weak handles clear before reclamation.
WeakRef kept-alive sets last through the specified job boundary. Finalization
Registry cleanup is queued after collection, never executed during sweep; tokens,
holdings and unregister edges follow their distinct strengths. Native resource
release cannot execute JS or resurrect swept cells. OOM leaves roots and frame
state valid; collection and resource destruction are iterative. Track allocated,
live, retained-page and external-buffer bytes; release empty pages with a bounded
reserve so arena high-water usage cannot become permanent RSS growth.

## Objects, atoms and caches

Keep descriptor/exotic algorithms, ordering and observation semantics; replace
the four ordinary storage authorities with one shape/slot model. A compact header
identifies object kind, shape and exotic flags; two inline value slots are the
initial design, larger objects spill into contiguous value storage. Shape entries
own AtomId/SymbolId, slot and descriptor attributes; accessor pairs are cold.
Arrays keep a dense element vector plus hole state, with sparse/dictionary mode;
functions, arrays and wrappers share named-property authority without pretending
their exotic indices or length are ordinary fields.

Intern canonical UTF-16 keys once per runtime group; compiled code links AtomIds
once per group. Public strings are not atomized on every access. Shared immutable
shapes cover literal, constructor, class, JSON and host creation alike. Bound
transition retention and use dictionary mode for mutation/churn; delete/re-add
preserves key order. Retire duplicate payload-side name indexes and the four-store
probe chain. Symbols retain identity and enumeration rules, not string equality.

Per-site monomorphic then bounded polymorphic ICs guard shape, property kind and
prototype validity. Own writable data hits load/store a slot without cloning a
descriptor; prototype hits guard every required dependency. Descriptor/prototype
mutation invalidates dependent guards; value-only writes retain slot guards.
Cache metadata is weak or versioned with non-reused IDs; sweep/address reuse
cannot validate a stale entry. Getters, Proxy, receiver-sensitive Set, namespace,
private fields and exotic operations enter the complete semantic slow path.

## Bytecode, frames and native calls

Replace large enum execution records with verified 32-bit register instructions
(opcode plus small operands), extension words for wide registers/constants/jumps,
and side tables for source spans, ICs, exception regions and root/liveness maps.
The compiler computes temporary registers and max window size once. A decoder
verifies boundaries/operands/jumps before execution; input failures never panic.
Keep byte spans and early-error/binding analysis; an initial old-IR lowerer can
bootstrap M0, but the old interpreter executes no v3 instruction.

Use a Rust match dispatcher over the compact stream, keeping PC/window pointers
local within a non-reentrant segment. Cold semantic helpers are explicit calls.
Computed goto/FFI/assembly and tail-call dispatch are not requirements. Measure
emitted instructions/spills; a smaller encoding alone proves no speed gain.

One runtime driver owns frames and reusable register windows. Frame headers hold
code, PC, base, argument range, return destination, realm and typed call context;
cold state owns handlers/with/private/module metadata. Call seeds a window once,
return writes its destination once; generic JS calls build no nested Vm/CallEnv
or name map. Reacquire views after growth/re-entry, and retain no Vec pointer
across either. Synchronous native callbacks re-enter this driver through a saved
continuation, with bounded native recursion and the existing stack-limit behavior.

Native ABI: `native(&mut NativeContext, CallInfo, ArgRange) -> NativeResult`.
CallInfo supplies rooted callee/this/new.target and realm; ArgRange indexes rooted
windows. Natives copy needed words into a root scope before callbacks; they never
retain borrowed argument slices. NativeContext supplies property/coercion/call/job
capabilities. Bind a builtin's dispatch entry at installation, retiring family
probe chains and the &mut CallEnv ABI as each family ports. Rust Result/completion
enums carry throws; errors do not allocate host message strings on successful ops.

## Bindings, abrupt completion and suspension

Preserve every [binding invariant](env-model-rewrite.md): uncaptured slot storage;
one heap cell after capture; shared sibling, loop-iteration, mapped-arguments,
realm and module cells; immutable self names and TDZ. Only eval/with resolve names
dynamically, through cell references. Replace call-context name assembly with
compile-time BindingPlan plus typed ActivationContext, never caller snapshots.
Capture redirects slot access to the cell once; promotion does not keep a second
binding value. Leases for dynamic scope retain cells, not an entire caller frame.

A rooted thrown word and structured completion carry return/throw/jump through
handler tables, finally, iterator close and disposal; no Rust panic unwinding for
JS throws. Preserve error realm, source PC and observable stack behavior. Suspended
heap frame records retain their own window, context, handlers, pending completion
and the same cells. Resume attaches a record, not a reconstructed name environment.
Keep generator prologue, Executing rejection, yield*, async request FIFO and job
ordering from [suspension invariants](generator-suspension.md); use no Rust async.

## Strings and existing tiers

Adopt primary Latin-1/UTF-16 storage, immutable after publication, with direct
code-unit lengths/indexing. Keep shared String/RegExp subjects and semantic work
from [string-code-units](string-code-units.md); retire sentinel UTF-8 Deref/Borrow
inside the new backend. Parser source text remains UTF-8. Port RegExp coordinates
and host conversions explicitly; no duplicate permanent UTF-8/unit buffers.
Builders may mutate unpublished buffers; defer ropes/slices until independently
measured and bounded against parent retention. GC sharing supplies no Rc uniqueness.

Keep typed-loop/numeric analyses and general semantic admission, but port them to
the common frame/root/exit ABI after the generic core works. Retire compact and
wide as separate owners/drivers: their useful register lowering joins v3 bytecode.
Keep scalar replacement only with exact materialization/root maps at every exit,
throw, callback and suspension. Do not delete profitable typed execution by fiat;
external-lane gates decide whether each port can replace its legacy executor.
