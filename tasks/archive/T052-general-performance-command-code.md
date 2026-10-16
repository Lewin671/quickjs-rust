# T052: General performance work with Command Code

- Status: closed-landed (maintenance only; no runtime speedup claim).
- Implementation: maintenance patch present on lead; no runtime optimization
  retained. Command Code contributed structural matching and tests; host
  corrected audit, slot, alias and fixture issues and independently verified.
- Verified at: transport 4e43ae7f, 2026-10-16; iMac change-aware and full
  checks pass, with 469 tool tests and 5170 curated Test262 cases.
- Evidence: starting revision and execution contracts below; current baseline
  20261016T122024Z-general-current-baseline and exact profiles are complete.
- Unresolved: none for this repair; T054 separately owns preservation under
  future source edits and has no retained build change yet.
- Next action: follow [T054](T054-execution-layout-regions.md); no rejected
  runtime budget is reopened by this maintenance result.

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

## Current measured evidence and collaboration

- 20261016T122024Z-general-current-baseline: full 76 cases x 30 blocks, workers
  four, wall metric, no imprecise cases, issues or DISTURBED warning; ready
  for unit gates. Exact base/candidate executable hash remains 926ee634ca3901b89c778478702a4fe993288ef7e8e69cd93c238c694595aa85.
- 20261016T122336Z-general-fresh-top8-profiles and
  20261016T123302Z-general-fresh-route-counts are complete. The host froze
  and validated T053 separately before writing any of its runtime changes.
- 20261016T123733Z-general-natural-order-replay passed the single predeclared
  SHA equality check: 4205 text functions, complete executable SHA equal to
  the base above. The production x86 order is replaced by exactly this replay;
  there was no timing-guided search and no speedup is claimed for the replay.
- Host fixed layout-scan.sh to use the canonical performance builder, with two
  simulated integration tests covering binary paths, relink failure and file
  restoration; both pass and the old helper fails that build contract.
- Command Code session 2dc5c139-04e4-4eed-be5e-4c1a18c2b097 uses the CLI's
  configured deepseek/deepseek-v4.1-flash. Host interrupted its unfinished
  first pass to correct audit-by-canonical-name and pin-head omissions and
  resumed the same session with concrete tests. No first-pass result is
  treated as verified. The second pass and host review remain pending.

## Host review outcome

The first Command Code pass was interrupted without a usable result. Its next
80-turn and scoped 30-turn runs both hit their limits, returning partial
results (warnings relayed). The host did not treat those summaries as success:
production fixes and a fresh 66-test run now pass. Default model was
 deepseek/deepseek-v4.1-flash; no model or effort was changed.

The retained tooling distinguishes semantic selection from exact raw linker
currency; audits complete heads and budgets; rejects ambiguous required bodies
and unsafe alias extents; includes the last text extent and excludes the Mach
header; uses exact multi-page slot padding; and offers preserve-offset mode.
Static audit rejects the old file (609 stale symbol/budget entries) and accepts
the 4205-name natural replay with no strict problems. The old "541 runtime"
count includes names mentioning the runtime in instantiating-crate metadata;
the decoder's path/type classification labels 521 as engine-related, but
strict audit rejects every missing raw entry regardless of that label.

Host also split policy, extents and symbol decoding into reviewable modules,
retained the reference corpus license, preserved the interpreter's offset in
the scan helper, and corrected optional-interpreter preservation. Addresses
are reported observations: the auditor does not claim future source edits
will keep every function fixed.

T053's sole screen reduced hash-map time by 8.3% and instructions by 7.2%, but
six controls regressed beyond 3%; its runtime is reverted. Restoration rebuilt
exact baseline 926ee634 and passed full checks. No formal or promotion was
spent on that rejected unit. Its complete record is integrated separately.

## Combined verification

The first combined gate (20261016T141428Z-general-layout-maintenance-gates)
failed only three old scan mocks that still expected a raw cargo command.
After adapting their fake canonical builder and interpreter-offset query,
all four scan integration checks pass without weakening the failure/restore
conditions. 20261016T142224Z-general-layout-maintenance-gates-fixed passes
fast and full gates, all 469 benchmark-tool tests and 5170 Test262 cases.
Strict audit of the actual production baseline plus replayed order passes.
No Rust runtime, parser, AST, Cargo recipe, workload or decision threshold
changed; canonical binary equality was established by the separate replay.
