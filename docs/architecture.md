# Architecture

Where does execution and state live today? This file is the map; detail lives
in the module headers it names and in two invariant notes,
[design/env-model-rewrite.md](design/env-model-rewrite.md) (bindings) and
[design/generator-suspension.md](design/generator-suspension.md)
(suspension). How to verify a change is in [harness.md](harness.md).

```text
source text -> qjs-lexer (tokens with spans) -> qjs-parser (qjs-ast)
            -> qjs-runtime (bytecode compiler, VM, builtins) -> qjs-cli
```

## Crates

- `qjs-ast`: syntax and span types (`expression`, `statement`, `class`,
  `span`). Depends on no other workspace crate.
- `qjs-unicode`: Unicode tables shared by the lexer and the runtime.
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
`scripts/bootstrap.sh` initialises only these two top-level submodules;
QuickJS-NG's own nested `test262` is not used.

## Execution

Paths below are relative to `crates/qjs-runtime/src/`. The source-local map of
the executors is the module header of `bytecode/mod.rs`; each module's header
states its admission rule and why its fallback is unobservable.

**Pipeline.** `bytecode/compiler*.rs` lower a parsed script or function to a
`Bytecode` (`bytecode/ir.rs`): a stack-machine instruction stream plus the
slot table for its locals. `Vm` (`bytecode/vm.rs`) interprets one `FrameState`
at a time. Everything below is an optional faster route to the same
semantics. Each one decides admission before any observable step, and
otherwise leaves the work to the interpreter.

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

**Per-site caches.** `bytecode/named_property_cache.rs` caches named property
reads and writes per instruction site, keyed by receiver layout.
`bytecode/enumerate_keys_cache.rs` caches a `for-in` key list while the whole
prototype chain keeps its identities and layouts. Both validate against the
object revisions described below.

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
  a `Value` never deep copies.
- `ObjectRef` is `Rc<ObjectData>` and `ArrayRef` is `Rc<ArrayData>`. Identity
  is pointer identity. Caches hold `ObjectWeakRef`/`ArrayWeakRef`, never a
  strong reference. There is no tracing collector: a reference cycle is
  never freed.
- `ObjectData` holds the fields every object needs inline. Rare state
  (symbol-keyed properties, generator state, private names, ArrayBuffer and
  TypedArray slots) sits behind a lazily allocated `ObjectColdData` box.
- String-keyed properties use one of four `PropertyStorage` forms: `Small`
  (an insertion-ordered vector, up to 12 properties), `Dynamic` (a boxed
  slot vector with a name index and an order list), and `Shaped` or
  `ShapedPair` for object literals, which share one `ObjectLiteralShape` key
  layout and store only values or descriptors per object.
- `Property` is 32 bytes: value and flags inline, the accessor pair behind an
  optional shared `Rc`.
- Each object has two counters. `property_revision` advances on any own
  string property change. `layout_revision` advances only when a slot index
  could change (insert, delete, descriptor replacement), so a slot-keyed
  cache survives ordinary field assignment.
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
