# T055: Layout-robust policy qualification

- Status: active; feedback-3 makes precision power allocation standing/automatic.
- Implementation: automatic screen/formal subset continuations and cap fallback present.
- Verified at: 1600baa9 full gate, 543 tests/5170 subset; e7cd6d71 record.
- Evidence: [feedback-2 snapshot](archive/T055-feedback-2-qualification.md),
  [feedback-1 record](archive/T055-feedback-1-qualification.md),
  [original B plan/log](archive/T055-phase-b-plan-and-log.md).
- Unresolved: bitops continuation and final qualification not yet executed.
- Next action: finish staged gate, commit collector, append bitops blocks,
  qualify, publish/close on success; stop only for a new non-precision blocker.

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
