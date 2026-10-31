# T077 S1+S2 acceptance (lead receipt, 2026-10-31)

Candidate: branch `agent/t077-s2-named-stores/codex` at `fc3deef7` (runtime `dc710e4b`,
regenerated x86 order `4098ee67`; later commits are records only). It contains S1
(`agent/t077-s1-property-authority/codex`, including its review repairs) and S2
rounds 1–4. Base: main `dcee3c56` (runtime `71294a20`, S3a accepted). All timing ran
through the workspace `imac-bench.sh` queue on the iMac (x86_64, workers-4).

| Gate | Job | Result vs main `dcee3c56` |
| --- | --- | --- |
| Formal, 30 blocks | `20261031T031953Z-t077-s1s2-formal` | development **0.9282**, external **1.0025** (≈95% CI 1.0016–1.0035), selection 0.9826, sentinel 0.9199; all cases healthy |
| Held-out, six layouts × 30 pairs | `20261031T031953Z-t077-s1s2-heldout` | wall geomean **0.9620**, instructions 0.9655 |
| RSS, 15 alternating pairs | `20261031T031954Z-t077-s1s2-rss15` | development geomean **0.8928**, worst big.js 1.0373 (ceiling 1.10 / 1.05) |
| Correctness | owner receipt [freeze qualification](T077-s2-freeze-qualification.md) | full check, 268 NG fixtures (default/IC-off/verify), 42,672/42,672 exact in cached and IC-off modes, 350 differential fuzz seeds |
| Independent reviews | different model, three rounds | no correctness blocker; should-fix items routed to the S2 follow-up and S3b |

Development per case: matter-js 0.816, meriyah 0.848, diff 0.892, js-yaml 0.919,
immutable 0.922, graphlib 0.925, moment 0.968, lodash 0.973, big.js 1.005,
papaparse 1.037. Rust/NG development geomean 2.531 → **2.351** (rebuild baseline 2.632).
Held-out per case: esprima 0.819, acorn 0.820, handlebars 0.939, crypto-js 0.966,
decimal.js 0.993, ramda 1.003, mustache 1.015, marked 1.056, d3-array 1.085.
External worst: bitops-3bit-bits-in-byte 1.110, string-unpack-code 1.106,
access-nbody 1.091, string-validate-input 1.088, 3d-raytrace 1.067; RSS meriyah 0.527.

Cumulative versus the fixed rebuild baseline `6663738f` (products of accepted stage
ratios): development 0.893, external 0.996, selection 0.978, held-out 0.929.

**Lead decision.** S1 was the enabling stage and S2 its payer. Development, selection
and held-out improve clearly; external is +0.25% against the preceding milestone and
improved against the fixed rebuild baseline. The residual external losses sit in the
typed-loop and wide-tier executors (static sizes 3,649/3,651 vs base 3,024/3,006)
that S3e/S4 replace; another tuning round there would be spent on code scheduled for
deletion. The lead accepts S1+S2 under the user's overall-first rule, records external
+0.25% (and the held-out d3-array/marked/mustache and development papaparse/big.js
cases) as local regressions, and makes recovering them an explicit S3e/S4 obligation.
The user may overturn this decision.
