# T036: Mixed scalar/boxed execution across ordinary calls

- Status: active. Four-stage migration NOT approved; corpus pricing and
  instruction attribution approved, followed only conditionally by standalone ABI replay.
- Implementation: none. Documents and a not-yet-frozen schema-2 draft only;
  no production runtime edits or performance candidate. Temporary feature-gated
  diagnostic instrumentation/builds are authorized and will not be committed.
- Verified at: production `e54e33eb`, diagnostic follow-up 2026-10-12:
  five SHA-matched corpus outputs/counters, nonempty instruction pricing,
  disassembly/timer profiling and conditional standalone storage replay.
  Earlier design commits `86ae3603`/`d3662651` passed branch CI.
- Evidence: [measurements](archive/T036-mixed-call-execution-measurements.md)
  retain component costs, formulas, commands and limits. Brief 01 hashes
  and full standing are in [T018](T018-broad-performance.md#standing-at-e54e33eb).
- Unresolved: exclusive per-function instruction attribution and a closed
  cycle partition; no production savings or parity combination verified.
- Next action: lead selects a narrow leaf pricing/implementation unit from
  the ranked list in the measurement record. The replay is not approval
  to integrate or freeze the four-stage migration.

## Goal

A decision-ready design for Unit 2: remove repeated scalar boxing and wide
value transport across ordinary calls, keeping the protected slot/cell model
and exact semantic exits. The target architecture, historical exclusions,
landable stages and pre-build falsification gates are in
[the design](../docs/design/mixed-call-execution.md). This task owns measured
state; it does not change which optimizations execute at HEAD.

## Scope

- Allowed paths: `docs/`, `tasks/`, excluding other owners' performance-unit
  plans and index rows. Measurement artifacts under this worktree's `target/`
  are untracked and have their reproducer retained in the evidence record.
- Temporary diagnostic scope: feature-gated counters/profiling edits in this
  worktree only, restored before commit; builds and measurements use timing lock.
- Forbidden committed paths: runtime, AST/parser, tooling, benchmark source, CI,
  third_party, main working tree, and the other owners' worktrees.
- Owner boundary: design only. Future mixed-call work shares executors with
  global-sink work; no simultaneous implementation on those paths.
- Base sha: `e54e33eb9371c5cb4627bd8e590a648085a7fa26`.
- Branch: `agent/mixed-call-execution-design/codex`.
- Worktree: `../quickjs-rust-worktrees/wt-quickjs-rust-mixed-call-execution-design-codex`.

## Draft and decision contract

[mixed-call-execution-plan-draft.json](../docs/design/mixed-call-execution-plan-draft.json)
is schema 2, unit_kind migration, four stages, common cumulative regression
ceiling 1.03, four final targets <=0.95 cycles/base, controls <=1.03, max_attempts 1.
The draft proposes validate-input as a mandatory control (all five remain
cumulative stage targets): its measured 2-4% saving scenario cannot justify
a uniform five-target 5% payoff gate. The lead accepted validate as a mandatory control but did NOT approve the
four-stage program. The linked draft is historical, not authorization to build.
It is **NOT FROZEN**, intentionally not in `tasks/performance-units/` and not
listed in that index. No rule/schema modification is required: the design's
stricter first-stage 1.02 budget is an extra screen precondition; the schema
continues to evaluate every stage against its common 1.03 ceiling.
Each stage gets one implementation screen attempt; the final payoff gets one.
The current validator does not count stages' attempts; record each in this
resume/task before proceeding. Attempts are not consumed by design-only
micro measurements of the unchanged base. Do not change thresholds to rescue
a failed implementation. Formal comparisons/decisions are scheduled by the lead.

The draft's lower priority is an explicit override: global numeric sinks
(rank 1) and class-field installation have separate implementation owners;
tofte needs another mechanism. Rebase/reprofile/refreeze if integration
changes this base before implementation; never treat old receipts as current.

## Acceptance

- [x] Record e54e33eb standing in a separate commit; update T018/T022 resume.
- [x] Read T021/T024/T027/T030/T033, every T033 Rejected entry, compact-fn
  module headers, and the protected environment-model design.
- [x] Specify activation/continuation ABI, lane representation, ownership,
  shared-cell authority, exact exits and independent staged kill gates.
- [x] Price representative nonempty 4/8 kernels vs NG, record uncertainty,
  removed-component budgets and per-target payoff/parity limits.
- [x] Complete ordered corpus pricing and source/disassembly attribution;
  record the nonexclusive cycle model and unresolved instruction residuals.
- [x] Conditional standalone storage replay; no runtime integration.
- [ ] Freeze any unit only after the lead selects it from the revised ranking.
- [ ] Implement, screen and formally decide any stage (future owner).

## Verification

```sh
./scripts/check-context.py
./scripts/check-touched.sh --staged --explain
./scripts/performance-decision.sh check-unit \
  --unit docs/design/mixed-call-execution-plan-draft.json
./scripts/performance-decision.sh validate-unit \
  --unit docs/design/mixed-call-execution-plan-draft.json \
  --queue <main>/target/comparison/brief01-e54e33eb-20261011-queue.json \
  --profile-root <main>/target/profiles/brief01-e54e33eb
```

Follow-up builds were diagnostic-only, with all measurements serialized by
`lockf /tmp/qjs-lead/host-timing.lock`. No formal perf-compare, runtime candidate,
full gate, new Test262 or NG correctness gate was run. Temporary runtime
instrumentation was restored byte-for-byte to e54e33eb before handoff;
`git diff --exit-code e54e33eb -- crates` proves no runtime changes remain.
Docs-only staged gate skips Rust/Test262; the record preserves experiment
limits and reproducible sources. No plan was frozen or stage integrated.
T033's resume still cites d1313d52 as its latest whole-tree standing; its
historical unit rejection remains valid, but that standing pointer is stale.
It was left outside this unit's standing update to avoid mixing task ownership.
