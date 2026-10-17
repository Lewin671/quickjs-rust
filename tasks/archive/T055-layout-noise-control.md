# T055: Layout-robust policy qualification

- Status: closed; layout-robust rule adopted with qualified host policy v2.
- Implementation: present; collector ce13cefc, policy publication3be0ba66.
- Verified at: 3be0ba66, final iMac full gate (552 tests/5170 Test262),
  branch gate and independent91-plan validation; final record changes docs only.
- Evidence: [qualified v2](../../benchmarks/policies/imac-x86_64-rust1.95-fat16-layout-v2.json),
  [feedback-2 snapshot](T055-feedback-2-qualification.md),
  [feedback-1 record](T055-feedback-1-qualification.md),
  [original B plan/log](T055-phase-b-plan-and-log.md); final jobs below.
- Unresolved: no qualification blocker; T038 owns engine wall-tail diagnosis.
- Next action: freeze fresh schema-3 queue/unit plans under [T022](../T022-performance-priority-controller.md)
  using qualified v2 on the recorded host before T056 implementation.

## Frozen feedback-3 standing protocol (2026-10-16, before timing)

Authorization: workspace tmp/decision-rule-b/feedback-3.md. This supersedes
prior stop-on-precision rules, including the formal-precision exclusion in
feedback-2. It applies automatically to every calibration and formal A/A,
all cases and held-out programs, without another incident-specific decision.
No engine change, dependency, project-remote push or merge.

For a valid frozen pilot count base_n and relative CI half-width h>0.03,
required n=min(240,ceil(base_n*(h/0.03)^2)). Preserve every earlier valid
observation and append only the deficit under its original collector,
engines, protocol, iteration count and randomization/role rotation. The first
predicted stage is followed, if still imprecise, by one final cap stage at240.
All stopping/count choices use precision alone, never effect direction.
At the cap without 3%, mark "imprecise on this host": the case's band is
diagnostic only, K=6 is mandatory for any unit touching/measuring it, and
precision alone no longer blocks qualification. Other instruction, payoff,
stage budget, lane/portfolio envelope, identity, health and correctness gates
remain binding. Formal A/A cap status also makes that case's policy band
diagnostic; it cannot be hidden behind a precise screen calibration.

Screen/held-out continuations keep their original prefix/paths/N and separate
load logs. Formal subset continuations reuse the sealed three-engine bundle's
identities, exact argv/workload hashes, per-role N/startup checks for internal
cases, per-case timeout, source manifests and role rotation at absolute block
indices. No setup/calibration or old measurement is repeated. The original
bundle, reports and raw files remain byte-immutable; each new segment binds
all source hashes and its predecessor. Global/lane qualification retains the
original joint cohort; extended case statistics use all retained observations.
No replication/downsampling creates a false common cohort.

The old formal collector removes temporary snapshots/workloads on close.
Restore only those transient assets at the original argv paths, exclusively
from matching pinned images and regenerated identical workload bytes; never
overwrite an existing file. Preflight each restored image before measured
blocks and keep it immutable through every continuation stage. Existing
sealed evidence files are not modified. All three roles are measured, with
original seeded rotation continued from block30 rather than restarted.

This attempt's sole formal deficit is
external/sunspider-1.0/bitops-3bit-bits-in-byte in sealed bundle
20261017T050401Z-decision-rule-b-feedback2-formal30, collector3c082e6a.
Its base_n=30, h=0.08134702043130693 gives n=221: append191 complete
three-engine blocks (573 process samples). If still imprecise, append19
more blocks to240 automatically. Base=candidate source4c9f959e/imagec7f0f66a;
NG remains919f7235/f7830186. External v3 timeouts and role/hash paths stay
unchanged. The full bundle is never re-run or selectively reconstructed.

A formal continuation uses whole-segment load validity: any heavy monitor
sample (unchanged >=200% sum, 5-second cadence) makes only that appended
segment ineligible; retain it and replace the segment whole, at most three
load-invalid replacements. Never discard individual formal values or blocks.
Case failures/timeouts, hash drift and correctness failures remain blockers.
A new precision miss automatically takes the frozen next stage/cap fallback,
not a favorable whole-bundle repeat or a lead question.

The prior 30+90+120 recursive screen cohort is already precise at240 and is
retained unchanged. All other screen cases and all nine held-out programs
keep their precise30 observations. A2 bands, host/recipe bindings and 20,000-
draw seed-0 95% paired-median/geomean statistics remain unchanged. Keep old
policy v1 and every earlier archived log/plan immutable; publish a new
qualified v2 only after the complete replay and applicable gates pass.


## Feedback-3 implementation checks before timing

Generic three-role subset collector restores hash-identical transient assets,
keeps absolute seeded schedules and frozen per-role argv/N, and seals separate
immutable segment chains. Calibration completes deficient screen/held-out cases
automatically; formal continuation defaults to all deficient cases or accepts
a subset. Both finish predicted then cap stages without a precision question.
Formal cap metadata also forces K=6 and diagnostic bands. Focused 25 tests
passed (append counts/rotation, raw conservation/tamper refusal, timer/counter/
correctness/host contracts, exact asset recovery, cap qualification and gates);
115 layout tests passed. Staged fast gate passed all551 tool tests (79.339s),
context/file-size/shell checks. Sealed source replay checked10 artifact hashes,
30 original blocks, timeout60s, and573 scheduled append samples. No new timing yet.


Pre-sample correction: job20261017T055212Z-decision-rule-b-feedback3-formal221
at43027c89 exited1 in3s after quiet110s: Rust-only launch preflight sent
`--raw` to NG. No measurement file/block was written. All assets restored
with matching hashes; keep them unchanged. The corrected preflight runs the
same `-e 0` expression via Rust `--raw` and NG's supported argv, explicitly
tested before resubmission; no iteration/timing/sample protocol changes.


## Feedback-3 continuation outcome

Corrected collector ce13cefc passed staged fast gate:552 tests in75.645s,
context/file-size/shell checks. Job20261017T055601Z-decision-rule-b-feedback3-formal221-fixed
completed exit0 in21s, with zero monitor rows/heavy samples. Only191 new
three-engine blocks (573 successful processes) were measured at indexes30..220.
All30 pilot blocks remain; no further cap stage is needed. Combined221:
ratio1.002087489398624, 95%CI[0.9924644484027332,1.0097705040547589],
h=0.009696106506765112 (0.96961%) <=3%. No observations are removed,
reconstructed, downsampled or imputed. All10 sealed source artifact hashes
are unchanged. Original argv paths and restored images/workloads remain
hash-identical. Three launch preflights are separate from measured blocks.

Continuation set SHA256:
`5a7989df54ba960242baca2e35c64587e392a1766efb8b6d4e6a8d6faec578e5`;
appended raw SHA256:
`57bfa100c989ed19ce6414cf3d05e50b172d95aad57b1a08fb2d4c229f2f3866`.
Job20261017T055730Z-decision-rule-b-feedback3-qualify replays pilot/continuation
chains, sealed formal raw, original joint lane cohorts and source/host/load
bindings. It completed exit0 in84s: qualified=true, failures=[]; live policy
loading passed on iMac. T053 replay=control event (5 events), T054=wall event
(2 events); neither directly rejects, both need K=6, historical standing
unchanged/claim_eligible=false. These are diagnostics, not reopened units.


## Qualified policy publication

[Policy v2](../../benchmarks/policies/imac-x86_64-rust1.95-fat16-layout-v2.json)
and adjacent SHA256 sidecar publish the exact qualified output:
`22a1907dae06eb963dcb25592a05973e3bd69153e3c23a70ddec08ef64f07ca6`.
The v1 artifact/hash stay unchanged and unqualified. All76 screen cases,
76 formal cases and9 held-out programs now have precise intervals; no
cap-imprecise fallback is needed in v2. Recursive screen retains30+90+120=240;
formal bitops retains30+191=221; every other case/program retains30. Lane
statistics keep original joint30 cohorts. A2 case bands are unchanged;
ceilings broad1.0129493300226688, sentinel1.015687001889914,
external1.0073802375536396, portfolio1.0082623319647204, held-out1.0029.
All qualification lane intervals fit these envelopes. Host/recipe/revision
and image distinctions remain as frozen above; this is adoption on the
recorded iMac only, not an engine performance promotion or portable budget.


## Final handoff verification

Publication3be0ba66 passed staged fast gate552 tests (80.341s).
`check-touched.sh --base main --explain` passed552 tests (76.871s), context,
file-size and shell checks. No runtime/parser/lexer semantics changed, so
no focused semantic slice was selected; the full gate ran the curated subset.
Job20261017T060112Z-decision-rule-b-feedback3-full-check at3be0ba66 ran
`./scripts/check.sh`: exit0/163s,552 tool tests (101.748s), Rust/feature checks,
script guards and5170 Test262 subset cases (48s) pass.

Exact environment warning: `./scripts/check.sh: line 63: rg: command not found`.
That script's plan enumeration did not run. No claim is based on that skipped
step: job20261017T060329Z-decision-rule-b-feedback3-plan-check at3be0ba66
independently enumerated the same JSON plans in sorted order with Python and
called the original `performance-decision.sh check-unit` on every file:
all91 valid, exit0/10s. No tool installation or OS-service change was made.
The script's missing-rg assumption remains outside this task's boundary.

Final qualified T053/T054 diagnostic outputs preserve historical standings
and exhausted budgets. Main is unchanged; commits stay on
`agent/decision-rule-b/codex`, with no project-remote push/merge. Queue transport
refs only. Final report and raw outputs live in the workspace outside this
worktree; the clean task worktree is removed after the final record commit,
retaining the branch for later integration.
