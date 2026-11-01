# Numeric helper preparation and replay effects

Parent `fbe8dc0b`; standalone R2 `a6de336d` is unchanged. Lead P1 reproduces
as 101 captured writes for 100 calls (`before.log`). Twelve body/consumer
combinations now preserve 100 (or 200 for +=2) including later accessor deopt.

Root: `helper_graph::closed_form_already_answers` executed a numeric leaf on
dummy arguments, assuming numeric meant pure. Numeric leaves can commit
received-cell writes at Return. Removing that probe alone exposed flattening
which treated received-cell destinations as private undefined locals.

The shared numeric `Effect` policy now also gates speculative leaf evaluation.
Its exhaustive FastOp classification rejects captured stores, including the
terminal update shortcut. Actual ordinary calls still commit writes normally.
Both entry probing and typed-loop fallback use this pure entry. Flattening
rejects received-cell and immutable destinations before producing any plan.

Audit of deletion, forwarding and replay paths:

- Numeric/compact DCE keeps Call, Native, unmodeled Other and control ops under
  `numeric/effects.rs`; constant hoisting and result forwarding move only
  scalar work/copies, retaining producers once in original order (round 2).
- Compact numeric bytecode lowering accepts private writable slots only;
  captured destinations fail `Lowering::writable`, captured inputs start Unset.
  It executes only validated numeric callees and private registers; Bail/depth
  retries cannot publish effects. Helper flattening now has the same boundary.
- Typed copy forwarding removes only Move/MoveBoxed/scalar ToNumeric, retains
  call/property/store producers and all deopt operand liveness. Fused producers
  publish their destination only after success. No call is moved or replayed.
- Typed deopt publishes already-completed slots and reconstructs the stopped
  opcode's exact operand stack. Calls that decline have performed no user code;
  the shared purity gate now enforces this for JS numeric leaves. Later Unbox
  failures resume at their consuming opcode, preserving the completed result.
- Fast native table inputs exclude observable coercions; successful calls are
  retained in order. RNG draws and Date.setTime are not hoisted or probed.
  An error/decline in their guarded routes has not performed observable work.
- Invariant property hoisting admits ordinary data only, excludes accessor,
  Proxy/exotic reads and writes to the named property; call hoisting uses its
  guarded pure whitelist, excluding RNG and user callbacks.
- Wide unused-constant/alias rewrites and compact Pop-to-Drop keep effectful
  producers; neither reconstructs an executed callee after a guard failure.

Focused before/after logs are retained here. The staged fast gate and final
qualification are recorded in T077; no performance claim follows from this fix.
