# T058: Fixed lazy-quantifier candidate under policy v6

- Status: active; coordinator accepted the general fix; new single-candidate contract committed; all nine guarded jobs submitted in dependency order, fresh evidence/freeze and acceptance pending.
- Implementation: present at fixed candidate `736200aa`; no further runtime edits or optimization retained by a decision.
- Verified at: `01ab9a0c` prospective tooling/runtime tree passed full `check.sh`; runtime unchanged from `736200aa`, whose focused semantic/fast gates passed; `645afdd9` records diagnosis/unclear diagnostic triage. Final dispatch syntax and deadline smoke checks passed; staged metadata gate below.
- Evidence: [prospective contract](proposals/T058-bounded-helper-v6.json), [diagnosis](archive/T058-instruction-diagnosis.md), [v6 rejection](archive/T058-v6-rejection.md), [attempt 1](archive/T058-attempt1-incomplete.md), [attempt 2](archive/T058-attempt2-inconclusive.md).
- Unresolved: fresh rank/profile bindings, committed schema-3 plan, complete selection/formal/held-out/correctness gates and promotion decision. No new acceptance result yet.
- Next action: inspect every status and timed log for the guarded chain below before interpreting any result. Any DISTURBED invalidates the whole job, irrespective of exit code.

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
workload capped at 900 s). A subsequent check also found `20261019T233209Z-t057-stage2-external-profiles`
(two 20-second samples with the same 900-second workload timeouts). These
two assessed predecessors receive a combined 70-minute waiting allowance;
conservative completion is about 16:45. Further unassessed predecessors stop
dispatch. Case-dependent formal continuation can lengthen it;
this is an estimate, not a precision waiver. No jobs are retried automatically.

All job command bodies have an absolute 2026-10-21 02:45 CST deadline,
15 minutes before the known 03:00-05:00 background window. They refuse late
starts and terminate their own command process tree (including separately grouped
collector children) at the deadline, exit
124, retaining partial artifacts as incomplete. This also applies after any
runner quieting delay. Every timed job uses `imac-bench.sh run --timing`.
Every dependent command requires successful statuses and clean timed logs;
queue-reported DISTURBED means the entire job and dependent evidence are
invalid, never per-pair recovery. Replacement would require a separate
record within the standing load-only budget and a fresh window assessment.

## Jobs and verification

Final chain submitted at 2026-10-20 07:36:13-18 CST after another queue/time check.
Tooling/contract commit: `01ab9a0cb7e4999c38dc75e069b5cd169f397673`;
contract SHA-256: `6df3506cd648355470791e6f3bfc5fe4b3a0cd491bfaf6fd88b2b57582bf04fc`. The fixed runtime source
remains `736200aa`; the documentation tip is not substituted as candidate.
Only the assessed T057 external-profile job was still running at final
submission; the preceding T057 profile had finished. Retain the conservative
70-minute queue allowance rather than tightening the window from that result.
The chain is fresh baseline,
fresh queue/profile, validate/commit plan, full selection K=6, acceptance
formal30, policy-bound formal power/continuation, held-out K=6, exact Test262,
then promotion decide. Later jobs stop on failed prerequisites, preserving
errors without measuring. The exact scan is for `736200aa`; only a complete
scan can produce a burndown entry. No source change, merge or origin push.

| Phase (submission order) | Job ID | Timing |
| --- | --- | --- |
| Current-base formal30 | `20261019T233613Z-t058-fixed-v6-current-base-formal30-window` | yes |
| Fresh queue/profile | `20261019T233614Z-t058-fixed-v6-fresh-queue-profile-window` | yes |
| Validate/commit schema-3 plan | `20261019T233614Z-t058-fixed-v6-validate-freeze-plan-window` | no |
| Selection K=6 | `20261019T233615Z-t058-fixed-v6-selection-k6-window` | yes |
| Acceptance formal30 | `20261019T233616Z-t058-fixed-v6-acceptance-formal30-window` | yes |
| V6 formal continuation | `20261019T233616Z-t058-fixed-v6-formal-power-window` | yes |
| Held-out K=6 | `20261019T233617Z-t058-fixed-v6-heldout-k6-window` | yes |
| Exact Test262 | `20261019T233617Z-t058-fixed-v6-test262-exact-window` | no |
| Promotion decide | `20261019T233618Z-t058-fixed-v6-promotion-decide-window` | no |

Read back every submitted `command.sh` and runner timing flag; shell syntax,
embedded Python, the absolute deadline and exact contract bindings checked.
The guard uses PID ancestry to stop only its command descendants before
termination, including collector-created sessions; it never stops host daemons.
Local smoke verification covered a detached sleeping grandchild, refusal
after an expired deadline, and preservation of normal exit status; all passed.
All nine jobs were queued at readback; no acceptance result is claimed.
Submitted command SHA-256, in the table's order (including deadline wrapper):

- baseline: `b42e4a0dfca1a80a08d374b1ecb168926a2b1f7c099ff051926c64df262653d9`.
- evidence: `320f41369ef24c9c11614dcdc01b8217983ecd5c5a8cc0bd8333bf48b8f7df2e`.
- freeze: `e5eb470bc31c43ada90aac920be58a4caa24fad50811199b979e43345a2726dc`.
- selection: `23f3866da938c5e4f87ec939d9ad696781692832a207a31a1f285fcdee2bfbe7`.
- formal: `53406651106a28bdd2a333d4e7b5a3672c11d4f8f216f026f5e8bd7d028cf562`.
- power: `8370e8fae7cba4eef1592443492823edef194bb016ba7cc6a71971fa99b64e39`.
- heldout: `7cac1bb9bc78858f2c32854b23cf105c58c1f5a20dcfe6bd3f5c7a4853a72ca2`.
- correctness: `2513468e7a7c06851207f7d3990c3f112cec66c0b8a2f707d369e69f1fd84681`.
- decide: `b5e687a96d1af32b18d70ac10ac654963901fd4f2e3eecadeff4594196f5faf3`.

The freeze output owns `unit.json`, `validation.json`, `frozen-plan.json`
(commit/unit/queue/profile hashes) and `frozen-plan.bundle`. Fetch it after
success and preserve/import its exact committed plan before recording the
result; do not reconstruct the JSON or replace hashes. The final job owns
`out/decision.json`; only inspect it after every prerequisite succeeds and
all timed logs are clean. A failed/incomplete prerequisite or deadline is
recorded as such, never interpreted as a passing promotion. Commit and
fetch operations use runner-only `refs/bench/`; no origin push or merge.

Full `PYTHONDONTWRITEBYTECODE=1 CARGO_INCREMENTAL=0 ./scripts/check.sh`
passed (exit 0) on the prospective tooling/source tree at `01ab9a0c` with
handoff documentation pending: format/clippy and agent/perf-counter features,
workspace tests, 659 benchmark-tool tests, 35 script tests, all existing unit
plans/context/size/hooks/syntax gates and all 5,170 curated Test262 cases.
Runtime is unchanged from verified `736200aa`; final metadata passes the
staged fast gate (context check). Local `target/debug` (905 MiB) was removed
after verification; canonical binaries and all measured/diagnostic outputs stay. Check log SHA-256:
`58e339933fbe5be61eae0fb3c3ecb40d40e74be9d838957d078d526202319fd2`
(local artifact, not retained). Previous source checks and semantic coverage
remain in the [diagnosis](archive/T058-instruction-diagnosis.md).


## Pre-start dispatch correction

The initial 07:32 batch was cancelled in reverse dependency order while
**every job was still queued**, before any command/build/sample/plan commit.
All nine statuses are `done 130 (cancelled)`. The first deadline wrapper
only killed its root process group; collectors start separate sessions, so
it could leave a descendant. The final `-window` batch instead stops the
owned command tree at the same absolute cutoff. Candidate, source/tooling
refs, contract SHA, prospective choices and inner measurement commands are
unchanged; prerequisites point to the final IDs. No attempt was spent, no
result was selected and no disturbed/valid run was retried. Cancelled IDs
and original submitted command hashes remain independently auditable:

| Cancelled job ID | Original command SHA-256 |
| --- | --- |
| `20261019T233238Z-t058-fixed-v6-current-base-formal30` | `1bee39f5196eaf628d2c2f8f1df3298993e27cbdd2e9900d10c038dd413c7afa` |
| `20261019T233239Z-t058-fixed-v6-fresh-queue-profile` | `4a14746e0a2562a32859f6be1e962c97efacce15daa0c2a1feb6e07550484e62` |
| `20261019T233239Z-t058-fixed-v6-validate-freeze-plan` | `a287c20dd008b01aaa80715c4b9514f7aa337a89dbd605efa0ed793640f63929` |
| `20261019T233240Z-t058-fixed-v6-selection-k6` | `4cb6317b8be1a0526c778871d716759d804d699955eab887e7a303bca510bfe5` |
| `20261019T233240Z-t058-fixed-v6-acceptance-formal30` | `0dfd7c5257464a2a73c126365c5a5fffd8956d83028bb7eae64fcc89cd7df0f7` |
| `20261019T233241Z-t058-fixed-v6-formal-power` | `9e6091927ba1c1c4acfe6be483c6d692b131fd5594186714fd46b74bf90e31cc` |
| `20261019T233241Z-t058-fixed-v6-heldout-k6` | `d74a38a6377c480b5b6c2df7a6a4ac5f4ba5c49b28e7e55ea8d776d330fe5708` |
| `20261019T233242Z-t058-fixed-v6-test262-exact` | `89b8a2c64497f0c0dbce776fc5162a5c50e0a6320393374547b9606176af387f` |
| `20261019T233242Z-t058-fixed-v6-promotion-decide` | `784c747f19c134e4a796a372a7c7bc571bda26503847448e0781bf5553240d96` |
