# Public verification: capture-mask bound

The first round-2 public sweep exposes lodash and moment panics under overflow
checks at `function/value.rs`: `then_some(1_u128 << slot)` evaluates the shift
even when its preceding `slot < 128` predicate is false. A 130-capture source
also panics on baseline `3cc12c6f` ([base](base.log)); this predates R1.

The fix filters the bounds before mapping accepted slots to bits. Large received slots remain on their normal
cell route; the 128-bit field is an optional fast mask, not a language limit.
The boundary regression checks 128, 129 and 130 captures and their computed sum:
[before](before.log) fails at the eager shift; [after](after.log) passes all
three sizes. The two sibling `then_some(... << slot)` sites in
`vm_frame_bindings.rs` enumerate through `.take(u128::BITS)`, so their shifts are
already bounded independently of the predicate; no change is needed there.

The failed public sweep is retained in the [R1 record](../T077-s3b-round2-r1/README.md).
Fresh complete sweeping with an optimized verifier must keep both debug
assertions and overflow checks enabled; do not hide this panic by disabling
overflow checks. Full/exact requalification remains deferred by round-2 feedback.

The staged fast gate passes 2,644 default runtime tests, 52 selected Test262
cases, strict Clippy, formatting and context checks.
