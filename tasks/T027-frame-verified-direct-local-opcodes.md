# T027: Frame-verified direct-local opcodes

## Status: frozen before implementation

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
