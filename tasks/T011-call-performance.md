# T011: Cut per-call environment-cloning cost

## Goal

Make user-function calls and prototype-method property accesses cheap by
sharing the realm environment instead of cloning a full
`HashMap<String, Value>` at every call boundary. The leaf-call clone has
already been removed (see "Landed"); the remaining work is the
environment-model redesign that removes the per-call and per-property full
globals copies.

## Scope

- Allowed paths: `crates/qjs-runtime/**`
- Forbidden paths: `third_party/**`
- Owner boundary: whole `qjs-runtime` crate (the change touches the
  `env: &mut HashMap<String, Value>` API threaded through every builtin, so it
  must serialize on one branch; do not split across parallel agents).

## References

- `crates/qjs-runtime/src/function/call.rs` — `call_function`, `function_env`,
  `insert_runtime_intrinsics`, `insert_caller_scope_bindings`,
  `propagate_caller_bindings`.
- `crates/qjs-runtime/src/bytecode/vm.rs` — `Vm`, `call_callee`, `call_env`,
  `current_env`, `apply_env`, `apply_call_env`.
- `crates/qjs-runtime/src/bytecode/vm_call.rs` — `insert_scope_call_bindings`.
- `crates/qjs-runtime/src/property/{mod.rs,prototype.rs}` — prototype
  resolution reads intrinsics out of the env map (`object_prototype(env)` etc.).

## Measured hot spots (debug build, 2026-06-11, HEAD a0f31b1)

Targeted `std::time::Instant` accumulators (temporary, removed before commit)
around the call path, plus a `current_env()` call counter:

Bare 20k-call loop `function f(x){return x+1;} ... s=f(s)` — total call cost
~191 us/call, of which actual bytecode execution is ~15 us:

| phase | per-call |
| --- | ---: |
| `call_env` (caller-side env build) | ~45 us |
| `function_env` (frame env build) | ~103 us |
| activation `captured_env` clone | ~31 us (NOW SKIPPED for leaf bodies) |
| `eval_function_bytecode` (real work) | ~15 us |
| `apply_call_env` write-back | ~6 us |

Method-call loop `a.indexOf(2)` x 50k — ~150 us/call and **2 full
`current_env()` clones per method call** (one for the prototype-chain property
lookup, one inside the native call). `current_env()` clones all of
`self.globals` (~77 entries) plus locals, then `apply_env` writes the whole map
back.

Root cause: the engine passes the realm/global environment **by value** (a full
`HashMap<String, Value>` clone) across every call boundary and on every
non-fast-path property get/set. The intrinsics copy itself is cheap (skipping
all ~50 intrinsic inserts changed wall time by <1%); the cost is the repeated
HashMap allocation, String-key hashing/`to_owned()`, and `Value` (Rc) clones of
~55-130 entries, done 2-3 times per call.

Why the copy exists (constraints any redesign must preserve):

- A callee must see the **current** value of a reassigned builtin. Verified:
  `Array = function(){...}; f()` makes `f` see the reassigned `Array`. So the
  per-call snapshot conveys live global reassignments — intrinsics cannot simply
  be sourced from the closure's `captured_env` snapshot, which can be stale.
- Sloppy-mode dynamic global creation (`propagate_caller_bindings` /
  `sloppy_global_names`) writes new bindings back to the caller.
- Prototype resolution (`object_prototype(env)`, `array_prototype(env)`,
  `inherited_*_prototype_descriptor(env, ...)`) reads intrinsics implicitly by
  name out of the env map, so a callee that never *names* `Object` still needs
  `Object` present for `{}.toString()` etc. — filtering by `global_names()`
  alone is not safe.
- Generators/async snapshot `globals` and `captured_env`
  (`vm_generator.rs`); any shared-globals representation must remain compatible
  with suspend/resume.

## Landed (contained win, this session)

- `Bytecode::creates_closures()` + gating the activation `captured_env` clone in
  `call_function`: a leaf body that never runs `NewFunction`/`NewClass` never
  reads the activation captured env, so it now starts empty instead of cloning
  the whole frame env. 20k-call loop ~5.2s -> ~4.4s; closure-call loop ~11.7s ->
  ~10.4s (debug). Guarded by
  `functions::nested_closures_capture_live_outer_bindings`.

- Read-only prototype-chain get fast path (`Vm::try_direct_get` in
  `vm_props.rs`, wired into `get_prop` and `pop_method_callee`): a property get
  whose descriptor is a plain data property is now resolved straight off the
  base value's own/linked prototype chain, reading the realm intrinsics
  (`Array.prototype`, `String.prototype`, %Function.prototype%, ...) out of
  `&self.globals` rather than cloning the full `current_env()` map. Any accessor
  descriptor, a `Proxy` target, a native-function base with a possible
  native-error parent, or a symbol on a function/primitive still falls through
  to the clone-and-writeback path, so observable semantics are unchanged
  (verified by `cargo test -p qjs-runtime` and `compare-qjs`). This removes the
  two `current_env()` clones per method call. `fill/coerced-indexes.js`
  ~74.6s -> ~55.4s (debug); the plain/closure call loops are unchanged because
  they perform no property reads. The remaining cost there is the call-boundary
  env clone, addressed by the redesign below.

## Fresh call-path profile (debug, 2026-06-11, HEAD c02ea14)

`std::time::Instant` accumulators around `call_callee` (vm.rs) and the leaf
branch of `call_function` (call.rs), 20k plain `f(x){return x+1;}` calls:

| phase | total / 20k | per call |
| --- | ---: | ---: |
| `call_env` (caller-side build) | 913 ms | ~46 us |
| `function_env` (frame build) | 2086 ms | ~104 us |
| `eval_function_bytecode` (real work) | 29 ms | ~1.5 us |
| `apply_call_env` (write-back) | 117 ms | ~6 us |

So ~150 us/call of the ~157 us is pure environment building; the actual VM
loop is ~1.5 us. The redesign target is `call_env` + `function_env`. For a
trivial leaf call these do **two** independent `HashMap` allocations and copy
the ~50 `RUNTIME_INTRINSIC_NAMES` entries (String key + Rc value clone) into
each, plus `function_env`'s `insert_caller_scope_bindings` iterates every
caller key and runs an O(50) `RUNTIME_INTRINSIC_NAMES.contains` linear scan per
key. Step 1 (prototype-read fast path) does not touch this path, which is why
the plain/closure call loops were unchanged by it.

## Remaining redesign (the real fix) — NEXT STEP, not yet landed

The flat `env: &mut HashMap<String, Value>` contract is what forces a fully
materialized per-call map. Eliminating the copies requires changing that type,
which is a single serialized whole-crate change (775 signature/usage sites
across 134 files; verified with
`grep -rn 'env: &mut HashMap<String, Value>\|env: &HashMap<String, Value>'`).
It cannot be split into a smaller *compiling* sub-commit, so plan for it as one
landing.

Concrete migration recipe (in dependency order):

1. Introduce `pub(crate) struct CallEnv { realm: Rc<RefCell<HashMap<String,
   Value>>>, locals: HashMap<String, Value> }` (suggest `src/function/env.rs`).
   The `realm` cell holds intrinsics + true globals, shared by `Rc::clone` into
   every frame; `locals` holds only this/arguments/params/captures/caller
   scope bindings for the current frame.
2. Decide read/write layering: `get(name) -> Option<Value>` (OWNED, not
   `&Value` — a layered value behind a `RefCell` cannot hand out a reference);
   check `locals` first, then borrow `realm` briefly and clone out. `insert`
   must route to the right layer — keep the existing `bytecode.local_slot`
   discrimination: a name with a local slot goes to `locals`, otherwise to
   `realm`. This is the single biggest correctness risk.
3. **Borrow discipline**: never hold `realm.borrow()`/`borrow_mut()` across a
   call back into user code (getters, Proxy traps, valueOf/toString, setters,
   callbacks). Always clone the needed value out, drop the borrow, then call.
   Tests that exercise this: anything with getters/Proxy/`Symbol.toPrimitive`
   under `tests/` and `compare-qjs` `proxy-*`, `symbol-to-primitive.js`.
4. Migrate the 84 `env.get(...)` sites: most are already `env.get(x).cloned()`
   and become `env.get(x)`; the `if let Some(v) = env.get(x)` borrow sites need
   the owned value bound to a local first. The 93 `env.insert` and 2
   `env.get_mut`/`entry`/`remove`/`keys`/`contains_key` sites each need a
   `CallEnv` method. The ~1840 `, env` threading sites are unaffected once the
   parameter type changes.
5. The 18 `env.clone()` sites (run
   `grep -rn 'env\.clone()' crates/qjs-runtime/src`) mostly snapshot the env
   into `Rc<RefCell<HashMap>>` for closure/generator/class `captured_env`.
   Provide `CallEnv::snapshot_locals() -> HashMap` (or keep capturing the
   `realm` Rc + a `locals` snapshot) and decide the closure capture model:
   closures should capture the `realm` Rc (so reassigned builtins stay live)
   plus a snapshot of the outer `locals` they close over. Preserve observable
   semantics — `functions::nested_closures_capture_live_outer_bindings` and
   `async_generators.rs` regression tests are the contract.
6. VM frame: make `Vm.globals` the shared `Rc<RefCell<HashMap>>`; `current_env`
   /`apply_env`/`call_env`/`function_env`/`apply_call_env` collapse — global
   reads/writes hit the realm cell directly, so `propagate_caller_bindings`'
   write-back scan and the `refresh_from_caller`/`propagate_to_caller`
   stale-binding workarounds in `vm_generator.rs` likely become unnecessary
   (re-verify against `async_generators.rs`).
7. Prototype-resolution helpers in `property/prototype.rs` (`object_prototype`,
   `array_prototype`, ...) currently take `&HashMap`; either keep that and pass
   a brief realm borrow, or change them to take the realm cell. The Step 1
   `try_direct_get` already reads these out of `&self.globals` and should be
   re-pointed at the realm cell.

Original higher-level sketch below for reference:

1. Store globals as `Rc<RefCell<HashMap<String, Value>>>` (or a `Realm` struct)
   owned once per `Vm`/script and shared by reference into every call frame, so
   `call_env`/`function_env` stop copying intrinsics and unreferenced globals.
   The per-call map then holds only the frame's own locals/params/`this`/
   `arguments`/captures; global reads fall through to the shared realm, and
   global writes mutate it in place (giving live reassignment for free and
   removing `propagate_caller_bindings`' write-back scan).
2. Replace `current_env()`/`apply_env` (full clone + writeback per property
   access) with read access to the shared realm. Prototype-resolution helpers
   (`object_prototype`, `array_prototype`, ...) should borrow the realm rather
   than take `&HashMap`. This removes the 2-clones-per-method-call tax.
3. Migrate the `env: &mut HashMap<String, Value>` parameter that is threaded
   through every builtin to a realm handle (or a small two-layer view: shared
   realm + frame locals). This is the bulk of the work and why it must be one
   serialized change.

A smaller intermediate win, if the full redesign is deferred again: add a
read-only prototype-chain get fast path in `vm.rs` (`pop_method_callee`,
`get_prop`) that resolves non-accessor data properties through the prototype
chain using `&self.globals` directly and only falls back to the
`current_env()` clone-and-writeback path when an accessor (getter) is actually
involved. This targets the method-call double-clone without touching the
builtin `env` API. It must bail to the slow path for Proxy targets and any
accessor descriptor to preserve semantics.

## Acceptance Criteria

- 20k-call loop and `a.indexOf` method loop drop to single-digit us/call (target
  >5x off the current ~150-191 us/call).
- `test/built-ins/TypedArray/prototype/fill/coerced-indexes.js` finishes well
  under the 10s Test262 case timeout (currently ~74s after the leaf-call win);
  the ~9 `TypedArray/prototype/fill` timeouts clear.
- No observable semantic change: `cargo test -p qjs-runtime`,
  `./scripts/compare-qjs.sh`, and the full `./scripts/check.sh` stay green,
  including reassigned-builtin visibility and sloppy-mode global write-back.

## Verification

```sh
./scripts/check.sh
./scripts/compare-qjs.sh
./scripts/test262-baseline.sh --all --filter test/built-ins/TypedArray/prototype/fill --engine quickjs-rust 2>&1 | tail -5
```

## Status: WIP — `CallEnv` foundation landed, migration in progress

The shared-realm `CallEnv` type is implemented and compiling
(`crates/qjs-runtime/src/function/env.rs`, exported from `function.rs`). The
crate still builds and the test suite is unchanged because nothing else has
been re-pointed at it yet. The remaining mechanical migration (775 signature
sites) is the bulk of the work and is NOT yet done — the branch builds today
precisely because the flat-`HashMap` model is still in place everywhere except
the new, currently-unused type.

### Design decided (read before resuming)

`CallEnv { realm: Rc<RefCell<HashMap<String, Value>>>, locals: HashMap }`.

- `realm` is the shared cell: runtime intrinsics + true globals. `Rc::clone`d
  into every frame, so reassigned builtins and sloppy globals are live for
  free.
- `locals` is the per-frame layer: `this`/`arguments`/params/captures/
  caller-scope bindings. Only this is cloned per call.
- `get(name) -> Option<Value>` (OWNED): locals first, then a short realm
  borrow + clone. `contains_key`, `remove`, `get_local_mut` operate on locals.
- `insert(name, value)` writes to **locals** (the frame layer). The VM
  write-back (`apply_env`/`apply_call_env`) keeps routing locals entries to
  real locals-or-globals via `bytecode.local_slot`, exactly as today. This
  preserves the existing "build a frame, then write back" contract for runtime
  builtins with no per-site routing decisions.
- `insert_realm(name, value)` writes straight to the shared cell — used by the
  builtin **install path** and explicit global definition.

### Critical routing/borrow rules (the correctness contract)

1. The **install path stays on `&mut HashMap`**, NOT `CallEnv`.
   `initialize_builtins` and every `install_*` (object/install.rs, array/
   install.rs, ...) run once in `Vm::new` against the raw realm map *before any
   frame exists*; leave their signatures as `env: &mut HashMap<String, Value>`.
   They populate the map; `Vm::new` then wraps it: `let realm =
   Rc::new(RefCell::new(globals));`. CAUTION: install and runtime builtins live
   in the SAME files and share the `env: &mut HashMap` parameter name — a blind
   per-file sed will wrongly convert install signatures. Convert by function,
   not by file: install_* + the helpers they call keep `&mut HashMap`; runtime
   builtins (the ones reachable from `call_native_function`) move to `&CallEnv`/
   `&mut CallEnv`.
2. NEVER hold `realm.borrow()`/`borrow_mut()` across a call back into user code
   (getters/setters/Proxy traps/`valueOf`/`toString`/iterators). `CallEnv::get`
   already drops its borrow before returning an owned value; keep it that way at
   every new call site.
3. `env.clone()` now shares the realm Rc and copies only locals — this is the
   intended closure-capture model. The `Rc<RefCell<HashMap>>` `captured_env`
   fields (Function, GeneratorStart/Snapshot, vm_class, vm_private,
   async_*) must be reworked to capture `realm: Realm` + a `locals` snapshot
   instead of a deep `HashMap` snapshot. Preserve
   `functions::nested_closures_capture_live_outer_bindings` and the
   `async_generators.rs` stale-binding regression tests.

### Mechanical migration order (compiler-driven; nothing compiles mid-way)

1. **VM core (`bytecode/vm.rs`)**: replace `globals: HashMap<String, Value>`
   with `realm: Realm`. `Vm::new` builds the map, wraps it in the realm cell.
   - `current_env()` is deleted; reads go straight to the realm
     (`self.realm.borrow().get(...)`) or a `CallEnv` over `self.realm`.
   - `call_env`/`function_env`/`apply_env`/`apply_call_env` collapse: build a
     `CallEnv::new(self.realm_rc())` whose `locals` are only the frame's
     referenced caller bindings (keep `insert_referenced_call_bindings` /
     `insert_scope_call_bindings`, but they fill `locals`, not a full clone).
     Global reads/writes hit the realm directly, so the
     `propagate_caller_bindings` write-back scan over realm names goes away
     (locals write-back via `local_slot` stays).
   - 66 `self.globals` sites across 11 `bytecode/vm_*.rs` files become
     `self.realm.borrow()`/`borrow_mut()` (short borrows!) or `self.realm_rc()`.
     `vm_call.rs` / `vm_bindings.rs` take `globals: &HashMap` params — change to
     `&Realm` or pass a borrow.
   - `array_prototype_cache` and `try_direct_get` (vm_props.rs) re-point from
     `&self.globals` to `&self.realm.borrow()` (short borrow; clone the
     prototype ObjectRef out before any re-entrant call).
2. **`function/call.rs`**: `call_function`/`construct_function`/
   `function_env`/`initialize_instance_fields`/`native_mapped_argument_*` take
   `env: &mut CallEnv`. `function_env` builds the frame `locals` (no intrinsics
   copy — those live in the shared realm). `insert_runtime_intrinsics` is
   deleted (realm already has them). `insert_caller_scope_bindings`'
   O(50)-per-key intrinsic scan goes away. `propagate_caller_bindings`'
   realm-name write-back is unnecessary (shared cell); keep only the
   locals/caller-binding write-back.
3. **`eval_function_bytecode`** (vm.rs) takes the realm + locals instead of a
   flat `HashMap` + `captured_env: Rc<RefCell<HashMap>>`.
4. **`property/{mod.rs,prototype.rs}`** and every helper there: change
   `env: &HashMap` → `env: &CallEnv` (or `&Realm` for the pure prototype
   lookups — they only read `Object`/`Array`/`String`/`Function`). `env.get("Object")`
   etc. now returns `Option<Value>` (owned); the `let Some(Value::Function(f)) =
   env.get(...)` patterns still match on the owned value. The two
   `let mut proxy_env = env.clone()` sites become `CallEnv` clones.
5. **Builtins (the 1419 `, env` threading sites)**: once the parameter type
   flips to `&CallEnv`/`&mut CallEnv`, threading is mechanical. The 82
   `env.get(x).cloned()` become `env.get(x)`; the `if let Some(v) = env.get(x)`
   borrow sites bind the owned value first. The 93 `env.insert` sites: runtime
   inserts → `CallEnv::insert` (frame) or, where they define a true global,
   `insert_realm`. Hand-review each. The 9 get_mut/entry/remove/keys/
   contains_key sites (enumerated in the recipe above) each map to a `CallEnv`
   method.
6. **Generators/async (`vm_generator.rs`, async_*.rs)**: snapshots store the
   `Realm` Rc + a locals snapshot instead of a `globals` map copy. The
   `refresh_from_caller`/`propagate_to_caller` workarounds likely become
   unnecessary because the realm cell is shared — remove them ONLY if the
   `async_generators.rs` regression tests still pass without them.
7. **`global.rs` indirect eval** (`eval_bytecode_with_env`, the `env.clone()` at
   global.rs:176 and the `entry().or_insert` at :203) and `bytecode/mod.rs`
   `eval_bytecode_with_env` (:139): these build a fresh realm/captured env for
   eval'd code — pass the realm cell through.

### Verification gates (must all pass before commit)

- `cargo test -p qjs-runtime` (577 tests), `cargo test -p qjs-parser`
- `cargo fmt`, `./scripts/check.sh` exit 0, `./scripts/compare-qjs.sh` exit 0
- Timings (debug): 20k plain-call loop + 20k closure-call loop in single-digit
  µs/call; `fill/coerced-indexes.js` well under 10s; `TypedArray/prototype/fill`
  timeouts → ~0.
- Watch for double-borrow panics in tests touching getters/Proxy/
  `Symbol.toPrimitive` — those mean rule 2 was violated.

## RELAY HANDOFF — leg 1 (agent-i, 2026-06-11)

**Leg status: WIP, no migration edits committed. The tree is CLEAN at the
foundation state (HEAD 6368cf7); `cargo check -p qjs-runtime` error count = 0.**
This leg was spent fully scoping the migration surface and the closure/generator
capture rework, not flipping signatures. No `CallEnv` wiring was committed
because the migration has no compiling intermediate state and a partial flip
would have left an incoherent tree with inaccurate notes — per the task's
"accuracy of the handoff over extra distance" rule, the scoped map below is the
deliverable. `crates/qjs-runtime/src/function/env.rs` `CallEnv`/`Realm` remain
as the (still unused) foundation; everything else is untouched.

### What is migrated vs remaining

- **Migrated: nothing yet** beyond the pre-existing `CallEnv`/`Realm` foundation
  type (already present and exported from `function.rs:15`).
- **Remaining: the entire mechanical migration** (all 775 signature sites, 134
  files). The recipe order in "Mechanical migration order" above is correct and
  validated against the current source; the notes below pin down the exact
  surface so the next relay does not have to re-derive it.

### Verified migration surface (current HEAD; use these as the worklist)

Site counts (re-run to confirm before starting):
- 775 `env: &mut HashMap<String, Value>` / `env: &HashMap<String, Value>` sites,
  134 files.
- 82 `env.get(...)`, 93 `env.insert(...)`, 18 `env.clone()`, 8
  get_mut/entry/remove/keys/contains_key, 1209 `, env)` threading sites.
- 63 `self.globals` sites across 11 `bytecode/vm_*.rs` files; distribution:
  vm.rs 16, vm_props.rs 15, vm_class.rs 14, vm_bindings.rs 5, vm_generator.rs 3,
  vm_errors.rs 2, vm_literals.rs 2, vm_ops.rs 2, vm_private.rs 2, vm_iter.rs 1,
  vm_result.rs 1.
- `self.captured_env`: vm.rs 2, vm_generator.rs 2, vm_class.rs 1.
- `current_env()` callers: vm.rs 8, vm_iter.rs 15, vm_ops.rs 4, vm_class.rs 4,
  vm_private.rs 2, vm_literals.rs 1, vm_props.rs 1, vm_jobs.rs 1.
- `.apply_env(` callers: vm_iter.rs 15, vm.rs 8, vm_class.rs 4, vm_ops.rs 4,
  vm_private.rs 2, vm_literals.rs 1.

### Capture-model rework (step 6 — the delicate part), pinned down

`Function.captured_env: Rc<RefCell<HashMap<String, Value>>>` appears at:
- `function/value.rs:59` (the `Function` struct field) and `:134` (the
  `CompiledUserFunction` builder field). Plus `CompiledUserFunction` ALSO carries
  `pub env: HashMap<String, Value>` at `value.rs:55-57` — that is the function's
  *captured creation env*, a second snapshot to rework alongside `captured_env`.
- Construction sites in `value.rs`: `:224`/`:284`/`:291`/`:378`/`:420`/`:425`.
- `bytecode/vm_generator.rs`: `GeneratorStart.captured_env` (:38) and
  `GeneratorStart.env` (:37); `GeneratorSnapshot.globals` (:47) and
  `.captured_env` (:48); `into_snapshot` (:140) threads both.
- `bytecode/vm.rs`: `Vm.captured_env` field (:57), set in
  `new_with_globals_and_captures` (:91), read in `Op::NewFunction`
  (`self.captured_env.clone()` :286) and `refresh_captured_env` (:494).
- async machinery: `async_function.rs` / `async_generator.rs` consume the same
  `function_env.env` + caller `env` (see `call.rs:76,123`).

Per the decided design (lines 269-275): each of these `Rc<RefCell<HashMap>>`
snapshots must become `realm: Realm` (shared Rc) + a `locals: HashMap` snapshot.
The regression contract is `functions::nested_closures_capture_live_outer_bindings`
and the `async_generators.rs` stale-binding tests. NOTE: if the realm is truly
shared, the `vm_generator.rs` `refresh_from_caller` (:112) /
`propagate_to_caller` (:91) workarounds for realm-name staleness should be
deletable — but ONLY remove them if those async/generator tests still pass; they
also currently propagate *locals* the body shares with the caller, so re-check
whether the locals-propagation half is still needed after the split.

### CRITICAL install-vs-runtime classification (the "convert by function, not
file" trap), enumerated

The native-call boundary is `native.rs:27` `call_native_function(.., env: &mut
HashMap)`. Everything reachable from it is a **runtime builtin → moves to
`&CallEnv`/`&mut CallEnv`**. Everything in an `install_*` path runs once in
`Vm::new` before any frame → **stays `&mut HashMap` / `&HashMap`**.

`install_*` functions that STAY on the raw map (do NOT convert):
- symbol.rs: `install_symbol` (:31), `install_well_known_symbols` (:127),
  `install_function_has_instance` (:313, takes `&HashMap`)
- array_buffer.rs `install_array_buffer` (:17)
- bigint.rs `install_bigint` (:14), `install_bigint_well_known_symbols` (:63)
- async_generator.rs `install_async_generator` (:71)
- boolean.rs `install_boolean` (:10); proxy.rs `install_proxy` (:68)
- weak_map.rs `install_weak_map` (:11); weak_set.rs `install_weak_set` (:11)
- error.rs `install_error` (:19), `install_error_cause` (:185),
  `install_native_error` (:330)
- global.rs `install_globals` (:12); promise.rs `install_promise` (:54)
- set.rs `install_set` (:18); math.rs `install_math` (:12)
- async_function.rs `install_async_function` (:36)
- typed_array.rs `install_typed_arrays` (:50),
  `install_typed_array_prototype_accessors` (:88, `&HashMap`),
  `install_typed_array_prototype_methods` (:132, `&HashMap`),
  `install_typed_array_constructor` (:209)
- generator.rs `install_generator` (:28); regexp.rs `install_regexp` (:34)
- map.rs `install_map` (:17); data_view.rs `install_data_view` (:48)
- regexp/symbol_split.rs `install_regexp_prototype_split` (:11, `&HashMap`)
- typed_array/construct.rs `install_view` (:256)
- regexp/symbol_replace.rs `install_regexp_prototype_replace` (:8)
- regexp/match_all.rs `install_regexp_prototype_match_all` (:14)
- regexp/symbol_match.rs `install_regexp_prototype_match` (:8, `&HashMap`)
- array/install.rs `install_array`; date/install.rs `install_date`
- reflect/install.rs `install_reflect`; function/install.rs `install_function`
- function/value.rs `install_class_prototype` (:337)
- number/install.rs `install_number`; iterator/mod.rs `install_iterator` (:106)
- json/install.rs `install_json`; object/install.rs `install_object` (:5)
- bytecode/vm_class.rs `install_method` (:197), `install_field_value` (:568)
- bytecode/vm_private.rs `install_private_elements` (:23)
- string/install.rs `install_string` (:76),
  `install_string_well_known_symbols` (:117, `&HashMap`)

The files where install + runtime builtins SHARE the `env: &mut HashMap` param
name (so a per-file sed corrupts the install signature) — convert function by
function here: symbol.rs (27 env sites), promise.rs (25), async_generator.rs
(25), regexp.rs (17), set.rs (13), global.rs (12), map.rs (11),
regexp/symbol_split.rs (10), regexp/symbol_replace.rs (10), bigint.rs (10),
typed_array/construct.rs (9), generator.rs (8), error.rs (8), array_buffer.rs
(8), regexp/match_all.rs (7), iterator/mod.rs (7), data_view.rs (7),
async_function.rs (7), weak_map.rs (6), typed_array.rs (6), weak_set.rs (5),
regexp/symbol_match.rs (5), proxy.rs (5).

### Property + prototype layer (step 4), pinned down

env-typed function sites: property/mod.rs 9, property/prototype.rs 20,
property/key.rs 1, plus vm_props.rs property helpers (get_property/set_property/
property_set_uses_setter/etc. — ~20 `&mut HashMap`/`&HashMap` sites). The pure
prototype lookups in `property/prototype.rs` (`object_prototype`,
`array_prototype`, `string_prototype`, `function_intrinsic_prototype`,
`constructor_named_prototype`, and the `inherited_*` family) ONLY read
`Object`/`Array`/`String`/`Function` by name via `env.get(name)` — they are the
ideal `&CallEnv` (or `&Realm`) conversions, and `CallEnv::get` returning owned
`Option<Value>` keeps the existing `let Some(Value::Function(f)) = env.get(..)`
matches working unchanged. `vm_props.rs::try_direct_get` and
`array_prototype_has_index_property` currently read `&self.globals`; re-point to
a short `self.realm.borrow()` and clone the prototype `ObjectRef` out BEFORE any
re-entrant call (rule 2).

### Public/adjacent surfaces that carry the realm (do not miss these)

- `bytecode/mod.rs`: `EvalOutcome.env: HashMap` (:105) + `EvalOutcome::run_jobs`
  (:116) → `vm_jobs::run_pending_jobs(&mut env)`; `eval_function_bytecode`
  (:62), `eval_bytecode_with_env` (:135, builds `captured_env` from
  `env.clone()` at :139).
- `bytecode/vm_jobs.rs`: `eval_bytecode_keep_jobs` returns `(Value, HashMap)`
  (:21, builds via `vm.current_env()`), `run_pending_jobs(env: &mut HashMap)`
  (:31). The returned map IS the realm — return the `Realm` Rc instead.
- `vm.rs`: `VmCallEnv` struct (:23) + `call_env`/`apply_call_env`/
  `apply_selected_env`/`apply_env`/`current_env`/`insert_runtime_intrinsics`/
  `insert_referenced_binding`/`function_capture_env`/`refresh_captured_env`
  collapse per recipe step 1.
- `call.rs`: `function_env` + the `FunctionCallEnv` struct, `insert_runtime_*`,
  `insert_caller_scope_bindings` (the O(50)-per-key intrinsic scan at :714-731
  is what step 2 removes), `propagate_caller_bindings` (:733, realm write-back
  half becomes unnecessary; keep locals/caller-binding half),
  `construct_function`'s `env.insert(NEW_TARGET_BINDING..)` (:251) +
  `env.remove` (:272) → these are frame locals, route to `CallEnv::insert`/
  `remove`. `native_mapped_argument_get/set` (:576/:592) read/write a parameter
  binding by name → `CallEnv::get`/`get_local_mut`.

### env.insert routing decisions (the per-site correctness risk)

Per the decided design (lines 244-250): `CallEnv::insert` writes to the **frame
locals** layer and the existing VM write-back routes them to real
locals-or-globals via `bytecode.local_slot` — so MOST runtime `env.insert` sites
become `CallEnv::insert` with no per-site routing decision. Only sites that
define a *true global* (e.g. sloppy-mode global creation, explicit global
definition) use `insert_realm`. The `vm_call.rs::insert_scope_call_bindings`
`env.entry().or_insert_with` (:82) fills frame locals → `CallEnv` locals.
`store_global_strict`/`store_global_sloppy` (vm_props.rs:149/170) and
`define_global_var` write the realm directly → `self.realm.borrow_mut()`.

### Recommended execution order for the next relay

Follow the recipe (VM core → call.rs → eval entry → property/prototype →
builtins by batch → generators/async/class capture LAST). Do NOT try to make
intermediate states compile; gauge progress with
`cargo check -p qjs-runtime 2>&1 | grep -c '^error'` (falling = progress).
Budget the capture-model rework (step 6) generously — it is the only part with
subtle semantic regressions, and the `nested_closures_capture_live_outer_bindings`
+ `async_generators.rs` tests are the contract.

## Notes

- The 19 `TypedArray/prototype/fill` failures in the baseline scan are
  correctness gaps (resizable buffers etc.), not performance; out of scope here.
- Profiling was done with temporary `std::time::Instant` accumulators and
  `eprintln` counters in `call.rs`/`vm.rs`/`lib.rs`, all removed before
  committing. `cargo flamegraph` is not installed and new deps are forbidden.
