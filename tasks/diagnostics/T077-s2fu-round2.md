# T077 S2 follow-up, lead feedback round 2

- Status: active; A–D implementation present, final performance qualification in progress.
- Implementation: runtime `f9d6d86c`; 80-byte bound nodes/shared tiny data, canonical templates, live-root reuse/dead-root sweeps, cached hash width, visible-prefix own-data reads and allocation-free cycle checks. Default x86 order regenerated from this source.
- Verified at: fast gate passes 2,523 runtime tests; `f9d6d86c` passes 268 NG fixtures cached/off/verify. Native full check is running. `4e788129` native/x86 mem3 passes; final public86, exact and resource qualification remain due.
- Evidence: this record and the named job/artifact receipts below; [round 1](T077-s2-followup.md) is historical.
- Unresolved: final per-lane instruction/wall limits, exact cached/off with burndown, complete native/x86 resource gates.
- Next action: focused direct compact-factory replay, then strict relink/public86 and complete final exact/resource qualification. No origin push or merge.


## Authority and ordered scope

Runner-ref transfer through `imac-bench.sh` is explicitly authorized by the
2026-10-31 feedback. No push to origin, no merge into main, no cancellation of
others' jobs. Check status/time first; no iMac timing in local 02:45–05:00.
Base for performance is integration `1e6e2a2b`; baseline owner tip `3c6bec4e`.
C permits only the function-target cycle helper and its focused tests under
`function/`; the other S3b paths remain outside this owner's boundary.

Lead public86 lane wall/instruction ratios: development 1.0178/1.0138,
external 1.0040/1.0086, broad 0.9974/1.0111, sentinel 0.9973/1.0087.
Lead 15-pair development RSS geomean 1.0016 passes. Round 1 is not performance
accepted: ~1% added instructions must be removed.

## A target and migration

Own-data guards compare the directly published layout only; deferred empty
receivers cannot contain own data. Full/missing/inherited guards retain the
cold deferred-root fallback. Keep the four-way shared-own identity mask free
of seed tests. Move the dictionary epoch from receiver cold storage into its
private Layout, beside the metadata already loaded for identity guarding;
shared layouts keep epoch zero. The existing structural version/rollover
contract remains, and no hot payload grows. This also removes incidental cold
boxes on ordinary dictionary stores, keeping ordinary value writes on the
single absent-cold-pointer branch. Inline the inherited constant decoder into
its hit caller. Verify private-version invalidation, deferred guard semantics
and holder value-write barriers before committing and measuring.

B then makes tiny property metadata cheap and adds all three owned-child
patterns to the memory gate (<=1.5 pre-S2 peak, goal <=1.25). C then removes
allocation/recursion from both cycle walks. D then corrects the admission
comment. Final qualification covers full check, all NG modes, complete exact
cached/off with burndown, native/x86 memory/complexity and final public86.

## A first iMac screen

Job `20261031T100004Z-t077-s2fu-r2-a-public86`, candidate `d86f5c30` versus
`1e6e2a2b`: 430 valid pairs, zero invalid/heavy pairs, automatic verdict unclear.
Lane wall/instruction: development 1.000609/1.003846; external
0.995804/0.999885; broad 0.988923/0.999621; sentinel 0.971908/0.999080.
A's per-lane instruction target is still open for development. Largest residual
instruction ratios include string-key churn 1.01722 and diff 1.01681.

Source inspection found another unconditional addition on existing-key writes:
`NamedStore::lookup` checked/materialized a deferred seed before every probe.
Move publication to the missing-key define branch; pure missing probes need no
root either. Empty seeds and semantic roots share the untouched empty ID index,
so the exclusive miss proof remains valid. The new test proves a missing probe
allocates/publishes nothing and a subsequent define still publishes correctly.

A2 public86 job `20261031T102438Z-t077-s2fu-r2-a2-public86`, `70b6efbf`:
development 1.001942/1.003463; external 0.994150/0.999976;
broad 0.984990/0.999618; sentinel 0.979273/0.996129 (wall/instructions).
Counter job `20261031T102742Z-t077-s2fu-r2-a2-counters` uses the original public
argv and instrumented builds only, with identical stdout. In diff, base/head
shape creations are 125,800/647,782; transition hits 758,549/236,595; retained
nodes 37/1; block allocations 15,683,620/17,704,850. The remaining development
cost includes metadata churn/cache-budget pressure, not just hit guards.
A's development limit remains open; proceed to B's structural metadata repair,
then require every final lane to meet the unchanged A target.

## B target architecture and migration

Keep the 64-byte NamedStore and 96-byte ordinary payload. Reduce each bound
Layout to an 80-byte hot header: domain/budget/owner, property-data handle,
length/version/prototype/flavor flags, and one optional cold pointer. Move
parent/incoming/root links, transition tables and compact-edge memos to cold
metadata. Share prototype-independent zero/one/two-property metadata by leased
KeyId/descriptor sequence within the owner; no text/source recognition. Tiny
metadata probes linearly; allocate a hash index only past the tiny threshold.

Do not mint a per-prototype empty root just to write the first property.
Unobserved writes use a transient prototype-bound node; append nodes do not
retain their previous transient nodes. Own-key data is shared independently
of the prototype, while a bound layout still includes its exact prototype and
flavor. Observation/IC admission publishes the canonical root/transition path
before exposing guards or reusable constructor metadata. Private dictionary
versions and all write barriers remain unchanged. Cold guards protect a
version exposed before publication. Store sites that are disabled or already
megamorphic must not create unused before-write versions.

Stages: compact the header/cold fields; add tiny data sharing/linear indexes;
add transient binding and canonical publication; test sharing, mutation,
prototype separation and cache admission; measure the three owned-child
patterns plus mem1..mem7 before committing B's final candidate.

## B implementation and preliminary check

The header is statically asserted at 80 bytes (named/ordinary headers unchanged).
Cold links are allocated only for observed/canonical graph paths. Tiny leased
buffers are weakly interned across prototypes; linear probes avoid the ID-index
allocation through two entries. Transient appends retain no previous binding.
IC-disabled/megamorphic stores no longer capture unused before-write versions.
Root/template publication rebuilds only metadata, preserving values and slot
order; every structural edit clears a pending publication guard. Own-slot
admission also publishes already-Full array/function storage without forcing
empty authorities to allocate. These paths have focused tests.

Preliminary native single-run RSS (pre-S2 / B, decimal MB): owned-chain
184.37/216.25; owned-fan 83.44/91.13; reconstructed mx2 95.17/110.84.
The mx2 source was not supplied; a half-million one-field chain reproduces the
round-1 reported footprint (639 MB here versus lead ~636 MB), so that is the
explicit provisional gate. An additional half-million two-field control is
also retained (95.21/110.87 MB). Exact source clarification remains requested.
All meet <=1.25 in this preliminary check; final native/x86 pairs remain due.

B fast gate passes 2,514 runtime tests and 65 selected Test262 cases; clippy passes. The 80-byte header assertion compiles on the native target.


## C implementation target

Use one iterative allocation-free Brent checkpoint walk for object, array and
function targets. Normalize Map/Set carrier identities as before and stop at a
Proxy without traps. The function edit is confined to
`prototype_chain_contains_function_inner`; its existing wrapper's empty Vec
arguments perform no heap allocation and are unused by the new helper. No
function/env/call/context representation or behavior is changed. Tests cover
a host-created cycle, a target reached through mixed object/function edges,
and the reported 100,000-deep function-target stack-overflow case.

C passes 2,517 runtime tests and its fast gate, including 52 selected function Test262 cases.

D now documents the cleared/rebased Full-array/function seed case and its cold-guard fallback; it does not claim every empty authority is already published.

## B public86 and template publication repair

B job `20261031T111429Z-t077-s2fu-r2-b-public86` (`3a7e0a79`): 430 valid,
zero invalid/heavy pairs, verdict likely loss. Wall/instruction ratios:
development 1.014732/1.001395; external 1.001043/1.004772;
broad 0.988278/1.002776; sentinel 0.993556/0.998313. Limits remain open.
Counter job `20261031T111430Z-t077-s2fu-r2-b-counters` shows diff metadata
recovery: shape creations 125,800 base / 37,096 B; retained nodes 37/329;
block allocations 15,683,620/15,288,420. But ID probes increase
9,094,831/10,389,811. Reusable sequence and String-wrapper templates had kept
transient layouts; their instances repeatedly missed canonical own guards.
Publish these templates before retaining predictions/minting instances. Two
focused tests prove canonical prediction reuse and shared String template IDs.
The repair's fast gate passes 2,519 runtime tests (no mapped curated slice).

The B wall-over-instruction signature triggered the required default order
regeneration. Job `20261031T112639Z-t077-s2fu-r2-order` profiles the declared
default external/sentinel/broad set on frozen `c0f99d4e`, producing 464 symbols.
That intermediate order is retained; regenerate against the repaired template
source before final adoption rather than treating an old-image pin as current.

Template-repaired source `362d1bf0` passes all 268 NG fixtures in cached/off/
debug-verify modes, and eight focused cycle tests pass on x86_64. Final order
job `20261031T113502Z-t077-s2fu-r2-final-order` uses that exact source and the
default external/sentinel/broad set, producing 491 symbols at the unchanged
default pin offset. No held-out/development selection or offset search is used.
The generated order and image receipt are applied; final relink/audit and
public86 remain required.


## Final ordered candidate

Applied order at `68046c8a`, derived from job
`20261031T113502Z-t077-s2fu-r2-final-order` and runtime `362d1bf0`.
The final public86 job `20261031T114212Z-t077-s2fu-r2-final-public86` first
builds the exact canonical image and runs strict layout audit; that audit passes
and collection starts. Resource jobs `20261031T114323Z-t077-s2fu-r2-final-resources`
and `20261031T114323Z-t077-s2fu-r2-final-rss15` are serialized behind it.
Native pinned release/agents SHA-256:
`af703b95498c650b430f93a966c03ea03dbc2712abc6fea4950e8ba81c23c502`;
actual --agent smoke passes. Final exact scans execute this image with ICs
cached/off, four --all/engine-both partitions per mode, one worker per shard,
20-second timeout, no selection/exclusion changes.


Ordered candidate `68046c8a`, job `20261031T114212Z-t077-s2fu-r2-final-public86`:
strict layout audit passes (zero stale entries, declared wide offset 0x400),
430 valid pairs, no invalid/heavy pairs. Lane wall/instruction: development
0.998060/0.999644; external 0.992885/1.001476; broad 0.989289/1.001083;
sentinel 0.977227/0.998313. Wall targets pass, external/broad instruction limits
remain open. Do not label this accepted. The remaining correction removes
receiver-era `set_epoch(0)` work on immutable shared layouts after append/rebase
and in literal construction. Such layouts already carry epoch zero by
construction; private copies still initialize to one. Own guards explicitly
short-circuit the private epoch load when the expected shared epoch is zero.

The ordered resource job completes: owned-chain/fan/mx2 peaks are
215.04/90.98/109.29 MB versus pre-S2 187.19/83.41/93.62 MB (ratios
1.149/1.091/1.167). The reconstructed mx2 provenance caveat still applies.
All original memory cases except mem3 pass; mem3 churn is 6.30 versus 4.49 MB,
ratio 1.404, so B is not yet qualified. Empty roots were retained independently
in the owner's byte-bounded cache, keeping dead prototype Rc allocations alive
via weak identities. Keep empty roots weakly cached instead: live receivers and
retained property transitions already own roots that can be reused. The focused
compact-authority test now proves both the edge and unused binding disappear.
Class 2k/4k/8k wall seconds are 0.03/0.11/0.43 versus integration main
0.50/3.43/27.21; ch is 0.09 versus 11.46. These remain intermediate measurements.
Job `20261031T115955Z-t077-s2fu-r2-epochs-public86` measures the shared-epoch
cleanup; the empty-root correction needs its own final qualification.
The partial cached/off exact scans on superseded `68046c8a` were stopped after
its instruction gate failed; no burndown rows were recorded for partial scans.


The shared-epoch and weak-empty-root public86 preflights stop on stale compiler
alias entries in the previous order (jobs `20261031T115955Z-t077-s2fu-r2-epochs-public86`
and `20261031T120354Z-t077-s2fu-r2-weak-roots-public86`); neither contains timing.
Default-set order job `20261031T121128Z-t077-s2fu-r2-weak-roots-order` completes
from exact runtime `3c6ce3d7`, yielding 468 symbols. Apply this order before the
next strict audit/public86. The obsolete 68046c8a RSS replay is canceled after
four development cases; queued pre-relink resource jobs are also canceled.
Only this owner's jobs are affected, and partial results are not acceptance.

Native quiet three-pair replay at `3c6ce3d7`: mem3 is 5.69 MB versus pre-S2
6.16 MB; the other mem1..mem7 cases and all new owned-child patterns pass so far.
All 268 NG fixtures pass cached/off/IC-verify (debug verify uses two workers at
the original ten-second timeout after the initial parallel-load timeout).
An inline annotation on tiny/indexed probe dispatch gives no useful native
instruction reduction and is reverted without committing a runtime change.


Relinked `5b2728ef`, job `20261031T121533Z-t077-s2fu-r2-relinked-public86`,
passes strict audit and collects 430 valid pairs without invalid/heavy pairs.
Wall/instruction ratios: development 0.999139/1.007950; external
0.999616/1.006343; broad 0.995045/1.003246; sentinel 0.937821/0.998041.
A still fails. Restore the earlier direct `matches_layout` own-guard expression;
the explicit short-circuit experiment worsens the generated engine. Do not
restart expensive final scans until instruction diagnostics identify a candidate.
The partial cached/off scans on superseded 5b2728ef are stopped without burndown.
Native release at 5b2728ef exactly reproduces 3c6ce3d7 SHA-256
`b419ab0bde59761f486e48c7cc4c2fbd746ae7836478831cac81c938db75ec84`;
full check, 268 fixtures in all modes and a real agent start/report smoke pass.
The native resource replay overlaps the order-file fast gate and is rejected
for timing acceptance despite its load monitor missing the brief CLI tests.
A final quiet replay remains due. Diagnostic counter replays now choose measured
internal argv rather than their initial calibration sizes; a focused instruction
helper retains two N/2N pairs (startup-subtracted for internal cases), without
making acceptance or wall-time claims.


Focused instruction job `20261031T122853Z-t077-s2fu-r2-guard-instructions`
shows 61039679 essentially equal to rejected 5b2728ef, including captured-write
1.02833 and dynamic-read 1.06669 (68046c8a is 0.99996 / 1.04584).
The guard-expression spelling is not the cause. Next migration restores the
previous version initialization and live-prototype root retention, then sweeps
retained metadata whose weak prototype is dead. An owner countdown spaces
sweeps geometrically by live root count, only on new canonical root creation;
no new work enters own hits or unobserved transient appends. Test stable-root
reuse after the last receiver dies and bounded dead-binding churn before
measuring; retain 80-byte nodes and unchanged tiny shared-data/slot invariants.


Live-root job `20261031T123913Z-t077-s2fu-r2-live-roots`, runtime `4e788129`,
recovers the earlier profile: dynamic-read 1.045843 and captured-write 1.000059;
diff 0.999991, lodash 1.000887, meriyah 1.001367. x86 mem3 is 4.03 MB versus
4.49 MB pre-S2 (quiet three pairs), native 5.57 versus 6.16 MB. The new
liveness test and 2,520-test fast gate pass. Live-prototype reuse must remain.
The residual generic keyed lookup pays the tiny/indexed mode branch plus
recomputing the power-of-two hash shift on every indexed probe. Next stage
stores that capacity-derived shift in IdIndex, refreshing only on construction
or rebuild. Prefix clones preserve it with their bucket allocation; probe
capacity proofs and collision/deletion semantics stay unchanged. The bound
Layout/NamedStore sizes stay 80/64 bytes; LayoutData grows by eight bytes, and
its existing size-based accounting includes that change. Run collision, growth,
delete/reinsert and prefix tests via the fast gate, then focused instructions
and original memory before any final qualification.


Cached-width source `b731ea83`, focused job
`20261031T124537Z-t077-s2fu-r2-hash-width`: dynamic read improves
1.045807 -> 1.036185 and string churn 1.012467 -> 0.995199 versus 4e788129;
NG cached/off/verify all pass 268 fixtures. Other residuals remain, including
diff 1.002063 and JSON parse 1.008229. Next stage separates position-only
reads from consumable insertion proofs: indexed reads skip vacancy tracking and
return only Option<usize>; tiny reads remain linear. Exclusive writes keep
IdProbe and one probe per insertion. Prefix bounds and probe counters stay
unchanged. Existing collision/delete/reinsert/growth tests will exercise the
read path as well as insertion proofs before another instruction replay.


Position-only source `ebd7421c`, job
`20261031T125244Z-t077-s2fu-r2-position-read`, leaves dynamic-read unchanged
at 1.036172 and worsens string churn to 1.006811; the duplicate read loop is
removed. Its native full check and all 268 NG fixtures in three modes pass,
but it is not performance accepted. Next use the same linear mechanism through
four fields, promoting the fifth field to an index; interning remains through
two fields. This removes hashing for all narrow layouts, independent of source
or access sequence. A focused prefix test crosses promotion and then forks a
four-field prefix with a different fifth descriptor, exercising proof refresh
and hidden-tail rejection. Final instructions/resources still decide acceptance.


Threshold-only `269b3c5f`, job `20261031T130305Z-t077-s2fu-r2-linear-four`,
does not improve dynamic read (1.041006 versus b731ea83 1.036241). Keep the
allocation threshold, but choose own-data reads by Layout's visible prefix,
not the append-only buffer's physical width. Read narrow prefixes directly as
ShapeEntry; wide layouts necessarily have an index and skip the generic mode
branch. Both paths count exactly one probe and reject hidden shared-tail keys.
The promotion/fork test now checks direct data reads in both paths as well.


Visible-prefix `3b98d1e0`, job `20261031T131158Z-t077-s2fu-r2-visible-prefix`:
dynamic read improves 1.040868 -> 1.017671. Default profiles still sample the
entry_for_key call boundary. Next inline that adapter so read/write callers can
discard unused ShapeEntry fields. In a separate source checkpoint, remove the
redundant shared-epoch zero write only in from_slot_parts; dictionary copies
still initialize to one. Shared constructors/copies already initialize zero.
Measure both checkpoints independently before choosing the final source.


Paired adapter job `20261031T132140Z-t077-s2fu-r2-slot-adapters`:
inline-only 03e4b6be / plus-construction f9d6d86c instruction ratios are
0.974262/0.974288 on dynamic read, 0.998714/0.998712 on string churn,
1.000004/0.999999 on captured write. The inlined adapter removes the
remaining dynamic-read excess. Use f9d6d86c, which avoids shared construction
writes without changing private initialization. Its default external/sentinel/
broad order generation produces 467 symbols; apply it for strict relink audit
and a full public86 before declaring A accepted. All 268 NG fixtures pass
cached/off/verify on f9d6d86c; its native full check remains in progress.


Ordered `70bedf7c` public86, job `20261031T133013Z-t077-s2fu-r2-slot-public86`:
430 valid pairs, no invalid/heavy samples, strict audit passes. Wall/instruction:
development 1.001065/0.999139; external 0.998117/1.001694;
broad 0.986517/0.998244; sentinel 0.960250/0.995992. A remains open for external
instructions and development wall. Native f9d6d86c full check passes 2,521
runtime/5,170 curated cases, all NG modes pass, and its complete quiet resource
replay passes all original/new RSS limits (owned ratios 1.173/1.091/1.165).
No final exact scans were restarted on this unaccepted source.

Next architecture step packs deferred/transient publication state into one
byte enum while retaining the dictionary flag. Ready and private-transient
states need no publication; deferred seed and shared transient states do.
Copying into a dictionary preserves root-on-demand behavior but makes the
private version directly guardable. This replaces the multi-flag predicate on
construction/guard publication with one state-range test, without growing the
80-byte Layout or changing any slot/value/prototype semantics. Migrate all
constructors and flag reads together, then run guard/prefix tests and focused
instructions before new final qualification.


Publication-tag `7a3a1108`, job
`20261031T134556Z-t077-s2fu-r2-publication-state`, passes full native/NG gates
but loses instructions: lodash 1.016421, meriyah 1.007601, captured write
1.028373 (JSON parse improves to 1.005406). Revert the tag and retain the
private-version regression test. Next remove the compact-authority factory's
ordinary-seed/publication round trip: Empty authorities require canonical roots
by invariant, so request root_for directly and retain the existing compact edge
memo. No temporary NamedStore, cold guard or unused neutral seed is needed.
Test that construction remains Empty, can guard directly and creates no seed.


Direct compact factory `76fbd142`, job
`20261031T135516Z-t077-s2fu-r2-compact-direct`: focused ratios are lodash
0.999272, meriyah 0.999697, diff 0.999261, date-tofte 1.002786, dynamic read
0.974129 and string churn 0.998686. Apply its default-set order (482 symbols)
and require all four full lanes, rather than extrapolating the focused cases.


Direct-factory ordered `1744d8a9`, job
`20261031T140901Z-t077-s2fu-r2-compact-public86`, records wall/instructions:
development 0.995563/0.998079; external 0.994878/1.001128;
broad 0.996384/0.998250; sentinel 0.989893/0.996013. External instructions
remain open. The native 76fbd142 replay is rejected: other S3b test shells
were detected. Its RSS numbers are diagnostic only. Extend the native collector
to recognize cargo-clippy, clippy-driver, runtime-test binaries and the whole
check-touched shell; do not cancel the other owner's work. Earlier frozen
collector hashes describe their named revisions, not this amended script.

Next preserve the caller-owned prototype in DataSequence's predicted phase
and move it into the completed store or mismatch prefix. Reacquiring the same
prototype from weak metadata adds dispatch/refcount work, and predictions must
not own JS values after construction. Keep the cached layout weak with respect
to prototypes; only the live builder/receiver owns the value. Add a lifetime
regression covering an otherwise-unowned prototype and both predicted/mismatch
completion before measuring this constructor path.


Owned-builder source `674891f8`, job
`20261031T142943Z-t077-s2fu-r2-owned-prototype`, reduces JSON parse
1.010808 -> 1.006605 and diff 0.999271 -> 0.997952 while keeping dynamic read
0.974203. The literal factories have the same redundant reacquisition: both
with_literal_properties and with_literal_pair already own the exact prototype
passed to linked_layout. Move that edge into NamedStore as well and remove the
weak-reacquiring slot wrapper. Private dictionary copies still initialize their
own metadata/version; cache layouts continue owning only weak prototype links.
The external limit remains unchanged; measure both bulk-construction checkpoints.


Literal-owned prototype `1634dcca`, job
`20261031T144202Z-t077-s2fu-r2-literal-prototype`, improves meriyah to
0.998372 and diff to 0.997625; constructor-heavy external residuals remain.
All matches_prototype callers are construction caches: empty_cached publishes
before caching, literal links use layout(), DataSequence publishes predictions,
and constructor capture uses shared_layout(). Therefore cached inputs cannot be
neutral seeds. Keep this precondition as a debug assertion, inline the guard,
and directly match optional weak/actual identities without building Option
identity tuples. This removes the follow-up's seed branch from this second
hot guard family while preserving flavor/domain/prototype identity checks and
all publication requirements at admission. No function/env consumer is edited.


Forced-inline guard `7f453e84`, job `20261031T144922Z-t077-s2fu-r2-bound-guard`,
adds work broadly (lod 1.014941, meriyah 1.006394, captured write 1.028319).
Remove forced inlining and restore the prior comparison expression, keeping
only the release seed-check removal and debug cache-admission assertion.
This isolates the redundant flag test without expanding constructor callers.


Non-inlined bound guard `4c6fc5c6`, job
`20261031T145539Z-t077-s2fu-r2-bound-comparison`, restores the earlier profile
(lod 0.999281, meriyah 0.998479, JSON parse 1.006059).
The next isolated repair removes set_epoch(0) after installing AppendRecipe /
AppendBatchRecipe.after. Both plans admit immutable shared layouts, whose epoch
is already zero; the setter merely repeats a dictionary-mode branch on every
cached append. The older broad epoch experiment was confounded by root-cache
retention changes; this retains live-root reuse and changes only the two append
plan publications. Existing append/invalidation tests cover the shared-version
and prototype-holder contracts; final hardware measurements still decide A.


Shared-append experiment `da1d0367`, job
`20261031T150432Z-t077-s2fu-r2-shared-append`, adds work broadly (lod
1.015174, meriyah 1.006593, string-fasta 1.017934). Restore the two calls.
Retain runtime inputs exactly equivalent to `4c6fc5c6`; apply its default-set
order from `20261031T145539Z-t077-s2fu-r2-bound-comparison` and run full public86.
The retained source includes both prototype ownership repairs and the bound
cache guard, which have not yet had a complete aggregate screen together.
