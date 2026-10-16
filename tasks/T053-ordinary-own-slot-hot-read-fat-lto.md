# T053: Ordinary OwnSlot hot reads under the current compiler recipe

- Status: active.
- Implementation: none; frozen plan before runtime edits.
- Verified at: base 9a0eeeca, fresh 30-block iMac standing and three exact
  binary hash-map profiles, plus source-equivalent feature-only route counts.
- Evidence: the frozen plan and current baseline/profiles below.
- Unresolved: correctness, one complete screen and any subsequent formal gate.
- Next action: validate and commit the plan, then port and review the guarded
  OwnSlot cache mechanism; do not reopen or relabel the old T041 attempt.

## Scope and generality

Plan: [ordinary-own-slot-hot-read-fat-lto](performance-units/ordinary-own-slot-hot-read-fat-lto.json).
Host owns named_property_cache.rs, its tests, this plan/records, and the
neutral x86 natural-order replay. No parser, AST, environment model, reference
corpus or benchmark changes. Workspace worktree: worktrees/quickjs-rust/
ordinary-own-slot-fat; branch agent/ordinary-own-slot-fat/root; base 9a0eeeca.
Command Code separately owns layout tooling on its own worktree, and the host
reviews/integrates its result. All heavy checks and measurements use iMac queue.

The mechanism is selected by cache entry kind and guarded object state, never
by JS source, names, arity, loop shape, iteration count or checksum. The target
is current external rank 7, inside the declared top-eight ceiling; no override.
Fresh counts also observe OwnSlot rejection in 3d-raytrace and Kraken AES.
Those counts establish path reach, not speedup. Their complete measurements
remain mandatory controls, as do the other 74 portfolio cases.

T041's old implementation/gate remain rejected and exhausted. This is a newly
priced unit at a different compiler recipe and source revision, with fresh
profiles, independent controls and exactly one frozen screen. Reuse code only
after verifying that its guards and cache-state transitions remain correct.
No favorable reruns or post-result layout tuning are authorized.

## Current evidence

- 20261016T122024Z-general-current-baseline: complete 76 cases x 30 blocks,
  workers four, wall time, ready_for_unit_gates; no imprecise/invalid cases or
  host disturbance. Both binaries: 926ee634ca3901b89c778478702a4fe993288ef7e8e69cd93c238c694595aa85.
- 20261016T122336Z-general-fresh-top8-profiles: exact current binary, original
  unmodified programs. Hash-map cold named-read envelopes: 59/494, 63/490,
  70/492; caller samples overlap and cannot be added as removable work.
- 20261016T123302Z-general-fresh-route-counts: source-equivalent diagnostic
  executable hash verified and stdout matched all eight original programs.
- 20261016T123733Z-general-natural-order-replay: exactly one replay of all
  4205 unique text functions; complete executable SHA equals the baseline.
  This preserves the base binary, not future function sizes or every layout.

## Verification

Cover live updates, descriptors/getters, receiver identity, Small-to-other
storage conversion, cache replacement/clear/Exact-to-OwnSlot transitions,
prototype/Proxy fallback and no cache borrow across user callbacks.
Use focused Rust tests, selected/full Test262, full check, all NG fixtures,
the complete frozen screen and, only after a pass, formal 30-block comparison
plus exact-candidate zero-gap Test262 evidence. Record every outcome here.
