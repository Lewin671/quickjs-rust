# T058: Fixed lazy-quantifier candidate under policy v6

- Status: closed-landed; unit `regexp-lazy-quantifier-bounded-helper-v6` **retained** (`ready_for_unit_gates`) on its sole attempt; merge into main is the coordinator's step.
- Implementation: present at candidate `736200aa`; the closing branch adds only task/plan records, so its source tree outside `tasks/` and `docs/` is byte-identical to the measured candidate.
- Verified at: `ca6e383a` (source identical to `736200aa`), 2026-10-20: full `check.sh` on the iMac passed; local fast gate and `compare-qjs.sh` passed; iMac `compare-qjs.sh` has one pre-existing time-zone mismatch, below.
- Evidence: [frozen plan](../performance-units/regexp-lazy-quantifier-bounded-helper-v6.json), [prospective contract](../proposals/T058-bounded-helper-v6.json), results below; workspace `outputs/imac-bench/<job>/` (local artifacts, not retained in Git). Earlier records: [diagnosis](T058-instruction-diagnosis.md), [v6 rejection](T058-v6-rejection.md), [attempt 1](T058-attempt1-incomplete.md), [attempt 2](T058-attempt2-inconclusive.md).
- Unresolved: two broad controls are about 7% slower at formal with flat instructions (below); they pass the frozen v6 rule and are not a rejection. The iMac photo library was deleted by the user before this chain (2026-10-19 21:00 CST); the recorded quieting state matches the policy's requirements, and whether that host change needs recalibration is a coordinator decision.
- Next action: none for this unit. Coordinator integrates the branch into main.

## Promotion result (fixed-candidate v6)

All nine `-window` jobs ended `done 0`; no log contains DISTURBED. Each
fetched `command.sh` matches the submitted SHA-256 listed below. The only
monitor rows are two light samples in the current-base formal30 job
(XprotectService 53.2%, UserEventAgent 128.7%), below the 200% threshold;
selection, acceptance formal30, held-out and queue/profile intruder logs are
empty. Runner clock was CST (UTC+8); every job finished before the
2026-10-21 02:45 deadline, outside 03:00-05:00.

| Job (`20261019T...Z-t058-fixed-v6-*-window`) | Runtime | Result |
| --- | --- | --- |
| `233613Z` current-base formal30 | 3,199 s | `89a13699` vs `a68a9f46` image `897eaa88...`, ready, summary `702a8e7f...` |
| `233614Z` fresh queue/profile | 101 s | queue `1e768cd1...`; papaparse/csv rank 1, 28.80x NG; 50,158 samples, replace branch 46,355, atom 28,663, inclusive >=0.9242; receipt `f933e9b6...` |
| `233614Z` validate/freeze plan | 3 s | runner commit `6747434f` (parent `01ab9a0c`), unit SHA-256 `ae318d30...`; imported unchanged by merge `ca6e383a` |
| `233615Z` selection K=6 | 4,094 s | `pass`, complete, 86/86 cases pass; one wall event (`broad/many_locals_call`) resolved nonpersistent; no failure |
| `233616Z` acceptance formal30 | 2,484 s | `a68a9f46` vs `736200aa` image `1f35cff0...`, ready, summary `48188bfb...` |
| `233616Z` formal power | 1 s | `continuations: []`; every case precise at 30 blocks |
| `233617Z` held-out K=6 | 6,141 s | 9/9 programs pass, no event |
| `233617Z` exact Test262 | 1,746 s | `736200aa`: 42,672/42,672 configured pass, 0 actionable gap; entry `8ce3ee31...` |
| `233618Z` promotion decide | 89 s | `retained`, `ready_for_unit_gates`; reasons, failures, missing, precision exclusions all empty |

A local directory `20261019T233619Z-t058-fixed-v6-promotion-decide-window`
is empty (created 16:43 local, seven minutes after submission) and has no
runner job or status: it is left by a `fetch` of a job ID that was never
queued (`fetch` creates the destination before rsync fails). It is not
evidence; the 233618Z job is the only promotion decision.

Target `development/papaparse/csv` (payoff <=0.70): selection K=6
aggregate 0.080534, wall median 0.080634, instructions median 0.049012;
formal 0.080628, threshold-median-v2 interval 0.080053-0.080909 (half-width
0.72%). Formal lanes: broad 0.996321, sentinel 0.990783, external 0.999678,
development 0.774577, portfolio 0.968906, all precise. Selection lane
ratios over six layouts: development 0.772-0.781, portfolio 0.966-0.971,
external 0.995-0.997, broad 0.988-1.007, sentinel 0.974-1.025 (layout 2
above its 1.018672 ceiling; the lane gate passes on the six-layout rule).
`sentinel/recursive_call_tree`, which rejected the earlier v6 unit, now has
instructions median 0.986178 and wall 0.954369 (formal 0.953685).

Controls above 1.01 at formal: `broad/property_dynamic_read` 1.074180
(band 1.088021, panel 1.071390), `broad/many_locals_call` 1.070626 (band
1.050127, panel 1.067109, the nonpersistent wall event),
`external/sunspider-1.0/access-fannkuch` 1.017549, `broad/dynamic_method_call`
1.016845, `broad/array_index_of` 1.016825. All five have instruction medians
within 1.0000 +/- 0.00005, so the candidate does no extra work there; the
frozen v6 rule passes them. Treat them as layout sensitivity, not a payoff.

Held-out K=6: ceiling 1.018686; layout lanes 0.992759, 0.998232,
0.989316, 0.993601, 0.996576, 0.999147 (geomean 0.994933). Program wall
medians: acorn 1.000425, crypto-js 0.990998, d3-array 1.009346, decimal.js
0.997554, esprima 0.987415, handlebars 0.985623, marked 0.993126, mustache
0.976881, ramda 0.993814.

Hashes (SHA-256, local artifacts): decision `ae8318de3ad25bbd069d10a7fb2cc0d4973a07f06947a07b995dbe390ddc6b85`;
selection verdict `5d0fe2f78029273bde2cb54cb49107fc349a895d312fd724e343f12ca8c45d88`,
natural panel `69b538e1a73858d7b100735450ef1ace1820f41cbe454bdcc57ddc0c65a08efd`,
complete panel `1ce82ec3ff28bf0c79d5b7753d946105bd887ad126dc89eacad40b39c2607e5b`;
held-out panel `6de66eac9f58ebcd6d06598244e6916be49d41b9655b29a46c9b9e3c23fffd28`;
acceptance summary `48188bfb8e8c8c12bc9074fd0f86b536348c224a748e478c36e34b8b216f3230`;
current-base summary `702a8e7f6aa95f81988335ab5bdaf5e648a63b57ee8b7a7cc3a91fc2ffa193e8`;
queue `1e768cd150a6bb14c904886290dfb1740603c172b9ebc36ac7dca53d78ace29c`;
profile receipt `f933e9b65d3c0eb02fc1633ffc046587b955c3d531191e93ad3685a0c7bda821`;
power `ad56ffde519870c2f85e7eaaaf3b8d81038e1c45d2a6628447b8a8e3c0e64cf3`;
Test262 entry `8ce3ee3117751434e3746c2098feccb9a63b22931cc0fe8a6c7a5699a18de751`;
policy v6 `be123f7c5cfd46aa863467256eb8bf6947ff64f72541efc371f05a36c5535a31`.

## Unit gates before integration

The branch already contains main `a68a9f46` (the plan's base, including T057
stage 1, T061 and T062), so integration needs no rebase or conflict
resolution. `git diff 736200aa ca6e383a` touches only `tasks/`; no
re-measurement is implied by the freeze rules.

- Full gate, iMac no-timing job `20261020T055339Z-t058-unit-gates-full-check`
  at `ca6e383a`: `CARGO_INCREMENTAL=0 ./scripts/check.sh` exit 0 (format,
  clippy default/agents/perf-counters, feature and workspace tests,
  benchmark-tool and script tests, unit plans, context, hooks, shell syntax,
  file size, 5,170 curated Test262 cases). Log SHA-256 `503f7086...`.
- Fast gate, local: `./scripts/check-touched.sh --base a68a9f46 --explain`
  exit 0 (fmt, clippy, 2,313 qjs-runtime tests, all three T058 unit plans,
  65 selected Test262 cases).
- `./scripts/compare-qjs.sh`: local (America/Los_Angeles) exit 0, all 251
  fixtures. In the iMac job above it exits 1 on `date-basic.js` only:
  `Number.isNaN(new Date(275760, 8, 13, 0, 0, 0, 1).getTime())` is `true`
  in quickjs-rust and `false` in QuickJS-NG. The runtime's local time zone is
  UTC by design (`date/legacy.rs`), while QuickJS-NG uses the host's UTC+8
  zone, where that local time is inside the time-value range. Job
  `20261020T055946Z-t058-main-compare-qjs-tz-check` reproduces the same
  mismatch on main `a68a9f46` and passes with `TZ=UTC`; it is a
  host-time-zone dependence of the fixture, unrelated to this unit.
- Correctness: exact full Test262 for `736200aa` is in the result table.

## Prospective scope (recorded before any result)

The coordinator explicitly accepts `736200aaf49202dcfb6adf4169cc60d52d719c06`
as a general bounded-helper fix and authorizes a changed candidate as a new
unit, `regexp-lazy-quantifier-bounded-helper-v6`. Its fixed base is current
`origin/main`, `a68a9f469a1c36c8080b71e1546851a30f448da6`. T057 belongs to
both roles; no T057 gain is credited to T058. The mechanism and risks include
the incremental lazy atom traversal, shortcut removal and fixed 0-4 argument
copies. There is exactly one candidate and one valid attempt; all old units
and outcomes stay immutable. No old acceptance data are reused or re-decided.
The unclear quick triage neither passes acceptance nor spends/resets a budget.

The new [contract](../proposals/T058-bounded-helper-v6.json) freezes every choice
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
remain in the [diagnosis](T058-instruction-diagnosis.md).


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
