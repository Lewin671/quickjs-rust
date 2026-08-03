# T027: Frame-verified direct-local opcodes

## Status: closed after mechanism success and fast-gate rejection

This is the next T018 leaf unit selected from the exact T022 queue for runtime
candidate `bea6aacf`. The frozen plan is
`performance-units/frame-verified-direct-local-opcodes.json`.

## Goal

Move repeated authoritative-local admission out of the hottest dispatch arms
and into the existing lowered-program selection boundary. Eligible ordinary
`LoadLocal`, `StoreLocal`, and `AssignLocal` instructions become appended
direct-local variants only in the same-offset derived execution stream. The
immutable source bytecode and every specialized loop plan remain unchanged.

## Why this unit is current

Performance Preview run `30856702773` produced the exact queue for
`bea6aacf`. Its SHA-256 is
`02b163bebca28e2d6a177176430bfed008f1bab48eaeac7e6e5cd1ece3ec3dca`.
HashMap remains external rank 1 at 7.2361x QuickJS-NG; public-field Raytrace is
rank 2 at 6.0040x.

Fresh generic opcode-family counters isolate local binding as the only dynamic
family above 20 percent in HashMap, Raytrace, and CDjs. The two receipted queue
cases are:

| Case | Local operations | Current authoritative hits |
| --- | ---: | ---: |
| HashMap | 34,306,316 / 105,261,606 (32.59%) | 33,226,260 (96.85% of local) |
| public-field Raytrace | 10,008,334 / 43,946,288 (22.77%) | 6,717,404 (67.12% of local) |

The current inline fast paths repeat dynamic-scope, slot-range, authority-mask,
metadata, initialization, and stack guards at every local instruction. This
unit does not fuse operations or change the binding representation. It uses an
already-existing frame-entry proof boundary to select simpler opcodes for the
subset that is known safe.

## Mechanism and proof obligations

Append `DirectLoadLocal`, `DirectStoreLocal`, and `DirectAssignLocal` to `Op`
so existing hot discriminants remain stable. Static lowering rewrites only
frame-local, non-captured slots whose metadata permits the operation. Every
rewritten slot is added to the lowered variant's
`required_authoritative_slots`; `code_for_frame` selects that stream only when
the live frame mask covers the complete requirement. A direct arm still falls
back to the canonical handler for TDZ, immutable, empty-stack, or malformed
states that cannot be discharged statically.

The mechanism gate is all of:

1. focused lowering tests prove ordinary eligible slots are rewritten while
   captured, received-upvalue, direct-eval/with, sloppy-global-fallback,
   eval-deletable, and uncertain class-capture slots are not;
2. a frame with an incomplete authority mask selects the original stream, and
   generator setup/resume refreshes selection through the same gate;
3. the immutable source stream and specialized loop plans contain no direct
   opcode and retain their existing shapes;
4. `size_of::<Op>()` remains 96 bytes and the three variants are appended;
5. candidate counters show direct-local variants cover at least 80 percent of
   the base HashMap authoritative hits; and
6. focused semantics cover TDZ, const assignment, captured updates, direct
   eval, modules, and generator resume before timing.

If the timing gate passes without the mechanism gate, attribution is
falsified and the unit is not retained.

## Frozen timing gate

HashMap is the sole payoff target and must be at most 0.97 candidate/base.
Public-field Raytrace, date-format-xparb, access-binary-trees, CDjs, Tagcloud,
validate-input, recursive control flow, broad local read, and the three current
allocation broad cases are controls and must each be at most 1.03. The attempt
budget is one. Promotion additionally needs complete broad and external
evidence plus a zero-gap exact Test262 receipt.

## Scope

Allowed implementation paths:

- `crates/qjs-runtime/src/bytecode/ir.rs`
- `crates/qjs-runtime/src/bytecode/virtual_object.rs`
- `crates/qjs-runtime/src/bytecode/virtual_object/lower.rs`
- `crates/qjs-runtime/src/bytecode/vm.rs`
- qjs-runtime diagnostics and focused tests
- `docs/benchmarking.md`
- this task, its frozen plan, and the performance task index

Forbidden within this unit:

- changing the environment, frame, upvalue-cell, `Value`, or public error
  representations;
- emitting direct variants into source bytecode or teaching specialized loop
  matchers to consume them;
- broadening direct authority to dynamic scopes, captures, received upvalues,
  modules, or sloppy global fallbacks;
- combining local operations, adding value-type specialization, or changing
  property, call, allocation, or benchmark behavior; and
- source-path, benchmark-name, function-name, iteration-count, checksum, or
  expected-result conditions.

## Verification sequence

1. Validate the frozen unit against the exact queue before code.
2. Use the saved exact base release executable SHA-256
   `f6bd01fd4fedabfa16ec133f937b6f3dc2476e31e3fdd680b037145c3d8aebe1`.
3. Add focused lowering, fail-closed selection, semantics, counter, and opcode
   layout tests; run qjs-runtime tests and clippy.
4. Build release and perf-counters binaries; prove the HashMap coverage gate.
5. Run strictly alternating same-host A/B for the target and controls.
6. Record `retained`, `rejected`, or `inconclusive` from the exact frozen gate;
   do not retune the target list or threshold after timing.

## Result

The one-attempt prototype appended all three direct-local variants without
changing `Op`'s exact 96-byte size. Static lowering admitted only real function
bodies and excluded captures, descendant dynamic scope, class uncertainty,
received upvalues, sloppy-global fallback, eval-deletable slots, scripts,
modules, and direct-eval top-level code. Each existing full/data lowered
variant retained its original fallback stream; an incomplete live authority
mask therefore disabled only direct-local opcodes, not existing scalar
replacement or superinstructions. Constant-binary lookahead inputs also kept
their established `LoadLocal` shape because they are matcher data rather than
dispatched operations.

The mechanism gate passed. Focused tests covered derived-only emission,
incomplete-mask fallback, captures, direct eval, TDZ, const assignment, mapped
arguments, module bindings, and generator resume. All 2,040 qjs-runtime tests
passed, the perf-counters partition test passed, and runtime clippy with
`-D warnings` passed. The candidate HashMap diagnostic reported
`direct_local_ops=33,226,238` against the frozen base's 33,226,260
authoritative local hits: 99.9999 percent coverage, above the required 80
percent. The diagnostic receipt SHA-256 is
`429d1f94c2e8e281953650f794be9c3144e683f71fe49540091fc0afdf3fc423`.

The frozen payoff gate nevertheless failed. The standard release base and
prototype executable SHA-256 values were
`f6bd01fd4fedabfa16ec133f937b6f3dc2476e31e3fdd680b037145c3d8aebe1`
and
`1c0a657c6e0346c4cf695780355dbc9e9b16aa0c0d3b52d1b3cc207591f06e28`.
Eleven strictly alternating amplified HashMap pairs measured median
candidate/base **0.984055x**, missing the required `<= 0.97x`. The full
observed range was 0.780686-0.993385; the first pair contained a cold base
outlier (2.600s versus the later 1.606-1.618s base range), while the ten warm
pairs ranged from 0.977848 to 0.993385 and lead to the same rejection. Median
base and candidate times were 1.6153s and 1.5888s. The A/B receipt SHA-256 is
`22bc5a3b9cad4f803698eccca49947198f07bcfa62fbf032040ddff3cb12b095`.

Because the sole payoff target failed, controls, complete hosted promotion,
and Test262 promotion evidence were unwarranted. The runtime, diagnostics,
tests, and benchmarking-doc changes were reverted; the checked-in engine is
source-identical to the frozen base. The rejected prototype diff has SHA-256
`eb149c9380415e8ea96a85231ecd743e16707e1e40b77bd0eea3babaebb6f695`.
Do not retry direct-local variants, authority-proof placement, lowered-stream
layering, or lookahead exclusions. A successor local-binding proposal needs a
new current profile and a different mechanism capable of exceeding the
remaining roughly 1.6 percent gain.
