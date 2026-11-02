# Core rebuild: bytecode, activations and native ABI

Target for [core rebuild](core-rebuild.md), S3; S4 adds scalar instructions in
the same loop. Source paths are under `crates/qjs-runtime/src/`.

## Encoding and compiler

Choose register bytecode. Today's `compiler*.rs` already determines binding slots;
`compact_fn/compile.rs` already maps operand-stack depth to registers. Registers
make argument destinations and ownership transfers explicit. A stack encoding
would be simpler (NG proves it can be fast); an accumulator would introduce
spills around JS calls and multiple property operands. No measured register win
is claimed: T021's virtual-stack reduction lost overall, and T065's smaller Op
was nearly neutral. Acceptance is whole-program wall, never opcode count alone.

`Code` owns `Box<[u32]>` words, constant/atom pools, BindingPlan, call-site and
handler tables, source-PC spans and optional scalar/materialization metadata.
Common ABC word: opcode/three operands, each 8 bits; ABx uses 8/8/16 bits.
`Wide` prefixes carry explicit u32 operands for registers/constants/jumps that
do not fit. The verifier checks canonical prefix lengths, all instruction starts,
operand bounds, register initialization, call ranges, handler targets and CFG
joins; no branch targets an extension word. PC is a word offset. Limits fail
with structured engine errors. No truncation or arbitrary 256-register admission.

Initially the compiler may emit ephemeral stack IR and lower it universally;
it is never executable or cached in parallel at stage exit. No `LegacyOp` opcode
or unsupported-function fallback. All Op families, eval/modules/classes and
suspension compile. Later compiler cleanup can emit registers directly through
the same verified builder; no second language/binding analysis is introduced.
Named operations use a side-table site ID, not embedded descriptors or Strings.

Locals occupy stable low registers; temporaries follow. Distinguish InitLocal,
AssignLocal, LoadCell, StoreCell, LoadRealm, LoadImport and dynamic-name operations.
The `AssignLocal`/`StoreLocal` distinction is semantic (T021's failed admission).
Before capture, move the sole local value into the shared cell and redirect all
subsequent accesses. TDZ/init and cell-route bitmaps accompany registers; there
is no copied binding cache. Liveness may consume only a dead temporary; reading
a local directly must not let coercion helpers `mem::replace` that local away.
At CFG merges, exceptions, callbacks and suspension the state map is complete.

## Frames and the T021 cost

`Executor` owns one contiguous `Vec<Value>` and a `Vec<FrameHeader>` for
ordinary activations. Four prefix slots retain callee/receiver/new.target/home;
only used inputs are written. A 64-byte header retains optional root code,
lazy Context/cold owners, typed CallSlotRefs, window and continuation indices.
Calls push headers, not boxed general records, and return clears written ranges.
Cells and leases retain independent identity and never point into either Vec.

Statically known suspendable functions (generator, async, async generator and
TLA) own a box from entry, including the prologue, and resume the same record.
Only suspended or exposed records can enter a directory; FrameIds are lazy.
General synchronous VM state is owned directly by its driver, with no per-call
boxing. Immutable root/eval code owners are retained once, never deep-copied per
entry. Each resume resets segment declines/handback, preserving semantic state.
Cleared register/header and general operand/local/cold allocations share one
[bounded pool](frame-storage-pool.md); code bodies own no free-capacity state.

**Hot addressing is `regs[r]`, never `store[base + r]`.** Bind a window once
per non-reentrant segment. Calls end the borrow before stack/header growth;
return rebinds the caller. No pointer survives a segment or escapes into a cell,
closure, arguments object or eval holder. Native callbacks pin the owning stack
while borrowing its arguments, with no live Executor borrow across JavaScript.

T021's [1.2678 experiment](../../tasks/archive/T021-single-vm-frame-stack-log.md)
shows why counting fewer activations is insufficient. Inspect release assembly
for repeated directory/base loads, aggregate switch copies or Action spills.
Keep safe slice bounds checks until separately measured; stack reallocation is
not a reason to add unchecked pointers. S3e owns universal Code lowering; current
general stack-IR banks remain semantic state of the same owning record.

## Calls and context

Evaluate callee/receiver/arguments left-to-right into a compiled outgoing range;
resolve callable kind once, reserve the callee block, then move consumed arguments
straight into its incoming/parameter slots. No intermediate Vec or name map.
Actual extra arguments live only when arguments/rest/eval can observe them;
defaults/destructuring run in the callee with correct parameter TDZ and body split.
Spread/bound/apply may grow a reusable staging block and perform their observable
iteration once; they do not pretend to be allocation-free ordinary calls.

BindingPlan replaces per-entry name searches with positional parameter/capture/
realm/module routes. `ActivationContext` contains realm, this/new.target,
home/super/derived-this owner, private environment, ScriptOrModule/referrer,
agent and optional dynamic-scope lease. Ordinary typed fields do not mirror
CallEnv names. Static closures retain indexed cells; eval-capable closures retain
all transitively visible cells plus scope descriptors and with objects, including
bindings absent from static free-name lists. Eval-added var holders are shared.
Scope leases outlive frames and cannot point at recyclable register blocks.
T074's [eval requirements](../../tasks/T074-direct-eval-scope-model.md) must be
resolved into these contracts before S3 implementation, not bypassed by a cohort.

JS→JS call/construct, bound forwarding and bytecode getter/setter invocation push
frames in the one driver. Effectful property/coercion/iterator algorithms that
need JS use cold resumable operation records, returning CallRequest once, then
continuing at a saved semantic step. They cannot recursively invoke eval through
a supposedly synchronous helper. Preserve proxy trap order, construct/new.target,
derived-this and field failure rules. Count JS-depth separately from native depth.
Frame-layout qualification runs the complete default inventory in
`tasks/diagnostics/T077-s3a-depth-probe.py` in debug and release, against the
preceding accepted milestone with `--baseline`: native callbacks, call/apply,
delegation, try/catch, construct, spread, arguments, super and general-VM fallback.
Use the ordinary CLI main-thread stack; no stack-policy override.

## Native boundary

S3a installs operation-specific entries with the existing `CallEnv` service
parameter and borrowed `CallInfo`/arguments. S3b changes that service parameter
to Context; it does not add an adapter or another environment authority.
The final ABI is `NativeEntry = fn(&mut Context, CallInfo<'_>, &[Value]) -> Result<Value>`
directly on native functions; CallInfo borrows callee/this and records construct mode; Context supplies
the active realm, caller context and actual new.target lazily. Port family implementations to Context
capabilities, deleting `&mut CallEnv`; S3a has already removed family probing.
All return/throw values own references. Ordinary natives allocate no CallEnv.

For safe re-entry, the driver detaches and owns the caller's block in a local
`NativeBoundary`, outside Context's active frame directory. Borrow its immutable
argument slice while Context drives callbacks to a boundary token. Callback frames
cannot reuse/mutate that block; no RefCell borrow of Executor remains held. A
native retaining an argument clones it. Host slices work identically. No borrowed
argument range/pointer escapes the invocation. Restore typed current-context
fields and the detached block on return/throw. S3d owns the resource policy: one runtime-wide stack-pointer anchor and a
host-configured budget, with the CLI running the engine on an explicitly sized
thread. Once JS calls use the explicit activation stack, count JS depth by frames
without Rust recursion. Synchronous native→JS→native boundaries remain checked
against that runtime-wide budget. S3a adds no native-only or realm-local limit.

## Abrupt completion and suspension

Handler row budget: `start,end,target,stack_map,scope_depth,kind_flags` as six u32
= 24 bytes. A Completion owns Normal/Return/Throw/Jump; handler tables choose
catch/finally/iterator-close/disposal. Cold continuations retain pending completion
and resource order; a finally can replace it, and throw identity/realm/span remain.
Do not implement JS exceptions using Rust panic unwinding.

Suspend detaches the same owning block, context, cells, semantic continuations and
pending completion into a suspension record. Resume attaches it to this driver;
it builds neither CallEnv nor a fresh VM. Preserve generator parameter prologue,
Executing rejection, yield*, async FIFO, await adoption, module await and job
boundaries from [generator suspension](generator-suspension.md). Ordinary return
does not drain jobs. Typed scalar state is materialized before suspension.
