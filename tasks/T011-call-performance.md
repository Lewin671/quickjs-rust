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

## Notes

- The 19 `TypedArray/prototype/fill` failures in the baseline scan are
  correctness gaps (resizable buffers etc.), not performance; out of scope here.
- Profiling was done with temporary `std::time::Instant` accumulators and
  `eprintln` counters in `call.rs`/`vm.rs`/`lib.rs`, all removed before
  committing. `cargo flamegraph` is not installed and new deps are forbidden.
