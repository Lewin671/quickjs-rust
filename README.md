# quickjs-rust

A Rust-native JavaScript engine aiming for QuickJS-class embeddability,
correctness, and performance without binding to the C runtime.

`quickjs-rust` is a from-scratch JavaScript engine written in safe Rust. It uses
QuickJS-NG as a behavioral reference, but the goal is not to wrap, translate, or
clone QuickJS. The goal is to build a mature embeddable runtime with Rust
ownership, modular internals, span-preserving diagnostics, and conformance-driven
development from the beginning.

The project is still early, but it is not a demo. The repository already has a
working engine pipeline:

```text
source text
    |
    v
lexer -> parser -> AST -> bytecode compiler -> bytecode VM -> CLI / tests
```

That pipeline is being expanded vertically: syntax, AST representation,
bytecode lowering, runtime semantics, builtins, QuickJS comparison fixtures, and
curated Test262 coverage move together so each feature can be reviewed and
verified as engine behavior rather than isolated parser or runtime shortcuts.

## Why This Exists

QuickJS and QuickJS-NG prove that a compact, embeddable JavaScript engine can be
practical. `quickjs-rust` takes that target seriously while choosing a different
implementation foundation:

- Safe Rust instead of C runtime ownership and memory management.
- Small crate boundaries for AST, lexer, parser, runtime, and CLI layers.
- Source spans and structured errors preserved from the lexer upward.
- A bytecode VM architecture that can grow into a production runtime.
- QuickJS-NG comparisons and Test262-derived cases as regular development
  inputs, not late-stage afterthoughts.

The long-term bar is a real embeddable JavaScript engine: small enough to reason
about, strict enough to test, and ergonomic enough for Rust applications to
embed directly.

## Current Status

The engine currently supports a focused JavaScript subset across the full
pipeline. It can tokenize source text, parse supported syntax into shared AST
types, compile AST nodes into runtime bytecode, execute selected semantics in
the VM, and expose that behavior through a thin command-line wrapper.

It is not yet a drop-in replacement for QuickJS or QuickJS-NG. Conformance work
is intentionally incremental: unsupported syntax and runtime behavior are added
with focused tests, QuickJS comparison fixtures where useful, and curated
Test262-derived cases when the harness can run them deterministically.

## Try It

Install Rust 1.85 or newer with `rustup`, then initialize the repository:

```sh
./scripts/bootstrap.sh
```

Run a small JavaScript expression through the CLI:

```sh
cargo run -p qjs-cli -- -e "1 + 2;"
```

Run the standard local verification suite:

```sh
./scripts/check.sh
```

For a fresh clone, either clone submodules up front:

```sh
git clone --recurse-submodules git@github.com:Lewin671/quickjs-rust.git
```

or run `./scripts/bootstrap.sh` after cloning. The bootstrap script initializes
the top-level third-party references and fetches Cargo dependencies.

## Workspace

- `crates/qjs-ast`: shared syntax tree, statement/expression nodes, and source
  span types.
- `crates/qjs-lexer`: tokenizer that emits span-preserving tokens.
- `crates/qjs-parser`: parser that turns tokens into AST nodes and structured
  parse errors.
- `crates/qjs-runtime`: bytecode compiler and VM for JavaScript values,
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

## Verification

The standard project check is:

```sh
./scripts/check.sh
```

See `scripts/README.md` for the full script catalog and intended usage.

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
Test262-derived cases that the current harness can execute deterministically;
cases listed in `tests/test262/expected-failures.txt` may fail until the named
support gap is implemented, and passing expected-failure cases are reported as
stale entries.

## Design Principles

- Build vertically through the engine stack instead of adding parser-only or
  runtime-only shortcuts.
- Keep public APIs small, typed, and documented when they cross crate
  boundaries.
- Return structured errors for source input failures rather than panicking on
  malformed JavaScript.
- Preserve byte-offset source spans for future diagnostics.
- Keep QuickJS-NG and Test262 as references and conformance inputs, not Rust
  dependencies.
- Keep first-party modules small enough for direct review.

For deeper design and workflow context, see:

- `docs/architecture.md` for crate boundaries, span policy, and growth strategy.
- `docs/harness.md` for autonomous-agent worktree and integration procedures.
- `AGENTS.md` for repository-specific agent instructions and commit discipline.

## License

This project is licensed under the MIT license.
