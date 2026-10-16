# T055: Layout noise control — B qualification blocked

- Status: blocked; tools implemented, rule adoption incomplete.
- Implementation: present; versioned host policy v1 is unqualified and cannot
  be used for a new promotion. No runtime engine change.
- Verified at: 3a66cda9, 2026-10-16; iMac full gate passes (166 s), 523 benchmark
  tests. Base-main fast/context checks pass. Fresh six-variant build/audit,
  5170 subset and 248 NG fixtures pass.
- Evidence: [B frozen plans and log](archive/T055-phase-b-plan-and-log.md),
  [A2 data](archive/T055-phase-a2-data.md),
  [unqualified policy v1](../benchmarks/policies/imac-x86_64-rust1.95-fat16-layout-v1.json).
- Unresolved: three consecutive complete timing runs were DISTURBED; no valid
  B A/A or held-out budgets. No remaining timing jobs from this task.
- Next action: lead chooses a controlled quiet iMac window for the unchanged
  plans, or separately authorizes/freezes/qualifies a new sampling schedule.
  Resume timing only after that decision; publish a new qualified artifact version.

## Scope and boundary

The lead's phase B brief authorizes the layout-robust tool implementation and
calibration, and explicitly says to stop and report when a lead decision is
needed. The repeated shared-host interference reached that boundary. The task
is not closed/archived as adopted, and no valid failure was favorably rerun.

The rule lives in [T022](T022-performance-priority-controller.md#layout-robust-decisions),
procedure in [the workflow](../docs/performance-workflow.md#screen-gate),
and schemas in [artifact contracts](../docs/layout-policy-artifacts.md).
The linked B log owns frozen plans, job identities, exact binaries, structural
audit correction and gate results. Invalid observations remain retained but
ineligible; held-out remains acceptance-only and its budgets remain unknown.

## Resume requirements

Keep the source/host/recipe identities and total measurement/attempt budget
frozen before timing. Retain/replace invalid or DISTURBED runs whole, never
pool observations. All formal precision/correctness/identity gates still apply.
The first policy is intentionally unqualified; do not edit it into acceptance.
T053/T054 rejected standings and exhausted budgets are unchanged.
