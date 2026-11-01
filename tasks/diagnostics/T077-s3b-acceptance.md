# T077 S3b acceptance (lead receipt, 2026-11-01)

Candidate: branch `agent/t077-s3b-context/codex`, frozen at `8abd4c69` (runtime
`0148b2eb` plus the regenerated x86 order; `32e7c5a9` adds records only). Base: main
`3cc12c6f` (S2 follow-up accepted; `ed022fa1` changed only tooling). Timing ran
through the workspace `imac-bench.sh` queue on the iMac (x86_64, workers-4). Owner
qualification: [final owner receipt](T077-s3b-final3-owner/README.md).

| Gate | Job | Result vs main `3cc12c6f` |
| --- | --- | --- |
| Formal, 30 blocks | `20261101T150314Z-t077-s3b-formal2` | development **1.0315**, external **1.0276**, selection **1.0223**, sentinel 0.9988; all cases healthy |
| Held-out, six layouts × 30 pairs | `20261101T150316Z-t077-s3b-heldout2` | wall geomean **0.9698**, instructions 0.9712 |
| RSS, 15 alternating pairs | `20261101T150316Z-t077-s3b-rss15b` | development geomean **1.0286**, worst diff 1.0598 (ceiling 1.10 / 1.05) |
| Public86 triage | `20261101T140707Z-t077-s3b-final3` | wall/instructions: broad 1.017/1.033, development 1.034/1.019, external 1.026/1.030, sentinel 0.991/1.032 |
| Correctness | owner receipt | full gate; seven stress modes; compare-qjs 319 × 9 modes and module fixtures; public sweep 351/351; differential 8,000 programs; exact Test262 42,672/42,672 cached and IC-off |
| Independent reviews | three (code audit and differential fuzz on `55f5d1d6`; round-4 re-review on `689efca2`) | every blocker found (receiver loss, deleted calls, NFE free names, eval-var parameters, global-store semantics) repaired with regression tests |

**Per-case results.**
- Development: graphlib 0.951, js-yaml 0.969, big.js 1.000, moment 1.009, diff 1.011,
  papaparse 1.030, matter-js 1.040, meriyah 1.054, immutable 1.074, lodash 1.196.
  Rust/NG development geomean 2.320 → 2.392.
- Held-out: handlebars 0.861, d3-array 0.899, marked 0.903, decimal.js 0.956,
  crypto-js 0.965, mustache 0.998, acorn 1.037, esprima 1.062, ramda 1.072.

**Correctness fixed for main.** S3b repairs defects that main had:
- function declarations in `try`/`finally` blocks were never instantiated;
- inner block shadows broke outer captures;
- discarded call statements were deleted from numeric plans;
- indirect-eval function writes were lost;
- default-parameter closures saw the wrong binding;
- inherited-global lookup was wrong;
- `with` receivers in tagged and optional calls were lost;
- labeled-continue TDZ was not enforced;
- module import homonyms resolved wrongly;
- `Object.freeze(arguments)` kept the mapping;
- global-lexical redeclaration was not rejected;
- Annex B publication timing was wrong;
- strict cross-script global-lexical writes failed.

**What S3b removes and enforces.** It deletes CallEnv, per-call snapshots and
apply-back, and the caller-environment equality proof. 19.1M environment events go
to zero. The receiver has one authority (the activation's call owner). Every
global store follows GlobalEnv.SetMutableBinding. Two permanent tools come with it:
the public correctness sweep and the seeded binding differential (`tools/binding_fuzz`).

Cumulative versus the fixed rebuild baseline `6663738f` (products of accepted stage
ratios): development 0.909, external 1.017, selection 0.988, held-out 0.901.

**Lead decision (user-approved 2026-11-01).** S3b is accepted as an **enabling
stage**. It fails the overall-first rule on three aggregates by about 2–3% while
improving held-out by 3%. The user chose to land it because of the correctness fixes
and the target architecture. The residual cost is structural to activation ownership,
which the approved plan assigns to S3c.

**Binding obligations.**
- S3c becomes a payoff stage and must recover the S3b regression. It must remove:
  - per-call activation binding, ~130–150 instructions per call;
  - checked captured-cell writes, ~125 instructions per write;
  - global-code reference hooks and eval-local cell reads.
- S3d owns restoring debug-build call/apply/Reflect.apply depth, which is below base
  since `f609c0ae`; release depth is above base.
- The earlier S3e/S4 obligations stand. External is now above the rebuild baseline
  (1.017), and recovering it is the first measured target of S3c.
