# T020: Unify realm binding storage behind the shared cell

## Status: Landed (commit `bfcd53da`, 2026-07-21)

The originally-scoped "slice 1" (make reads cell-aware first, keep the raw
map) turned out to be the wrong design — see "Design correction" below. The
actual fix was a full unification: `RealmState.bindings` is now
`DynamicBindings` directly (every realm binding is cell-backed from
creation), not a `HashMap<String, Value>` with a separate lazily-populated
cell registry layered on top. This was implemented, verified, and pushed in
one session once the design was corrected — the blast radius (34 call sites)
turned out to be mechanical once `DynamicBindings` (which already existed
for other binding classes) was reused as the unified storage type, rather
than designing a new API.

## Goal (as originally scoped)

Remove the redundant per-store name-table hashing that remains for
global-scope hoisted `var`/function bindings even after T016 landed shared
upvalue cells for the closure-capture binding classes. This is a *different*
binding class from T016's scope (closures capturing outer variables): global
bindings are additionally observable through the `globalThis` object
(enumeration, `delete`, `Object.defineProperty`), which is why the old
triple-sync model survived here on purpose, not by omission.

## Design correction (before implementation)

The original scoping proposed "slice 1: make `CallEnv::get`/`get_realm`
check the cell first, falling back to the raw map" as a safe, perf-neutral
first step. On reflection this was wrong: it would have added a *second*
hash lookup (into the cell registry) to the common **no-cell** read path
(most realm bindings — builtins, names never captured — never had a cell in
the old model), a real regression with no matching win until a second slice
landed. The actual fix is not "check two maps in a different order" but
"there is only one map": `RealmState.bindings` became `DynamicBindings`
(already existed in the codebase, used for other cell-backed binding
classes) instead of `RefCell<HashMap<String, Value>>`. Every read or write
is exactly one hash lookup either way, cell-backed or not — no path pays
more than before, and cell-backed names no longer pay for a second lookup
into a separate registry.

## What changed

- `RealmState.bindings: RefCell<HashMap<String, Value>>` + a separate
  `binding_cells: DynamicBindings` registry → single `bindings: DynamicBindings`
  field. `RealmState::borrow()`/`borrow_mut()` (which exposed the raw
  `HashMap` directly to ~30 call sites) removed; replaced with named methods
  (`get_value`, `contains`, `cell`, `insert_value`, `remove_value`,
  `entry_or_insert_value`) that delegate to `DynamicBindings`.
- `CallEnv::realm_binding_cell` no longer lazily creates a cell on first
  capture — every binding already has one from creation, so it's a plain
  lookup.
- `CallEnv::replace_existing_realm`/`replace_existing_realm_with_cell` no
  longer touch two storages; they call `cell.set(value)` directly.
- `Vm::initialize_script_global_bindings` signature changed from
  `&mut HashMap<String, Value>` to `&Realm`, since all three call sites
  (`vm.rs::new`, `vm_module.rs::eval_prelude_script`,
  `vm_module.rs::eval_module_body`) already had a realm, not a bare map.
- Added `Upvalue::with_value_mut` for the one genuine in-place-mutation call
  site (`vm_string_append.rs`'s `Rc::make_mut`-based string-append fast
  path), which no longer needs a "refresh the cell after the raw map's
  in-place edit" step since the cell *is* the edit target now.
- Deleted `CallEnv::to_flat_map` (confirmed unreferenced anywhere in the
  crate before removal).

## Verification performed

- `cargo test -p qjs-runtime`: 1413 passed, 0 failed.
- `./scripts/test262-subset.sh`: 5139 cases, 0 regressions.
- `./scripts/compare-qjs.sh`: all fixtures match QuickJS-NG.
- `./scripts/find-qjsng-gaps.sh --all` exact scans, zero actionable gaps on:
  `test/language/eval-code`, `test/language/module-code`,
  `test/language/global-code`, `test/annexB/language/global-code`,
  `test/language/statements/with`, `test/language/statements/for`,
  `test/built-ins/Function`.
- Hand-written script covering `delete`, `Object.defineProperty(writable:
  false)`, closures observing live global updates across frames, indirect
  `eval`, and dynamic property assignment — output matched expectations
  exactly.
- Local (informal, session already established ~15-20% noise floor on this
  machine) A/B: 20M-iteration global-var-write loop dropped from ~5.7-6.0s to
  ~5.05s user time (cumulative with the session's other commits, from a
  ~9.3s original baseline — ~46% faster overall). A `sample`-based CPU
  profile showed SipHash time drop from ~44% to ~8% of samples over the
  session's full commit range. A 25-case local broad-micro portfolio (5
  blocks) showed no case regressing beyond the established noise band, with
  several (`global_read`, `math_abs`, `method_call`, `property_read`)
  clearly improved.
- `./scripts/check.sh`: full pass (format, clippy ×2, workspace tests,
  benchmark tool tests, Test262 subset, file-size guard).

## Notes

Scoped and landed 2026-07-21 during the `perf-2x-campaign-2026-07-21` session
(see agent memory of the same name), after landing 10 unrelated,
safely-verified micro-fixes to the same store paths in the same session (see
commits `f8ad7b44`..`cefd08f7` on `main`, landed before this one). The
remaining gap between qjs-rust and QuickJS-NG on `top_level_function_call`
(and similar call/global-var-heavy cases) after this commit still requires
further work — this removed the redundant hashing but not the fundamental
per-call `Vm`/`CallEnv` construction cost or the array/property-write
families' own bottlenecks, which are separate, already-investigated (and in
some cases already-exhausted, e.g. `array_write`) territory.
