# T077 S3c acceptance (lead receipt, 2026-11-02)

**Candidate.** Branch `agent/t077-s3c-v2/codex` at `6640f7da`. Runtime frozen at
`400a14de`, with the sealed native orders; later commits are records only. It
implements the ABI of [Lead decision 2](../proposals/T077-s3c-brief.md):
- ordinary activations on one contiguous register stack with 64-byte headers;
- boxed records only for statically suspendable functions, with resume reattaching
  the same box;
- lazy Context;
- static captured-write routes;
- explicit binding-hook checkpoints.

The first ABI attempt is retained unmerged on `agent/t077-s3c-activations/codex`. It
used a boxed record per call and was rejected at its C2 checkpoint: about 940 more
instructions per call; [attribution](T077-s3c-c2/attribution/README.md).

**Base.** S3b main `8322fc98`, the frozen S3c base. Main `e60e7cfa` differs only by
the class-chain fix. Timing ran through the workspace `imac-bench.sh` queue on the
iMac (x86_64, workers-4).

| Gate | Job | Result vs `8322fc98` |
| --- | --- | --- |
| Formal, 30 blocks | `20261102T120606Z-t077-s3c-final-formal` | development **0.9570**, external **0.9938**, selection **0.9503**, sentinel 0.9433; all cases healthy |
| Held-out, six layouts × 30 pairs | `20261102T120607Z-t077-s3c-final-heldout` | wall geomean **0.9660**, instructions 0.9836 |
| RSS, 15 alternating pairs | `20261102T120607Z-t077-s3c-final-rss15` | development geomean **1.0019**, worst big.js 1.0117 (ceiling 1.10 / 1.05) |
| Public86 triage | `20261102T120605Z-t077-s3c-final-public86` | wall/instructions: broad 0.953/0.953, development 0.952/0.985, external 0.995/0.997, sentinel 0.947/0.952 |
| Correctness | [owner receipts](T077-s3c5-final/README.md), [depth repair](T077-s3c-r4/README.md) | full gate; stress and poison modes; NG 322 scripts and 10 modules × 9 modes; public sweep 351/351; binding_fuzz ≥2,000 per profile; exact Test262 42,672/42,672 cached and IC-off (production build); 14-probe depth matrix ≥ base in debug and release |
| Independent review | on `c80ac690` | no semantic regression across 8,000 binding_fuzz seeds, 4,600 targeted programs (7 modes), NG and workload checksums; one blocker (general-VM native depth), repaired in `400a14de` and re-verified |

**Per case.**
- Development: lodash 0.845, graphlib 0.922, diff 0.944, papaparse 0.961,
  immutable 0.965, moment 0.972, meriyah 0.986, js-yaml 0.990, big.js 0.998,
  matter-js 0.998. Rust/NG development geomean 2.392 → **2.289**.
- Held-out: d3-array 0.909, ramda 0.922, esprima 0.937, decimal.js 0.966,
  handlebars 0.982, crypto-js 0.986, mustache 0.988, marked 0.997, acorn 1.013.

**Cumulative** versus the fixed rebuild baseline `6663738f` (products of accepted stage
ratios): development 0.870, external 1.011, selection 0.939, held-out 0.870.

**Lead decision.** S3c is accepted as a **payoff stage**: all four aggregates improve
and RSS passes. It recovers the S3b enabling-stage regression on development,
selection and held-out. External improves only 0.6%, which misses the ≤0.983 target,
so the remaining external deficit against the rebuild baseline (~1.1%) becomes an
**S3d obligation**, as the brief specified.

**Carried to S3d:**
- the external deficit;
- native stack overflow should raise RangeError instead of aborting;
- the single runtime stack guard;
- S3b's debug-depth deficit against `3cc12c6f`.

**Pre-existing bugs, with repros recorded:**
- nested try/finally `continue` skips the outer finally;
- a sloppy function with duplicate parameter names returns the first argument.
