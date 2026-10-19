# T058: Rejected v6 unit; diagnostic-only instruction regression work

- Status: active diagnosis; v6 current-base unit rejected on its sole valid screen; no re-decision or budget reset.
- Implementation: T058 changes present at `9e415ff4`; no new fix yet and no optimization retained.
- Verified at: `88bec228`, 2026-10-19 UTC; fetched r2 rejection and raw K=6 evidence, original iMac exact-image instruction/profile diagnostic.
- Evidence: [v6 rejection](archive/T058-v6-rejection.md), [attempt 1](archive/T058-attempt1-incomplete.md), [attempt 2](archive/T058-attempt2-inconclusive.md). New diagnostic jobs/results recorded below.
- Unresolved: identify and, if general, fix the recursive helper instruction increase; explain darkroom; full checks and diagnostic triage only if a fix is justified.
- Next action: compare exact-image profiles and feature execution counters; inspect the helper's generated argument-copy prologue. No new plan or acceptance is authorized.

## Current evidence and scope

The rejected source pair is base `a68a9f46` versus candidate `9e415ff4`.
Both contain T057; the old pair `351004f3`/`392ca717` did not show the
recursive instruction excess. This comparison does not imply T057's call
semantics caused it. New non-timing diagnostics run on the iMac only; local
M1 instruction counts would be host-incomparable and are not used as x86
instruction evidence. Wall durations from non-timing jobs are not results.

The user authorizes diagnosis and a general fix, full `check.sh` and quick
triage against `a68a9f46` including recursive_call_tree, then a report. No
new plan, formal acceptance, re-decision or budget reset. Avoid timing jobs
in the iMac's 03:00-05:00 local window; initial clock was 06:54 CST (UTC+8).

Exact-image diagnostic `20261019T225612Z-t058-exact-call-instruction-diagnose`
uses the immutable r2 natural binaries (hashes in the rejection archive).
N=65536/131072, three retained increments per role: base
363.3551/363.3962/363.3429 retired instructions per declared call; candidate
381.3429/381.3499/381.3529, median ratio 1.049524. Thus about 18 extra
instructions per physical numeric helper call, independent of startup.
Whole darkroom instruction medians: 15,524,633,752 versus 15,699,241,916,
ratio 1.011247. These are diagnostic counts, not new acceptance samples.

Exact profiles show recursive_call_tree runs in
`typed_loop::helper_graph::numeric::NumProgram::run`, with new memcpy /
platform memmove samples only in the candidate. The register argument
initialization differs in the audited assembly: base has up to four direct
scalar copies, candidate calls dynamic-length memcpy. Source of that helper
is unchanged between the two revisions. The older base/candidate both have
the direct-copy prologue. This is a code-generation lead, not yet a complete
causal attribution or a fix result. Inline-cache/call-path counters and
intermediate T058 builds will test it.

Diagnostic jobs are non-timing and leave every raw counter/profile artifact:
- `20261019T225612Z-t058-exact-call-instruction-diagnose`: exact counts,
  eight-second samples and function-body comparison; exit 0.
- `20261019T225638Z-t058-execution-path-diagnose`: independent feature builds
  of both source revisions; execution counters and compact/IC traces.
- Code-generation lineage job: submission/result recorded before handoff.

Record conclusions, hashes, revision and checks here before ending. Any
new candidate remains diagnostic-only and cannot reopen the rejected unit.
