# T077 S2 follow-up: frozen qualification

- Status: frozen, qualifying under lead round-3 direction; A iteration is closed.
- Implementation: present at **40814de6**, including its existing default x86 order. Runtime/build inputs remain frozen; only qualification records may change.
- Verified at: 40814de6 full gate passes 2,525 runtime tests and 5,170 curated cases; NG default/IC-off/debug-verify each pass 268 fixtures. Complete exact and resource gates are in progress.
- Evidence: [early iterations](../archive/T077-s2fu-r2-iterations.md), [late iterations](../archive/T077-s2fu-r2-late-iterations.md), receipts below.
- Unresolved: complete cached/off exact scans with burndown; quiet native/x86 resources and x86 RSS15. The supplied mx2 source was reconstructed as described below.
- Next action: finish the lead's ordered qualification, record final standing, commit and stop for lead acceptance. Never push to origin or merge.

## Lead decision and frozen source

Round-3 feedback accepts the remaining external/sentinel instruction cost and
assigns recovery to S3e/S4. No more hot-path experiments or order searches.
Frozen source: `40814de6`; measurement baseline: `1e6e2a2b`; pre-S2 RSS reference:
`d38f30a7`. Runner-ref transport through imac-bench.sh is authorized. Never
cancel others' jobs; no iMac timing jobs in local 02:45–05:00. RSS15 runs without
--timing, as explicitly directed by the lead.

Existing job `20261031T161153Z-t077-s2fu-r2-witness-public86` has 430 valid,
zero invalid/heavy pairs; automatic verdict remains unclear, with lead acceptance
of the following reported standing:

| Lane | Wall / 1e6e2a2b | Instructions / 1e6e2a2b |
| --- | ---: | ---: |
| Development | 0.995108 | 0.999145 |
| External | 0.993689 | 1.002211 |
| Broad | 0.995292 | 0.999297 |
| Sentinel | 0.988814 | 1.001887 |

The [object](../../docs/design/core-rebuild-objects.md),
[prototype](../../docs/design/core-rebuild-prototypes.md) and
[shape](../../docs/design/core-rebuild-shapes.md) contracts own implementation
facts. No new dependencies, unsafe code or environment/binding snapshots.
The function edit remains confined to its cycle helper and tests.

## Ordered gates

| Gate | Frozen-candidate standing |
| --- | --- |
| 1. Full check | Pass: all feature configurations, 2,525 runtime tests, 5,170 curated Test262 cases. |
| 2. NG comparisons | Pass: 268 default, 268 IC-off, 268 debug IC-verify; verify uses two workers and the default ten-second timeout. |
| 3. Complete exact | Cached and IC-off pending; no burndown is recorded for partial scans. |
| 4. Native/x86 resources | Pending quiet full replays. |
| 5. x86 RSS15 | Pending; no --timing. |

The native exact pin is a release CLI with agents enabled and an actual agent
start/report smoke. Both modes execute the same image; only QJS_DISABLE_IC
changes. Burndown permits one row per commit: cached will name 40814de6; IC-off
will name the documentation-only qualification checkpoint, whose runtime,
configuration, submodule and harness inputs equal the frozen source. The
receipt records that equivalence and both modes' complete results.

## Memory source caveat

The lead did not supply mx2 source. The retained reconstruction is a
500,000-node one-field chain matching the reported round-1 head footprint
(~639 MB here versus ~636 MB supplied); [program](t077-s2fu-r2/mx2.js).
An additional [two-field control](t077-s2fu-r2/owned-two-chain.js) is measured.
The exact lead-source match is not claimed. The million-node owned chain and
250,000-parent two-field fan are separately retained alongside it.
