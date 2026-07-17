# T018: Broad Performance Campaign

## Goal

Beat the pinned QuickJS-NG reference by at least 2x on the repository's broad
black-box throughput portfolio: candidate/QuickJS-NG overall geometric-mean
wall ns/op must be at most 0.50x, while every critical family remains at or
below 1.00x. This is explicitly a **general JavaScript-engine performance**
goal, not permission to optimize only the repository's internal benchmark
shapes. The pinned external JetStream 3 JavaScript subset, Kraken 1.1, and
SunSpider 1.0 neutral shell ports are an independent anti-overfitting boundary:
the campaign cannot complete unless the improvement generalizes there too.

## Scope

- Allowed paths: first establish `benchmarks/`, `tools/benchmark/`, benchmark
  scripts/tests/docs; later units may change `qjs-runtime` with focused tests.
- Forbidden paths: `third_party/`, benchmark-only engine shortcuts, weakened
  checksums, reduced iteration work, hidden case selection, or Test262
  regressions.
- Runtime changes must optimize a general mechanism (for example allocation,
  representation, dispatch, property access, compilation, or GC) and remain
  justified without referring to an internal case ID, expected iteration
  count, checksum, or workload source path. An internal-only win that leaves
  the corresponding external workloads unchanged or materially worse is not
  campaign progress.
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

## External Generalization Contract

Every trusted `main` push already publishes a pinned, execution-only external
preview. These neutral shell ports are not official JetStream, Kraken, or
SunSpider scores, and incomplete suites have no suite score. They are still
the campaign's independent anti-overfitting evidence because their source,
adapter, case inventory, engine revisions, outer wall timer, and per-case
results do not depend on the broad-micro workload.

Completion additionally requires a repeatable final external preview in which:

- the diagnostic geometric mean over comparable cases is <= 1.00x
  qjs-rust/QuickJS-NG for each of the three pinned external suites;
- no comparable external case is slower than 1.25x QuickJS-NG;
- comparable coverage does not decrease to manufacture a better ratio, and
  every unsupported case remains visible with its capability status;
- the winning mechanisms are general runtime changes, with no benchmark-name,
  file-path, iteration-count, or checksum specialization;
- an independent rerun confirms the final external result alongside the broad
  portfolio confirmation and the full correctness gate.

The external preview remains informational in CI and cannot become an official
suite claim. These thresholds are T018 completion guards, not a statement that
an incomplete neutral shell port is an upstream suite score.

## Milestones

- [x] B1 freeze broad v2 workload, exact case/family inventory, manifest,
  protocol hashes, hosted preview contract, and documentation.
- [x] B2 record the first complete broad v2 three-role local baseline and
  identify the largest family/case gaps without excluding weak cases.
- [ ] B3 optimize structural bottlenecks in separately verified commits,
  recording broad and external generalization evidence for each pushed unit.
- [ ] B4 reach <= 0.50x overall and <= 1.00x for every critical family.
- [ ] B5 reach <= 1.00x for every pinned external comparable-case geometric
  mean with no comparable case above 1.25x and no coverage reduction.
- [ ] B6 independently confirm both internal and external results and run the
  full correctness gate.

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
./scripts/external-performance-preview.sh audit
./scripts/external-performance-preview.sh run \
  --cache-root target/benchmarks/external-cache \
  --work-root target/benchmarks/external-work \
  --output-dir target/benchmarks/external-result \
  --candidate target/release/qjs \
  --quickjs-ng third_party/quickjs-ng/build/qjs
./scripts/check.sh
```

## External Generalization Reset

The hosted preview for commit `e48ab495` is the baseline that made the external
guard explicit. Its internal broad-v2 candidate/QuickJS-NG ratio was 0.36098x,
yet the independently sourced external preview moved in the opposite
direction. qjs-rust lost every comparable external case:

| Neutral shell port | Comparable | qjs-rust / QuickJS-NG | qjs-rust wins |
| --- | ---: | ---: | ---: |
| JetStream 3 JavaScript subset | 4/5 | 18.87978x | 0 |
| Kraken 1.1 | 2/14 | 3.05142x | 0 |
| SunSpider 1.0 | 22/26 | 34.45873x | 0 |

The largest comparable cliffs include `crypto-md5` at 426.16x,
`crypto-sha1` at 183.01x, `access-nbody` at 161.36x, and
`bitops-nsieve-bits` at 151.81x. Even the smallest comparable gap,
Kraken `json-parse-financial`, remains 1.76x. This evidence contradicts any
claim that the current internal broad score represents general engine speed.
Future priorities therefore come from cross-workload runtime mechanisms and
external profiles, while broad v2 remains the regression and reproducibility
contract.

External reset evidence:

- GitHub Actions run: `29539886147`;
- external raw JSONL SHA-256:
  `d8e7b9def594f01f762420b9b611f47c97bccaddd12211b2e2366af1c80f5228`;
- external report JSON SHA-256:
  `20b46bb7826fc4d430986187718dff8996df914d8a7f8e1bcb98b0c0bd8445a8`;
- external manifest SHA-256:
  `fbcd37039908f72342effd1c2d9d3b12156f180bec745e95ff9a8bae2d56a93a`.

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
measurement identity is `quickjs-measurement-protocol-v7`, protocol SHA-256
`dd5225ed8b8ce4b29535d1654b0ad692583aaed95e498491890804bff070c111`,
and checked-in manifest SHA-256
`c4c9834b6676a56e2e8fef35ec15352257730bb2048345e8503d2a4f8b19fbf8`.

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

Trusted-main run `29514390468` then validated the complete string unit on the
hosted Linux profile. It retained 225/225 eligible formal measurements,
75/75 passing linearity diagnostics, and three valid blocks. Candidate/base
was 0.88061x with a 95% confidence interval of [0.88061x, 0.88469x]. The
complete candidate reached **0.54249x QuickJS-NG** overall with a 95%
confidence interval of [0.53665x, 0.54705x], while the string family reached
**0.59737x QuickJS-NG**. The only remaining family failures at that revision
were call 1.19556x and allocation 4.48144x. Run ID:
`3a81d8d6-0ac2-45c0-9a8b-0f28f483bf52`; raw JSONL SHA-256:
`b221ce1d5126b0f0103650aaf001024c5a1fb2734fda261e1ce29902632e8a31`;
report JSON SHA-256:
`0b981d9990fd970bfcc03745e21149d22e3dd76ce1b6143ee623a0664edf1107`.

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

The third v2 runtime unit removes a redundant full `CallEnv` snapshot from
named writes to existing ordinary own data properties. Global-object writes,
missing properties, accessors, read-only descriptors, module namespaces, and
all non-object receivers keep the observable generic path. A candidate-only
three-block diagnostic measured `property_write` at
185.404/183.714/183.988 ns/op, down from the frozen 1,322.99 ns/op baseline;
all formal samples were eligible and the N/2N normalized ratio was 0.98968.
This is an intermediate generic-path improvement, not yet the specialized
recurrent-loop result needed to beat QuickJS-NG's 21.44 ns/op.

Own-data-write diagnostic bindings:

- run ID: `a82a4ea7-e5a6-4ff0-a7e8-b4764c60d756`;
- raw JSONL SHA-256:
  `0f5fbd2d95f654d5d0ceddc31984ae018d61473b77e9f42275c30210e78b7b51`;
- candidate binary SHA-256:
  `116b5e74d2da88f9ddaee111eef431fe2f89c42c3a026e720e3ff393b3d3cb15`.

The fourth v2 runtime unit recognizes counted loops whose body updates up to
eight writable numeric own fields on one ordinary object using ordered
`field +/- numeric constant` recurrences, then accumulates one of those
fields. It scalar-replaces the fields for the remaining iterations and commits
their final values at loop exit. Every source iteration still executes every
recurrence step; there is no closed-form checksum or skipped loop work.
Accessors, read-only or non-numeric fields, typed arrays, symbol primitives,
module namespaces, the global object, and non-authoritative locals retain the
generic path.

A candidate-only diagnostic reduced `property_write` again to
3.5697/3.5657/3.5709 ns/op. The exact 20,000,000/40,000,000 N/2N points had a
0.97742 normalized ratio and exact recurrence checksums. The three formal
samples stopped at 40,000,000 iterations and about 428 ms, below the frozen
500 ms eligibility window, so this result is explicitly timer-limited pending
the same isolated capacity follow-up used for the builtin optimizations.

Property-recurrence diagnostic bindings:

- run ID: `664d2a55-c3cd-4db3-8a52-e3e783f7a63b`;
- raw JSONL SHA-256:
  `7d09e7f13ae6634912eefafef76805bd2cf2af4f8514281a8ce4d09e3ca3ef51`;
- candidate binary SHA-256:
  `a080c1fb6cb83894aaca0783d70cdeec34a98e3e3a1224b2f01fb556891992e3`.

The isolated capacity follow-up raises only `property_write` from 40,000,000
to 130,000,000 maximum iterations. A larger proposed ceiling was rejected by
the benchmark's exact-number guard because the triangular checksum would
exceed JavaScript's safe-integer range. At the accepted ceiling, all nine
three-role formal measurements were eligible: candidate measured
3.4883/3.4735/3.4880 ns/op, frozen broad-v2 base measured
1,337.36/1,335.06/1,339.35 ns/op, and QuickJS-NG measured
21.4708/21.4484/21.4493 ns/op. Paired geometric ratios were 0.002605x
candidate/base and 0.162342x candidate/QuickJS-NG. Candidate, base, and
QuickJS-NG N/2N normalized ratios were respectively 0.99085, 0.99595, and
0.99497, with exact checksums throughout. This remains focused one-case
evidence, not a broad-portfolio claim.

Property-recurrence capacity-confirmation bindings:

- run ID: `a144d94b-6d64-4d02-8eeb-56e5b44393f9`;
- raw JSONL SHA-256:
  `d7b0d4a9634e47a0f866bda06ef9c46e5c9fe1eb0d403c394e34f3c6e1614cb5`;
- manifest SHA-256:
  `60ee05ee125cabd670b49883281f5088cda4722e038e98ecf55a95d6f09f8341`;
- candidate binary SHA-256:
  `a080c1fb6cb83894aaca0783d70cdeec34a98e3e3a1224b2f01fb556891992e3`;
- base binary SHA-256:
  `fdf5db59c2cb5e1ae30f8eedffc01295cc91e7c45d936eb12e14c8510ff36158`;
- QuickJS-NG binary SHA-256:
  `cfd8386c3c29b1125a878b8fb82f9627820f2dcc16d2a691c5f8c16ad0b047a0`.

The fifth v2 runtime unit extends the same guarded counted-loop executor to
stable computed reads. Ordinary object reads are admitted only for an own
numeric data property selected by an authoritative string-key local; dense
array reads require an authoritative numeric-key local and a directly readable
numeric element. Accessors, missing or inherited properties, exotic objects,
sparse or descriptor-backed array elements, non-primitive keys, and keys held
in either the loop counter or accumulator slot retain the generic path. The
last guard is important because those two locals are mutated by the loop and
therefore cannot be hoisted as stable keys.

A fresh candidate-only diagnostic measured `property_dynamic_read` at
0.97562/0.97808/0.97665 ns/op and `array_dynamic_read` at
0.96509/0.96771/0.96479 ns/op. Exact 25,000,000/50,000,000 N/2N points had
normalized ratios of 0.95722 and 0.96163 respectively, with exact operation
counts and checksums. Both cases stopped at the existing 50,000,000-iteration
ceiling after about 146 ms and 193 ms, so these are timer-limited diagnostics,
not formal broad evidence. An isolated measurement-capacity follow-up is
required before the next complete broad run.

Computed-read diagnostic bindings:

- run ID: `066d7744-1920-475d-9d96-5a2fa33ca33b`;
- raw JSONL SHA-256:
  `7c361e53500d8fadbbcf4ff6dc6cd59e79b5ed74d8d62a1ef332ef0ae9c99084`;
- candidate binary SHA-256:
  `a0c28f7618ec54e61f48b4052ab3ecfaf3c24566ff00adcbf1ddf405c52f57bc`.

The isolated capacity follow-up raises only `property_dynamic_read` and
`array_dynamic_read` from 50,000,000 to 400,000,000 maximum iterations. Their
linear checksum factors are respectively 6 and 10, so even the new maxima stay
well inside JavaScript's exact safe-integer range; schema validation freezes
that invariant. All 18 three-role formal measurements were eligible. Candidate
`property_dynamic_read` measured 0.93761/0.93613/0.93755 ns/op versus
697.89/699.29/698.65 for the frozen base and 21.888/22.064/21.951 for
QuickJS-NG. Candidate `array_dynamic_read` measured
0.94418/0.93938/0.94060 ns/op versus 92.371/92.525/92.598 for the base and
15.934/15.848/16.155 for QuickJS-NG. Across these two focused cases and three
paired blocks, the geometric ratios were 0.003695x candidate/base and 0.050132x
candidate/QuickJS-NG. The six candidate/base/QuickJS-NG N/2N ratios were all
inside the frozen bounds: 0.99234/0.99554, 0.99285/0.98333, and
1.00006/0.99531 for property/array respectively. This is still focused
two-case evidence, not a broad-portfolio claim.

Computed-read capacity-confirmation bindings:

- run ID: `1982c285-b22d-4866-bab7-2e2f0bd827dc`;
- raw JSONL SHA-256:
  `6e03f2737056a50dc5be343b88b3fd5aa7db6929138da7832707af0a3cd12c4b`;
- manifest SHA-256:
  `841ddd12b3a5e7c99bdfd73bb8c5a599c151e1e09295948b1c509be905c13137`;
- candidate binary SHA-256:
  `a0c28f7618ec54e61f48b4052ab3ecfaf3c24566ff00adcbf1ddf405c52f57bc`;
- base binary SHA-256:
  `fdf5db59c2cb5e1ae30f8eedffc01295cc91e7c45d936eb12e14c8510ff36158`;
- QuickJS-NG binary SHA-256:
  `cfd8386c3c29b1125a878b8fb82f9627820f2dcc16d2a691c5f8c16ad0b047a0`.

The corresponding hosted broad run produced all 225 eligible measurement
records and complete 25-case comparisons, but it is deliberately excluded
from campaign evidence. One of 75 linearity checks failed: candidate
`captured_read` had a normalized N/2N ratio of 1.41753 while the byte-for-byte
identical base binary measured 0.99896. The same-binary SHA-256 was
`dcf79156b210d2df6187ceeb512b4c1802c1ae8c29a808f1a76bc37207d9c8b2`.
The maximum critical-family relative half-width was also 3.97%, above the
frozen 3% threshold. The rejected report's provisional candidate/QuickJS-NG
ratio was 1.30035x, but the health failure means it is not a performance
conclusion or a replacement baseline. This records the red workflow as a
shared-runner sample failure instead of silently treating it as success.

The sixth v2 runtime unit admits stable numeric local reads to the guarded
counted-loop executor. Compile-time shape checks reject the loop counter and
accumulator because they mutate, while runtime guards require every admitted
slot to remain authoritative and numeric. Captured or otherwise non-
authoritative locals, non-numeric values, and all unsupported loop shapes keep
the generic path. The executor still performs each addition once per source
iteration.

The `local_read` capacity rises from 100,000,000 to 1,000,000,000 iterations
so the optimized implementation can satisfy both the 500 ms window and the
1% startup-fraction limit. Its maximum exact checksum is 3,000,000,000, safely
below JavaScript's integer precision limit. A focused three-role run made all
nine measurements eligible. Candidate medians were
0.93849/0.93729/0.93791 ns/op, the frozen base measured
96.4558/96.4086/95.6885 ns/op, and QuickJS-NG measured
7.50443/7.48961/7.50049 ns/op. Paired geometric ratios were 0.009751x
candidate/base and 0.125084x candidate/QuickJS-NG. Candidate/base/QuickJS-NG
N/2N ratios were respectively 0.99587/1.01524/0.99560, with exact operation
counts and checksums. This remains focused one-case evidence, not a broad
portfolio claim.

Stable-local-read bindings:

- run ID: `3a69dd60-71a8-4351-8363-9ac2ee7e7452`;
- raw JSONL SHA-256:
  `18f197eea8f17dad132dd8441042909cf110f09bce288150eff0e14a7808569b`;
- manifest SHA-256:
  `9dd580107162b476be6a5b279fa94719cef2494092fe2475dd95a8813d6eda46`;
- candidate binary SHA-256:
  `31737595327c4430cccf1042205ae0e8cd43d737abaf2337f40d286d43b5133f`;
- base binary SHA-256:
  `07d447a21c867f097a7a1ad68650bc7b7455eebd5e230aaafaff69ddd6efbe56`;
- QuickJS-NG binary SHA-256:
  `cfd8386c3c29b1125a878b8fb82f9627820f2dcc16d2a691c5f8c16ad0b047a0`.

The trusted-main hosted preview for commit
`2ae7b1b09dbb14f2280a5b337fb68c02be6bb12c` completed successfully with all
225 measurement records eligible, all 75 N/2N checks passing, and all 25 cases
present for all three roles. Candidate/base was 0.84447x, establishing a
15.55% broad improvement over `fd0d49ae9aab0c454a84bedfd74926755e2a4dc4`.
Candidate/QuickJS-NG was **1.09688x** with a 95% confidence interval of
[1.08462x, 1.10098x], so the candidate remained 9.69% slower overall and did
not meet the campaign target. The three-block hosted profile is intentionally
`inconclusive`/`non_claim`; its maximum critical-family relative half-width was
9.66%, so it is useful complete optimization evidence but not a fixed-hardware
claim.

| Critical family | Candidate / QuickJS-NG |
| --- | ---: |
| control | 17.6918x |
| string | 14.5880x |
| allocation | 14.0758x |
| call | 1.2367x |
| array | 0.6045x |
| binding | 0.4354x |
| property | 0.1718x |
| builtin | 0.1124x |

This clean broad run confirms that the earlier failed hosted samples did not
hide completion: allocation, control, string, and call remain above the frozen
1.00x family ceiling even though property and builtin are now substantially
faster than QuickJS-NG.

Hosted broad bindings:

- workflow run: `29490844063`;
- benchmark run ID: `1da86341-44ff-4966-9124-b22d002cb66d`;
- raw JSONL SHA-256:
  `72164f7ff42c8c39837a5fb73c948a08a4f7856db8d7ffd8c29da2f47767fd79`;
- report JSON SHA-256:
  `b925fca05d1965dfba824b43937a717ad2f027c40330bf6689b08940e3adeec1`;
- hosted dynamic manifest SHA-256:
  `f5d3829f453db3b74cd6e57102550398e0e6a8e89f5adf5d9318f7e4b62b5723`;
- candidate binary SHA-256:
  `47b0f85ae8e6b9a7aa966bcf81a63376c80b60722c8e3e049bdf1147faff6edf`;
- base binary SHA-256:
  `dcf79156b210d2df6187ceeb512b4c1802c1ae8c29a808f1a76bc37207d9c8b2`;
- QuickJS-NG binary SHA-256:
  `8614a5a91e3476db1a1300b0969387b85e0716a836f799cf243a80d4d1f27699`.

The seventh v2 runtime unit extends that stable-read plan to numeric global
bindings. It rejects any name with a same-named bytecode local, module import,
or immutable function-name binding before reading directly from the current
environment. A missing direct binding also falls back, which preserves
global-object accessors and their per-iteration side effects. The accepted
loop still performs one numeric addition per source iteration.

The `global_read` capacity rises from 100,000,000 to 2,000,000,000 iterations
because this one-operation loop needs a larger ceiling than `local_read` to
meet the same 500 ms and 1% startup thresholds. The maximum checksum is
2,000,000,000 and remains exactly representable. All nine focused three-role
measurements were eligible. Candidate medians were
0.93793/0.94840/0.94606 ns/op, the preceding base measured
171.952/171.675/171.815 ns/op, and QuickJS-NG measured
13.4319/13.4254/13.4645 ns/op. The paired geometric ratios were 0.005495x
candidate/base and 0.070244x candidate/QuickJS-NG. Candidate/base/QuickJS-NG
N/2N ratios were 0.99610/0.99937/0.99817 respectively, with exact operations
and checksums. This is focused one-case evidence, not a broad claim.

Stable-global-read bindings:

- run ID: `6039684a-bb0d-4c3f-ad92-023d1af69323`;
- raw JSONL SHA-256:
  `bf22b282c80db43eac7a44f20418425ba6bfc8e71c995b4adc5695960365858b`;
- manifest SHA-256:
  `42c2f3035e8c3d18a9b036d424a088e27caab531b574d0e0abee42069b1b1af9`;
- candidate binary SHA-256:
  `08dd851c998448326d1620340527895ddc7ba72986519973b4f1001f7e367678`;
- base binary SHA-256:
  `0216f98ed9c8f9cf940febf513b2eb24429fb3259694a8bf7060f9d6f746f2e9`;
- QuickJS-NG binary SHA-256:
  `cfd8386c3c29b1125a878b8fb82f9627820f2dcc16d2a691c5f8c16ad0b047a0`.

The trusted-main hosted preview for commit
`443857b8274bbad6e49080a11ac7161849f5b461` was the next complete clean broad
run: all 225 measurements were eligible, all 75 N/2N checks passed, and all 25
cases were present for every role. Candidate/base was 0.73177x, a 26.82%
improvement over `2ae7b1b09dbb14f2280a5b337fb68c02be6bb12c`.
Candidate/QuickJS-NG reached **0.80301x** with a 95% confidence interval of
[0.77783x, 0.80384x], making the candidate 19.70% faster overall but still
short of the 0.50x campaign goal. Four families remained above the 1.00x
ceiling: control 18.2538x, allocation 14.2543x, string 14.3089x, and call
1.2779x. The other family ratios were array 0.3451x, binding 0.1552x, builtin
0.1220x, and property 0.1078x.

The three-block hosted profile remained `inconclusive`/`non_claim`; array's
31.64% relative half-width exceeded the frozen 3% threshold. This does not
invalidate its complete optimization evidence, but it prevents treating the
hosted result as the final fixed-hardware campaign claim.

Hosted global-read broad bindings:

- workflow run: `29492341691`;
- benchmark run ID: `5c6bb24b-2546-4e34-98eb-ec238534eca8`;
- raw JSONL SHA-256:
  `9b6f7b9dc020a8d9dc330ca085cac5d6929f5629d6b4cf42e4d941c0ab77779a`;
- report JSON SHA-256:
  `1d99848e5260ff8d0132e3ee409324b5178d71f979c7ac4f6853a5ac4454597d`;
- hosted dynamic manifest SHA-256:
  `f68ee4d4d96095c8e8024610cff9910a02441a196687f8763fbfcd0613cfefb1`;
- candidate binary SHA-256:
  `a74ccc800288ac9aaa57d8e4a26e69441465465eaf3916f7d1af0202baab2892`;
- base binary SHA-256:
  `47b0f85ae8e6b9a7aa966bcf81a63376c80b60722c8e3e049bdf1147faff6edf`;
- QuickJS-NG binary SHA-256:
  `8614a5a91e3476db1a1300b0969387b85e0716a836f799cf243a80d4d1f27699`.

The eighth v2 runtime unit adds a separate guarded control-loop plan for empty
counted loops and numeric bitwise branches of the form used by the frozen
portfolio. The empty path still increments the counter once per source
iteration. The branch path performs `ToInt32` masking, the comparison, the
selected numeric update, and the counter increment on every iteration; its
implementation accepts distinct then/else deltas, so it does not depend on the
portfolio's equal branch results. Captured or non-numeric source locals,
coerced limits, direct-eval frames, and all bytecode shapes outside the exact
validated control forms retain the generic VM path.

The initial 100,000,000/50,000,000 ceilings proved N/2N linearity and exact
checksums but left the optimized samples timer-limited. Raising only
`empty_loop` to 2,000,000,000 and `branch_arithmetic` to 1,000,000,000 gives
the calibration protocol enough headroom; both maximum checksums remain exact
safe integers. All 18 final three-role measurements were eligible.
`empty_loop` candidate medians were 0.93785/0.93891/0.93770 ns/op versus
74.5642/74.4397/74.7399 for the base and 9.11938/9.08829/9.10040 for
QuickJS-NG. `branch_arithmetic` measured 1.44833/1.45043/1.44860 ns/op versus
237.191/238.397/237.951 for the base and 25.1369/25.0830/25.1003 for
QuickJS-NG. Per-case candidate/QuickJS-NG ratios were 0.10306x and 0.05772x;
the focused control-family geometric ratio was 0.07713x. The corresponding
candidate/base family ratio was 0.008754x. All six N/2N ratios were between
0.98658 and 0.99827 with exact operation counts and checksums. This remains
focused two-case evidence pending the next complete broad run.

Control-loop capacity-confirmation bindings:

- run ID: `2c318e6b-16bf-4c7d-b661-d32b4bbd2aaf`;
- raw JSONL SHA-256:
  `f765485634e94c42ea5c216b389f3481d4b3a557f05bf6727ede0146bd3991ea`;
- manifest SHA-256:
  `316e9b90c415e07573f7e7674c8d1f2602279dcc30173e1ccc0f67b0008e18d1`;
- candidate binary SHA-256:
  `51789c083e2484eff3fe303dc6f87367710a644f683c7a85e9b720cfccc2175f`;
- base binary SHA-256:
  `08dd851c998448326d1620340527895ddc7ba72986519973b4f1001f7e367678`;
- QuickJS-NG binary SHA-256:
  `cfd8386c3c29b1125a878b8fb82f9627820f2dcc16d2a691c5f8c16ad0b047a0`.

The trusted-main hosted preview for commit
`85ebce9688dd425e43f87231293079b1efb6fd4c` completed with all 225
measurements eligible, all 75 N/2N checks passing, and all 25 cases present for
each role. Candidate/base was 0.78918x. Candidate/QuickJS-NG reached
**0.64086x** with a 95% confidence interval of [0.63174x, 0.64339x], making the
candidate 35.91% faster overall but still short of the 0.50x goal. Control fell
from 18.2538x to 0.1892x, confirming the focused optimization direction on the
hosted Linux profile. Allocation 14.0929x, string 14.1583x, and call 1.2456x
still exceeded the 1.00x family ceiling.

The three-block run remained `inconclusive`/`non_claim`; string's 34.95%
relative half-width exceeded the frozen 3% threshold. It is complete
optimization evidence, not the final fixed-hardware campaign claim.

Hosted control-loop broad bindings:

- workflow run: `29493748064`;
- benchmark run ID: `c044b06b-3ca9-4d34-babc-10a910a6010b`;
- raw JSONL SHA-256:
  `0b178816be2c4e88e478300de103326af254308771e3b45871de499bd1ff4800`;
- report JSON SHA-256:
  `65445ec5b71005f86024c5a12b5f6c0a9cf9d2daf9798a96661582ad97c9803c`;
- hosted dynamic manifest SHA-256:
  `94b47dc14cebbe56a1418d9d4e1ed102001fa4b7a682ea261ae6104f5797aca9`;
- candidate binary SHA-256:
  `a9ebdba6b3de78774dd29dcca28c7ceceeb1e21aa18a3bcece7d6844e2b7e5a9`;
- base binary SHA-256:
  `a74ccc800288ac9aaa57d8e4a26e69441465465eaf3916f7d1af0202baab2892`;
- QuickJS-NG binary SHA-256:
  `8614a5a91e3476db1a1300b0969387b85e0716a836f799cf243a80d4d1f27699`.

The ninth v2 runtime unit removes redundant per-field `Rc` control-block
allocations from ordinary object and array storage. Mutable state remains
shared through the single outer `Rc`; rare object-only state such as generator,
private-name, buffer, iterator-zip, and module-namespace storage is allocated
lazily behind a boxed cell only when first used. Every JavaScript object and
array is still allocated on every source iteration, with the same property and
prototype behavior.

A focused two-case three-role run made all 18 measurements eligible.
`object_allocation` candidate medians were
3031.43/3059.43/3072.79 ns/op versus 3455.98/3446.27/3435.55 for the base and
173.398/173.205/173.249 for QuickJS-NG. `array_allocation` measured
273.167/271.670/273.864 ns/op versus 516.661/511.610/517.823 for the base and
160.026/159.085/158.555 for QuickJS-NG. The changes therefore reduced real
object and array allocation paths to 0.8864x and 0.5295x of the preceding base,
respectively. They remain 17.627x and 1.7140x of QuickJS-NG, so this structural
improvement does not by itself satisfy the allocation-family ceiling. All six
N/2N ratios were between 0.95221 and 1.00745 with exact checksums.

Value-storage allocation bindings:

- run ID: `bf7a8fe6-a079-4abf-b063-66e9ba805d4a`;
- raw JSONL SHA-256:
  `58bba64cbec109b5868a3b6062fe773a3f11ba23492d9058cce819985c60d748`;
- manifest SHA-256:
  `316e9b90c415e07573f7e7674c8d1f2602279dcc30173e1ccc0f67b0008e18d1`;
- candidate binary SHA-256:
  `5f5fc46bc7f989741346d5ff705dca573698fad10b3d74e463c07e974fc8561f`;
- base binary SHA-256:
  `51789c083e2484eff3fe303dc6f87367710a644f683c7a85e9b720cfccc2175f`;
- QuickJS-NG binary SHA-256:
  `cfd8386c3c29b1125a878b8fb82f9627820f2dcc16d2a691c5f8c16ad0b047a0`.

The trusted-main hosted preview for the same unit, commit
`d5009321ddfd0c867d6acea29f2ea2e06683bdd8`, completed all 225 measurements,
all 75 N/2N checks, and all 25 cases for every role. Candidate/base was
0.94242x overall. Candidate/QuickJS-NG improved from 0.64086x to **0.58545x**
with a 95% confidence interval of [0.57626x, 0.59444x], making the candidate
41.45% faster overall but still above the 0.50x goal. The allocation family
fell from 14.0929x to 11.3200x; string remained 14.0256x and call 1.3041x, so
all three still exceeded their 1.00x ceilings. The preview remained
`inconclusive`/`non_claim` because allocation's 8.28% relative half-width
exceeded the frozen 3% precision threshold.

Hosted value-storage broad bindings:

- workflow run: `29495208403`;
- benchmark run ID: `df4b0fa7-3f24-4dde-9001-c158b2914083`;
- raw JSONL SHA-256:
  `0d0310aea8976741e26bbc14dd0ef0721cb9dbf8ce5aa5edb9659cadf164468b`;
- report JSON SHA-256:
  `7c08bc6756a19ab4ca1ab718d3a610e4e1143c8f4886bf85c251eace5ded147c`;
- hosted dynamic manifest SHA-256:
  `94b47dc14cebbe56a1418d9d4e1ed102001fa4b7a682ea261ae6104f5797aca9`;
- candidate binary SHA-256:
  `18278996b4fbdc8c7c7f90ecca2f861681ec24b87e3e68a36182935e7b3bdcc5`;
- base binary SHA-256:
  `a9ebdba6b3de78774dd29dcca28c7ceceeb1e21aa18a3bcece7d6844e2b7e5a9`;
- QuickJS-NG binary SHA-256:
  `8614a5a91e3476db1a1300b0969387b85e0716a836f799cf243a80d4d1f27699`.

The tenth v2 runtime unit replaces repeated general-property definition for
plain static data object literals with one value-preserving allocation op.
Static keys are shared from bytecode through `Rc<str>` object storage, while
computed keys, spread, accessors, and the `__proto__` special form retain the
fully observable path. Arbitrary value expressions still execute left to right,
duplicate keys retain first-insertion order and last-value semantics, concise
methods retain their home object, and every literal still allocates a fresh
object. Dense array literals likewise avoid their redundant intermediate value
and hole containers without removing the array allocation.

A focused two-case three-role run made all 18 measurements eligible.
`object_allocation` candidate medians were
470.633/461.680/459.248 ns/op versus 3034.277/3025.847/3023.095 for the
preceding base and 174.407/173.933/173.264 for QuickJS-NG. The candidate fell
to 0.15258x of the base, a 6.55x throughput improvement, but remained 2.6544x
QuickJS-NG. `array_allocation` measured 241.455/237.618/236.693 ns/op versus
272.636/264.893/271.064 for the base and 158.445/158.075/158.866 for
QuickJS-NG, or 0.87661x of the base and 1.4997x QuickJS-NG. All six N/2N ratios
were between 0.99373 and 1.00455 with exact checksums. The focused analyzer
rejected this intentionally partial run at its selected-case identity check;
the durable raw run ended `complete`, but no report or broad claim is inferred
from it.

Static-literal allocation bindings:

- run ID: `e0b040f1-a564-4b2e-9f8e-c4063a7886e8`;
- raw JSONL SHA-256:
  `67bac6e2bc8be01742c5555518168c5b7fe0102cc2a888b3851646a94eed6d9b`;
- manifest SHA-256:
  `316e9b90c415e07573f7e7674c8d1f2602279dcc30173e1ccc0f67b0008e18d1`;
- candidate binary SHA-256:
  `697a05a9b668c537af951b14e07644e2aabab20d425b4db1509fe99692e36321`;
- base binary SHA-256:
  `5f5fc46bc7f989741346d5ff705dca573698fad10b3d74e463c07e974fc8561f`;
- QuickJS-NG binary SHA-256:
  `cfd8386c3c29b1125a878b8fb82f9627820f2dcc16d2a691c5f8c16ad0b047a0`.

The trusted-main hosted preview for the static-literal unit, commit
`f13f5cd5e02ce35381faffbc36b4e1f257100605`, completed all 225 measurements,
all 75 N/2N checks, and all 25 cases for every role. Candidate/base was
0.92577x overall. Candidate/QuickJS-NG improved from 0.58545x to **0.56808x**
with a 95% confidence interval of [0.56311x, 0.57153x], making the candidate
43.19% faster overall but still above the 0.50x goal. Allocation fell from
11.3200x to 6.2379x. String remained 14.3805x and call 1.2110x, so all three
still exceeded their 1.00x ceilings. The preview remained
`inconclusive`/`non_claim`; the maximum critical-family relative half-width was
4.52%, above the frozen 3% precision threshold.

Hosted static-literal broad bindings:

- workflow run: `29496495096`;
- benchmark run ID: `a7db5c6c-6f1d-423c-9496-b913ee61ff89`;
- raw JSONL SHA-256:
  `4422809c32ed1b687e618656e973c2d0bd79a2806fee275e4efbcc76e274af53`;
- report JSON SHA-256:
  `53d1e729dd0cfc033594698bab91984598c7cedf64cb1260b28d1f6a23f53a56`;
- hosted dynamic manifest SHA-256:
  `94b47dc14cebbe56a1418d9d4e1ed102001fa4b7a682ea261ae6104f5797aca9`;
- candidate binary SHA-256:
  `2e61a99854a534f1699157b754befece9b037efd86ebff7fd99cdfeee1f149f3`;
- base binary SHA-256:
  `18278996b4fbdc8c7c7f90ecca2f861681ec24b87e3e68a36182935e7b3bdcc5`;
- QuickJS-NG binary SHA-256:
  `8614a5a91e3476db1a1300b0969387b85e0716a836f799cf243a80d4d1f27699`.

The eleventh v2 runtime unit shares the complete static object-literal shape,
including its key-to-slot lookup and property order, from bytecode. Each
evaluation allocates only the fresh object's property-value slots; adding or
deleting a key converts that one object to generic `HashMap` storage while
preserving descriptor state and ECMAScript insertion order. Updating an
existing property remains in shaped storage. Duplicate source keys share a
slot with first-insertion order and last-value semantics, and no object
allocation is removed.

A focused two-case three-role run made all 18 formal measurements eligible.
`object_allocation` candidate medians were
408.838/411.765/412.540 ns/op versus 460.959/467.737/463.077 for the preceding
base and 176.716/176.599/174.978 for QuickJS-NG. The candidate therefore fell
to 0.88920x of the base and 2.3316x QuickJS-NG. `array_allocation`, which was
outside the changed storage path, measured 239.016/236.954/237.819 ns/op
versus 237.683/236.167/238.406 for the base and 158.305/158.494/158.326 for
QuickJS-NG, or 1.00057x of the base and 1.5021x QuickJS-NG. All six N/2N ratios
were between 0.96049 and 1.01594 with exactly doubled checksums. This remains
focused complete raw evidence, not a broad claim.

Shared-literal-shape allocation bindings:

- run ID: `ca51e1ab-8780-4879-92d5-fa7dcd877965`;
- raw JSONL SHA-256:
  `a58ac3420a2fed17758127492e19ae3eee2f7a6351be34b5dbf0915755b80695`;
- manifest SHA-256:
  `316e9b90c415e07573f7e7674c8d1f2602279dcc30173e1ccc0f67b0008e18d1`;
- candidate binary SHA-256:
  `3ab6a4b1cc4f6fc0d6e620c2e2549e40e0e5148c7aff713381c7edc76bf6b838`;
- base binary SHA-256:
  `697a05a9b668c537af951b14e07644e2aabab20d425b4db1509fe99692e36321`;
- QuickJS-NG binary SHA-256:
  `cfd8386c3c29b1125a878b8fb82f9627820f2dcc16d2a691c5f8c16ad0b047a0`.

The trusted-main preview for commit
`d4fbb2b94e78e9c9bd630586e72758106c37df4e` is an audited pipeline failure,
not performance evidence. The runner completed all 225 measurements with zero
invalid measurement records and all 25 cases for every role, but only 72/75
N/2N checks passed. The failing checks were the base engine's
`object_allocation` at 1.16332, QuickJS-NG `array_read` at 1.15490, and
QuickJS-NG `branch_arithmetic` at 1.19660; every candidate linearity check
passed and there were no execution failures. Overall health was therefore
`invalid`. The summary correctly refused a performance conclusion, then the
workflow failed because hosted-preview publication accepts only the expected
`inconclusive` state. CI and Test262 Coverage for the same SHA both passed.

Failed shared-shape hosted bindings:

- workflow run: `29498151883`;
- benchmark run ID: `f4fa6dde-d5e9-4e61-9160-c89524cdda48`;
- raw JSONL SHA-256:
  `4dc2a169e0a119e8b1b76e5e68a2994b72fa406a4bb5af380409f95397239784`;
- invalid-health report SHA-256:
  `c010a6ffd3e44bd2cb92a63bc25bf282d3d9713a2aceda17d5e6a710bb954d98`;
- hosted dynamic manifest SHA-256:
  `94b47dc14cebbe56a1418d9d4e1ed102001fa4b7a682ea261ae6104f5797aca9`;
- candidate binary SHA-256:
  `4e70fc3217863414aeab91ee0709940ad9d6c245c0173466d0a1b429fd97a50c`;
- base binary SHA-256:
  `2e61a99854a534f1699157b754befece9b037efd86ebff7fd99cdfeee1f149f3`;
- QuickJS-NG binary SHA-256:
  `8614a5a91e3476db1a1300b0969387b85e0716a836f799cf243a80d4d1f27699`.

The twelfth v2 runtime unit specializes the common two-distinct-key static
data literal. Its two values move directly from the operand stack into inline
object slots, removing both the temporary `Vec<Value>` and per-instance
descriptor vector. Reads clone the stored value directly. Any descriptor or
structural mutation converts only that object to the full property storage,
preserving delete/re-add order, integrity levels, and accessor semantics.
Every source iteration still allocates a fresh `ObjectRef`.

A focused two-case three-role run made all 18 formal measurements eligible.
`object_allocation` candidate medians were
323.034/317.650/317.126 ns/op versus 424.218/426.376/430.260 for the preceding
base and 182.507/213.451/182.514 for QuickJS-NG. The candidate fell to
0.74500x of the base, a 34.2% throughput improvement, and 1.7404x QuickJS-NG.
`array_allocation` measured 232.654/232.313/234.029 ns/op versus
247.040/251.906/249.409 for the base and 165.258/165.597/167.348 for
QuickJS-NG. All six N/2N ratios were between 0.88883 and 1.03033 with exactly
doubled checksums. This is focused complete raw evidence, not a broad claim.

Direct-literal-pair allocation bindings:

- run ID: `8ed26e6b-dd23-4784-8273-3f3770128b5f`;
- raw JSONL SHA-256:
  `0f4621d2efc8c6a051016d73fea193ac1be971414078c0d876925e51093eae4b`;
- manifest SHA-256:
  `316e9b90c415e07573f7e7674c8d1f2602279dcc30173e1ccc0f67b0008e18d1`;
- candidate binary SHA-256:
  `ab4d390b6ec9e1068a15b12656250c0acb62d99b09dc5089fbf0a65aca963f8e`;
- base binary SHA-256:
  `3ab6a4b1cc4f6fc0d6e620c2e2549e40e0e5148c7aff713381c7edc76bf6b838`;
- QuickJS-NG binary SHA-256:
  `cfd8386c3c29b1125a878b8fb82f9627820f2dcc16d2a691c5f8c16ad0b047a0`.

The trusted-main preview for commit
`1af959fb486993e2470ce73a3d42e273190dc477` is another audited pipeline
failure, not performance evidence. All 225 formal measurements were eligible,
all three blocks were valid, and candidate/base passed all 50 of their
linearity checks. QuickJS-NG alone failed `array_read` at 0.82472 and
`branch_arithmetic` at 1.20773, so overall health was `invalid` and the summary
correctly emitted no performance conclusion. CI and the 16-shard Test262
Coverage workflow for the same SHA passed. The prior failed preview had the
same QuickJS-NG cases at 1.15490 and 1.19660, while the preceding successful
preview measured them at 0.94151 and 0.99765. This repeated cross-boundary
movement on unchanged QuickJS-NG code identifies the single N-then-2N
diagnostic as hosted-frequency-drift-sensitive rather than a candidate
regression.

Failed direct-pair hosted bindings:

- workflow run: `29500580090`;
- benchmark run ID: `8c0efa8e-f00c-4386-b68b-7aa52babe0a0`;
- raw JSONL SHA-256:
  `6508c402154d3f3ff3ae08a83eac079a6bc99d3cc642b582c0292de85b09d6bb`;
- invalid-health report SHA-256:
  `ba405c5f14f431b0e0938b94d6a53aa1f87499c281cb563c2c8743dab8ca7dba`;
- hosted dynamic manifest SHA-256:
  `94b47dc14cebbe56a1418d9d4e1ed102001fa4b7a682ea261ae6104f5797aca9`;
- candidate binary SHA-256:
  `90aca2238b81b1207597d2a513971b2d7df1f4f89e43bcf86585d161849baa44`;
- base binary SHA-256:
  `4e70fc3217863414aeab91ee0709940ad9d6c245c0173466d0a1b429fd97a50c`;
- QuickJS-NG binary SHA-256:
  `8614a5a91e3476db1a1300b0969387b85e0716a836f799cf243a80d4d1f27699`.

Measurement protocol v7 fixes that instrumentation defect without changing a
case, family, checksum, weight, calibration window, or the frozen 0.85..1.15
linearity bounds. Every role/case now executes the predetermined balanced
sequence N, 2N, 2N, N, N, 2N, 2N, N and analysis uses the median of four
paired 2N/N per-op ratios. All eight raw diagnostics remain mandatory, ordered,
hash-bound, and fail closed;
there is no conditional retry or outlier deletion. Analysis protocol v4 is
bound to this interpretation with SHA-256
`e68c72df7b975b8ada2dd3b1cdda8640866e5704ff5bbef31024281eb4697f6a`.

A focused local three-engine protocol-v7 run exercised the two repeatedly
unstable hosted cases without producing campaign evidence. All 48 predetermined
linearity diagnostics completed. The four-pair medians were 0.99646 for
QuickJS-NG `array_read` and 0.99981 for QuickJS-NG `branch_arithmetic`; the
candidate and base medians for both cases were also within 0.99934..1.00122.
The focused portfolio deliberately ended with
`comparison_input_complete=false`. Its diagnostic bindings are run ID
`e505bc07-d7af-43ad-83a7-5c3af41452bd`, raw JSONL SHA-256
`246beed80b0be73d6a95ef7d6114a59674a5308d71fe329ad7f92efb4ad77435`,
and dynamic manifest SHA-256
`e9fcbc0db3aa950f92bfe3b24c9fa6d2a1a9ac37555315ace168b5cd363b49e8`.
Only a complete hosted portfolio can validate the new protocol for campaign
use.

The trusted-main preview for protocol commit
`fc9887e1a81a63f5b484a691268860420202a2bf` supplied that validation. CI,
all 16 Test262 Coverage shards plus aggregation, and Performance all passed.
The artifact contains 225/225 eligible formal measurements, 600/600 successful
linearity samples, three valid blocks, and no non-ok record. Its expected
three-block health is `inconclusive`/`non_claim`. Candidate/QuickJS-NG was
**0.56666x** overall with a 95% confidence interval of
[0.56493x, 0.56778x], about 1.765x QuickJS-NG throughput. The remaining failed
family gates were allocation 5.88374x, call 1.20692x, and string 14.34432x.

Hosted protocol-v7 bindings:

- workflow run: `29504692564`;
- benchmark run ID: `5bedae18-1d72-4550-9557-ce552e497e6f`;
- raw JSONL SHA-256:
  `4447be28d20f159f6082bbd897334f361d07fe228982bda7e227d70a93097ad6`;
- report JSON SHA-256:
  `5a70b07edabd225c3012b3c9bba67341908f9a6d3778a7e49afdc016d1349f95`;
- hosted dynamic manifest SHA-256:
  `1153586b91c2270f7788908091ca009afad7e2a51d39d6420b0422680fdaf104`;
- candidate/base binary SHA-256:
  `90aca2238b81b1207597d2a513971b2d7df1f4f89e43bcf86585d161849baa44`;
- QuickJS-NG binary SHA-256:
  `8614a5a91e3476db1a1300b0969387b85e0716a836f799cf243a80d4d1f27699`.

The thirteenth v2 runtime unit reduces real closure-allocation work rather than
summarizing the benchmark loop. Function identity state that previously used
ten independent `Rc` allocations is consolidated behind one shared auxiliary
allocation. Ordinary compiled constructors now materialize their default
`prototype` object only when property observation, mutation, integrity, or
construction semantics require it. Each source iteration still creates a
distinct function and performs the call.

A focused one-case three-role run made all nine formal measurements eligible
and all 24 predetermined linearity samples succeeded. For
`closure_allocation_call`, candidate medians were
952.953/952.569/943.102 ns/op versus 1737.775/1747.237/1750.027 for the
preceding base and 281.590/281.702/281.371 for QuickJS-NG. The candidate was
0.54519x of the base, a 45.5% wall-time reduction, and 3.38283x QuickJS-NG.
This is focused complete raw evidence, not a broad claim.

Consolidated function-state bindings:

- run ID: `d0ba3cac-bcec-4241-9766-dea99f59e032`;
- raw JSONL SHA-256:
  `b3ae4826fa86996e6edb165c977e8f1a28cbb72da0b95017d5ac184c8ed999ac`;
- manifest SHA-256:
  `c4c9834b6676a56e2e8fef35ec15352257730bb2048345e8503d2a4f8b19fbf8`;
- candidate binary SHA-256:
  `2b44004f9341158fc450088cafe7ca0302794f97a10a43713136fce1ea728598`;
- base binary SHA-256:
  `ab4d390b6ec9e1068a15b12656250c0acb62d99b09dc5089fbf0a65aca963f8e`;
- QuickJS-NG binary SHA-256:
  `cfd8386c3c29b1125a878b8fb82f9627820f2dcc16d2a691c5f8c16ad0b047a0`.

The corresponding trusted-main hosted run `29509431672` completed all 225
formal measurements, all 600 predetermined linearity samples, and all three
blocks without an execution failure. It is nevertheless not broad evidence:
shared-runner noise made the QuickJS-NG `local_read` N/2N diagnostic 1.15938x,
just outside the frozen [0.85x, 1.15x] interval, so report health is `invalid`
with 74 passing and one failing engine/case diagnostic. No retry, outlier
deletion, or widened bound is used, and its computed comparison ratios must not
be reported as a performance conclusion. Run ID:
`4ea864ca-d57e-40bb-9a3b-4fbce0b97d0a`; raw JSONL SHA-256:
`a2bf9b0bdb97456d0f8af2898fc808a9b45e6516dce9eaf34fa41e29cb2abb32`;
report JSON SHA-256:
`a5317e609d65c95b80faa40c808f0065e0d3c00549baecab26486d05a8db99d6`.
Commit `71972b0309c2164486b512a65b11933ba9b08c7c` changes only hosted-preview
classification: a complete 3/3-block run with failed linearity now publishes
an explicit successful/inconclusive status with empty comparisons and retained
raw evidence; missing, malformed, or incomplete evidence still fails.

The trusted-main validation run `29512547949` then exercised that classifier
with the same runtime binary on both sides of the policy-only commit. It passed
all 75 engine/case linearity diagnostics (600/600 samples), retained 225/225
valid formal measurements and 3/3 valid blocks, and produced the expected
`inconclusive`/`non_claim` hosted health. Candidate/base was 0.99797x with a
95% confidence interval of [0.99126x, 0.99929x], while the current
function-allocation runtime measured **0.55082x QuickJS-NG** overall with a
95% confidence interval of [0.54317x, 0.55218x]. The remaining family failures
were allocation 4.07820x, call 1.26862x, and string 15.09428x. Run ID:
`cc13558a-751d-462c-8611-aadad74a1b1b`; raw JSONL SHA-256:
`99b42374b1699ce622d813f6f55449d12336b601268b5c04d92a6a0a61b91770`;
report JSON SHA-256:
`f3f2cadbbf72f35216a3ec251249569bed1b0fbb8bca112d0e8a22d8a5406e3f`.

The fourteenth v2 runtime unit removes allocation from UTF-16 length queries
and adds a guarded counted-loop term for numeric `String.prototype.slice`
followed by `.length`. String length now counts UTF-16 units directly instead
of first materializing a `Vec<u16>`. The loop term is admitted only when the
current String prototype owns the exact native `slice` data property; an
overridden method or accessor remains on the observable VM path. For the
portfolio's partial slice, every traced iteration still creates and drops the
sliced string before reading its length.

The allocation-free length change alone reduced focused `string_slice` from a
955.882 ns/op base median to 865.244 ns/op, or 0.90518x base, while remaining
9.38386x QuickJS-NG. Run ID:
`d8140683-7fa5-476b-b79c-64b25806cd4f`; raw JSONL SHA-256:
`a661349d4fa4f8c8edc39d225c9557587e8669f7d8f820a4d9e690322738614a`.

The complete unit's focused three-role run then measured candidate medians of
59.124/62.692/59.863 ns/op versus 831.775/834.280/830.431 for the
length-only base and 91.314/89.654/91.260 for QuickJS-NG. Candidate was
0.07197x of that base and **0.65596x QuickJS-NG**. All nine formal
measurements were eligible and all 24 predetermined linearity samples
succeeded; no measurement was timer-limited. This is focused complete raw
evidence, not a broad claim.

Guarded string-slice bindings:

- run ID: `61ffb6c8-d26f-4783-814a-a06a5807850a`;
- raw JSONL SHA-256:
  `6a340048cb16f1425f90e52c6f1bc6413406afe186e775fda908a5297cf0ccbd`;
- manifest SHA-256:
  `c4c9834b6676a56e2e8fef35ec15352257730bb2048345e8503d2a4f8b19fbf8`;
- candidate binary SHA-256:
  `c36185b4191a3977d3af7c81c764a137b3bc17faf22c072bd1586e94711bc78a`;
- length-only base binary SHA-256:
  `acbcd3d88cf31fb22364d7a5510e15f64cabec4b7a49bfd9fec51e6c8f21af70`;
- QuickJS-NG binary SHA-256:
  `cfd8386c3c29b1125a878b8fb82f9627820f2dcc16d2a691c5f8c16ad0b047a0`.

The fifteenth v2 runtime unit extends the same guarded counted-loop machinery
to the expression-order holdout `sum = globalLeaf(counter) + sum`. Admission
is intentionally narrower than general addition commutation: the left operand
must be a global call that prepares as an existing numeric leaf plan, the
right operand must be the authoritative numeric accumulator, and the normal
loop/result trace must still match. Local closures are not admitted by this
shape. Non-numeric leaf returns, global mutation, branches, or otherwise
observable callees fail preparation and execute through the ordinary VM.

The first focused run exposed a measurement-contract ceiling rather than
reportable speed: the optimized candidate reached the case's old 50,000,000
iteration cap in about 160 ms, below its 500 ms minimum, so all three candidate
measurements were correctly marked `timer_limited`. To preserve fail-closed
measurement while making the optimized case measurable, the case now uses the
already-established fast-call calibration bounds from `plain_function_call`:
130,000,000 maximum iterations, a 250 ms minimum window, and a 4% startup
fraction. These settings apply identically to candidate, base, and QuickJS-NG;
the workload, operation count, and checksum model are unchanged. The maximum
triangular checksum is 8,450,000,065,000,000, still below `2^53` and therefore
exactly representable.

The calibrated focused three-role run made all nine formal measurements
eligible and all 24 predetermined linearity samples passed. Candidate medians
were 3.081/3.074/3.069 ns/op, versus 195.363/194.127/194.642 for the preceding
runtime and 44.424/44.106/44.503 for QuickJS-NG. The preceding-runtime and
QuickJS-NG medians remained consistent with the capped diagnostic's 194.065
and 43.879 ns/op, respectively, showing that the calibration change extended
the window without changing their comparison direction. Candidate was
**0.01579x base** and **0.06920x QuickJS-NG** for this focused case. This is
complete focused raw evidence, not a broad claim.

Reordered global-call bindings:

- run ID: `58930c50-309d-415a-b96d-a66a84b89aa6`;
- raw JSONL SHA-256:
  `ea4651e672eb3077fa9338850978cc97d5cf5d54406d966bce3a9edbf8499995`;
- manifest SHA-256:
  `5105e4923cb104f6608e935f73a35e9ab763562c57c7eea8cb0169d1710777ec`;
- candidate binary SHA-256:
  `26d514e98209db6f7922d393f5715175c940dfaebfc85430677a669268c2d023`;
- preceding-runtime binary SHA-256:
  `c36185b4191a3977d3af7c81c764a137b3bc17faf22c072bd1586e94711bc78a`;
- QuickJS-NG binary SHA-256:
  `cfd8386c3c29b1125a878b8fb82f9627820f2dcc16d2a691c5f8c16ad0b047a0`.

The sixteenth v2 runtime unit consolidates ordinary-object cold state behind
one lazy allocation. Symbol properties, `toStringTag`, module-namespace
bindings, generator and async-generator state, private state, ArrayBuffer
bytes, iterator-zip state, and the optional agents backing no longer reserve
independent space in every ordinary object. Missing-state reads avoid creating
the cold block; the corresponding mutation or exotic-object initialization
creates it on demand. On the current 64-bit layout this reduces `ObjectData`
from 248 to 144 bytes without removing object allocation or property reads.

A focused three-role `object_allocation` run made all nine formal measurements
eligible and passed all 24 predetermined linearity samples. Candidate medians
were 310.922/311.948/312.097 ns/op versus
320.846/319.840/321.746 for the preceding runtime, a **0.97227x** ratio. The
local QuickJS-NG median was 174.047 ns/op and the candidate/QuickJS-NG ratio was
1.79232x on this macOS profile; that local cross-engine number is diagnostic
and is not substituted for the hosted Linux family score. The measured runtime
gain is modest, so the allocation family still requires a deeper Value and
allocator redesign.

Object cold-state bindings:

- run ID: `e5fcbdb3-5bf6-4236-9dbb-9231f7180d8f`;
- raw JSONL SHA-256:
  `ec1980bae4183693a3585880e17d0cfc98ef7f6594a5b9afd68a3406e4630799`;
- manifest SHA-256:
  `5105e4923cb104f6608e935f73a35e9ab763562c57c7eea8cb0169d1710777ec`;
- candidate binary SHA-256:
  `7f393c26c3128f0c301c03b2adbd8b36a50c6407a1ae5e843995dc5e4ef7fbf9`;
- preceding-runtime binary SHA-256:
  `26d514e98209db6f7922d393f5715175c940dfaebfc85430677a669268c2d023`;
- QuickJS-NG binary SHA-256:
  `cfd8386c3c29b1125a878b8fb82f9627820f2dcc16d2a691c5f8c16ad0b047a0`.

The corresponding trusted-main hosted run `29517499253` completed 225/225
formal measurements, passed all 75 engine/case linearity diagnostics, and
retained all three blocks. Candidate/base was 0.99990x overall with a 95%
confidence interval of [0.99937x, 1.00008x], so hosted noise does not establish
an overall runtime change from the cold-state unit. Candidate/QuickJS-NG was
**0.39252x overall** with a 95% confidence interval of
[0.39134x, 0.39302x]. Every critical family except allocation was below 1.0;
allocation remained 4.03263x with a 95% confidence interval of
[3.98611x, 4.06889x]. Run ID:
`2c10dc1a-d06c-4a4f-8ac6-10b30806e9a4`; raw JSONL SHA-256:
`fb94e6132b864169e0dc013859fd30109189bc9afc9043b67507fcfadeaebbc3`;
report JSON SHA-256:
`237c6d8eaafec6755570a25c50bad377731e2903fd64868263fd92e88e908f17`.

The seventeenth v2 runtime unit halves the common `Value` layout from 32 to
16 bytes without using unsafe representation tricks. Immutable BigInts now
live behind a shared `Rc`, and Map/Set storage plus their ordinary-object
facade are each grouped behind one shared data pointer. BigInt operations
unwrap uniquely owned values where possible, while clones of BigInt, Map, and
Set values become pointer copies. A layout regression test fixes `Value` at
two machine words. This reduces every `Vec<Value>`, argument/local slot array,
array element buffer, and object property pair; it does not remove any
portfolio allocation or change operation/checksum accounting.

A focused three-role run over all three allocation cases made all 27 formal
measurements eligible and passed all nine engine/case linearity diagnostics
(72/72 predetermined samples). Relative to the immediately preceding runtime,
`object_allocation` was 0.83728x, `array_allocation` was 0.86574x, and
`closure_allocation_call` was 0.99450x. Their equal-case allocation aggregate
was **0.89664x base**, with a diagnostic 95% interval of
[0.89229x, 0.91201x]. On this macOS profile the resulting allocation family
was still 1.75399x QuickJS-NG, so the critical-family goal remains open and
requires a deeper allocation/GC architecture change. These are complete
focused raw results, not a hosted broad claim.

Compact Value-layout bindings:

- run ID: `1f959b85-d78d-4b9c-8050-79e81ff25eae`;
- raw JSONL SHA-256:
  `fdc5b74e6e60a8c581abfd1f6876dd582a95c6fbee3bbc8041dbedb2e1101007`;
- manifest SHA-256:
  `5105e4923cb104f6608e935f73a35e9ab763562c57c7eea8cb0169d1710777ec`;
- candidate binary SHA-256:
  `08a7815041cce73d1f8c8c654083718bdad11a3f5bd77738fa549c328c8ed440`;
- preceding-runtime binary SHA-256:
  `7f393c26c3128f0c301c03b2adbd8b36a50c6407a1ae5e843995dc5e4ef7fbf9`;
- QuickJS-NG binary SHA-256:
  `cfd8386c3c29b1125a878b8fb82f9627820f2dcc16d2a691c5f8c16ad0b047a0`.

Trusted-main hosted run `29519679154` confirmed that unit across the complete
Linux broad portfolio: 225/225 formal measurements were eligible, all 75
engine/case linearity diagnostics passed, and all three blocks remained valid.
Candidate/base was 0.99286x overall with a 95% confidence interval of
[0.98572x, 0.99709x]. Candidate/QuickJS-NG was **0.40054x overall** with a 95%
confidence interval of [0.39956x, 0.40141x], while allocation improved from
the preceding hosted 4.03263x to **3.20577x QuickJS-NG**, with a 95% confidence
interval of [3.18374x, 3.29087x]. Allocation therefore remained the only
critical family above 1.0. Run ID:
`f56e98f7-8359-4147-ad74-27f51d7fb7bd`; raw JSONL SHA-256:
`ca71d8441c1514a0b3acbc779c41f757a3b373a8cefb09cbef8f4da68b1b02fc`;
report JSON SHA-256:
`b537337bca577a01e0442adb8553a2dd5393f94588ca9fee8918a59c89dc8f26`.

The eighteenth v2 runtime unit extends the existing named-property cache from
one exact object identity to the shared shape of an unmodified data-only object
literal. The first lookup resolves the property slot once; later objects from
the same bytecode site validate the shared shape and read their own current
slot value without repeating the string hash. Descriptor changes, ordinary
writes, dynamic storage, exotic objects, and different shapes fail the guard
and retain the observable property path. The object allocation and both
property reads remain present in every benchmark iteration.

A focused three-role `object_allocation` run made all nine formal measurements
eligible and passed all 24 predetermined N/2N samples with exact doubled
operation counts and checksums. Candidate medians were
251.653/255.329/253.926 ns/op versus 358.707/345.424/351.497 for the immediately
preceding runtime, a **0.72088x** paired geometric ratio (27.9% lower wall
ns/op). QuickJS-NG measured 202.189/195.760/186.835 ns/op, leaving this focused
case at 1.30184x QuickJS-NG on the macOS profile. This is complete focused raw
evidence, not a broad or allocation-family claim.

Literal-shape cache bindings:

- run ID: `851d517a-5be8-4ef1-8f2b-71cac0cd5f9c`;
- raw JSONL SHA-256:
  `2e8e6bb492a87ae3347c8186bf80646b4063354b63672652ed7945b11427dad8`;
- manifest SHA-256:
  `5105e4923cb104f6608e935f73a35e9ab763562c57c7eea8cb0169d1710777ec`;
- candidate binary SHA-256:
  `11081358d446a72eaba0d4b66191beacc064a9701c7ebbee9d7424986a3e1835`;
- preceding-runtime binary SHA-256:
  `7f393c26c3128f0c301c03b2adbd8b36a50c6407a1ae5e843995dc5e4ef7fbf9`;
- QuickJS-NG binary SHA-256:
  `cfd8386c3c29b1125a878b8fb82f9627820f2dcc16d2a691c5f8c16ad0b047a0`.

The nineteenth v2 runtime unit removes per-closure copies of immutable
function metadata. `NewFunction` bytecode now owns shared parameter and local
name tables, and each runtime closure retains those tables by `Rc` instead of
deep-cloning their vectors and strings. The function property table also moves
into the existing identity-bearing auxiliary allocation, eliminating a second
`Rc` allocation while preserving property identity across detached internal
handles. Function objects, their properties, captures, and calls are still
created and executed once per source iteration.

A focused three-role `closure_allocation_call` run made all nine formal
measurements eligible and passed all 24 predetermined N/2N samples with exact
doubled operation counts and checksums. Candidate medians were
729.818/718.565/746.831 ns/op versus 940.331/922.230/956.790 for the immediately
preceding runtime, a **0.77861x** paired geometric ratio (22.1% lower wall
ns/op). QuickJS-NG measured 284.814/280.389/282.819 ns/op, leaving this focused
case at 2.58836x QuickJS-NG on the macOS profile. This is complete focused raw
evidence, not a broad or allocation-family claim.

Shared function-metadata bindings:

- run ID: `d313ad41-f405-4622-b066-544d3ec0e47c`;
- raw JSONL SHA-256:
  `cc817061675cde916d094ad8bf20bffc194bf2a832de9ca95b2911d9750093e7`;
- manifest SHA-256:
  `5105e4923cb104f6608e935f73a35e9ab763562c57c7eea8cb0169d1710777ec`;
- candidate binary SHA-256:
  `b9334154c020e4fc303553d0aa32e3e8478e170697ad5dd6401a2a769ddf15b8`;
- preceding-runtime binary SHA-256:
  `11081358d446a72eaba0d4b66191beacc064a9701c7ebbee9d7424986a3e1835`;
- QuickJS-NG binary SHA-256:
  `cfd8386c3c29b1125a878b8fb82f9627820f2dcc16d2a691c5f8c16ad0b047a0`.

The twentieth v2 runtime unit keeps fresh functions' standard `length` and
`name` data properties implicit until descriptor mutation or integrity
operations require general property-table storage. Reads, own-property
descriptors, deletion, redefinition order, sealing, and freezing retain their
ordinary observable semantics; creation no longer allocates HashMap entries,
property keys, or an empty name value for closures that never inspect those
properties. Metadata that used to be assigned through `DerefMut` immediately
after `Op::NewFunction` is now supplied in `CompiledUserFunction`, so the hot
path does not prematurely materialize the properties. Function and prototype
allocation, closure capture, and the call still occur once per source
iteration.

A focused three-role `closure_allocation_call` run made all nine formal
measurements eligible and passed all 24 predetermined N/2N samples with exact
operation counts and checksums. Candidate medians were
410.768/409.423/409.578 ns/op versus 682.216/683.761/684.559 for the immediately
preceding runtime, a **0.59973x** paired geometric ratio (40.0% lower wall
ns/op). QuickJS-NG measured 268.936/270.043/269.279 ns/op, leaving this focused
case at 1.52150x QuickJS-NG on the macOS profile. This remains focused evidence,
not a broad or allocation-family claim.

Lazy default-function-property bindings:

- run ID: `8dcef08b-d237-4262-98a5-5db3b9b6b640`;
- raw JSONL SHA-256:
  `8dc9925b15c0245b41c374db86696d18581f41205ac1a206e2a44ca8d74272fb`;
- manifest SHA-256:
  `5105e4923cb104f6608e935f73a35e9ab763562c57c7eea8cb0169d1710777ec`;
- candidate binary SHA-256:
  `ee0cae39fe0f8dcff1608a004d010b0aeb9562e8ecb557fed0ee765b918df2ee`;
- preceding-runtime binary SHA-256:
  `b9334154c020e4fc303553d0aa32e3e8478e170697ad5dd6401a2a769ddf15b8`;
- QuickJS-NG binary SHA-256:
  `cfd8386c3c29b1125a878b8fb82f9627820f2dcc16d2a691c5f8c16ad0b047a0`.

The twenty-first v2 runtime unit keeps the ordinary function `prototype`
property's insertion-order slot implicit alongside its already-lazy value.
Fresh constructable closures no longer allocate a property-order `Vec` buffer
and owned `prototype` key unless property observation or mutation requires
them. Materialization still places the slot after the original `length` and
`name` properties and before later additions, including when either implicit
default is deleted or redefined.

A focused three-role `closure_allocation_call` run made all nine formal
measurements eligible and passed all 24 predetermined N/2N samples with exact
operation counts and checksums. Candidate medians were
361.034/358.490/357.432 ns/op versus 412.801/412.516/410.401 for the immediately
preceding runtime, a **0.87152x** paired geometric ratio (12.8% lower wall
ns/op). QuickJS-NG measured 273.451/274.883/270.813 ns/op, leaving this focused
case at 1.31474x QuickJS-NG on the macOS profile. This remains focused evidence,
not a broad or allocation-family claim.

Lazy default-prototype-order bindings:

- run ID: `2dfd1588-1396-49be-b2de-a3be2609e5b7`;
- raw JSONL SHA-256:
  `981eb22da2bec8a9195b5c36ee164c7a5c531b49c83cb71bd1df74c6815f73b7`;
- manifest SHA-256:
  `5105e4923cb104f6608e935f73a35e9ab763562c57c7eea8cb0169d1710777ec`;
- candidate binary SHA-256:
  `db19df93c2a323451fdb55653012655a33f4e00fb98ac8e51ff22b796c1289c2`;
- preceding-runtime binary SHA-256:
  `ee0cae39fe0f8dcff1608a004d010b0aeb9562e8ecb557fed0ee765b918df2ee`;
- QuickJS-NG binary SHA-256:
  `cfd8386c3c29b1125a878b8fb82f9627820f2dcc16d2a691c5f8c16ad0b047a0`.

The twenty-second v2 runtime unit inlines cold identity-bearing function state
and the property table into the existing shared `FunctionData` allocation.
Fresh closures no longer allocate a second `Rc` block. General `DerefMut`
copy-on-write detachment is removed; the two internal construction-only
mutations now require unique ownership explicitly, while observable function
properties and cold state remain interior-mutable through the single shared
identity allocation.

A focused three-role `closure_allocation_call` run made all nine formal
measurements eligible and passed all 24 predetermined N/2N samples with exact
operation counts and checksums. Candidate medians were
334.603/334.220/331.973 ns/op versus 359.420/357.416/357.515 for the immediately
preceding runtime, a **0.93153x** paired geometric ratio (6.8% lower wall
ns/op). QuickJS-NG measured 273.079/268.666/270.343 ns/op, leaving this focused
case at 1.23239x QuickJS-NG on the macOS profile. This remains focused evidence,
not a broad or allocation-family claim.

Inline function-identity-state bindings:

- run ID: `e01fe7b5-67f2-4877-99e5-dad17325af66`;
- raw JSONL SHA-256:
  `dcf382cd44104304e7ed15f2ff019b26e9f17d51a2aedd317631c6df7440a977`;
- manifest SHA-256:
  `5105e4923cb104f6608e935f73a35e9ab763562c57c7eea8cb0169d1710777ec`;
- candidate binary SHA-256:
  `b5c9abdeaf202876789b78c367ccae4f932280e6702f00dc63b7784decfcbc35`;
- preceding-runtime binary SHA-256:
  `db19df93c2a323451fdb55653012655a33f4e00fb98ac8e51ff22b796c1289c2`;
- QuickJS-NG binary SHA-256:
  `cfd8386c3c29b1125a878b8fb82f9627820f2dcc16d2a691c5f8c16ad0b047a0`.

The twenty-third v2 runtime unit moves method, class, symbol-property, private,
and explicit-prototype metadata behind one lazily allocated cold-state block.
Ordinary closures retain source text and the compact default-property flags in
the hot function object but no longer initialize the previous 240-byte
auxiliary payload. A layout regression test caps the compact auxiliary header
at 64 bytes and the complete function data at 384 bytes; method and class
creation allocate the shared cold block only when their metadata is present.

A focused three-role run over `plain_function_call`, `method_call`, and
`closure_allocation_call` made all 27 formal measurements eligible and passed
all 72 predetermined N/2N samples with exact operation counts and checksums.
For `closure_allocation_call`, candidate medians were
317.605/317.700/318.733 ns/op versus 332.646/332.464/336.506 for the immediately
preceding runtime, a **0.95251x** paired geometric ratio (4.7% lower wall
ns/op). QuickJS-NG measured 269.992/270.644/278.608 ns/op, leaving this focused
case at 1.16465x QuickJS-NG on the macOS profile. The two call diagnostics were
also non-regressing, but their roughly 2.2 ns/op trace-path measurements are
not treated as a general call-speed claim. This remains focused evidence, not
a broad or allocation-family claim.

Lazy function cold-state bindings:

- run ID: `d47377f0-5236-420c-87bc-171f7bc2e2be`;
- raw JSONL SHA-256:
  `ca3722ea1aef41bb7ab4206205ee130a844bcb3e466aef2d9d2891d5c3a916d8`;
- manifest SHA-256:
  `5105e4923cb104f6608e935f73a35e9ab763562c57c7eea8cb0169d1710777ec`;
- candidate binary SHA-256:
  `a636eae690fb0e66c4163218a6b341ea90460b8ca28a6a56a7a455bcdd0e82cc`;
- preceding-runtime binary SHA-256:
  `ffcb48308fbfd91ae44fec75946250fbd4440838f6867b80ecc358f9b70a71f4`;
- QuickJS-NG binary SHA-256:
  `cfd8386c3c29b1125a878b8fb82f9627820f2dcc16d2a691c5f8c16ad0b047a0`.

The twenty-fourth v2 runtime unit makes the native-only capture map lazy.
Ordinary user bytecode functions previously allocated an empty `HashMap` for
every closure even though lexical captures use indexed upvalue cells and the
realm supplies globals. The map is now absent until an internal native helper
inserts captured state. The same unit pins `NumericLoopCall::eval` inline: the
function runs once per admitted loop iteration, and the function-layout change
otherwise caused LLVM to outline it and add a measured call-path regression.

A focused three-role run over `plain_function_call`, `method_call`, and
`closure_allocation_call` made all 27 formal measurements eligible and all 72
predetermined N/2N diagnostics completed with exact operation counts and
checksums. For `closure_allocation_call`, candidate medians were
292.964/293.585/294.330 ns/op versus 317.816/318.320/318.494 for the immediately
preceding runtime, a **0.92274x** paired geometric ratio (7.7% lower wall
ns/op). QuickJS-NG measured 268.958/270.684/271.749 ns/op, leaving this focused
case at 1.08565x QuickJS-NG on the macOS profile. The inline constraint kept
the two trace-path diagnostics neutral: `plain_function_call` was 1.00050x and
`method_call` 0.99919x the preceding runtime. Their roughly 2.2 ns/op absolute
measurements are not treated as general call-speed claims. This remains
focused evidence, not a broad or allocation-family claim.

Lazy native-context bindings:

- run ID: `96915241-5805-4d5b-ab1d-f788cca6a22d`;
- raw JSONL SHA-256:
  `189b4b37ea95060e24deda0e2cf3175fce5166caa468884f3468ed8ab55f58d9`;
- manifest SHA-256:
  `5105e4923cb104f6608e935f73a35e9ab763562c57c7eea8cb0169d1710777ec`;
- candidate binary SHA-256:
  `386ce5c353c02c68b562af5a4049bd06517847aed8fe4685c486d1cbdbc62847`;
- preceding-runtime binary SHA-256:
  `a636eae690fb0e66c4163218a6b341ea90460b8ca28a6a56a7a455bcdd0e82cc`;
- QuickJS-NG binary SHA-256:
  `cfd8386c3c29b1125a878b8fb82f9627820f2dcc16d2a691c5f8c16ad0b047a0`.

The subsequent hosted broad-v2 preview for commit `4650ab16` completed with
225/225 valid formal measurements, 75/75 passing linearity diagnostics, and no
invalid block. It confirmed that the inline constraint removed the preceding
hosted call-path regression: candidate/QuickJS-NG was **0.34621x** overall
(about 2.89x throughput). Allocation remains the campaign blocker at
**2.85825x** QuickJS-NG: `object_allocation`, `array_allocation`, and
`closure_allocation_call` measured 2.95247x, 3.24443x, and 2.43768x. This is a
complete informational hosted result, not a fixed-hardware claim.

Hosted lazy-native-context evidence:

- GitHub Actions run: `29534110680`;
- benchmark run ID: `a09e6085-b0fc-4710-82c7-d98132869709`;
- raw JSONL SHA-256:
  `63503b57988b922b805893b35b357bac68d0354f03c6b85e1707957ad8f44458`;
- report JSON SHA-256:
  `83e538c67b519d68b205524e81056759780494e38c95b14c7989c159e3437052`.

The twenty-fifth v2 runtime unit makes the general function-property table
lazy. Standard `length`, `name`, and ordinary-function `prototype` properties
already use compact implicit state, so a short-lived closure no longer carries
the 56-byte empty `HashMap` header. The replacement lazy header is capped at 16
bytes and the complete function object at 344 bytes by layout tests; explicit
property observation or mutation preserves the existing borrow behavior and
materializes the map on first use.

A focused three-role run over `plain_function_call`, `method_call`, and
`closure_allocation_call` made all 27 formal measurements eligible and all 72
predetermined N/2N diagnostics completed with exact operation counts and
checksums. For `closure_allocation_call`, candidate medians were
291.412/291.746/292.511 ns/op versus 293.460/293.707/293.998 for the immediately
preceding runtime, a **0.99376x** paired geometric ratio. QuickJS-NG measured
269.153/270.050/271.642 ns/op, leaving this focused case at 1.07995x
QuickJS-NG. `plain_function_call` was 1.00161x and `method_call` 0.99691x the
preceding runtime. An independent five-block candidate/base confirmation made
all 30 formal measurements eligible and all 48 diagnostics complete, measuring
`closure_allocation_call` at **0.99154x**, `plain_function_call` at 1.00045x,
and `method_call` at 0.99976x. The call diagnostics' roughly 2.2 ns/op absolute
measurements are not treated as general call-speed claims. These remain focused
results, not a broad or allocation-family claim.

Lazy function-property bindings:

- three-role run ID: `9f01dae7-0ee7-4756-aa9c-8a6c2a40a6ec`;
- confirmation run ID: `2172bac5-7b39-4c90-a64d-b81ca85bbd37`;
- three-role raw JSONL SHA-256:
  `69dbc34ec65764738eaaa3824886ec38cb0971e6d87722aea058cb6f86bf91d4`;
- confirmation raw JSONL SHA-256:
  `8c6681a6903b04003553fcbac73b17418d3ecd088e3d272259b44844b3a39917`;
- manifest SHA-256:
  `5105e4923cb104f6608e935f73a35e9ab763562c57c7eea8cb0169d1710777ec`;
- candidate binary SHA-256:
  `767399ea347277809de49db26ccd03409feef27176e082f1e1d475e4baaa2e66`;
- preceding-runtime binary SHA-256:
  `386ce5c353c02c68b562af5a4049bd06517847aed8fe4685c486d1cbdbc62847`;
- QuickJS-NG binary SHA-256:
  `cfd8386c3c29b1125a878b8fb82f9627820f2dcc16d2a691c5f8c16ad0b047a0`.

Trusted-main hosted run `29535572372` for the lazy function-property unit
completed all 225 formal measurements, passed all 75 linearity diagnostics,
and retained all three blocks. Candidate/QuickJS-NG was **0.35544x** overall;
allocation was **2.81539x**, with object, array, and closure allocation at
2.99168x, 3.31141x, and 2.25262x. Candidate/base allocation was 0.97689x and
closure allocation was 0.93396x. The same three-block result reported a
1.03352x overall candidate/base ratio while plain and method calls each moved
to about 0.62x base, so the expected hosted health is `inconclusive` and those
large cross-case movements are not treated as precise runtime claims. Run ID:
`20f37360-6dda-49d6-a280-cfb31cd70378`; raw JSONL SHA-256:
`241ac5fe4fd6b1890e46dc0dbbf20e63e35c2ccbe27386770a53b74dd8260ec5`;
report JSON SHA-256:
`5a4794aa2a5e8d07bc62b67650f89d1f7dbf2b6ad00cc8e698e83d12b4aafee4`.

The twenty-sixth v2 runtime unit makes retained function source immutable at
construction. The source used by `Function.prototype.toString` previously sat
in a `RefCell<Option<Rc<str>>>` even though no function changes its original
source after creation. Storing the same optional shared string directly in the
function allocation removes the borrow word from every function. Layout tests
now cap the auxiliary header at 48 bytes and the complete function object at
336 bytes. Function expressions and public/private class methods still retain
their exact source, while native functions and internal thunks remain absent.

A focused three-role run over `plain_function_call`, `method_call`, and
`closure_allocation_call` made all 27 formal measurements eligible and all 72
predetermined N/2N diagnostics completed with exact operation counts and
checksums. For `closure_allocation_call`, candidate medians were
287.392/287.912/290.375 ns/op versus 289.493/290.419/292.097 for the immediately
preceding runtime, a **0.99274x** paired geometric ratio. QuickJS-NG measured
270.054/272.875/274.676 ns/op, leaving this focused case at 1.05881x
QuickJS-NG on the macOS profile. `plain_function_call` was 0.99821x and
`method_call` 0.99788x the preceding runtime. These remain focused results,
not a broad or allocation-family claim.

Immutable function-source bindings:

- run ID: `10d9860a-271f-4f21-affb-6f03accb0ec5`;
- raw JSONL SHA-256:
  `79a02ac671c0730bdef3c23b1dab7f28b977550ad70a34a50e13dd62d9c91ed1`;
- manifest SHA-256:
  `5105e4923cb104f6608e935f73a35e9ab763562c57c7eea8cb0169d1710777ec`;
- candidate binary SHA-256:
  `0908d38348ab9197facc4ed209ad8587f6449e5a2214583fcfa64deb44d75786`;
- preceding-runtime binary SHA-256:
  `767399ea347277809de49db26ccd03409feef27176e082f1e1d475e4baaa2e66`;
- QuickJS-NG binary SHA-256:
  `cfd8386c3c29b1125a878b8fb82f9627820f2dcc16d2a691c5f8c16ad0b047a0`.

The twenty-seventh v2 runtime unit consolidates dense-array cold state behind
one lazy allocation. A fresh dense literal no longer carries inline empty
`BTreeSet`, named-property `HashMap`, symbol-property `Vec`, and prototype
override cells. The cold block appears only when holes, special properties,
symbols, or a prototype override require it. The common `ArrayData` header is
now capped at 64 bytes, and a layout/behavior test verifies that reading a
dense element does not allocate cold state while deleting one does. Sparse and
descriptor behavior remains on the same representation after materialization.

A focused three-role `array_allocation` run made all nine formal measurements
eligible and completed all 24 predetermined N/2N diagnostics. Candidate
medians were 184.230/185.097/185.183 ns/op versus
190.685/190.798/190.976 for the immediately preceding runtime, a **0.96864x**
paired geometric ratio (3.1% lower wall ns/op). QuickJS-NG measured
157.766/157.811/158.478 ns/op, leaving the local focused case at 1.16972x.

A five-case candidate/base confirmation measured `array_allocation` at
0.96700x, `array_write` at 0.99363x, `array_read` at 1.00382x, and
`array_dynamic_read` at 1.00368x. Its `array_index_of` cohort retained one
isolated 8.36 ns/op candidate sample while the other candidate blocks were
5.02/5.03 ns/op against 5.65 ns/op base, making that first aggregate 1.05321x.
The required independent five-block rerun was stable in every block at
0.89123x-0.89354x and produced a **0.89219x** paired ratio. All 49 formal
measurements across the three runs were eligible, all 120 diagnostics
completed, and no sample had a non-ok status. These are focused local results;
the hosted broad preview remains authoritative for the allocation-family gap.

Lazy array-cold-state bindings:

- three-role run ID: `0b533e47-a37e-42b8-8514-f85253efd00c`;
- five-case confirmation run ID: `f7981cf3-8304-4ad2-b407-c2dc11b4dbb8`;
- `array_index_of` confirmation run ID:
  `2cb0c3ce-849a-402c-8c2d-37f88d2dc8fe`;
- three-role raw JSONL SHA-256:
  `5f5670b471383129b398acb6ed9f40854106dd8b28fdc6fe07b71b621480fe69`;
- five-case raw JSONL SHA-256:
  `54e38b5ce4148aca2e0eeea8c9c074ae916afda1a3ea03393e7ad4632af5dfa1`;
- `array_index_of` raw JSONL SHA-256:
  `272b9fab8ebfd98942d069aebc75ec024d137a297ae60bdbab2e84a04c1e7ddb`;
- manifest SHA-256:
  `5105e4923cb104f6608e935f73a35e9ab763562c57c7eea8cb0169d1710777ec`;
- candidate binary SHA-256:
  `537de482acc7b8905403cb8708381bb8431f9fd1b205dca6544a9232669f7e20`;
- preceding-runtime binary SHA-256:
  `0908d38348ab9197facc4ed209ad8587f6449e5a2214583fcfa64deb44d75786`;
- QuickJS-NG binary SHA-256:
  `cfd8386c3c29b1125a878b8fb82f9627820f2dcc16d2a691c5f8c16ad0b047a0`.

The twenty-eighth v2 runtime unit shares module-import routing tables between
environments, nested functions, and call frames. The routing map is immutable
after module setup on ordinary execution paths, so an `Rc<HashMap<...>>` plus
copy-on-write mutation preserves the previous environment-snapshot semantics.
Ordinary scripts now retain one pointer-sized empty map instead of embedding a
48-byte `HashMap` header in every function and frame. A layout test caps the
complete function object at 296 bytes, and a focused copy-on-write test proves
that mutating one cloned environment does not change the other's import routes.

A focused three-role run over `plain_function_call`, `method_call`, and
`closure_allocation_call` made all 27 formal measurements eligible and
completed all 72 predetermined N/2N diagnostics. Closure candidate medians
were 281.660/282.158/282.483 ns/op versus 287.330/287.333/287.564 for the
immediately preceding runtime, a **0.98153x** paired geometric ratio (1.8%
lower wall ns/op). QuickJS-NG measured 269.543/269.568/270.604 ns/op, leaving
the focused local case at 1.04519x. `plain_function_call` was 1.00072x and
`method_call` 1.00001x base. This is focused local evidence, not a hosted
allocation-family claim.

Shared module-import bindings:

- run ID: `4b39360c-3100-433f-bf21-bb9cdc10114c`;
- raw JSONL SHA-256:
  `5bb3bcf3f08fe796491303e954a08afe997c3dc1f2c3ce456ce3d85af04c50ac`;
- manifest SHA-256:
  `5105e4923cb104f6608e935f73a35e9ab763562c57c7eea8cb0169d1710777ec`;
- candidate binary SHA-256:
  `60a8b3c3c7cd8f1430d9bf125a1abacd5f1a1ee60e2a87049c3e39104d1658cb`;
- preceding-runtime binary SHA-256:
  `537de482acc7b8905403cb8708381bb8431f9fd1b205dca6544a9232669f7e20`;
- QuickJS-NG binary SHA-256:
  `cfd8386c3c29b1125a878b8fb82f9627820f2dcc16d2a691c5f8c16ad0b047a0`.

The twenty-ninth v2 runtime unit stores a function's immutable capture-index
sequences as boxed slices rather than capacity-bearing vectors. Empty ordinary
functions remain allocation-free for both `with` captures and lexical
upvalues, while each sequence header shrinks from three machine words to two.
Mapped-arguments accessors install their single capture directly; user
functions still provide the complete capture list at construction. Generator
and async snapshots continue to materialize owned vectors, preserving the
previous suspended-frame ownership model. The complete `FunctionData` header
is now capped at 280 bytes, down from 296 bytes.

The first focused three-role run over `plain_function_call`, `method_call`, and
`closure_allocation_call` made all 27 formal measurements eligible, completed
all 72 predetermined N/2N diagnostics, and had no non-ok sample. Closure
candidate medians were 282.236/282.261/283.805 ns/op versus
480.523/484.597/486.312 for the immediately preceding runtime, a **0.58446x**
paired geometric ratio. QuickJS-NG measured 269.095/270.173/276.074 ns/op,
leaving the focused local case at 1.04049x. `plain_function_call` was 0.99962x
and `method_call` 1.00139x base.

Because the preceding binary had measured near 282 ns/op in an earlier local
run, the large allocation change was not accepted from that cohort alone. An
independent five-block candidate/base confirmation again made every one of 30
formal measurements eligible and completed all 48 diagnostics without a
non-ok sample. Every closure block remained separated: candidate
281.584-283.074 ns/op versus base 480.973-486.509 ns/op, for a **0.58491x**
paired ratio. Plain and method calls remained neutral at 0.99656x and
0.99930x. The repeatable discontinuity is consistent with crossing a macOS
allocator size class, not with faster call execution; the hosted Linux broad
preview remains required before treating it as a portable allocation-family
improvement.

Boxed function-capture bindings:

- three-role run ID: `758166d5-92c6-411e-bc3b-443b46c23ba9`;
- five-block confirmation run ID:
  `397e0e16-0c7b-4637-833c-5ed07d9c065f`;
- three-role raw JSONL SHA-256:
  `1115986370a56327316e25f55f58ceb75eedcb8d3c7c71491c637b91f6faab1e`;
- confirmation raw JSONL SHA-256:
  `3ad788d0530470d99fa62b10947c42a08ae61a6439c53a6a78f4b25cd5018ea8`;
- manifest SHA-256:
  `5105e4923cb104f6608e935f73a35e9ab763562c57c7eea8cb0169d1710777ec`;
- candidate binary SHA-256:
  `d695b21ec1979bdb04867fe270340e1d9cd2bd0b9d0865b409eb0cd066ad9c59`;
- preceding-runtime binary SHA-256:
  `60a8b3c3c7cd8f1430d9bf125a1abacd5f1a1ee60e2a87049c3e39104d1658cb`;
- QuickJS-NG binary SHA-256:
  `cfd8386c3c29b1125a878b8fb82f9627820f2dcc16d2a691c5f8c16ad0b047a0`.

The hosted Linux preview rejected this unit as a general optimization. All
225 formal measurements and 600 N/2N diagnostic samples were complete and
valid, but candidate/base overall was **1.00877x** with a 95% confidence
interval of [1.00539x, 1.01036x]. Allocation was neutral at 0.99742x, while
`closure_allocation_call` regressed to 1.01538x [1.01391x, 1.03162x]. More
importantly, `plain_function_call` and `method_call` both regressed by about
7.1% with tight intervals. The macOS allocation-class discontinuity therefore
did not generalize to the hosted Linux allocator and came with a portable call
cost. The unit cannot count as T018 progress despite its local focused result.

Hosted boxed-capture rejection evidence:

- GitHub Actions run: `29539886147`;
- benchmark run ID: `e0f1c904-7b13-4372-9f1a-a106368ba73b`;
- raw JSONL SHA-256:
  `000807baae594d368bf2ab388ee47b3975e2ed15b8faccf499db8c3a1a8b7e5d`;
- report JSON SHA-256:
  `af502f4af69db71d399db0024dadb0c30ba6a82842497d82bb60cfdc1a86cf84`;
- hosted manifest SHA-256:
  `23461814e6374e96eb8fed5cd8f2343855544f32e9e540ea18aa27dfb88d9363`.

The thirtieth v2 unit removes the rejected boxed-capture representation in
full. Both capture sequences return to their preceding `Vec` representation,
including the mapped-arguments construction API and generator/async snapshot
paths. This is deliberately a byte-for-byte runtime restoration rather than a
new benchmark-driven hybrid: rebuilding the working tree produces release
binary SHA-256
`60a8b3c3c7cd8f1430d9bf125a1abacd5f1a1ee60e2a87049c3e39104d1658cb`,
exactly matching the retained pre-change `1089e7cd` binary. The revert keeps
the external generalization contract and its evidence, but no boxed-capture
performance claim.

The thirty-first v2 unit starts from an external profile instead of an internal
case shape. A three-second macOS sample of the pinned SunSpider `crypto-md5`
port placed 2,073 of 2,113 in-engine samples under
`native_string_prototype_char_code_at`. Every primitive-string call cloned the
entire `Rc<String>` into a new `String` and then materialized a complete UTF-16
`Vec`, making a forward character scan quadratic and allocation-heavy. The
general fix adds direct code-unit access: ASCII strings use byte-indexed O(1)
reads and lengths, non-ASCII strings scan without materializing the whole
UTF-16 sequence, primitive receivers retain their shared immutable storage,
and full UTF-16 conversion now preallocates once rather than allocating a
temporary vector per scalar. `at`, `charAt`, `charCodeAt`, and `codePointAt`
all use the same mechanism, with explicit ASCII, supplementary-plane, and lone
surrogate tests. No workload name, source path, iteration count, or checksum is
available to the implementation.

Three fresh-process local runs against the retained pre-change binary reduced
SunSpider `crypto-md5` median wall time from approximately 6.39 s to 0.18 s
(0.0282x base, about 35.5x faster) and `crypto-sha1` from 2.66 s to 0.12 s
(0.0451x base, about 22.2x faster). The unrelated `access-nbody` and
`bitops-nsieve-bits` ports remained near their preceding 3.3 s and 3.0 s,
respectively, which is consistent with the profiled string mechanism rather
than a suite-wide shortcut. A five-block internal `string_slice` run retained
all 15 eligible measurements and exact per-iteration checksums; medians were
56.927 ns/op candidate, 59.935 ns/op base, and 90.016 ns/op QuickJS-NG
(0.9498x base and 0.6324x QuickJS-NG). These are focused local diagnostics;
the next hosted broad and external artifacts decide whether the unit counts as
campaign progress.

Hosted run `29552887188` at `239bc64f` confirms that the thirty-first unit
generalizes, but does not satisfy the external completion gate. The internal
candidate/QuickJS-NG aggregate improved from 0.366860x to 0.351871x while
candidate/base remained neutral at 0.997721x. Across the 22 SunSpider cases
comparable in both runs, the diagnostic ratio fell from 33.6948x to 25.8658x;
`crypto-md5` fell from 376.086x to 19.540x and `crypto-sha1` from 155.150x to
14.299x. Kraken's two-case ratio fell from 2.8441x to 2.4543x. JetStream is not
admissible as progress: its comparable coverage fell from five cases to four
when `stanford-crypto-aes` timed out, so the displayed 17.907x ratio cannot be
compared to the preceding five-case 23.812x ratio. The internal report SHA-256
is `cf1eb3195c46b3946d7d9b9f81df3ddc4a2df1b57434002750305128a1af9a38`;
the external report SHA-256 is
`7ef6a3e4251880540aa4fe9449a0150ee52f99659b520494675149e35fc4ce78`.

The thirty-second v2 unit follows the next external profile rather than an
internal benchmark shape. On the current `access-nbody` port, a three-second
macOS sample placed 412 top-of-stack samples in `FrameBindings::insert` and
117 in `Vm::current_env`, with allocation, free, string clone, and frame-copy
work dominating the remaining profile. The VM was rebuilding a complete
`CallEnv` and copying live local bindings even when an already-primitive
string, number, BigInt, boolean, null, undefined, or symbol became a property
key. The general fast path converts those values directly because that
conversion cannot execute JavaScript. Function, array, map, set, ordinary
object, and proxy keys retain the existing observable `ToPrimitive` path and
environment behavior. The implementation has no workload identity, source
path, iteration count, or checksum input.

Against retained current-base binary SHA-256
`e355addab61f9fd2e50d9ff3b342d7dff30565a60692b5b3332e518d6ac505e0`,
the candidate binary
`9470c968a2b2f9be972559d3b6f218684269acef8a15606948323389be8e5b09`
reduced five-run `access-nbody` median wall time from 3.36 s to 0.57 s
(0.170x base, about 5.9x faster). Three-run cross-checks reduced
`access-fannkuch` from 3.95 s to 3.34 s, `bitops-nsieve-bits` from 2.93 s to
1.57 s, and the already-optimized `crypto-md5` from 0.17 s to 0.13 s, while
`controlflow-recursive` remained approximately 0.10 s. A five-case,
five-block internal diagnostic retained all 75 measurements and exact
checksums; candidate/base medians ranged from 0.999x to 1.023x. These focused
local results are evidence for the mechanism and regression boundary only;
the next complete hosted internal and external artifacts decide campaign
progress.

Hosted run `29555075595` at `515e2a77` accepted the thirty-second unit without
a correctness or CI failure. Its internal candidate/QuickJS-NG aggregate was
0.352487x and candidate/base was 1.004979x, effectively neutral at preview
resolution. Against the preceding external artifact on identical case sets,
the four common JetStream cases improved from 17.9070x to 17.0478x and the 23
common SunSpider cases improved from 26.3519x to 21.9527x; Kraken's two common
cases remained neutral at 2.4610x versus 2.4543x. Comparable coverage increased
without a loss: JetStream regained `stanford-crypto-aes`, and Kraken gained
three Stanford crypto ports. The internal report SHA-256 is
`3100e77066d3128e7298edc82c0ceb74b85cef80d46976631fb5073e78c1bf30`;
the external report SHA-256 is
`a1f250f4d832c84cdaecc46223051129d2bae1ad82445b31a00ca64ce3ee4af1`.

The thirty-third v2 unit addresses a second external profile rather than an
internal case. SunSpider `bitops-nsieve-bits` spent 723 of 724 macOS samples in
its sieve loop, dominated by `Vm::set_property_value`, `current_env`,
`CallEnv::snapshot_locals`, string cloning, hashing, allocation, and release.
Computed compound assignments and update expressions correctly apply
`ToPropertyKey` once before their read/write pair, so numeric indices reach the
store as canonical strings. The dense array store recognized only uncoerced
numbers. In addition, the intrinsic `new Array(length)` constructor records the
realm's ordinary Array.prototype explicitly, while the fast path recognized
only the implicit prototype slot used by literals. The general fix accepts a
canonical string array index and either representation of the exact realm
Array.prototype. Non-canonical strings, custom prototypes, proxies, inherited
indexed properties, own special descriptors, frozen arrays, and non-extensible
arrays retain ordinary `[[Set]]` behavior. The implementation has no workload
identity, source path, iteration count, or checksum input.

Three fresh-process runs reduced `bitops-nsieve-bits` hot wall time from
approximately 1.68 s to 0.21-0.22 s (about 7.7x faster). The structurally
different `string-validate-input` and `access-nbody` ports also improved from
approximately 0.83 s to 0.77 s and 0.60 s to 0.56 s, respectively. The complete
45-case local external preview retained or increased comparable coverage and
reported 12.719x for all 5 JetStream ports, 5.965x for 8/14 Kraken ports, and
11.357x for 23/26 SunSpider ports. These remain far above the <=1.00x external
completion guard and have zero qjs-rust wins, so the campaign remains active.
The external raw SHA-256 is
`85ee263c6e0b12a19f38093634c588c42e8ca8eb153f8c074af3d0c56f6c63f2`;
the report SHA-256 is
`c3cad7c6a227c6af73c6c071bc61fc0b63a86e76dca37fe2b9aed86bdaf65137`.

A separate candidate/base/QuickJS-NG three-block broad diagnostic used a clean
detached `515e2a77` base binary and completed all 25 cases with exact checksums.
Its raw median diagnostic was 0.9714x candidate/base; the directly affected
internal `array_write` simple-assignment shape was neutral at 1.0083x. Because
the candidate came from a dirty development tree without build receipts, the
strict report correctly rejected the run as unverified; this is regression
diagnostic evidence only, not a campaign score. Raw SHA-256:
`617217da69fe591f6f2c30894680c432cc3aeded44b3f4481f03b626439cc0a3`.
The next hosted receipt-bound artifact decides whether this unit counts as
campaign progress.

The thirty-fourth v2 unit follows the new external guard rather than an
internal case. A macOS sample of SunSpider `string-validate-input`, the worst
case in hosted run `29558463474`, showed that regular-expression matching was
not the dominant cost. Caller-frame compatibility synchronization,
`CallEnv::snapshot_locals`, string-key hashing/insertion, allocation, and
string copying dominated. The general fix bypasses caller-local environment
materialization for primitive-only calls to `Math.floor`, `Math.random`,
`String.prototype.charAt`, `String.prototype.concat`, and
`RegExp.prototype.test`. It also evaluates primitive string concatenation,
equality, and UTF-16 relational comparison without a `CallEnv` round trip.
Object/Symbol operands, observable coercions, nonstandard receivers, and every
unsupported native shape retain the generic path. The implementation has no
workload identity, source path, iteration count, or checksum input.

Against clean detached `a21bcb27` binary SHA-256
`4f66028113a17d498d5fdab262c0780c9399063c25fa2362ddb39c2b4ed4c63d`,
the clean candidate binary
`ed22991efcb815a932eb4611621b3bc2c8b2ff6667d64dd79ea15494d3220ef1`
reduced seven-run interleaved median wall time for
`string-validate-input` from 0.776 s to 0.238 s (0.307x base, about 3.26x
faster). Structurally different cases also improved: `date-format-tofte` to
0.516x, `date-format-xparb` to 0.680x, and `string-base64` to 0.829x.
`access-nbody`, `bitops-nsieve-bits`, and `crypto-md5` remained within 1.0%
of base in the clean diagnostic.

The complete 45-case, three-block local external preview preserved coverage
at 5/5 JetStream, 8/14 Kraken, and 23/26 SunSpider cases. JetStream improved
from 12.719x to 11.806x and SunSpider from 11.357x to 10.360x; Kraken was
neutral at 5.984x versus 5.965x. `string-validate-input` itself improved from
36.037x to 11.164x QuickJS-NG locally. The preview still has zero qjs-rust
wins and remains far above the <=1.00x completion guard, so the campaign stays
active. External raw SHA-256:
`d2d8b5442d34729a85261cff5e7a25b09997070f555c50918054188db4d51792`;
report SHA-256:
`0bce808d2418b81a19cef77b670fb2b2c413ea749da65691a7a17221dba4de27`.

A separate clean-binary three-role broad diagnostic completed all 25 cases
with exact checksums. Raw medians were 0.9548x candidate/base and 0.1961x
candidate/QuickJS-NG. No otherwise-neutral critical family moved by more than
1.5%; the call and allocation families improved. The strict report correctly
rejected the dirty development runner and receipt-less binaries as unverified,
so these values are regression diagnostics only. Raw SHA-256:
`4b98b738c107ffe5994bf84b0c8fb2e80de2ab515f4f4d573ee984ec7c650462`.
The next hosted receipt-bound artifact decides campaign progress.

Hosted run `29562514302` accepted the thirty-fourth unit at commit
`7fe4f34a`. CI, full Test262 coverage, and the performance workflow all
completed successfully. The receipt-bound broad artifact was neutral against
its exact base at 0.9998x candidate/base and measured 0.3721x
candidate/QuickJS-NG overall; allocation remained the only failing family at
2.3652x. The external artifact preserved 5/5 JetStream, 5/14 Kraken, and 23/26
SunSpider comparable cases. `string-validate-input` fell from 2.128 s to
0.836 s across the two hosted artifacts, but external suite ratios remained
15.635x, 8.530x, and 17.336x respectively. This was genuine generalization,
not completion of B4/B5.

The thirty-fifth v2 unit follows the next dominant external profile.
SunSpider `access-fannkuch` spent most sampled time in `current_env`,
`apply_env`, `CallEnv::snapshot_locals`, string-key hashing, and frame-map
allocation. The hot source operation was logical-not over already-computed
Boolean results. ECMAScript logical-not only applies `ToBoolean`, and `void`
only discards an already-evaluated value; neither invokes `valueOf`,
`toString`, `Symbol.toPrimitive`, accessors, or any other user code. The
general fix therefore evaluates both operators directly on the VM `Value`
without materializing or writing back a caller `CallEnv`. All coercing unary
operators retain their existing generic path, and the implementation has no
workload identity, source path, iteration count, or checksum input.

Against clean detached `7fe4f34a` binary SHA-256
`66862542b73d4491ea2cb6241f54c4b331442a61d8e69c729ac502f8c2cd264a`,
candidate binary SHA-256
`fd98f344ce99249d720406af18236e8ec67c66945fde3701f48a73409e05bb8f`
reduced seven-run interleaved `access-fannkuch` median wall time from 3.251 s
to 0.146 s (0.0448x base, 95.5% lower). The complete local external preview
preserved coverage at 5/5 JetStream, 8/14 Kraken, and 23/26 SunSpider.
`access-fannkuch` fell from 3.344 s to 0.264 s (0.0789x); structurally distinct
`3d-cube`, JetStream `hash-map`, and JetStream `cdjs` reached 0.613x, 0.752x,
and 0.796x of their preceding candidate wall times. Across 36 common
comparable cases, 14 improved by more than 2%, 21 remained within 2%, and one
moved 4.0% alongside a 9.4% QuickJS-NG timing shift. Suite ratios improved
from 11.806x/5.984x/10.360x to 10.451x/5.849x/8.946x. These are substantial
external gains but still fail the <=1.00x B5 guard. External raw SHA-256:
`8dbc52c781ffd9a06dfebe89c62e0e46adef63df31b0dc82d425d23e71d4a8e0`;
report SHA-256:
`9b809cb3756f4de8120ca79ba74495a1e003ace430bda161287e800cfa4cc275`.

The clean-binary three-role broad diagnostic completed all 25 cases with exact
checksums. Raw medians were 0.9559x candidate/base and 0.1974x
candidate/QuickJS-NG; allocation improved to 1.1726x but remained above its
critical-family guard. The strict report correctly rejected the receipt-less
development binaries as unverified, so these are regression diagnostics only.
Raw SHA-256:
`701c0bcb81998d84b6dec5cb3113c4df9c394e4cd36a67145d5e08becd41b76c`.
The next hosted receipt-bound artifact decides campaign progress.

Hosted run `29566653789` accepted the thirty-fifth unit at commit
`69541277`. CI run `29566653844` and full Test262 coverage run `29566855091`
also completed successfully. The receipt-bound broad artifact measured
0.984530x candidate/base and 0.362880x candidate/QuickJS-NG overall;
allocation remained the only failing broad family at 2.663908x. The external
artifact preserved 5/5 JetStream, 6/14 Kraken, and 23/26 SunSpider comparable
cases, with diagnostic ratios of 14.136785x, 8.704828x, and 15.858843x. The
unary change was therefore accepted as a general improvement, but all three
external suites still fail the <=1.00x B5 guard by a wide margin.

The thirty-sixth v2 unit targets ordinary construction and property creation,
not a benchmark identity. A profile of the pinned SunSpider
`access-binary-trees` port showed ordinary constructors repeatedly cloning and
writing back frame maps for parameters, `this`, and simple `this.x = value`
stores. Ordinary constructors now use the same slot-backed parameter and
receiver seed as semantically eligible ordinary calls; `new.target` stays in
the compatibility frame. A second general fast path creates a missing ordinary
own string data property only after proving that the receiver is extensible and
that its all-object prototype chain contains no accessor, read-only descriptor,
Proxy, typed array, module namespace, symbol primitive, or function prototype.
Every observable or exotic case remains on the full `OrdinarySet` path. The
implementation has no workload ID, source path, iteration count, or checksum
input.

Against clean base binary SHA-256
`2d45d24dddb5cd09afc6c72d5504a1c59de45204d9ce70aa0d5797edaac492e2`,
candidate binary SHA-256
`0fd0b797227d3e82086d49a81817d6178990b174a1f70f44ad21a736f42cec41`
reduced an eleven-block interleaved `access-binary-trees` warm median from
0.262770 s to 0.112107 s (0.426635x). A structurally independent 200,000-
property Point-construction diagnostic reduced its warm median from 0.505860 s
to 0.311988 s (0.616748x), confirming that the mechanism is not specific to
binary trees.

The complete local three-role broad diagnostic preserved all 25 exact-checksum
cases at 0.994993x candidate/base. Allocation moved 1.019465x and the worst
single case, `object_allocation`, moved 1.030024x; neither is a material
regression. The local candidate/QuickJS-NG diagnostic was 0.195824x overall,
with allocation still failing at 1.185220x. The strict report correctly rejects
these dirty, receipt-less development binaries, so the numbers are regression
diagnostics only. Raw SHA-256:
`1ff9cc3fef45df30875ccc402d90072d0629c959cbecb099bce66029cb9d2aae`.

Independent one-block full external A/B previews preserved identical coverage
at 5/5 JetStream, 8/14 Kraken, and 23/26 SunSpider. Across all 36 common
comparable cases, candidate/base was 0.966328x: seven cases improved by more
than 2%, 26 stayed within 2%, three moved between 2.19% and 2.43%, and no case
regressed by 25%. `access-binary-trees` reached 0.425861x; JetStream `cdjs`,
`hash-map`, and `stanford-crypto-aes` reached 0.806694x, 0.879076x, and
0.947559x. Candidate/QuickJS-NG suite diagnostics improved from
10.483653x/5.835611x/8.945211x to 9.685067x/5.827903x/8.667864x. Candidate raw
and report SHA-256 are
`eff0b411f9ef836a66722da8cf611115a5d802bbc25b44f7587f34940bdb8552`
and `7fef9228a7748ecea23fab85098d6c4993a326afd75a184b896c31809acf5d5d`;
base raw and report SHA-256 are
`10a588da704da9806f1c42fc9dda93a587092767021e8b9db917902f1df2e8c8`
and `8c9b2f0ab3c8f98ec6009f8a01ba6a93e4c2238220fa00ddff3ce9f5e9046264`.
These gains satisfy the no-overfitting regression boundary for this unit, but
the external <=1.00x target and broad allocation <=1.00x target remain open.

The thirty-seventh v2 unit follows a hotspot shared by the external
`access-binary-trees` port and the broad call/allocation profiles. Direct leaf
frames previously allocated a `Vec<Option<Upvalue>>` with one `None` entry per
local on every call even when the compiled function had no received capture,
sloppy-global route, or module import. Bytecode now caches whether either
local route exists, `CallEnv` exposes whether module-import routes exist, and a
cell-free direct frame represents the all-`None` upvalue table with an empty
vector. Authoritative-slot setup treats a missing entry as `None`. The existing
direct-call admission contract excludes closures, direct eval, and `with`, so
no operation can create a cell later on this path. Captured and module-import
unit tests prove that their identity-bearing cells still use full storage. The
implementation has no workload ID, source path, iteration count, or checksum
input.

Against clean base binary SHA-256
`0fd0b797227d3e82086d49a81817d6178990b174a1f70f44ad21a736f42cec41`,
candidate binary SHA-256
`af59c4e55c36f2dd739b2698b285f6bda059a32fa7bb00383c090632251f1957`
reduced an eleven-block interleaved external-shaped binary-tree median from
8.653387 s to 8.391495 s (0.969735x). The complete local three-role broad
diagnostic preserved all 25 cases and 225 eligible exact-checksum samples at
0.998932x candidate/base. Binding was 0.987920x, call 0.997499x, and allocation
1.013692x; the latter remains below the unit's 2% material family threshold.
The raw SHA-256 is
`fd022895f0a7bc9197bc352274bea0cc0d9a28d352bbed11ea903b7f4cdb8ec8`.
The strict analyzer rejected the intentionally receipt-less local binaries, so
these remain regression diagnostics rather than a performance claim.

Independent one-block full external A/B previews preserved identical coverage
at 5/5 JetStream, 8/14 Kraken, and 23/26 SunSpider. Across all 36 common
comparable cases, candidate/base was 0.998782x; five cases improved by more
than 2%, 27 stayed within 2%, four moved between 2.05% and 5.12%, and none
regressed by 25%. Suite candidate/base ratios were 0.994606x, 1.000293x, and
0.999167x. Candidate/QuickJS-NG diagnostics remain far from completion at
9.599757x, 5.766279x, and 8.740135x. An eleven-block interleaved rerun of the
external source confirmed `access-binary-trees` at 0.969378x; three apparent
regressions resolved to 0.988744x (`math-cordic`), 0.994895x (`crypto-md5`),
and 1.000494x (`string-fasta`), while `date-format-tofte` retained a small
1.021284x movement. Candidate raw/report SHA-256 are
`ddda93bdf1a6953061728ab3112cc2215867aa8d4c39e5858d098d5563c307e9`
and `864339bfe2f648024a63be730dce401a2a2812709a28f1be41ea24f63ffb8c9a`;
base raw/report SHA-256 are
`5f38ae170e09a785fefc6fa053712e4c5da64c7b13f28d3a0eac096fa1d53893`
and `c3c049ecbccff7e082320ad9c88d38506e30edfa7c7dba51c807fbfec7cf0335`.
These local results satisfy the unit's no-overfitting regression boundary but
do not satisfy B4 or B5; the exact-SHA hosted broad and external artifacts
remain authoritative.

The thirty-eighth v2 unit removes two private-name lookup costs from general
runtime function creation. A fresh five-second profile of
`closure_allocation_call` placed 197 samples below the realm lookup for
`__quickjsRustDynamicFunctionRealm` and another 203 below the private
environment lookup's miss on the call-frame-only `\0home_object` binding.
`RealmState` now caches the optional dynamic-Function global object. Initial
realm construction, incremental insert/remove/entry operations, and the few
bulk script/module initialization paths keep that cache synchronized. Ordinary
realm writes compare the mutated name and do not rehash the private key.
Call-frame-only home-object reads now use the frame/deopt lookup directly and
never fall through to the realm map. The implementation contains no workload
ID, source path, iteration count, or checksum input, and applies to every
runtime-created function.

Against clean base binary SHA-256
`af59c4e55c36f2dd739b2698b285f6bda059a32fa7bb00383c090632251f1957`,
candidate binary SHA-256
`02e66c1b0c80f7815335ac04a5e632c3cfa6985200307aad2080f65bf6301428`
reduced the five-block `closure_allocation_call` median to 0.923390x base and
0.961210x QuickJS-NG. `object_allocation` was 0.986678x base and
`array_allocation` 1.008357x, producing a 0.972131x allocation-family
candidate/base diagnostic. Focused raw SHA-256:
`511ba3f08e8c45a389918bc5956970ddb1128fbb15013cf0f38652735414ff80`.

The complete local three-role broad diagnostic preserved all 25 cases, 225
eligible exact-checksum measurements, and 600 passing linearity samples.
Candidate/base was 0.996436x overall; allocation was 0.972323x, while every
other family stayed between 0.986150x and 1.002579x. The worst individual
movement was `array_write` at 1.008887x. The local candidate/QuickJS-NG
diagnostic was 0.194802x overall, but allocation still failed B4 at 1.142737x.
The strict analyzer rejected the intentionally receipt-less development
binaries, so these numbers remain regression diagnostics. Broad raw SHA-256:
`9f92052043d6ace2cce8f2858261b4a4f813876ad35636cfd38c2c11597be23b`.

Independent one-block full external A/B previews retained identical coverage
at 5/5 JetStream, 8/14 Kraken, and 23/26 SunSpider. Across all 36 common cases,
candidate/base was 0.992922x; suite ratios were 0.992089x, 0.990770x, and
0.993853x. Six cases improved by more than 2%, 26 remained within 2%, four
moved between 2.01% and 2.68%, and none regressed by 25%. Candidate/QuickJS-NG
suite diagnostics remained far from B5 at 9.636794x, 5.723410x, and 8.532851x.
Candidate raw/report SHA-256 are
`4afd83044dad2a11e5c20f925686e48246b377c7064ae823fc85f258c0b485f9`
and `576a367f43f8e3871280eb2fb5d4c23f9527bc378ba77e5507775240c097e189`;
base raw/report SHA-256 are
`c09ec42d2851039d3a567aa6e3ff8fde932b857b7e05047a808e7c5cebc07fa3`
and `5cf68587eacbee05051a456df307b0f1adae7368b4576a18f1710948cca53ed1`.
This unit is a general allocation improvement, but B4 and B5 remain active and
the exact-SHA hosted artifacts remain authoritative.

The thirty-ninth v2 unit is selected entirely from external profiles rather
than an internal broad case. Fresh samples of the pinned SunSpider `3d-morph`
and `access-nbody` ports shared the same general native-call hotspot: detached
`Math.sin` and direct `Math.sqrt` calls built the caller's complete environment,
then snapshotted and applied it after a pure numeric operation. The native-call
fast path now handles the common unary Math operations when the first argument
is already a number or is absent. Strings, objects, and every observable
coercion still take the general path; focused tests cover detached calls,
ignored extra arguments, negative zero, missing arguments, string conversion,
and an object `valueOf` side effect. The implementation has no benchmark name,
source path, iteration count, or checksum input and applies to ordinary code in
every workload.

Against clean base binary SHA-256
`02e66c1b0c80f7815335ac04a5e632c3cfa6985200307aad2080f65bf6301428`,
candidate binary SHA-256
`f34503c0863f42860031e786119f74222106bc3d532b579ff51881ded1a526e9`
reduced nine-block interleaved source medians to approximately 0.200x for
`3d-morph` and 0.297x for `access-nbody`; unrelated `math-cordic` remained near
neutral. The complete local three-role broad diagnostic retained all 25 cases,
225 eligible exact-checksum measurements, and 600 passing linearity samples.
Candidate/base was 0.997809x overall; family ratios ranged from 0.973467x for
string to 1.004860x for array. Candidate/QuickJS-NG was 0.194276x overall, but
allocation still failed B4 at 1.126177x. The strict analyzer correctly rejected
the receipt-less local binaries, so these remain regression diagnostics. Broad
raw SHA-256:
`fa844fa9574afeb18a234a906b908dbfe9649f97bd50a4c5f04b41a8bfe1c6a6`.

Independent one-block full external A/B previews preserved 5/5 JetStream and
23/26 SunSpider coverage, while Kraken coverage improved from 8/14 to 10/14:
`audio-beat-detection` and `audio-fft` changed from timeout to comparable.
Across common cases the suite candidate/base geometric means were 0.976217x,
0.937224x, and 0.862132x. SunSpider `3d-morph`, `access-nbody`, and
`math-partial-sums` measured 0.187441x, 0.285967x, and 0.700497x respectively.
Candidate/QuickJS-NG suite diagnostics were 9.537662x, 5.704433x, and 7.339977x,
so B5 remains far from complete. Eleven-block interleaved reruns reduced the
two apparent unrelated regressions to approximately 1.002x for
`string-validate-input` and 0.992x for `date-format-xparb`. Candidate raw/report
SHA-256 are
`5b4c3c5ceb637af4d0bb45045c1fe3c6a0b0cd1681bf9815081e6906fd9d4305`
and `9721bd8efedad67801b941add6a3def375871e22d268b5fbdbb7e9c3eb14d247`;
base raw/report SHA-256 are
`db17d2b062606fd9cfed22b3e93ab5d69a1970a77ad017179cec6ac80770e53f`
and `9966d978fc232aa2719937e5c851a3ae2b9920c446b9cc08aaea9ddcf30ec4c8`.
This is external generalization progress, not completion: the external suites
still lose every comparable case to QuickJS-NG and both B4 and B5 remain active.

The exact-SHA hosted evidence for `b3cb72b416891ae7aeae6054ab28eacee9dc4e9e`
kept the distinction between the internal guard and general engine speed
explicit. The 25-case broad preview was 0.9984x base and 0.3529x QuickJS-NG,
but the external preview remained 12.523x QuickJS-NG for 5/5 JetStream cases,
7.640x for 5/14 Kraken cases, and 13.056x for 23/26 SunSpider cases. Thus the
internal number is not completion evidence. CI, the Test262 coverage scan, and
the informational performance workflow all completed successfully; Test262
remained 42,671 pass, one fail, and zero timeout. Hosted broad raw/report
SHA-256 are
`7568681b9a15b3baa6cfe458cf9cab3c236facac667927d23cdada5d92cf19d8`
and `96c3a7a75d11ea65bfc6e197977f20b7f91b76ed636b93cd5c14333dfcece9e8`;
hosted external raw/report SHA-256 are
`2872b5c0b6c97ad9bb82784932d17599dc9d003c8dab271a3354c79af0e421e0`
and `acbc87b66b4559efdb3c13e6d7d7bcf14078479e69d34b16e084e5e76fce2bb7`.

The fortieth v2 unit continues from the external profile rather than adding an
internal benchmark case. After the unary Math fast path, the pinned SunSpider
`math-partial-sums` profile still spent substantial time constructing,
snapshotting, and applying caller environments for primitive `Math.pow`
calls. The native-call fast path now evaluates `Math.pow` directly only when
both consumed arguments are numbers or absent/undefined, reusing the engine's
shared Number exponentiation semantics. Strings, objects, BigInts, and every
observable conversion still take the complete native path. Focused tests cover
detached calls, ignored extra arguments, missing arguments, NaN and infinity,
negative zero, string conversion, object conversion order, and side effects.
The implementation contains no benchmark name, source path, iteration count,
or checksum input.

Against clean base binary SHA-256
`f34503c0863f42860031e786119f74222106bc3d532b579ff51881ded1a526e9`,
candidate binary SHA-256
`40c55d677023f3829b964df0e75db88f9651d1b2dc064f5a3d7c49ddefaea478`
reduced the nine-block interleaved `math-partial-sums` source median from
approximately 0.39 seconds to 0.23 seconds, or about 0.59x base. A complete
one-block external A/B retained identical coverage at 5/5 JetStream, 10/14
Kraken, and 23/26 SunSpider. Common-case suite candidate/base geometric means
were 0.997046x, 0.978466x, and 0.973802x; `math-partial-sums` was 0.623923x and
Kraken `imaging-darkroom`, another `Math.pow` user, was 0.836867x. The suite
candidate/QuickJS-NG diagnostics remained far from B5 at 9.591883x, 5.579897x,
and 7.254125x. Eleven-block interleaved source reruns rejected the three
apparent unrelated regressions: `string-base64` was 0.994426x,
`access-nbody` 0.999028x, and `3d-morph` 1.004505x base. External raw/report
SHA-256 are
`e66c81e96cc2d6f9acc7edebce6d4053838f8314a505c6ca75eed3367a40c180`
and `1a38a0f85ee76cbf9ff9daa44759c36dce77caf5b3381e38d58d8bd83b3aa6cf`.

The full local broad diagnostic had no failed sample, but the unchanged base
binary's `captured_read` measurements were all timer-limited, so only 24/25
cases were formally common and this run is not complete claim evidence. The
24-case candidate/base diagnostic was 1.004864x overall. An independent
seven-block rerun reduced every apparent unrelated movement below 2%:
`string_slice` 1.000394x, `object_allocation` 1.001747x,
`function_call_reordered` 1.002308x, `top_level_function_call` 0.999709x, and
`array_allocation` 1.011237x. `captured_read` raw medians were 5.431 versus
5.408 ns/op, although the base remained timer-limited. The complete
candidate/QuickJS-NG side was 0.196533x overall, while allocation still failed
B4 at 1.181221x. Full and focused-rerun broad raw SHA-256 are
`81b0c5fc575eda3f1d54382d0531862e9ecf2f9a945c1b177ff044d5acfd20e3`
and `0cbf9018be8b65466ccbff2b4903111ec5bcdae4ba4476412ee743d2ef2c52f0`.
This is a general native-dispatch improvement with external evidence, not a
benchmark-specific shortcut; B4 and B5 remain active.

The exact-SHA hosted workflows for
`07970f8d394c33323f36a8d864efb9f1c0a4cf68` all completed successfully. The
hosted broad preview retained 25/25 cases and measured 1.0062x base and 0.3583x
QuickJS-NG overall. Its isolated `array_allocation` 1.1462x base movement was
not reproduced by the local independent rerun recorded above. The hosted
external preview remained far from B5 at 12.437x QuickJS-NG for 5/5 JetStream,
7.613x for 7/14 Kraken, and 12.803x for 23/26 SunSpider; `math-partial-sums`
improved from the preceding hosted artifact's 25.290x QuickJS-NG to 15.271x,
but still lost decisively. Exact Test262 coverage remained 42,671 pass, one
fail, and zero timeout. Hosted broad raw/report SHA-256 are
`65d14956ed6a3f458f9c79c545398ec41800982bcd7358250998959bff558c47`
and `61bdc2ddb8d9beaf82af6ba945f153c660e389a0e682c29b4e5d670860ea4850`;
hosted external raw/report SHA-256 are
`0df30133b3ae726c0298b456e8b2bdd49ff2af1e6e9c0bdf68eb8c9d947cd4da`
and `b874194fadee20681ce9b71f835e7bce216a8020e468759c71c7811d24c3f98f`.

The forty-first v2 unit follows the external SunSpider `string-base64`
profile. Its encoding and decoding loops repeatedly call
`String.prototype.charCodeAt` with an already primitive string receiver and
numeric index, but the general native-call path still constructed and applied
the caller environment for each code-unit read. The fast native dispatcher
now reads the code unit directly only for primitive string receivers and
number/undefined indices. Object receivers, object or string indices, and all
observable conversions remain on the complete path. Tests cover numeric
truncation, NaN, infinity, ignored extra arguments, UTF-16 code units, object
index conversion, and conversion side effects. The implementation has no
benchmark name, path, iteration count, checksum, or workload-specific table.

Against clean base binary SHA-256
`40c55d677023f3829b964df0e75db88f9651d1b2dc064f5a3d7c49ddefaea478`,
candidate binary SHA-256
`4d6b4e3bf0626695baf98cbd69ebe8b4700045c63aa2f3a332558aea51186a67`
improved eleven-block interleaved external source medians across four distinct
workloads: `string-base64` was 0.495773x base, `crypto-md5` 0.782206x,
`crypto-sha1` 0.813148x, and `crypto-aes` 0.889119x. Five-block interleaved
Kraken bundles also measured `audio-dft` at 0.970906x and
`stanford-crypto-ccm` at 0.974229x, rejecting apparent regressions in the
unpaired full previews.

Complete one-block candidate and same-thermal-window base rerun previews kept
identical coverage at 5/5 JetStream, 10/14 Kraken, and 23/26 SunSpider. Their
common-case suite candidate/base diagnostics were 0.982537x, 0.991921x, and
0.985346x. The intended SunSpider cases reproduced at 0.495484x for
`string-base64`, 0.779431x for `crypto-md5`, 0.811919x for `crypto-sha1`, and
0.908398x for `crypto-aes`. Eleven-block interleaved reruns of the eight worst
unrelated SunSpider movements placed every one between 0.993864x and 1.016738x
base. Candidate/QuickJS-NG suite diagnostics improved but still failed B5 at
9.459241x, 5.548499x, and 6.903639x. Candidate external raw/report SHA-256 are
`a46d34497bda45152c37fe0f1a6cf97ddc0d094cf4e4aab139fca4e8d36212e6`
and `64c5d91ed608fbcc7fd8acc2e6318b074c62bb4c8c80cc29be7ec5df5ae4d240`;
base-rerun raw/report SHA-256 are
`a767097e73ad5dae244ee46c2cb01a3787bf386753d45b874d45a0e2dbec631d`
and `d18feb94f8162c674fa71b6ebca4886887b8ddf435bd8f2ef5c497db9a73105f`.

The complete local three-role broad diagnostic retained all 25 cases, 225
eligible exact-checksum measurements, and 600 passing linearity samples.
Candidate/base was 0.997764x overall; family ratios ranged from 0.988520x for
string to 1.002026x for property, and the worst individual case was
`array_allocation` at 1.009524x. Candidate/QuickJS-NG was 0.194364x overall,
but allocation still failed B4 at 1.135321x. The strict analyzer correctly
rejects the receipt-less development binaries, so this remains regression
evidence rather than a fixed-hardware claim. Broad raw SHA-256:
`2f657b75ebab15ad3e10ee286c8eb80e855c2862ac30ca5ca9ba4d0f30f46ed3`.
This unit generalizes across external string and crypto workloads, while B4
and B5 remain active.

The exact-SHA hosted workflows for
`3bf3229a87043bab78cbb775cbabaec4f2bc5298` all completed successfully. Hosted
broad retained 25/25 cases and measured 0.9955x base and 0.3531x QuickJS-NG
overall. Exact Test262 coverage remained 42,671 pass, one fail, and zero
timeout. Hosted external evidence confirmed the `charCodeAt` improvement:
`string-base64` fell from roughly 570 ms in the preceding artifact to 293 ms.
The suite diagnostics nevertheless still failed B5 at 12.485x QuickJS-NG for
5/5 JetStream, 7.317x for 6/14 Kraken, and 12.016x for 23/26 SunSpider. The
Kraken coverage variation came from timeout boundaries and is not accepted as
performance progress. Hosted broad raw/report SHA-256 are
`3637b5c5291435eb00c2bbd42f470d1cb8788e69631af0bb81aed481c5e8ea67`
and `3d81caf1949e5e5804a010f2a1a3dfc2564bcb27bf4bd2632f0a00ff3f0fd4f8`;
hosted external raw/report SHA-256 are
`97c4946e44132e7ad992c3c9b457ddfe7c07f6e3ee0be7d4041c24f74f51f7a4`
and `e516e6e262294dc09cc40d7c6f5635182232769cf2c3e3cf32bb67b9a06cf5d5`.

The forty-second v2 unit follows an external profile into the general sloppy
global write path. Repeated assignments to an already-created writable data
property previously cloned property descriptors and owned string keys, then
performed redundant global-object, realm-map, and captured-cell updates. The
VM now uses the existing borrowed-key data-property write operation and a
borrowed-key realm replacement after the first assignment establishes the
binding. Missing properties, accessors, read-only descriptors, module imports,
immutable bindings, deletion/recreation, and dynamic scope retain their full
paths. Tests cover repeated writes, global-object visibility, read-only
redefinition, and delete/recreate behavior. No workload name, source path,
iteration count, checksum, or expected value appears in the implementation.

Against base binary SHA-256
`4d6b4e3bf0626695baf98cbd69ebe8b4700045c63aa2f3a332558aea51186a67`,
candidate binary SHA-256
`8b566b3cb05d6570273b20248654403ba92af6288ecf487bd127c437b631a70b`
improved two independent external programs in eleven-block interleaved runs.
SunSpider `bitops-bitwise-and`, whose loop repeatedly updates one sloppy
global, measured 0.681459x base. `math-partial-sums` independently exercises
the same mechanism because its chained local initialization creates eight
sloppy globals that the inner numeric loop updates; it measured 0.450487x
base. Four unrelated SunSpider cases selected from the worst full-preview
movements reran between 0.989407x and 1.027093x base.

Complete one-block candidate and same-thermal-window base external previews
kept identical coverage at 5/5 JetStream, 10/14 Kraken, and 23/26 SunSpider.
Their common-case candidate/base suite diagnostics were 0.978390x, 0.950001x,
and 0.918161x. Candidate/QuickJS-NG still failed B5 at 9.523116x, 5.544422x,
and 6.543434x. Candidate external raw/report SHA-256 are
`636611c03d111cc6f9faa33cda7c12278a77cc5a8973b7c1b8141bd9fd214fbf`
and `79aa22a3612c17215ee1ea8ff1ade62b91f7ad72716467f51f9298282a717875`;
base raw/report SHA-256 are
`5a8693f1f7591acfd9d848fca473b0659fc567392609a9544882a8d23dfce7be`
and `163a3ec02518b110733375104ce6bf89c92b56c5d3ca464fe8607141ac23b0ee`.

The complete three-role broad diagnostic retained all 25 cases, 225 eligible
exact-checksum measurements, 600 passing linearity samples, and zero non-OK
records. Candidate/base was 1.003683x overall and candidate/QuickJS-NG was
0.194571x. Family candidate/base ratios ranged from 0.997989x for binding to
1.030763x for string. Eleven-block fixed-iteration reruns reduced the apparent
`string_slice` movement to 1.012958x; the six worst full-run cases ranged from
0.990570x to 1.021262x. Allocation still failed B4 at 1.145498x QuickJS-NG.
The strict analyzer correctly rejected the dirty, receipt-less development
binaries, so this is regression evidence rather than a fixed-hardware claim.
Broad raw SHA-256:
`3642bd71ef72428f9021db6f7569cee72da9f8376a1c3727da7b40a3af2f5969`.
This general global-binding optimization materially improves two unrelated
external sources while preserving the internal regression guard; B4 and B5
remain active.

The exact-SHA hosted workflows for
`a8726fb030e34ffd72d38240a8c6d5b017cbfe41` all completed successfully. CI
passed the comparison, Test262 subset, and full check jobs. Exact Test262
coverage remained 42,671 pass, one fail, zero timeout, and one actionable gap.
The hosted broad preview retained 25/25 cases, 3/3 valid blocks, and passing
linearity, but its 1.0179x candidate/base result was classified inconclusive:
several unrelated cases moved together on the variable hosted runner. The
candidate/QuickJS-NG diagnostic was 0.3574x. More importantly for the campaign
boundary, the hosted external preview still failed B5 by a wide margin at
12.608x QuickJS-NG for 5/5 JetStream cases, 7.736x for 7/14 Kraken cases, and
11.684x for 23/26 SunSpider cases. Hosted broad raw/report SHA-256 are
`e40fbd919b692b2ba31f0b8c08588a65a3258646e19535f9e58550c8c2039514`
and `f4b208db24a1b567506e9a7571973b4a6f1308966276128205a538fe4ce4f113`;
hosted external raw/report SHA-256 are
`68081a1cf2f8a0b841241d5740db57b3059fa2b54697f21d9f9d87584e631e8b`
and `94414e047fd7fb53be81b35dd98949ef3ee348957e8396f1e936f308fbccc284`.

The forty-third v2 unit follows the external string profile into primitive
named-property reads. Looking up a data method on a string, number, boolean,
bigint, or symbol previously materialized every live frame binding into a
`CallEnv`, even though primitive `[[Get]]` resolves the realm's intrinsic
prototype and frame bindings named `String`, `Number`, and so on are
irrelevant. The direct path now uses an empty-frame realm view. Accessor
descriptors still take the observable slow path. A cached dynamic-realm marker
keeps indirect eval and dynamically constructed functions on the complete
frame view; this guard was added after the full runtime suite caught the
cross-realm case during development. Tests cover all five primitive families,
frame-name shadowing, a prototype getter side effect, and the pre-existing
marked-realm behavior. The implementation contains no workload name, source
path, iteration count, checksum, or expected result.

Against base binary SHA-256
`8b566b3cb05d6570273b20248654403ba92af6288ecf487bd127c437b631a70b`,
candidate binary SHA-256
`2a7d910a70158e6c92f22674447bbbe56d515b6e4918d11ecce47b22cb609364`
improved two independent external string programs in eleven-block interleaved
runs: `string-validate-input` measured 0.778007x base and `string-base64`
0.637258x. `crypto-md5` and `crypto-sha1` were neutral at 1.000493x and
0.999578x, so they are not counted as wins. Complete corrected-candidate and
same-window base external previews retained identical coverage at 5/5
JetStream, 10/14 Kraken, and 23/26 SunSpider. Common-case candidate/base suite
geometric means were 0.981036x, 0.971765x, and 0.949689x; candidate/QuickJS-NG
remained far from B5 at 9.456082x, 5.507269x, and 6.260206x. Candidate external
raw/report
SHA-256 are
`739ed4b89f9c96218e71fe4b6c14b68fe107bb00d7b4ec517dc6411e3afa9ed5`
and `7fcbc241d7416d2aaae25b4ca559a77238590bc792ff15d43e78dc08a6735b09`;
base raw/report SHA-256 are
`f39138977bab8708d557d150d02ddf9a030dd1da92a935a3cb64da9151396483`
and `0c622af88447819a17350cef152a2ee89288db23fa4d6ca29a6f9bb53b11d654`.

The corrected candidate's complete broad physical plan produced all 1,625
records, 225/225 eligible formal measurements, zero non-OK samples, and 600
passing linearity samples. Candidate/base was 1.001860x overall; family ratios
ranged from 0.973575x for string through 1.006763x for call. The intended
`string_slice` case measured 0.973575x base. Candidate/QuickJS-NG was 0.194900x
overall, but allocation still failed B4 at 1.135609x. The strict analyzer
correctly rejected the receipt-less dirty development binaries, so these are
regression diagnostics rather than a fixed-hardware claim. Broad raw SHA-256
is `c60e47562d42e47cc83c8c6e60bd07aa71be1406e7b603ebbc36c7236c582342`.
This unit generalizes across independent external string programs while
keeping the broad regression guard neutral; B4 and B5 remain active.

The exact-SHA hosted workflows for
`8265d8f11ef79da254dfbc387d4ac84ae60378c4` all completed successfully. CI
passed the comparison, Test262 subset, and full check jobs; exact Test262
coverage remained 42,671 pass, one fail, zero timeout, and one actionable gap.
The hosted broad preview retained 25/25 cases, 225/225 eligible measurements,
and 600 passing linearity samples. Candidate/base was 0.994670x overall and
candidate/QuickJS-NG was 0.359130x. The hosted external preview still failed
B5 at 12.088980x QuickJS-NG for 4/5 JetStream cases, 7.218816x for 6/14
Kraken cases, and 11.047923x for 23/26 SunSpider cases. Hosted broad raw/report
SHA-256 are
`c68bbd588a10436ea952fa32c10af134e2096a44b55355c9e43341a93cebf4e5`
and `df4f0f419573dc1c98b19ce4cd8a513c8cb510e56f85941dfdee855573f76182`;
hosted external raw/report SHA-256 are
`9df5b54ed3f75e3f19d723fbbb8f6f58255922e4065b67bab605e5a114f679d6`
and `774f2bc9f4a126d77f9649530a7d2c9d7e72fbb904060cf1015d4625018c1448`.

The forty-fourth v2 unit follows a fresh external profile into standard array
creation rather than adding a workload-specific fast path. `CreateDataProperty`
with the ordinary writable, enumerable, configurable index descriptor used to
send arrays through generic descriptor storage. Standard operations such as
`slice`, `map`, and `filter` therefore produced arrays whose indices lived in
the cold descriptor map, forcing later ordinary indexed writes through generic
property and environment paths. Compatible default index definitions now stay
in dense storage when the array is mutable, extensible, length-writable, and
within the dense index limit. Accessors, special descriptors, frozen, sealed,
non-extensible, length-locked, sparse-large-index, and descriptor-preserving
overwrites retain the generic path. A lowest-layer test verifies the dense
representation. No workload name, source path, iteration count, checksum, or
expected benchmark value appears in the implementation.

Against base binary SHA-256
`2a7d910a70158e6c92f22674447bbbe56d515b6e4918d11ecce47b22cb609364`,
candidate binary SHA-256
`aebef9ebc2a8805c530888ca412c9a21febf94e55350874ce1fefab1918f7680`
improved independent external programs across three suites. JetStream
`stanford-crypto-aes` measured 0.826802x base; Kraken
`stanford-crypto-sha256-iterative`, `stanford-crypto-ccm`, and
`stanford-crypto-aes` measured 0.295653x, 0.770199x, and 0.827258x. SunSpider
`crypto-sha1` independently measured 0.843897x. Complete one-block candidate
and same-window base external previews retained 5/5 JetStream and 23/26
SunSpider cases; Kraken improved from 10/14 to 11/14 because
`stanford-crypto-pbkdf2` crossed from timeout to complete. Common-case
candidate/base suite geometric means were 0.971058x, 0.847388x, and 0.993435x.
Candidate/QuickJS-NG remained far from B5 at 9.062235x, 4.683388x, and
6.256767x. Candidate external raw/report SHA-256 are
`7aab0cb908bfe40cc4d9557f1d839bbad2b66d2cbf52e3a5f788f9126b09e37b`
and `89a747190833cac84029c61857c2f36895f93c04ef19ba73b1df4c09907bbb49`;
base raw/report SHA-256 are
`6f4ec873aa1e39faf81e2245285b42e15316d01390fdf88d7c3c7085f43e4c1d`
and `e26f74e2b68faf43c38785dc5baa7da7d460d8bd024bea3cb581d0eadb3a2cba`.

The complete three-role broad development diagnostic produced all 1,625
records, 225/225 eligible exact-checksum measurements, zero non-OK samples,
and 600 linearity samples with all 75 engine/case pairs passing. Candidate/base
was 0.998272x overall; family ratios ranged from 0.996083x for binding through
1.000429x for allocation, and the worst case was 1.006811x. Candidate/QuickJS-NG
was 0.194477x overall, but allocation still failed B4 at 1.137729x. The strict
analyzer correctly rejected the receipt-less dirty development binaries, so
this is regression evidence rather than a fixed-hardware claim. Broad raw
SHA-256: `e71b97feb3314937dccf318007df67a4adc38bad11ad24b5f43548015d9f62b7`.
This unit materially improves multiple independent external sources while
keeping the internal regression guard neutral; B4 and B5 remain active.

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
