# Architecture

Where does execution and state live today? This file is the map; detail lives
in the module headers it names and in two invariant notes,
[design/env-model-rewrite.md](design/env-model-rewrite.md) (bindings) and
[design/generator-suspension.md](design/generator-suspension.md)
(suspension). How to verify a change is in [harness.md](harness.md).

The proposed replacement architecture is [core rebuild](design/core-rebuild.md);
[T077](../tasks/T077-core-rebuild.md) owns its staged migration. Nothing below
claims that target is implemented. The existing String/RegExp invariants are in
[string-code-units](design/string-code-units.md); [T057](../tasks/T057-string-code-units.md)
records what is implemented and verified.

```text
source text -> qjs-lexer (tokens with spans) -> qjs-parser (qjs-ast)
            -> qjs-runtime (bytecode compiler, VM, builtins) -> qjs-cli
```

## Crates

- `qjs-ast`: syntax and span types (`expression`, `statement`, `class`,
  `span`). Depends on no other workspace crate.
- `qjs-unicode`: Unicode tables and canonical WTF-16 text builders shared by
  the lexer and the runtime (`wtf16`).
- `qjs-lexer`: UTF-8 source to tokens (`scanner`, `token`, `error`). Every
  token carries a byte span.
- `qjs-parser`: tokens to AST (`expression`, `statement`, `cursor`). It is
  deterministic, never evaluates code, and returns structured errors.
- `qjs-runtime`: compiles the AST to bytecode and executes it. It never
  evaluates AST nodes, and re-parses (for `eval`, `Function`) only through
  the parser's public API. `value` holds values, objects and property
  descriptors; `function` holds function objects, the call path and
  `CallEnv`; `bytecode` holds the compiler, the IR and every executor; the
  remaining modules are builtins grouped by standard object.
- `qjs-cli`: thin command-line wrapper; no engine semantics.

`third_party/quickjs-ng` (behavioural oracle and reference) and
`third_party/test262` (conformance input) are pinned submodules outside the
Cargo workspace. They are read-only and never a build dependency.

## Execution

Paths are relative to `crates/qjs-runtime/src/`; `bytecode/mod.rs` and executor
headers own their source maps, admission rules and fallback invariants.

**Pipeline.** `bytecode/compiler*.rs` lower a parsed script or function to a
`Bytecode` (`bytecode/ir.rs`): a stack-machine instruction stream plus the
slot table for its locals. `Vm` (`bytecode/vm.rs`) interprets one `FrameState`
at a time. Everything below is an optional faster route to the same
semantics. Admission precedes observable work; declined paths use the interpreter.

**Native entries.** `native/entry.rs` binds each operation at installation. `CallInfo`
borrows callee/receiver and records construct mode; CallEnv supplies lazy realm/
new.target services until S3b. Host/callback and compact/wide entries borrow slices.
Generic VM calls/construct detach their operand owner through return/throw,
restoring it before exception handling. Fixed-arity JS admission keeps its order.
Retained values/forwarding lists own references. S3d owns stack policy; S3a adds no limit. No Executor/RefCell borrow crosses callbacks.

**Call dispatch.** `call_function` (`function/call.rs`) first resolves the
callable kind: Proxy, bound function, native builtin, generator, async
function, class constructor. For an ordinary bytecode function it then tries,
in order:

1. If the function passes the direct-leaf predicate (`is_direct_leaf_function`:
   among other conditions no `eval`, `with`, `super` or live dynamic scope,
   identifier-only parameters, no `arguments` object, no capturing closure,
   not a generator or async function), `call_direct_leaf_function` tries:
   1. the numeric leaf, a closed-form evaluation of a small body over
      numbers, booleans and `undefined` (`bytecode/vm_numeric_leaf.rs`);
   2. the this-property leaf, a closed-form evaluation of a body that only
      combines own data properties of its receiver or arguments
      (`bytecode/vm_this_property_leaf.rs`);
   3. a compact tier in the caller's own environment, when that environment
      equals the one the callee's frame would get (`try_run_in_caller_env`,
      `bytecode/compact_fn/activation.rs`): the numeric compact tier, then
      the wide tier;
   4. a slot-seeded frame (`eval_direct_call_bytecode`, `bytecode/vm.rs`).
      It first offers the body to the compact tiers standalone: the numeric
      compact tier, which tries its `f64` plan
      (`bytecode/compact_fn/numeric_plan.rs`) when every argument is a
      number, or the wide tier (`bytecode/compact_fn/wide/`) for a body the
      numeric tier does not admit. Only then does it build an interpreter
      frame, with arguments written straight into parameter slots.
2. Otherwise the general path (`prepare_bytecode_call`) builds the call
   environment and runs an interpreter frame.

The interpreter's own call instruction applies the same predicate
(`bytecode/vm_call.rs`), and natives that call back into user code use
`call_function_slice`. `new` on a plain function has a matching
direct route (`try_construct_direct_leaf_function`).

The compact tiers execute a whole body on registers without a `FrameState`.
Both run admitted callees in a window of their own register stack and
re-enter `call_function` for everything else. The wide tier can also exit to
an interpreter frame mid-body, and take a loop back from it
(`bytecode/vm/wide_resume.rs`).

`bytecode/frame_stack.rs` is the driver every activation runs under
(`run_completion`). Its ability to push a callee frame onto the same `Vm` is
dormant: nothing in production requests it, so each interpreter-frame call is
a nested `Vm` on the Rust stack.

**Loop dispatch.** At a backward jump the interpreter consults
`bytecode/vm_loop_dispatch.rs`, which tries two accelerators in order and
falls through to a plain jump when both decline:

1. numeric-mutation plans (`bytecode/vm_numeric_mutation_loop/`): register
   programs over dense arrays and TypedArrays, nested dense loops, hole-tail
   append, and a dense predicate scan;
2. the typed loop (`bytecode/typed_loop/`): any loop region built from a
   whitelist of instructions, compiled to scalar and boxed registers, with
   helper calls flattened into the program. It hands the loop back to the
   interpreter at the exact instruction it cannot complete.

Plans are compiled once per `Bytecode` on first use; a frame that
deoptimises a numeric-mutation plan takes a private copy.

**Compile-time lowering.** `bytecode/virtual_object/` proves which object,
array and function literals have no observable identity and produces a
lowered instruction stream that keeps their fields in a frame-local value
bank. The stream is chosen per activation; its instructions are handled by
the ordinary dispatch (`bytecode/vm_virtual_object.rs`).

**In-place string append.** Before a compound string assignment the
interpreter drops its own extra references to the target, so a uniquely held
buffer is extended in place; a real JavaScript alias keeps the buffer shared
and immutable (`bytecode/vm_string_append.rs`).

**Named operands.** A named opcode indexes its body's pool and links it once
per `AtomDomain` (`bytecode/linked_code.rs`). Ordinary operations use KeyLease
IDs; host text boundaries resolve leases once. Strings and symbol identities
share recyclable IDs; live leases prevent reuse. Domain indices are weak.

**Creation inputs.** Realm-aware constructors receive stable intrinsic prototypes;
low-level builders receive explicit prototype/domain inputs, including null.
Realm roots own PropertyOwner outside the domain; layouts retain the domain.
Native functions share empty native state per domain and copy it on mutation.

**Per-site caches.** `bytecode/named_property_cache.rs` caches named property
reads and writes per instruction site, keyed by receiver layout.
Layouts include prototype identity; shared guards use allocation identity,
dictionaries also use versions. Inherited, missing, accessor, enumeration and
creation proofs share per-prototype validity generations with a weak user graph.
The [object contract](design/core-rebuild-objects.md) owns these invariants.

## State

**Bindings.** A local lives in a frame slot; a captured binding lives in one
shared cell; every realm binding has one cell in the realm. No per-call
name-keyed snapshot exists. See
[design/env-model-rewrite.md](design/env-model-rewrite.md).

**Suspended frames.** A generator or async body owns its whole frame while
suspended. See [design/generator-suspension.md](design/generator-suspension.md).

### Object representation

Defined in `value/mod.rs`, `value/object.rs`, `value/property.rs` and
`value/array.rs`; the size and behaviour tests are beside them.

- `Value` is a 16-byte enum. Every heap variant (string, BigInt, function,
  array, object, proxy, Map, Set) is a reference-counted handle, so cloning
  a `Value` never deep copies. `value/access.rs` is the representation-
  independent facade (`ValueKind`, typed borrowed views) that callers outside
  `value/` should use instead of matching variants.
- `ObjectRef` is `Rc<ObjectData>` and `ArrayRef` is `Rc<ArrayData>`. Identity
  is pointer identity. Caches hold `ObjectWeakRef`/`ArrayWeakRef`, never a
  strong reference. There is no tracing collector: a reference cycle is
  never freed.
- `ObjectData` owns NamedStore, a cold pointer, index count and packed flags;
  NamedStore owns the live prototype. Buffer, namespace, generator and private
  state remain cold. Sizes are asserted in the object contract.
- Ordinary string and symbol properties share one `NamedStore`
  (`value/layout/`): immutable shared shapes or a private dictionary, one ID
  index, keyless values and cold shared accessor pairs. Property is a 32-byte
  descriptor API result. No literal-specific payload or ordinary symbol table.
- Literal input-slot metadata is compiler-owned; its per-domain layout links
  feed the same transition tree as constructors, class fields, JSON and hosts.
  String virtual indices remain exotic and do not materialize/intern on reads.
- Arrays/functions embed a lazy named authority with explicit prototypes and
  canonical empty layouts; probes allocate no payload or unrelated defaults.
  Materialized descriptors and order live in the same NamedStore.
- An array stores its elements in a `Vec<Value>`; holes are recorded in a
  lazily allocated side set.

## Errors and spans

Malformed JavaScript never panics: each layer returns a structured error,
with a source span where it has one. A panic is acceptable only for an
internal invariant that indicates an engine bug. Spans are byte offsets into
the original UTF-8 source.

## File shape

Add a language feature vertically: token, AST type, parser test, bytecode
lowering, runtime behaviour, focused test. Split a file by semantic
responsibility before it approaches the `scripts/check-file-size.sh` limits;
those are an upper bound, not a target. `scripts/source-size-report.sh`
reports first-party file sizes, and vendored ones only with `--vendor`.
