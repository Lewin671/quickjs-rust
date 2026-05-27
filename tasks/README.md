# Tasks

Use these as agent-sized work items. Keep each task independently testable.
Concrete task files live next to this index. For new tasks, copy
`tasks/TEMPLATE.md` and fill in scope, owner boundary, acceptance criteria, and
verification commands before assigning an agent.

## Task Files

- `T001-lexer-coverage.md`
- `T002-parser-expressions.md`
- `T003-runtime-values.md`
- `T004-quickjs-comparison.md`
- `T005-test262-subset.md`

## T001: Lexer Coverage

Expand `qjs-lexer` to cover comments, template literals, regex ambiguity notes,
and more punctuators. Add focused token tests.

- Allowed paths: `crates/qjs-lexer/**`, lexer-specific tests.
- Coordinate before changing: `crates/qjs-ast/**`.
- Verification: `cargo test -p qjs-lexer` and `./scripts/check.sh`.

## T002: Parser Expressions

Add precedence parsing for unary, multiplicative, comparison, equality, logical,
assignment, and comma expressions.

- Allowed paths: `crates/qjs-parser/**`, parser-specific tests.
- Coordinate before changing: `crates/qjs-ast/**`, `crates/qjs-runtime/**`.
- Verification: `cargo test -p qjs-parser` and `./scripts/check.sh`.

## T003: Statements

Add blocks, variable declarations, `if`, loops, `return`, and function
declarations. Keep AST additions separate from runtime behavior.

- Allowed paths: `crates/qjs-parser/**`, statement parser tests.
- Coordinate before changing: `crates/qjs-ast/**`.
- Verification: `cargo test -p qjs-parser` and `./scripts/check.sh`.

## T004: Runtime Values

Introduce JavaScript value types, basic coercion, lexical environments, and
structured runtime errors.

- Allowed paths: `crates/qjs-runtime/**`, runtime tests.
- Coordinate before changing: `crates/qjs-ast/**`, `crates/qjs-parser/**`.
- Verification: `cargo test -p qjs-runtime` and `./scripts/check.sh`.

## T005: Conformance Harness

Create a small test harness that can run local fixtures and later import slices
of Test262.

- Allowed paths: `tests/test262/**`, harness scripts, test documentation.
- Coordinate before changing: `Cargo.toml`, `Cargo.lock`, workspace scripts.
- Verification: `./scripts/test262-subset.sh` and `./scripts/check.sh`.

## T006: QuickJS Comparison Runner

Build `third_party/quickjs-ng` locally and add a script that compares selected
`qjs-cli` output with QuickJS-NG output for simple smoke programs.

- Allowed paths: `scripts/compare-qjs.sh`, `tests/fixtures/compare-qjs/**`.
- Coordinate before changing: `crates/qjs-cli/**`.
- Verification: `./scripts/compare-qjs.sh` and `./scripts/check.sh`.
