# Environment / Binding Model Rewrite

Status: complete through S6. The final exact all-case comparison reports zero
QuickJS-NG-pass/quickjs-rust-fail cases; campaign verification is recorded in
`tasks/T016-environment-model-rewrite.md`.

T016 is the environment-model keystone named in `AGENTS.md`. It replaces the
old snapshot-and-writeback closure representation with one shared cell per
captured binding. The same rewrite also removes the per-user-call locals
`HashMap` that dominated T011 call cost.

## Completed representation

### Frame locals and captured bindings

- `Vm.locals` remains the dense `Vec<Option<Value>>` fast path for ordinary
  non-captured locals.
- `Vm.local_upvalues: Vec<Option<Upvalue>>` is a parallel sparse side vector.
  A slot classified as captured owns one `Upvalue = Rc<RefCell<Value>>`; local
  loads and stores use that cell instead of the bare value.
- `Function.upvalues: Vec<Upvalue>` holds a closure's received cells by
  compiler-assigned index. `UpvalueSource::ParentLocal(slot)` and
  `UpvalueSource::ParentUpvalue(index)` describe how a child receives each
  cell when it is created.
- A callee's received binding is attached to its local slot during frame
  initialization. The declaring frame, sibling closures, nested closures, and
  suspended continuations therefore read and write the same cell directly.
- Captured `for`/`for-in`/`for-of` lexical bindings receive a fresh cell at
  each specification-required per-iteration environment boundary.

The resolver and VM use slot/upvalue identity, not encoded binding names.
Shadowed names are distinct slots and cells, so class inner names, Annex B
bindings, and nested same-named lexicals cannot alias accidentally.

### Function and realm state

Bytecode functions store a typed shared `Realm` plus indexed upvalues. Their
opaque native-context map is empty; it remains only for native functions that
need closure-like host state. Bound functions inherit the target's creation
realm. Dynamic Function constructors create a detached realm from the required
dynamic scope snapshot.

Named function expressions have one narrow extra cell,
`immutable_env_value`, for the immutable self-name when that outer binding is
not represented as a normal received upvalue. This preserves self-reference in
nested closures and generators without restoring a general name snapshot.

Captured script-global `var` and function bindings use the lazily allocated
`RealmState.binding_cells` table. Closures receive those cells by upvalue index,
and global-object/realm writes synchronize an existing cell. Sloppy
global-fallback slots attach the same cell, so a caller observes a nested
callee's global write immediately instead of retaining a frame snapshot.
Accessor redefinition, deletion, or replacement with an incompatible data
descriptor invalidates the corresponding cached global cell. Hoisted
non-global locals and call-frame bindings are excluded, so
derived-constructor `this`, parameters, and local declarations do not fall
through to unrelated realm values.

Arrow functions capture lexical `this`, `arguments`, and `new.target` through
explicit slots/cells. The declaring ordinary function materializes its own
call-frame slots first; those slots are not incoming upvalues, which keeps
same-named globals from shifting positional upvalue indices.

Sloppy simple functions with a mapped `arguments` object likewise attach
parameter slots to the exact cells held by the mapped index accessors. A
nested helper reading `arguments[0]` therefore sees the current parameter
without caller-name forwarding.

### Call environment

`CallEnv` is no longer the authoritative storage for ordinary lexical
bindings. It carries:

- the shared realm and global lexical metadata;
- private/module/host context;
- a small `FrameBindings` cell vector used by call metadata, native builtins,
  and cold dynamic compatibility paths;
- an optional `DynamicBindings` name-to-cell map for dynamic scope;

The realm binding-cell table belongs to the shared `RealmState`, not to
`CallEnv`. Consequently every call view in a realm sees the same captured
global cell without carrying another per-frame field.

Ordinary user calls create an empty function frame and insert required call
metadata directly into the small vector. They do not allocate a per-call
locals `HashMap`, copy caller locals, refresh a captured snapshot, or write
callee values back to a caller map. Cold consumers may request an owned
`BindingSnapshot`, but snapshots do not participate in lexical identity.

### Direct `eval` and `with`

Direct `eval` and `with` are the only ordinary-language paths that require a
per-frame runtime name map. Bytecode caches `contains_direct_eval` and
`contains_with`; only those frames, or frames inheriting such a dynamic scope,
install the deopt `DynamicBindings = Rc<RefCell<HashMap<String, Upvalue>>>`.
The realm's separate lazy global-cell table is shared across calls and does not
materialize a frame-local environment.

The VM registers live local cells in the deopt map. Eval-created bindings and
with fallback assignments therefore resolve by name while still updating the
exact same cell used by compiled slot access. A function retained from a
`with` body keeps the object-environment stack, but its own local slots stay
closer than that stack; only free names use with-aware bytecode. Fast-path
functions pay no name-map allocation cost.

### Suspension and modules

Generator, async-function, async-generator, and top-level-await snapshots retain
both `upvalues` and `local_upvalues`. Resuming a frame preserves cell identity;
there is no per-step capture refresh or writeback.

Module declaration instantiation creates cells for every top-level module
binding, including unexported bindings captured by a hoisted function. The
body frame reuses those cells and does not recreate the already-instantiated
function object. Imports attach directly to the exporting module's cell, while
each module retains one shared host so declaration instantiation, the body,
nested functions, and dynamic imports observe one `import.meta` identity.

This explicit module cell graph is a live-binding mechanism, not a closure
snapshot compatibility path. Module writes are limited to the canonical
top-level slot identity so a nested same-named lexical cannot overwrite an
export cell.

Compiler-owned `\0\0` temporary slots never participate in lexical capture.
This prevents nested rest/default-parameter functions from aliasing an outer
function's same-named prologue temporary.

## Removed model

Before T016, one logical binding could exist simultaneously in a VM slot,
`Function.env`, a shared name-keyed `captured_env`, and a
`CaptureWriteback` chain. Correctness depended on refresh/writeback passes
covering every call, closure creation, loop, generator, async, class, and eval
path. User calls also rebuilt a `HashMap<String, Value>` through
`function_env`/`with_frame_locals`.

S5 deleted:

- `Function.env`, `Function.captured_env`, and `CaptureWriteback`;
- the `function/captures.rs` snapshot/writeback implementation;
- the VM captured-environment stack and refresh/writeback family;
- `PushCapturedEnv`/`PopCapturedEnv` loop operations;
- `CallEnv`'s locals `HashMap`, caller-local forwarding, and
  `with_frame_locals`/`with_current_frame_locals` cloning;
- bytecode-function copies of runtime intrinsics in native context.

S6 replaced the remaining direct-eval/with compatibility bridge with the
explicit gated name-to-cell map described above.

## Slice history

- **S1:** introduced `Upvalue`, resolver classification, indexed
  `UpvalueSource`, and call microbenchmarks.
- **S2:** switched the simplest captured lexical bindings to shared cells.
- **S3:** covered shadowing, sibling/nested closures, class names, and fresh
  per-iteration cells; removed generic lexical-name mangling.
- **S4:** covered parameters, `var`/function declarations, generators, async
  functions/generators, and top-level-await suspension.
- **S5:** cut over functions/calls/realms completely and deleted the old
  capture/writeback model.
- **S6:** installed the gated direct-eval/with name-to-cell deoptimization
  path.

## Performance result

The S1 baseline measured a user function call at about 70 us versus about
0.12 us in QuickJS-NG. A later pre-cutover measurement was 11.72 us for
`function_call` and 9.77 us for `closure_call`. Five final post-cutover runs
produced stable medians of 6.84 us and 4.88 us respectively, versus 0.122 us
for both in QuickJS-NG.

That is about 90.2%/93.0% faster than the original 70 us baseline and
41.7%/50.0% faster than the later pre-cutover measurements. The remaining
roughly 56x/40x gap is outside T016 and requires profiling-driven
VM/allocation work rather than another environment snapshot heuristic.

## Final conformance result

The final `./scripts/find-qjsng-gaps.sh --exact --all` scan is recorded at
`target/test262-gaps/all-20260714-053539-93994`:

- 53,572 total Test262 cases and 42,672 configured cases;
- quickjs-rust: 42,591 pass, 0 fail, 20 timeout, 61 not run;
- QuickJS-NG-pass/quickjs-rust-fail: **0**;
- all 20 timeouts are already classified as excluded slow cases;
- all 61 not-run cases require the Test262 `$262.agent` harness and are not
  environment-model engine failures.

This closes T016's engine-difference queue without claiming that the excluded
timeouts or unexecuted agent cases have passed Test262.

## Remaining structural risks

1. `Rc` cycles remain possible when a cell contains a function that owns the
   same cell. The planned tracing GC/arena work must collect these cycles.
2. `DynamicBindings` is intentionally more expensive. Any new use must be
   justified by genuinely dynamic name resolution and remain gated away from
   ordinary calls.
3. Call-frame internals such as `this`, `arguments`, and `new.target` must stay
   frame-local unless compiler analysis proves that a nested arrow captures
   them.
