# T060: Case-sharded layout collection and schedule-bound policy v5

- Status: active; calibration plan frozen before timing.
- Implementation: tooling present; qualified workers-4 v5 artifact published on this branch; no engine change.
- Verified at: 76e456e8, staged fast gate (646 tests), context and size gates; prepare job passed on iMac; full gate queued.
- Evidence: frozen protocol below; job receipts and results will be recorded here.
- Unresolved: screen/panel elapsed speedup and full handoff gate pending.
- Next action: finish speed jobs and final full gate, record their results, then hand off this branch without push/merge.

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
- [x] Frozen seed and full qualification; v5 published only after a qualifying replay.
- [ ] Compare v5/v4 bands and measured screen/panel speedup.
- [ ] Update T022, task index and owning artifact documentation.
- [ ] Hosted implementation aggregate updated; focused benchmark tests,
  `PYTHONDONTWRITEBYTECODE=1 ./scripts/check-touched.sh --base main --explain`,
  `./scripts/check-context.py`, and full gate before handoff.

## Queue dispatch (tooling 93ce2534)

All timing jobs use `run --timing`; queued after the three pre-existing owners.
No other owner job was cancelled. Raw artifacts stay in the queue/fetch output.

| Job | Purpose | Result |
| --- | --- | --- |
| `20261018T044114Z-t060-prepare` | Reproduce/audit band sources and prepare equal-length role matrices | exit 0 / 362 s; no DISTURBED |
| `20261018T044128Z-t060-full-gate` | Superseded by current-tooling gate before execution | cancelled queued, unmeasured |
| `20261018T044128Z-t060-a2-portfolio` | Six 76-case A2 placements, workers=4 | exit 0 / 2110 s; no DISTURBED |
| `20261018T044128Z-t060-a2-development` | Six development A2 placements, workers=4 | exit 0 / 2424 s; no DISTURBED |
| `20261018T044129Z-t060-a2-heldout` | Six held-out A2 placements, workers=4 | exit 0 / 1127 s; no DISTURBED |
| `20261018T044210Z-t060-heldout-estimator` | Independent 30-pair held-out envelope estimator and power allocation | exit 0 / 1018 s; no DISTURBED |
| `20261018T044315Z-t060-seed` | Replay A2/estimator and freeze v5 bands | queued |

Qualification/speed jobs use f83cfe11. Every qualification collector refuses
to start before the immutable seed exists and binds the frozen plan hash.

| Job | Purpose | Result |
| --- | --- | --- |
| `20261018T044638Z-t060-full-gate-current` | Superseded before execution by wrapper-inclusive full gate | cancelled queued, unmeasured |
| `20261018T044640Z-t060-pilot-portfolio` | Fresh 76-case screen qualification A/A | queued |
| `20261018T044642Z-t060-pilot-development` | Fresh development qualification A/A | queued |
| `20261018T044644Z-t060-pilot-held-out` | Fresh held-out qualification A/A | queued |
| `20261018T044646Z-t060-formal-aa` | Canonical 86-case workers-4 formal A/A | queued |
| `20261018T044647Z-t060-formal-power` | Automatic formal precision allocation | queued |
| `20261018T044649Z-t060-qualify` | Replay qualification and v4/v5 band comparison | queued |
| `20261018T044651Z-t060-speed-serial` | Serial natural screen plus K=6 elapsed measurement | queued |
| `20261018T044653Z-t060-speed-parall` | Workers-4 natural screen plus K=6 elapsed measurement | queued |

| `20261018T050252Z-t060-full-gate-final` | Full check.sh at 76e456e8 (no timing) | queued |

Prepare reproduced the frozen 926ee634 natural image and passed all six
original-source audits; the c7f0f66a npm matrix and equal-length role copies
were verified. Only two light background samples, below disturbance threshold.
Portfolio A2 started at 08:15:13 UTC on 2026-10-18, with four case-owned forks.

## Frozen seed (before qualification results)

Seed job `20261018T044315Z-t060-seed`: exit 0. SHA-256
`d9f25f111d2c49fcf897fcc2932a0b8d6613bcfed972bd5fca417a4cf3585391`.
All 570 A2 packets replayed: 2,850 valid attempts, zero invalid/heavy samples
(portfolio 2,280, development 300, held-out 270). No observations pooled.

All 95 B_i increased versus v4. Selection B_i min/median/max:
1.050127 / 1.050127 / 1.113165; held-out: 1.050127 / 1.050127 / 1.067810.
The global reciprocal natural median floor is 1.0501274307821105, from
`broad/dynamic_method_call`. The instruction median floor is 1.0068368339681264
from `external/kraken-1.1/audio-beat-detection`, rounded to 1.0069. Both maxima
are in the original 76 cases: using the v3/v4 successive floor scopes gives
these same floors. Selection I_i: 84 increased / 2 decreased; held-out:
8 increased / 1 decreased. Thresholds remain frozen even if qualification fails.

| Lane ceiling | v4 | workers-4 v5 seed |
| --- | ---: | ---: |
| broad | 1.012949330 | 1.016805577 |
| sentinel | 1.015687002 | 1.018672233 |
| external | 1.007380238 | 1.005804468 |
| development | 1.009630314 | 1.009347089 |
| portfolio | 1.007249456 | 1.006849231 |
| held-out | 1.005018254 | 1.018685972 |

Portfolio qualification A/A started at 10:11:37 UTC, after the seed was sealed.

## Qualification outcome (tooling f83cfe11)

Job `20261018T044649Z-t060-qualify`: exit 0 / 127 s, `qualified: true`,
`failures: []`. Published policy SHA-256
`f3e8ef40b8b45601b5b86edaf4b47c2d9d904d0045eb024e837f1c447a52bec4`.
All seed bands/ceilings are unchanged in the policy; exact per-case differences
are reproducible from committed v4/v5 policies with `compare_bands`.

All 86 screen, 86 formal and 9 held-out precision records are precise. Screen
power allocation extended `broad/dynamic_method_call` to 38 pairs and
`sentinel/string_key_map_churn` to 39; formal
`external/sunspider-1.0/bitops-3bit-bits-in-byte` needed 44 blocks. No cap-240
fallback. Every instruction guard passed; every original 30-block lane
interval is precise and inside its frozen reciprocal envelope. Formal portfolio
0.999520 [0.998770, 1.000346]; held-out 1.000274 [0.998240, 1.002317].

Formal job `20261018T044646Z-t060-formal-aa`: exit 0 / 3,524 s, no DISTURBED;
both Rust roles reproduce c7f0f66a, source 351004f3, with `workers-4`. Formal
power job `20261018T044647Z-t060-formal-power`: exit 0 / 29 s. Qualification
replayed all raw files, identities, guards, continuations and unchanged seed.
Speed serial began at 12:37:20 UTC on 2026-10-18; no speed result claimed yet.
