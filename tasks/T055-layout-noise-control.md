# T055: Layout noise control, phase B qualification

- Status: active; phase B tools present; adoption blocked after three complete
  timing runs were marked DISTURBED; replacements stopped for lead decision.
- Implementation: builder/auditor, schema-3 dual-metric decisions and mandatory
  held-out/resource gates present; no qualified policy or adoption yet.
- Verified at: b9e47b19, 2026-10-16; staged fast gate passes, including 521
  benchmark tool tests. Original 76x30 and 9x30 raw conservation replays pass;
  their timing/counter observations remain ineligible due to disturbance.
- Evidence: [A2 tables](archive/T055-phase-a2-data.md), committed replay fixtures
  under tools/benchmark/tests/fixtures/layout-policy, frozen B plan/jobs below.
- Unresolved: a quiet measurement window or separately approved protocol is needed;
  builder correction and final full gate are being completed.
- Next action: complete static/correctness gates, then report the host blocker.
  Resume timing only after the lead chooses suitable measurement conditions.

## Historical phase A

The [original frozen contract](archive/T055-phase-a-plan.md) is superseded
by the lead amendment below. Its failed samples never enter A2.

## Lead amendment and frozen phase A2 (2026-10-16)

The lead's brief-phase-a2.md supersedes phase A's sampling and R1 decision
above. R1 is dropped: mitigation not available via this toolchain on macOS;
hardware hypothesis untested. No further JCC work. The fixed-path binary-
replacement sampler is retired; do not debug or reuse it. Phase A's invalid
partial samples do not enter A2. Historical results are in the linked log.

Reuse the six already audited R0 executables (source 153c1fe5), verifying
SHA-256 against the phase A report before any measurement. Rebuild only if
the originals cannot be verified. Each retains its existing immutable,
equal-length path under the build job's out/binaries/R0v0..R0v5/qjs;
never swap or overwrite them. New repository revision is a tooling context,
not a claim that these executables were built from it.

Exactly six complete sequential screens: v0/v0 A/A, then v1/v0 through
v5/v0. Each uses T054's tools.benchmark.screen CLI, all 76 original cases
(25 broad, 6 sentinel, 45 external), five alternating pairs, unchanged
calibration, launch preflight, quiet checks, timeout and measurement lock.
Five pairs matches T054; this is a diagnostic band, not formal precision.
Use workspace scripts/imac-bench.sh run --timing for each complete screen;
retain/replace DISTURBED runs in full, never pool or remove observations.
No held-out programs are used to select or profile work in this diagnostic.

A workspace wrapper only records the existing screen.sample return values
and case boundaries outside timed subprocesses, preserving wall/instruction
costs as well as the CLI's ratios; it changes no tools or sampler decisions.
Require all five positive wall/instruction pairs per case; missing/short or
nonpositive increments are reported as invalid, never silently discarded.
Keep calibration samples separate. All raw samples, driver, identities,
CLI JSON and logs are retained. A/A must complete validly before v1 timing.

Per case report every paired wall/instruction median ratio vk/v0. For a
cross-screen layout spread normalize each role's cost by iterations (internal
only), divide each candidate's median cost by its same-screen base median,
include v0=1, and take max/min over v0..v5. Also report the literal unnormalized
six wall medians using the v0 role from A/A, flagging their cross-run drift;
these cannot separate host drift from layout. A/A width is max/min of its
five ratios, with min/max and median bias reported for both metrics. Per
lane and portfolio report geometric means of case paired-median ratios per
variant and their max/min spread; retain all cases with equal case weights.

Phase B is a proposal only at workspace tmp/layout-noise-control/
PROPOSAL-phase-b.md. Derive thresholds from A/A and the layout band, assess
T053/T054 using their recorded pairs, cover instruction-blind effects and
reserve held-out acceptance. Do not change production tools, workflow docs,
rejected budgets or acceptance policy. Commit on this branch only; no push
or merge (private queue transport refs are the existing authorized runner).

## Phase A2 results and proposed B rule (2026-10-16)

A2 was frozen at eef1a203 before timing, on repository base 09e8e0d3.
Every original R0 hash matches phase A; no rebuild or path replacement.
Screen code and its external loading/bundling functions are unchanged from
T054. Six complete 76-case x 5-pair runs: 2280 pairs, 6420 measured process
samples plus 978 separately retained calibration samples. All five wall/
instruction pairs are positive; every internal wall increment >=50 ms.
No samples removed/pooled and no DISTURBED run. Light activity remains in
v2 (one sample) and v4 (seven samples), below the queue's 200% threshold.

Queue jobs (UTC prefix 20261016; all exit 0):

- T173552Z-layout-noise-a2-aa (524 s), T174435Z-layout-noise-a2-v1 (522 s).
- T174436Z-layout-noise-a2-v2 (522 s), T174436Z-layout-noise-a2-v3 (524 s).
- T174436Z-layout-noise-a2-v4 (518 s), T174437Z-layout-noise-a2-v5 (523 s).

| Wall max/min statistic | Median | p95 | Maximum |
| --- | ---: | ---: | ---: |
| Six literal wall medians | 1.017945 | 1.053941 | 1.100113 (global_read) |
| Same-screen-v0-normalized medians | 1.018054 | 1.071217 | 1.106055 (dynamic_method_call) |
| A/A five-pair width | 1.034037 | 1.185257 | 1.738377 (captured_read) |

Same-v0 cross-run median drift reaches 1.079650 (dynamic_method_call).
These are observed layout-plus-sampling bands, not isolated causal effects
or confidence bounds. A/A maximum reciprocal median bias is 1.025025.
Instruction layout-paired medians range 0.995911..1.004090; exact 1.0000
invariance is false for some whole-process cases. Full per-case ratios,
A/A min/max/median/width, literal medians and drift are in the linked table.

Proposed diagnostic ceilings use max(layout spread, reciprocal A/A bias),
with the same geomean-of-case-medians statistic; no policy is adopted:

| Lane | Layout geomean spread / proposed ceiling |
| --- | ---: |
| Broad | 1.012949 |
| Sentinel | 1.015687 |
| External | 1.007380 |
| Portfolio (76 equally weighted cases) | 1.008262 |

Instruction controls: A/A-derived ceiling +0.21% for 71/76 cases, own A/A
pair envelope for five noisy cases (max +1.44%); reject only with all five
pairs beyond the case ceiling. Zero triggers in 380 neutral-layout cases.
Case wall bands cover both ratio-of-medians and median-of-ratios layout
spreads plus A/A median bias. Targets must beat frozen payoff and that band.
A lone wall event is recorded, then checked across fixed K=6 layouts, never a best
offset. Persistent six-of-six wall cost can reject despite flat instructions.
K=6 costs 52.22 screen minutes plus >=6 quiet minutes, before build/formal
costs. Memory/Value/allocator units require the full wall panel and resource
checks; held-out stays acceptance-only with independent calibration.

Hypothetical replay: T053 target 0.917431 passes and all lanes fit, but two
instruction controls fail (+0.242% binary-trees, +1.240% controlflow-recursive).
Both have faster recorded wall times: this is an instruction-contract failure,
not proof of wall slowdown. Four wall events also lack K=6 evidence. T054
fits all lane ceilings, but two wall events and one inconsistent cdjs
instruction alert need more evidence; it is not retained. Historical rejected
standing/budgets remain unchanged. Longer frozen A/A (proposed 30 blocks),
formal precision and held-out qualification precede production adoption.

Full-gate job: 20261016T174543Z-layout-noise-a2-full-gate. Result documents
use context/fast/whitespace/scope checks; no runtime semantics changed, so no
additional focused semantic slice is selected by the fast gate.

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
