# Frame activation: cost inventory, driver contract and stage scope

Refines the frames section of
[interpreter-core-v2](interpreter-core-v2.md#frames-storage-activation-context-and-lazy-names) against measured costs.
[T056](../../tasks/T056-interpreter-core-v2.md#ordered-migration) owns stage order, gates and plans. The numbers come from
the [M2 design inputs](../../tasks/archive/T056-m2-design-inputs.md); they are diagnostic samples, not profile receipts or
forecasts. Proposed, not implemented.

## What the frames section already covered, and what it missed

Sampled cost classes on the development lane: 7 libraries, 28.7 s of quickjs-rust samples (compact rows: 30.9 s of wall).

| Cost class | ms | Covered by the frames section | Gap closed here |
| --- | ---: | --- | --- |
| Env assembly, export, write-back (`function_env`, `FrameBindings`, `apply_env`, `current_env`, realm refresh, env teardown) | 2,171 | Yes: indexed entry, lazy consumers | The entry stage kept legacy adapters beside the typed fields, an overlay. It now removes them for the ordinary cohort. |
| Nested Vm and executor plumbing | 1,854 | Yes: same-Vm driver | The frame store and the driver now land together; a store with nested-Vm routing unchanged has no reach. |
| Direct-leaf `CallEnv` per call | 224 | No: the leaf cohort was treated as already clean | Leaf calls take the typed context like every call. |
| Static facts resolved by name per call (`Bytecode::local_slot` hashing; immutable-self, `arguments` and global-shadow checks) | 210 | Partly (BindingPlan) | BindingPlan precomputes every fact a call or a global load asks by name. |
| Argument vectors (`pop_arguments`, `Vec<Value>` copies) | ~400 | No: `CallSeed` allowed owned vectors | Arguments move once, from the caller's operand buffer to the callee's slots. Natives borrow the slice in place. |
| Slot-kind checks on every local access (`load_local_slow`, `store_local_slow`) | 439 | Partly (realm-row slot sources) | The access form is fixed at link time from BindingPlan, with no per-access classification. |
| Native boundary (`call_native_function`, an `empty_frame()` CallEnv per call) | 504 | Deferred to the last stage | The per-call CallEnv goes in the entry stage; the capability API stays last. |
| Compact driver: in-memory action, re-resolution, whole-window clear | 1,582 | No: tiers were left untouched | Compact windows become frame-store buffers under one driver (below). |
| Compact upvalue read: u128 popcount per read | 108 | No | The received-cell index is resolved at link time. The cell's `RefCell` borrow (155 ms) belongs to M3. |
| Compact calls into non-compact code (`call_from_activation`) | 15.4 s inclusive | Partly (natives) | These calls stay on the same driver. The time includes the callees' real work. |

These are upper bounds on reach. Each class contains semantic work that remains.

## Ordinary cohort and replacement rule

A call is **ordinary** when it needs no caller names. The caller is not a direct-eval, with or deopt frame, and the
callee creates no eval-capable closure. This is the condition under which `callee_env` already selects the realm-only
view. Callee kind does not matter: bytecode, direct-leaf, compact and native callees all qualify.

Every stage replaces the old route for its cohort. It never computes both representations:
- in a stage that removes per-call `CallEnv` assembly, an ordinary call builds no `CallEnv`, no `FrameBindings` and
  no write-back;
- calls outside the cohort keep the whole legacy route unchanged until their stage.

A typed field with a legacy name entry mirrored beside it is an overlay and is forbidden. This was the M1.2 failure.

Natives keep their `&mut CallEnv` signature until the last stage, but they no longer receive a per-call frame. Each
`ExecutionContext` owns one realm-only services env, built once and lent to each native call. A debug assertion
checks after every call that it still has no frame bindings. A native that would write frame state takes the legacy
route, so the shared env never accumulates state.

## Entry: BindingPlan, CallSeed and ActivationContext

- **BindingPlan** (immutable, per owned code) carries:
  - parameter slots and received-cell order, already resolved to cell indices;
  - each slot's source kind: local, cell, realm reference or module import;
  - the immutable-self slot, the `arguments` use and the global names shadowed by locals;
  - per-scope TDZ ranges.

  No call or global load hashes a name to answer any of these. The access opcodes are chosen from the slot kind at
  link time for every slot. This is not a recognizer for particular locals, which is what T027 was.
- **CallSeed** holds callee, code, this, new.target and cells, plus the caller's argument range. Entry moves each
  argument into its parameter slot (or into the rest/arguments object) exactly once. No intermediate vector is built.
  The range is consumed before any switch.
- **ActivationContext** carries realm, home and super, the private environment, the module host and the agent as
  typed fields. Direct-leaf, compact-boundary and generic entries all build this one context.

## Driver contract (frame store, compact windows and exits)

- **One frame store.** Generic frames and compact windows lease slots from the same `FrameStore`. The compact
  executor's thread-local register pool and its frame stack go away. A compact body calling a non-compact body pushes
  a FrameId on the same driver: no `call_from_activation` CallEnv, no nested Vm.
- **No yield token in memory.**
  - The hot call and return between frames of one executor stay inside the dispatch loop. Code pointer, window base,
    pc and frame index are loop-carried values, not fields reloaded from the stack on every op.
  - `Enter`, `Return`, `Throw` and `NativeRequest` leave the loop only for cross-executor or native boundaries.
  - The emitted code of those transitions is audited in the release binary with the method of the
    [inputs](../../tasks/archive/T056-m2-design-inputs.md): no aggregate store-then-reload of the transition value on the
    admitted call/return path, and no re-resolution of program, bytecode or upvalue owner when the callee's code is
    already known.
- **Clear what was written.** Return releases only the registers the activation wrote, bounded by a per-frame
  high-water mark, not every register. Reuse still clears values and cells before the generation advances.
- **Exits.** An exit keeps its FrameSlot. The generic dispatcher continues on the same slot at the recorded IP, with no
  window copy or rebuilt activation. A backedge whose probe declined for this code no longer emits the exit: the
  program records the decline and its op stream jumps directly. New wide opcodes for exit kinds (object literals,
  `SetPropIndex`) are tier work, deferred to L4.

## Interface with M1 (atoms and layouts)

- **Code linkage.** A FrameSlot header holds an owned `LinkedCode` handle (M1.1, on main). The atom domain comes from
  the context's realm, and cross-realm entry obtains the callee's cached linkage for that domain.
- **Caches live in code, not frames.** Property and global caches live in linked code, keyed by site. They are never
  in a FrameSlot: slots are reused across unrelated activations, and a cache must outlive them. Frames store no layout
  token, and no layout or cache borrow lives across a switch.
- **M2 uses no M1.2 API.** M1 stage A changes the stores; stage B routes Get/Set, including the compact tier's inline
  arms. M2 changes neither.
- **The dependency is ordering and evidence.** Structural runtime work is serialized. Both migrations edit the same
  dispatch arms and `vm_props`, and M1 changes the property share and the codegen of the loops M2 measures. So M2's
  plan is frozen on post-M1 receipts once M1 is finished: either stage B accepted, or M1 closed with its last verified
  stage on main. M2 does not need both M1 stages to land.

## Diagnosed leaf candidates: where each goes

History says leaf reach of 1–2% does not survive layout noise and complete controls: T041 and T049 failed their
screens; T053's 0.917 on one case did not excuse six failing controls; T027 missed at 0.984. Each candidate therefore
goes to the structural stage whose work-count tests prove it. None becomes a standalone unit.

| Candidate (inputs record) | Reach | Home |
| --- | ---: | --- |
| 3a: non-object named reads in compact (arrays, strings, functions) | ~0.5 s | M1 stage B: its routed Get covers the ordinary named part of these receivers, and the compact inline arm uses the same lookup |
| 4: upvalue cell index | 0.11 s | M2 entry stage (BindingPlan) |
| 4: borrow-free cache state | part of 0.42 s | M1 stage B (its cache replaces `NamedPropertyCache` state) |
| 4: borrow-free cell reads | 0.16 s | M3 (cells hold the value word) |
| 5: exit fast path | ≤1.18 s inclusive | M2 driver stage (declined backedges, slot continuation); new exit opcodes go to L4 |
| 6: `is_html_dda` on truthiness | 0.08 s | no unit; any later truthiness change, such as M3.3's tag tests, may absorb it |

## Work-count obligations (perf-counters, before any timed screen)

Counters are plain `cfg(feature = "perf-counters")` code. A fixture set counts exact events per call and covers:
fixed and variable arity, method calls, closure creation and invocation, a native with a JS callback (sort, map), a
getter, a compact-to-non-compact call, and a constructor.

| Stage | Obligation |
| --- | --- |
| Entry (ordinary cohort) | `call_env_constructions`, `frame_binding_inserts`, `env_write_backs`, `env_exports`, `realm_refresh_scans`, `argument_vectors` and runtime `name_slot_lookups` are 0 per ordinary call. The non-ordinary fixtures still count their legacy work, which shows the cohort split. |
| Dynamic consumers | Callback eval, with, coercion and result fixtures reach their names through leases. Ordinary calls stay at 0. Legacy `CallEnv` name storage is deleted. |
| Driver | `nested_vm_constructions` is 0 for JS-to-JS calls, including compact-to-non-compact calls. `same_vm_frame_entries` equals those calls. Warm frame-slot allocations plateau. `window_slots_cleared` equals written registers. `compact_driver_breaks` is 0 on admitted call/return. Declined backedges execute no exit. |
| Final routing | 0 nested Vms in an active context. No per-native-call frame. Every adapter is gone. |
