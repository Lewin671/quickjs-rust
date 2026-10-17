# T055: Layout noise control — B pair-level qualification

- Status: active; feedback-1 authorizes pair-level disturbance recovery.
- Implementation: present; versioned host policy v1 is unqualified and cannot
  be used for a new promotion. No runtime engine change.
- Verified at: 3a66cda9, 2026-10-16; iMac full gate passes (166 s), 523 benchmark
  tests. Base-main fast/context checks pass. Fresh six-variant build/audit,
  5170 subset and 248 NG fixtures pass.
- Evidence: [B frozen plans and log](archive/T055-phase-b-plan-and-log.md),
  [A2 data](archive/T055-phase-a2-data.md),
  [unqualified policy v1](../benchmarks/policies/imac-x86_64-rust1.95-fat16-layout-v1.json).
- Unresolved: no valid B qualification yet; Spotlight was still enabled at
  the pre-implementation check. Photo/media analysis disable flags confirmed.
- Next action: implement/test the frozen recovery protocol below, recheck
  indexing, then queue fresh 76x30, 9x30 and canonical 30-block calibrations.

## Scope and boundary

The lead's phase B brief authorizes the layout-robust tool implementation and
calibration, and explicitly says to stop and report when a lead decision is
needed. The repeated shared-host interference reached that boundary. The task
is not closed/archived as adopted, and no valid failure was favorably rerun.

The rule lives in [T022](T022-performance-priority-controller.md#layout-robust-decisions),
procedure in [the workflow](../docs/performance-workflow.md#screen-gate),
and schemas in [artifact contracts](../docs/layout-policy-artifacts.md).
The linked B log owns frozen plans, job identities, exact binaries, structural
audit correction and gate results. Invalid observations remain retained but
ineligible; held-out remains acceptance-only and its budgets remain unknown.

## Resume requirements

Keep the source/host/recipe identities and total measurement/attempt budget
frozen before timing. For the original attempts retain/replace DISTURBED runs whole. New
attempts follow the frozen amendment below; never pool independent jobs. All formal precision/correctness/identity gates still apply.
The first policy is intentionally unqualified; do not edit it into acceptance.
T053/T054 rejected standings and exhausted budgets are unchanged.


## Frozen feedback-1 amendment (2026-10-16, before new timing)

Authorization: workspace tmp/decision-rule-b/feedback-1.md. Keep all source,
recipe, binaries, pair counts, A2 envelopes, qualification statistics and
valid-failure stopping rules in the frozen B log. Prior invalid runs stay invalid.
New screen/panel/held-out collectors use the existing queue intruders.log:
5-second samples, individual >=30% process inclusion, summed CPU >=200%
per UTC second. Do not change monitoring, thresholds or process exemptions.
Record epoch start/end around every measured process. A pair's window spans
its first process start through last process end, including intervening gaps.
Invalidate iff that window expanded by 2 seconds intersects a heavy sample's
one-second timestamp bucket (conservative timestamp quantization). Load alone
selects pairs; retain all raw values and invalid reasons before calculating
accepted costs. Nonpositive/short valid increments still fail, never trigger
value-based replacement. Iteration calibration remains separate and frozen.

Measure the original count first, wait 8 seconds for monitor/guard completion,
then append a round containing exactly the missing count. Continue alternating
base/candidate-first by cumulative attempt index, without resetting on retries.
All rounds/samples survive in the packet; exactly the first required valid pairs
enter statistics, in acquisition order. Fail the whole run if invalid attempts
exceed 20% after a completed round (per case and whole run), or any consecutive
heavy-sample cluster spans >=60 seconds (adjacent sample gaps <=6 seconds).
Retain failed packets. No within-job or cross-job cherry-picking; no automatic
retry of a statistically valid failed qualification. At most three whole-run
replacements per calibration, only for load-invalid runs; then stop/report.

Apply this protocol to all layout-rule screens, six-layout panels and held-out
collectors, including calibration. Legacy screen artifacts retain their original
protocol. Canonical perf-compare uses multi-worker, three-engine calibrated
blocks without a sealed pair-window/retry contract; extending it would change
its independently qualified scheduler. Keep formal 30-block A/A whole-run
replacement on any heavy sample (maximum three load-invalid replacements),
not partial filtering or pooling. Existing formal health and precision gates
remain mandatory. Queue warning is retained for pair-recovered jobs; acceptance
requires raw load replay, caps and complete valid counts, not the warning alone.

Before every timing job record mdutil -s / and disabled photo/media service
flags through the queue. Indexing must be disabled before starting timing;
otherwise stop without measurements. Qualification publishes a new v2 artifact;
v1 and the original B log remain immutable. Document actual jobs and decisions
in this active task before archiving the completed outcome.
