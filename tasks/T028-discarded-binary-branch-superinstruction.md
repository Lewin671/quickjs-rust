# T028: Discarded binary-branch superinstruction

## Status: rejected after the frozen one-attempt gate

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

## Result

The one-attempt prototype appended the derived opcode while preserving the
exact 96-byte `Op` layout. Lowering required the complete flow analysis, the
exact two-`Pop` successor shape, checked targets, and a linear source range;
the existing local-local comparison fusion retained priority. Focused tests
covered both numeric outcomes, strings, observable left-to-right object
coercion, thrown coercion, surrounding stack values, missing edge Pops,
malformed targets, unproven ranges, and incomplete analysis. All 2,039
prototype qjs-runtime tests passed, the 13 perf-counter diagnostics passed,
and runtime clippy with `-D warnings` passed.

The mechanism gate passed. The diagnostic candidate executable SHA-256 was
`e5c1626914fc6138542e7b837b2828f5c0fe8eff0a28833310d30188e586d220`.
It dispatched the fused operation 5,122,203 times, covering 99.9999% of the
5,122,209 frozen HashMap sequences, and reduced `executed_ops` from
105,261,606 to 95,017,200: 10,244,406 fewer dispatches, or 9.73%. The counter
receipt SHA-256 is
`16a7ba8d095622c06fccf3b608e142d7d6f9ff28f0b6922a63f7efa395cd4d0c`.

The frozen payoff gate nevertheless failed. The exact base and standard
prototype executable SHA-256 values were
`f6bd01fd4fedabfa16ec133f937b6f3dc2476e31e3fdd680b037145c3d8aebe1`
and
`ab97ef4892c9af6c3fc9b1b790e914602c1d07bf63fccc9b08b2aa3953d4f7ed`.
After one warmup per role, eleven strictly alternating HashMap pairs measured
median candidate/base **0.983685x**, missing the required `<= 0.97x`. Median
base and candidate times were 1.6493s and 1.6235s; all pair ratios were
0.974149-0.987993. The A/B receipt SHA-256 is
`431ac268b16478c7b16e3fca7d0adc171f19c1bfd72fe37f59c1667a6df5047a`.

Because the sole payoff target failed, the frozen controls and complete
promotion runs were unwarranted. The opcode, lowering, dispatch, diagnostics,
and focused tests were reverted; the checked-in runtime is source-identical to
the frozen base. The rejected prototype diff SHA-256 is
`08a6601734a3c7127e5ea5f892d0f173c597d1742ef4a1d18966d743dc173422`.
Do not retry this discarded binary-branch fusion or merely move its handler;
a successor dispatch proposal needs new exact evidence and a structurally
different mechanism capable of clearing the remaining roughly 1.4 percentage
points to the frozen threshold.
