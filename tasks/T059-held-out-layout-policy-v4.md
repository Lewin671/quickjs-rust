# T059: Held-out layout persistence and candidate formal continuation (policy v4)

- Status: active; calibration plan frozen, tooling landed, v4 timing queued.
- Implementation: present on branch `agent/policy-v4/claude` (no runtime change).
- Verified at: tooling commit, staged fast gate 629 tool tests passing.
- Evidence: frozen plan below; raw job outputs will be workspace-local under
  `outputs/imac-bench/<job>/`.
- Unresolved: v4 not yet calibrated or qualified.
- Next action: run the frozen jobs, then publish v4 and set `HELD_OUT_SUCCESSOR`.

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
B_i. A held-out lane above its ceiling likewise needs the panel and rejects
only if every layout is above it. Promotion intervals: an upper bound above
B_i or the lane ceiling needs the complete panel; imprecision stays missing.

## Candidate formal continuation

`layout_formal_continuation` accepts candidate/base bundles: it appends
blocks to every pilot-imprecise case (or a `--case` subset) under the sealed
three-role protocol and identities, by the power rule (cap 240), and marks
the chain `candidate-base`. `decide --formal-continuation` replays the
latest chain per case; case statistics use all blocks, lanes keep the
original joint 30 blocks. A precise case cannot be continued.

## Dispatch (tooling `1228aa64`, behind T056's running screen)

| Job | Frozen phase |
| --- | --- |
| `20261017T205859Z-t059-v4-heldout-a2` | Replay reused inputs, held-out A2 (6 x 5 pairs), freeze seed (timed). |
| `20261017T205900Z-t059-v4-heldout-aa30` | Fresh 30-pair held-out A/A and automatic power (timed). |
| `20261017T205901Z-t059-v4-qualify` | Replay all inputs; write v4 with its actual state (no timing). |
