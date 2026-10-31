# T077 S2 follow-up, lead feedback round 2

- Status: active; implementation present, final performance qualification in progress.
- Implementation: runtime `4c6fc5c6`, exactly retained at `f05baa01` except x86 order/docs; no pending runtime edits.
- Verified at: full native check on `4c6fc5c6` passes 2,524 runtime tests and 5,170 curated Test262 cases; all 268 NG fixtures pass cached/off/debug-verify. Final exact/resource standing is pending.
- Evidence: [iteration history](../archive/T077-s2fu-r2-iterations.md), [architecture](../../docs/design/core-rebuild-objects.md), named receipts below.
- Unresolved: final all-lane performance limits, complete cached/off exact scans with burndown, final native/x86 resource replays; mx2 source reconstruction caveat.
- Next action: collect retained-source public86, then complete final qualification. Commit here only; no origin push or merge.

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
