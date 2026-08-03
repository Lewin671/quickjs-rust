# T025: Allocation-free RegExp backtracking

## Status: active staged migration

This is the current T018 structural unit selected by the exact T022 queue at
`c62314aa`. The runtime at current descendant `2a3fe1cd` is source-identical
and its release executable has the same SHA-256; that commit changes only the
performance decision controller and tests.

## Goal

Replace first-match RegExp execution's cloned `MatchState` graphs with one
mutable state, explicit choice points, a capture undo trail, and reusable
repetition scratch. Preserve ECMAScript leftmost-first, greedy/lazy, capture,
lookaround, backreference, Unicode, and reverse-capture semantics.

The frozen schema-2 plan is
`performance-units/regexp-choice-stack-capture-undo.json`. Its final fast gate
requires `string-tagcloud <= 0.80` candidate/base, every declared control
`<= 1.03`, and allocator samples at least halved. An improvement without the
allocation drop falsifies the attribution and requires a new profile before
widening.

## Current evidence

The current complete hosted preview ranks `string-tagcloud` second at 6.10x
candidate/QuickJS-NG. A 32-copy diagnostic sample of the hash-verified source
places 1,498 of 4,661 main-thread samples in `PreparedRegexp::match_input`;
`match_pattern_first`, `repeat_atom`, capture-vector clones, state hash keys,
and allocator entry points dominate that subtree. An independent
`string-validate-input` sample places 412 of 3,401 samples in the same path, so
it is a shared mechanism and a useful control, but not a payoff target.

## Stages

1. **Failure-atomic mutable first matching.** Reuse one state across candidate
   starts, alternatives, and simple-atom continuation choices. Keep
   capture-bearing compound atoms behind an owned compatibility bridge.
2. **Capture journal.** Route capture set/clear operations through checkpoints
   and an undo trail, including nested alternatives and lookaround isolation.
3. **Choice stack and scratch reuse.** Replace generic repeated-atom result
   graphs, capture-bearing state hash keys, and per-expansion work/child/dedup
   vectors with explicit ordered choices and invocation-local reusable storage.
4. **Complete first-match path.** Remove the compatibility bridge, re-profile
   allocation, and judge the ordinary final fast/promotion gates.

Every intermediate stage uses the fixed `c62314aa` migration base and the
schema-2 `stage` decision vocabulary. `advance` permits the next stage but is
not a performance claim.

## Stage 1 evidence

The new first-match module mutates the input index in place and guarantees
that `false` restores the entry state. Complex capture-bearing atoms remain
isolated behind owned candidate states, so this stage does not partially
migrate capture writes.

The amplified same-host screen against the byte-identical saved base reports:

| Case | Candidate/base | Observed range | Role |
| --- | ---: | ---: | --- |
| `string-tagcloud` | 0.9087 | 0.8856-1.0028 | cumulative target |
| `string-validate-input` | 1.0035 | 0.9837-1.0274 | control |
| `regexp-dna` | 1.0030 | 0.9898-1.0174 | control |
| `string-base64` | 1.0106 | 0.9962-1.0196 | control, 11-rep rerun |
| `controlflow-recursive` | 1.0030 | 0.9720-1.0420 | non-RegExp control |

The stage-1 candidate profile contains 4,302 main-thread samples. The RegExp
matching subtree falls from 32.1% to 27.2%. The four main malloc/free stack-top
groups fall from 25.0% to 22.3% in aggregate, an approximately 10.9% relative
drop. This confirms the mechanism direction while leaving most allocation in
the compatibility bridge for stages 2-3. Stage 1 is inside the 1.10 cumulative
budget; formal fixed-base portfolio classification remains pending complete
same-host evidence.

## Scope

- Allowed paths: `crates/qjs-runtime/src/regexp/matcher.rs`,
  `crates/qjs-runtime/src/regexp/matcher/**`, focused RegExp tests, this task,
  its performance-unit plan, and the task index.
- Forbidden paths: `third_party/**`, workload/source/pattern/input-specific
  admission, parser changes, observable RegExp protocol changes, FFI, global
  mutable scratch, and the protected environment/binding model.
- Owner boundary: serialize on one branch; matching state is shared by all
  RegExp builtins.

## Verification

For every stage:

```sh
cargo test -p qjs-runtime regexp::matcher::tests
./scripts/test262-subset.sh
./scripts/check-touched.sh --staged --explain
./scripts/check.sh
```

Run the predeclared amplified target and controls against the saved migration
base before advancing. At stage 4, re-profile `string-tagcloud`, require the
allocator reduction, and run the complete T022 fast/promotion decision.
