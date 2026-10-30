# T077 S3a acceptance (lead receipt, 2026-10-30)

Candidate: branch `agent/t077-s3a-native-entries/codex` tip `3872d617` (runtime
`03c8680c`), after independent review (different model) and its repair round.
Base: `d19d3aa6` (runtime identical to the rebuild baseline `6663738f`). All jobs ran
through the workspace `imac-bench.sh` queue on the iMac (x86_64, workers-4).

| Gate | Job | Result |
| --- | --- | --- |
| Formal, 30 blocks | `20261030T102325Z-t077-s3a-formal-r2` | development **0.9620**, external 0.9937, selection 0.9951, sentinel 1.0031 (worst 1.038); all cases healthy |
| Held-out, six layouts × 30 pairs | `20261030T110158Z-t077-s3a-heldout` | wall geomean **0.9657** (worst d3-array 1.0077), instructions 0.9679 |
| RSS, 15 alternating pairs | `20261030T110159Z-t077-s3a-rss15` | development geomean 0.9932, worst papaparse 1.0167 (ceiling 1.10 / 1.05) |
| Correctness | owner receipt [S3a results](T077-s3a-results.md) | full check, 257 NG fixtures, 42,672/42,672 configured exact, burndown recorded |

Development per case (candidate/base): moment 0.875, immutable 0.919, js-yaml
0.950, diff 0.958, lodash 0.968, papaparse 0.980, graphlib 0.984, meriyah 0.988,
big.js 0.993, matter-js 1.011. Rust/NG development geomean 2.632 → 2.531.
Held-out per case: ramda 0.919, handlebars 0.924, crypto-js 0.957, esprima 0.963,
mustache 0.969, decimal.js 0.980, acorn 0.980, marked 0.996, d3-array 1.008.

All four overall aggregates improve, so S3a is accepted as a payoff stage under the
T077 contract. Integration re-ran `./scripts/compare-qjs.sh` (257/257) on the merged
tree; the pre-push full gate runs on push. Review history: the first candidate
`c8d15de4` added a native depth guard that rejected ordinary programs (e.g. forEach
recursion depth 88 vs base 500); it was removed and depth parity re-established before
these measurements.
