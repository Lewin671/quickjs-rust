# T059: Held-out layout persistence and candidate formal continuation (policy v4)

- Status: closed-landed; policy v4 qualified and published.
- Implementation: present; tooling `1228aa64` + lane-event rule `a13d7004`,
  [policy v4](../../benchmarks/policies/imac-x86_64-rust1.95-fat16-layout-v4.json),
  SHA-256 `1eb7de462de0cb9f85c581716392d36dedc7c0c655afde1280c5c94c5052754b`,
  bound as `HELD_OUT_SUCCESSOR` of v2/v3. No runtime code changed.
- Verified at: iMac jobs `20261017T205859Z-t059-v4-heldout-a2`,
  `20261017T223239Z-t059-v4-heldout-aa30-r2`, `20261017T223240Z-t059-v4-qualify-r2`
  (all exit 0, no DISTURBED); qualification `qualified: true, failures: []`;
  branch fast gate passing (see Outcome).
- Evidence: plan, seed and outcome below; raw job outputs are workspace-local
  under `outputs/imac-bench/<job>/`.
- Unresolved: a candidate formal case still imprecise at 240 blocks stays
  incomplete (no diagnostic fallback for candidate bundles).
- Next action: none here; T057 stage 1 is re-measured fresh under v4 by its owner.

## Lead decision (2026-10-17, recorded verbatim before any timing)

> the held-out acceptance gate is structurally inconsistent with every other
> lane. Controls and the broad/sentinel/external/development lanes use bands
> calibrated across six layout variants and resolve flat-instruction wall
> excesses with a K=6 persistence panel; held-out bands and the held-out lane
> ceiling (1.0029) come only from same-binary A/A, and held-out has no
> persistence check. Any new binary's code placement can therefore fail
> held-out. This was exposed by T057 stage 1 (candidate 026476d5): held-out
> handlebars 1.0233 / esprima 1.0087 / ramda 1.0090 with instruction ratios
> 1.0000/0.9994/0.9996, lane 1.0050 vs 1.0029. The correction applies to all
> future units, and the T057 candidate will be re-measured fresh under it (no
> reuse of the already-seen held-out result).

Scope given with the decision: policy v4 = v3 plus held-out bands and lane
ceiling calibrated across the six declared layouts of the qualified
band-source image, exactly like T057's development A2, plus the existing
30-pair held-out A/A (max of the estimators); a held-out wall event resolved
by a K=6 matching-layout held-out panel; and case-subset precision
continuation of candidate/base formal bundles. Tooling, policy and docs only;
no runtime engine change. Promotion still requires the held-out gate.

## Frozen v4 calibration plan (before timing)

Parent: qualified v3 bytes, SHA-256 `f655f25a86f4933cfb11a097a7e109b493628bc1f9a9d320cd27bd7bf300ba33`.
Every v3 case band, instruction band, selection-lane and portfolio ceiling,
precision record and evidence entry is copied unchanged; only `held_out`,
`lanes["held-out"]` and `precision["held_out"]` are replaced, and
`held_out_rule`, `held_out_manifest_sha256` and `held_out_evidence` are added.
v2/v3 bytes stay frozen.

Source image: the qualified band-source image of T057's development A2,
revision `4c9f959e4ae3f508a10828a46cf59ec042d7ec13`, natural SHA-256
`c7f0f66a519181b214bdc5ca6404cf428f0d18cbd1ab1d0e26386097619b949e`, using the
six audited variants (natural/16/64/240/512/1008) of job
`20261017T080252Z-t057-development-qualified-matrix`; every hash is
re-verified. No placement is selected.

1. **Held-out A2:** all nine held-out programs, five paired whole-process
   samples per program for each of six placements; base is always natural,
   candidate is placement k (natural/natural at k=0); role order alternates;
   raw costs/ratios, load-only recovery and caps as in the v2 contract.
2. **Bands, frozen from A2 before the qualification A/A:** per program,
   B_i = max(six-layout spread of same-screen role medians normalized to v0,
   spread of paired medians with v0=1, global reciprocal natural median wall
   bias, v3 band B_i). The global bias is the max over the original 76-case
   A2 A/A medians, T057's development A2 natural medians and these nine A2
   natural medians. The v3 band must reproduce from the existing 30-pair
   held-out A/A (job `20261017T022616Z-decision-rule-b-heldout30-pairs`,
   SHA-256 `1ca33dab...`) as round_up(max(1, upper, 1/lower)) or the run stops.
   I_i = round_up(max(global median instruction bias over the same inputs,
   reciprocal extremes of A2 natural instruction pairs and of the 30-pair
   A/A instruction pairs)). Held-out lane ceiling = max(six-layout geomean
   spread, reciprocal natural A2 lane bias, v3 ceiling 1.0029). Never widened
   after observing qualification.
3. **Qualification A/A:** a fresh 30-pair natural/natural held-out A/A of
   c7f0f66a with automatic power continuation (predicted stage, then cap 240).
   Gates: every program's precision record complete (precise, or
   cap-imprecise and diagnostic); no program with every instruction pair above
   max(1.01, I_i); the lane interval (original 30-pair cohort) precise and
   inside [1/ceiling, ceiling]. No formal A/A: held-out is not in the formal
   bundle and no selection-lane band changes.
4. **Publish** `benchmarks/policies/imac-x86_64-rust1.95-fat16-layout-v4.json`
   and its `.sha256` only from a qualifying replay of all inputs. A failed
   qualification is recorded, not repeated favorably.

Queue only, `run --timing` for timed jobs, one at a time behind other owners'
jobs; a DISTURBED job is replaced whole, never pooled.

## v4 held-out rule (applies to every promotion decided after v4)

Per program, with ceiling max(1.01, I_i): natural median within B_i passes.
Above B_i with every instruction pair above the ceiling rejects. Above B_i
without that is a held-out wall event, resolved by a K=6 matching-layout
held-out panel (the 30-pair natural screen is variant zero, five completed
layouts at the same pair count): reject only if all six medians exceed B_i,
or if any layout shows the consistent instruction excess with its wall above
B_i. The held-out lane follows the lane-event rule below. Promotion intervals: an upper bound above
B_i or the lane ceiling needs the complete panel; imprecision stays missing.

## Lane-event rule (lead decision 2026-10-17, recorded before any decision uses it)

Rationale given: each lane ceiling is the observed maximum lane ratio over six
neutral layouts, so a neutral candidate is effectively a seventh layout and
exceeds each lane's max-of-six with probability about 1/7; with five or six
lanes, a natural-only hard lane gate falsely rejects neutral or beneficial
units often, and it is inconsistent with the per-case rule, where a
single-layout wall excess is an event resolved by the K=6 panel. Exposed by
T058 screen 1 (`20261017T222653Z-t058-screen1`): broad lane 1.0172 vs ceiling
1.0129, driven by broad/top_level_function_call 1.43 with instructions
1.0001-1.0002 and bimodal pairs (1.046, 1.018, 1.491, 1.526, 1.432) in a unit
that only changed RegExp code; target papaparse 0.186.

Rule (part of v4; no new calibration): a lane or the portfolio above its
ceiling in the natural layout is a lane event, not a rejection, and requires
the complete K=6 matching-layout panel. It rejects only if the geometric mean
of that lane's six per-layout ratios exceeds the ceiling, or if every layout
exceeds it. The same holds for the held-out lane. Consistent instruction
regressions above the instruction ceiling and the per-case hard cap still
reject immediately. A formal lane interval above its ceiling likewise needs
the panel. The T058 natural packet is a test fixture (lane event, panel
required).

## Candidate formal continuation

`layout_formal_continuation` accepts candidate/base bundles: it appends
blocks to every pilot-imprecise case (or a `--case` subset) under the sealed
three-role protocol and identities, by the power rule (cap 240), and marks
the chain `candidate-base`. `decide --formal-continuation` replays the
latest chain per case; case statistics use all blocks, lanes keep the
original joint 30 blocks. A precise case cannot be continued.

## Dispatch and frozen seed

Tooling `1228aa64`. A2 job `20261017T205859Z-t059-v4-heldout-a2`: exit 0 /
3565 s, no DISTURBED; reused inputs replayed and every v3 held-out band and
the 1.0029 ceiling reproduced from their A/A. Seed SHA-256
`c1ee591ffa22edf63933daa113e15ba1ecef5a5fcb30b0dd6d56be7019923a99`, frozen
before the qualification A/A:

| Program | B_i (v3) | I_i |
| --- | --- | --- |
| acorn/parse | 1.025025 (1.0088) | 1.0060 |
| crypto-js/hash | 1.025025 (1.0053) | 1.0021 |
| d3-array/aggregate | 1.025289 (1.0042) | 1.0021 |
| decimal.js/arith | 1.025025 (1.0038) | 1.0021 |
| esprima/parse | 1.025025 (1.0030) | 1.0086 |
| handlebars/render | 1.035681 (1.0066) | 1.0021 |
| marked/render | 1.025025 (1.0076) | 1.0021 |
| mustache/render | 1.027152 (1.0100) | 1.0021 |
| ramda/pipeline | 1.025025 (1.0065) | 1.0021 |

Held-out lane ceiling 1.005018 (v3 1.0029). 1.025025 is the global A2 median
bias floor that every v3 selection case already carries.

The first A/A/qualify submissions (`20261017T205900Z`, `20261017T205901Z`)
were cancelled while queued, unmeasured, to pick up main's wall-clock settle
fix (`f1dc1032`, merged as `7bc826bb`); replacements
`20261017T223239Z-t059-v4-heldout-aa30-r2` (timed) and
`20261017T223240Z-t059-v4-qualify-r2` (no timing) run at `7bc826bb`.

## Outcome

Qualification A/A `20261017T223239Z-t059-v4-heldout-aa30-r2` (c7f0f66a, 30
pairs, exit 0 / 3193 s, no DISTURBED, no continuation needed): every program
precise (half-widths 0.34%-0.93%), no instruction control event; lane 1.00032
[0.99895, 1.00203] inside [1/1.005018, 1.005018]. Qualify job (no timing,
6 s) replayed the A2, seed, reused v3 inputs and the A/A, and wrote v4 with
the seed's bands unchanged (table above). All v3 case bands, selection-lane
and portfolio ceilings, host and precision records are byte-equal in v4.

The lane-event rule and the held-out rule are selected by v4's
`held_out_rule`; v2/v3 plans keep their selection bands but, after this
publication, every screen (`layout_iteration`) and decision (`decide`) uses
v4's held-out gate and lane-event rule and records `held_out_policy_sha256`.
