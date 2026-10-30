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

`Executor` owns a reusable directory of boxed activation records, a bounded pool
of cleared register blocks, and a stack of suspended FrameIds. Each activation
owns one `RegisterBlock { values:Vec<Value>, init_bits, cell_routes }`; allocation
is amortized, not a fresh Vec per ordinary call. Deep recursion may grow capacity;
return clears written owning slots and caps retained pool bytes. Cells never pool.

Proposed 64-bit hot `FrameHeader` budget: owned Code handle 8, block handle 8,
context handle 8, callee owner 8, continuation handle 8, cold-state handle 8,
PC 4, return register 4, actual argc 4, flags 4 = **64 bytes**. A directory
`FrameId { index:u32, generation:u32 }` is 8 bytes; boxed blocks stay stable when
the directory grows. Header is initialized fieldwise, not copied wholesale.
Context/cold allocations are optional or pooled; the 64-byte budget excludes
register data, cell arrays and semantic argument/rest/arguments objects.

**Hot addressing is `regs[r]`, never `store[base + r]`.** On entry/switch take
the current block out of the directory into the driver's local owner and bind
`let regs = current.values.as_mut_slice()` once per non-reentrant segment. Its
data pointer/length and code/PC are loop locals. A JS call ends the borrow,
stores only continuation state, pushes the old block/header, and takes the new
block. Return restores them. No FrameId resolution or base addition per access.
Use safe slices initially; no raw pointer is needed just to persuade Rust.

This generalizes the actual `compact_fn/activation.rs::run_frames` zero-based
`window` inner loop, without its contiguous global register stack or admissions.
T021's [1.2678 experiment](../../tasks/archive/T021-single-vm-frame-stack-log.md)
shows why merely counting fewer activations is insufficient. Release disassembly
must confirm base is established at switches, no repeated directory/base loads,
no aggregate action store/reload on ordinary opcode backedges. Keep bounds checks
until separately measured; safe indexing does not justify unchecked operations.

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

## Native boundary

Install `NativeEntry = fn(&mut Context, CallInfo<'_>, &[Value]) -> Result<Value>`
directly on native functions; CallInfo borrows callee/this/new.target and identifies
the active realm and caller context. Port family implementations to Context
capabilities, deleting `&mut CallEnv` and the 28-family `Option` probe dispatch.
All return/throw values own references. Ordinary natives allocate no CallEnv.

For safe re-entry, the driver detaches and owns the caller's block in a local
`NativeBoundary`, outside Context's active frame directory. Borrow its immutable
argument slice while Context drives callbacks to a boundary token. Callback frames
cannot reuse/mutate that block; no RefCell borrow of Executor remains held. A
native retaining an argument clones it. Host slices work identically. No borrowed
argument range/pointer escapes the invocation. Restore typed current-context
fields and the detached block on return/throw. Synchronous native→JS→native may
nest Rust boundaries under a checked native-depth limit; pure JS recursion cannot.

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
