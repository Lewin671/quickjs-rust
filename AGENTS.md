# Agent Instructions

This repository is designed for autonomous work through Codex or Harness.

## Prime Directive

Build a Rust-native JavaScript engine incrementally. Preserve subsystem
boundaries and make each change verifiable with focused tests.

## Standard Commands

- Full check: `./scripts/check.sh`
- Format only: `cargo fmt --all`
- Test only: `cargo test --workspace`
- CLI smoke test: `cargo run -p qjs-cli -- -e "1 + 2;"`

If Rust is not installed, report that clearly and do not fake test results.

## Work Boundaries

- Parser work should not mutate runtime behavior unless the task explicitly
  requires it.
- Runtime work should prefer existing AST types over adding parser shortcuts.
- Lexer tokens must carry spans.
- Public APIs should stay small and documented.
- Avoid `unsafe`; the workspace forbids it.

## Definition of Done

For code changes:

1. The relevant crate has unit tests or integration coverage.
2. `./scripts/check.sh` passes, or the failure is explained with exact output.
3. Public docs are updated when behavior or architecture changes.
4. The final response names changed files and verification performed.

## Suggested Autonomous Loop

1. Pick one task from `tasks/`.
2. Read the related crate and docs.
3. Implement the smallest useful slice.
4. Add or update tests.
5. Run `./scripts/check.sh`.
6. Summarize behavior, risks, and next task.
