# Environment / Binding Model Rewrite

Status: proposed design, 2026-06-21. Implementation tracked by
`tasks/T016-environment-model-rewrite.md` (slices S1+). This rewrite is the
keystone named in `AGENTS.md`: it is the shared root of the remaining
closure/`eval`/method capture-staleness failures *and* the per-call allocation
cost, and it subsumes `tasks/T014-var-closure-binding-staleness.md` and
`tasks/T011-call-performance.md`. Keep this document in sync as slices land.

## How the binding model works today (relevant facts)

References verified 2026-06-21 against `crates/qjs-runtime`.

- `Value` (`value/mod.rs:23`) is an enum; `Function`, `Array`, `Object`,
  `Proxy`, `Map`, `Set` hold `Rc`/`Rc<RefCell<…>>`. There is no boxed-binding
  variant.
- A frame's locals are slot-indexed already: `Vm.locals: Vec<Slot>` where
  `Slot = Option<Value>` (`bytecode/vm.rs:56`), seeded by
  `initial_slots` (`bytecode/vm_bindings.rs:146`). Reads/writes use
  `Op::LoadLocal(slot)` / `Op::StoreLocal(slot)` (`vm_bindings.rs:593`, `:648`).
- Captured bindings, however, are **name-keyed and snapshot-based**:
  - `Function.env: HashMap<String, Value>` (`function/value.rs`) is a
    creation-time snapshot built by `function_capture_env`
    (`bytecode/vm_capture.rs:19`).
  - `Function.captured_env: Rc<RefCell<HashMap<String, Value>>>` is a cell
    *shared* between sibling closures created in the same frame, so a write by
    one is seen by another (`vm_capture.rs:235` `refresh_captured_env`).
  - `Function.capture_writeback: Option<CaptureWriteback>`
    (`bytecode/vm_generator.rs:51`: `{ target, names, aliases, parent }`) is a
    heuristic descriptor that propagates a closure's writes back up to the
    declaring frame's cell, with a `parent` chain for nesting.
  - `CallEnv.locals: HashMap<String, Value>` (`function/env.rs:44`) is the
    name-keyed env layer rebuilt per call by `with_frame_locals`
    (`env.rs:464`).
- Keeping the snapshot, the shared cell, and the slot copy coherent requires a
  family of refresh passes in `vm_capture.rs`: `refresh_captured_env`,
  `refresh_locals_from_captured_env`, `refresh_live_locals_from_captured_env`,
  `refresh_shared_captured_locals_after_call`, `refresh_call_env_from_captured_env`,
  plus `write_through_capture_writeback_slot` (`vm_bindings.rs:204`). Blast
  radius today: ~55 `captured_env` sites and ~23 `capture_writeback` sites
  across 12 files.
- Names that shadow an outer binding are stored under a mangled key
  `\0lexical:<name>:<slot>` (`compiler_lexical.rs:208` `lexical_storage_name`);
  the refresh/eval paths must unmangle these (`vm.rs` `unmangle_lexical_storage_name`),
  and several known failures (M2 class-method capture, direct-`eval` capture)
  come from a path that resolves the plain name and misses the mangled slot.

### Why this is the root of two problem classes

1. **Correctness (capture staleness).** Because a binding lives in three places
   (frame slot, snapshot `env`, shared `captured_env`) kept in sync by
   heuristics, any path the heuristics do not cover desyncs: a sibling/forwarded
   closure writes the cell but the declaring frame reads its stale slot
   (T014, fixed only for the leaf case), a class method stores its inner name
   under a plain key while the constructor uses the mangled key (M2), a
   per-iteration `let` is captured by value instead of by fresh cell,
   generators re-import a stale snapshot on resume, and a direct `eval` resolves
   a free name dynamically but the capturing closure does not.
2. **Performance.** Every call rebuilds a `HashMap<String, Value>` of locals and
   clones caller bindings into it (`with_frame_locals`, `function_capture_env`).
   Under nested-call load this is the dominant cost and the source of the ~536
   `TypedArray/*` timeouts (T011).

A single mechanism — a **shared cell per captured binding, resolved by index** —
removes all three storage copies and every refresh/writeback heuristic at once.

## Target model: slot-indexed locals + indexed upvalue cells

The standard closure representation (Lua upvalues, V8 context slots, QuickJS
`var_ref`). One concept replaces the snapshot + cell + writeback trio.

- `type Upvalue = Rc<RefCell<Value>>` — one heap cell per *captured* binding.
  (Stays `Rc<RefCell<…>>` until the GC slice of the perf track lands; the public
  shape is `Upvalue`, so the backing store can change without touching call
  sites.)
- A local slot the compiler proves is captured by some inner closure is a
  **cell slot**: `Slot` becomes
  `enum Slot { Empty, Value(Value), Cell(Upvalue) }` (or the declaring frame
  keeps `Vec<Option<Upvalue>>` for the captured subset alongside the existing
  `Vec<Option<Value>>` for the non-captured majority — decided in S1 by
  benchmark; most slots are never captured and must stay a bare `Value`).
- A `Function` stores `upvalues: Vec<Upvalue>` — the cells it closed over,
  indexed by a compile-time **upvalue index**, *not* a name. `Function.env`,
  `Function.captured_env`, and `Function.capture_writeback` are deleted.
- New ops: `Op::LoadUpvalue(u16)`, `Op::StoreUpvalue(u16)` for closure access to
  a captured outer binding; `Op::LoadCellLocal(slot)`/`Op::StoreCellLocal(slot)`
  (or fold into `LoadLocal` via the `Slot::Cell` arm) for the declaring frame's
  access to its own boxed local. `Op::NewFunction` carries
  `upvalue_sources: Vec<UpvalueSource>` where
  `enum UpvalueSource { ParentLocal(slot), ParentUpvalue(u16) }` — at closure
  creation the VM reads each source from the *current* frame (its cell slot or
  its own upvalue vector) and pushes the shared `Rc` into the new function's
  `upvalues`. No names, no HashMap, no snapshot.

Reads/writes of a captured binding now go straight through the one shared cell,
so siblings, forwarded calls, generators-on-resume, and the declaring frame all
observe every write with zero refresh passes.

## Compiler changes (`qjs-parser` unaffected; AST unchanged)

- Resolver pass (extends the existing scope analysis that already produces
  `lexical_captures`): classify every binding as **plain** (never captured) or
  **cell** (captured by ≥1 nested function). Assign cell slots and, per nested
  function, an ordered `upvalue_sources` list. This replaces
  `closure_referenced_global_names` / `closure_written_binding_names` /
  `lexical_captures` (`bytecode/ir.rs`, `compiler_lexical.rs`).
- Per-iteration `let`/`const` in loops: emit a fresh cell per iteration
  (the spec's per-iteration environment) — a new cell allocation at the loop
  back-edge, which is exactly the correct semantics and removes the
  `compiler_control.rs` per-iteration captured-env juggling.
- The `\0lexical:<name>:<slot>` mangling (`compiler_lexical.rs:208`) is **deleted**:
  shadowing is now expressed by distinct slot indices, so name collisions cannot
  occur and the unmangling paths in `vm.rs` go away.
- `with` and direct `eval` (the two dynamic-scope escape hatches) keep a
  name-keyed fallback: a function containing a direct `eval` is compiled in a
  "deopt" mode where its captured bindings are *also* exposed by name (the cells
  are registered in a name→cell map handed to the eval'd code). This is the one
  place a name map survives, and it is gated on `bytecode.contains_direct_eval`.

## VM changes

- `Vm.captured_env`, `Vm.captured_env_stack`, `Vm.parameter_captured_envs`,
  `Vm.capture_writeback` are deleted. A frame gains `upvalues: Vec<Upvalue>`
  (its own closed-over cells, moved in from `Function.upvalues` at entry).
- `initial_slots` (`vm_bindings.rs:146`) allocates an `Upvalue` cell for each
  cell slot (seeded `Undefined` or the hoisted value); plain slots stay bare.
- The entire refresh/writeback family in `vm_capture.rs` and
  `write_through_capture_writeback_slot` in `vm_bindings.rs` is removed.
- `Op::NewFunction` handler (`vm.rs:471`): build `upvalues` by reading each
  `UpvalueSource` from the current frame; drop `function_capture_env`,
  `insert_lexical_captures`, `capture_writeback_for_bytecode`,
  `refresh_captured_env`.
- `CallEnv` (`function/env.rs`) keeps only what is genuinely cross-call and
  not a local: realm handle, global-lexical/immutable sets, private
  environment, module host/imports, catch bindings. Its `locals: HashMap` and
  the `activation_captured_env` / `captured_binding_source_env` /
  `parameter_captured_envs` fields are removed; `with_frame_locals` no longer
  clones a per-call locals map — the perf win.
- Generators/async (`vm_generator.rs`, `async_function.rs`): a suspended frame
  already owns its `locals`; it now also owns its `upvalues: Vec<Upvalue>`.
  Because cells are shared `Rc`, resume sees caller writes for free — the
  per-step capture write-back the generator design called out (risk #2 there)
  is deleted.

## Migration / slice plan (each slice = one reviewable unit, gated)

Every slice runs `./scripts/check.sh` and `./scripts/compare-qjs.sh` before
push (runtime semantics), and reverts on any regression — the ~42k passing
cases are the safety net. Slices are ordered so the engine stays green at each
step; the name-keyed model and the cell model coexist until S5 deletes the old
one.

- **S1 — Upvalue type + resolver classification (no behavior change).** Add
  `Upvalue`, the `UpvalueSource`/cell-slot resolver output, and the new ops, but
  keep emitting the old capture path; gate the new path behind an internal
  flag, off by default. Land the benchmark harness for the call path
  (baseline numbers for T011).
- **S2 — Cell slots for the simplest case: a captured non-shadowing `let`/
  `const` read+written by one nested function.** Switch only this class to
  cells end-to-end; everything else stays on the old path. Gate: the T014
  counter-callback idiom and `closure_state` tests go green with the new path
  on for this class.
- **S3 — Shadowing + multiple/nested closures + per-iteration loop cells.**
  Delete `\0lexical` mangling for cell slots; cover M2 (class-method inner
  name) and per-iteration `let`. Gate: the M2 and per-iteration Test262 slices.
- **S4 — Generators/async + parameter-scope captures.** Move `upvalues` into
  the suspended frame; delete the per-step generator write-back.
- **S5 — Delete the old model.** Remove `Function.env`/`captured_env`/
  `capture_writeback`, the `vm_capture.rs` refresh family, the `CallEnv` locals
  HashMap, and `with_frame_locals` cloning. This is the slice that realizes the
  T011 perf win; record a burndown entry and re-measure the call benchmark.
- **S6 — direct-`eval` / `with` deopt path on cells.** Replace the dynamic
  name resolution with the name→cell deopt map; close the eval-capture failures.

## Risks

1. **Blast radius.** ~90 sites across 12 files. Mitigation: the flag-gated
   coexistence (S1–S4) so each class flips independently with its own gate;
   never a big-bang cutover.
2. **Cell-slot over/under-classification.** A binding wrongly classified plain
   that is actually captured → stale; wrongly classified cell → a needless
   allocation. The resolver must be conservative-correct (capture ⇒ cell) and
   is the single highest-value thing to unit-test directly (S1).
3. **Rc cycles.** A cell holding a `Function` that holds the same cell leaks
   under `Rc`. This is the existing engine behavior, made more uniform here; the
   GC slice on the perf track (separate, post-S5) is what actually collects it.
   Document, do not block on it.
4. **`with`/`eval` dynamic scope.** The one place names must survive; keep it
   explicitly gated on `contains_direct_eval`/`with` so the fast path never pays
   for a name map (S6).
5. **`arguments`, `this`, `new.target` and other call-frame internals** must
   stay frame-local and never become cells unless actually captured by an arrow
   — the current `is_call_frame_binding` guard logic moves into the resolver's
   classification.
