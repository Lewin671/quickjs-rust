# T055: Layout noise control — B pair-level qualification

- Status: blocked; canonical timeout plus valid component precision failure.
- Implementation: present; versioned host policy v1 is unqualified and cannot
  be used for a new promotion. No runtime engine change.
- Verified at: 5d255e5e staged fast gate, 2026-10-16; 531 benchmark tests,
  fmt/clippy/context/shell pass. Full iMac gate passes (167 s). Prior builder
  f2fb8067 six-variant audit, 5170 subset and 248 NG fixtures pass.
- Evidence: [B frozen plans and log](archive/T055-phase-b-plan-and-log.md),
  [A2 data](archive/T055-phase-a2-data.md),
  [unqualified policy v1](../benchmarks/policies/imac-x86_64-rust1.95-fat16-layout-v1.json).
- Unresolved: recursive_call_tree A/A half-width 5.9945% exceeds 3%; canonical
  Gaussian blur candidate block 27 times out at 15 s. Neither is load-invalid.
- Next action: lead resolves/freezes variance and canonical timeout protocols
  before new qualification. No policy adoption or favorable rerun now.

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


## Feedback-1 execution record

Pair-recovery implementation: 5d255e5e. Staged fast gate: 531 tests pass;
focused overlap/guard/cap/conservation tests pass. Host readback initially
showed indexing enabled, then disabled before any new timing. Both approved
photo/media service disable flags are true. Full no-timing gate job:
20261017T022339Z-decision-rule-b-pair-full-check passes, including 5170 subset
cases. Qualification additionally requires a sealed formal load log; its
focused guard and updated fast gate pass (532 tests). New timing collectors
remain frozen at 5d255e5e, never the older collectors or invalid samples.

| Job | Plan | State |
| --- | --- | --- |
| 20261017T022615Z-decision-rule-b-aa30-pairs | 76x30, pair recovery | exit 0 / 3690 s |
| 20261017T022616Z-decision-rule-b-heldout30-pairs | 9x30, pair recovery | exit 0 / 3181 s |
| 20261017T022617Z-decision-rule-b-formal30-whole | canonical 30 blocks, workers-4 | exit 0 / 1361 s; incomplete external |


The new portfolio collector completes all 76 cases: 2280 attempts, 2280 valid,
0 invalid, 0 heavy load samples; 2 light intruder rows remain below threshold.
Every case has 30 valid pairs; collector raw/load conservation passes.
This establishes collection integrity only; statistical qualification awaits
all three new inputs. No old observations or bands have been recomputed.


Held-out collection also completes: all 9 programs, 270 attempts, 270 valid,
0 invalid, 0 heavy samples (6 light intruder rows). Every program has exactly
30 valid pairs and matching expected checksums. Statistical ceilings remain
pending until the canonical formal input completes and qualification replays
all raw data. No replacement attempt has been spent in this amendment.


## Canonical input blocker (feedback-1 stop boundary)

The canonical job has zero heavy intruder samples. Broad (25 cases) and
sentinel (6) have 30 valid blocks, passing input/linearity/precision health.
External has one non-ok measurement: kraken-1.1/imaging-gaussian-blur,
candidate block 27 (human block 28), duration 15021374156 ns, exit -9,
status timeout at the frozen 15-second case limit. All three engine
capabilities are ok; candidate/base and candidate/NG comparisons are missing
for this case because the complete paired input is absent. The entire formal
input remains ineligible; do not omit/impute/retry that observation or waive
health. It is not a load-invalid replacement. No replacement budget was spent.

Qualification job 20261017T025048Z-decision-rule-b-qualify-pairs-v2 at a03b984a
fails closed (exit 1 / 36 s): `formal external layout input invalid or on a
different host`; the inspected reason is the timeout, not host identity.
No v2 policy was generated. The v1 unqualified artifact remains untouched.
A diagnostic-only analysis of the two valid component calibrations will be
recorded below; it cannot substitute for canonical qualification or promotion.


## Valid component diagnostics (not a policy)

Job 20261017T044845Z-decision-rule-b-component-diagnostics at a320b65f
passes raw sample/load replay and source/host binding (exit 0 / 23 s).
It uses the exact frozen 20,000-draw seed-0 bootstrap, without canonical
statistics or any omission/imputation of the formal timeout. Its artifact is
`quickjs-layout-calibration-diagnostic`, qualified/claim_eligible false.

One of 76 screen cases fails precision: sentinel/recursive_call_tree has
ratio 0.9984805608, 95% CI [0.9420116061, 1.0468012246], relative half-width
0.0599450733 >0.03. All 30 pairs remain valid; this is a valid statistical
failure, not a load-invalid replacement. No favorable repeat, 60-pair
extension, outlier deletion, band widening or precision waiver is authorized.
No other component case fails the frozen checks. All component lane CIs fit
existing A2 envelopes; all 9 held-out program/lane precision checks pass.

| Component lane | Ratio | 95% interval | Existing or diagnostic ceiling |
| --- | ---: | --- | ---: |
| broad | 1.001537836 | [0.999739931, 1.002973665] | 1.012949330 |
| sentinel | 1.000153930 | [0.990211861, 1.008485743] | 1.015687002 |
| external | 0.999664706 | [0.998456165, 1.000627944] | 1.007380238 |
| portfolio | 1.000319118 | [0.999014938, 1.001381464] | 1.008262332 |
| held-out | 1.001143444 | [0.999443171, 1.002899398] | 1.0029 (diagnostic only) |

Held-out program diagnostic bands (rounded-up reciprocal CI envelope):
acorn/parse 1.0088; esprima/parse 1.0030; marked/render 1.0076;
mustache/render 1.0100; handlebars/render 1.0066; crypto-js/hash 1.0053;
d3-array/aggregate 1.0042; ramda/pipeline 1.0065; decimal.js/arith 1.0038.
They are measured component results, not adopted promotion ceilings. All
A2 case/lane policy values and the frozen unqualified v1 artifact are unchanged.
T055 cannot close as adopted; both blockers need a separately authorized plan.
