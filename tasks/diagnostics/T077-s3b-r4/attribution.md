# S3b performance attribution (independent investigator, 2026-11-01)

Deterministic local arm64 evidence. Base `3cc12c6f` and candidate `8413f925` were built
with the same fat-LTO recipe. Instructions come from `/usr/bin/time -l` (±0.1%), plus
perf-counters, `QJS_TL_TRACE`/`QJS_CF_TRACE` and `-Cinstrument-coverage` line counts.
Local ratios match the iMac: bitwise-and 44.2 (iMac 50.5), partial-sums 15.6 (iMac 15.9).

Reproducers are in `repro/` next to this file; counter and instruction data for all 55
cases (base/cand/experiment) are in `allcnt/`. Line references are to the candidate at
`8413f925`, `crates/qjs-runtime/src/`.

The bisect shows the loss is structural, not caused by the review fixes:
- 55f5d1d6: development 1.083, external 1.250, broad 1.067
- 8413f925: 1.087, 1.253, 1.079

## 1. Configurable or missing global names are treated as unstable references (~50% of loss)

Cases: bitwise-and, partial-sums, validate-input.

**Base vs candidate.**
- Base runs bitwise-and as a typed loop. partial-sums' inner loop runs a typed loop
  (5/5 Ran). validate-input `doTest` runs on the wide tier.
- The candidate falls back to the VM with reference events on every op:
  - bitwise-and executes 7.8M ops, at ~9,300 instructions per iteration;
  - `doTest` is silently declined from the wide tier.

**Where.** `bytecode/reference_plan.rs:184-210` `stable()` treats a Name site as
stable only if the globalThis own property is non-configurable (:209). Implicit globals
(`g = …`, `var a1 = a2 = …`) are configurable, so `can_skip` is false. That result:
- enables `reference_hooks` at `vm_frame_init.rs:280`;
- skips both loop accelerators before `loop_backedges` is even counted
  (`vm_loop_dispatch.rs:70-80`);
- rejects compact/wide entry (`compact_fn/activation.rs:280-287`, `:699`;
  `wide/activation.rs:1563`).

Per-event cost: `vm.rs:549-572`, then `vm_references.rs:60-133` doing a by-name
`select_binding_reference`, then `binding/global_reference.rs:18-130`. That path
allocates `name.to_owned()` at :36 and makes 3–4 property probes, a realm hash and a
String clone. Even stable `var` sites re-prove `can_skip` by name on every call, once in
compact and once in wide: call-global-var-write is 1.57×, ~2,900 extra instructions per
call.

**Why.** B3b2 selects the reference before the RHS. That is correct when a global
lexical is created during the RHS, or when a strict name is missing. Base resolved late
everywhere: wrong in those corners, but always fast.

**A2 (partial-sums only).** `vm_bindings.rs:520` requires the typed-loop sloppy-global
sink to hold a value copy in a local slot at entry. S3b's canonical realm-cell routing
clears that copy after the first call, so calls 2–20 are all Declined (instrumented:
19 local-empty).

**Repro ratios (cand/base instructions).**

| Repro | Ratio | Control |
| --- | --- | --- |
| bitand-global.js | 39.0 | bitand-var.js 1.02 |
| partial-global.js | 25.1 | partial-local.js 1.045 |
| validate-like.js | 2.02 | |
| call-implicit-global-write.js | 3.18 | |
| call-global-var-write.js | 1.57 | |

**Experiment.** `repro/experiment-A-P2.patch` is measurement only and semantically
wrong: it treats sloppy Name sites as stable and lets the sink read the canonical cell.
Results:
- bitwise-and 44.2 → 1.02
- partial-sums 15.6 → 1.09
- validate-input 1.54 → 1.08
- local external geomean (45) 1.256 → 1.081; development unchanged at 1.080

**Proposed fix: epoch-validated Name references.**
- Sloppy sites, and strict sites that are currently bound, are stable at compile time.
  Compute `can_skip` once per plan and cache it.
- The realm keeps a `global_lexical_epoch`, bumped only when `declare_global_lexical`
  (`context/activation_api.rs:213`) creates a global lexical, with a small (epoch, name)
  log.
- VM, WideFrame and compact frames compare the epoch once after each call/construct
  returns. On a mismatch, pin the live interval containing the current pc to its
  pre-call resolution (object record or unresolvable), and enable hooks for that frame
  only.
- Strict unbound sites stay hooked; such code throws anyway.
- The sink accepts a value whose source is pointer-equal to the canonical realm cell.
- This keeps Context, compile-time plans and the B3b2 semantics, with no name snapshot.

## 2. Every JS call builds a full ActivationContext (~20–25% of loss)

Cases: captured_write, all call cases, binary-trees, hash-map, raytrace, cdjs, cordic,
and most of development.

**Base vs candidate.** Tier, counters and op counts are identical; the candidate adds
435–650 instructions per call (binary-trees +553, hash-map +449, raytrace +626,
immutable +435, moment +518).

**Where.** Coverage shows every call running `enter_indexed_activation`
(`function/call.rs:1406-1421`):
- `selected_function_creation` clones an Rc;
- `ActivationContext::new` generates an identity eagerly (`context/activation.rs:53-55`;
  `typed_variable_identities` equals the call count);
- three `with_cold` borrows (lexical_call/home/super);
- `CallOwner::root`, then `bind_indexed_call_owner`.

`wide/activation/entry.rs:84-108` then moves the ~160-byte context into the WideFrame
and moves it back out on return, where it is dropped (`wide/activation.rs:1264`).
`wide/activation.rs:495` builds it before even trying the numeric plan (entry.rs:24).
Base shared the caller's CallEnv, with one `shares_caller_environment` proof per call.

Repros: `upw0.js` 1.53 (~480 extra instructions per call) and `upw1.js` 1.51
(captured_write). Each captured write adds ~125 more: `wide/activation.rs:777-784` calls
`cell.assign`, which does TDZ/metadata checks in `binding/cell.rs:186-208` and reads
`env.caller().strict` every time.

**Proposed fix.**
- Inline frames carry only callee, receiver and new_target. Materialize the
  ActivationContext only on exit to the VM, on native re-entry that needs a Context, and
  on closure/arguments/scope exposure.
- Generate the identity on first exposure.
- Add a "has cold owner state" fact bit so common functions skip the three `with_cold`
  borrows.
- Borrow the CreationContext from the callee instead of cloning it.
- Build no context at all for numeric calls.
- Remove per-call identity generation on the closed-form path (`function/call.rs:364`,
  `:369`, `:587`).
- Decide the StoreUpvalueLocal write policy at compile time: an initialized mutable var
  takes an unchecked set.

## 3. Closures in eval-extensible scopes lose their tier (~7% of loss)

Cases: date-format-xparb 1.80, string-tagcloud 1.52, date-format-tofte.

**Base vs candidate.** Base runs on wide/standalone both an eval-created format function
and `walk`, whose parent has a sloppy eval. In the candidate, generic_call_frames goes
from 2 to 8,002 and 7,503, with by-name `String`/`Date`/`Math` lookups through the scope
cursor in the VM.

**Where.**
- `function/call.rs:1264`: `lexical_scope().is_some()` rules out direct leaf.
- `context/activation_api.rs:416-421`: `indexed_bindings_only` requires no cursor.
- VM ops in eval-containing functions also cost more: `vm.rs:535-547` checks pc_scopes
  on every op, and local reads go through cell routing (`vm-ops-local.js` 1.32).

Repros: `eval-parent-closure.js` 2.98, `eval-closure.js` 1.53.

**Proposed fix.** Admit the function to direct-leaf/wide when the scope chain has no
`with` and no extensible holder contains any of the function's free names. Cache that
proof per (function, holder-shape epoch) and compare the epoch once at entry. Fall back
to VM routing only once eval actually declares one of those free names.

## 4. Closed-form leaf functions rescan purity on every call (~4% of loss)

Cases: broad function_call_*, captured_read, plain, reordered, top_level, sentinels.

Both sides run on the same tier (typed-loop closed form), but
`vm_numeric_leaf/effects.rs:14-51` scans the plan ops linearly on every call, even
though the plan already has `writes_received_upvalues`. `typed_loop/execute.rs:621`
also builds `loop_context()` on every call.

Repro: `closed-form-call.js` 1.185. Caching `pure` in GeneralNumericLeafPlan brings it to
1.098 (broad function_call_two_args 1.183 → 1.097).

**Proposed fix.** Compute `pure` in `NumericLeafPlan::compile`, and hoist the context
construction out of the per-call path.

## 5. Per-op VM checks and VM global routing (~10–15% of loss)

Cases: array_write, property_*, diff, meriyah, …

`vm.rs:535-575` adds three checks to every op: binding_scope/pc_scopes, reference_hooks
plus the event cursor, and an authority compare. `vm.rs:923` compares authority again
after every general op. That is ~15 extra instructions per op in plain functions
(`vm-ops-plain.js` 1.086, broad array_write 1.12). Global reads and writes in top-level
VM code also do by-name realm hashing and String allocation (`vm-ops.js` 1.20, 4 extra
NameHasher calls per iteration).

**Proposed fix.** Move scope switching, hooks and authority refresh into the ops that
can change them, or test one frame-level slow-flags word per op. Route globals through
cached realm-cell handles instead of passing `String` names.

## Diagnostic gaps

- Rejections caused by `can_skip` and `indexed_bindings_only` print nothing in
  `QJS_CF_TRACE`.
- Loop edges blocked by hooks are not counted in `loop_backedges`.

Add reason tracing and counters for both.
