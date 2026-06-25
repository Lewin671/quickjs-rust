# quickjs-rust

[![CI](https://github.com/Lewin671/quickjs-rust/actions/workflows/ci.yml/badge.svg)](https://github.com/Lewin671/quickjs-rust/actions/workflows/ci.yml)
[![Test262 Coverage](https://github.com/Lewin671/quickjs-rust/actions/workflows/test262-coverage.yml/badge.svg)](https://github.com/Lewin671/quickjs-rust/actions/workflows/test262-coverage.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-stable-orange.svg)](rust-toolchain.toml)

**A Rust-native ECMAScript engine and embeddable bytecode runtime.**

`quickjs-rust` is a Rust implementation of the ECMAScript language runtime. It
includes a span-preserving lexer and parser, bytecode compiler, virtual machine,
module linker, standard-library builtins, and a command-line host for local
execution and conformance testing.

The current normative target is ECMA-262 16th edition, June 2025
(ECMAScript 2025 / ES2025), corresponding to the `tc39/ecma262@es2025`
specification tag. TC39 living-draft features are tracked separately from the
default conformance baseline. Test262 does not publish edition-specific stable
tags; this repository therefore pins a concrete Test262 commit and evaluates it
against the ES2025 target and a pinned
[QuickJS-NG](https://github.com/quickjs-ng/quickjs) comparison baseline.

## Conformance

Conformance is measured continuously with Test262 and differential checks
against QuickJS-NG.

- 42,672 / 42,672 configured Test262 cases pass in the latest CI coverage scan.
- The current QuickJS-NG comparison baseline reports zero actionable gaps.
- Full-scan burndown entries are recorded under
  [`docs/conformance/burndown.jsonl`](docs/conformance/burndown.jsonl).
- CI runs Rust checks, QuickJS-NG comparison smoke tests, curated Test262
  subsets, and a sharded full Test262 coverage workflow.

Active work is focused on production engineering and deeper specification
coverage: performance, the slot-indexed environment/upvalue-cell model,
Temporal, and TC39 draft features beyond ES2025.

## Runtime Surface

The runtime supports:

- Script and module execution, including dynamic `import()` and top-level
  `await`.
- Lexical bindings, closures, classes, private names, generators, async
  functions, promises, regular expressions, destructuring, and Annex B behavior
  covered by the configured test suite.
- Typed arrays, ArrayBuffer and SharedArrayBuffer behavior, Atomics, and the
  Test262 `$262.agent` multi-agent harness behind the `agents` feature.
- Core standard-library objects including `Object`, `Function`, `Array`,
  `Map`, `Set`, `WeakMap`, `WeakSet`, `String`, `Number`, `BigInt`, `Math`,
  `JSON`, `Date`, `RegExp`, `Promise`, `Reflect`, `Proxy`, symbols, iterator
  helpers, and resource-management builtins.
- Structured lexer, parser, compile, and runtime errors. Tokens and syntax
  nodes preserve byte-offset spans for diagnostics.

The workspace denies Rust `unsafe` code by default.

## Getting Started

Install Rust with [`rustup`](https://rustup.rs), then:

```sh
git clone --recurse-submodules https://github.com/Lewin671/quickjs-rust.git
cd quickjs-rust
./scripts/bootstrap.sh   # initializes submodules if you forgot --recurse-submodules
```

Use the CLI for scripts, modules, or direct evaluation:

```sh
cargo run -p qjs-cli -- --raw -e 'JSON.stringify([1, 2, 3].toReversed())'
cargo run -p qjs-cli -- --module path/to/module.mjs
cargo run -p qjs-cli -- path/to/script.js
```

The runtime crate exposes direct evaluation and bytecode entry points for tests
and embedding layers:

```rust
use qjs_runtime::{Value, eval};

fn main() -> Result<(), qjs_runtime::RuntimeError> {
    let value = eval("const x = 20 + 22; x")?;
    assert_eq!(value, Value::Number(42.0));
    Ok(())
}
```

## Development

Run the standard local verification gate before submitting changes:

```sh
./scripts/check.sh
```

Additional harness commands are available for targeted conformance work:

```sh
./scripts/compare-qjs.sh                 # differential fixtures vs. pinned QuickJS-NG
./scripts/find-qjsng-gaps.sh             # ranked queue of Test262 areas where NG passes and we don't
./scripts/test262-subset.sh              # curated, deterministic Test262-derived cases
./scripts/test262-burndown.sh --help     # record full-scan conformance trend entries
```

See [`scripts/README.md`](scripts/README.md) for the full catalog and
[`docs/harness.md`](docs/harness.md) for flags, behavior, and the
agent-integration workflow.

For deeper context, see [`docs/architecture.md`](docs/architecture.md) for the
runtime architecture.

## License

This project is licensed under the [MIT license](LICENSE).
