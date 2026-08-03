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

Trusted fixed-base run `30826178138` then completed successfully with 3/3
valid broad blocks, complete external comparisons, and the exact migration
base. Formal Stage 2 decision SHA-256
`38476819d822eb52add7756cb047262627c46da85773119ec5ec8d60ed428fca`
is `inconclusive`: the frozen plan misspelled the existing canonical broad
control `closure_allocation_call` as nonexistent `closure_allocation`. The
available watched ratios were all below 1.10, including Tagcloud 0.8762,
regexp-dna 0.7429, raytrace 1.0277, and object allocation 0.9739; these do not
constitute `advance` while one declared ID is missing. The checked-in plan now
uses the canonical ID already named by the pre-measurement T025 diagnostic.
This clerical correction is frozen before a new hosted run; the completed run
is retained only as inconclusive evidence and is not reclassified.

The run's preview summary, broad report, and external report SHA-256 values are
`6e187ac26e721b84c347d7ee628b0213e3c9f64f3215e5d3464e4377f5adcf78`,
`0920666358882dec0aa5f9a748b9f06443e8d591e92017d27dea2b85121d6832`,
and `789ec55f336fb341bf9fbf4900b81b85b79dc599539b3c17150f6f301fb41a3d`.

Corrected fixed-base run `30828792160` measured candidate `7ab0814d` against
the same `c62314aa` migration base. Formal Stage 2 decision SHA-256
`e1d66d60a350811af340dd51441b3b77b8fa3bb6fa0f56ab9e03411ad5e3000f`
is `advance`: Tagcloud is 0.8912 candidate/base and every declared control is
inside the 1.10 cumulative stage budget. The largest watched ratio is
`controlflow-recursive` at 1.0334; HashMap is 1.0284, raytrace 1.0022,
`ai-astar` 1.0075, validate-input 0.9624, regexp-dna 0.7193, base64 0.9710,
object allocation 0.9813, and closure allocation call 0.9656. The plan now
advances to Stage 3; this classification earns the right to continue and is
not a performance claim.

The workflow-level preview remains inconclusive: all 3/3 broad blocks and all
watched ratios are valid, but an unrelated QuickJS-NG `local_read` linearity
probe read 1.1707 outside the frozen 0.85-1.15 interval, and the maximum broad
relative half-width was 0.03283 against a 0.03 claim limit. The stage contract
deliberately classifies only the complete predeclared cumulative target and
controls; this distinction is retained rather than presenting the hosted
summary as a broad performance direction. The rerun's preview summary, broad
report, and external report SHA-256 values are
`31105b796d92898a5b9aac70afbd8edf6402ea8cfb572e8b552777de2ea90eca`,
`149ce9d642155243968822c9c34df6db078b223f4c8dea2ef692ee633a010a74`,
and `7f3c8500d4972011128ceca563a8eef24a5bcc09319243bdb06e10f1e4ec22f1`.

## Stage 3 evidence

The first Stage 3 slice gives one `FirstMatcher` the lifetime of an entire
`PreparedRegexp::match_input` invocation. Failed candidate starts and
top-level alternatives now retain the capacity of the continuation and
capture-undo journals instead of constructing fresh vectors. The failure
contract asserts that both journals are empty before reuse; successful
matching still returns immediately and drops invocation-local scratch.

The Stage 2 and slice-candidate release executable SHA-256 values are
`b757712e2a31bb8ecc470e0333560e591dff9f158f9b100ac455818deda276b4`
and `e46ea94a7eed5f005a5c0e9428a73c01d6966e5adebb399e73001ebbcc871033`.
An 11-pair amplified Tagcloud A/B is noise-bound at 1.0018 Stage-3-slice/Stage
2 with an observed range of 0.9508-1.0336. This slice establishes reusable
storage for the ordered repetition work but does not claim a speedup by
itself; generic repeated-atom state graphs still remain behind the bridge.

The second Stage 3 slice streams generic repeated-atom accept states directly
into the surrounding continuation instead of first materializing the complete
result vector. One invocation-local scratch object retains the ordered work
stack and expanded-state set across candidate starts and alternatives. The
bridge still owns `MatchState` choices and capture-bearing hash keys; removing
those clones is the next slice rather than a claim that Stage 3 is complete.
Focused greedy and lazy compound-group coverage verifies that a failed trailing
continuation retries the first viable accept state without losing the retained
iteration's captures. All 47 focused matcher tests and runtime clippy pass.

The first-slice base and streaming-candidate executable SHA-256 values are
`e46ea94a7eed5f005a5c0e9428a73c01d6966e5adebb399e73001ebbcc871033`
and `7acf88f06971cc0667f98f70051acf3ff947fd1c4a91e6f8397e69f8b6304b4b`.
Eleven-pair amplified screens are noise-bound: Tagcloud is 0.9909
streaming/first-slice [0.9634, 1.0592], while regexp-dna is 1.0073 [0.6792,
1.3120]. An earlier inlined build regressed regexp-dna to a 1.0703 median;
symbol inspection showed the choice walker folded into hot `match_pattern`.
Keeping the generic walker out of line restored the control to neutral. This
is a code-layout guard and a structural migration slice, not a speedup claim.

The third Stage 3 slice moves forward quantified groups off the legacy
all-state matcher. A continuation can now target either the surrounding
pattern or a depth-indexed reusable result slot. That lets one group iteration
enumerate through the mutable `FirstMatcher` and capture journal, including
nested repeated groups, while the outer explicit work stack retains
greedy/lazy order and stops after the first successful full continuation.
Lookbehind remains on the reverse compatibility path. Owned choice states and
capture-bearing expanded keys remain for the next slice, so Stage 3 is still
open.

The pre-change profile (SHA-256
`0559960a89be4730d54afe84f76b46017b93729457f503f80501c0657194c81f`)
contains 4,069 main-thread samples. `PreparedRegexp::match_input` accounts for
759; the streamed repetition bridge accounts for 122, of which 113 enter
legacy `match_atom` and 110 reach legacy all-state `match_pattern`. The new
profile (SHA-256
`dc0faefef0af4d4f107b277d80c701b400fa8046e68c96918b5dd7c4eccbaf3d`)
contains 11,428 main-thread samples under heavier host load. Its RegExp path is
1,721 samples and its new quantified-group walker is 195; the legacy
all-state matcher is absent from that subtree. Both samples use the same
40-copy source, SHA-256
`1a3650d435575a4497a6143749d6fde6b32bc1178ac9088d18ca713121cc3fe5`.
These profiles establish route removal, not a cross-run timing ratio.

The exact prior and new release executable SHA-256 values are
`7acf88f06971cc0667f98f70051acf3ff947fd1c4a91e6f8397e69f8b6304b4b`
and `e52be1164dae878b1cf2347d5eb0acda895c071194e435a06fd83f90e04b0ebc`.
Eleven-pair exact-increment screens remain noise-bound and inside the 1.03
median control ceiling: Tagcloud 0.9915 [0.9088, 1.0949], regexp-dna 0.9960
[0.9904, 1.0357], validate-input 1.0134 [0.9965, 1.0536], and base64 1.0099
[0.9907, 1.0287]. As with the preceding representation slices, this is not a
speedup claim. Keeping the large quantified-group walker out of line is again
required to avoid folding cold choice logic into hot `match_pattern`. All 48
focused matcher tests, 2,029 runtime tests, 5,169 curated Test262 cases, and
the QuickJS-NG fixture comparison pass.

The fourth Stage 3 slice reuses simple-atom boundary buffers by active
continuation depth. This preserves an outer quantifier's greedy/lazy retry
order while a nested continuation scans into a separate slot, and retains
capacity across candidate starts for the lifetime of one `FirstMatcher`.
The preceding profile, SHA-256
`dc0faefef0af4d4f107b277d80c701b400fa8046e68c96918b5dd7c4eccbaf3d`,
places 35 of 86 inner `match_pattern` samples in
`simple_atom_boundaries`, including 28 direct boundary-vector grow/realloc
samples below the 195-sample quantified-group walker. That measured allocator
cost supersedes the earlier tentative ordering that put capture-key clones
first.

The new profile, SHA-256
`2e3607e3969cf644a88d3ea1b5ec9799ab0f40e2d2d3389f4c708d53f18444c3`,
contains 4,956 main-thread samples. Its quantified-group walker has 50 samples
and no boundary-buffer grow/realloc descendant; nine first-capacity growth
samples remain in the direct top-level simple-atom path. The different host
load and sample count prohibit treating this as a timing ratio, but the
targeted allocation route is removed. Both profiles use the same 40-copy
source SHA-256
`1a3650d435575a4497a6143749d6fde6b32bc1178ac9088d18ca713121cc3fe5`.

The exact base and boundary-reuse release executable SHA-256 values are
`e52be1164dae878b1cf2347d5eb0acda895c071194e435a06fd83f90e04b0ebc`
and `2d46be64f099b9ff3132cd2871cb2f1f455146b9ada05a1ae6c53c7c75aeb1ab`.
Eleven-pair amplified screens remain noise-bound and inside the 1.03 median
control ceiling: Tagcloud 0.9867 [0.9152, 1.0115] at four copies, regexp-dna
0.9963 [0.9771, 1.0171] at four, validate-input 0.9990 [0.8014, 1.0169]
at ten, and base64 0.9986 [0.9379, 1.0966] at ten. This slice removes a
measured repeated allocation without claiming a timing improvement; owned
choice states and capture-bearing expanded keys remain Stage 3 work. All 49
focused matcher tests, 2,030 runtime tests, 5,169 curated Test262 cases, and
the QuickJS-NG fixture comparison pass.

The fifth Stage 3 slice makes group-alternative discovery a lazy range
iterator. Prepared patterns still collect their top-level alternatives once,
while group, lookaround, and reverse-matching hot paths now consume ranges
without constructing a temporary vector. A direct iterator test covers
nested groups, escaped pipes, character-class pipes, and empty alternatives.

The boundary-reuse profile above contains 31 samples in
`group_alternatives`, 30 of which directly grow its temporary vector. The new
profile, SHA-256
`7fe47ad7d7a239892f8eb378e780afb42183724b06d16b16800312bbc5c6333a`,
contains 4,894 main-thread samples and no `group_alternatives` allocation
descendant anywhere below `PreparedRegexp::match_input`. Different host load
again prevents a timing-ratio interpretation; this establishes removal of
the measured route only. Both profiles use the same 40-copy source SHA-256
`1a3650d435575a4497a6143749d6fde6b32bc1178ac9088d18ca713121cc3fe5`.

The exact base and lazy-alternatives release executable SHA-256 values are
`2d46be64f099b9ff3132cd2871cb2f1f455146b9ada05a1ae6c53c7c75aeb1ab`
and `84da42beb6a60e156c725efa9fb329a3be7f5d2347e66d1f71d2cbf13b0233d9`.
Eleven-pair amplified screens remain noise-bound and inside the 1.03 median
control ceiling: Tagcloud 0.9863 [0.9072, 1.0828] at four copies, regexp-dna
0.9997 [0.9110, 1.0236] at four, validate-input 0.9955 [0.9323, 1.0673]
at eleven, and base64 0.9953 [0.9741, 1.0107] at ten. This is another
allocation-removal slice, not a timing improvement claim. All 50 focused
matcher tests, 2,031 runtime tests, 5,169 curated Test262 cases, and the
QuickJS-NG fixture comparison pass.

The sixth Stage 3 slice replaces capture-bearing `HashSet` keys with reusable
exact visited slots. Each state computes a cheap fingerprint into a standard
randomized map, then follows a collision chain and compares the full input
index, repetition count, and capture vector. Fingerprint collisions therefore
cannot change matching semantics. Slot capture buffers and both map and slot
capacity survive failed candidate starts, while lookup remains approximately
constant-time instead of turning large repetition graphs into a linear scan.
A focused test proves that two states at the same index and count but with
different captures remain distinct when a following backreference observes
the difference.

The preceding profile places eight of the 30 sampled quantified-group walker
frames in capture-bearing `HashMap::insert` and its SipHash descendants. The
new profile, SHA-256
`46c1f14c41fe82c11e41405b1366bff9cc4427b3fd99c9120f5356089ad04f9d`,
contains 3,831 main-thread samples. `PreparedRegexp::match_input` accounts for
521 and the quantified-group walker for 60. Only two walker samples enter
`RepeatVisited::insert`, including one integer-key map insertion; none hash or
grow a capture buffer. This comparison uses the same 40-copy source SHA-256
`1a3650d435575a4497a6143749d6fde6b32bc1178ac9088d18ca713121cc3fe5`
and establishes removal of the measured path rather than a cross-profile
timing ratio.

The exact lazy-alternatives base and visited-slot candidate release executable
SHA-256 values are
`84da42beb6a60e156c725efa9fb329a3be7f5d2347e66d1f71d2cbf13b0233d9`
and `b502de5b86eb32c5174fb1f7519bfa85a138f0234936542b2c4dd200ca017afe`.
Eleven-pair exact-increment screens remain noise-bound and within the 1.03
median control ceiling: Tagcloud 1.0150 [0.9869, 1.0934] at five copies,
regexp-dna 1.0001 [0.9816, 1.0610] at six, validate-input 0.9965
[0.9081, 1.0933] at nine, and base64 0.9291 [0.8478, 1.0078] at nine.
This slice removes the capture-bearing visited-key allocation route without a
timing improvement claim. Owned `MatchState` choices remain Stage 3 work.

The seventh Stage 3 slice reuses one top-level `MatchState` across candidate
start positions. Failed first matching is already failure-atomic; before each
new candidate the matcher now resets the input index and fills the retained
capture buffer with `None` instead of allocating and freeing another buffer.
A focused regression first writes capture 1 before an alternative and the
entire candidate start fail, then verifies that the next start succeeds with
only capture 2 set.

The preceding profile (SHA-256
`46c1f14c41fe82c11e41405b1366bff9cc4427b3fd99c9120f5356089ad04f9d`)
contains 3,831 main-thread samples. Below its 521-sample
`PreparedRegexp::match_input` subtree, the per-candidate state lifetime has 11
direct allocation samples and 15 direct free samples. The new 4,722-sample
profile (SHA-256
`dd9b4420c2f8c95538cbecf16c97ed6b246c9c0859df190984b715c616693016`)
contains 637 `match_input` samples and no corresponding per-candidate capture
allocation or free descendant. Matcher destruction still frees invocation-
local journals and scratch once after matching finishes. Both profiles use the
same 40-copy source SHA-256
`1a3650d435575a4497a6143749d6fde6b32bc1178ac9088d18ca713121cc3fe5`.

The exact base and candidate release executable SHA-256 values are
`b502de5b86eb32c5174fb1f7519bfa85a138f0234936542b2c4dd200ca017afe`
and `f16c08c7a066a54130cff358d959e4f8263f30c7ff63f87a106a053a3c1ebd01`.
An 11-pair fixed-source Tagcloud A/B is 0.9777 candidate/base and every pair
favored the candidate, with an observed range of 0.9498-0.9993. Eleven-pair
controls remain inside the 1.03 median ceiling: regexp-dna is 0.9890
[0.9583, 1.0103] at nine copies, validate-input is 0.9989 [0.9565, 1.0049]
at 17 copies, and base64 is 0.9991 [0.8651, 1.1565] at 28 copies. The base64
range is noise-bound, so only its predeclared median gate is used. This slice
removes a measured candidate-start allocation and records a local Tagcloud
improvement; the quantified choice stack still owns `MatchState` snapshots,
so Stage 3 remains open.

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
