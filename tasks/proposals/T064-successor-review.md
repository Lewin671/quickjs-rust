# T064/T056 successor: lead review notes (draft)

The [schema-3 proposal](T064-kernel-layout-authority.json) is validated, but is
not a frozen performance unit or implementation permission. Do not pass it to
`perf-loop.sh`. Its `mechanism` carries the complete stage/static/ledger
contract; its typed fields bind the queue, receipts, targets and measurement
budget. The [input record](../archive/T064-successor-plan-inputs.md) owns the
measurements, hashes, target rationale and validator result.

Authority: [T064 lead decision](../T064-kernel-isolation.md#lead-decision-2026-10-21),
[transition and ledger](../T064-kernel-isolation.md#frozen-plan-transition-and-attempt-ledger-proposal-for-review),
[property-layout design](../../docs/design/property-layouts.md) and
[T056 work-count obligations](../T056-interpreter-core-v2.md#new-shape-work-count-obligations-perf-counters-before-any-timed-screen).
The [old-plan closure draft](T064-layout-edges-closure.md) takes effect only
when the approved successor is committed under `tasks/performance-units/`.

## Decisions before freezing

- Approve the proposed structural-prerequisite priority override. K and A
  request intermediate `advance`; only B can earn final retention. The plan
  includes K cost in every cumulative comparison against M.
- Review the conservative instruction-blind classification, memory targets
  and 60-block formal budget in the JSON: A changes representation/retention,
  so instruction counts alone cannot price its costs.
- Accept the explicit manual ledger checks. Schema 3 encodes one aggregate
  failure budget, not separate K and shared A+B accounts. `validate-unit`
  cannot enforce their stricter limits or the stop-and-redirect rule.
- Accept the stricter manual stage-band audit. On diagnostic-only cases the
  existing stage evaluator substitutes the stage cap for the case band;
  the proposed contract requires the stricter of both. Its formal diagnostic
  branch likewise needs that additional audit before advancing.
- Resolve whether a quantitative resource ceiling is required before freezing.
  Existing resource replay verifies identity/completeness and retains costs;
  it does not define a separate RSS neutrality threshold. A complete resource
  report must not be described as proof of memory neutrality.
- Integrate/review `scripts/check-kernel-isolation.sh` from
  `agent/t064-k0-checker/codex` and the separate T056 allocation/startup inputs.
  These M samples certify neither checker qualification nor a passing K.

## Stage handoff

Use the JSON's K→A→B architecture and semantic invariants; do not automatically
port the old diagnostic layout experiment. Every acceptance candidate is M
plus the reviewed K/M1 runtime changes, with tooling/docs merges allowed.
Record the runtime scope diff before each screen. Unrelated runtime changes
or changed priority/scope require lead review rather than a silent new base.

After K integrates at I, obtain exact I profiles to confirm the selected M1
mechanism still has reach. Those profiles cannot retarget the frozen plan or
replenish its accounts. I→A attribution remains supplemental. Advance only
`migration.current_stage` after a recorded stage decision; all acceptance
comparisons keep M fixed, including K cost.

Before each screen explicitly check the carried failure identity, per-stage
reserve, static/correctness results and exact-image instruction pre-gate. A
successful schema validation is not evidence that those obligations passed.
Incomplete evidence follows the existing completion/load/precision rules;
it creates no new allowance. No timing screen was submitted by this proposal.

If approved, freeze the JSON and enact the closure together, update the plan
index and T056/T064 resume blocks, then authorize K1. Until then the old JSON,
valid failed screen and shared A+B reserve remain intact; this draft spends
no acceptance attempt and makes no runtime performance claim.
