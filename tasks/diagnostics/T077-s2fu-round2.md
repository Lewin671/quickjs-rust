# T077 S2 follow-up: frozen qualification

- Status: owner qualification complete; frozen under lead round-3 direction; final acceptance belongs to the lead. A iteration is closed.
- Implementation: present at **40814de6**, including its existing default x86 order. Runtime/build inputs remain frozen; only qualification records may change.
- Verified at: 40814de6 full gate passes 2,525 runtime tests and 5,170 curated cases; NG default/IC-off/debug-verify each pass 268 fixtures. Complete exact cached/IC-off, native/x86 resources and x86 RSS15 pass; receipts below bind the frozen images.
- Evidence: [early iterations](../archive/T077-s2fu-r2-iterations.md), [late iterations](../archive/T077-s2fu-r2-late-iterations.md), receipts below.
- Unresolved: lead final acceptance and integration; accepted residual instruction costs remain S3e/S4 obligations. The mx2 source caveat is below.
- Next action: owner stops; the lead runs final acceptance and merges. Never push to origin or merge from this branch.

## Lead decision and frozen source

Round-3 feedback accepts the remaining external/sentinel instruction cost and
assigns recovery to S3e/S4. No more hot-path experiments or order searches.
Frozen source: `40814de6`; measurement baseline: `1e6e2a2b`; pre-S2 RSS reference:
`d38f30a7`. Runner-ref transport through imac-bench.sh is authorized. Never
cancel others' jobs; no iMac timing jobs in local 02:45–05:00. RSS15 runs without
--timing, as explicitly directed by the lead.

Existing job `20261031T161153Z-t077-s2fu-r2-witness-public86` has 430 valid,
zero invalid/heavy pairs; automatic verdict remains unclear, with lead acceptance
of the following reported standing ([receipt](t077-s2fu-r3/public86.json)):

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
| 3. Complete exact | Pass: cached and IC-off each 42,672/42,672; zero failures/timeouts/not-run. Both complete scans have burndown entries. |
| 4. Native/x86 resources | Pass: 18 cases per host, three quiet alternating pairs per case; all RSS limits and output checks pass. Native and NG also pass the 100,000-function cycle replay. |
| 5. x86 RSS15 | Pass: `20261031T181931Z-t077-s2fu-r3-rss15`, done 0, without --timing; geomean 0.985046, maximum case 1.017111. |

The native exact pin is a release CLI with agents enabled and an actual agent
start/report smoke. Both modes execute the same image; only QJS_DISABLE_IC
changes. Burndown permits one row per commit: cached names 40814de6; IC-off
names documentation-only checkpoint 960e8923, whose runtime,
configuration, submodule and harness inputs equal the frozen source. The
[manifest](t077-s2fu-r3/exact/manifest.json) records that equivalence.
The release-agents image SHA256 is
`a4790dfc816aebbf5e15709dc69b26719f6010b3464e9d861c1286b9f5ab99fb`.
[Gate receipt](t077-s2fu-r3/initial-gates.json);
[exact validation](t077-s2fu-r3/exact/validated.json). Each mode covers all
53,572 unique pinned corpus paths: 42,672 configured passes and 10,900 configuration
skips; NG has 42,602 passes and 70 known failures. Mode maps are identical, with
canonical SHA256 `a633d502aaf8a01bf43641fb020f843eeeec372c604767ab1c6ebe3465c1b6aa`.
No exclusions changed. The pinned Test262 and NG revisions are in the manifest.

## x86 RSS15

Job `20261031T181931Z-t077-s2fu-r3-rss15` completed with exit 0, using
40814de6 versus 1e6e2a2b. It was submitted without --timing and waited behind
the lead's jobs; no other job was canceled. The standard ten development cases
each have 15 alternating pairs and passing output/checksum checks.
Peak RSS is collected with wait4. The automatic verdict is **pass**:
geomean **0.985046** (limit 1.05), maximum case
**1.017111** (`development/diff/compare`, limit 1.10).
[Full samples and image identities](t077-s2fu-r3/x86/rss15.json);
[submission](t077-s2fu-r3/x86/rss15-job.json).

## Memory and complexity qualification

Median peak RSS, decimal MB; columns are pre-S2 / accepted S2 / frozen follow-up.
The baselines are d38f30a7 / 1e6e2a2b, and all candidate images are 40814de6.
[Native samples and health](t077-s2fu-r3/native/resources.json),
[x86 samples and image identities](t077-s2fu-r3/x86/resources.json),
[validation](t077-s2fu-r3/resources-validation.json).

| Case | Native pre-S2 | Native 1e6 | Native frozen | x86 pre-S2 | x86 1e6 | x86 frozen |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| mem1 | 119.489 | 1143.783 | 118.899 | 117.858 | 1385.578 | 117.342 |
| mem2 | 58.294 | 287.031 | 54.395 | 57.975 | 299.958 | 54.202 |
| mem3 | 6.160 | 5.980 | 5.505 | 4.485 | 4.415 | 3.998 |
| mem4 | 162.316 | 394.199 | 158.433 | 166.490 | 411.111 | 162.722 |
| mem5 | 5.964 | 5.456 | 5.358 | 4.280 | 3.813 | 3.781 |
| mem6 | 46.891 | 201.671 | 46.301 | 45.707 | 206.610 | 45.158 |
| mem7 | 35.537 | 34.996 | 34.914 | 34.378 | 33.849 | 33.825 |
| owned-chain | 184.369 | 1421.640 | 216.187 | 187.171 | 1487.413 | 223.646 |
| owned-fan | 83.460 | 595.214 | 91.030 | 83.407 | 622.281 | 90.956 |
| mx2 (reconstructed) | 95.175 | 712.098 | 110.756 | 93.618 | 735.117 | 109.273 |
| owned-two-chain | 95.207 | 720.224 | 110.805 | 93.622 | 749.011 | 109.281 |

All mem1..7 candidate/pre ratios are below 1.10. Owned-child ratios are below
1.50: native chain/fan/mx2 1.173/1.091/1.164, x86 1.195/1.091/1.167.
The two-field chain control passes too. Limits are against pre-S2; the much
larger 1e6 footprint is reported separately.

Native collection accepted the first three undisturbed three-engine pairs for
each case, reversing engine order every other pair. Fifteen disturbed pair
attempts were discarded by host-health checks; scheduling, not speed, selected
replays. Earlier interrupted full-matrix attempts were also discarded for
concurrent builds; the per-pair schedule uses the unchanged collector. No other
owner's build or job was canceled. The x86 serialized quiet
job is `20261031T173713Z-t077-s2fu-r3-resources`, done 0. All non-timer outputs
agree across revisions on both hosts. The 12 original replay inputs are byte
identical to the supplied files; [source hashes](t077-s2fu-r3/resource-inputs.json).

Median elapsed seconds (diagnostic, not formal acceptance timing):

| Case | Native pre-S2 | Native 1e6 | Native frozen | x86 pre-S2 | x86 1e6 | x86 frozen |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| rep | 0.085 | 2.575 | 0.078 | 0.120 | 2.680 | 0.110 |
| casc1 | 0.089 | 0.740 | 0.077 | 0.110 | 1.050 | 0.100 |
| casc2 | 0.099 | 0.421 | 0.095 | 0.140 | 0.630 | 0.120 |
| ch | 18.392 | 11.901 | 0.077 | 11.440 | 11.440 | 0.090 |
| deepcls (2,000 classes) | 0.600 | 0.515 | 0.047 | 0.770 | 0.470 | 0.040 |
| class4000 | 3.983 | 3.722 | 0.175 | 5.350 | 3.430 | 0.100 |
| class8000 | 29.455 | 28.490 | 0.682 | 39.940 | 27.140 | 0.400 |

The cycle walk is O(chain length) with constant visited-state storage. Building
a full class chain still performs repeated walks and is O(N²); these measurements
do not establish linear whole-program complexity. The 100,000-function helper
also passes natively and in NG ([receipt](t077-s2fu-r3/function-deep.json)); the
x86 resource receipt records the same check. No stack-overflow waiver is used.

## Memory source caveat

The lead did not supply mx2 source. The retained reconstruction is a
500,000-node one-field chain matching the reported round-1 head footprint
(~639 MB here versus ~636 MB supplied); [program](t077-s2fu-r2/mx2.js).
An additional [two-field control](t077-s2fu-r2/owned-two-chain.js) is measured.
The exact lead-source match is not claimed. The million-node owned chain and
250,000-parent two-field fan are separately retained alongside it.
