# T030: Packed typed-loop scalars

## Status: frozen before implementation

This T018 leaf unit is bound to the completed exact Performance Preview for
`e68abfc9`. The frozen plan is
`tasks/performance-units/packed-typed-loop-scalars.json`; its SHA-256 is
`b3270727b546ae6ed2b44a0fe7f5bf8aee04676b56178003bb61d207282823ee`.

## Goal

Replace the typed-loop tier's private 16-byte scalar enum with one safe
eight-byte packed word for Number, Boolean, and `undefined`, without changing
admission or any public runtime representation.

## Why this unit is current

Performance Preview run `30868378542` produced the completed exact queue for
`e68abfc9`; its SHA-256 is
`13493876129d438a3ecb1c758260f7836fda4798568f8c10b85a440a2f45b900`.
The first twelve external opportunities reproduce closed frame, call, object,
property, RegExp, string, constructor, and typed-loop admission mechanisms.
Rank-13 SunSpider `bitops-bits-in-byte` remains **3.8986x** QuickJS-NG, while
rank-21 Kraken iterative SHA-256 remains **2.6116x**.

Fresh current-runtime profiles isolate the same retained executor in both
cases. Bits-in-byte places 1,297 of 3,658 main-thread samples (**35.46%**) flat
in `typed_loop::execute::run`; iterative SHA-256 places 829 of 2,794
(**29.67%**) there. Their profile SHA-256 values are
`adb24ed6315943d0ec6aa6aaa9783ae43a2ca2759e3d27e15c5271cadc7dae6b`
and
`244180ec1e4fdd80f219242b21b2fbdf66c99ee0aebe0638c820492bd4034aee`.

A temporary feature-gated diagnostic build counted 519,680,000 typed
operations in bits-in-byte and 291,670,744 in SHA-256. Bits-in-byte used no
boxed operation at all. SHA-256 used only 56,868 boxed operations; its
227,969,348 numeric operations alone were 78.16% of the typed total. Layout
diagnosis reports `size_of::<Typed>() == 16` and
`size_of::<TypedOp>() == 12`. The width and tag handling of the scalar register
file, not another admission gap, is the shared boundary.

## Mechanism and proof obligations

The private scalar becomes a transparent `u64`-backed type. Ordinary f64 bit
patterns encode Numbers. Explicit quiet-NaN payloads encode false, true, and
`undefined`; every numeric NaN is canonicalized to a separate Number payload
before storage, so it cannot alias a non-number tag. This is safe Rust: no raw
pointers, pointer tagging, transmute, or `unsafe` block.

The mechanism gate requires all of:

1. `size_of::<Typed>()` falls from 16 to exactly 8 bytes and remains `Copy`
   without changing `size_of::<TypedOp>()`;
2. Number round trips cover finite values, infinities, positive and negative
   zero, canonical and non-canonical NaNs, while Boolean and `undefined` tags
   remain disjoint from every Number;
3. truthiness, `ToNumeric`, arithmetic, bitwise operations, comparisons,
   updates, dense reads/writes, helper results, boxing, writeback, and deopt
   materialization preserve existing results;
4. focused diagnostics report unchanged typed operation-family counts for the
   two frozen wrappers and no change in which loop plans run; and
5. default and `perf-counters` builds, qjs-runtime tests, focused Test262,
   QuickJS-NG comparisons, and the exact wrapper outputs all pass.

A timing win without the layout and path proofs is not attributable. A layout
win that misses either timing target is rejected rather than widened into
public `Value`, another execution tier, or an admission change.

## Frozen timing gate

Both `bitops-bits-in-byte` and iterative SHA-256 must be at most **0.90x**
their exact base. HashMap, public-field Raytrace, A*, CCM, PBKDF2, SunSpider
AES and SHA-1, three-bit bits-in-byte, spectral norm, recursive control flow,
broad local reads, dynamic array reads, object allocation, and dynamic method
calls are controls capped at **1.03x**. The attempt budget is one. Complete
promotion additionally requires the full broad and external portfolios plus
zero exact Test262 gaps.

## Scope

Allowed implementation paths:

- `crates/qjs-runtime/src/bytecode/typed_loop/`, including a small scalar module;
- qjs-runtime diagnostics and focused typed-loop tests;
- `docs/benchmarking.md`;
- this task, its frozen plan, and the performance task index.

Forbidden within this unit:

- changing public `Value`, `FastValue`, other loop tiers, object ownership, GC,
  parser, AST, bytecode IR, or typed-loop admission;
- unsafe code, pointer tagging, transmute, or a new dependency;
- changing deoptimization sites, boxed-register semantics, observable NaN,
  signed-zero, property, array, binding, or call behavior; and
- source-path, benchmark-name, function-name, input-size, iteration-count,
  checksum, or expected-result conditions.

## Verification sequence

1. Validate and commit this frozen unit against the exact queue.
2. Preserve the exact `e68abfc9` standard executable as the timing base.
3. Implement the private packed scalar and focused edge/layout tests.
4. Run qjs-runtime tests, Clippy with warnings denied, the exact wrapper
   outputs, and feature-gated operation counters.
5. Run warmup-then-alternating local A/B for both targets. Stop and revert if
   either target is above 0.90x or any control exceeds 1.03x.
6. Only after the fast gate passes, run complete broad/external evidence,
   exact Test262, `check.sh`, and `compare-qjs.sh`; retain or reject from the
   frozen decision contract without retuning.
