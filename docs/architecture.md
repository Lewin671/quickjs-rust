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

## Growth Strategy

Add language features vertically when useful: token support, AST type, parser
tests, runtime behavior, and CLI smoke coverage. This keeps Harness tasks
self-contained and makes regressions easier to localize.

When a feature is large enough to need conformance coverage, start with a small
allowlist from `third_party/test262`, compare selected behavior against
`third_party/quickjs-ng`, then expand the allowlist as implementation coverage
improves.
