# T061: Queue-only optimization triage

- Status: closed-landed.
- Implementation: present; tooling only, no runtime or frozen-plan changes.
- Verified at: ef7626fc (2026-10-19 UTC), full `scripts/check.sh` and the queue run below; final documentation-only closure passed context and staged fast gates.
- Evidence: this task; raw queue artifacts are local and not retained in Git.
- Unresolved: none for tooling. T058 triage is unclear; its acceptance standing is unchanged.
- Next action: use this diagnostic before new expensive acceptance; preserve every frozen plan and resolve T058 only under its existing acceptance rules.

## Scope and design

Base `378f1ac0`, branch `agent/quick-triage/codex`; no merge or public push.
Own `tools/benchmark/{quick_triage,triage_build}.py`, focused tests,
`scripts/quick-triage.sh`, script index, performance workflow and this task/index.
No engine, dependencies, third_party or frozen performance-unit changes.
The workflow owns permitted decisions/accounting. The triage command seals a
request before building and emits a distinct diagnostic artifact; it never
calls the plan evaluator or emits a panel/formal bundle. Existing lossless
pair recording, checksums, load caps/replay and workers-4 ownership are reused.
Two canonical committed builds share Cargo dependencies and identity-checked
binary caches; temporary source worktrees are removed even on failure.
Only natural placement is built; no six-layout audit/promotion claim is made.

## Prospective validation contract (before timing)

One timing job, after checking queue status; base
`351004f386f4c2c1d870467ae6e1e481952a330c`, candidate T058 branch tip
`agent/t058-regexp-lazy/claude` resolved at submission. Its sole declared target
is `development/papaparse/csv`. No changes to T058's v3/v4 acceptance or standing.
Fixed v1 controls are in `quick_triage.py`: six sentinels plus 13 controls.
M1.2's recorded 5.21x JSON, 3.51x class-fields and 3.07x trees motivate those
three external cases; JSON parse also guards construction. Churn and dynamic
reads exercise generic access; allocation/calls/strings guard common paths;
lodash/diff/moment add real development consumers. Audio DFT and CDjs catch
known placement-sensitive costs (T038/T058). Held-out is never consulted.
Selection is an engineering diagnostic, not measured regression-detection coverage.

Five pairs, exact workloads, v5 workers-4, no optional subsets or early looks.
Exploratory seed-0 20,000-draw case-median 95% intervals. Likely loss if any
wall lower endpoint exceeds its v5 band, or instruction lower exceeds
max(1.03,I_i). Likely win requires all target wall upper endpoints below
reciprocal B_i and no case upper endpoint above those loss limits; else unclear.
No calibrated error-rate, multiple-testing adjustment or layout protection.
A failure/over-budget run is retained as unclear; no favorable automatic retry.
30-minute goal includes queue quieting/builds; queued wait is separate.
Record every job, request/result hashes, stages, verdict and workload limits.
Full acceptance cost is reported from T060/Q1 evidence, explicitly estimated
where the held-out speedup is extrapolated.

## Verification

At ef7626fc, staged fast gate passed; focused triage tests 4/4; full
`PYTHONDONTWRITEBYTECODE=1 ./scripts/check.sh` passed: Rust runtime 2,306,
CLI 13, lexer 63, parser 274, Unicode 59, integration 7; agents 21,
perf-counters 24; benchmark tools 651, script tests 35; all 5,170 curated
Test262 cases. All clippy/format/plan/context/size/hook/syntax gates passed.
An initial full-gate invocation stopped at Test262 with
`error: Test262 allowlist validation failed`: fresh worktree submodules were
uninitialized. `scripts/bootstrap.sh` checked out both pinned top-level
submodules (no recursive QuickJS-NG modules); the complete gate then passed.
No engine paths changed, so no focused semantic Test262 slice matched;
the full curated subset still ran. Manual `bash -n scripts/quick-triage.sh`
and CLI `--help` passed. Final edits after the full gate only close this task/index.

## Full acceptance cost on v5 (planning estimate, not a shortened protocol)

T060 measured selection natural 742 s (12.4 min), K=6 total 4,457 s
(74.3 min). Its 2.55x parallel speedup applied to Q1's T058 held-out
natural 3,204 s and completion 15,898-15,972 s estimates about 125 min
for the full held-out panel. That held-out duration is extrapolated, not
measured by this task. T058/T057 formal30 already uses workers-4 and costs
46-58 min; precision continuation 36-53 s and decision/replay about 2 min.
Thus both panels plus formal/decision cost about 4.1-4.3 h per candidate,
excluding builds, quieting, queued waiting, disturbances and retries. With
neither panel required, about 1.4-1.6 h. Exact correctness adds 28-29 min
of iMac occupancy if not performed locally. Existing serial plans retain
their schedule; triage does not convert T058's acceptance to v5.

Sources: T060 speed table; T058 queue jobs `20261018T080835Z-t058-formal30`,
`20261018T080836Z-t058-heldout30`, `20261019T033833Z-t058-formal-bitops-continuation`,
`20261019T033847Z-t058-heldout-k6-complete`; T057 formal/correctness timing
summarized in the supplied Q1 analysis. These are service costs, not queue-delay estimates.

## Completed end-to-end validation

One queue job `20261019T132248Z-t061-t058-triage`, tooling ef7626fc;
exit 0, no other queued/running job at submission, no automatic rerun.
Dispatch/commit transfer took 0.80 s. Raw artifacts were fetched to workspace
`outputs/imac-bench/20261019T132248Z-t061-t058-triage/out/triage/`
(local artifacts, not retained). This file preserves all conclusions/numbers.
Base `351004f386f4c2c1d870467ae6e1e481952a330c`; candidate
`0f61bbab640bc535edcf1037ee82df26f7c7f28b` (T058 branch tip at submission).
Both executable-cache lookups missed; Cargo dependencies were reused.
The canonical builds reproduced T058's original natural images:

- base: `c7f0f66a519181b214bdc5ca6404cf428f0d18cbd1ab1d0e26386097619b949e`.
- candidate: `d2bc9b8d755bcb70ebb02d8a3e0bde21d30d89add36dd66cbea64cebae72b7c8`.

- request.json SHA-256: `aec43ecdb3f3f873880559b98a01a31560a1b2bf1f530c376a882b6676b14d6a`.
- triage.json SHA-256: `2417a63aa2c74bee83522484ed24066cabce7fc3dec4ca3140bdbea2f9cd09cb`.
- triage.md SHA-256: `6d23d3218faae9e1112b4cacc1545e2488cddd7781eb60b2839b9499a169f8eb`.

| Stage | Wall seconds |
| --- | ---: |
| queue_quieting | 62.378 |
| validate_and_freeze | 0.154 |
| build_quieting | 0.029 |
| build_base | 71.071 |
| build_candidate | 69.895 |
| prepare_workloads | 0.158 |
| settle_and_launch | 0.994 |
| collection | 325.411 |
| seal_and_replay | 8.046 |
| analysis | 2.354 |
| command_total | 478.292 |
| end_to_end | 540.632 |

Total 540.632 s = 9.01 min, including 62.378 s queue quieting and both
builds; 20 cases x 5 pairs = 100 valid / 0 invalid / 0 heavy samples.
Queue wrapper reported command exit after 480 s (quieting excluded there).
There was no waiting behind another job. All temporary source worktrees were
removed. Identity-checked executables and shared Cargo caches are intentionally
retained for reuse; no warm-cache elapsed time was measured. Initial free disk:
local 90 GiB, iMac 101 GiB. Local disposable incremental outputs were removed
at handoff. No project push or merge; only the requested runner ref transfer.

Verdict **unclear**. Papaparse wall ratio 0.1848 [0.1812, 0.1869], about
81.5% less time; instructions 0.1146, about 88.5% less work. No obvious
regressions under the prospective heuristic. Four upper wall endpoints still
cross B_i=1.050127: heterogeneous_property_read, polymorphic_call_site,
top_level_function_call and json-stringify-tinderbox. Their instruction ratios
are near 1; these data cannot rule out wall regressions or diagnose their cause.
No favorable rerun was requested, no frozen attempt was consumed, no gate was
waived, and T058 was not promoted or re-decided. The engineering result is a
large target benefit with unresolved neutrality, not a complete-portfolio claim.

All ratios are candidate/base. Five-pair 95% bootstrap intervals below are
exploratory and unadjusted; natural layout alone does not cover layout bias.

| case | wall ratio [95% exploratory interval] | instructions [95% exploratory interval] |
| --- | ---: | ---: |
| development/papaparse/csv (target) | 0.1848 [0.1812, 0.1869] | 0.1146 [0.1146, 0.1146] |
| sentinel/capturing_closure_call | 1.0188 [0.9230, 1.0229] | 1.0000 [0.9999, 1.0002] |
| sentinel/heterogeneous_property_read | 1.0463 [0.9947, 1.0591] | 1.0000 [1.0000, 1.0001] |
| sentinel/polymorphic_call_site | 1.0144 [1.0053, 1.0536] | 1.0001 [1.0000, 1.0001] |
| sentinel/prototype_method_call | 1.0324 [1.0255, 1.0449] | 1.0000 [1.0000, 1.0000] |
| sentinel/recursive_call_tree | 0.9528 [0.8138, 1.0254] | 1.0000 [0.9998, 1.0001] |
| sentinel/string_key_map_churn | 0.9409 [0.7810, 1.0058] | 0.9999 [0.9996, 1.0001] |
| broad/property_dynamic_read | 1.0212 [0.9607, 1.0266] | 0.9814 [0.9814, 0.9814] |
| broad/object_allocation | 0.9996 [0.9900, 1.0111] | 1.0000 [1.0000, 1.0000] |
| broad/string_slice | 0.9931 [0.7285, 1.0659] | 0.9999 [0.9997, 1.0002] |
| broad/top_level_function_call | 1.0363 [1.0001, 1.1902] | 1.0000 [1.0000, 1.0001] |
| development/lodash/transform | 1.0091 [0.9989, 1.0263] | 0.9868 [0.9865, 0.9872] |
| development/diff/compare | 1.0252 [1.0116, 1.0309] | 0.9963 [0.9961, 0.9963] |
| development/moment/dates | 0.9837 [0.9670, 1.0010] | 0.9961 [0.9955, 0.9965] |
| external/kraken-1.1/json-stringify-tinderbox | 1.0228 [0.9960, 1.0696] | 1.0006 [1.0003, 1.0034] |
| external/kraken-1.1/json-parse-financial | 1.0051 [0.9891, 1.0184] | 1.0012 [0.9974, 1.0018] |
| external/jetstream3-js-subset/raytrace-public-class-fields | 0.9805 [0.9682, 1.0167] | 0.9866 [0.9861, 0.9886] |
| external/sunspider-1.0/access-binary-trees | 1.0144 [0.9802, 1.0193] | 0.9917 [0.9904, 0.9918] |
| external/kraken-1.1/audio-dft | 1.0234 [1.0061, 1.0589] | 0.9998 [0.9987, 0.9999] |
| external/jetstream3-js-subset/cdjs | 1.0323 [1.0187, 1.0342] | 0.9889 [0.9884, 0.9898] |
