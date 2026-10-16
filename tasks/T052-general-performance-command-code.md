# T052: General performance work with Command Code

- Status: active.
- Implementation: none in production; Command Code owns isolated layout-tool
  work, host owns integration, profiles, measurement and future unit plans.
- Verified at: starting main 9a0eeeca; earlier T050/T051 diagnostics and gates
  are archived and do not promote a runtime optimization.
- Evidence: starting revision and execution contracts below; current baseline
  job 20261016T122024Z-general-current-baseline is pending.
- Unresolved: layout-tool repair and a newly profiled general runtime unit.
- Next action: review Command Code's diff, collect fresh same-host standing and
  exact-binary profiles, then freeze one unit before runtime implementation.

## Scope and ownership

Workspace root: /Users/qingyingliu/Code/co-work-space. Base 9a0eeeca.
Lead: worktrees/quickjs-rust/general-performance-lead, branch
agent/general-performance/lead. Command Code:
worktrees/quickjs-rust/general-performance-command-code, branch
agent/general-performance/command-code. Other existing worktrees are excluded.

Command Code may change tools/benchmark/layout_pin.py, narrowly scoped layout
helpers/tests, and order_file.py only if necessary for an existing caller.
It must not commit/push, edit runtime/build/order files, or dispatch measurements.
The host reviews command logs and changes, runs iMac checks, and integrates.

## Generalization contract

The user requests sustained GENERAL optimization, explicitly rejecting ranking
tricks. Preserve the T018 external contract and T022 evidence/attempt budget:

- Admit programs by operations, types and data flow, not source templates,
  benchmark identity, paths, iteration counts or expected checksums.
- Do not remove cases, increase timeouts to conceal regressions, select favorable
  pairs, tune thresholds after seeing results, or count an internal-only win.
- Keep all broad, sentinel and pinned external cases as regression evidence.
  Add semantic holdouts when a mechanism introduces assumptions.
- Diagnose with exact-binary profiles and feature-only counters; measure only
  on iMac through the workspace queue, wall time decides on this host.
- Layout work preserves measured baseline placements and audits matching; no
  benchmark-driven offset search. Any adopted layout needs whole-portfolio
  validation. Tool repairs alone are not a runtime speedup claim.
- Every runtime unit gets a newly frozen validated plan before implementation,
  correctness checks, complete external comparison, and an explicit outcome.
  T049 and previous rejected units keep their exhausted historical budgets.
- Investigate structural ceilings when evidence supports them; avoid an endless
  series of narrow hot-path exceptions. Correctness and protected shared-cell
  binding semantics remain mandatory.

## Verification

The host checks Command Code's actual diff and log, not its summary alone.
Run focused tooling tests, change-aware gate and full gate for the repair.
Runtime units additionally require semantic coverage, NG comparisons and
Test262 verification as specified by their frozen plan and T018/T022.

## Frozen layout-maintenance diagnostic

Before adopting a new production order, try exactly one natural-order replay:
list the exact baseline's unique named functions in actual __text address
order, excluding __mh_execute_header and symbols outside that section. Build
with the identical canonical recipe in a clean temporary worktree. Adoption
requires FULL executable SHA-256 equality to the measured baseline. If it
changes any byte, retain the diagnostic and do not tune/retry or adopt it.
No function offset is searched or selected from timing. This proves neutrality
of this replay, not invariance under future function-size/toolchain changes.
The new symbol audit must expose a future stale or ambiguous layout.
