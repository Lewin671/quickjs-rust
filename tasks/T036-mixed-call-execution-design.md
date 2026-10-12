# T036: Mixed scalar/boxed execution across ordinary calls

- Status: active. Unit 2 design, awaiting the lead's experiment decision.
- Implementation: none. Documents and a not-yet-frozen schema-2 draft only;
  no runtime edits, dependency changes, builds or performance candidate.
- Verified at: `e54e33eb` runtime baseline; design branch based on it,
  2026-10-12. Standing commit `86ae3603` passed check-context and staged
  check-touched. Draft check-unit and validate-unit passed against brief 01.
- Evidence: [measurements](archive/T036-mixed-call-execution-measurements.md)
  retain component costs, formulas, commands and limits. Brief 01 hashes
  and full standing are in [T018](T018-broad-performance.md#standing-at-e54e33eb).
- Unresolved: corpus-wide eligible scalar-transport coverage is unpriced;
  the draft is not a frozen performance unit. No migration payoff verified.
- Next action: lead approves or declines the bounded pre-build experiments;
  an implementation owner first finishes coverage/kill gates and freezes a
  plan, serializing execution-path work after the other owners' integrations.

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
- Forbidden paths: runtime, AST/parser, tooling, benchmark source, CI,
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
a uniform five-target 5% payoff gate. This requires the lead's scope decision.
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
- [ ] Freeze a plan only after the lead's decision and corpus coverage probe.
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

Docs-only staged gate explicitly skips Rust and Test262. The user prohibited
builds beyond measurement needs, so `check.sh`, cargo, focused Test262 and
compare-qjs were not run; this handoff establishes no new runtime correctness
claim. The two timing binaries and diagnostic binary were reused, not built.
T033's resume still cites d1313d52 as its latest whole-tree standing; its
historical unit rejection remains valid, but that standing pointer is stale.
It was left outside this unit's standing update to avoid mixing task ownership.
