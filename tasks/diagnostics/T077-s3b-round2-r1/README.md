# Round-2 R1: canonical call owners

Parent `55f5d1d6`. This verified increment repairs R1; it is not S3b acceptance.
R2 and the public sweep's newly exposed mask-bound panic remain to fix.

The compiler/binding plan correctly reports receiver demand. Wide inline entry
created a typed owner with undefined this while storing the actual receiver only
in `current_this`. A typed loop's equality deopt (or an ordinary wide exit) passed
the undefined owner to the VM. Positional receiver data reached `Vm.direct_this`,
but semantic identifier reads correctly consulted the owner. Simple/top-level
calls bypassed this inconsistent inline state; IC disabling could not repair it.

All indexed entries now prepare/bind the canonical owner, including constructor
new.target, callee home/super metadata and inherited arrow owners. Removed
`DirectCallSlots.this_value`, `Vm.direct_this`, `WideFrame.this_value`, the driver's
`current_this` and receiver transfer arguments on wide/typed-loop/VM resumes.
Fast reads borrow the initialized owner; hooks release that borrow before JS.
Dense plans keep an eligibility bit and derive their receiver from the same owner.
No admission was narrowed to recognize the reproducer or equality/loop shape.

Sibling audit: implicit arguments, observable arrow this/new.target, own self-name
and super operations keep their existing guarded/general paths. Low-layer tests
assert those admissions, callee home identity, parked caller identity, constructor
target and forced VM exits. A 108-program matrix spans four equality operators,
three loop forms, function/shorthand/class methods and function/arrow/map callers.
Separate tests cover native standalone call/apply/Reflect.apply/bound/map/Array.from
entry and the lead's compound-member reproducer with missing and explicit args.
The two low-layer owner tests fail before the repair and pass afterward
([before](owner-before.log), [after](owner-after.log)).

Fast gate passes **2,643 runtime tests**, **117 selected curated cases**, 764
benchmark-tool tests and the script/context/format/Clippy checks. Focused wide
**84**, dense-loop **198**, and combined owner stress **6** pass. All **301 NG
fixtures** pass in default, IC-off and IC/binding verification modes. No full or
exact requalification was started, per the lead.

The new non-timing [public sweep](../../../docs/public-workload-correctness.md)
resolves all four public inventories and checks complete stdout at initial/2N
internal counts, with whole external/development programs intact. Its first
complete sweep covers **86 cases, 258 case-modes, 351 invocations: 348 pass / 3
fail** ([summary](summary.json), [request](request.json), [failures](failures.json)).
Audio-beat-detection passes all three modes. IC-verify on an unoptimized debug
binary times out gaussian-blur at 120 seconds and exposes a real eager-shift panic
in lodash/moment at `function/value.rs` (guarded `then_some(1_u128 << slot)`).
These failures are retained, never waived. Fix the shift, use an optimized verifier
with debug assertions **and overflow checks**, and rerun the complete sweep.
R2's discarded-call bug still requires its own effect-model commit.

[Metadata](metadata.json) binds the parent/diff, raw logs and sweep hash. Held-out
programs, timing, full/exact requalification, origin pushes and main merges are
outside this increment.
