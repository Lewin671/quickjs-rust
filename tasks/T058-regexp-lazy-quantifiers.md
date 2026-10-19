# T058: Current-base RegExp lazy-quantifier proposal under v6

- Status: active; old attempts 1/2 closed incomplete/inconclusive; fresh current-base proposal, not a third retry.
- Implementation: present on rebased candidate `9e415ff4`; its runtime diff from current main is T058 alone. No new runtime edits.
- Verified at: clean rebase onto `a68a9f46` at `9e415ff4`; attempt-2 hashes/logs verified on 2026-10-19. Full and staged gates recorded before handoff.
- Evidence: [attempt 1](archive/T058-attempt1-incomplete.md), [attempt 2](archive/T058-attempt2-inconclusive.md), [prospective contract](proposals/T058-current-base-v6.json).
- Unresolved: genuinely fresh queue/profile, schema-3 materialization/commit and all current-base acceptance are pending the chained jobs. Neither old decision is changed.
- Next action: fetch the new freeze/decision jobs below; import the frozen-plan bundle, verify its bindings and read acceptance at the new base/candidate only.

## Goal and attribution

Keep lazy single-code-point RegExp repetition incremental and remove the
literal-source replace/Date shortcuts; the original implementation, semantic
risks and correctness history live in the attempt-2 archive. RGI_Emoji
property-of-strings support remains separate work.

[T062](archive/T062-formal-precision-rule.md#prospective-scope-and-t058)
allows a fresh proposal only with a new current-base comparison, opportunity
queue, exact-image profile and newly validated/frozen plan. Existing plans
remain immutable and both old outcomes remain inconclusive. This proposal
uses current main `a68a9f469a1c36c8080b71e1546851a30f448da6` as its base and
rebased `9e415ff45cb909e41d182dc3d6f03d3c1007b3f9` as candidate. Both include
T057 and T062; only T058's RegExp implementation/tests differ in engine
paths. No improvement from T057 is credited to T058. No old timing, profile,
queue or exact-candidate correctness entry can verify this source pair.

## Prospective choices

The committed contract is a proposal, not a schema-3 unit with fabricated
future hashes. A chained job fills only measured queue/profile bindings,
records actual sampled branch counts, validates the schema-3 unit and
commits it on an isolated iMac `agent/t058-regexp-lazy/claude` worktree.
Acceptance refuses to start without that commit, its exact plan bytes and
successful validation. The commit is also exported as a small Git bundle for
the coordinator to import; neither origin nor this local branch is pushed.
If fresh evidence does not justify this proposal, all dependent jobs stop.

| Choice | Frozen value and reason |
| --- | --- |
| New priority | Fresh formal30 current-main queue, compared with preceding `89a13699`; identical runtime here is acceptable for queue generation. Papaparse must still rank 1 in development; no override or replacement target if it does not. |
| Profile | Exact current-main queue image; two complete papaparse passes, macOS sample 60 s at 1 ms. Keep stdout/raw/receipt; require lazy-scan atom and RegExp replace frames. This diagnostic sampling is never acceptance timing. |
| Plan/policy | New schema-3 `regexp-lazy-quantifier-current-base-v6`; policy v6 SHA-256 `be123f7c5cfd46aa863467256eb8bf6947ff64f72541efc371f05a36c5535a31`, the newly required prospective policy. V5 bands/schedule remain unchanged. |
| Target/payoff | Only `development/papaparse/csv`, <=0.70 plus reciprocal B_i and all-pair improvement. These original choices stay fixed; fresh evidence may stop the proposal, never loosen them. |
| Mechanism/controls | Same general `cpu-work` traversal, no memory-wall targets; all other 85 selection cases, six sentinels and all lane/portfolio/instruction gates. No resource waiver is introduced. |
| Layouts | Fresh audited base/candidate K=6 at 0/16/64/240/512/1008, workers-4, selection 5 pairs/layout. Mandatory full panel avoids post-result layout selection. |
| Held-out | All nine programs, fresh 30 pairs at every layout, workers-4; unchanged persistence and bootstrap precision gates, never priority input. |
| Formal/power | One fresh 30-block workers-4 bundle after a passing screen; v6 exact median >=97.5% interval at fixed looks 30 and 240. Extend only unresolved cases directly to 240 via `--plan`; crossing intervals at cap remain incomplete. Joint lanes/held-out keep their original rules. |
| Budget | One fixed current-base candidate, `max_attempts: 1`; no implementation variation or favorable rerun. This is a new evidence-bound proposal allowed by T062, not an extension/reset of the exhausted old unit. |
| Recovery | Existing load-only pair recovery/caps; whole disturbed formal/cap-invalid jobs can be replaced at most three times, never pooled. No non-load retry. This dispatch does not auto-submit whole-job replacements. |
| Correctness | Fresh exact-candidate zero-gap Test262 scan for `9e415ff4`, separate from local full checks; original `392ca717` entry is inadmissible for the new pair. |

Queue/profile hashes are necessarily unknown before those jobs run. The
materializer has no discretion over targets, thresholds, offsets, rule,
source pair or budgets. It records a lower-bound inclusive fraction from
the largest sampled replace branch divided by main-thread samples; overlapping
frames are not summed. Guard failure blocks acceptance and preserves output.

## Jobs and result reading

Pending submission after this prospective contract is committed. All jobs
use workspace `scripts/imac-bench.sh`, status first; timing jobs use `--timing`.
New queue/profile and all acceptance output paths are exclusive. Spotlight/
photo/media disable flags and daemon states are recorded without STOP/CONT
or service changes. No completion is awaited.

Allow about 5-6 hours of iMac occupancy: fresh queue comparison 45-60 min,
profiling/freezing a few minutes, build/audit overhead, selection K=6 about
61-74 min, formal about 45-60 min, held-out K=6 about 101-125 min, exact
correctness about 28-30 min, power/decision several minutes. The lower panel
figures are attempt-2 observations (3673/6061 s), not a promise for new images;
upper figures come from T060/T061. Queue waiting, quieting, precision work
and load-only replacements are additional.

Coordinator: fetch all new jobs, first verify fresh queue/profile priority,
base/candidate and the freeze job's validation, `unit.json`, commit/hash and
`frozen-plan.bundle`. Fetch its owner branch into a temporary ref and
cherry-pick only the plan/index commit onto this branch; preserve its frozen
bytes. Then inspect every prerequisite's status and the decision job's
`out/decision.json`: `decision`, `decision_readiness`, reasons and bound hashes.
Only complete clean `retained` evidence can support the new source pair.
A failed priority/profile/freeze guard, valid rejection or inconclusive result
stops this proposal. No old attempt is re-decided under v6 or pooled.

## Scope and checks

Own this task, its two index entries, new attempt-2 archive and prospective
contract; the queued freeze adds only its new performance-unit plan/index.
No engine/dependency/third_party/benchmark-tool implementation changes.
The local full `check.sh` and staged fast gate must pass before handoff;
plan-specific `validate-unit` runs on the fresh original-host artifacts
before its remote commit. Exact scan writes a burndown entry for coordinator
recording after completion; no current full-scan count is claimed early.
