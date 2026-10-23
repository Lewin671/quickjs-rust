# T075 ranked incremental proposals

Evidence at `59eb72e5`: [measurements](measurements.md), [source-map method](method.md).
These are engineering proposals, not approved units, frozen targets, savings
forecasts or reasons to reopen an old attempt. Rank uses canonical measured
reach and practical scope. Size/risk are estimates. Admission must depend on
operations, types and data flow; no program-name or bytecode-template test.

## 1. Destination-aware primitive register publication

**Reach:** transport 7.9273% of sampled main-thread time, with 4.3949% in
`mem::replace<Value>` contexts; primitive clone selection is a separate,
overlapping 1.8010% context. Lodash transport is 15.105%, graphlib 9.816%,
matter-js 9.474%. Numeric-result publication at canonical `0x10010353d`
alone is 0.4798%; Move/Dup publication at `0x100100e8e` is 0.7670%.
Do not promise to remove the entire transport row or add it to stack-load reach.

**Unit:** give proven immediate producers and register copies a safe
`clone_into`/primitive-write path that writes an already borrowed destination,
avoiding a full temporary `Value` round trip where the tag/data flow permits
it. Start with constants, moves and numeric fast results; preserve owned-value
fallback, self-copy semantics, old-value destruction order, TDZ and recovery.
Keep the slot/upvalue model and Value representation intact. Compare generated
store instructions and register-base reloads, not only source-level clone counts.

Likely files: `bytecode/compact_fn/execute.rs`, `vm_bindings.rs`,
`compact_fn/wide/activation.rs`, and existing executor tests. **Size M;
semantic risk M; layout/codegen risk high.** Stop if the code is merely moved,
register spills increase, or the real primitive-producer subset is too small.
The 8.2201% stack-source instruction share is motivation to inspect generated
code, not evidence that a stack load can all be eliminated or that it stalls.

**Earlier attempts:** [T070](../../archive/T070-value-transport.md) used
recovery-aware last-use ownership takes and later borrowed dense scalar reads.
Its broadened seven-program triage was 0.975245 against the lead's <=0.97
condition; it closed without promotion, with one screen unused. This proposal
changes destination publication/temporary traffic, not its dead-source proof.
[T024](../../archive/T024-general-register-core.md) removed dispatch spills but
moved the external corpus only to 0.9906; repeating its dispatch-preamble split
would not address these Value store PCs. T021's shared register-stack design
paid a base-plus-offset on every access and was reverted; do not recreate it.

## 2. Keep numeric indices scalar across boxed consumers

**Reach:** indexed-access and its guards occupy 4.0688% overall, 26.687% of
big.js and 6.896% of matter-js. Typed `ComputedRead` result-side PCs
`0x1002627a1` and `0x1002627e2` alone own 0.3438% and 0.2765%; the helper,
its caller checks and publication must be considered together. This is not a
claim that every indexed read is an avoidable boxed read.

**Unit:** inspect final typed IR and identity/provenance joins at the sampled
numeric-key paths. Where existing type/data-flow proofs permit it, keep a
numeric index in the scalar namespace and use existing indexed operations
while the receiver/result remains boxed. Insert required boxes only at actual
boxed uses, preserving exact deopt stack/IP, multi-successor joins, zero-trip
loops and alias identity. First measure how many sampled operations qualify;
if none do, close this proposal rather than adding a new speculative tier.

Likely files: `bytecode/typed_loop/compile*.rs` and boxed-demand/provenance
support, final-IR lowering and recovery tests; existing `ElementRead`/Unbox/Box
semantics stay authoritative. **Size M; semantic/recovery risk M–high;
layout risk medium**, rising to high if executor arms change. A compiler-only
slice should avoid growing hot dispatch code.

**Earlier attempts:** [T070](../../archive/T070-broader-transport.md) already
borrowed present dense scalar elements (`with_direct_dense_index_value` with
`Typed::from_value`), so that exact optimization is not new. It did not remove
boxed key/result flow in `ComputedRead`. [T030](../../archive/T030-packed-typed-loop-scalars.md)
changed the private scalar to eight bytes and regressed its first target to
1.01702; representation width is not this proposal. [T044](../../archive/T044-selected-stream-virtual-bank-typed-loops.md)
failed two screens with nonlocal codegen/control regressions. Existing
[T045](../../T045-typed-loop-boxed-demand-provenance.md) owns identity-flow
boxed-demand joins: reconcile its retained candidate/attempt status first,
and fold overlapping work into that owner rather than start a duplicate unit.
T075 does not establish that T045's old seed failure is the current cause.

## 3. Split the shared-slot hot cache path from cold cache variants

**Reach:** named/cache instructions inside executors occupy 3.2790% overall,
11.536% in matter-js and 4.041% in meriyah. The inline `probe_hot` context is
1.7509%; the narrower shared-slot live read context is 0.4962%, not all of it
removable. Sampled PCs show RefCell borrow state, hot-entry selection, entry
kind checks and shared key/slot validation, rather than an opaque property call.

**Unit:** keep one small shared-own-slot hot path and move prototype and
other cold variants behind a shared helper, preserving polymorphic entries,
weak identities and live layout/value validation. Avoid duplicating receiver
views in caller and callee. Inspect whether the current always-inline
`probe_hot` expands branches/spills for variants a given site rarely uses;
price the shared-slot route before choosing the final split. Release borrows
before any JS callback, getter, Proxy, coercion or cache update. This is a
cache-kind/code-size unit, not a new global "property absent" memo.

Likely files: `bytecode/named_property_cache.rs`, `compact_fn/property.rs`,
`value/object/slot_reads.rs` and property reentrancy/invalidation tests.
**Size M; semantic risk M; layout risk very high.** Audit executor codegen, executed instruction counts and ASLR-on fresh-process
tails before interpreting an apparent wall gain.

**Earlier attempts:** [T049](../../archive/T049-property-access-path-study.md)
removed a duplicate probe but missed the target and regressed controls;
[T041](../../archive/T041-own-data-cache-hot-answers.md) failed its hot-answer
screen; [T053](../../archive/T053-ordinary-own-slot-hot-read-fat-lto.md) gained
8.3% on its target but failed six controls. [T072](../../archive/T072-live-property-proof.md)
shared live receiver/slot proofs: final triage 0.949774 was promising, but both
screen collections failed, so it has no complete performance standing and
stays closed. [T073](../../T073-local-read-bimodal.md) links its CPU tails to
address geometry, not extra property operations or a proven allocator cause.
A successor must be materially different from rerunning T072 or changing a
winning placement; fresh queue, frozen budget and ASLR-on layout controls apply.

## 4. Prelink received-upvalue indices

**Reach:** the whole bindings row is 2.6477%, but the narrower inline
`upvalue_cell` context is only 0.7328%, including received-index calculation
0.4236%. `count_ones` alone is 0.3480%. Do not budget the whole bindings row
as savings from an index cache. Upvalue value borrows/TDZ checks remain required.

**Unit:** precompute an immutable wide-slot → received-upvalue index at
compilation/linking/admission and carry that typed index to the read/write,
instead of recomputing mask membership/rank on each instruction. Retain the
same live shared cell, runtime owner admission and lexical initialization
checks; never snapshot values or bindings. Prefer a local metadata table over
an opcode-enum size change until generated code proves the latter is needed.

Likely files: `bytecode/ir.rs`, wide compiler/admission/activation and received
upvalue tests. **Size S–M; semantic risk M; layout risk medium–high** because
wide operand/addressing changes can perturb the complete executor. The
whole-development deletion envelope is below one percent; this may be too
small for a worthwhile unit without a stronger measured target population.

**Earlier attempts:** [T027](../../archive/T027-frame-verified-direct-local-opcodes.md)
removed repeated local-authority checks with 99.9999% mechanism coverage but
only 0.984055 target wall, missing <=0.97. It did not prelink received-cell
rank; do not repeat its direct-local opcodes or authority-proof placement.
[T069](../../archive/T069-call-activation.md) already caches call binding and
frame-initialization metadata and is promoted. This proposal concerns repeated
in-executor cell routing, not another call-environment preparation rewrite.

## Lower priority from this evidence

A generic dispatcher rewrite is not the first unit: identified dispatch is
5.7042% and ordinary compiler lowering already answers most of it. T024's
result above and [T028](../../archive/T028-discarded-binary-branch-superinstruction.md)
(9.73% fewer dispatches, target 0.983685 versus <=0.97) warn against treating
that envelope as payoff. Completion is only 0.2079% here;
[T026](../../archive/T026-compact-hot-op-error-abi.md) previously priced a
narrow success ABI and failed its targets (1.0073 / 0.9963). No current evidence
justifies reopening it, the T065 general rewrite, a Value representation
change, a universal bounds-elision unit or a new allocator experiment.

Every proposed unit needs a fresh formal opportunity queue, exact-target
profiles, explicit mechanism coverage, a frozen finite budget and the existing
correctness/layout gates. None may infer its acceptance from these diagnostic
time shares. Keep current host layout and controls; never select a winning
address or suppress a minority CPU tail.
