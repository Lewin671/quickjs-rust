# T057: Development decision policy qualification

- Status: active; calibration contract frozen before development policy timing.
- Implementation: development collectors, sealed formal bundles, control/target
  rules, continuation and additive qualification are committed through 9cb73da7;
  no policy v3 is published and no runtime code changed.
- Verified at: tooling `9cb73da7`, 2026-10-17; 586-test staged fast gate
  (83.259 s), context/file-size/whitespace and shell syntax pass; full retry queued.
- Evidence: frozen contract below; no new calibration observations yet.
- Unresolved: collectors/replay/tests, development calibration, qualification.
- Next action: obtain the reference-derived timeout manifest and dispatch
  frozen calibration after the queued full gate and matrix audit.

## Frozen calibration contract (2026-10-17, before timing)

Lead authorization: workspace `tmp/string-index-cost/feedback-t057-1.md`.
Scope: generic benchmark tooling, tests, policy v3, runbooks and task records;
no runtime, interpreter-core, property/value-word, dependency or third_party
change. No project push/merge. Original [T057](T057-string-code-units.md)
resumes only after this prerequisite qualifies. Existing v2 bytes stay frozen
and remain usable for plans without development targets.

Source image is qualified v2's canonical `4c9f959e4ae3f508a10828a46cf59ec042d7ec13`,
SHA-256 `c7f0f66a519181b214bdc5ca6404cf428f0d18cbd1ab1d0e26386097619b949e`.
Use its audited six variants (natural/16/64/240/512/1008 NOP bytes), or rebuild
and require the exact natural hash plus zero body/order/uniform-shift drift.
Runtime/build diff to `351004f3` is empty. Freeze all paths once, with equal
filesystem byte lengths, distinct roles, separate launch preflight, all
process samples/costs retained. No winning placement is selected.

1. **A2 extension:** all ten pinned development programs, five paired
   whole-process samples per case for each of six placements. Base is always
   the natural image; candidate is placement k (natural/natural at k=0).
   Alternate role order continuously per case within each packet. Record
   raw paired ratios and role costs. Existing 76-case A2 observations remain
   frozen; only their estimator inputs are reused, never their samples as
   new joint measurements. No held-out profiling or selection.
2. **Freeze new bands from A2 before longer A/A:** per development case,
   B_i=max(six-layout spread of same-screen role medians normalized to v0,
   spread of paired medians with v0=1, global reciprocal natural A/A median
   bias). Include the existing A2 global bias as a floor for new cases;
   leave all original 76 B_i/I_i unchanged. I_i rounds up to one basis point
   the max of reciprocal natural A/A instruction pairs and global median
   instruction bias (including the existing floor). New development lane
   ceiling uses max(six-layout geomean spread, reciprocal natural A/A lane
   median bias), exactly the A2 estimator. The new 86-case portfolio ceiling
   uses these same estimators over all 86 measured case-estimator inputs;
   this is an empirical envelope, not a pooled/bootstrap timing result.
   Never widen either ceiling after observing longer qualification.
3. **Screen qualification:** 30 alternating natural/natural pairs for each
   development program, exact qualified image. The pilot stays immutable;
   automatically append the precision-only allocation, then at most 240.
   Per-case statistic is median paired ratios; lane is equal-case geomean.
   Use seed-0 20,000-draw 95% joint bootstrap and <=3% relative half-width.
   A/A instruction guard is max(1.01,I_i), no all-pair excess; development
   lane interval must fit its frozen reciprocal envelope. Keep its original
   30-pair joint cohort for lane statistics. Do not fabricate a joint screen
   cohort by mixing the original 76-case calibration with this new run.
4. **Canonical formal qualification:** new sealed 30-block workers-4 bundle,
   candidate=base=`351004f386f4c2c1d870467ae6e1e481952a330c`, all 86 selection
   cases and pinned NG, byte-identical two Rust roles. It includes all four
   selection lanes under the existing whole-process/increment protocols.
   Require complete identities, raw replay, correctness, health, linearity,
   unchanged original lane ceilings, frozen new development/portfolio
   ceilings and the same precision. Automatic formal A/A continuation keeps
   original argv/images/iterations/absolute role rotation and every pilot
   observation, reaches predicted count then cap; joint lane statistics keep
   the initial 30 complete blocks. No favorable whole-bundle precision rerun.

Development timeouts use the uniform reference rule max(60 seconds, 4 x
pinned NG median), derived before policy calibration from the latest valid
complete development reference run (queued baseline follow-up
`20261017T074651Z-t057-stage1-evidence-fresh`). Freeze source raw/report hashes,
all medians and derived thresholds in the development manifest; no candidate
or later observed time may change a threshold. Existing manifest v1 remains
readable for its frozen preliminary baseline. Held-out protocol is unchanged.

Queue-only timing, quieting/service-state attestation, pair-level load-only
recovery and caps are the v2 contracts. Preserve invalid attempts/rounds;
formal disturbed jobs/segments are replaced whole, at most three load-only
replacements. Non-load failures stop, and no samples are deleted or imputed.
At 240, still-imprecise case bands become diagnostic and force K=6 for
every unit including that case; payoff/all-pairs/instruction/lane/correctness
gates remain binding. A valid qualification outside frozen lane envelopes
does not authorize widening or another favorable attempt.

Publish v3 only after complete replay and all applicable qualification gates
pass. Preserve v2 held-out bands, precision and source evidence unchanged;
held-out remains acceptance-only and cannot appear in priority/targets or
profile evidence. New plans bind exact policy/manifest hashes. Every
non-target development case follows the external control rules, and the
development lane and 86-case portfolio ceilings bind each layout and formal
decision. A v2 plan cannot name a development target.

## Verification obligations

Tests cover development target and control admission, instruction floor and
hard cap, six-layout wall persistence, independent lane/portfolio ceilings,
sealed raw/manifest/host/image replay, formal continuation of npm checksums,
v2 compatibility, disjoint development/held-out programs and held-out target
refusal. Fast/full gates and qualified host-policy loading precede handoff.
Record every job, revision, artifact hash, count and qualification outcome
before closing this task; no runtime promotion is claimed by calibration.

## Tooling checkpoint

Calibration plan commit `1b61d515` precedes all v3 timing. Tooling
`2368e6498c9c188e9cfa21072f58e86e45681135` adds development collection to the
existing worker pool, seal schema 2 with twelve files, automatic sealed
development queue selection, policy-derived controls/lanes, exact npm checksum
continuation and additive A2/qualification collectors. Legacy seal schema 1
and policy v2 remain readable. Development manifest schema 2 supports uniform
NG-derived timeouts; schema 1 remains for the frozen reference run. No runtime,
held-out protocol or dependency change.

Staged fast gate: 583 tests / 72.308 s, all pass. Thirteen focused tests
include both formal schedules, the shared workers-4 pool, target/controls,
hard cap, wall persistence, lane ceiling, v2 and held-out boundaries,
qualification envelope refusal and timeout derivation. The hosted implementation
file inventory/hash is updated with its new transitive tools.

Queued via the workspace dispatcher at this tooling revision (no timing):

| Job | Obligation |
| --- | --- |
| `20261017T080252Z-t057-development-tooling-full-check` | Full check plus explicit enumeration/check of every frozen plan (runner lacks rg). |
| `20261017T080252Z-t057-development-qualified-matrix` | Fresh six-variant build/audit of frozen qualified source; require exact c7f0f66a natural image. |
| `20261017T080253Z-t057-development-timeout-reference` | Validate preliminary development raw and freeze all NG-derived thresholds before calibration. |

These jobs are pending, not passing results. The earlier current-main
baseline and development/profile job precede them in the shared queue.

## Pre-calibration preparation amendment

Before any v3 calibration timing, the initial profile/development job
`20261017T074651Z-t057-stage1-evidence-fresh` was cancelled: manifest ordering
sampled diff before papaparse and its 100 complete passes were excessive
(17 checksums observed while waiting). Preserve that partial diagnostic and
its raw sample; it did not reach development baseline measurement and supplies
no qualification observation. The child PID was verified absent after queue
cancellation. No formal baseline or completed measurement was discarded.

Replacement `20261017T082306Z-t057-stage1-profiles-and-development30` explicitly
orders papaparse first (2 passes), then diff (3 passes), retaining 100 passes
only for the shorter suite profiles. All three-engine development cases still
use the unchanged 30-block whole-process protocol. Timeout-reference job
`20261017T082337Z-t057-development-timeout-reference-fresh` replaces its
cancelled predecessor and will use this latest complete NG run. The timeout
formula, corpus, A2, qualification and power budgets are unchanged.

Full-check jobs at 2368e649/12b4ab47 were cancelled while queued as metadata
and boundary checks advanced; current full gate is
`20261017T081521Z-t057-development-tooling-full-final` at 353e9b76. No v3 timing
has started. Runtime remains unchanged.

## Build and gate checkpoint

Matrix job `20261017T080252Z-t057-development-qualified-matrix` passed: all
six hashes reproduce T055's qualified-image matrix exactly. Natural is c7f0f66a;
each shifted variant audits 4201 functions, zero body/address drift and verified
NOPs. Variant 5 has section-relative +1008 and whole-text -4096 (total -3088),
as already allowed by the frozen audit; no production layout was changed.

Full job `20261017T081521Z-t057-development-tooling-full-final` at 353e9b76
failed one of 583 Python tests after passing its preceding Rust checks:
`test_new_bundle_seals_and_replays_all_development_artifacts` compared
`/private/var/...` with `/var/...` without normalizing the expected temp path.
The assertion is fixed at 9cb73da7. Retry
`20261017T084357Z-t057-development-tooling-full-path-fixed` additionally checks
the parent's qualified-image receipt and explicitly enumerates frozen plans.
No full pass or Test262 result is claimed yet for this tooling delta.

The final local gate has 586 passing tests, including both A2 estimators,
source/seed hash binding and v2 control scope when newer bundle metadata exists.
