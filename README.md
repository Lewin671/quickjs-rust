# quickjs-rust

An incremental Rust rewrite of the QuickJS JavaScript engine.

This repository is structured as small Rust crates with narrow interfaces and a
single workspace check command. Human-facing project context lives here; agent
execution rules live in `AGENTS.md`.

## Current Scope

The first implementation target is a small but real JavaScript pipeline:

1. Lex source text into tokens.
2. Parse a script into an AST.
3. Execute a tiny expression subset.
4. Grow conformance with focused tests.

The goal is not to bind to the C QuickJS runtime. The goal is a Rust-native
engine that can be expanded gradually while preserving clear subsystem
boundaries.

## Workspace

- `crates/qjs-ast`: shared syntax tree and source span types.
- `crates/qjs-lexer`: tokenizer.
- `crates/qjs-parser`: parser from tokens to AST.
- `crates/qjs-runtime`: interpreter/runtime experiments.
- `crates/qjs-cli`: command-line entry point for smoke tests.
- `docs/`: architecture and implementation notes for humans and agents.
- `tasks/`: small, agent-sized work items.
- `docs/harness.md`: autonomous agent runbook.
- `scripts/check.sh`: standard verification command.
- `scripts/source-size-report.sh`: reports largest first-party and vendored
  files separately.
- `scripts/compare-qjs.sh`: smoke comparison against QuickJS-NG.
- `scripts/test262-subset.sh`: runs curated Test262-derived subset cases.
- `tests/fixtures/`: local JavaScript smoke fixtures.
- `tests/test262/`: curated Test262 allowlist and expected failures.
- `third_party/quickjs-ng`: pinned QuickJS-NG reference implementation.
- `third_party/test262`: pinned TC39 ECMAScript conformance tests.

## Setup

Install Rust with `rustup`, then initialize third-party references and run the
workspace checks:

```sh
./scripts/bootstrap.sh
./scripts/check.sh
```

For a fresh clone, either clone with submodules:

```sh
git clone --recurse-submodules git@github.com:Lewin671/quickjs-rust.git
```

or run `./scripts/bootstrap.sh` after cloning. The bootstrap script initializes
the top-level upstream references and fetches Cargo dependencies.

For a quick local run:

```sh
cargo run -p qjs-cli -- -e "1 + 2;"
```

To compare the current runtime against QuickJS-NG on local smoke fixtures:

```sh
./scripts/compare-qjs.sh
```

To run the curated Test262-derived subset:

```sh
./scripts/test262-subset.sh
```

## Engineering Notes

- `qjs-ast` is the shared contract between parser and runtime.
- `qjs-lexer` and `qjs-parser` should return structured errors for user input.
- `qjs-runtime` should execute AST semantics rather than adding parser-specific
  shortcuts.
- `third_party/quickjs-ng` is a behavioral oracle, not a build dependency.
- `third_party/test262` should be consumed through small allowlisted runnable
  subsets until the engine is mature enough for broader conformance runs.
- Large files in `third_party/` are pinned upstream references. First-party Rust
  code is kept modular and is guarded by `scripts/check-file-size.sh`; use
  `scripts/source-size-report.sh` to inspect size trends without conflating
  vendored sources with engine modules.

## Automation

Codex, Harness, and other autonomous agents should start with `AGENTS.md`. That
file defines repository-specific execution rules, task boundaries, commit
discipline, parallel worktree policy, and verification expectations.
Use `docs/harness.md` for the concrete worktree, handoff, integration, and
cleanup runbook.
