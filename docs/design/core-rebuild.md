# Core rebuild: target architecture

Target for [T077](../../tasks/T077-core-rebuild.md), authorized by the user and
lead on 2026-10-29. **Design only; none of this document establishes implemented
behavior or measured gains.** The current system remains [architecture](../architecture.md).
This replaces the v2/v3 proposed targets and their migration routes. Binding and
suspension invariants remain authoritative in their existing documents.

## Decision and boundaries

Replace subsystems in place inside `qjs-runtime`, landing complete stages on main.
Keep parser/AST, ES2025 semantics, slot locals/shared upvalue cells, builtin family
algorithms, engine jobs and feature-gated agents. No second engine, new crate,
dependency, FFI, Rust async, tracing GC or benchmark recognizer. A stage may use
temporary scaffolding during development; delete it before closing that stage.
Existing, not-yet-replaced accelerators remain only until their named deletion stage.
No runtime operation silently falls back to a replaced implementation.

| Authority | Destination | Detailed contract |
| --- | --- | --- |
| Ownership | Non-Copy owning values; reference counts; separate backup cycle collector | [Memory](core-rebuild-memory.md) |
| Named properties | Runtime `u32` keys, shared shapes or private dictionary, keyless slots | [Objects](core-rebuild-objects.md) |
| Execution | Verified compact register bytecode, one dispatch loop and explicit activation stack | [Execution](core-rebuild-execution.md) |
| Optimizations | Data-flow analyses and specialized instructions in that same loop | [Retirement](core-rebuild-retirement.md) |
| Text | Primary Latin-1/UTF-16; explicit host UTF-8 boundary | [Strings](core-rebuild-strings.md) |
| Migration/acceptance | S1–S7; correctness at every landing; aggregate wall improvement | [T077](../../tasks/T077-core-rebuild.md) |

Builtin semantic code uses owned `Value`, borrowing `ValueView<'a>` and typed
`Context` operations: `to_key`, `get`, `set`, `define`, `delete`, `own_keys`,
`call`, `construct`, `coerce`, `enqueue_job`. Property descriptors are cold API
results, never the common slot representation. No builtin receives mutable shape,
raw word/header, register pointer or an execution-authoritative binding snapshot.
Views end before callbacks; an owned clone may cross any callback or collection.

## Code-grounded starting point

Paths below are relative to `crates/qjs-runtime/src/`; inspected at `6663738f`.

| Actual type/path | Replacement implication |
| --- | --- |
| `value/mod.rs::Value`, `value/access.rs` | 16-byte owning enum; facade has primitive views but is incomplete, not a ready representation firewall. Extend it in S5, not a speculative whole-runtime prerequisite. |
| `value/object.rs::{ObjectData,PropertyStorage,ObjectLiteralShape}` | Four string stores, two revisions, literal name index, separate cold symbol descriptors. Replace storage, not add another index. |
| `value/object/dynamic_storage.rs::DynamicPropertyStorage` | Text index + entries + order duplicate key ownership; all three lookup/order responsibilities move to layout authority. |
| `property/atom.rs::{AtomId,AtomDomain,Atom}` | Already `NonZeroU32`, weak interning and monotonic IDs; extend lifetime/domain ownership and symbol identity, reuse canonicalization. |
| `function/value.rs::{FunctionData,LazyFunctionProperties}`, `value/array.rs::ArrayColdData` | Functions and arrays have independent named tables; ordinary-object-only ICs cannot cover them. |
| `bytecode/ir.rs::{Op,Bytecode}` | Stack IR with owned cold operands, name pool and cached tier plans; `Op` size test is a ceiling, not a measured 112-byte fact. |
| `bytecode/vm.rs::{FrameState,Vm,pop_arguments}` | General calls build nested VMs, vectors and service context; `frame_stack.rs` has unused general callee-push capability. |
| `bytecode/compact_fn/activation.rs::{FrameStorage,run_frames}` | Compact already has an explicit stack and binds a zero-based slice before its inner loop. Reuse this lesson; do not claim all current calls recurse in Rust. |
| `function/call.rs::{call_function,call_function_slice}` | Proxy/bound/native/async/generator routes precede ordinary calls; slice API still allocates on non-leaf fallback. |
| `native.rs::call_native_function` | Cached family plus initial 28-family probing; bind a direct entry once at installation. |
| `bytecode/vm_generator.rs::GeneratorSnapshot` | Owned suspended state; migrate identity and completion, not just PC/registers. |
| `value/js_string.rs::StringData` | Canonical sentinel UTF-8 with mode-specific matcher cache; primary units replace both. |
| `weak_ref.rs`, `weak_map.rs`, `finalization_registry.rs` | Strong-property/Map storage today; real weak edges and ephemerons are required when cycle reclamation becomes observable. |

## Why this order

S1–S2 establish one property authority without changing Value ownership or calls.
That gives every executor and builtin a single API before the execution replacement.
S3 replaces call/bytecode machinery together; bytecode compression alone was nearly
neutral in [T065 ablation](../../tasks/archive/T065-ablation-r1.md). S4 absorbs useful
scalar analyses and deletes the remaining loop executors; it pays for S3 if needed.
S5 then changes Value behind the now smaller executor/API surface. S6 adds cycle
collection only after every owning heap family has a common audited header; S7's
text/RegExp work pays for a neutral collector stage. S5 does not remove RC traffic.

Skipping a large facade-only prerequisite avoids an unmeasured mechanical port
across roughly 133k historical production lines. Object and execution changes can
use today's owning enum. Strings stay last because atoms already canonicalize
code units and can change backing without changing their IDs.

## Evidence limits and challenges

**No deviation from the eight lead defaults is proposed.** Two qualifications
challenge payoff assumptions, not those defaults:

1. **Challenge: shapes/ICs and an 8-byte word do not explain NG's advantage.**
   The pinned NG source inventory in [gap anatomy](../../tasks/archive/T065-gap-anatomy.md#pinned-source-comparison-per-mechanism)
   shows a 16-byte 64-bit JSValue, stack VM, atom/shape lookup, RC/cycle collector;
   [independent review](../../tasks/archive/T065-design-review.md) identifies no per-site ICs.
   Keep the specified IC/register/word destination, but test IC-disabled execution
   and compare real wall time. The cheaper alternative is atom-hash lookups and a
   compact stack VM: less compiler/guard machinery, but retains operand traffic;
   the evidence does not quantify that tradeoff. It is not a second backend plan.
2. **Challenge: object-first is a dependency choice, not the largest measured win.**
   [T068](../../tasks/archive/T068-post-t066-profile.md) prices visible family reach,
   not savings: calls+env 15.63%, all named work 14.19%, ownership 11.95% zero-cost
   envelopes. The lookup oracle helped diff but was unsound for moment; Op shrink
   gave 0.998 wall. Keep object-first to settle shared APIs and delete duplicated
   authority; a call-first alternative moves the largest visible family earlier
   but must port property consumers twice. S2 must demonstrate wall payoff.

Historical instruction geomeans differ: T065 **2.3763× NG** at `9eba7a1d`,
T068 **2.1199×** at `f49be95c`; neither is current HEAD standing. Historical
wall is about 2.6–3× NG; the rebuild baseline at `6663738f` measured development
**2.632×** NG wall ([receipt](../../tasks/diagnostics/T077-baseline.md)).
Shares overlap; T075's dispatch samples (5.7042% whole-process self) are not a
removable instruction budget. No component percentages are added into a forecast.

Surpassing NG remains the campaign goal; T018's 0.50× per-case contract remains
open. Rebuild completion means the architecture and aggregate improvements in
T077, not a claim that merely improving by 20% achieves parity or that target.

## Lead decisions still needed before their dependent stages

No route/collector choice blocks S1. Before S3, settle T074's eval visibility and
scope-instantiation details within the protected cell model. Before S5, approve
the supported packed-pointer target matrix and the host source-API/release
migration for opaque Value; S1's explicit constructor migration is already
specified. High-address rejection must be explicit. These are compatibility decisions, not permission to retain
old execution/storage. Baseline receipt and later measured payoff remain evidence
dependencies; there is no invented terminal speed threshold beyond T018.
