# T026: Compact hot-op error ABI

## Status: closed after mechanism and fast-gate rejection

This is the next T018 leaf unit selected from the exact T022 queue for runtime
candidate `2093eeab`. The current `39f3b530` descendant changes task documents
only and produces the same release executable. The frozen plan is
`performance-units/compact-hot-op-error-abi.json`.

## Goal

Make a successful hot out-of-line opcode handler return in one register rather
than through a caller-provided memory buffer. Keep `RuntimeError` unchanged at
all public and semantic boundaries; only the private handler result stores an
abrupt completion behind a non-null owned pointer and converts it back at the
dispatch boundary.

The unit covers the existing out-of-line local, global, property, coercion,
unary, update, and binary handlers that return `Result<(), RuntimeError>`. It
does not cover call, construction, return, suspension, `run_general_op`, rare
opcodes, or the global error representation.

## Why this unit is current

The queue from Performance Preview run `30849130056` has SHA-256
`d4aed5990fe8d59eb25558c681ffdedf91506e2b1b87b0682a61ad51c3ace3ec`.
Its first two external opportunities are HashMap at 7.3889x QuickJS-NG and
raytrace-public-class-fields at 6.0761x.

Fresh current profiles expose the same shared boundary:

| Case | `run_current_activation` exclusive samples | Queue rank |
| --- | ---: | ---: |
| HashMap | 561 / 2,543 (22.1%) | 1 |
| Raytrace | 171 / 1,365 (12.5%) | 2 |

The percentages are conservative exclusive lower bounds: the handler bodies
called by the loop are counted separately. On ARM64 the current function has a
448-byte stack frame. Before a hot property or arithmetic handler it computes
a stack address into `x0`, calls the handler, then reloads the returned
multiword `Result` discriminant from that memory. This is an ABI cost on every
success, not another dispatch-preamble, frame, property-cache, or value-clone
heuristic.

## Mechanism and proof obligations

Introduce a private transparent owned-error newtype and a private result alias.
`From<RuntimeError>` allocates only after an abrupt completion is produced;
`From<owned error>` moves the exact error back out at the dispatch boundary.
Existing `?` sites inside handlers therefore preserve the error payload while
the common success result uses the pointer's null niche.

The mechanism gate is all of:

1. a compile-time or focused runtime assertion proves the private result is
   exactly one pointer wide;
2. release ARM64 disassembly shows the covered handler calls no longer pass an
   sret buffer or reload their result from it;
3. `run_current_activation`'s release stack frame is smaller than the current
   448 bytes; and
4. focused throw/catch tests preserve thrown-value identity and diagnostic
   messages through property, coercion, binary, and binding failures.

If the timing gate passes without the ABI gate, attribution is falsified and
the unit is not retained.

## Frozen timing gate

HashMap and raytrace are both payoff targets and must each be at most 0.97
candidate/base. Date-format-xparb, access-binary-trees, CDjs, Tagcloud,
validate-input, recursive control flow, and the three current allocation broad
cases are controls and must each be at most 1.03. The attempt budget is one.
Promotion additionally needs complete broad and external evidence plus a
zero-gap exact Test262 receipt.

## Scope

Allowed implementation paths:

- `crates/qjs-runtime/src/bytecode/vm/general_ops.rs`
- `crates/qjs-runtime/src/bytecode/vm.rs`
- focused qjs-runtime tests for the private ABI and abrupt completions
- this task, its frozen plan, and the performance task index

Forbidden within this unit:

- changing `RuntimeError`, `Value`, or any public API;
- changing call, construction, return, generator, async, or rare-opcode result
  types;
- adding dispatch arms, opcode admission, or benchmark-specific conditions;
- changing property semantics, caches, environment ownership, VM frames, or
  operand cloning; and
- broad formatting or cleanup.

## Verification sequence

1. Validate the frozen unit against the exact queue before code.
2. Save and receipt the exact base release executable.
3. Add focused ABI and abrupt-completion tests, then run qjs-runtime tests and
   clippy.
4. Build release and inspect handler call sites plus the dispatch stack frame.
5. Run amplified same-host A/B targets and controls before requesting complete
   hosted evidence.
6. Record `retained`, `rejected`, or `inconclusive` from the exact frozen gate;
   do not retune the target list or threshold after timing.

## Result

The one-attempt implementation changed all sixteen covered handlers to a
pointer-sized `Result<(), HotOpError>`. A compile-time size assertion passed,
and a focused runtime test preserved thrown-object identity through property,
unary-coercion, and binary-coercion failures plus the native ReferenceError
diagnostic through an outer caller. Runtime clippy passed.

The release mechanism inspection was mixed and therefore failed the frozen
all-of gate. The covered ARM64 calls no longer computed a stack sret address:
they pass `Vm` in `x0`, receive the nullable owned-error pointer in `x0`, and
branch directly on `cbz x0`. However, `run_current_activation` retained its
exact 448-byte frame because the excluded `run_general_op`, call/return, and
other payload-bearing results still require the same maximum return buffer.
Its symbol interval also grew from 6,720 to 6,992 bytes because each compact
error result needs an exceptional branch back to the cold unbox path. The
mechanism gate required both sret removal and a smaller frame, so it did not
pass.

The standard release base and prototype executable SHA-256 values were
`cb5702a4ae650da9e6f8a39cc6e24ecc8290d9da69a8df3c873c142319eb32d9`
and
`91e2bcee5128d1b9adc48eaa161071fd67582a5518e36f6ef906597f83d71f4d`.
Eleven strictly alternating amplified pairs then rejected both frozen payoff
targets independently:

| Case | Candidate/base median | Observed range | Gate |
| --- | ---: | ---: | ---: |
| HashMap | 1.007270 | 1.000535-1.012563 | <= 0.97 |
| public-field Raytrace | 0.996292 | 0.988754-1.004635 | <= 0.97 |
| CDjs control | 0.996993 | 0.966791-1.001224 | <= 1.03 |

The complete 11-pair amplified SunSpider/Kraken diagnostic was likewise
neutral: 39 comparable cases had geometric mean 0.9949. The predeclared
external controls were date-format-xparb 0.9828, access-binary-trees 0.9948,
Tagcloud 0.9991, validate-input 1.0019, and recursive control flow 0.9972.
Every interval crossed 1.0. The worst corpus medians were math-partial-sums
1.0191, bitops-nsieve-bits 1.0138, and regexp-dna 1.0118, all below the 1.03
control ceiling.

The target failure makes the remaining broad controls, complete hosted
portfolio, and Test262 promotion evidence unwarranted. The runtime and focused
test changes were reverted; the checked-in engine is source-identical to the
base. Do not retry compact error pointers, handler selection, or unbox-branch
placement. A future completion representation would need to remove the
payload-bearing dispatch returns as a different, newly profiled structural
mechanism rather than widening this rejected leaf.
