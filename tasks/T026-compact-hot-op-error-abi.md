# T026: Compact hot-op error ABI

## Status: planned from the exact current queue

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
