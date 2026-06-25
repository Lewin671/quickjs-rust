# quickjs-rust

[![CI](https://github.com/Lewin671/quickjs-rust/actions/workflows/ci.yml/badge.svg)](https://github.com/Lewin671/quickjs-rust/actions/workflows/ci.yml)
[![Test262 Coverage](https://github.com/Lewin671/quickjs-rust/actions/workflows/test262-coverage.yml/badge.svg)](https://github.com/Lewin671/quickjs-rust/actions/workflows/test262-coverage.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-stable-orange.svg)](rust-toolchain.toml)

**A Rust-native ECMAScript runtime with QuickJS-class embeddability goals.**

`quickjs-rust` is a JavaScript engine implemented from scratch in safe Rust. It
ships its own span-preserving lexer and parser, bytecode compiler, virtual
machine, module linker, standard-library builtins, and conformance harness. It
is not a binding, wrapper, or translation of C QuickJS.

The default conformance target is the latest ratified ECMAScript standard:
ECMA-262 16th edition, June 2025 (ECMAScript 2025 / ES2025), corresponding to
the `tc39/ecma262@es2025` specification tag. TC39 living-draft work is tracked
separately as future input. Test262 does not publish edition-specific stable
tags, so this repository pins a concrete Test262 commit and interprets results
through the ES2025 target plus a pinned
[QuickJS-NG](https://github.com/quickjs-ng/quickjs) comparison baseline.

```text
source text → lexer → parser → AST → bytecode compiler → bytecode VM → CLI / tests
```

## Status

This is a substantial, end-to-end ECMAScript implementation, not a parser demo.
Every supported feature flows through the engine pipeline: source text, tokens,
AST, bytecode, VM execution, host integration, and tests.

- **Conformance:** 42,672 / 42,672 configured Test262 cases pass in the latest
  CI coverage scan, with zero Rust failures and zero actionable gaps against
  the pinned QuickJS-NG comparison baseline.
- **Safety:** the workspace denies Rust `unsafe` code by default.
- **Runtime surface:** scripts, modules, dynamic `import()`, top-level `await`,
  promises, async functions, generators, classes and private names, regular
  expressions, typed arrays and buffers, Atomics/Test262 agents, explicit
  resource-management syntax, and broad ES builtins including `Object`, `Array`,
  `Map`, `Set`, `String`, `Math`, `JSON`, `Promise`, and iterator helpers.
- **Diagnostics:** tokens and AST nodes preserve byte-offset spans, and
  malformed JavaScript returns structured errors instead of panicking.
- **Verification:** CI runs Rust checks, QuickJS-NG differential smoke tests,
  the curated Test262 subset, and the sharded full Test262 coverage workflow.

The main remaining engineering tracks are production performance, the
slot-indexed environment/upvalue-cell rewrite, wider Temporal coverage, and
future TC39 draft features beyond the ES2025 baseline.

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

## Usage

`qjs-cli` is the command-line entry point used by smoke tests, examples, and the
Test262 harness:

```sh
cargo run -p qjs-cli -- path/to/script.js
cargo run -p qjs-cli -- --module path/to/module.mjs
cargo run -p qjs-cli -- --raw -e 'JSON.stringify([1, 2, 3].toReversed())'
```

The runtime crate also exposes library entry points for tests and embedders:

```rust
use qjs_runtime::{Value, eval};

fn main() -> Result<(), qjs_runtime::RuntimeError> {
    let value = eval("const x = 20 + 22; x")?;
    assert_eq!(value, Value::Number(42.0));
    Ok(())
}
```

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

## Conformance Tooling

`./scripts/check.sh` is the standard gate. Beyond that, the repository ships a
conformance and comparison harness:

```sh
./scripts/compare-qjs.sh                 # differential fixtures vs. pinned QuickJS-NG
./scripts/find-qjsng-gaps.sh             # ranked queue of Test262 areas where NG passes and we don't
./scripts/test262-subset.sh              # curated, deterministic Test262-derived cases
./scripts/test262-burndown.sh --help     # record full-scan conformance trend entries
```

See [`scripts/README.md`](scripts/README.md) for the full catalog and
[`docs/harness.md`](docs/harness.md) for flags, behavior, and the
agent-integration workflow.

Full-scan conformance history is tracked in
[`docs/conformance/burndown.jsonl`](docs/conformance/burndown.jsonl).

## Design

- Rust-native implementation: no C QuickJS runtime underneath and no QuickJS-NG
  build dependency.
- Clear crate boundaries: AST, lexer, parser, runtime, Unicode tables, and CLI
  are separate packages.
- Vertical feature work: syntax, bytecode, VM behavior, builtins, and tests land
  together.
- Conformance-first development: Test262, focused fixtures, and QuickJS-NG
  differential checks drive behavior.

For deeper context: [`docs/architecture.md`](docs/architecture.md) covers crate
boundaries and growth strategy. [`AGENTS.md`](AGENTS.md) is the contract for
autonomous-agent contributions; it intentionally contains workflow details that
do not belong in this human-facing overview.

## License

This project is licensed under the [MIT license](LICENSE).
