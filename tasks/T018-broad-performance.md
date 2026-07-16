# T018: Broad Performance Campaign

## Goal

Beat the pinned QuickJS-NG reference by at least 2x on the repository's broad
black-box throughput portfolio: candidate/QuickJS-NG overall geometric-mean
wall ns/op must be at most 0.50x, while every critical family remains at or
below 1.00x.

## Scope

- Allowed paths: first establish `benchmarks/`, `tools/benchmark/`, benchmark
  scripts/tests/docs; later units may change `qjs-runtime` with focused tests.
- Forbidden paths: `third_party/`, benchmark-only engine shortcuts, weakened
  checksums, reduced iteration work, hidden case selection, or Test262
  regressions.
- Owner boundary: serialize manifest/protocol changes and broad runtime
  architecture changes on the main branch; one measured change per commit.

## References

- `AGENTS.md`
- `docs/architecture.md`
- `docs/benchmarking.md`
- `docs/harness.md`
- `tasks/T016-environment-model-rewrite.md`

## Portfolio Contract

Broad v1 contains 25 critical cases across eight families: call (6), binding
(5), property (3), array (3), control (2), builtin (2), string (1), and
allocation (3). The seven historical T016 cases remain as a trace cohort; 18
shape and subsystem holdouts prevent the historical exact-loop trace fast path
from standing in for general engine performance.

The authoritative ratio is candidate wall ns/op divided by pinned QuickJS-NG
wall ns/op. Acceptance requires all of the following on a complete, healthy,
same-host run:

- overall geometric-mean ratio <= 0.50;
- every critical family ratio <= 1.00;
- no invalid block, failed linearity probe, checksum mismatch, or timer-limited
  case;
- focused tests plus `scripts/check.sh` pass, preserving Test262 behavior;
- a second independent run confirms the final result before completion.

The target is a campaign acceptance criterion. Existing hosted previews remain
informational and non-gating until T017 M6/M7 fixed-hardware qualification is
complete.

## Milestones

- [x] B1 freeze broad v1 workload, exact case/family inventory, manifest,
  protocol hashes, hosted preview contract, and documentation.
- [x] B2 record the first complete three-role local baseline and identify the
  largest family/case gaps without excluding weak cases.
- [ ] B3 optimize structural bottlenecks in separately verified commits,
  recording broad candidate/base and candidate/QuickJS-NG evidence each time.
- [ ] B4 reach <= 0.50x overall and <= 1.00x for every critical family.
- [ ] B5 independently confirm the result and run the full correctness gate.

## Verification

```sh
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s tools/benchmark/tests -v
./scripts/performance-policy-audit.sh
./scripts/benchmark.sh --dry-run --blocks 3
./scripts/benchmark.sh --candidate target/release/qjs \
  --base target/release/qjs \
  --quickjs-ng third_party/quickjs-ng/build/qjs \
  --blocks 3 --output target/benchmarks/broad-v1-baseline.jsonl
./scripts/benchmark-report.sh \
  --input target/benchmarks/broad-v1-baseline.jsonl \
  --output target/benchmarks/broad-v1-baseline-report.json
./scripts/check.sh
```

## Initial Broad V1 Baseline

The first complete baseline was recorded on 2026-07-15 at commit
`21af319082471fad1f2dc6a0501df25efd5d7b27`, seed `20260721`, against pinned
QuickJS-NG `f7830186043e4488f2998759d60a514faf07cbc9`. Candidate and base used the
same generic-CPU release binary, producing a 0.99998x A/A ratio with a 95%
confidence interval of [0.99669x, 1.00265x]. All 225 formal measurements were
valid, all 75 N/2N linearity checks passed, and all three blocks were valid.
The expected three-block health is `inconclusive`/`non_claim`; this is a
campaign baseline, not a fixed-hardware public claim.

Candidate/QuickJS-NG was **2.86410x** overall with a 95% confidence interval of
[2.86022x, 2.86677x]. Reaching 0.50x therefore requires a 5.73x improvement
from this baseline (82.5% lower geometric-mean wall ns/op), without pushing any
critical family above 1.00x.

| Critical family | Candidate / QuickJS-NG |
| --- | ---: |
| builtin | 38.8248x |
| string | 10.2434x |
| property | 10.1712x |
| control | 8.1002x |
| allocation | 7.5618x |
| call | 1.3798x |
| array | 1.0857x |
| binding | 0.5794x |

The largest case cliffs are `property_write` 371.41x, `math_abs` 39.22x,
`array_index_of` 38.44x, `property_dynamic_read` 31.94x, and
`top_level_function_call` 30.39x. In contrast, the historical trace cohort
ranges from `many_locals_call` 0.0319x through `captured_read` 0.1203x. This
split proves that the previous seven-case result was real for those exact
shapes but was not representative of broad engine performance.

Evidence bindings:

- run ID: `ef12a64a-a88a-4b02-b42c-82067717c04e`;
- measurement protocol SHA-256:
  `b2f2e85343c1fbe2bb4bc58d3540a1a666cab85b71118e45e019400541ee75c6`;
- dynamic manifest SHA-256:
  `184d49b5a421625d3b45be40a3bca8df9714ae4a43a1430645d559ba5a99d53e`;
- raw JSONL SHA-256:
  `ed13fdde0ae588ebc00e88a118c7eef5a904168eedbb2f92e17f3f4b2d306a14`;
- report JSON SHA-256:
  `5da0d1e49b38d1983f2e1a0092e14ca8857000e8c52cd460ded83afa0ac0bca3`;
- candidate/base binary SHA-256:
  `b718cf294af3dacab1929f791d314aa5a15b1e509ed62c61f10513d800b2fa81`;
- QuickJS-NG binary SHA-256:
  `cfd8386c3c29b1125a878b8fb82f9627820f2dcc16d2a691c5f8c16ad0b047a0`.

## Notes

Broad v1 is still a first-party micro portfolio, not a substitute for an
admitted external macro suite. T017's external-corpus audit remains the path to
broader public claims. The immediate purpose is to make optimization robust to
code-shape changes and multiple runtime subsystems before chasing the 2x goal.
