# T077 S2 follow-up acceptance (lead receipt, 2026-10-31)

Candidate: branch `agent/t077-s2-followup/codex`, frozen runtime `40814de6` (its x86
order included; `960e8923`/`834a3ae1` are records only). Base: main `1e6e2a2b`
(S1+S2 accepted). Timing ran through the workspace `imac-bench.sh` queue on the iMac
(x86_64, workers-4). Owner qualification: [round 2 record](T077-s2fu-round2.md).

| Gate | Job | Result vs main `1e6e2a2b` |
| --- | --- | --- |
| Formal, 30 blocks | `20261031T175844Z-t077-s2fu-formal` | development **0.9871**, external **0.9936**, selection **0.9879**, sentinel 0.9930; all cases healthy |
| Held-out, six layouts × 30 pairs | `20261031T210500Z-t077-s2fu-heldout-r3` | wall geomean **0.9995**, instructions 1.0035 |
| RSS, 15 alternating pairs | `20261031T181931Z-t077-s2fu-r3-rss15` | development geomean **0.9850**, worst diff 1.017 (ceiling 1.10 / 1.05) |
| Correctness | owner receipt | full check, 268 NG fixtures (default/IC-off/verify), 42,672/42,672 exact cached and IC-off, 18 native and 18 x86 resource cases |
| Independent review | different model, on `40814de6` | no correctness blocker; ~3,400 differential fuzz programs and 47 targeted scripts agree across default/IC-off/IC-verify/NG |

Two earlier held-out jobs are void: `20261031T175845Z-t077-s2fu-heldout` was
DISTURBED (iMac maintenance at local 03:11) and `20261031T195531Z-t077-s2fu-heldout-r2`
failed its host check when the iMac hostname changed mid-job. Neither result is used.

Development per case: moment 0.963, papaparse 0.968, immutable 0.981, js-yaml 0.987,
diff 0.990, meriyah 0.993, lodash 0.993, big.js 0.996, graphlib 1.000, matter-js 1.002.
Rust/NG development geomean 2.351 → **2.320**. Held-out per case: marked 0.976,
decimal.js 0.990, acorn 0.992, ramda 0.993, mustache 1.000, crypto-js 1.007,
esprima 1.010, handlebars 1.013, d3-array 1.014. External worst: json-parse-financial
1.127 (instructions ≈1.005, a layout signature; 0.23× NG), bitops-bitwise-and 1.019.

x86 peak RSS (MB, pre-S2 `d38f30a7` / `1e6e2a2b` / candidate): mem1 118/1386/117,
mem4 166/411/163, mem6 46/207/45, owned-chain 187/1487/224, owned-fan 83/622/91,
mx2 94/735/109. `class extends` 8000-deep build 0.67 s (base 0.49 s at 2000);
`Object.setPrototypeOf(fn, 1e5-deep chain)` no longer overflows the native stack.

Cumulative versus the fixed rebuild baseline `6663738f` (products of accepted stage
ratios): development 0.881, external 0.990, selection 0.966, held-out 0.929.

**Lead decision.** Accepted: all four aggregates are at or below 1.00, and S2's
prototype-mode memory and complexity pathologies are removed. The residual held-out
instruction cost (+0.35%) and json-parse-financial join the S3e/S4 recovery list.
Queued from the review: `class extends` construction is still quadratic (a freshly
created constructor cannot be on a chain, so `vm_class.rs` can skip the cycle check),
a small fix after S3b lands; deep derived-constructor `new C()` aborting on native
stack overflow is pre-existing and S3d-owned.
