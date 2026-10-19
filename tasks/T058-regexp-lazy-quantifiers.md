# T058: Current-base RegExp lazy-quantifier proposal under v6

- Status: active; old attempts 1/2 closed incomplete/inconclusive; fresh current-base v6 evidence/freeze/acceptance chain queued.
- Implementation: present on rebased candidate `9e415ff4`; its runtime diff from current main is T058 alone. No new runtime edits.
- Verified at: clean rebase onto `a68a9f46` at `9e415ff4`; runtime/proposal at `c7c43e05` plus dispatch documentation passed full `check.sh` on 2026-10-19. Attempt-2 hashes/logs checked; final staged fast gate covers handoff metadata.
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

Submitted on 2026-10-19 at 17:43 UTC after status showed idle. Tooling and
prospective contract commit: `c7c43e05`; contract SHA-256
`d9f61ce3f3a66816faf7643fbd4954721aced28876b4ada44b317bbc17552aa8`.
All jobs use workspace `scripts/imac-bench.sh`; timing jobs use `--timing`.

| Job ID | Purpose / output under its `out/` |
| --- | --- |
| `20261019T174302Z-t058-v6-current-base-formal30` | Fresh current-base comparison; `bundle/summary.json` and sealed raw |
| `20261019T174303Z-t058-v6-fresh-queue-profile` | Fresh `queue.json`, raw sample, `profile-summary.json`, `profiles/papaparse--csv/receipt.json` |
| `20261019T174304Z-t058-v6-validate-freeze-plan` | Validate and commit new schema-3 plan; `unit.json`, `validation.json`, `frozen-plan.json`, `frozen-plan.bundle` |
| `20261019T174305Z-t058-v6-selection-k6` | Build/audit fresh six-layout matrices and screen; `attempt/screen/complete.json`, `attempt/verdict.json` |
| `20261019T174305Z-t058-v6-acceptance-formal30` | New candidate/base acceptance; `bundle/summary.json` |
| `20261019T174306Z-t058-v6-formal-power` | V6 fixed-look continuation with `--plan`; `power/power.json` |
| `20261019T174307Z-t058-v6-heldout-k6` | Fresh full K=6 held-out; `heldout/panel.json` |
| `20261019T174308Z-t058-v6-test262-exact` | Fresh exact-candidate scan; `test262/`, `burndown-entry.json` |
| `20261019T174309Z-t058-v6-promotion-decide` | New-unit replay only; `decision.json`, `decision.stdout`, `--require-retained` |

Baseline, profiling, selection, formal, power and held-out are timed. Freeze,
correctness and decision are non-timing and still serialized by the queue.
Every job checks its prerequisites; canonical formal DISTURBED logs stop the
chain, while layout recovery is validated by its own packet contract.
Fresh queue generation keeps the standard 0.5 NG opportunity ratio. The freeze
job never changes choices from its contract, and acceptance verifies its
Git-committed plan bytes, proposal ancestry, queue/profile hashes and validation.
The pending plan path is `tasks/performance-units/regexp-lazy-quantifier-current-base-v6.json`;
it is deliberately not an unbound placeholder checked into this branch.
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

Handoff verification (2026-10-19 UTC):
`PYTHONDONTWRITEBYTECODE=1 CARGO_INCREMENTAL=0 ./scripts/check.sh` passed:
2,312 runtime tests, feature lint/tests (21 agents, 24 counters), all other
workspace crates, 659 benchmark-tool tests, 35 script tests, all committed
unit-plan/context/size/hook/syntax gates, and all 5,170 curated Test262 cases.
The recording/proposal commits change no engine source, so the staged fast
gate selects context checks rather than a new semantic slice; the full
curated semantic subset still ran. Every dispatched shell command and Python
heredoc passed syntax checks before submission. New evidence-dependent
plan validation and acceptance remain pending their queue jobs.

Local disposable `target/debug` (2.4 GiB) was removed after checks; canonical
performance images and all measurement evidence remain. Initial free data
volume: local 86 GiB, iMac 101 GiB. No origin push or merge. After the new
exact scan completes, record its retained full-scan entry with
`scripts/test262-burndown.sh --entry <fetched-burndown-entry.json>`; partial
scans are never recorded as complete. Do not use the old candidate entry.
