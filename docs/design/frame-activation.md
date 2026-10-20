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
| Env assembly, export, write-back (`function_env`, `FrameBindings`, `apply_env`, `current_env`, realm refresh, env teardown) | 2,171 | Yes: indexed entry, lazy consumers | The entry stage kept legacy adapters beside the typed fields, an overlay. M2.1 now removes them for callee-ordinary entries. |
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

## Cohorts: two predicates, and the replacement rule

Today two different decisions exist (source paths relative to `crates/qjs-runtime/src/`):
- `vm_call_env.rs::callee_env` / `callee_env_origin` look at the **caller**. They pick the environment lent to natives,
  getters, Proxy traps and coercion hooks.
- A bytecode callee always gets `FreshFunctionFrame` from `call_env`, and `function_env` inserts its names regardless.

M2.1 removes callee-side work, so it needs both predicates, decided separately:

- **Callee-ordinary.** The entry route builds no names. All of the following must hold:
  - the callee's code contains no direct eval and no `with` (today `vm_frame_init.rs` installs deopt bindings for those);
  - the function object carries no `deopt_bindings`, or `scope_bypassed` is set (`function/call.rs`, in `function_env`);
  - no parameter/body scope split (`vm_bindings.rs::split_body_deopt_scope`, reached from direct eval in parameter
    expressions);
  - the code creates no eval-capable closure. **Eval-capable** means that the closure's code or any function nested in
    it, at any depth, contains direct eval or `with`. BindingPlan computes this transitively at compile time.
- **Caller-ordinary.** The environment lent to natives and hooks carries no caller names. Today this requires no direct
  eval, no `with`, no deopt bindings and `!creates_closures()`, which means any closure. M2.1 may narrow "any closure"
  to "eval-capable closure" under the same transitive definition, nested `with` included.

Fixtures for each predicate cover:
- callee side: callee eval and `with`, a reused deopt-carrying function, parameter-expression eval, and a closure
  three levels deep whose innermost body uses `eval` or `with`;
- caller side: the same deep closure passed to `Array.prototype.map` and to a getter, and then run.

Every stage replaces the old route for its cohort; it never computes both. In M2.1 a callee-ordinary entry builds no
`CallEnv`, no `FrameBindings` and no write-back. Every other entry keeps the whole legacy route until its stage. A
typed field mirrored by a legacy name entry is an overlay and is forbidden; that was the M1.2 failure.

## Native environment (M2.1 until natives take a capability in M2.5)

`empty_frame()` is not realm-only. `new_function_frame` inherits the caller's:
- `module_host`, including its referrer;
- `module_imports`;
- `private_environment`;
- `direct_eval_with_stack` (cloning a `Vec` on every call);
- `catch_bindings`;
- `direct_eval_var_conflicts`.

So one shared env per `ExecutionContext` would be wrong. For example, code made by indirect eval or `Function` inside a
module that then runs `import()` must keep the module's referrer. The chosen fix:

- **Typed caller-varying fields.** The current ScriptOrModule (module host and referrer), module imports, private
  environment, with stack, catch bindings, var conflicts, deopt and new.target become typed fields of the caller's
  `ActivationContext`. The services env refers to them; it does not copy them. A native call sets one O(1) "current
  activation" field and clones no `Vec`. The invariant part (scope, realm, agent) is built once per context.
- **Stack-discipline borrowing.** The services env is lent down the Rust call chain as a reborrow and is never
  mutably borrowed twice. An activation entered from a native (`call_sync`) saves the caller-varying field it replaces
  and restores it on return or unwind, through a guard. No activation observes another's fields.
- **Writers stay legacy.** Natives that write frame state are listed statically by `NativeFunction`. The list is
  frozen in the plan from an audit of every `CallEnv` mutation reachable from native code. Listed natives keep a
  legacy frame.
- **Debug assertion.** After every native call it checks that frame bindings, deopt, with stack, private environment,
  module host and referrer, module imports, catch bindings, var conflicts and new.target are unchanged.
- **Tests.**
  - The Test262 dynamic-import and private-name directories.
  - A fixture: module → indirect eval and `Function` → `import()` resolves against the module's referrer.
  - `Function` and indirect eval inside a class do not see `#x`.
  - A native callback run under `with` sees the right with stack.
  - `new.target` inside callbacks invoked from natives.

## Nested Vms during M2.1

M2.1 runs while nested Vms still exist. So that `call_env_constructions` is 0 rather than moved, `Vm` stops owning a
`CallEnv`. It holds the callee's `ActivationContext` plus a borrowed services env (`Vm<'a>` already carries a
lifetime). Its environment field is one of two forms, chosen at entry by cohort and never both:
- a typed form, for callee-ordinary entries;
- a legacy form, for everything else.

This replaces the env the Vm used to own, and M2.3's driver reuses the same field. It changes the `Vm` construction
API widely: every `new_with_globals*` and `with_frame_bytecode` caller in function calls, eval, modules and
generators. Cloning an env into each nested Vm instead would be another adapter layer and is not allowed.

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

## Driver contract (M2.3 generic frames, M2.4 compact windows)

The driver is staged by call population, not by mechanism:
- **M2.3: generic frame calling generic frame.** The frame store and the same-Vm driver land together for this
  population.
- **M2.4: compact windows join the same store.** Their thread-local register pool and frame stack go away. A compact
  body calling a non-compact body then pushes a FrameId on the same driver, with no `call_from_activation` CallEnv and
  no nested Vm.

Rules for both stages:
- **Generator and async frames.** They do not lease store slots before M2.5, because they break LIFO order. They keep
  their own frames until resume leases exist.
- **Indices, not pointers.** Window bases are store indices. Every view (slot, buffer, program) is fetched again after
  any call that may re-enter: a native, an accessor, a Proxy trap, coercion or a callback.
- **No transition value in memory.**
  - Call and return between frames of one executor stay inside the dispatch loop. Code, window index, pc and frame
    index are loop-carried values, not reloaded from the stack on every op.
  - `Enter`, `Return`, `Throw` and `NativeRequest` leave the loop only at cross-executor or native boundaries.
  - A release-binary audit uses the method of the [inputs](../../tasks/archive/T056-m2-design-inputs.md). It must find
    no aggregate store-then-reload of the transition on the admitted path, and no re-resolution of program, bytecode
    or upvalue owner when the callee's code is known.
- **Clear what was written.** Return releases only the registers below a per-frame high-water mark. Reuse still
  clears values and cells before the generation advances.
- **Exits (M2.4).** An exit continues on its own FrameSlot at the recorded IP only if the compact register layout maps
  directly onto the generic frame's slots: locals at the same indices, and operand registers at the generic stack
  positions for that IP's depth. Where it does not, the exit pays one recorded conversion, which is counted. A backedge
  whose probe declined for this code stops emitting the exit. New wide opcodes for exit kinds (object literals,
  `SetPropIndex`) are tier work for L4.

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

Leaf placement and per-stage work-count obligations are planning decisions; [T056](../../tasks/T056-interpreter-core-v2.md#m2-leaf-placement-and-work-count-obligations) owns them.
