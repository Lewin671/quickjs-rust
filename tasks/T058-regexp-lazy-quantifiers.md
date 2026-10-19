# T058: Fixed lazy-quantifier candidate under policy v6

- Status: active; coordinator accepted the general fix; new single-candidate unit proposed prospectively, fresh evidence/freeze and acceptance pending.
- Implementation: present at fixed candidate `736200aa`; no further runtime edits or optimization retained by a decision.
- Verified at: `736200aa` full/fast gates and focused semantic tests passed; `645afdd9` records exact x86 diagnosis and diagnostic-only quick triage (unclear).
- Evidence: [prospective contract](proposals/T058-bounded-helper-v6.json), [diagnosis](archive/T058-instruction-diagnosis.md), [v6 rejection](archive/T058-v6-rejection.md), [attempt 1](archive/T058-attempt1-incomplete.md), [attempt 2](archive/T058-attempt2-inconclusive.md).
- Unresolved: fresh rank/profile bindings, committed schema-3 plan, complete selection/formal/held-out/correctness gates and promotion decision. No new acceptance result yet.
- Next action: submit the guarded chain below; inspect every status and timed log before interpreting any result. Any DISTURBED invalidates the whole job, irrespective of exit code.

## Prospective scope

The coordinator explicitly accepts `736200aaf49202dcfb6adf4169cc60d52d719c06`
as a general bounded-helper fix and authorizes a changed candidate as a new
unit, `regexp-lazy-quantifier-bounded-helper-v6`. Its fixed base is current
`origin/main`, `a68a9f469a1c36c8080b71e1546851a30f448da6`. T057 belongs to
both roles; no T057 gain is credited to T058. The mechanism and risks include
the incremental lazy atom traversal, shortcut removal and fixed 0-4 argument
copies. There is exactly one candidate and one valid attempt; all old units
and outcomes stay immutable. No old acceptance data are reused or re-decided.
The unclear quick triage neither passes acceptance nor spends/resets a budget.

The new [contract](proposals/T058-bounded-helper-v6.json) freezes every choice
before fresh evidence. It is not a fabricated schema-3 plan with placeholder
hashes. A fresh 30-block workers-4 comparison of `89a13699` versus `a68a9f46`
creates the current-base queue; a new 60-second profile of two papaparse passes
uses that queue's exact candidate image. Papaparse must still rank 1 and the
profile must support the proposed path, otherwise the chain stops. Only the
observed queue/profile bindings and inclusive branch counts may be filled in.
The isolated runner freeze job validates and commits the real schema-3 JSON
and plan index, then exports the exact plan/commit/bundle before selection.
Its unique plan branch is `agent/t058-fixed-v6-plan/claude`; it never overwrites
the old rejected plan branch. Plan SHA/commit are pending that job.

Target, payoff, reciprocal bands, instruction caps, controls and resource,
precision, correctness and promotion requirements are unchanged. The contract
binds v6 policy SHA-256
`be123f7c5cfd46aa863467256eb8bf6947ff64f72541efc371f05a36c5535a31`;
CPU-work, memory targets empty, mandatory K=6, five selection pairs/layout,
30 held-out pairs/layout and 30 formal blocks, workers-4. Papaparse/csv is the
sole target, payoff <=0.70 plus its reciprocal band; all 85 non-target selection
cases remain controls, including all six sentinels. All nine held-out cases
are mandatory. V6 formal precision uses fixed 30/240 looks and exact >=97.5%
median intervals; lane/held-out precision stays unchanged. No favorable-run
selection, target substitution or automatic retry is authorized.

## Dispatch window and guards

Status was checked first: no queued/running jobs. iMac clock at inspection:
2026-10-20 07:29:40 CST (UTC+8); 98 GiB disk free. Previous fresh baseline
was 3,347 s, profile 104 s, selection including builds 4,099 s. Full chain
estimate is 5-6 hours, with an 8-hour planning allowance: at about 07:30
submission, expected completion 12:30-13:30, conservative allowance to 15:30,
well before next 03:00. A later status check found T057 profile job
`20261019T232925Z-t057-stage2-fresh-evidence` ahead (two samples, each
workload capped at 900 s). Allow another 35 minutes of queue waiting, so
the conservative completion estimate is about 16:05. Any additional
unassessed predecessor stops dispatch. Case-dependent formal continuation can lengthen it;
this is an estimate, not a precision waiver. No jobs are retried automatically.

All job command bodies have an absolute 2026-10-21 02:45 CST deadline,
15 minutes before the known 03:00-05:00 background window. They refuse late
starts and terminate their own command process group at the deadline, exit
124, retaining partial artifacts as incomplete. This also applies after any
runner quieting delay. Every timed job uses `imac-bench.sh run --timing`.
Every dependent command requires successful statuses and clean timed logs;
queue-reported DISTURBED means the entire job and dependent evidence are
invalid, never per-pair recovery. Replacement would require a separate
record within the standing load-only budget and a fresh window assessment.

## Jobs and verification

Submission and metadata verification are pending. The chain is fresh baseline,
fresh queue/profile, validate/commit plan, full selection K=6, acceptance
formal30, policy-bound formal power/continuation, held-out K=6, exact Test262,
then promotion decide. Later jobs stop on failed prerequisites, preserving
errors without measuring. The exact scan is for `736200aa`; only a complete
scan can produce a burndown entry. No source change, merge or origin push.
