# T012: ES Modules Campaign

## Goal

Bring up ECMAScript modules end to end — parser goal symbol, module records,
linking and namespace objects, a Test262 module channel, dynamic `import()`,
and top-level `await` — so the module-flagged conformance backlog stops being
structurally not-run and dynamic-import / top-level-await behavior gains a
pass/fail signal.

## Evidence

- `test/language/expressions/dynamic-import`: ~674 fails (no `import()`).
- ~700 module-flagged cases are marked not-run by the harness: the baseline
  emits `skip_reason "module"` for any test carrying the `module` flag
  (`scripts/test262-baseline.sh`).
- `test/language/module-code/top-level-await`: ~247 cases, blocked on both
  the module goal symbol and module-scoped `await`.

## Slices

- [x] S1 Parser: `import`/`export` declarations. Default, named, namespace,
      and side-effect imports (`import x, {a as b}, * as ns from "mod"`,
      `import "mod"`); named exports, `export default` (expr/function/class),
      and re-exports (`export * from`, `export * as ns from`,
      `export {x} from`). Import assertions are out of scope. Add a
      `parse_module` entry that parses under the Module goal; `import.meta`
      and the stricter module await/reserved-word rules are noted for later
      slices, so S1 stays syntax-only. Module item AST nodes land in
      `qjs-ast` with spans. The runtime compiles any module item to a
      structured "modules are not yet supported" error. Focused parser tests
      in `tests/modules.rs`. Script-mode parsing is unchanged.
- [ ] S2 Runtime: module records + instantiation. Build a Source Text Module
      Record per module: parse, collect import entries / local + indirect /
      star export entries, link the module graph (resolve imports to exporting
      modules, allocate environment bindings), and construct Module Namespace
      exotic objects. No execution yet beyond evaluating linked module bodies
      in dependency order; ordering and binding tests at the runtime layer.
- [ ] S3 Harness: module channel. Run module-flagged Test262 cases as modules,
      resolve relative specifiers against the test's directory, wire harness
      includes as module-scope preludes, and lift the `module` skip in
      `scripts/test262-baseline.sh` to only cases the channel still cannot
      judge. Record a fresh burndown entry.
- [ ] S4 Runtime: dynamic `import()`. `import(specifier)` is a call-like
      expression returning a promise that resolves to the module namespace,
      reusing the T007 job queue; rejects on resolution/link/eval errors.
      Valid in both script and module goal. Probe
      `test/language/expressions/dynamic-import`.
- [ ] S5 Parser + runtime: top-level `await`. Permit `await` at module top
      level (Module goal only), reusing the async suspend/resume machinery
      from T007 so a module with top-level await becomes an async evaluation
      whose completion gates dependents. Probe
      `test/language/module-code/top-level-await`.

## Scope

- Allowed paths: `crates/qjs-ast/**`, `crates/qjs-lexer/**`,
  `crates/qjs-parser/**`, `crates/qjs-runtime/**`; S3 also
  `scripts/test262-baseline.sh`, `tests/test262/**`.
- Forbidden paths: `third_party/**`.
- Owner boundary: one slice per owner; S1 precedes S2, S2 precedes S3/S4,
  S5 depends on the T007 async machinery and the S1 goal symbol.
- Parser changes must not alter script-mode behavior; module-only syntax and
  reserved-word rules are gated behind the Module goal.
- Reuses the T007 deterministic single-threaded job queue for S4/S5; no
  threads or host async runtime.

## References

- `docs/architecture.md`
- `tasks/T007-async-foundation-campaign.md` (job queue, async suspension).
- QuickJS-NG: `third_party/quickjs-ng/quickjs.c`
  (`js_parse_export`, `js_parse_import`, `js_create_module_*`,
  `js_inner_module_*`, `js_dynamic_import`).
- Test262: `test/language/module-code/**`,
  `test/language/expressions/dynamic-import/**`, `harness/`.

## Acceptance Criteria

- S1: module syntax round-trips through `parse_module` with spans; script
  mode still rejects `import`/`export`; the runtime returns a structured
  not-yet-supported error for module items.
- After S3, module-flagged cases appear as pass/fail in
  `./scripts/find-qjsng-gaps.sh` output instead of not-run.
- Campaign exit: dynamic-import and top-level-await buckets show real signal
  in the burndown series.

## Verification

```sh
cargo test -p qjs-parser
cargo test -p qjs-runtime
./scripts/find-qjsng-gaps.sh --filter test/language/module-code --all
./scripts/check.sh
```

## Notes

Keep Test262 conventions (specifier resolution, harness includes) inside the
baseline harness layer; engine crates expose a module-loading API and must not
know about Test262 directory layout.
