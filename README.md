# quickjs-rust

[![CI](https://github.com/Lewin671/quickjs-rust/actions/workflows/ci.yml/badge.svg)](https://github.com/Lewin671/quickjs-rust/actions/workflows/ci.yml)
[![Test262 Coverage](https://github.com/Lewin671/quickjs-rust/actions/workflows/test262-coverage.yml/badge.svg)](https://github.com/Lewin671/quickjs-rust/actions/workflows/test262-coverage.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-stable-orange.svg)](rust-toolchain.toml)

**A JavaScript engine written from scratch in safe Rust.**

`quickjs-rust` is not a binding, wrapper, or translation of QuickJS. It is a
Rust-native lexer, parser, bytecode compiler, and virtual machine that uses
[QuickJS-NG](https://github.com/quickjs-ng/quickjs) purely as a behavioral
reference, with the long-term goal of QuickJS-class embeddability,
correctness, and performance — without a C runtime underneath.

```text
source text → lexer → parser → AST → bytecode compiler → bytecode VM → CLI / tests
```

## Highlights

- **100% safe Rust.** `unsafe` is forbidden across the entire workspace.
- **Conformance-driven.** 18,252 official [Test262](https://github.com/tc39/test262)
  cases pass today (June 2026, of the 42,672 in the comparison configuration),
  measured per commit in CI and tracked as a burndown trend in
  [`docs/conformance/burndown.jsonl`](docs/conformance/burndown.jsonl).
- **Differential testing.** Behavior is continuously compared against a pinned
  QuickJS-NG build, and an automated gap finder ranks the next areas to fix.
- **Diagnostics-first design.** Byte-offset source spans are preserved from the
  lexer upward, and malformed input always produces structured errors — the
  engine never panics on bad JavaScript.
- **Small, reviewable crates.** AST, lexer, parser, runtime, and CLI live in
  separate crates with strict boundaries and enforced file-size limits.

## Quick Start

Install Rust with [`rustup`](https://rustup.rs), then:

```sh
git clone --recurse-submodules https://github.com/Lewin671/quickjs-rust.git
cd quickjs-rust
./scripts/bootstrap.sh   # initializes submodules if you forgot --recurse-submodules
```

Run JavaScript through the CLI:

```sh
$ cargo run -p qjs-cli -- -e 'const fib = (n) => n < 2 ? n : fib(n - 1) + fib(n - 2); fib(10);'
Number(55.0)

$ cargo run -p qjs-cli -- -e 'const greet = (name) => `Hello, ${name}!`; greet("Rust");'
String("Hello, Rust!")

$ cargo run -p qjs-cli -- -e '[1, 2, 3, 4].filter(n => n % 2 === 0).map(n => n * 10).join("-");'
String("20-40")
```

Run the full local verification suite (format, lints, tests, size guards):

```sh
./scripts/check.sh
```

## Current Status

The engine executes a substantial JavaScript subset end to end — every
supported feature flows through the real pipeline (lexer → parser → bytecode →
VM), never through parser-only or runtime-only shortcuts.

**Working today:** closures and arrow functions, template literals,
`for`/`for...of`/`while` loops, `try`/`catch` with real error types, regular
expressions, and a growing standard library (`Object`, `Array` iteration
methods, `String`, `Math`, `JSON`, ...).

**Not yet:** classes, `async`/`await`, generators, and modules. Conformance work is intentionally incremental; the Test262 burndown
above is the honest scoreboard.

## Workspace

| Crate / directory | Role |
| --- | --- |
| `crates/qjs-ast` | Shared syntax tree and source-span types |
| `crates/qjs-lexer` | Tokenizer emitting span-preserving tokens |
| `crates/qjs-parser` | Tokens → AST, with structured parse errors |
| `crates/qjs-runtime` | Bytecode compiler, VM, values, and builtins |
| `crates/qjs-cli` | Thin command-line wrapper for smoke testing |
| `docs/` | Architecture notes and harness documentation |
| `scripts/` | Bootstrap, verification, comparison, and Test262 tooling |
| `tests/test262/` | Curated Test262-derived cases, allowlists, expected failures |
| `third_party/` | Pinned QuickJS-NG reference and TC39 Test262 corpus (read-only) |

## Verification & Conformance Tooling

`./scripts/check.sh` is the standard gate. Beyond that, the repository ships a
small conformance harness:

```sh
./scripts/compare-qjs.sh                 # differential fixtures vs. pinned QuickJS-NG
./scripts/find-qjsng-gaps.sh             # ranked queue of Test262 areas where NG passes and we don't
./scripts/test262-subset.sh              # curated, deterministic Test262-derived cases
./scripts/test262-burndown.sh --help     # record full-scan conformance trend entries
```

See [`scripts/README.md`](scripts/README.md) for the full catalog and
[`docs/harness.md`](docs/harness.md) for flags, behavior, and the
agent-integration workflow.

## Design Principles

- Build vertically through the engine stack; each feature lands with lexer,
  parser, runtime, and test coverage together.
- Keep public APIs small, typed, and documented across crate boundaries.
- Return structured errors for bad input; preserve byte-offset spans for
  diagnostics.
- Treat QuickJS-NG and Test262 as references and conformance inputs, never as
  Rust build dependencies.

For deeper context: [`docs/architecture.md`](docs/architecture.md) covers crate
boundaries and growth strategy, and [`AGENTS.md`](AGENTS.md) is the contract
for autonomous-agent contributions.

## License

This project is licensed under the [MIT license](LICENSE).
