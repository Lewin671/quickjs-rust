# quickjs-rust

`quickjs-rust` is an incremental Rust-native JavaScript engine project inspired
by QuickJS. The repository is not a binding to the C runtime; it is a ground-up
implementation that uses small Rust crates, explicit subsystem boundaries, and
focused conformance checks to grow engine behavior safely.

The project is currently in early engine development. It has a working pipeline
for tokenizing source text, parsing a growing JavaScript syntax subset into an
AST, and evaluating selected runtime behavior through a thin CLI and crate-level
tests.

## Goals

- Build a JavaScript engine in safe Rust with clear ownership of lexer, parser,
  AST, runtime, and CLI responsibilities.
- Preserve source spans and structured errors so diagnostics can improve without
  reshaping the core architecture.
- Use QuickJS-NG as a behavioral reference and Test262 as curated conformance
  input, without making either a Rust build dependency.
- Keep each implementation step reviewable, testable, and small enough to
  reason about independently.

## Non-Goals

- This is not a drop-in replacement for QuickJS today.
- This project does not expose C FFI bindings to QuickJS.
- The full Test262 suite is not expected to pass during early development.
- Runtime shortcuts should not bypass the parser or shared AST model.

## Workspace Layout

```text
source text
    |
    v
qjs-lexer  -> tokens with byte spans
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

- `crates/qjs-ast`: shared syntax tree, statement/expression nodes, and source
  span types.
- `crates/qjs-lexer`: tokenizer that emits span-preserving tokens.
- `crates/qjs-parser`: parser that turns tokens into AST nodes and structured
  parse errors.
- `crates/qjs-runtime`: interpreter/runtime layer for JavaScript values,
  operations, builtins, and evaluation semantics.
- `crates/qjs-cli`: minimal command-line wrapper for smoke testing engine
  behavior.
- `docs/`: architecture notes and human-readable implementation guidance.
- `scripts/`: bootstrap, verification, source-size, QuickJS comparison, and
  Test262 subset tooling.
- `tests/fixtures/`: local JavaScript programs used for smoke comparisons.
- `tests/test262/`: curated Test262-derived cases, allowlists, and expected
  failures.
- `third_party/quickjs-ng`: pinned QuickJS-NG reference implementation.
- `third_party/test262`: pinned TC39 conformance test corpus.

## Quick Start

Install Rust 1.85 or newer with `rustup`, then initialize the repository:

```sh
./scripts/bootstrap.sh
```

Run the full local verification suite:

```sh
./scripts/check.sh
```

Run a small JavaScript expression through the CLI:

```sh
cargo run -p qjs-cli -- -e "1 + 2;"
```

For a fresh clone, either clone submodules up front:

```sh
git clone --recurse-submodules git@github.com:Lewin671/quickjs-rust.git
```

or run `./scripts/bootstrap.sh` after cloning. The bootstrap script initializes
the top-level third-party references and fetches Cargo dependencies.

## Verification

The standard project check is:

```sh
./scripts/check.sh
```

Useful focused checks include:

```sh
cargo fmt --all
cargo test --workspace
cargo run -p qjs-cli -- -e "1 + 2;"
./scripts/compare-qjs.sh
./scripts/test262-subset.sh
./scripts/source-size-report.sh
```

`./scripts/compare-qjs.sh` compares selected local fixtures against the pinned
QuickJS-NG reference. `./scripts/test262-subset.sh` runs only curated
Test262-derived cases that the current harness can execute deterministically.

## Development Model

Engine work should grow vertically through the layers: token support, AST shape,
parser coverage, runtime semantics, and targeted smoke/conformance tests. Public
APIs should stay small and typed, user input failures should return structured
errors, and all tokens or AST nodes that represent source text should preserve
byte spans.

First-party source files are intentionally kept reviewable. Large files under
`third_party/` are vendored upstream references and should not be used as
examples for first-party Rust module structure.

For deeper design context, see:

- `docs/architecture.md` for crate boundaries, span policy, and growth strategy.
- `docs/harness.md` for autonomous-agent worktree and integration procedures.
- `AGENTS.md` for repository-specific agent instructions and commit discipline.

## License

This project is licensed under the MIT license.
