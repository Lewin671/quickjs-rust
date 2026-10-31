# T077 S2 follow-up, lead feedback round 2

- Status: active; checked clean branch at `3c6bec4e`; executing A, then B, C, D.
- Implementation: A/B/C present, including both allocation-free cycle checks; D comment corrected; final qualification pending.
- Verified at: A increment on `3c6bec4e` passes 2,507 runtime tests and the fast gate, including 65 selected Test262 cases; iMac lane measurements pending.
- Evidence: this record; lead jobs `20261031T083411Z-t077-s2fu-public86` and `20261031T083411Z-t077-s2fu-rss15`.
- Unresolved: all round-2 acceptance gates.
- Next action: measure the B candidate and finish C/D; the final per-lane A target remains unchanged.

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
