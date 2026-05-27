# Architecture

The rewrite is split into small crates so autonomous agents can work without
touching unrelated engine layers.

```text
source text
    |
    v
qjs-lexer  -> tokens with spans
    |
    v
qjs-parser -> qjs-ast
    |
    v
qjs-runtime
    |
    v
qjs-cli / tests
```

## Third-Party References

This repository pins upstream references as git submodules:

- `third_party/quickjs-ng`: mature QuickJS-derived engine used as a behavioral
  oracle and implementation reference.
- `third_party/test262`: official ECMAScript conformance tests used as input for
  future subset runners.

These directories are intentionally outside the Cargo workspace. They should not
be imported by library crates or edited as part of normal engine work.

Only top-level submodules are initialized by `scripts/bootstrap.sh`. QuickJS-NG
also declares its own nested `test262` submodule, but this repository uses the
top-level `third_party/test262` checkout as the conformance source to avoid
duplicated test trees.

## Crates

### qjs-ast

Owns syntax and span types shared by every higher layer. Keep this crate free of
lexer, parser, and runtime dependencies.

### qjs-lexer

Converts UTF-8 source text into tokens. Tokens preserve source spans for parser
errors and future diagnostics.

### qjs-parser

Builds AST nodes from token streams. The parser should be deterministic and
should return structured errors rather than panicking on user input.

### qjs-runtime

Executes AST nodes. Early runtime work is intentionally tiny; correctness and
clear semantics matter more than breadth.

### qjs-cli

Thin wrapper for manual smoke tests. Keep policy and engine behavior in library
crates rather than the CLI.

## Error and Span Policy

User input errors should be returned as structured errors with source spans
where the layer has enough information. Panics are acceptable only for internal
invariants that indicate a bug in the engine implementation.

Spans are byte offsets into the original UTF-8 source. This matches Rust string
slicing and keeps diagnostics deterministic while the diagnostic layer is still
small.

## Verification Layers

- Crate unit tests validate local behavior.
- Workspace checks validate formatting, lints, and all tests.
- QuickJS-NG comparison tests should be added for selected semantic questions.
- Test262 should be introduced through curated allowlists with explicit expected
  failures.

## Growth Strategy

Add language features vertically when useful: token support, AST type, parser
tests, runtime behavior, and CLI smoke coverage. This keeps Harness tasks
self-contained and makes regressions easier to localize.

When a feature is large enough to need conformance coverage, start with a small
allowlist from `third_party/test262`, compare selected behavior against
`third_party/quickjs-ng`, then expand the allowlist as implementation coverage
improves.
