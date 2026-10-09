# Binding Model Invariants

What binding identities must any implementation preserve, and where do they
live today? This describes the code, not a plan; the T016 migration record is
[design-env-model-migration.md](../../tasks/archive/design-env-model-migration.md).
Source paths are relative to `crates/qjs-runtime/src/`; tests are named
`tests/<file>.rs::<test>`.

## The rule

One JavaScript binding has exactly one storage location for its whole life.
Everything that can observe the binding reads and writes that location.

`AGENTS.md` protects this as an architectural boundary: never reintroduce a
per-call name-keyed snapshot. A snapshot gives one binding two locations, so
every call, closure creation, loop iteration, suspension and `eval` needs a
refresh and write-back pass, and any path that misses one reads a stale
value. It also makes every call build a name map the callee may never use.

## Where a binding lives

- A local nothing else can reach: `FrameState.locals[slot]`
  (`bytecode/vm.rs`), seen by its frame only.
- A captured local: an `Upvalue` in `FrameState.local_upvalues[slot]`. Each
  closure receives it as `Function.upvalues[index]` and attaches it to its
  own slot at frame entry, so the declaring frame, sibling closures and
  nested closures hold the same cell.
- A realm binding (global `var`, function, builtin): the cell for its name in
  `RealmState.bindings` (`function/env.rs`), shared by the whole realm.
- A script's top-level `let`/`const`/`class`: a slot of the script frame,
  captured like any other local.
- A module's top-level binding: a cell created at module instantiation,
  shared by the module body, its functions and its importers.

An `Upvalue` (`function/upvalue.rs`) is `Rc<RefCell<Value>>`: cloning shares
the cell, and identity is `Upvalue::ptr_eq`. The compiler assigns slots and
upvalue indices, so shadowed names are different slots and different cells.

## Invariants

1. **Slots are private until captured.** `locals` is `Vec<Option<Value>>`. A
   slot with no cell is read and written directly; the `authoritative_slots`
   mask records which (`bytecode/vm_bindings.rs`). Once a slot has a cell, the
   cell is the binding and the bit is cleared (`bytecode/vm_capture.rs`).
2. **Capture is by cell, by position.** `bytecode/upvalue_resolver.rs`
   classifies each captured parent slot as `ParentLocal` (box the parent's
   slot) or `ParentUpvalue` (pass on a cell the parent received).
   `captured_upvalues_for_function` builds a closure's `upvalues` in the
   callee's received-slot order; producer and consumer must agree on
   `Local::is_received_upvalue`, because one extra entry shifts every later
   index. Tests: unit tests in `upvalue_resolver.rs`; `closure_state.rs`
   `::captured_lexical_cell_is_shared_by_parent_and_sibling_closures`,
   `::shadowed_lexical_closures_keep_distinct_shared_cells`.
3. **Call-frame values are frame-local.** `this`, `arguments`, `new.target`
   and the other `is_call_frame_binding` names are seeded by call setup into
   the declaring function's own slots and are never received upvalues. An
   arrow function captures that local cell as an ordinary `ParentLocal`.
   Compiler temporaries (names starting `\0\0`) are never captured. Tests:
   `functions.rs::evaluates_arrow_functions_with_lexical_this`,
   `::evaluates_arrow_functions_with_lexical_arguments`.
4. **Each loop iteration gets a fresh cell.** `fresh_iteration_scope` gives
   every captured per-iteration lexical a new cell holding the current value.
   Tests: `closure_state.rs::loop_body_lexicals_get_per_iteration_environment`,
   `::for_let_head_allocates_a_cell_after_the_initializer_and_each_back_edge`.
5. **Realm bindings have one cell each, from creation.** `RealmState.bindings`
   is one name-to-cell map (`DynamicBindings`); there is no separate value map
   and no lazily filled cell table. A global-scope `var` or function slot
   stays empty and its `local_upvalues` entry is the realm cell; a sloppy
   global-fallback slot attaches the same cell, and the `realm_binding_slots`
   mask makes the read a direct cell load. Deleting the global, or redefining
   it as an accessor, stores the uninitialized marker in the cell and unbinds
   the name (`CallEnv::remove_realm`); a reader that finds the marker asks the
   global object (`load_local_slow`). Tests: `closure_state.rs`
   `::sibling_calls_share_a_captured_global_var_cell`,
   `::deleting_a_sloppy_global_invalidates_its_cached_cell`,
   `::global_accessor_definition_deopts_a_cached_sloppy_global_cell`;
   `global.rs::keeps_global_object_properties_and_bindings_in_sync`.
6. **Intrinsics are not global bindings.** `RealmState` keeps the Object,
   Array, String and Number prototypes and the well-known symbols in set-once
   fields; rebinding the global `Object` does not change them.
7. **Mapped `arguments` shares the parameter's cell.** Each parameter slot
   attaches the cell held by the arguments object's index accessor
   (`initial_local_upvalues`). Test: `parameters.rs`
   `::mapped_arguments_keeps_parameter_cell_across_nested_descriptor_helper`.
8. **Immutable self names are cells too.** A class's inner name and a named
   function expression's own name are normally received cells; when the
   function-expression name is not, the function carries one extra cell,
   `Function.immutable_env_value` (`function/value.rs`). Tests:
   `functions.rs::named_function_expression_name_binding_is_immutable`,
   `classes.rs::class_inner_name_binding_is_immutable_inside_members`.
9. **TDZ is a state of the location, not a second table.** An unentered
   lexical slot is `None`; a cell created before initialization holds the
   uninitialized-lexical marker. Reading either throws `ReferenceError`
   (`load_local`, `checked_local_value`). Tests:
   `expressions.rs::assignments_respect_lexical_tdz`,
   `parameters.rs::default_parameter_initializers_use_parameter_tdz`.
10. **Modules use cells for live bindings.** An import slot attaches the
    exporting module's cell (`CallEnv::module_import_cell`); a module's own
    top-level slot attaches its instantiation cell. Test: `bytecode/`
    `vm_frame_init.rs`, `direct_module_frame_keeps_import_cell_storage`.
11. **Suspension keeps the cells.** A suspended frame owns its `locals`,
    `local_upvalues` and `upvalues`: [generator-suspension.md](generator-suspension.md).

## Dynamic scope: direct `eval` and `with`

Only these resolve a name at run time, so only they use a name-keyed map, and
that map holds cells, not values.

- A frame whose bytecode contains a direct `eval` or a `with`, or that
  inherits such a scope, gets `CallEnv.deopt_bindings`, a `DynamicBindings`
  name-to-cell map (`with_frame_bytecode` in `bytecode/vm_frame_init.rs`). No
  other frame allocates one.
- In such a frame every non-temporary local gets a cell at entry, and
  `frame_deopt_bindings` overlays the live cells into the map under their
  source names, so `eval` code and compiled slot access update the same
  cell. A `var` created by `eval` becomes a new named cell in the map.
- `with` pushes its object on the frame's `with_stack` (`bytecode/vm_with.rs`).
  Only free names inside the block compile to with-aware operations that
  consult the object first. A function created inside the block keeps the
  stack in `Function.with_stack`; its own slots stay closer than the stack.
- A closure that provably resolves none of the scope's names may drop or
  bypass the scope (`closure_scope_use`); `DynamicBindings` revokes every
  bypass as soon as a name is added, removed or remapped.

Tests: `closure_state.rs::direct_eval_and_closures_share_the_same_local_cell`,
`::with_fallback_assignment_updates_the_outer_local_cell`,
`::closure_created_inside_with_resolves_the_retained_with_object`,
`::closures_made_beside_a_direct_eval_see_every_rebinding`.

## What `CallEnv` is and is not

`CallEnv` (`function/env.rs`) is per-call context, not binding storage. It
shares the realm scope through one `Rc` and carries a small `frame_bindings`
vector (call metadata, native and dynamic consumers), the optional
`deopt_bindings`, and private-name, module and host state. An ordinary call
builds no locals map. `BindingSnapshot` is an owned copy for the few
consumers that enumerate a frame; it never takes part in identity.

The compact tiers run only when the environment can answer no name at all
(`environment_is_slot_only` in `bytecode/compact_fn/activation.rs`). A
slot-seeded direct frame that only reads its received cells resolves them
from the function it retains instead of building a cell vector
(`bytecode/vm_direct_upvalues.rs`; unit tests in `bytecode/vm_frame_init.rs`).

## Known limits

- A cell holding a function that captures the same cell is an `Rc` cycle and
  is never freed; collecting it needs the GC or arena `AGENTS.md` allows.
- `DynamicBindings` is deliberately the slow path. A new use must need
  run-time name resolution and stay gated away from ordinary calls.
