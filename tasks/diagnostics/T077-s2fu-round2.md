# T077 S2 follow-up, lead feedback round 2

- Status: active; implementation present, final performance qualification in progress.
- Implementation: retained 4c6fc5c6 runtime plus a shared-entry witness fast path under current qualification. Entry/receiver/layout sizes are unchanged.
- Verified at: full native check on `4c6fc5c6` passes 2,524 runtime tests and 5,170 curated Test262 cases; all 268 NG fixtures pass cached/off/debug-verify. Final exact/resource standing is pending.
- Evidence: [iteration history](../archive/T077-s2fu-r2-iterations.md), [architecture](../../docs/design/core-rebuild-objects.md), named receipts below.
- Unresolved: final all-lane performance limits, complete cached/off exact scans with burndown, final native/x86 resource replays; mx2 source reconstruction caveat.
- Next action: focused shared-witness replay, then default-order relink/public86 and complete final qualification. Commit here only; no origin push or merge.

## Authority and targets

The lead's round-2 feedback explicitly authorizes runner-ref transport through
`imac-bench.sh`. Never cancel others' jobs; no iMac timing in local 02:45–05:00.
Performance baseline is `1e6e2a2b`; pre-S2 RSS reference is `d38f30a7`.
A requires every public86 lane instruction ratio <=1.00 and wall not worse.
B requires original mem1..mem7 <=1.10 pre-S2 peak, new owned-child patterns
<=1.50 (goal <=1.25), plus preserved speed. C requires iterative O(chain)
object/function checks without allocation; D corrects Full-storage seed comments.

## Retained implementation

The [prototype contract](../../docs/design/core-rebuild-prototypes.md) and
[shape contract](../../docs/design/core-rebuild-shapes.md) own the architecture.
The runtime retains direct own guards/private-layout epochs, 80-byte bound
nodes, prototype-independent tiny buffers, visible-prefix reads, reusable
canonical templates, live-root reuse with dead-binding sweeps, direct compact
root construction, and ownership-preserving bulk construction. NamedStore and
ordinary payload remain 64/96 bytes. No dependency, unsafe or environment-model
change is introduced. The function edit is confined to the cycle helper/tests.

## Qualification checkpoint

| Gate | Revision / standing |
| --- | --- |
| Fast | Runtime 4c6fc5c6: 2,524 tests; no focused filter matched the latest metadata files. Earlier C gate selected 52 function cases; full curated gate below covers the retained runtime. |
| Full native | 4c6fc5c6: passes, including 5,170 curated Test262 cases and feature configurations. |
| NG | 4c6fc5c6: 268/268 cached, IC-off and debug IC-verify (two workers, default ten-second timeout). |
| Public86 | `20261031T151127Z-t077-s2fu-r2-retained-public86`, candidate f05baa01, pending. |
| Exact cached/off | Not restarted on unaccepted source; partial superseded scans produced no burndown rows. |
| Native resources | Pending quiet replay. S3b overlap rejected the 76fbd142 batch; collector now recognizes clippy, runtime tests and check-touched. |
| x86 resources/RSS15 | Final retained-source replays pending. |

The default external/sentinel/broad order is generated from 4c6fc5c6 by job
`20261031T145539Z-t077-s2fu-r2-bound-comparison`; no development/held-out
selection or pin-offset search is used. [Image receipt](T077-s2fu-r2-comparison-order-image.json).
The final relink must pass strict audit before timing. Every earlier rejected
screen and constructor experiment remains in the iteration history.

## Memory source caveat

The lead did not supply the mx2 source. The retained reconstruction is a
500,000-node one-field chain, reproducing round-1 head peak near 639 MB versus
lead ~636 MB. [Program](t077-s2fu-r2/mx2.js); an additional
[two-field control](t077-s2fu-r2/owned-two-chain.js) is also measured. The exact
lead-source match is not claimed. The million-node owned chain and 250,000-parent
two-field fan are separately retained in the same diagnostic directory.


## Retained-screen decision and next guard stage

Job `20261031T151127Z-t077-s2fu-r2-retained-public86`, f05baa01: 430 valid,
zero invalid/heavy pairs. Wall/instructions: development 0.998221/0.997611;
external 1.002905/1.001250; broad 0.987326/0.998252;
sentinel 0.978639/0.996009. External remains open. The corrected native
collector accepts the first undisturbed retained-source replay; results are
under the native-retained receipt pending final source selection.

Next classify shared monomorphic own-data entries in an unused existing flags
bit at admission. Their metadata is immutable, so a direct identity guard
suffices, as in SharedOwnPoly; keep private/versioned entries on the original
guard and keep shared-own masks branch-free. Entry/CacheData/receiver sizes
must not grow. A dictionary-conversion test must retire the shared witness,
then prove private-epoch invalidation after delete/re-add. No cached JS values,
prototype snapshots or binding-model changes are introduced.

The witness fast gate passes 2,525 runtime tests, 65 selected Test262 cases and
clippy. Shared generic object guards retain the cold-seed fallback; only
monomorphic shared own-data hits omit epochs entirely. Versioned own hits keep
the original guard in a separate adapter; no private-dictionary or global
binding guard is weakened. The task-index budget issue from the archive
checkpoint is repaired in this commit and the fast gate is rerun successfully.

The shared-witness focused replay has no clear overall gain (date-tofte
1.004163, binary trees 1.008911, JSON parse 1.005448, diff 0.996534).
Collect measured operation/allocation counters on retained 4c6fc5c6 for
access-nsieve, binary trees, date-tofte and JSON parse before any further hot
adapter changes. The instruction-only suite was missing the largest residual
(nsieve 1.016722 in the retained full screen); add explicit case selection to
the existing diagnostic counter collector. Counters remain non-timing evidence.

Use a complete public86 screen of the shared witness as well as the focused
probes: the explicit target is aggregate per-lane neutrality, and the ten
probes do not cover the whole external lane. Apply only its generated default
order from `20261031T160025Z-t077-s2fu-r2-shared-witness`. The retained-source
operation-counter job is serialized ahead of that full screen.
