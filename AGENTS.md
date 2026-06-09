# Agent Instructions

This repository is designed for autonomous work through Codex or Harness.
It is the authoritative instruction file for agents. `README.md` is the
human-facing project overview; do not duplicate detailed agent workflow there.

## Prime Directive

Build a Rust-native JavaScript engine incrementally. Preserve subsystem
boundaries and make each change verifiable with focused tests.

## Standard Commands

- Fresh checkout setup: `./scripts/bootstrap.sh`
- Full check: `./scripts/check.sh`
- Source size report: `./scripts/source-size-report.sh [limit] [--vendor]`
- Create agent worktree: `./scripts/create-agent-worktree.sh <task-slug> <owner-id> [base-ref]`
- Validate agent branch: `./scripts/validate-agent-branch.sh <branch> <base-sha> <allowed-path>...`
- Format only: `cargo fmt --all`
- Test only: `cargo test --workspace`
- CLI smoke test: `cargo run -p qjs-cli -- -e "1 + 2;"`
- QuickJS comparison smoke tests: `./scripts/compare-qjs.sh`
- QuickJS-NG gap discovery:
  `./scripts/find-qjsng-gaps.sh [--limit N | --all] [--filter test/<prefix>]`
- Test262 subset runner: `./scripts/test262-subset.sh`
- Conformance burndown recorder:
  `./scripts/test262-burndown.sh --report <full-scan-dir> | --entry <ci-artifact>`
- Upstream Test262 baseline runner:
  `./scripts/test262-baseline.sh [--limit N | --all] [--filter test/<prefix>] [--engine quickjs-rust|quickjs-ng|both] [--shard I/N] [--summary-json PATH] [--no-fail]`
- Watch a pushed branch CI run: `gh run list --branch <branch> --limit 1`
  followed by `gh run watch <run-id> --exit-status`

If Rust is not installed, report that clearly and do not fake test results.
Both `scripts/bootstrap.sh` and `scripts/check.sh` fall back to
`$HOME/.cargo/bin/cargo` when `cargo` is not on `PATH`.

## Work Boundaries

- Keep each change scoped to one clear feature, fix, refactor, or documentation
  update.
- Prefer the existing crate boundaries and local APIs over introducing new
  abstractions.
- Preserve module boundaries inside crates. Large implementation files should
  be split by responsibility before adding unrelated behavior.
- Keep implementation files reviewable. `scripts/check-file-size.sh` enforces
  current upper bounds for Rust source files, Rust test files, and repository
  scripts; split by subsystem responsibility before raising those limits.
- First-party files should stay small enough for direct review. If a Rust
  implementation file is trending toward the guard limit, split it by semantic
  responsibility before adding more behavior; do not wait until it fails CI.
- Treat large files under `third_party/` as vendored reference material, not as
  examples for first-party Rust structure. Use
  `./scripts/source-size-report.sh` for first-party size audits and
  `./scripts/source-size-report.sh --vendor` only when you need to inspect
  pinned upstream submodule files.
- Thousands-line files are acceptable only in vendored upstream references or
  generated conformance data under `third_party/`. New first-party code,
  fixtures, scripts, and docs should remain modular and reviewable.
- Add a new crate or third-party dependency only when it removes clear
  complexity and is justified in the final summary.
- Parser work should not mutate runtime behavior unless the task explicitly
  requires it.
- Runtime work should prefer existing AST types over adding parser shortcuts.
- Lexer tokens must carry spans.
- Public APIs should stay small and documented.
- Avoid `unsafe`; the workspace forbids it.
- Avoid global mutable state. When shared state is required, make ownership and
  lifetime explicit in runtime data structures.
- Do not edit files under `third_party/` unless the task is explicitly to update
  a submodule pointer.
- Use `third_party/quickjs-ng` as a reference implementation, not as a Rust build
  dependency.
- Use `third_party/test262` as conformance input. Prefer small allowlisted
  subsets over running the entire suite during early engine work.
- Do not initialize or rely on nested submodules under `third_party/quickjs-ng`
  unless a task explicitly requires QuickJS-NG's own test setup.
- Keep generated files and build outputs out of commits.

## Rust Engineering Standards

- Keep public APIs small, typed, and documented when they cross crate
  boundaries.
- Return structured errors for source input failures; do not panic on malformed
  JavaScript.
- Preserve source spans when adding syntax, token, parser, or diagnostic
  behavior.
- Use byte offsets for spans unless the architecture document is intentionally
  changed.
- Prefer deterministic data structures and output for tests and diagnostics.
- Do not add async, threading, FFI, unsafe code, or custom allocators unless the
  task explicitly requires that design.
- Keep CLI behavior thin. Library crates should own engine semantics and error
  models.
- Keep tests in crate-local `tests` modules/files when they are large enough to
  distract from implementation structure.
- Split large test files by the behavior under test rather than by the feature
  that happened to add them. For example, Object tests should separate
  descriptors, enumeration, integrity, prototype operations, and constructor
  behavior.
- Keep performance changes evidence-based. Add a benchmark or explain the
  measured case before optimizing core engine paths.

## Dependency Policy

- Prefer the Rust standard library for early engine code.
- Before adding a dependency, check whether an existing workspace crate or a
  small local helper is enough.
- If a dependency is added, explain why it is needed, what surface area uses it,
  and whether it affects runtime, dev-only tooling, or tests.
- Do not add dependencies from `third_party/quickjs-ng` or generated Test262
  artifacts to Cargo crates.

## Documentation Sync

- Treat docs as part of the change when behavior, architecture, setup,
  verification commands, public APIs, supported syntax, runtime semantics, or
  conformance status changes.
- If an implementation change makes `README.md`, `docs/architecture.md`,
  `docs/harness.md`, task files, crate metadata, examples, allowlists, or
  expected-failure notes inaccurate or incomplete, update the relevant document
  in the same reviewable unit.
- Keep the audience boundaries intact: `README.md` is the human-facing overview,
  `AGENTS.md` is the agent workflow contract, and detailed architecture or
  harness mechanics belong under `docs/`.
- If documentation appears stale but the correct update is outside the current
  task boundary, call it out in the final response with the exact file and topic
  that need follow-up.

## Commit Discipline

- Prefer one commit per reviewable unit of work.
- A reviewable unit is one feature, one bug fix, one refactor, one dependency
  update, or one documentation/policy update.
- Do not mix unrelated formatting, cleanup, or documentation changes into a
  feature commit.
- Do not stage user changes or unrelated workspace changes.
- Split broad tasks into small commits that each compile and have relevant
  tests where practical.
- For QuickJS-NG gap work, treat one greedy recommendation queue area or one
  coherent JavaScript semantic family as the commit boundary. Use the printed
  queue to keep momentum after a global probe, but verify each chosen area with
  `./scripts/find-qjsng-gaps.sh --filter <area> --all` before implementation
  and again before committing. Do not create one commit per individual Test262
  case; include allowlist or expected-failure updates with the implementation
  or harness support that makes them meaningful.
- Commit messages should describe the behavior or policy change, for example
  `Add lexer support for comments` or `Tighten agent workflow docs`.

## Parallel Agent Workflow

Use isolated git worktrees for parallel coding only when the task can be split
into clear, non-overlapping ownership boundaries.

- Keep `main` as the stable integration branch.
- Create one short-lived branch and one worktree per coding owner.
- Branch names should follow `agent/<task-slug>/<owner-id>`.
- Every coding owner must start from the same recorded `base sha`, unless the
  main agent explicitly re-baselines that owner.
- Every coding owner must have a path boundary before editing. Examples:
  `crates/qjs-lexer/**`, `crates/qjs-parser/**`,
  `crates/qjs-runtime/**`, or `scripts/compare-qjs.sh`.
- Push locally verified `agent/**` branches when early GitHub Actions feedback
  is useful. The workflow is configured to run CI for these branches.
- Remote CI is additional evidence, not a substitute for local checks. Do not
  integrate a branch with a red, missing, or unexplained latest branch CI run
  when that branch was pushed for CI.
- Global files default to main-agent ownership: `Cargo.toml`, `Cargo.lock`,
  `rust-toolchain.toml`, `.gitmodules`, `AGENTS.md`, `README.md`,
  `docs/architecture.md`, `docs/harness.md`, and shared CI or bootstrap
  scripts.
- Coding owners must not merge each other's branches. The main agent integrates
  owner results one branch at a time.
- Before integration, inspect the owner branch's changed files against its path
  boundary and confirm the reported base sha. Use
  `./scripts/validate-agent-branch.sh` for this check.
- For pushed owner branches, inspect the latest branch CI run with `gh run list`
  and `gh run view` before merging.
- After each integration, run `./scripts/check.sh` before integrating another
  owner branch.
- Delete merged temporary worktrees and short-lived branches unless they are
  intentionally retained for diagnosis.

Prefer serialized work on one branch when a task changes shared AST types,
workspace configuration, global error models, or broad architecture documents.

## Architecture Expectations

- `qjs-ast` owns shared syntax and span types. It should not depend on lexer,
  parser, runtime, or CLI crates.
- `qjs-lexer` owns tokenization and should preserve byte spans.
- `qjs-parser` owns syntactic structure and should not evaluate code.
- `qjs-runtime` owns evaluation semantics and should not re-parse source except
  through public parser APIs.
- `qjs-cli` is a thin smoke-test wrapper. Keep engine policy in library crates.
- Runtime builtins should stay grouped by JavaScript object and behavior family
  (`array/iteration`, `date/iso`, `object/descriptor`, and similar modules)
  rather than accumulating unrelated native methods in a single file.

## Test Strategy

- Unit tests belong next to the crate behavior they exercise.
- Cross-crate behavior should use integration tests once the public surface is
  stable enough.
- Every behavior change should include a focused test at the lowest useful
  layer.
- Use QuickJS-NG comparisons for selected smoke programs when semantics are
  unclear.
- Use Test262 through curated runnable subsets with explicit provenance and
  expected failures; do not treat full-suite failure counts as useful signal
  during early development.
- Use `./scripts/test262-baseline.sh` to enumerate upstream Test262 coverage
  and classify unsupported harness metadata, parser/runtime failures, and
  timeouts before expanding the curated subset.
- When a Test262 case is expected to fail, record the reason near the allowlist
  or expected-failure list rather than relying on tribal knowledge.
- Record a conformance burndown entry with `./scripts/test262-burndown.sh`
  after every complete `--exact --all` comparison scan, and prefer the CI
  `test262-burndown` artifact for per-commit measurements. The trend in
  `docs/conformance/burndown.jsonl` is the signal for changing recommendation
  strategy or campaign priorities; do not record partial scans.
- Prefer small fixtures that are easy to inspect over broad generated tests for
  early parser and runtime work.
- Add simple QuickJS comparison programs under `tests/fixtures/compare-qjs/`.
- Add Test262-derived cases under `tests/test262/cases/` only when the current
  harness can run them deterministically. Each case must start with
  `// Derived from: <official Test262 path>` so the runner can validate
  provenance.
- Add those local case paths to `tests/test262/allowlist.txt`. The allowlist may
  also include raw upstream paths under `test/` when the pinned Test262 case can
  run deterministically through `scripts/test262-subset.sh` with its declared
  harness metadata.
- Run `./scripts/test262-subset.sh` after editing Test262 allowlists or expected
  failures.

## Definition of Done

For code changes:

1. The relevant crate has unit tests or integration coverage.
2. `./scripts/check.sh` passes, or the failure is explained with exact output.
3. Public docs and crate metadata are updated when behavior, architecture,
   setup, commands, APIs, or conformance expectations change.
4. New dependencies, public APIs, or architecture shifts are justified.
5. The final response names changed files and verification performed.

For documentation-only changes:

1. The changed document has a single clear audience.
2. Instructions are not duplicated across README and AGENTS unless the summary is
   intentionally brief.
3. `./scripts/check.sh` is run when scripts, Cargo files, or examples changed.

## Suggested Autonomous Loop

1. Pick one task from `tasks/`.
2. Read the related crate, `docs/architecture.md`, and `docs/harness.md`.
3. Implement the smallest useful slice.
4. Add or update tests.
5. Run `./scripts/check.sh`.
6. Summarize behavior, risks, verification, and the next useful task.

If requirements are ambiguous, prefer a small reversible implementation and
state the assumption. Ask for user input only when the ambiguity changes public
architecture, dependency choices, or long-term compatibility.
