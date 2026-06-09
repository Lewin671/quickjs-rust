# T008: Destructuring Completion Campaign

## Goal

Finish destructuring across all binding and assignment positions. At commit
`3e7feb0` (2026-06-09 full scan) `destructuring-binding` tags 2,228 actionable
gaps — the largest actionable feature cluster — plus 662 `default-parameters`,
116 `object-rest`, and 240 not-run cases behind the for-of destructuring
syntax pre-filter.

## Evidence

- Variable-declaration destructuring landed at `3e7feb0` (`Add variable
  destructuring declarations`); remaining positions still fail.
- The for-of head pre-filter lives in `rust_source_syntax_supported` in
  `scripts/test262-baseline.sh` (236 cases for-of-only, 4 overlapping class).

## Slices

- [x] S1 Function parameters: binding patterns in function/arrow parameters,
      including nested patterns and rest elements.
- [x] S2 Parameter defaults: default values in plain and pattern parameters
      (`default-parameters` cluster).
- [ ] S3 Assignment patterns: destructuring assignment expressions,
      including object rest (`object-rest`) and evaluation order.
- [ ] S4 Statement heads: patterns in `for-of`/`for-in` heads and `catch`
      parameters; lift the for-of pre-filter in
      `scripts/test262-baseline.sh` and record a fresh burndown entry.
- [ ] S5 Re-cluster: run a focused gap scan over
      `test/language/expressions/assignment` and
      `test/language/statements`; file follow-up slices for what remains.

## Scope

- Allowed paths: `crates/qjs-ast/**`, `crates/qjs-parser/**`,
  `crates/qjs-runtime/**`; S4 also `scripts/test262-baseline.sh`,
  `tests/test262/**`.
- Forbidden paths: `third_party/**`.
- Owner boundary: one slice per owner; reuse the binding-pattern AST and
  evaluation paths introduced for variable declarations — do not add a
  parallel pattern representation.

## References

- `docs/architecture.md`
- QuickJS-NG: `third_party/quickjs-ng/quickjs.c` (`js_parse_destructuring_element`).
- Test262: `test/language/expressions/assignment/dstr/**`,
  `test/language/statements/for-of/dstr/**`.

## Acceptance Criteria

- Each slice reduces the focused gap count for its position family in
  `./scripts/find-qjsng-gaps.sh --filter <area> --all` and adds parser plus
  runtime tests for coercion and evaluation order.
- Campaign exit: `destructuring-binding` stops appearing among the top
  feature clusters of the actionable gap report.

## Verification

```sh
cargo test -p qjs-parser
cargo test -p qjs-runtime
./scripts/find-qjsng-gaps.sh --filter test/language/expressions/assignment --all
./scripts/check.sh
```

## Notes

This campaign overlaps `test/language/expressions` (2,474 gap cases) and
`test/language/statements` (1,990), the two largest gap areas. Default
parameters interact with arguments-object semantics; record any intentional
deviation as expected failures with reasons.
