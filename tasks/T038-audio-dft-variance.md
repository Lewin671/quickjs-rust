# T038: Numeric scratch placement

- Status: active. Lead-approved leaf unit, plan frozen before implementation.
- Implementation: none. Two implementation attempts available; diagnostic
  sweep is next. Batch-1 outcomes are retained, independently of this unit.
- Verified at: integration source 8d24e554, 2026-10-12; exact af4d0d3c
  queue binary profile and frozen-plan validation before runtime edits.
- Evidence: [plan](performance-units/numeric-scratch-placement.json),
  [origin diagnosis](archive/T038-placement-origin.md), and profile below.
- Unresolved: distinguish aliasing from cache-line/page splits; alignment
  cannot be chosen from one favorable offset. Formal promotion not measured.
- Next action: complete ASLR-off page sweep with address/span correlations,
  then choose the first policy and measure its entry cost and full screen.

## Contract and scope

Lead approval: 2026-10-12. Owner branch agent/numeric-scratch-placement/codex,
from integration 8d24e554. The lead explicitly authorizes the af4d0d3c queue;
plan base/receipt use that identity, never a relabelled documentation tip.

Allowed: dense numeric executor bank/storage helpers and focused tests under
crates/qjs-runtime/src/bytecode/vm_numeric_mutation_loop/dense/ and its parent
module declarations; this task, plan/index, owning runtime docs; CLI hot order
only if a screen demonstrates a pin repair is needed (separate commit).
Main checkout, other units and frozen evidence remain untouched.

[The frozen plan](performance-units/numeric-scratch-placement.json) owns the
complete target/control inventory and two-attempt gates. Report audio-dft,
audio-fft, beat-detection, oscillator, every imaging/math/nsieve case,
3d-morph, AES/MD5, ai-astar and all six sentinels individually. Both cycles
and instructions matter. A successful target also needs a 40-run varied-env
spread at least halved relative to the matched base and a median toward the
fast band. No dropped samples, favorable-environment selection or 0xdb0 pin.
An unstable target cannot pass through the new base-case rule.

## Profile and frozen inputs

Exact queue binary SHA-256:
6878520ff5fc8fc480e9fd8086c2b5caeb4ba609236ea423cd5a6d573a6fc54b.
Queue SHA-256: 2882db837308dca8db304e3d0de0b32efcb4326332fe218393020c2f9e275f26.
Native sample of that executable, with the frozen DFT repeated 400 times
only for profiling, records 1775/2238 samples (79.3%) in legacy read-only
Number run_program. This is inclusive path reach; it is not removable cost.
Profile command and hash-bound receipt are local under
 target/numeric-scratch-placement/profiles/audio-dft/receipt.json;
raw artifacts are not retained. Receipt hash is in the plan. No runtime
optimization was written before plan validation. Host reports 128-byte cache
lines and 16 KiB pages; diagnostics also report the requested mod-64/mod-4096.

## Attempts and verification

No implementation screen has run. First prefer an aligned engine-owned bank
reused across activations; second, only if necessary, over-aligned stack.
Choose relative backing-store geometry if the experiment demonstrates an
alias hazard; choose alignment if splits account for it. Separate register
bank, local bank and operation-stream effects. Verify per-entry cost by N/2N
on the same executing tier, and placements by nm. Re-pin only on screen proof.
