# T064/T056 successor: lead decision and freeze

Lead decisions dated **2026-10-21**, feedback on draft `4a2c7993`, are enacted
with the validated [frozen schema-3 plan](../performance-units/interpreter-core-v2-kernel-layout-authority.json)
and [old-plan closure](../archive/T064-layout-edges-closure.md) in this commit.
K1 implementation and timing screens have not started; freeze is not evidence
that K or any performance gate passed. [Inputs](../archive/T064-successor-plan-inputs.md)
own the exact-M profiles, job/receipt hashes and original drafting history.

## Lead decisions (2026-10-21)

1. **Priority override approved.** K is the structural prerequisite before M1
   A/B; K and A are intermediate `advance` stages and only B can earn retention.
2. **Targets changed to moment, meriyah, hash-map; diff is a control.** Rank
   named reach under T022's required `min(.97, 1/B_i)` payoff, rather than
   selecting by reach alone. Diff's B=1.0991 requires a 0.9098 ratio: about 9.02%
   total cost reduction, or 38% of its 23.73% named owner share. The lead judges
   that payoff not credible from this reach. The replacement targets and
   recomputed exact-policy payoffs are below. **Prior stage-A measurements
   were not used for this selection**; it uses the fresh exact-M profiles and
   the v6 policy. This is selection evidence, not a predicted speedup.
3. **Quantitative manual RSS ceiling adopted.** Every target and every
   development case must have median per-process max-RSS candidate/M ≤1.05
   over its formal run resource samples. The mechanism specifies paired ratios
   and exact-image/host/cohort binding for the 11 distinct cases. Missing
   coverage blocks the decision; exceeding the ceiling blocks `advance` or
   promotion pending lead review. Tooling enforces neither this numerical cap
   nor this per-case coverage, so record a manual result before every stage
   decision, in addition to authenticated resource report/raw replay.
4. **Manual failure accounts and stricter stage-band audit accepted.** Schema
   3's aggregate cap cannot enforce the K versus shared A+B accounts. On
   diagnostic-only cases the stage evaluator substitutes its stage cap for
   the case band; record the additional stricter-band audit before advancing.
5. **Amended static contract adopted.** `scripts/check-kernel-isolation.sh`
   must implement the [T064 lead-decision amendment](../archive/T064-kernel-isolation.md#lead-decision-2026-10-21):
   pointer-sized slots inside referents compare relocated target identity and
   contents under the same rule, rather than raw pointer values. The contract
   is cited by document, not by a checker commit. K0 is being finished on
   `agent/t064-k0-checker/codex`; integration/qualification remains a prerequisite.

| Final target | Named inclusive / owner | v6 B_i | Final candidate/M maximum |
| --- | ---: | ---: | ---: |
| `development/moment/dates` | 26.13% / 19.96% | 1.050127431 | 0.952265383 |
| `development/meriyah/parse` | 33.04% / 28.10% | 1.071894924 | 0.932927265 |
| `external/jetstream3-js-subset/hash-map` | 21.69% / 19.01% | 1.050127431 | 0.952265383 |

All other 83 policy cases remain controls, including diff and all six
sentinels. Held-out was never used for selection. Retain the approved draft's
instruction-blind classification, v6 workers-4 policy, full K=6, 60-block
formal and 30-pair held-out budget. Their exact binding and the failure ledger
live in the frozen JSON; this handoff spends no acceptance attempt.

## Final validation and freeze

Validation runs locally against the fetched copy of the **same** queue and
profile root; all receipt assets are present, so no iMac job or new timing
is required. Command:

```sh
./scripts/performance-decision.sh validate-unit \
  --unit tasks/performance-units/interpreter-core-v2-kernel-layout-authority.json \
  --queue <workspace>/outputs/imac-bench/20261022T050553Z-t064-successor-exact-profiles/out/queue.json \
  --profile-root <workspace>/outputs/imac-bench/20261022T050553Z-t064-successor-exact-profiles/out/profiles
```

Exit 0, validator output:

```json
{
  "artifact_type": "quickjs-performance-unit-validation",
  "base_sha": "9eba7a1d6ae8b67397d20e0d6c93ac79037f5e30",
  "priority_mode": "override",
  "queue_sha256": "f6666a68cb8e1841418d6f5012e6f7810b0c25ebf982b45ef6d5564855e0ae13",
  "schema_version": 1,
  "status": "valid",
  "unit_id": "interpreter-core-v2-kernel-layout-authority",
  "unit_sha256": "a23cba70f9ffe45870d171f6065192eda06604befb26e2e7eae978e5c39822fb"
}
```

Plan SHA-256: `a23cba70f9ffe45870d171f6065192eda06604befb26e2e7eae978e5c39822fb`.
Validation-output SHA-256:
`5a791c13024757eb732768f0af2096fd1dadbaf68e17f34b74184e05eca386a2`.
All nine target receipts (three windows per target) pass the exact executable,
queue, opportunity and copied-asset checks. No old JSON or sealed raw receipt
was edited. The original layout-edges plan SHA-256 remains d184dbcc… and its
valid0a9af81b failure remains spent; its final performance standing is unresolved.

## Handoff

The mechanism is the full stage/static/manual-gate contract. Before each
screen check the carried failure identity, stage reserve, static/correctness
results and exact-image instruction pre-gate. Before each stage decision
record the manual ledger, stricter-band and per-case RSS checks; a valid schema
is not evidence that any of these passed.

After K integrates at I, obtain exact I profiles to confirm M1 reach before
porting A. Do not automatically port the old diagnostic shape or retarget or
replenish accounts. All cumulative comparisons retain M, including K cost;
I→A attribution is supplemental. Advance only `migration.current_stage` after
a recorded stage decision. Unrelated runtime changes or changed scope/priority
stop for lead review.

This commit enacts the freeze and closure, adds the index row and updates the
T056/T064 resume blocks. Next action is the lead's separate K0/input integration
and K1 handoff. No K1 implementation, screen, push or merge is part of this work.

## Freeze verification

At code revision `4a2c7993` plus this freeze-only documentation delta,
`scripts/check.sh` passed every stage, including the new frozen plan check,
2,335 runtime tests, 662 benchmark-tool tests, 35 script tests and all 5,170
Test262 subset cases. Full-gate log SHA-256:
`774c333a470316e1f6877a0dea5b0e65d5d9e1ebbd53b94eb99f7da0d85928af`.
The log is local, not retained in Git. `check-touched.sh --staged --explain`,
context/file-size/whitespace checks and changed archive-link checks passed;
the original receipt/workload manifest is byte-identical to4a2c7993.
No runtime, tool, recipe or dependency change was made. This worktree's
`target/debug` was deleted after the gate; profile assets and exact measured
images remain intact.
