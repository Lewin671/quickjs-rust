# T055: Layout-robust policy qualification

- Status: blocked; feedback-2 protocols complete, new canonical precision failure.
- Implementation: present; uniform timeout, retained power cohorts and cap fallback. No engine change.
- Verified at: 1600baa9 final iMac full gate, 543 tool tests and 5170 subset,
  exit0 / 165 s. Base-main fast/context pass; final record-only gate passes.
- Evidence: [feedback-1 record](archive/T055-feedback-1-qualification.md),
  [original B plan/log](archive/T055-phase-b-plan-and-log.md),
  [A2 data](archive/T055-phase-a2-data.md).
- Unresolved: new formal bitops-3bit-bits-in-byte A/A half-width 8.1347% >3%.
- Next action: lead resolves the new canonical precision blocker under a separately
  frozen plan; no favorable repeat, policy adoption or formal precision waiver.

## Frozen feedback-2 amendment (2026-10-16, before any new timing)

Authorization: workspace tmp/decision-rule-b/feedback-2.md. This supersedes
only timeout and precision allocation in the earlier frozen plans. All original
source images, host/recipe binding, A2 bands, statistic, 20,000-draw seed-0 95%
bootstrap, 3% precision limit, load-only eligibility/caps and correctness gates
stay unchanged. No candidate/base winner or payoff selects a protocol setting.
No runtime engine change, dependency, project-remote push or merge.

Timeout is a uniform hang detector: for each external case use
max(60 seconds, 4 x its pinned QuickJS-NG median in the latest fully valid
available canonical run, 20261016T122024Z-general-current-baseline). That
bundle is ready_for_unit_gates with no disturbance, 30 ok reference samples
per case, pinned NG revision f7830186043e4488f2998759d60a514faf07cbc9 /
binary 919f7235f61412ba69faf6aa05b534869aa7d69d1583b3d7aab3cd5b67cd6f40.
Record all reference medians, input hashes and derived per-case timeouts in
the manifest. Its largest reference median is 10.206905655 s, so all current
45 case thresholds are 60 s. Derive before timing; never update from the new
run. A timeout still fails. Bump manifest/protocol identity and hashes; archive
readers retain old-protocol replay. Replace canonical A/A whole: base=candidate
4c9f959e, 30 blocks, workers-4. It is the only eligible formal evidence.
Up to three whole replacements are allowed solely for load-invalid runs.

Power allocation uses only valid A/A precision, never gain or loss. For any
30-pair pilot case with h>0.03, freeze n=min(240, ceil(30*(h/0.03)^2)).
Retain the exact original 30 valid pairs. Pilot source is the complete
feedback-1 76x30 collector 20261017T022615Z-decision-rule-b-aa30-pairs;
held-out source is 20261017T022616Z-decision-rule-b-heldout30-pairs. Both have
zero heavy/invalid pairs and strict raw/source replay. The sole screen deficit
is sentinel/recursive_call_tree, h=0.059945073264363824 -> n=120. Append 90
valid pairs using the same image, immutable existing binary paths, frozen
iteration count and cumulative role alternation. No recopy/replacement of
these executables or remeasurement of their original pairs. Keep original
packets immutable; continuation files bind their source hashes and every load
log separately. All other screen cases and all nine held-out programs retain
30 pairs. Apply the identical power function to held-out deficits (none).

Precision-only second stage: if the frozen predicted n still misses 3%, append
to the 240-pair cap once, retaining all previous pairs. No intermediate search
or stopping based on effect size. If still imprecise at 240, record "imprecise
on this host": its wall band is diagnostic only; any unit measuring/touching
that case must complete K=6. This does not waive formal precision, target
payoff/all-pairs, instruction hard caps, lane/portfolio or correctness gates.
A complete panel supplies the required diagnostic evidence; that case's
unqualified wall band alone cannot reject/promote. Full-portfolio screens
include every policy case, hence any such fallback forces their full K=6.
Held-out remains acceptance-only; apply the same power/fallback rule, require
K=6 held-out evidence if it reaches the cap, and retain independent precise
formal/aggregate gates. No held-out case selects optimization work.

Unequal per-case qualification counts use all valid pairs for case statistics;
for lane statistics retain the original joint 30-pair cohort, since other cases
are not remeasured. Do not duplicate/downsample the extended case into a false
joint cohort or reinterpret independent continuation rounds as shared blocks.
The original A2 lane envelopes stay fixed. Extension source, count decision,
load replay and case precision are separately bound into policy evidence.

Engine finding: same-image Gaussian blur shows a heavy wall tail (+20-35%
versus NG about +5%), potentially related to T038's variance family. Record
it under T038 and the outcome as an engine finding with uncertainty; do not
encode a benchmark-specific exception or fix the engine in this task.

After frozen changes/tests, check iMac status and quieting before timing, run
only required continuation and one new canonical 30-block A/A, then qualify.
Publish new hash-checked qualified policy v2 only if all applicable checks
pass. Keep v1 and all prior logs/plans immutable; failed earlier formal input
never contributes a sample. Stop/report any new real blocker.


## Pre-timing implementation clarification

Timeout manifest v3 also corrects the external lane's unequal `candidate` /
`base` snapshot names: fixed equal-byte-length role/hash names are created
once and held stable for capability and all measured blocks. This fulfills
the original stable-path boundary, uniformly across all cases. Existing raw
archives retain their original argv and cannot become new formal evidence.
Timeout implementation 1cf3b751 passes its staged gate (536 tests); the
snapshot correction has actual PreviewRun path/order/identity regression.


## Feedback-2 execution record

Reference timeout implementation 1cf3b751 and equal-length path correction
3c082e6a pass 536 fast tests; power/continuation/fallback implementation
a1ec956e passes 543 fast tests. Focused tests cover formula/cap allocation,
immutable pilot/segment hashes, frozen iterations, absolute order/parity,
original lane cohort, diagnostic-only bands, mandatory K=6 and preserved gates.
No runtime source change or new dependency. T038 records the separate engine
wall-tail finding; it is not a policy exception.

| Job | Frozen source/protocol | State |
| --- | --- | --- |
| 20261017T050401Z-decision-rule-b-feedback2-formal30 | 3c082e6a; base=candidate4c; 30 blocks workers-4 | exit0 / 1360 s; precision failure |
| 20261017T050856Z-decision-rule-b-power120 | a1ec956e | cancelled before sampling for preflight parity |
| 20261017T052450Z-decision-rule-b-power120-preflight | 1600baa9; append90 valid to30 | exit0 / 140 s; h=3.4352% |
| 20261017T053205Z-decision-rule-b-power240-preflight | 1600baa9; append120 to120 | exit0 / 184 s; h=0.9350% |
| 20261017T053231Z-decision-rule-b-feedback2-qualify | 1600baa9; all retained sources/new formal | exit3 / 82 s; qualified false |


## Feedback-2 outcome

Both lead-authorized changes work without changing engine code or A2 bands.
All 4050 new formal external samples complete under their derived 60 s
limits, no timeouts or heavy monitor samples. The two formal roles are the
same c7f0f66a image at 4c9f959e; NG stays pinned at 919f7235/f7830186.
Raw source hashes and every per-case reference median/limit are in external
manifest v3. Stable external paths have equal filesystem-byte lengths.

The recursive sentinel retains its original 30 valid pairs and adds only
90 then 120 valid pairs (zero heavy/invalid in either segment), reusing the
original immutable executables, frozen N and cumulative role order. Counts:

| Valid pairs | Ratio | 95% CI | Relative half-width |
| --- | ---: | --- | ---: |
| 30 | 0.998480561 | [0.942011606, 1.046801225] | 5.9945% |
| 120 | 0.993925854 | [0.960916266, 1.003287270] | 3.4352% |
| 240 | 0.998217536 | [0.988970769, 1.005963685] | 0.9350% |

It is precise at 240; no diagnostic-only fallback is needed for this host
policy candidate. All nine held-out programs retain their original 30 pairs,
pass precision, and are never used for selection. Their measured program
bands and lane 1.0029 are unchanged from the prior diagnostic record. Primary
lane qualification retains the original joint 30-pair cohort; every interval
fits the unchanged A2 lane ceiling.

Qualification fails exactly one gate: new canonical formal
external/sunspider-1.0/bitops-3bit-bits-in-byte, 30 valid paired blocks,
ratio 1.0088128966, CI [0.9952546429, 1.0908768199], relative half-width
0.0813470204 >0.03. The complete raw input has no timeouts, missing samples,
linearity failures or heavy disturbance; this is a valid formal precision
failure. Screen power/cap fallback does not waive canonical formal precision
or authorize another formal sample count. No formal observation is removed,
retimed, imputed or pooled with earlier jobs.

The rejected candidate artifact remains in the qualification job's output,
qualified false; no qualified v2 is published. Frozen v1 is unchanged and
still refused by load_policy. No further timing is queued. T055 cannot close
as adopted. The separate [Gaussian engine-tail record](archive/T038-gaussian-wall-tail-data.md)
preserves the new 15.694 s base observation (+38.74%) versus NG +4.51%; the
uniform hang rule retains the tail rather than claiming an engine fix.

Final base-main fast gate at 1600baa9 passes 543 tests (73.632 s),
fmt/clippy/context/shell checks. Final full no-timing job
20261017T053457Z-decision-rule-b-feedback2-final-check at 1600baa9 passes
(exit0 / 165 s): 543 benchmark tests, Rust/workspace/feature checks, script /
hook / context / size guards and 5170 Test262 subset cases. Final record-only
staged/context gate also passes. No focused semantic slice matches tooling-only
changes. Runtime/parser/lexer sources remain unchanged from 4c9f959e.

Final T053/T054 replays use explicitly unqualified diagnostic A2 bands:
control event and wall event respectively, no direct rejection, missing K=6.
Their rejected standings and exhausted budgets are unchanged. No timing task
remains scheduled; the worktree/branch stays available for the lead decision.
