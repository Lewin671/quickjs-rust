# T055: Layout-robust policy qualification

- Status: active; lead feedback-2 authorizes uniform timeout and power rules.
- Implementation: feedback-1 present; feedback-2 changes pending. No engine change.
- Verified at: ea355b49 full gate, 532 tool tests and 5170 subset; 5a36b00f record.
- Evidence: [feedback-1 record](archive/T055-feedback-1-qualification.md),
  [original B plan/log](archive/T055-phase-b-plan-and-log.md),
  [A2 data](archive/T055-phase-a2-data.md).
- Unresolved: new protocol and qualification not yet implemented/measured.
- Next action: implement/test the frozen amendment below; append required pairs,
  replace canonical formal whole, qualify and close only after all gates pass.

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
