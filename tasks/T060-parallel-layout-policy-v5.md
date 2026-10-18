# T060: Case-sharded layout collection and schedule-bound policy v5

- Status: active; calibration plan frozen before timing.
- Implementation: tooling present in working tree; v5 not published.
- Verified at: plan commit 91c97d64; tooling working tree passed staged fast gate (643 tests), context/size gates and the additional forked cap-propagation regression.
- Evidence: frozen protocol below; job receipts and results will be recorded here.
- Unresolved: workers-4 bands, qualification and screen/panel elapsed speedup unmeasured.
- Next action: commit tooling, queue calibration behind existing owners, and record all receipts/results.

## Scope and architecture

Tooling, policy and documentation only; no engine/dependency/third_party edits.
Base 998dd5dddcdffab5b16459ab101c6d7c8461f1fc; branch
`agent/parallel-screens/codex`, worktree `parallel-screens`. Commit locally;
no push or merge. The queue transport may transfer the commit to its runner.

A single parent owns preparation, lock, preflight, aggregation and the final
load-log seal. Deterministic round-robin case shards run in N forked processes.
A case's roles, alternating pairs, load-only retries and all requested layouts
stay in one process. No simultaneous measurement of two layouts of one case.
The serial path retains the old layout-major order. Raw cases bind schedule
and shard; summaries retain the complete shard inventory. Replays conserve
all cases, layouts, attempts, role order and the append-only load log.
The existing disturbance protocol/caps are unchanged. Continuations preserve
the pilot schedule. Policy/evidence schedule mismatches fail closed, including
formal `workers-4` identity; old absent schedule means serial layout collection.

Machine-readable inputs: [frozen plan](calibration-plans/T060.json).

## Frozen calibration contract (before timing)

Recalibrate every band and ceiling under `case-shards-v1`, workers=4; never
reuse serial timing observations as parallel evidence. V4 stays immutable.
Use v4's band sources: original 76-case A2 audited matrix of `153c1fe5`
(natural SHA-256 `926ee634ca3901b89c778478702a4fe993288ef7e8e69cd93c238c694595aa85`),
and the development/held-out audited matrix of `4c9f959e4ae3f508a10828a46cf59ec042d7ec13`
(natural SHA-256 `c7f0f66a519181b214bdc5ca6404cf428f0d18cbd1ab1d0e26386097619b949e`).
Reuse exact immutable matrices if available; rebuilding must reproduce hashes
and all static audits. Offsets stay 0/16/64/240/512/1008, all roles distinct,
equal-length immutable paths. No chosen winning layout.

1. Collect six five-pair A2 placements for broad/sentinel/external,
   development and held-out. Base=natural, candidate=placement k, including
   natural/natural k=0. Preserve raw role costs and paired ratios.
2. Collect an independent 30-pair held-out A/A of c7f0f66a to estimate the
   v4 held-out reciprocal confidence-envelope floor. Freeze the seed BEFORE
   qualification. Per-case B_i is max(six-layout spread of role-median ratios
   normalized to v0, spread of paired medians with v0=1, global reciprocal
   natural median wall bias); held-out additionally includes its independent
   30-pair reciprocal 95% interval. I_i rounds upward to one basis point the
   max(global reciprocal natural median instruction bias, natural reciprocal
   instruction-pair extremes); held-out includes independent A/A instruction
   pairs too. Each lane and the 86-case portfolio ceiling is max(six-layout
   geomean spread, reciprocal natural lane bias); held-out additionally uses
   its independent A/A reciprocal lane interval. No old v4 ceiling floor.
3. Fresh 30-pair qualification A/A on c7f0f66a: portfolio, development and
   held-out separately. Case median, seed-0 20,000-draw bootstrap, <=3%
   half-width. Automatic power allocation min(240,ceil(30*(h/.03)^2)),
   predicted stage then cap; retain every pilot pair and original lane cohort.
   At cap, imprecise case bands become diagnostic and require K=6. No case
   may have every instruction pair above max(1.01,I_i). Every lane's original
   30-pair interval must be precise and inside its frozen reciprocal ceiling.
4. Fresh sealed canonical 30-block formal workers-4 A/A, candidate=base=
   `351004f386f4c2c1d870467ae6e1e481952a330c`, all 86 selection cases plus NG.
   Exact identities/raw replay/correctness/health/linearity required. Same
   precision allocation to cap; original 30-block lanes retain their cohorts.
   All formal lane intervals must fit frozen seed ceilings.
5. Qualify once from all replayed inputs. Never widen a seed after qualification
   or rerun a valid failure favorably. Publish v5 and its SHA-256 only on pass.
   Report every v4/v5 case/lane difference and diagnostic precision record.

Timing uses workspace `scripts/imac-bench.sh run --timing` only, after status;
queue behind T056/T057/T058 without cancelling them. DISTURBED pair caps or
formal runs replace whole, at most three load-only replacements; non-load
failures stop. Record every job id, revision, exit, disturbance and hash.

## Speed qualification

Measure complete matching-layout A/A natural screen and K=6 panel at 5 pairs,
86 cases, same c7f0f66a matrices/protocol. Run serial and workers=4 once each
on the queue; elapsed timing excludes builds/preflight and includes pair
settling/recovery. Retain raw evidence and per-screen/panel elapsed durations.
Use these as speed measurements, never as seed or qualification samples.

## Acceptance and verification

- [x] Case sharding, conservation, schedule mismatch, replay, load recovery tests.
- [x] Screen/panel/A2/calibration/npm lane worker CLI and immutable artifacts.
- [ ] Frozen seed and full qualification; v5 only if qualified.
- [ ] Compare v5/v4 bands and measured screen/panel speedup.
- [ ] Update T022, task index and owning artifact documentation.
- [ ] Hosted implementation aggregate updated; focused benchmark tests,
  `PYTHONDONTWRITEBYTECODE=1 ./scripts/check-touched.sh --base main --explain`,
  `./scripts/check-context.py`, and full gate before handoff.
