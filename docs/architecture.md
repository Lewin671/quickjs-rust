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
- `third_party/test262`: official ECMAScript conformance input consumed by the
  subset and baseline runners documented in `docs/harness.md`.

These directories are intentionally outside the Cargo workspace. They should not
be imported by library crates or edited as part of normal engine work.
Their large generated and monolithic upstream files are expected and should not
drive first-party Rust module shape.

Only top-level submodules are initialized by `scripts/bootstrap.sh`. QuickJS-NG
also declares its own nested `test262` submodule, but this repository uses the
top-level `third_party/test262` checkout as the conformance source to avoid
duplicated test trees.

## Crates

### qjs-ast

Owns syntax and span types shared by every higher layer. Keep this crate free of
lexer, parser, and runtime dependencies.

Internal organization target:

- `span`: source byte ranges.
- `stmt`: statement nodes and statement-adjacent types.
- `expr`: expression nodes and operators.

### qjs-lexer

Converts UTF-8 source text into tokens. Tokens preserve source spans for parser
errors and future diagnostics.

Internal organization target:

- `token`: token and token-kind definitions.
- `lexer`: scanner state machine and character handling.
- `tests`: crate-local lexer behavior tests.

### qjs-parser

Builds AST nodes from token streams. The parser should be deterministic and
should return structured errors rather than panicking on user input.

Internal organization target:

- `parser`: parser state machine.
- `error`: parse error type and helpers.
- `tests`: crate-local syntax tests.

### qjs-runtime

Compiles parsed AST into runtime bytecode and executes it in the bytecode VM.
The runtime should not evaluate AST nodes directly; AST remains the
parser/compiler input boundary.

Internal organization target:

- `value`: JavaScript value, object storage, and property descriptors.
- `function`: function objects and call metadata.
- `builtins`: native builtin registration and dispatch, split by standard
  object family as it grows.
- `bytecode`: AST-to-bytecode lowering, bytecode IR, and VM dispatch.
- `convert`: ECMAScript conversion helpers.
- `tests`: crate-local runtime behavior tests.

Execution tiers inside `bytecode`, from most general to most specialized:

- `vm`: the general interpreter over a `FrameState`; every body runs here
  unless a tier below admits it, and every tier below falls back to it.
- `typed_loop`, `vm_numeric_loop`, `vm_control_loop`,
  `vm_numeric_mutation_loop`: loop-region accelerators consulted at a
  frame's backward edges; they run one region and return to the frame.
- `compact_fn`: whole-body register execution without a `FrameState`. The
  numeric tier admits stack, local, binary and call operations only, and its
  dispatch loop is kept small on purpose. `compact_fn::wide` is a separate
  executor with a superset operation set (named property reads and writes,
  `this`, resolved calls, unary and update operators, computed reads, loops
  no accelerator claims); a body is tried on the numeric tier first, then
  the wide tier. Both tiers run admitted callees in a window of their own
  register stack and re-enter the ordinary call path for everything else.

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
- Test262 subset checks use curated allowlists with explicit expected failures;
  baseline and gap scans provide broader evidence as documented in the harness.

## Growth Strategy

Add language features vertically when useful: token support, AST type, parser
tests, bytecode lowering, VM/runtime behavior, and CLI smoke coverage. This
keeps Harness tasks self-contained and makes regressions easier to localize.

Keep files small enough for autonomous review. Implementation modules should
stay focused on one engine concern, and tests should be split by behavior under
test once a file becomes difficult to scan. The CI check in
`scripts/check-file-size.sh` is an upper bound, not a target size.

Use `scripts/source-size-report.sh` when auditing structure. It reports
first-party source files by default and scans vendored QuickJS-NG and Test262
submodule files only with `--vendor`, so agents can distinguish project
architecture problems from pinned reference material.

When a feature is large enough to need conformance coverage, start with a small
allowlist from `third_party/test262`, compare selected behavior against
`third_party/quickjs-ng`, then expand the allowlist as implementation coverage
improves.
