# T055 phase B: Frozen plans and blocked qualification log

- Status: historical-campaign; this attempt produced tools, not an adopted rule.
- Implementation: present; host policy v1 is explicitly unqualified.
- Verified at: 3a66cda9, 2026-10-16; full gate passes on iMac (166 s).
  Builder/auditor f2fb8067 passes its fresh six-variant/correctness job.
- Evidence: frozen plans/jobs below, [A2 data](T055-phase-a2-data.md),
  [policy v1](../../benchmarks/policies/imac-x86_64-rust1.95-fat16-layout-v1.json).
- Unresolved: no valid B calibration; three complete timing jobs were DISTURBED.
- Next action: see active [T055](../T055-layout-noise-control.md) for the lead decision.

This log owns the frozen B plans and observations at the revisions named.
The active task owns current status/action; T022 owns rules, and the workflow
owns commands. Invalid timing/counter observations never enter qualification.

## Frozen phase B qualification (2026-10-16, before timing)

Lead-authorized brief: workspace tmp/decision-rule-b/brief.md. Scope is tools,
policy and documentation only; no engine changes, push or merge. Adopt the
proposal with the lead's 1% instruction floor / 3% hard cap, mandatory held-out
promotion and host-bound policy. Historical rejected budgets stay closed.

Calibration source is the already audited natural A2 executable 153c1fe5,
SHA-256 926ee634ca3901b89c778478702a4fe993288ef7e8e69cd93c238c694595aa85.
Two separately queued `run --timing` jobs use layout_calibration: portfolio
(76 cases) then held-out (all 9). Each has exactly 30 alternating paired
blocks per case, unchanged screen calibration/increment/whole-process
protocol, timeout 120 s, positive costs, internal increments >=50 ms.
Each creates two distinct equal-length immutable executable paths once,
preflights both, retains every sample and separates calibration. Invalid or
DISTURBED jobs are retained and replaced entirely, never pooled.

Decision statistic: median of paired ratios per case, equal-case geomean
per lane and portfolio. Qualification uses deterministic joint-block
percentile bootstrap, 20,000 draws, seed 0, 95% interval and <=3% relative
half-width, matching the existing formal precision limit. Require every
case/lane precise, no all-pair instruction excess above max(1.01,I_i), and
lane interval within the proposed A2 lane ceilings; do not widen these
ceilings to accommodate a failed qualification. Case B_i remains the A2
maximum of both layout estimators and global A/A median bias. Longer A/A
qualifies this envelope, rather than enlarging it. Held-out per-program
bands and lane ceiling are the rounded-up reciprocal 95% A/A interval
endpoints (one basis point), with a 1.0 minimum. It is acceptance-only.
If qualification fails, record the exact failure and stop adoption for a
lead decision; no favorable repeat of a valid failed qualification.

The 30-pair increment qualification above additionally requalifies the screen
instruction guard. Before any canonical formal timing, freeze a separate
30-block three-engine A/A bundle: candidate=base=4c9f959e, `perf-compare.sh
--blocks 30 --workers 4`, exact current canonical recipe. This revision includes the RegExp correction e0cf202a after A2; each formal
role must use the same current-main image, not the older A2 image. Snapshot role/hash prefixes use a fixed width, preserving the role-dash
prefix so all three binary paths are distinct and equal length;
files are exclusively created once. This is a protocol correction, not a
binary replacement sampler. Keep all existing formal health/linearity and
<=3% CI requirements. Recompute geomean(case paired medians) and its joint
20,000-draw 95% interval from the sealed raw data and require the frozen A2
lane envelopes; no widening or precision waiver. Held-out remains its own
30-pair whole-process qualification. Record the runner's stable hostname as
an additional host attestation; same model/specifications on another host
do not authorize reuse.


The initially queued canonical job 20261016T190012Z-decision-rule-b-formal-aa30
was cancelled before timing: underscore padding before the role-dash would
be misclassified by the site's disturbance monitor. The corrected equal-
length prefix keeps `candidate-`/`base-`/`quickjs-ng-`; the same 30-block
plan is requeued, with no valid observation discarded or pooled.


Intermediate queue records (superseded/cancelled references are history; final successful gates are recorded below):

B calibration jobs: 20261016T184617Z-decision-rule-b-aa30 (screen guard),
20261016T184621Z-decision-rule-b-heldout30, and replacement canonical job
20261016T191531Z-decision-rule-b-formal-aa30-fixed. Builder/audit/correctness:
20261016T191531Z-decision-rule-b-builder-audit-fixed (no timing). The older
190030Z builder job was cancelled while queued so the corrected producer
would be tested. Tool implementation through 5953689d passed the staged
fast gate: context, Rust fmt/clippy, 518 benchmark tests and shell syntax;
no runtime/parser/lexer semantic slice matched these tool-only changes.


The first 30-pair screen completed (3052 s, exit 0) but the queue marked
DISTURBED: one >=200% CPU intrusion sample. Its timing/counter observations
are wholly ineligible. Retain the full job; replace it with
20261016T193947Z-decision-rule-b-aa30-replacement1-frozen at the identical
c35ed86c collector. A queued current-harness replacement was cancelled
before execution to keep both qualification lanes on the frozen harness.
The seed's host binding uses only non-timing identity from the discarded
job, cross-checked against recorded A2 node/CPU/OS/Rust/Cargo/recipe; the
replacement must match that identity exactly. No statistics are reused.


The held-out run also completed (3113 s, exit 0) but is wholly ineligible:
searchpartyuseragent reached 201.6% CPU at 20:23:13 UTC. Replacement:
20261016T203135Z-decision-rule-b-heldout30-replacement1-frozen (c35ed86c).
The guarded qualification job is now 20261016T203215Z-decision-rule-b-qualification-replacement;
full gate at b9e47b19 is 20261016T195940Z-decision-rule-b-full-gate-complete.

## Complete timing attempts (all ineligible)

| Job (20261016 prefix) | Measurement | Exit / queue seconds | Disturbance |
| --- | --- | --- | --- |
| T184617Z-decision-rule-b-aa30 | 76 cases x 30 pairs | 0 / 3052 | XProtectRemediatorMRTv3, 245.6% at 19:18:41 UTC |
| T184621Z-decision-rule-b-heldout30 | 9 programs x 30 pairs | 0 / 3113 | searchpartyuseragent, 201.6% at 20:23:13 UTC |
| T191531Z-decision-rule-b-formal-aa30-fixed | canonical 30 blocks, workers-4 | 0 / 1424 | searchpartyuseragent, 204.0% at 20:51:38 UTC |

All were run through the workspace queue with `--timing`. Full output/logs
are retained under workspace outputs/imac-bench/<job>. No observations are
pooled, dropped selectively or used to derive B ceilings. Raw conservation
replay passes 76x30 and 9x30 as an integrity/functional check only.

After the third disturbed run, queued whole replacements T193947Z (screen)
and T203135Z (held-out), plus guarded qualification T203215Z, were cancelled
before execution. This follows the brief's stop-for-lead-decision boundary.
No system service or disturbance threshold was changed.

## Source identity correction

A2 screen/held-out image: source 153c1fe5, SHA-256
926ee634ca3901b89c778478702a4fe993288ef7e8e69cd93c238c694595aa85.
Canonical formal/build source: 4c9f959e, natural SHA-256
c7f0f66a519181b214bdc5ca6404cf428f0d18cbd1ab1d0e26386097619b949e.
The initial claim that these Rust inputs were unchanged was wrong:
e0cf202a updates RegExp validation and tests. Formal A/A roles must share
their frozen revision and image; different revisions need not share a hash.
The checker and regression coverage were corrected before any B data was accepted.
No engine source was changed by this branch.

## Static builder verification

Initial builder T191531Z failed its absolute-shift check on variant 5.
Diagnostic T210631Z showed all 4201 functions/body/order unchanged: `__text`
moved -4096, while section-relative padding stayed 1008 and total function
shift was -3088. All declared NOP bytes matched. The corrected audit checks
section movement and relative padding separately, while keeping exact body,
order and uniformity checks. Regression tests cover this geometry and bad bytes.

Fresh public builder job 20261016T211407Z-decision-rule-b-builder-audit-final
at f2fb8067 passes: exit 0 / 427 s, six variants, 5170 optimized Test262
subset cases and 248 QuickJS-NG fixtures on the exact natural image.
Metadata copy job 20261016T212209Z-decision-rule-b-builder-evidence preserves
its identity. Every shifted variant audits all 4201 original functions,
with zero body/address drift and verified NOP bytes.

| Variant | NOP bytes | Text shift | Total function shift | SHA-256 |
| --- | ---: | ---: | ---: | --- |
| 0 | 0 | 0 | 0 | `c7f0f66a519181b214bdc5ca6404cf428f0d18cbd1ab1d0e26386097619b949e` |
| 1 | 16 | 0 | 16 | `18767d0f3ff9071158e34077ba429c27694f34a36074973cdb20d9abee181ae6` |
| 2 | 64 | 0 | 64 | `8f7bb0b500f736802b5ef14a9c9c9be01e25834a17d0485bac33182b63dc4699` |
| 3 | 240 | 0 | 240 | `f9b48ec6c1bcbdc918f275bf7d19a8ca8e9a68c7ac1c6dc46a41d03bc4088740` |
| 4 | 512 | 0 | 512 | `9f728ef01f2b6516307cb16559f9546bec47e942dbdc287e3a6bebdbc53ff433` |
| 5 | 1008 | -4096 | -3088 | `f795cbcfbbe3884fe82740f94da28678ad859ff7e35747ba75a63cb4cd58e11b` |

## Gates and unqualified artifact

At f2fb8067 the staged fast gate passes context, Rust fmt/clippy, 523 benchmark
unit tests and shell syntax. Final `check-context.py` and
`check-touched.sh --base main --explain` also pass (523 tool tests, fmt/clippy,
context and shell syntax); the final record-only commit has its own staged gate. Full job 20261016T211830Z-decision-rule-b-final-check
at 3a66cda9 passes (exit 0 / 166 s), including the 5170-case subset. The branch
changes no runtime/parser/lexer semantics, so no focused semantic slice matches.

Policy v1 retains valid historical A2 bands only: broad 1.012949, sentinel
1.015687, external 1.007380, portfolio 1.008262 (rounded display). All B
held-out program/lane budgets are null. `qualified: false` is enforced by
load_policy; no promotion can use it. Its SHA-256 sidecar verifies:
d5e5c4eca9ab8cccf602a989603b19f4955f3e314e3c23cf9663bca5851da4b2.

Diagnostic final-tool replay: T053 has one control event and four wall events,
T054 two wall events; neither has a direct rejection under the amended rule,
and both require missing K=6 evidence. Their point target/lane gates fit.
These are unqualified diagnostic replays, never promotion or extra attempts;
original rejected standings/budgets stay unchanged.
