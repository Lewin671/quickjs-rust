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

Broad v2 contains 25 critical cases across eight families: call (6), binding
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

- [x] B1 freeze broad v2 workload, exact case/family inventory, manifest,
  protocol hashes, hosted preview contract, and documentation.
- [x] B2 record the first complete broad v2 three-role local baseline and
  identify the largest family/case gaps without excluding weak cases.
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
  --blocks 3 --output target/benchmarks/broad-v2-baseline.jsonl
./scripts/benchmark-report.sh \
  --input target/benchmarks/broad-v2-baseline.jsonl \
  --output target/benchmarks/broad-v2-baseline-report.json
./scripts/check.sh
```

## Broad V1 Audit And V2 Reset

Broad v1 is retained as historical evidence but is no longer the campaign
score. Its write holdouts only observed the final values of repeatedly
overwritten slots. A general safe-integer loop-summary prototype reduced
`property_write` from about 1,110 ns/op to about 0.06 ns/op by computing those
final values directly. That result was semantically correct but timer-limited,
non-linear, and not evidence of sustained property-write throughput, so the
prototype was discarded rather than committed or counted as progress.

Broad v2 makes both `property_write` and `array_write` state-recurrent and adds
every round's resulting state to a triangular checksum. Focused three-role
diagnostics at the v2 contract show exact operation/checksum agreement,
eligible measurement windows, and N/2N linearity for both cases. The current
measurement identity is `quickjs-measurement-protocol-v6`, protocol SHA-256
`9cbd57169707fe1c2b691c340a04b60a4f127c6140b8df7823407299fb60c2b3`,
and checked-in manifest SHA-256
`4456d56d48c68417afcb980c71486ac77465834aee3da032763256835a8775b8`.

## Initial Broad V2 Baseline

The first complete v2 baseline was recorded at commit
`4297443e2ceda55eba7fc605dfb9881b993a1c7e`, seed `20250713`, against pinned
QuickJS-NG `f7830186043e4488f2998759d60a514faf07cbc9`. Candidate and base were the
same generic-CPU release binary. Their A/A ratio was **1.00291x**, with a 95%
confidence interval of [1.00145x, 1.00492x]; the three-block run is explicitly
`inconclusive`/`non_claim`, so this small same-binary offset is retained rather
than treated as an engine change. All 225 measurements were eligible, all 75
N/2N checks passed, and all three blocks were valid.

Candidate/QuickJS-NG is **2.31261x** overall, with a 95% confidence interval of
[2.30569x, 2.31630x]. Reaching 0.50x requires a further 4.63x geometric-mean
improvement (78.4% lower wall ns/op) without allowing any critical family to
exceed 1.00x.

| Critical family | Candidate / QuickJS-NG |
| --- | ---: |
| builtin | 39.4847x |
| string | 10.1758x |
| control | 8.1150x |
| allocation | 7.5084x |
| property | 5.6092x |
| array | 0.9960x |
| call | 0.7741x |
| binding | 0.5978x |

The largest current case cliffs are recurrence-protected `property_write`
61.7146x, `array_index_of` 40.0751x, `math_abs` 38.9030x,
`property_dynamic_read` 32.1911x, and `top_level_function_call` 30.4638x.
`property_write` measured 1,322.99 ns/op for the candidate and 21.44 ns/op for
QuickJS-NG, with passing linearity for all roles. This replaces the rejected
constant-time result with a credible sustained-work measurement.

Evidence bindings:

- run ID: `b59e142d-be12-4ac3-beb8-839c0996293d`;
- measurement protocol SHA-256:
  `9cbd57169707fe1c2b691c340a04b60a4f127c6140b8df7823407299fb60c2b3`;
- dynamic manifest SHA-256:
  `d886d709cec2d2ccb11bb9c6617aadd6a44fd42c244ec2448f06b22bf79f3c55`;
- raw JSONL SHA-256:
  `51b5b025a593cf860236e4c5c3a672480b95e32eaa56a9d3dfee818d15ff4184`;
- report JSON SHA-256:
  `a0290794d1cfd9ce906ac12668b289cfcc64ee67d20c971dbb7c05b167d98c50`;
- candidate/base binary SHA-256:
  `95f310fe996740d4d0031b6c3d0742e23114446ef806e90c371e3898911692bb`;
- QuickJS-NG binary SHA-256:
  `cfd8386c3c29b1125a878b8fb82f9627820f2dcc16d2a691c5f8c16ad0b047a0`.

## Broad V2 Optimization Evidence

The first v2 runtime unit admits guarded global-object method calls to the
existing numeric counted-loop plan and specializes the exact native
`Math.abs` identity for primitive numeric arguments. It still evaluates one
absolute-value operation per JavaScript iteration; it does not replace the
loop with a closed-form checksum. Accessor properties, replaced functions,
and object-to-number conversions retain the observable generic path.

An exact-case three-role diagnostic reduced `math_abs` from 1,362.85 ns/op on
the frozen base to 2.823 ns/op on the candidate. Pinned QuickJS-NG measured
34.994 ns/op, making the diagnostic ratios 0.00207x candidate/base and
0.08067x candidate/QuickJS-NG. Both fast engines reached the existing
20,000,000-iteration ceiling before the required 500 ms window, so these
numbers are explicitly timer-limited diagnostics rather than campaign claims.
Independent 100,000,000/200,000,000 candidate runs completed in 0.26/0.50 s
with exact checksums, confirming linear work and identifying a required
measurement-capacity follow-up before the next formal broad run.

The follow-up raises only the `math_abs` iteration ceiling from 20,000,000 to
400,000,000; the workload, checksum, operation count, warmup, minimum window,
and all other cases remain unchanged. A three-block exact-case confirmation
then made every role eligible: the candidate measured
2.5007/2.5007/2.5018 ns/op, base measured
1,346.45/1,343.48/1,363.80 ns/op, and QuickJS-NG measured
34.3324/34.3188/34.3339 ns/op. The paired geometric ratios are 0.00185x
candidate/base and 0.07286x candidate/QuickJS-NG. All N/2N diagnostics passed;
the run remains a focused non-claim because it intentionally covers one of 25
cases.

Diagnostic bindings:

- run ID: `e704b280-6903-4c56-87a6-813d5d05c412`;
- raw JSONL SHA-256:
  `aa1249abe6b053ac9d462b9d211649d5566601d4e25e3cbaf1d2adcc20cb90b9`;
- candidate binary SHA-256:
  `d324de478dd801019c1595593f96ad6f70e5151c2ac64717e770a60e0ef787a7`;
- base binary SHA-256:
  `95f310fe996740d4d0031b6c3d0742e23114446ef806e90c371e3898911692bb`;
- QuickJS-NG binary SHA-256:
  `cfd8386c3c29b1125a878b8fb82f9627820f2dcc16d2a691c5f8c16ad0b047a0`.

Capacity-confirmation bindings:

- run ID: `c67a4690-e8f1-48fe-9c62-fc85abc72988`;
- raw JSONL SHA-256:
  `4d3a27cecee05f8c31a01fb9ae60b6369ebac145f4c16594a384b36e68273c0e`;
- manifest SHA-256:
  `0eb67f3d13602f18bb280ae73094374a8d628971e5358ee9baa984a2d74d5d54`.

The second v2 runtime unit adds a conservative dense implementation fast path
for ordinary `Array.prototype.indexOf` calls, then admits the same operation to
the counted-loop executor only when the receiver has dense storage, the
default prototype, no intercepting descriptors, the prototype property still
resolves to the exact native function, and all admitted arguments are numeric.
The executor performs a dense search on every source iteration; it does not
precompute the search result or the final checksum. Own methods, accessors,
prototype replacements, sparse arrays, and coercing `fromIndex` values fall
back to the observable generic path.

The ordinary-call fast path first reduced `array_index_of` from about
1,989 ns/op to 496.90 ns/op. The guarded loop path then measured 5.784 ns/op,
versus 1,974.24 ns/op for the frozen base and 49.697 ns/op for QuickJS-NG:
0.00293x candidate/base and 0.11639x candidate/QuickJS-NG. The candidate again
reached the existing 20,000,000-iteration ceiling, so this exact-case result is
a timer-limited diagnostic. Independent 100,000,000/200,000,000 runs consumed
0.55/1.06 CPU seconds with exact checksums, confirming linear per-iteration
work and requiring the same measurement-capacity follow-up before a formal
broad run.

The follow-up raises only the `array_index_of` iteration ceiling from
20,000,000 to 400,000,000. This gives the optimized candidate and QuickJS-NG
enough capacity to reach the 500 ms window and run the required N/2N
diagnostic; the workload, checksum, operation count, warmup, timeout, and every
other case remain unchanged. A three-block exact-case confirmation made all
nine measurements eligible: candidate medians were 5.3114/5.3163/5.3141
ns/op, the frozen base measured 1,957.95/1,964.78/1,951.20 ns/op, and
QuickJS-NG measured 48.9590/48.9877/49.1140 ns/op. The paired geometric ratios
were 0.002714x candidate/base and 0.108403x candidate/QuickJS-NG. All three
N/2N ratios stayed within the frozen 0.85..1.15 bounds.

Array-search diagnostic bindings:

- run ID: `5303a382-a058-4c2b-8190-f1052b196349`;
- raw JSONL SHA-256:
  `b13c930f28059a0b55645a17180a3942b3424cacb02c5f6cd021c7dfa33de869`;
- candidate binary SHA-256:
  `e90cedb609107af91fce44161309dd223e8379431584135d5fafd163c86f1faa`.

Array-search capacity-confirmation bindings:

- run ID: `51ee7614-e246-4035-a966-b9a8deea5ede`;
- raw JSONL SHA-256:
  `3eefd91c9f2bae62606c3de837f502dfb330e22eb69cf20e61075c4e628af397`;
- dynamic manifest SHA-256:
  `5c3413f899e6de879206be146f86bd548934ec051527228e4797ccd6ba08b4e3`.

## Historical Broad V1 Baseline

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

## Historical Broad V1 Optimization Evidence

The first runtime unit, commit
`432f1c0afc2fffc95c726c7668f3dce74fb0b8f6`, generalized the counted-loop
numeric leaf-call path to two simple arguments. Against the preceding
`21af319082471fad1f2dc6a0501df25efd5d7b27` baseline it reduced the broad
candidate/base ratio to **0.89985x** with a 95% confidence interval of
[0.89307x, 0.90105x]. Candidate/QuickJS-NG fell to **2.57083x** with a 95%
confidence interval of [2.57054x, 2.57542x], and the call family fell from
1.37980x to 0.81756x. The clean three-role run had 225/225 valid measurements,
75/75 passing linearity probes, and three valid blocks. Run ID:
`4a464d4e-b117-4a1f-86b7-c546b023dc1d`; raw SHA-256:
`ef04e03bc9a606d470ee863da51678802894d3f517578e1de41331fc16a4ae7b`;
report SHA-256:
`9375afe27f69c3fcd38207526f6cf982a274146d6fa11f4b2bb636651633940a`.

The follow-up unit, commit
`64ba9361eede0b06ce5cccd0b9fb790de0738f7f`, replaced per-iteration argument
vectors with a fixed zero/one/two-argument plan and scalar dispatch. Against
`432f1c0afc2fffc95c726c7668f3dce74fb0b8f6` it reduced the broad ratio to
**0.98905x** with a 95% confidence interval of [0.98375x, 0.98998x]. The
improvement was concentrated in `captured_read` (0.82696x) and
`function_call_two_args` (0.89574x); the confidence intervals for the other
previously fast call shapes include 1.00x, so this run does not establish a
regression in those shapes. Candidate/QuickJS-NG is now **2.54217x** overall
with a 95% confidence interval of [2.53535x, 2.55487x]. Current family ratios
are:

| Critical family | Candidate / QuickJS-NG |
| --- | ---: |
| builtin | 40.0700x |
| string | 10.1817x |
| property | 10.1676x |
| control | 8.1016x |
| allocation | 7.5606x |
| array | 1.0781x |
| call | 0.8064x |
| binding | 0.6040x |

The clean three-role run again had 225/225 valid measurements, 75/75 passing
linearity probes, and three valid blocks. Run ID:
`23e73eba-8b55-4279-8ca9-006ff9d19564`; candidate binary SHA-256:
`6b5e6db88cf03ba1eb01e0f39ce99f39f3c774e5e4737faec1f8c05f7a7ffcca`;
raw SHA-256:
`519c4ddda56f674265e2fd57954cb7e1464b172b86481afd01e04a52f5e14847`;
report SHA-256:
`7b1fc99b1ac212c300d0a321e57ca76b411cec170a7b8a7b3b611d003afee811`.
Both runs used seed `20250713`, the frozen measurement protocol
`b2f2e85343c1fbe2bb4bc58d3540a1a666cab85b71118e45e019400541ee75c6`,
and pinned QuickJS-NG `f7830186043e4488f2998759d60a514faf07cbc9`.

Commit `6e4c4d3fc8923f920face6c6989fa0f58b42f435` added a direct named-property
store and an existing-own-data-property update path. Against
`64ba9361eede0b06ce5cccd0b9fb790de0738f7f`, the complete broad ratio was
**0.93998x** with a 95% confidence interval of [0.93608x, 0.94336x]. The
targeted `property_write` case fell to 0.30284x candidate/base and 112.85674x
candidate/QuickJS-NG; the property family fell from 10.16763x to 6.84045x.
Candidate/QuickJS-NG reached **2.38021x** overall with a 95% confidence interval
of [2.38021x, 2.38670x]. All 225 formal measurements were valid, all 75
linearity probes passed, and all three blocks were valid. Run ID:
`aa330f72-d5df-4a11-877a-a15dbf75223f`; candidate binary SHA-256:
`95f310fe996740d4d0031b6c3d0742e23114446ef806e90c371e3898911692bb`;
raw SHA-256:
`3df3e6b752501855a9422d9a947f5fc8902fc85f2f1215d9e539adbac364243f`;
report SHA-256:
`3b6d44a8ab13bae223d8e0b59981b80b6ae5e45b1e10b35fba2d0d11f6285ed7`.

## Notes

Broad v2 is still a first-party micro portfolio, not a substitute for an
admitted external macro suite. T017's external-corpus audit remains the path to
broader public claims. The immediate purpose is to make optimization robust to
code-shape changes and multiple runtime subsystems before chasing the 2x goal.
