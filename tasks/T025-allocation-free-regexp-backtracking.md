# T025: Allocation-free RegExp backtracking

## Status: active staged migration

This is the current T018 structural unit selected by the exact T022 queue at
`c62314aa`. The evidence-only controller commit `2a3fe1cd` is runtime- and
release-binary-identical to that migration base. Stage 1 is `6da38970`; Stage 2
continues from it without changing the fixed migration base.

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

Trusted-main Performance Preview `30817505652` compared exact Stage 1
`6da38970` with runtime-identical parent `2a3fe1cd`. All 225 broad measurements
and three blocks were valid. The declared external target and controls were
inside the 1.10 stage budget: Tagcloud 0.9772, validate-input 0.9866,
regexp-dna 0.9823, base64 0.9958, recursive 0.9905, A* 0.9945, HashMap 0.9974,
and raytrace 1.0022. Broad object allocation and closure allocation were
0.9783 and 1.0014. This is exact-parent hosted evidence, not the required
fixed-base decision.

Two direct fixed-`c62314aa` local preview attempts were inconclusive under the
formal broad protocol. The first invalidated every block on timer-limited
`plain_function_call` and `captured_write` records; the retry hit the same
host ceiling on six other candidate cases. The decision controller correctly
rejects the otherwise complete hosted bundle with `preview summary base SHA
does not match performance unit`. No formal `advance` artifact is claimed.
The source- and binary-bound amplified fixed-base target/control screen and the
hosted exact-parent screen both exclude the 1.10 abort condition, so the
migration continues while a trusted arbitrary-fixed-base hosted lane remains
an evidence-system follow-up.

## Stage 2 evidence

Stage 2 moves ordinary exact-once and optional groups plus lookahead assertions
onto a capture undo journal. Append-only continuation frames preserve the
outer sequence while nested alternatives retry; every recursive boundary
restores the input index, capture writes, and speculative frames on failure.
Positive lookahead retains the first body's captures but stays atomic and
zero-width, while negative lookahead rolls every body capture back. Optional
group capture clears use the same journal. Quantified groups beyond one and
lookbehind remain on the compatibility bridge for Stage 3.

Focused coverage proves that a failed nested alternative cannot leak capture
1 into a successful capture-2/backreference path, and that a negative
lookahead discards a capture written before its body later fails. All 45
matcher tests and qjs-runtime clippy pass.

The first full post-push Test262 coverage run exposed one Stage 2 regression:
`test/built-ins/RegExp/lookahead-quantifier-match-groups.js`. An optional
compound group whose body succeeds without advancing must reject that
one-repetition branch when its minimum is already zero, then take the
zero-count continuation with the nested captures cleared. Continuation frames
now carry that progress guard at the group boundary. The exact upstream case
passes in both quickjs-rust and QuickJS-NG, with focused coverage for `?`,
`{0,1}`, unquantified, and `{1,1}` forms.

The standard-recipe Stage 2 and fixed-base executable SHA-256 values are
`b757712e2a31bb8ecc470e0333560e591dff9f158f9b100ac455818deda276b4`
and `922de75a13296fb7049fccc840de72d94466b32854ea301b5129e25eb290e4bb`.
An amplified 11-pair Tagcloud screen is 0.8705 fixed-base
[0.7977, 0.8966]. The direct Stage-2/Stage-1 increment is noise-bound at
1.0083 [0.9187, 1.0647], as expected for a representation stage rather than a
leaf payoff claim.

The complete three-block external diagnostic against the fixed binary reports:

| Case | Stage 2 / fixed base | Role |
| --- | ---: | --- |
| `string-tagcloud` | 0.8676 | cumulative target |
| `string-validate-input` | 0.9473 | RegExp control |
| `regexp-dna` | 0.8129 | RegExp control |
| `string-base64` | 0.9574 | string control |
| `controlflow-recursive` | 1.0429 | non-RegExp control |
| `ai-astar` | 1.0006 | Kraken control |
| `hash-map` | 1.0002 | JetStream control |
| `raytrace-public-class-fields` | 1.0137 | JetStream control |

The first shorter recursive diagnostic read 1.1182, but the required longer
11-pair rerun was neutral at 0.9992 [0.9940, 1.0290]. Eleven-pair direct broad
screens put `object_allocation` at 0.9985 [0.9830, 1.0026] and
`closure_allocation_call` at 1.0094 [0.9894, 1.0171]. Every declared watched
case is therefore below the 1.10 stage budget. External suite geometric means
are 1.0023 for JetStream, 0.9979 for Kraken, and 0.9806 over the 25 mutually
comparable SunSpider cases. This remains diagnostic rather than a promotion
claim because SunSpider comparison coverage is incomplete and the broad
fixed-base formal report is host-inconclusive.

External raw/report SHA-256 are
`96abc83d77625328a374442f3e3bb9bf82c855c8c3ddb19f677dc092ce58e4a6`
and `8ae8d3bff6bde818d661afc4e329d2e695e0f8e82d464bbe7b9c3ee1e5f41d92`.

The first trusted fixed-base dispatch, run `30824684273`, failed before setup:
the ancestry guard ran inside the base checkout, which contained the selected
ancestor but not the candidate object. The guard now runs in the full-history
candidate checkout; the separately checked-out base remains exact and shallow.
This run contains no timing evidence and cannot influence the stage decision.
Retry `30825469936` exposed that the GitHub expression `&& 0 || 1` evaluates
to `1` because numeric zero is false, so the candidate checkout was still
shallow. The workflow now returns string depths `'0'` and `'1'`; a regression
assertion rejects the false-zero expression. This retry also contains no timing
evidence.

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
