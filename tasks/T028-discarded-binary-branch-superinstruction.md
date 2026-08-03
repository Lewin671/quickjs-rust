# T028: Discarded binary-branch superinstruction

## Status: planned from exact evidence; implementation not started

This T018 leaf unit is frozen in
`performance-units/discarded-binary-branch-superinstruction.json` against the
latest completed exact runtime evidence for `e11b5d19`. The later `bced98c4`
revision adds only the T027 rejection record; its runtime is source-identical.

## Goal

Collapse the common `Binary; JumpIfFalse; Pop` condition whose false target is
also a `Pop` into one derived stack operation. It computes the same binary
result, applies the same ECMAScript truthiness test, discards the result on
both edges, and advances directly past the corresponding `Pop`.

## Why this unit is current

Performance Preview run `30858694797` produced the latest completed exact
queue for `e11b5d19`. Its SHA-256 is
`fe1b970103d53cf7408e6da4398d2d456bffda056b3d4172627f191d86a056f9`.
HashMap is external rank 1 at 7.1323x QuickJS-NG; public-field Raytrace is rank
2 at 6.0931x and CDJS is rank 6 at 5.1548x.

A temporary `perf-counters`-only opcode breakdown, reverted immediately after
collection, found one shared remaining sequence:

| Case | Fully discardable binary branches | Theoretical dispatch removal |
| --- | ---: | ---: |
| HashMap | 5,122,209 | 10,244,418 / 105,261,606 (9.73%) |
| public-field Raytrace | 599,331 | 1,198,662 / 43,946,288 (2.73%) |
| CDJS | 1,884,381 | 3,768,762 / 72,858,332 (5.17%) |

The existing local-local comparison fusion already handles the stronger
`LoadLocal; LoadLocal; Binary; JumpIfFalse` shape and must run first. T028
targets the millions of remaining conditions fed by property, global,
constant, or mixed sources. Unlike the historical `BinaryLocals` experiment,
this unit fuses a dynamically measured two-edge discard boundary after the
retained dispatch split; it does not reopen direct-local guards, dispatch
preamble work, or the old pre-load-only opcode.

## Mechanism and proof obligations

Append one `Op` variant so existing discriminants stay stable. Lowering may
replace `Binary` only when the next instruction is `JumpIfFalse`, its
fallthrough instruction is `Pop`, its checked false target is also `Pop`, and
the source range is linear under the existing complete flow analysis. The
replacement stores the original `BinaryOp`, the first instruction after the
false-edge `Pop`, and the number of same-offset source instructions skipped on
fallthrough.

The mechanism gate is all of:

1. focused lowering tests prove the exact two-Pop shape fuses, while one-edge
   Pop, non-linear ranges, malformed targets, and incomplete analysis do not;
2. the existing local-local comparison fusion retains priority;
3. source bytecode, instruction count, jump offsets, and specialized loop
   plans remain unchanged;
4. focused runtime tests cover true and false Number comparisons, string and
   object coercion with observable order, thrown coercion, and surrounding
   stack values;
5. `size_of::<Op>()` remains 96 bytes and the new variant is appended; and
6. candidate counters cover at least 90% of the 5,122,209 frozen HashMap
   sequences and reduce its generic `executed_ops` by at least 8%.

A timing win without these mechanism checks is not attributable and cannot be
retained.

## Frozen timing gate

HashMap is the sole payoff target and must be at most 0.97 candidate/base.
Raytrace, CDJS, date-format-xparb, binary-trees, Tagcloud, validate-input,
3d-raytrace, recursive control flow, A*, broad branch arithmetic, local reads,
and the three allocation cases are controls capped at 1.03. The attempt budget
is one. Complete promotion additionally requires the full broad and external
portfolios plus zero exact Test262 gaps.

## Scope

Allowed implementation paths:

- `crates/qjs-runtime/src/bytecode/ir.rs`
- `crates/qjs-runtime/src/bytecode/virtual_object/lower.rs`
- `crates/qjs-runtime/src/bytecode/virtual_object/flow.rs`
- `crates/qjs-runtime/src/bytecode/vm.rs`
- qjs-runtime diagnostics and focused tests
- `docs/benchmarking.md`
- this task, its frozen plan, and the performance task index

Forbidden within this unit:

- changing parser, AST, environment, frame, upvalue, `Value`, property-cache,
  object-storage, or public error representations;
- changing source bytecode or teaching specialized loop plans a new shape;
- fusing branches whose condition remains observable on either edge;
- broadening incomplete control-flow analysis or combining unrelated opcode
  families; and
- source-path, benchmark-name, function-name, property-name, iteration-count,
  checksum, or expected-result conditions.

## Verification sequence

1. Validate the frozen unit against the exact queue before code.
2. Preserve an exact standard release executable for `e11b5d19` as the base.
3. Add focused lowering, semantics, counter, and opcode-layout tests; run the
   qjs-runtime suite and clippy with warnings denied.
4. Build release and `perf-counters` candidates; prove the coverage and
   instruction-reduction mechanism gates.
5. Run strictly alternating same-host A/B for HashMap, then the frozen controls
   only if the target passes.
6. Record `retained`, `rejected`, or `inconclusive` without changing the cases,
   thresholds, or one-attempt budget after timing.
