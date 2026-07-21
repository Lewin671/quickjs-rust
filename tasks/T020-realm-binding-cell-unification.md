# T020: Unify realm binding storage behind the shared cell

## Goal

Remove the redundant per-store name-table hashing that remains for
global-scope hoisted `var`/function bindings even after T016 landed shared
upvalue cells for the closure-capture binding classes. This is a *different*
binding class from T016's scope (closures capturing outer variables): global
bindings are additionally observable through the `globalThis` object
(enumeration, `delete`, `Object.defineProperty`), which is why the old
triple-sync model survived here on purpose, not by omission.

Concretely: `RealmState` (`crates/qjs-runtime/src/function/env.rs`) keeps
**two separate hash maps** for the same logical binding once a name has a
cell — `bindings: RefCell<HashMap<String, Value>>` (the raw name→value table
that `CallEnv::get`/`get_realm` and ~15 direct `self.realm.borrow().get(name)`
call sites read) and `binding_cells: DynamicBindings` (a
`Rc<RefCell<HashMap<String, Upvalue>>>`, looked up by
`RealmState::binding_cells.cell(name)`). A hot store to a cell-backed global
var (e.g. a `var` written in a tight top-level loop) pays for:

1. A property-map hash lookup to write the `globalThis` own-property value
   (`ObjectRef::write_existing_own_data_property` — unavoidable, this *is*
   the JS-observable side effect).
2. A **second, separate** hash lookup into the raw `bindings` HashMap to keep
   `env.get`/`env.get_realm` fresh (`CallEnv::replace_existing_realm`/
   `replace_existing_realm_with_cell`, `crates/qjs-runtime/src/function/env.rs`).

(2) is the removable one: it exists only because `CallEnv::get`/`get_realm`
read the raw map directly instead of checking the cell first. If those two
accessors became cell-aware, the raw map lookup on the write side could be
skipped entirely for any name that already has a cell.

## Why this needs its own scoped session, not a drive-by

- **Blast radius (audited 2026-07-21, session `perf-2x-campaign-2026-07-21`
  memory)**: 34 call sites across 6 files touch
  `self.realm.borrow()`/`borrow_mut()` directly:
  `bytecode/vm.rs`, `bytecode/vm_bindings.rs`, `bytecode/vm_props.rs`,
  `bytecode/vm_string_append.rs`, `function/env.rs`, `module/link.rs`. Most
  are point lookups by name (safe to reroute through a cell-aware accessor),
  but each needs individual judgment about which binding class it targets.
- **Read-side regression risk**: making `CallEnv::get`/`get_realm` check
  `binding_cells.cell(name)` first before falling back to the raw map adds a
  *second* hash lookup to the common **no-cell** case (most realm bindings —
  builtins, names never captured or hoisted-global — never get a cell today).
  This must be benchmarked across the *full* 25-case broad-micro portfolio,
  not just the global-var-heavy cases, before landing — a regression on
  `property_read`/`global_read`/etc. would be a net loss.
- **One known-dead-but-real-pattern already found**: `CallEnv::to_flat_map`
  (`function/env.rs`) does `self.realm.borrow().clone()` — a full clone of
  the raw map. It is currently unreferenced anywhere in the crate (verified
  2026-07-21), so it isn't a live staleness risk today, but if this task adds
  a caller later, `to_flat_map` must also become cell-aware or be reasoned
  about explicitly at that time.
- **Local benchmarking is unreliable for verifying this**: the perf-2x
  session that scoped this found same-binary self-vs-self A/B swings up to
  15% from thermal/scheduling drift alone on the available dev machine.
  Verification must lean on the hosted CI Performance Preview's full 25-case
  portfolio (`.github/workflows/performance-smoke.yml`), not local timing.

## Scope

- Allowed paths: `crates/qjs-runtime/src/function/env.rs`,
  `crates/qjs-runtime/src/bytecode/vm.rs`,
  `crates/qjs-runtime/src/bytecode/vm_bindings.rs`,
  `crates/qjs-runtime/src/bytecode/vm_props.rs`,
  `crates/qjs-runtime/src/bytecode/vm_string_append.rs`,
  `crates/qjs-runtime/src/module/link.rs`.
- Forbidden paths: `third_party/**`, `qjs-parser`/`qjs-ast` (no AST changes).
- Owner boundary: serialize on one branch — touches the same shared realm
  binding code T016 owned; do not run in parallel with other runtime work.

## Suggested slicing

1. Make `CallEnv::get`/`CallEnv::get_realm` check `binding_cells.cell(name)`
   first, falling back to the raw map — a pure read-side change, no writer
   changes yet. Verify with the full 25-case local broad-micro portfolio
   (multiple blocks) *and* `./scripts/check.sh`; this slice alone should be
   perf-neutral (same total lookups, just reordered) and is the safest place
   to catch an unexpected divergence between the two maps before touching
   writes.
2. Once (1) is verified safe, skip the raw-map write
   (`replace_existing_realm`/`replace_existing_realm_with_cell`/`insert_realm`)
   for names that already have a cell, since reads now check the cell first.
   Re-audit every direct `self.realm.borrow().get(name)`/`.contains_key(name)`
   call site found in the blast-radius list above individually — each one
   either needs to go through the now-cell-aware `CallEnv` accessor, or needs
   an explicit, commented reason why it's fine to keep reading the (now
   possibly stale for cell-backed names) raw map.
3. Full hosted Performance Preview run across the entire 25-case portfolio
   before merging, not just the case(s) this task targets.

## Acceptance criteria

- No behavior change observable from JS: `globalThis` property enumeration,
  `delete`, `defineProperty`, and cross-frame reads of the same global name
  all still see the same values in the same order as before.
- `./scripts/check.sh` and the full Test262 subset pass with zero
  regressions.
- Hosted Performance Preview shows no case outside `top_level_function_call`/
  `dynamic_method_call`/other global-var-heavy cases regressing beyond noise.

## Verification

```sh
./scripts/check.sh
./scripts/test262-subset.sh
# Full 25-case local sanity (informal, expect local noise up to ~15-20%):
cargo build --release -p qjs-cli
./scripts/benchmark.sh --candidate <new> --base <old> --blocks 6 --seed <fixed> \
  --output target/benchmarks/t020-sanity.jsonl
```

## Notes

Scoped 2026-07-21 during the `perf-2x-campaign-2026-07-21` session (see agent
memory of the same name) after landing 10 unrelated, safely-verified
micro-fixes to the same store paths (see commits `f8ad7b44`..`cefd08f7` on
`main`). Deliberately not attempted in that session: the read-side regression
risk (above) cannot be responsibly bounded without a full portfolio hosted-CI
run per change, which doesn't fit inside a single continued conversation
turn's verification loop. `top_level_function_call` remains roughly 15x
slower than QuickJS-NG even after those 10 commits; this task is the next
concrete lever, not the final one — closing that gap fully may still require
additional work in the array/property-write families independently.
