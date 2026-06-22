# Agent Instructions

Authoritative instruction file for autonomous agents (Claude Code, Codex, or
similar; `CLAUDE.md` is a symlink to this file). `README.md` is the
human-facing overview; architecture and harness mechanics live under `docs/`.
Do not duplicate agent workflow there.

## Prime Directive

Build a Rust-native JavaScript engine toward two converging goals: **100%
Test262 conformance** and **production-grade performance**. Work incrementally,
preserve subsystem boundaries, and make each change verifiable with focused
tests — but no longer cap the design at "small and safe by default." When a
correctness or performance ceiling is structural, the right change is to lift
the structure, not to add another heuristic around it.

The keystone of both goals is the **environment / binding model**. The current
per-frame `HashMap<String, Value>` snapshot plus capture-writeback heuristics is
the shared root of the remaining conformance failures (closure/`eval`/method
capture staleness — M2, per-iteration, generator) *and* the call-path cost
(per-call HashMap allocation + deep clone). Migrating to a slot-indexed locals +
shared upvalue-cell model is the highest-ROI work and should be sequenced first;
most remaining env-model gaps are expected to fall out of it rather than be
patched individually.

## Standard Commands

- Fresh checkout setup: `./scripts/bootstrap.sh`
- Touched check (staged, AI-friendly pre-commit gate):
  `./scripts/check-touched.sh --staged --explain`
- Touched check against a base ref:
  `./scripts/check-touched.sh --base <ref> --explain`
- Full check (fmt, clippy, tests, file-size guard, Test262 subset): `./scripts/check.sh`
- CLI smoke test: `cargo run -p qjs-cli -- -e "1 + 2;"`
- QuickJS-NG comparison smoke tests: `./scripts/compare-qjs.sh`
- Gap discovery: `./scripts/find-qjsng-gaps.sh [--all] [--filter test/<prefix>]`
- Test262 subset runner: `./scripts/test262-subset.sh`
- Test262 baseline scan: `./scripts/test262-baseline.sh`
- Burndown recorder: `./scripts/test262-burndown.sh --report <dir> | --entry <file>`
- Source size report: `./scripts/source-size-report.sh [limit] [--vendor]`
- Agent worktree: `./scripts/create-agent-worktree.sh <task-slug> <owner-id> [base-ref]`
- Branch scope check: `./scripts/validate-agent-branch.sh <branch> <base-sha> <path>...`
- Branch CI: `gh run list --branch <branch> --limit 1`, then
  `gh run watch <run-id> --exit-status`

Full flags and behavior for the Test262 and gap scripts are documented in
`docs/harness.md`. `bootstrap.sh` and `check.sh` fall back to
`$HOME/.cargo/bin/cargo` when `cargo` is not on `PATH`. If Rust is not
installed, report that clearly and do not fake test results.

## Work Boundaries

- One change = one clear feature, fix, refactor, or documentation update.
- Prefer existing crate boundaries and local APIs over new abstractions; a new
  crate or dependency must remove clear complexity and be justified in the
  final summary.
- Keep first-party files reviewable: split by semantic responsibility before a
  file trends toward the `scripts/check-file-size.sh` limits, not after CI
  fails. Thousands-line files are acceptable only under `third_party/`.
- Parser work must not mutate runtime behavior unless the task requires it;
  runtime work should use existing AST types instead of parser shortcuts;
  lexer tokens must carry spans.
- `unsafe` is permitted only as an audited, localized optimization (e.g.
  NaN-boxed/tagged values, inline caches, arena/GC internals). The workspace
  lint is `unsafe_code = "deny"`, overridden per-module with an explicit
  `#[allow(unsafe_code)]` and a `// SAFETY:` comment on every `unsafe` block;
  a sound safe equivalent must not exist or must be a measured bottleneck.
  Default to safe code; never reach for `unsafe` for convenience. No global
  mutable state; make shared ownership and lifetime explicit in runtime data
  structures.
- `third_party/` is read-only reference material: never edit it outside an
  explicit submodule-pointer task, never use `quickjs-ng` as a build
  dependency, and do not initialize its nested submodules. `test262` is
  conformance input, consumed through the harness scripts.
- Keep generated files and build outputs out of commits.

## Rust Engineering Standards

- Public APIs that cross crate boundaries stay small, typed, and documented.
- Return structured errors for source input failures; never panic on
  malformed JavaScript.
- Preserve source spans (byte offsets) in token, syntax, and diagnostic work.
- Prefer deterministic data structures and output for tests and diagnostics.
- No FFI and no Rust `async`: the engine stays pure Rust, and JS async is
  implemented by the engine's own event loop / VM, never by Rust's async
  runtime. Never take `quickjs-ng` as a build dependency.
- A tracing GC or arena allocator is allowed and expected to replace
  `Rc`/`RefCell` for runtime values — needed both to collect reference cycles
  (a correctness/leak requirement for production grade) and to make allocation
  cheap. Treat it as a deliberate, staged subsystem, not a drive-by change.
- OS threads are allowed only behind a cargo feature and only to back
  `SharedArrayBuffer` / `Atomics` and the Test262 `$262.agent` harness. The
  single-threaded core must stay correct and unburdened with the feature off.
- Keep `qjs-cli` thin; library crates own engine semantics and error models.
- Split large test files by the behavior under test (descriptors,
  enumeration, prototype operations, ...), not by the feature that added them.
- Performance changes need evidence: a benchmark or a measured case.

## Dependency Policy

- Prefer the standard library; check existing workspace crates or a small
  local helper before adding anything.
- A new dependency must state why it is needed, what uses it, and whether it
  affects runtime, dev-only tooling, or tests.

## Documentation Sync

- Docs are part of the change: when behavior, setup, commands, APIs,
  supported syntax, or conformance expectations change, update `README.md`,
  `docs/`, task files, allowlists, or expected-failure notes in the same
  reviewable unit.
- Audience boundaries: `README.md` human overview, `AGENTS.md` agent
  contract, mechanics under `docs/`.
- If a stale doc is outside the task boundary, name the file and topic in the
  final response instead of fixing it silently.

## Commit Discipline

- One commit per reviewable unit: one feature, fix, refactor, dependency
  update, or documentation/policy update. No unrelated formatting or cleanup
  mixed in; never stage user or unrelated workspace changes.
- Before committing, run `./scripts/check-touched.sh --staged --explain`
  unless the installed pre-commit hook has already run it. This is the
  AI-friendly fast gate: it selects crate tests and focused Test262 slices from
  staged paths. It does not replace `./scripts/check.sh` for final handoff or
  push.
- For gap work, one recommendation-queue area or one coherent semantic family
  is the commit boundary. Verify the area with
  `./scripts/find-qjsng-gaps.sh --filter <area> --all` before implementation
  and again before committing. No one-commit-per-Test262-case; allowlist and
  expected-failure updates ride with the change that makes them meaningful.
- Commit messages describe the behavior or policy change, for example
  `Add lexer support for comments`.
- Push promptly after each locally verified commit so remote CI starts as
  early as possible; do not batch finished commits locally.

## Parallel Agent Workflow

Use isolated worktrees only when ownership boundaries are clear; serialize on
one branch when a task touches shared AST types, workspace configuration,
global error models, or broad architecture docs. Full runbook:
`docs/harness.md`.

- `main` is the stable integration branch; one short-lived
  `agent/<task-slug>/<owner-id>` branch and worktree per coding owner, all
  from the same recorded base sha.
- Every owner gets a path boundary before editing; global files
  (`Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml`, `.gitmodules`,
  `AGENTS.md`, `README.md`, `docs/`, shared CI/bootstrap scripts) default to
  main-agent ownership.
- Owners never merge each other's branches. The main agent validates scope
  with `./scripts/validate-agent-branch.sh`, integrates one branch at a time,
  and runs `./scripts/check.sh` plus `./scripts/compare-qjs.sh` (for any
  merge touching `crates/qjs-runtime`) after each integration before
  pushing.
- Pushed `agent/**` branches get CI; a red or unexplained latest run blocks
  integration, but green remote CI never replaces local checks.
- Remove merged worktrees and branches unless retained for diagnosis.

## Architecture Expectations

- `qjs-ast`: shared syntax and span types; depends on no other engine crate.
- `qjs-lexer`: tokenization, preserving byte spans.
- `qjs-parser`: syntactic structure; never evaluates code.
- `qjs-runtime`: evaluation semantics; re-parses only through public parser
  APIs. Builtins stay grouped by object and behavior family
  (`array/iteration`, `object/descriptor`, ...).
- `qjs-cli`: thin smoke-test wrapper.

## Test Strategy

- Every behavior change gets a focused test at the lowest useful layer; unit
  tests live next to the crate behavior they exercise.
- Use QuickJS-NG comparisons (`tests/fixtures/compare-qjs/`) when semantics
  are unclear.
- Use Test262 through curated subsets, not full-suite failure counts.
  Derived cases under `tests/test262/cases/` must start with
  `// Derived from: <official Test262 path>`; list them (or directly runnable
  upstream `test/` paths) in `tests/test262/allowlist.txt`, and run
  `./scripts/test262-subset.sh` after editing allowlists or expected
  failures. Expected failures always carry a written reason.
- Record a burndown entry (`./scripts/test262-burndown.sh`) after every
  complete `--exact --all` scan; prefer the CI `test262-burndown` artifact
  for per-commit numbers. The trend in `docs/conformance/burndown.jsonl`
  decides when recommendation strategy or campaign priorities change. Never
  record partial scans.

## Definition of Done

For code changes:

1. The relevant crate has unit or integration coverage.
2. `./scripts/check-touched.sh --staged --explain` passes before commit, and
   `./scripts/check.sh` passes before final handoff or push, or any failure is
   explained with exact output.
3. Docs and crate metadata are updated when behavior, commands, APIs, or
   conformance expectations change.
4. New dependencies, public APIs, or architecture shifts are justified.
5. The final response names changed files and verification performed.

For documentation-only changes: single clear audience, no README/AGENTS
duplication, and `./scripts/check.sh` when scripts, Cargo files, or examples
changed.

## Suggested Autonomous Loop

1. Pick one task from `tasks/`. For gap work, take quick wins from the
   `find-qjsng-gaps.sh` recommendation queue while they exist; when the queue
   is dominated by hard-hinted broad areas, switch to the next unchecked
   slice of the highest-priority campaign task in `tasks/README.md` instead
   of re-running global probes.
2. Read the related crate, `docs/architecture.md`, and `docs/harness.md`.
3. Implement the smallest useful slice, with tests.
4. Run `./scripts/check-touched.sh --staged --explain` before committing; for
   runtime, parser, or lexer semantics, include the focused Test262 slices it
   selects or explain why no slice matched.
5. Run `./scripts/check.sh` before final handoff or push.
6. Summarize behavior, risks, verification, and the next useful task.

When an LSP tool is available, prefer it over text search for semantic
navigation: `findReferences`, `goToDefinition`, and call-hierarchy give
exact, scope-aware results across crates (e.g. disambiguating same-named
symbols like `Value`, or tracing a VM call path) where `grep` returns
noise from comments, strings, and unrelated matches. Fall back to
`grep`/`glob` for plain-text searches or when no LSP server is configured.

If requirements are ambiguous, prefer a small reversible implementation and
state the assumption. Ask for user input only when the ambiguity changes
public architecture, dependency choices, or long-term compatibility.
