# T006: Class Campaign

## Goal

Implement `class` declarations and expressions end to end so the Test262
baseline can stop pre-filtering class syntax. This is the single largest
conformance unlock: at commit `3e7feb0` (2026-06-09 full scan), 7,138
QuickJS-NG-passing cases are structurally not-run only because the baseline
`syntax` filter excludes any file containing the `class` keyword.

## Evidence

- `comparison.ng_pass_rust_not_run` syntax bucket: 7,374 cases; 7,138 of them
  hit the `class` pre-filter (236 hit the for-of destructuring pre-filter,
  tracked by T008).
- The pre-filter lives in `rust_source_syntax_supported` in
  `scripts/test262-baseline.sh`.

## Slices

Work one slice per reviewable unit, in order. Tick a slice only after its
verification command is green and `./scripts/check.sh` passes.

- [ ] S1 Parser: class declarations and expressions with constructor and
      prototype methods (no extends, no static, no accessors). AST types,
      spans, focused parser tests.
- [ ] S2 Runtime: instantiate S1 classes — constructor function objects,
      `prototype` wiring, `new` semantics, method definitions.
- [ ] S3 Parser + runtime: `static` methods, getters/setters, computed
      method names.
- [ ] S4 Parser + runtime: `extends`, `super` calls and `super` property
      access, derived-constructor `this` rules.
- [ ] S5 Harness: narrow the class pre-filter in
      `scripts/test262-baseline.sh` so runnable class cases execute; record a
      fresh burndown entry from the next full scan.
- [ ] S6 Follow-up surface, one slice each as needed: class fields, private
      names (`#x`), static blocks. Re-cluster remaining class gaps first.

## Scope

- Allowed paths: `crates/qjs-ast/**`, `crates/qjs-lexer/**`,
  `crates/qjs-parser/**`, `crates/qjs-runtime/**`; S5 also
  `scripts/test262-baseline.sh`, `tests/test262/**`.
- Forbidden paths: `third_party/**`.
- Owner boundary: one slice per owner; AST changes serialize with other
  parser work.

## References

- `docs/architecture.md`
- QuickJS-NG: `third_party/quickjs-ng/quickjs.c` (`js_parse_class`,
  class runtime helpers).
- Test262: `test/language/statements/class/**`,
  `test/language/expressions/class/**`.

## Acceptance Criteria

- Each slice lands with focused parser/runtime tests at the lowest layer.
- After S5, `./scripts/find-qjsng-gaps.sh --filter test/language/statements/class --all`
  reports the remaining gaps as engine failures, not not-run cases.
- Campaign exit: class areas no longer dominate the not-run bucket in the
  burndown series.

## Verification

```sh
cargo test -p qjs-parser
cargo test -p qjs-runtime
./scripts/find-qjsng-gaps.sh --filter test/language/statements/class --all
./scripts/check.sh
```

## Notes

Keep the baseline pre-filter in place until S5; lifting it early floods the
gap report with parse failures and hides smaller wins. Update the checklist
above as slices land so the next session can resume without re-deriving
state.
