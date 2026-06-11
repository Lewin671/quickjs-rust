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

## Notes

- The 19 `TypedArray/prototype/fill` failures in the baseline scan are
  correctness gaps (resizable buffers etc.), not performance; out of scope here.
- Profiling was done with temporary `std::time::Instant` accumulators and
  `eprintln` counters in `call.rs`/`vm.rs`/`lib.rs`, all removed before
  committing. `cargo flamegraph` is not installed and new deps are forbidden.
