# T049: Property-read path cost study with the adopted compiler recipe

- Status: active.
- Implementation: none in production; research only until evidence and a
  newly frozen plan justify a candidate.
- Verified at: base `21e55fe2b530be09ea0453fcb87cde3b36423db3`, 2026-10-16;
  source inspection only; fresh measurement dispatched.
- Evidence: local artifact, not retained: workspace queue job
  `20261016T101305Z-property-current-baseline`.
- Unresolved: operation reach, exact-binary cost and net runtime benefit.
- Next action: finish the current-base comparison, derive its opportunity
  queue, collect exact-binary samples and feature-only path counters, then
  freeze independent units before production runtime edits.

## Scope

User-requested research into repeated property-cache probing and reuse of
receiver-own-property-miss proofs. Primary owner: `agent/property-path-study/root`,
workspace worktree `worktrees/quickjs-rust/property-path-study`, base above.
Production candidates, if justified, remain separate mechanisms. Do not repeat
T041's rejected hot-own-slot implementation or the old T033 prototype shortcut
without fresh profiles and a newly frozen plan. Preserve slots/shared cells,
property semantics, getter/Proxy behavior and complete test/benchmark inventories.

Allowed paths: this task, its plans/index rows, focused property/VM/cache code
and tests only after a validated frozen plan. Diagnostic counter additions may
be temporary and feature-gated; never time instrumented executables. No benchmark,
manifest, acceptance-rule, compiler recipe or unrelated runtime changes.

Every benchmark/profile runs through the workspace iMac queue. Timing uses
wall time on this host; instructions are supplemental. Fresh base/base/NG
comparison uses 30 blocks and four qualified workers. Existing main worktree
changes and the other campaign owner's worktree are outside this task.
