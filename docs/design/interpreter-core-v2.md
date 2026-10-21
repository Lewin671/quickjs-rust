# Interpreter core v2: target and migration

Proposed architecture, not implemented or a frozen performance plan. The ordinary interpreter must become cheaper even when accelerators decline.
[T056](../../tasks/T056-interpreter-core-v2.md) owns migration/evidence; [architecture](../architecture.md) owns current code. No FFI or Rust async.
The proposed kernel crate below is the only new dependency boundary; implementation awaits independent review and evidence-bound planning.

## Contracts shared by the changes

`ExecutionContext` owns one Vm, FrameStore, native boundaries and job driver; `FrameId` is store + slot + generation.
A FrameSlot owns a hot activation header, zero-based local/operand windows and optional cold state. LinkedCode owns bytecode and domain-linked atoms.
PropertyLayout owns key→slot/descriptor/order authority. Value is an owning opaque word accessed through typed views, not public enum patterns.
Frame/shape/context types stay private; migrate embedding/CLI callers before a Value backend change. Kernel isolation does not require M2 or M3.

## Frames: storage, activation context and lazy names

[Frame activation](frame-activation.md) owns cohorts, costs, native capability migration, driver stages and the M1 interface;
[environment model](env-model-rewrite.md) and [suspension](generator-suspension.md) remain protected. Required target contracts:
- Ordinary entry installs no name-keyed entries or per-call snapshots. Replace the whole legacy route for an admitted cohort, never mirror its fields.
- Immutable BindingPlan precomputes parameter/input slots, received-cell order, immutable-self/arguments facts, scope parents, TDZ and declaration relationships.
  CallSeed moves arguments once from caller windows into slots; default/destructuring/rest order and mapped-argument cell aliasing remain exact.
- Typed ActivationContext retains this/new.target, home/super, derived-this cell, private environment, module/referrer/imports, realm/agent and eval/field flags.
  Arrows retain their declaring context; cross-realm calls use callee intrinsics. Native arguments remain borrowed in place. No native callback inherits another activation's private names/referrer.
- Dynamic lookup resolves scope-instance metadata→existing slot/cell→parents/realm; promotion transfers authority, never copies it. Only genuinely new eval names extend a lazy cell map.
  Preserve parameter/body separation, strict lexical isolation, sloppy var placement/conflicts, TDZ/shadowing, own-local precedence, with/unscopables/Proxy effects and fresh loop scopes.
- Eval-capable closures retain every visible name via ScopeLease(code, positional cells, parent descriptors, with objects), even absent from static free-name lists, transitively through nested eval/with.
  Variable-scope extension holders are shared before eval adds names; leases outlive frames and never refer to reusable FrameIds. Static closures capture indexed cells only.
- Native capabilities expose typed caller-varying state with stack-disciplined restoration; legacy writers stay on their audited route until migrated.
  Coercion/accessor/iterator/disposal/call-apply-bind callbacks retain captures and cell identity, not exported name/value overlays or refresh scans.
- Globals/imports/exports use canonical cells; delete/accessor/recreate invalidates shared markers/generations, eval shadowing retargets slots, and no post-call refresh scan is introduced.
- FrameStore uses Rc-owned boxed chunks and checked borrows: growth moves directory pointers, not frames. Initialize/drop in place; clear values/cells/completions before generation reuse.
  Bind each zero-based window once, never resolve FrameId/base per op; empty retained capacity is not a GC root. Stable addresses alone do not prove cheap entry.
- Dispatch keeps same-executor call/return in the driver; cross-executor Enter/Return/Throw/Suspend/NativeRequest publishes IP/continuation and releases all frame/program/buffer borrows.
  Reacquire after re-entry/stream selection; preserve construction, derived completion, spans, thrown identity, finally/using replacement and depth limits.
- Natives drive this Vm synchronously to a boundary token; only driver re-entry may nest Rust. Suspensions own explicit continuations, never suspended Rust stacks.
  Generator/async leases preserve Executing rejection, yield*, finally, FIFO and job order; ordinary return does not drain jobs. No debugger-reflection API is assumed.
- Explicit result/diagnostic enumeration retains requested cells/services after frame release; owned value snapshots are output only, never execution authority.

## Properties: atoms and layouts are the ordinary lookup authority

An owned AtomDomain interns exact JS code units, including lone surrogates, to non-reused IDs; communicating realms share it.
Bytecode links its symbolic pool per domain; foreign keys re-intern by content. Objects/escaping values retain owners; weak interning and bounded transitions cannot retain arbitrary churn.
ToPropertyKey runs once. Numeric indices distinguish `01`/`1`; symbols/private names retain separate identity. Exhaustion fails, never wraps.
Every ordinary named store uses layout authority plus keyless slots: objects, arrays/functions, Map/Set wrappers and boxed primitives, including lazily installed named properties.
Small/Dynamic/Shaped/Pair payload adapters do not restore payload-side name indexes; literals receive no privileged admission. Domain selection happens at creation.
Shared immutable shapes own keys/slots/attributes/order; stable value writes change slots only. Bounded owned edges serve edits; high churn uses private dictionary identity/version.
[Property layouts](property-layouts.md) owns construction/retention/size contracts. Preserve numeric sorting, string/symbol order, duplicates and delete/reinsert semantics.
Get probes once, reads live data or follows prototypes with the original receiver. Release descriptor/storage borrows before any accessor/Proxy/callback.
Set requires actual receiver and writable data; additions/inherited setters/nonwritable fields, super/Reflect, strict errors and global cells use full semantics.
Shape identity never overrides module live exports, primitive virtual fields, indices/length, lazy String or TypedArray/detachment behavior.
Define/delete/freeze/seal/prototype/lazy changes publish layout+slots atomically; keep existing value/layout revisions until all consumers migrate.
Caches are optional later leaves: domain/layout/slot, never cached values; inherited guards cover every missing-own layout/prototype and dictionary version.
No recycled address alone is identity, strong receiver retention or borrow across JS. Unchecked slots need independent bottleneck evidence and an unsafe audit.

## Value word and lifetime

The future qualified64-bit backend is an opaque non-Copy8-byte NaN-boxed Value; typed views precede replacement of public enum matching.
Reserve high words0xfff8 for immediates and0xfff9 for a low48-bit heap pointer; other negative quiet-NaN tags remain reserved; canonicalize numeric NaNs to0x7ff8000000000000, preserving infinities/signed zero/identity.
Never rewrite raw ArrayBuffer bytes on a Number load; TDZ/empty sentinels never escape. Slot bitmaps avoid widening each slot; capture retains the existing Upvalue.
One allocation holds HeapHeader(strong,weak,kind,destroy)+payload, replacing Rc rather than boxing it. Strings/BigInts/functions/object families share audited ownership.
Cloning increments a count, moves transfer it, borrowed views do neither; last strong destroys once and weak observers hold only tombstones, with no address ABA.
Preserve String uniqueness/COW, iterative deep teardown and escaping lifetimes. Non-atomic core values never become Send/Sync; shared buffers retain their separate feature-gated protocol.
Pointer width/provenance are release blockers: never truncate, qualify each ABI, use a specified provenance-preserving reconstruction and retain enum fallback on unsupported targets.
Unsafe stays localized in word/header modules with explicit allowance and per-block SAFETY proofs (tag/alignment/provenance/kind/count/lifetime/uniqueness).
Safe views retain RefCell checks. Miri/ownership tests supplement conformance; measured transport/count savings justify selection. Cycle collection/arenas are separate, not hidden prerequisites.

## Kernel isolation prerequisite (K0, before M1 resumes)

[T064](../../tasks/T064-kernel-isolation.md) owns the implementation sequence, CI matrix, budget transition and review checklist.
**Invariant:** the hot numeric/dispatch kernel's machine code (bytes and intra-function relative layout of hot blocks) is invariant under object-model and cold-runtime changes.
This is an artifact-enforced contract for a pinned toolchain/recipe, not a Rust compiler theorem or a promise of stable wall time/cache behavior.
Protected scope is typed dispatch shared by Vm/WideLoopFrame, then the wide compact dispatch/numeric core; general Vm and numeric-mutation/compact executors remain separate.
The old29ca position is diagnostic history, not the new reference address. Do not restore it with padding, source-shape recognition or favorable-layout selection.

### Narrow Rust ABI and ownership

Recommend one internal std-only `qjs-kernel` crate, depending on no runtime/AST/property/value crate. It removes object-typed monomorphizations, not merely relocates their source.
It owns scalar/op/cursor types and non-generic `typed_run` / `wide_numeric_run`; translate AST operators once during lowering, preserving all admissions and op semantics.
Protected entries and runtime service entry points use `inline(never)`, verified from emitted artifacts rather than trusted as a guarantee.
Proposed signature: `run(ops: &[KernelOp], scalars: &mut [KernelScalar], cursor: &mut Cursor) -> StopTag` (safe Rust, no extern ABI).
Ops carry register/PC/service-site indices, never Value/ObjectRef/CallEnv/LoopFrame/layout/cache types, destructors, generic callbacks or embedded runtime enums.
One fixed Service(site) tag covers cold operations; adding an object kind must not grow the kernel dispatch enum. Runtime tables retain names, Value roots and caches.
Cursor is loaded once and updated at a boundary; return a small stop tag, not an aggregate Value-bearing action stored/reloaded each numeric op.
Numeric blocks perform no per-op frame resolution, virtual call, allocation or service call. Existing scalar sizing/alignment/op width must not grow without a new measured plan.
Operations needing an external runtime/math call stop for services if its hot fixup cannot satisfy the byte contract; count that cost.
Service ops stop before the effect. Runtime wrappers, outside the protected functions, use non-inlined service entry points and existing authoritative slots/cells/Value roots.
Release kernel/register/RefCell borrows, publish the exact continuation and dirty scalar values before an observer; execute an effect once, then resume or deopt at its original PC.
No raw pointer/Value bit reinterpretation, per-call name table, new GC/root arena or M2 frame-store rewrite is introduced to manufacture an opaque handle.
For wide code, batch consecutive boxed/service ops in the wrapper; numeric projections transfer authority only during the segment, then publish before any callback/throw/exit.
Reuse existing scratch capacity; no extra allocation per short callback, unbounded retained handles, duplicate live binding authority or extra boxing on every numeric op.
Mixed operations retain the old generic fallback at the same instruction; failure before an effect cannot replay a prior effect. This staging may be expensive and must pass the gates below.

### Mechanism choice and costs

| Option | Cost/limitation and decision |
| --- | --- |
| **Recommended: separate non-generic crate + service-yield ABI, original fat/16/abort recipe** | Extra boundary branches, spills, projection/publication and lost property inlining; typed first, wide second. Acceptance requires the static matrix, including linked literals/tables, to pass. A crate alone is not a guarantee. |
| Dedicated CGU / same-crate non-generic helpers | Smaller API migration, but modules do not guarantee CGU ownership and fat LTO can recombine code; useful diagnostic only, not the selected final boundary. |
| No cross-crate LTO / new profile | Stronger compiler separation but may lose T046's engine-wide wins; new recipe/cache identity, policy calibration and compiler study required. Not an automatic fallback or an unmeasured package override. |
| Handler-per-op / tail dispatch | More indirect branches/call-return and lost cross-op optimization; stable Rust does not promise tail dispatch. Reject unbounded native stack growth; no computed-goto/FFI workaround. |
| Linker entry pinning / alignment alone | Does not isolate internal blocks (T054). Cannot satisfy the invariant or select a winning offset. |

Fat LTO crosses crate boundaries; Cargo package overrides cannot set lto, and `false` means local thin LTO whereas `off` disables it ([Cargo profiles](https://doc.rust-lang.org/cargo/reference/profiles.html)).
`inline(never)` is a hint, not a proof ([Rust reference](https://doc.rust-lang.org/reference/attributes/codegen.html#the-inline-attribute)); audit emitted clones/calls and stop if the chosen design still changes.
Kernel-owned dispatch tables/literals belong to the protected closure. Hot RIP references cannot be hidden by normalization; an unstable literal may need an ABI constant input.
Backend jump-table/constant merging or LTO cloning is an explicit feasibility risk: if isolation cannot pass under the unchanged recipe, stop for review of the profile alternative; no pragma/padding search.
No production routing or performance attempt begins on a failed static design. A new compiler profile cannot borrow the old policy's bands or erase the recipe-only cost.

### Static acceptance (no timing)

The proposed `scripts/check-kernel-isolation.sh` and `tools/benchmark/kernel_isolation.py` build clean archives, never run a timing collector; implementation is T064 work.
Reuse the existing structural symbol selector/decoder/raw-byte reader, but extend to full CFGs and ELF as well as Mach-O; do not reuse address-normalized equality as acceptance.
Bind source, kernel/ABI/recipe hashes, features, target, rustc/LLVM/linker/disassembler versions and compiler invocations; require independent A/A rebuild reproducibility first.
Inventory exactly one body per kernel role, all handlers/dispatch edges/stack offsets/owned constants/tables and instruction starts, lengths and offsets from entry; missing/extra/cloned/undecodable roles fail closed.
Compare raw final hot instruction bytes and every hot block's entry-relative offset; compare owned table targets/constant contents and relative positions. Internal branches/RIP/table displacements are never masked.
Only classified external address fixups outside the hot closure may normalize, with equal target identity, operand width, relocation kind and referent; unknown relocations fail.
Uniform movement of the whole protected closure is allowed; same entry, opcode counts, disassembly mnemonics or masked bytes alone cannot pass. Native frame/spill layout and hot callee closure also remain equal.
Check pre-link kernel objects/IR and dependencies to diagnose compiler versus linker drift, but only final linked artifacts satisfy acceptance. Reject Runtime-typed instantiations and imported cold bodies.
The CI perturbations and real-PR comparison use unchanged kernel source/ABI/recipe; a legitimate kernel change needs reviewed fingerprints and the full matrix again, never an automatic golden refresh.
Finite perturbations are regression coverage, not proof for all compiler transformations. Every cold-only PR also compares its actual base/candidate kernel fingerprints.

## Tiers, exits and migration gates

Existing tier order/admission and scalar banks remain; numeric, wide and typed executors are not merged. Virtual objects retain their existing field bank and escape proof.
Exit records retain exact PC/operand depth, slot/cell authority and completion; publish before callbacks and reacquire afterwards. Never materialize an unobservable receiver gratuitously.
T064 must pass static invariance, full configured correctness and complete prospective neutrality gates before integration; static equality cannot waive wall/memory/call-boundary costs.
Stage order/effects/stops are in [T056](../../tasks/T056-interpreter-core-v2.md#ordered-migration). Kernel isolation precedes resumed M1; M2 still waits for M1 acceptance or explicit closure.
