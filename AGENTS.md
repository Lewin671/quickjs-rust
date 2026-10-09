# Agent Instructions

Authoritative rules for autonomous agents (Claude Code, Codex, or similar;
`CLAUDE.md` is a symlink to this file). This file says what the rules are and
where to start. Mechanics live in the documents it links; do not copy them
here.

## Prime Directive

Build a Rust-native JavaScript engine toward two converging goals: **100%
Test262 conformance** and **production-grade performance**. The language
target is the latest ratified standard, ECMA-262 16th edition (ES2025),
anchored to `tc39/ecma262@es2025`. Test262 has no edition tag; the pinned
`third_party/test262` commit is the executable coverage, and newer
living-draft or Stage 3+ cases are future work unless a task opts into them.

Work incrementally, preserve subsystem boundaries, and make each change
verifiable with focused tests. Do not cap the design at "small and safe": when
a correctness or performance ceiling is structural, lift the structure instead
of adding another heuristic around it. Do not add code that recognizes one
benchmark's shape; an optimization must admit programs by what they do
(operations, types, data flow), not by matching a source or bytecode template.

The **environment / binding model** (slot-indexed locals plus shared upvalue
cells) is a protected architectural boundary: never reintroduce a per-call
name-keyed snapshot. Its invariants are in
[docs/design/env-model-rewrite.md](docs/design/env-model-rewrite.md).

## Where To Start

A session starts with no memory. Read only what the work needs:

| Work | Read first | Then |
| --- | --- | --- |
| Any task | [tasks/README.md](tasks/README.md) | the task file it routes to; its first lines say what is next |
| Conformance | [docs/harness.md](docs/harness.md) | `./scripts/find-qjsng-gaps.sh` |
| Performance | [docs/performance-knowledge.md](docs/performance-knowledge.md) | [docs/performance-workflow.md](docs/performance-workflow.md) |
| Engine internals | [docs/architecture.md](docs/architecture.md) | the module header of the file you will change |
| A script's purpose | [scripts/README.md](scripts/README.md) | the script's `--help` |

Selecting work. For conformance, take quick wins from the
`find-qjsng-gaps.sh` recommendation queue while they exist; when it is empty
or dominated by hard-hinted broad areas, take the next action of the
highest-priority active task instead of re-running global probes. For
performance, priority comes from current measured evidence, never from an old
task description: the opportunity queue is generated from a fresh comparison
and is not stored in the repository, a unit's plan is frozen and validated
before runtime code is written, and the plan's attempt budget decides when it
closes. For structural changes, define the target architecture and migration
stages before coding.

`tasks/archive/` is history: what was tried and observed at a named revision.
Search it before repeating an experiment
(`grep -rn -i "<mechanism>" tasks/`); do not treat it as current guidance.

## Standard Commands

- Fresh checkout setup: `./scripts/bootstrap.sh`
- Fast gate before a commit: `./scripts/check-touched.sh --staged --explain`
  (or `--base <ref> --explain` for a branch)
- Full gate before handoff or push: `./scripts/check.sh`
- CLI smoke test: `cargo run -p qjs-cli -- -e "1 + 2;"`
- QuickJS-NG comparison: `./scripts/compare-qjs.sh`
- Gap discovery: `./scripts/find-qjsng-gaps.sh [--all] [--filter test/<prefix>]`
- Test262: `./scripts/test262-subset.sh`, `./scripts/test262-baseline.sh`,
  `./scripts/test262-burndown.sh --report <dir> | --entry <file>`
- Performance: `./scripts/perf-loop.sh --plan tasks/performance-units/<unit>.json`
  to iterate, `./scripts/perf-compare.sh --base <sha> --output-dir <dir>` for
  a formal comparison
- Context check: `./scripts/check-context.py`
- Agent worktree: `./scripts/create-agent-worktree.sh <task-slug> <owner-id> [base-ref]`
- Branch scope check: `./scripts/validate-agent-branch.sh <branch> <base-sha> <path>...`
- Branch CI: `gh run list --branch <branch> --limit 1`, then
  `gh run watch <run-id> --exit-status`

Bootstrap installs git hooks: pre-commit runs the fast gate and pre-push runs
the full gate. `bootstrap.sh` and `check.sh` fall back to
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
  runtime work uses existing AST types instead of parser shortcuts; lexer
  tokens carry spans.
- `unsafe` is permitted only as an audited, localized optimization (NaN-boxed
  or tagged values, inline caches, arena/GC internals). The workspace lint is
  `unsafe_code = "deny"`, overridden per module with an explicit
  `#[allow(unsafe_code)]` and a `// SAFETY:` comment on every `unsafe` block;
  a sound safe equivalent must not exist or must be a measured bottleneck.
  Never reach for `unsafe` for convenience. No global mutable state; make
  shared ownership and lifetime explicit in runtime data structures.
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
- No FFI and no Rust `async`: JS async is implemented by the engine's own
  event loop / VM, never by Rust's async runtime.
- A tracing GC or arena allocator is allowed and expected to replace
  `Rc`/`RefCell` for runtime values, to collect reference cycles and to make
  allocation cheap. Treat it as a deliberate, staged subsystem.
- OS threads are allowed only behind a cargo feature and only to back
  `SharedArrayBuffer` / `Atomics` and the Test262 `$262.agent` harness. The
  single-threaded core must stay correct and unburdened with the feature off.
- Keep `qjs-cli` thin; library crates own engine semantics and error models.
- Performance changes need evidence: a benchmark or a measured case.

## Dependency Policy

- Prefer the standard library; check existing workspace crates or a small
  local helper before adding anything.
- A new dependency must state why it is needed, what uses it, and whether it
  affects runtime, dev-only tooling, or tests.

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
- Name and split tests by the behavior they prove (descriptors, enumeration,
  prototype operations, ...), not by the feature or mechanism that added them.
- Use QuickJS-NG comparisons (`tests/fixtures/compare-qjs/`) when semantics
  are unclear.
- Use Test262 through curated subsets, not full-suite failure counts. Derived
  cases under `tests/test262/cases/` start with
  `// Derived from: <official Test262 path>`; list them (or directly runnable
  upstream `test/` paths) in `tests/test262/allowlist.txt`, and run
  `./scripts/test262-subset.sh` after editing allowlists or expected
  failures. Expected failures always carry a written reason.
- Record a burndown entry after every complete `--exact --all` scan; never
  record partial scans. Prefer the CI `test262-burndown` artifact for
  per-commit numbers. The trend in `docs/conformance/burndown.jsonl` decides
  when recommendation strategy or campaign priorities change; what a record
  establishes is in [docs/conformance/README.md](docs/conformance/README.md).

## Context Rules

The repository is the only memory a later session has. Keep it true and
small.

- Every fact has one home. Instructions live here and in the runbooks;
  what exists now lives in `docs/architecture.md`, `docs/design/`, and module
  headers; what to do next lives in the active task file; what was tried
  lives in `tasks/archive/`. Link to the home instead of restating it.
  `README.md` is the human overview and carries no agent procedure or task
  status.
- Documentation is part of the change. When behavior, commands, APIs, or the
  shape of a subsystem change, update the document that owns that fact in the
  same commit, and replace the stale statement rather than appending a
  correction under it.
- An active task opens with the resume block from
  [tasks/TEMPLATE.md](tasks/TEMPLATE.md). Update it before ending a session:
  status, whether the implementation is present, the revision and checks that
  verified it, where the evidence is, what is unresolved, and the next action.
  When a task closes, move it to `tasks/archive/` and update
  `tasks/README.md`.
- State what was verified and at which revision. "Checked at commit X" is not
  a claim about HEAD, and a rejected experiment is not a claim that its code
  is absent.
- Evidence a later session cannot open (`target/`, `/tmp`, private notes) does
  not count as recorded. Put the conclusion, the numbers, and the revision in
  the task file.
- Frozen evidence is never edited: committed `tasks/performance-units/*.json`
  plans, `docs/conformance/burndown.jsonl` records, and archived logs.
- A module header says what the module owns and the invariant that keeps it
  correct. Dated measurements and priorities belong in task files.
- `./scripts/check-context.py` enforces the mechanical part (document
  budgets, links, resume blocks, indexes). If a stale document is outside the
  task boundary, name the file and topic in the final response.

## Commit Discipline

- One commit per reviewable unit; no unrelated formatting or cleanup mixed
  in; never stage user or unrelated workspace changes.
- Run `./scripts/check-touched.sh --staged --explain` before committing unless
  the pre-commit hook has already run it. It does not replace
  `./scripts/check.sh` for final handoff or push.
- For gap work, one recommendation-queue area or one coherent semantic family
  is the commit boundary. Verify the area with
  `./scripts/find-qjsng-gaps.sh --filter <area> --all` before implementation
  and again before committing. No one-commit-per-Test262-case; allowlist and
  expected-failure updates ride with the change that makes them meaningful.
- Commit messages describe the behavior or policy change, for example
  `Add lexer support for comments`.
- Push promptly after each locally verified commit so remote CI starts early;
  do not batch finished commits locally.

## Parallel Agent Workflow

Use isolated worktrees only when ownership boundaries are clear; serialize on
one branch when a task touches shared AST types, workspace configuration,
global error models, or broad architecture docs. The runbook is in
[docs/harness.md](docs/harness.md).

- `main` is the stable integration branch; one short-lived
  `agent/<task-slug>/<owner-id>` branch and worktree per coding owner, all
  from the same recorded base sha.
- Every owner gets a path boundary before editing. Global files (`Cargo.toml`,
  `Cargo.lock`, `rust-toolchain.toml`, `.gitmodules`, `AGENTS.md`,
  `README.md`, `docs/`, shared CI/bootstrap scripts) default to main-agent
  ownership.
- Owners never merge each other's branches. The main agent validates scope
  with `./scripts/validate-agent-branch.sh`, integrates one branch at a time,
  and runs `./scripts/check.sh` plus `./scripts/compare-qjs.sh` (for any merge
  touching `crates/qjs-runtime`) after each integration before pushing.
- A red or unexplained CI run on an `agent/**` branch blocks integration;
  green remote CI never replaces local checks.
- Owners may build and test in parallel, but performance timing on one host
  is serialized through the main agent; parallel load corrupts every timing.
- Remove merged worktrees and branches unless retained for diagnosis.

## Definition of Done

1. The relevant crate has unit or integration coverage.
2. The fast gate passed before commit and the full gate before handoff or
   push, or any failure is reported with exact output. For runtime, parser,
   or lexer semantics the focused Test262 slices the fast gate selects ran,
   or the response explains why none matched.
3. The documents that own the changed facts are updated, including the
   task's resume block.
4. New dependencies, public APIs, or architecture shifts are justified.
5. The final response names changed files, verification performed, risks,
   and the next useful task.

For documentation-only changes: one clear audience, no duplication across
documents, and `./scripts/check.sh` when scripts, Cargo files, or examples
changed.

When an LSP tool is available, prefer it over text search for semantic
navigation (`findReferences`, `goToDefinition`, call hierarchy); fall back to
`grep`/`glob` for plain-text searches.

If requirements are ambiguous, prefer a small reversible implementation and
state the assumption. Ask for user input only when the ambiguity changes
public architecture, dependency choices, or long-term compatibility.
