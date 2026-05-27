# quickjs-rust

An incremental Rust rewrite of the QuickJS JavaScript engine.

This repository is intentionally structured for autonomous agent work: small
crates, narrow interfaces, explicit milestones, and one command that checks the
whole workspace.

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
- `scripts/check.sh`: standard verification command.

## Setup

Install Rust with `rustup`, then run:

```sh
./scripts/check.sh
```

For a quick local run:

```sh
cargo run -p qjs-cli -- -e "1 + 2;"
```

## Agent Workflow

Before making changes, read:

1. `AGENTS.md`
2. `docs/architecture.md`
3. The relevant file in `tasks/`

Keep changes scoped to one subsystem when possible, add tests with behavior
changes, and finish by running `./scripts/check.sh`.
