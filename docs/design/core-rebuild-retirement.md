# Core rebuild: one optimizer and executor retirement

Target for [core rebuild](core-rebuild.md). All paths below are under
`crates/qjs-runtime/src/`. Deletion is an exit condition, not an optional cleanup.

## Inventory and disposition

| Present mechanism | Source evidence / useful behavior | Disposition |
| --- | --- | --- |
| General stack interpreter | `bytecode/vm.rs`, `vm/general_ops.rs`, `vm/rare_ops.rs`; semantic owner today | S3 replaces dispatch with universal register code; retain semantic algorithms ported to Context/continuations, delete old stack execution and dormant `frame_stack.rs` machinery. |
| Numeric closed-form leaf | `vm_numeric_leaf.rs`, `vm_numeric_leaf_registers.rs`, `Bytecode::numeric_leaf_plan` | S3 delete evaluator/plan/admission; common arithmetic and constant folding express the operations, no separate call answer. |
| This-property leaf | `vm_this_property_leaf.rs`, `Bytecode::this_property_leaf_plan` | S3 delete; new own-slot IC load plus ordinary Return covers it without a pre-call body recognizer. Field initializer helpers use the same code/constant metadata, not a hidden leaf executor. |
| Caller-env activation | `compact_fn/activation.rs::{try_run_in_caller_env,shares_caller_environment}` and wide equivalent | S3 delete equal-environment proofs and service-env borrowing; every activation carries typed context without CallEnv. |
| Compact register tier | `compact_fn/{compile,execute,activation}.rs`, creation cache and `compact_inline_facts` | S3 delete executor/pools/facts/negative admission cache; reuse proven lowering ideas inside the universal compiler. |
| Numeric compact plan | `compact_fn/numeric_plan/`, `compact_numeric_plan` | S3 delete plan/driver/entry probes; generic Number operations initially replace it, S4 scalar instructions recover unboxed chains. |
| Wide tier and handback | `compact_fn/wide/`, `vm/wide_resume.rs`, `wide_inline_facts` | S3 delete alternate driver/windows/handback and recovery translations; generic properties/calls/loops already run in common frames. |
| Numeric mutation programs | `vm_numeric_mutation_loop/`, frame-local override plans | S4 subsume arithmetic/index guards with common typed instructions; delete dense/nested/predicate plan executors and backedge probes. |
| Typed loop | `typed_loop/`, `vm_loop_dispatch.rs`, decline masks | S4 retain general type/data-flow analysis as compiler passes; delete region driver, standalone scalar banks, admissions and deopt-to-old-VM path. |
| Virtual object stream | `virtual_object/`, `vm_virtual_object.rs`, `FrameState::virtual_values` | S3 lowers its semantics into the single register Code, deleting parallel executable streams/selector and old handlers; S4 consolidates escape/materialization analysis into the common optimizer. |
| Unique string append | `vm_string_append.rs`, JsString COW | Retain as one semantic string operation, port to owning word S5 and unit storage S7. Evidence: changing ownership must preserve uniqueness; no independent executor or caller admission. |
| Named/enumeration caches | `named_property_cache.rs`, `enumerate_keys_cache.rs` | S2 replace with shape/validity slot caches; enumeration uses same structural dependencies. Retain no legacy exact-value variant; weak prototype constants obey the [IC contract](core-rebuild-objects.md#inline-caches). |
| Native entry service | `native.rs`, `native/entry.rs` | S3a replaces family probing/cache with installed operation entries and borrowed arguments; S3b replaces their CallEnv service parameter with Context. Builtins retain object-family source boundaries. |

The existing compact driver really does share its own explicit stack; the general
VM does not. S3 deletes both ownership regimes and their boundary calls rather
than adding a third driver. Existing typed/mutation executors remain only until
S4 and are ported directly to the common frame API. Compile their metadata from
ephemeral IR, link exits to verified Code PCs and release that IR; no cached old
Op stream, legacy frame, CallEnv bridge or new compatibility executor survives S3.
An exit resumes common Code once, never deleted bytecode. S4 removes these
remaining execution owners; useful state maps remain part of the common API.

## General optimizations inside one loop

S3 starts with owning Value registers and inlined primitive tag checks. S4 adds
compiler-proven scalar registers and instructions (`AddF64`, compare/branch,
guarded indexed load/store, Box/Unbox), all dispatched by that same PC loop.
`ScalarBank { numbers:Vec<f64>, initialized:bitmap }` is frame-owned; Code maps
scalar temporaries and any observable binding locations. There is no per-call
numeric-plan eligibility ladder, whole-body scalar interpreter, separate loop
driver or speculative dual binding authority.

Type guards occur before effects. Failed guards branch to general instructions
in the same Code with an explicit materialization map. Publish affected cells
before callbacks, throw, capture, dynamic eval or suspension; re-check after
re-entry. A live binding cannot remain both a stale boxed value and an
authoritative scalar. Captured/imported/dynamic bindings use their canonical
cells unless a short segment's exclusive ownership is proved. Retain exact -0,
NaN, integer overflow, BigInt/coercion errors, holes, prototype indices, buffer
detach/resize and alias semantics. No effect is replayed on a guard failure.

Virtual objects are compiler SSA-like identities with fields, escape sites and
materialization order. Materialize exactly once before identity observation,
callback, throw/suspension or escape. Alias/CFG merges and cell captures must
retain identity. If proof is absent, allocate normally in the same core; no
call-time recognizer and no fallback executor. Analyses admit operations, types,
aliasing and data flow, never source text, package names or bytecode templates.

## Evidence and release discipline

[T075](../../tasks/archive/T075-executor-anatomy.md) attributes 39.3127% of whole
main-thread self time to six executors, including transport 7.9273%, dispatch
5.7042%, activation 4.5870%. Those are sampled contexts, not removable costs.
[T076](../../tasks/archive/T076-executor-bundle.md) independently failed all
three frozen instruction filters; none of those candidate branches is imported
as an accepted optimization. [T021](../../tasks/archive/T021-single-vm-frame-stack-log.md)
also shows fewer opcodes can lose overall. Therefore retain useful analyses by
semantic purpose, measure their new implementation as a whole, and delete their
old executors even when a local suite case regresses under the lead's overall rule.

Before S3/S4 closes, audit `Bytecode` plan fields, `FunctionData` eligibility bits,
TLS pools, frame decline masks, counters and compiler output. A source inventory
must find no deleted entry point or cached rejection machinery. Test flags may
disable ICs or optimization passes; they execute the same bytecode engine, not
the old VM. General numeric guards are allowed; per-call tier probes that decline
on ordinary functions are gone.

T064's failed machine-code isolation is history, not an S1/S2 prerequisite.
S1/S2 still measure the complete portfolio because object layout changes can
perturb old typed/wide machine code before retirement. Do not add a kernel crate,
padding, address selection or a new ABI to enforce impossible Rust/LTO isolation.
