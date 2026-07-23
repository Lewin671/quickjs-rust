# T018: Broad Performance Campaign

## Goal

Beat the pinned QuickJS-NG reference by at least 2x on every admitted benchmark
case. For broad v2, each of the 25 case ratios, every family ratio, and the
overall geometric-mean wall ns/op ratio must be at most 0.50x. For the external
JetStream, Kraken, and SunSpider neutral-shell portfolios, every pinned case
must become runnable and each candidate/QuickJS-NG ratio must be at most 0.50x;
suite geometric means must therefore also be at most 0.50x. This is explicitly
a **general JavaScript-engine performance**
goal, not permission to optimize only the repository's internal benchmark
shapes. The pinned external JetStream 3 JavaScript subset, Kraken 1.1, and
SunSpider 1.0 neutral shell ports are an independent anti-overfitting boundary:
the campaign cannot complete unless the improvement generalizes there too.
Optimization priorities must therefore come from general engine mechanisms and
independent external profiles; broad-micro is a regression guard, not a target
whose case shapes may dictate runtime design.

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

## General Optimization Acceptance Rule

This campaign accepts performance work only when the implementation is a
general engine mechanism and the evidence shows that it is not merely fitted
to broad-micro. Every runtime unit must therefore include both the complete
broad portfolio and the pinned external preview before it is closed. A unit is
rejected as campaign progress when its benefit depends on an internal case
shape, benchmark identity, fixed iteration count, expected checksum, or source
path; when it reduces comparable external coverage; or when a nominal internal
win leaves the relevant external workloads materially worse without an
explained, independently measured tradeoff. Focused microbenchmarks may locate
a bottleneck, but they cannot define the runtime semantics or serve as the
unit's only acceptance evidence.

External results remain informational neutral-shell measurements rather than
official suite scores. That limitation does not make them optional: their role
inside T018 is the mandatory independent check that an optimization improves
ordinary JavaScript mechanisms beyond the repository's own benchmark shapes.

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
- every one of the 25 case ratios <= 0.50;
- every critical family ratio <= 0.50;
- no invalid block, failed linearity probe, checksum mismatch, or timer-limited
  case;
- focused tests plus `scripts/check.sh` pass, preserving Test262 behavior;
- a second independent run confirms the final result before completion.

The target is a campaign acceptance criterion. Existing hosted previews remain
informational and non-gating until T017 M6/M7 fixed-hardware qualification is
complete.

## External Generalization Contract

Every trusted `main` push already publishes a pinned, execution-only external
preview. Each runtime unit must compare candidate, the exact preceding base,
and QuickJS-NG in the same external run; cross-run candidate-duration movement
is diagnostic only and cannot accept a unit because hosted-runner drift is not
controlled across runs. These neutral shell ports are not official JetStream,
Kraken, or SunSpider scores, and incomplete suites have no suite score. They
are still the campaign's independent anti-overfitting evidence because their source,
adapter, case inventory, engine revisions, outer wall timer, and per-case
results do not depend on the broad-micro workload.

Completion additionally requires a repeatable final external preview in which:

- every pinned external case is runnable and comparable; timeouts, unsupported
  cases, and reduced coverage cannot manufacture a better ratio;
- every external case is <= 0.50x qjs-rust/QuickJS-NG and the diagnostic
  geometric mean is <= 0.50x for each of the three pinned suites;
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
- [ ] B4 reach <= 0.50x overall, in every critical family, and in every one of
  the 25 broad cases.
- [ ] B5 make every pinned external case runnable, then reach <= 0.50x for each
  case and each suite geometric mean with no coverage reduction.
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
  --base /path/to/base/qjs \
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

The exact-SHA hosted workflows for
`d17cde1891f329debfeb023781ae13e3ae72318c` all completed successfully. CI
passed the comparison, Test262 subset, and full check jobs. Exact Test262
coverage remained 42,671 pass, one fail, zero timeout, and one actionable gap.
The hosted broad preview retained 25/25 cases, 225/225 eligible measurements,
and all 75 linearity pairs. Its 1.009458x candidate/base result was
inconclusive on the variable hosted runner; candidate/QuickJS-NG was 0.357441x.
The hosted external preview still failed B5 at 12.714481x QuickJS-NG for 5/5
JetStream cases, 7.670607x for 10/14 Kraken cases, and 11.436771x for 23/26
SunSpider cases. Hosted broad raw/report SHA-256 are
`5c38f33d55998c24c791a86d0f16b522932423d005217780aa0ba96d42244e40`
and `994c8f2464f94475266a4f46dfadc8b17dc88e9d98721105328c116879e4ea6a`;
hosted external raw/report SHA-256 are
`b8c64f9f8aa3c0b947a7462887168370c345597bc216b8068834d2fe989148a5`
and `f48754941ee1ee3f89514ea295fbbe3443a40c7610c814f9b02499a99db986f3`.

The forty-fifth v2 unit follows a fresh JetStream Gaussian Blur profile into
TypedArray representation. The five spec-internal view slots for kind, backing
buffer, byte offset, fixed length, and length tracking previously lived as
hidden string-keyed object properties. Each indexed access therefore repeated
property-map hashing and descriptor cloning before reading or writing bytes;
scripts that guessed the NUL-prefixed keys could also delete or overwrite the
metadata. Typed arrays now store one immutable slot record in the object's
existing lazily allocated cold data. Ordinary objects gain no hot-layout field,
while all TypedArray helpers read the record directly and continue to query the
backing buffer for live detached and resizable state. A focused test proves
that deleting or forging the old string keys cannot change a real Int16Array's
brand, element value, or element width. No workload name, source path,
iteration count, checksum, or expected benchmark value appears in the
implementation.

Against base binary SHA-256
`aebef9ebc2a8805c530888ca412c9a21febf94e55350874ce1fefab1918f7680`,
candidate binary SHA-256
`d2bda569d41560a1e00607163a0fe5f258d2c46257b8ae362231d0e02fa093f7`
cut JetStream `gaussian-blur` to 0.515644x base. Complete one-block candidate
and same-window base external previews retained identical coverage at 5/5
JetStream, 11/14 Kraken, and 23/26 SunSpider. Common-case candidate/base suite
geometric means were 0.878084x, 1.004602x, and 1.000667x; the non-target suite
movements were neutral, and the complete broad diagnostic below did not
reproduce the short `json-parse-financial` outlier. Candidate/QuickJS-NG still
failed B5 at 8.001698x, 4.671355x, and 6.202666x. Candidate external raw/report
SHA-256 are
`9c1153991587be3a76c1f06f22fe2a6c0084e892a7b42a2669d39fb5370c5f4e`
and `7f51336e473b1a37b56917449c25c607e74e7fc893e7e6ebd0b27532709d9194`;
base raw/report SHA-256 are
`7ccd6895ac6437fdec87a2711d5ea8be2cf6339a242123cc48fe616208bada1f`
and `c0a4d391b07a81935cc7bffab85ee20bc293d53b0fbce1d51d87498dde4b81ed`.

The complete three-role broad development diagnostic produced 225/225
eligible exact-checksum measurements, zero non-OK samples, and 600 linearity
samples with all 75 engine/case pairs passing. Candidate/base was 0.999558x
overall; family ratios ranged from 0.983997x for string through 1.003190x for
allocation, and the worst case was 1.008119x. Candidate/QuickJS-NG was
0.194814x overall, but allocation still failed B4 at 1.145679x. The strict
analyzer correctly rejected the receipt-less dirty development binaries, so
this is regression evidence rather than a fixed-hardware claim. Broad raw
SHA-256: `d42ede2ad98dd0866348e2c2b9a3007f35aeb276bd80cd6615dbb44816d477dd`.
This representation change materially improves an external typed-array
workload while preserving the non-target regression guard; B4 and B5 remain
active.

The exact-SHA hosted workflows for
`c13aaa5004b4b4537c2110e7bca4f91679dcbf28` all completed successfully. CI
passed the comparison, Test262 subset, and full check jobs. Exact Test262
coverage remained 42,671 pass, one fail, zero timeout, and one actionable gap;
the burndown and comparison-case artifact SHA-256 values are
`ff270ba5c2e1927c4774f3d95357ee2ee51b6a89e58fdee6c63453dbdca55d72`
and `3ba9a1a20f4cd430f9f33471ca97652a67a12602740b26453f2e06fd63a58329`.
The hosted broad preview retained 25/25 cases, 225/225 eligible measurements,
and all 75 linearity pairs. Its 1.016688x candidate/base result was
inconclusive on the variable hosted runner; candidate/QuickJS-NG was
0.366655x, with allocation still failing B4 at 2.706040x. The hosted external
preview still failed B5 at 10.804721x QuickJS-NG for 5/5 JetStream cases,
6.061909x for 7/14 Kraken cases, and 11.057512x for 23/26 SunSpider cases.
Hosted broad raw/report SHA-256 are
`9e8e3be5cf6a16caa674bf3dbc1a664dabbcfc5598fc14b701c912fa6e5e19de`
and `1d7fa415b203f3e74d31ef6797f98c5a953d42e81d236efbd246825d710d588f`;
hosted external raw/report SHA-256 are
`4ab4e3d229db2ee447790495d7231a4dfe73d9d5116f6c7676770c72c8d01556`
and `e952355b78ed56b1177b13f3207a2ee81cd30ab19816c5856d5e2e9e7077d01f`.

The forty-sixth v2 unit follows a fresh external class-heavy profile into the
general field-initializer call path. Public, private, instance, and static
fields previously evaluated every initializer thunk through a compatibility
environment, allocating argument and binding containers even when bytecode had
no direct eval, closure creation, `super` operation, complex parameters, or
other observable dynamic environment state. Simple field initializers now use
the existing slot-backed leaf-call path for their receiver and captured values;
the guard leaves every semantic-heavy initializer on the complete call path.
A focused test covers both an earlier-field `this` read and capture of the class
inner binding. Existing closure, derived-constructor, proxy, private-field, and
`super` tests continue to exercise the preserved slow paths. No workload name,
source path, iteration count, checksum, or expected benchmark value appears in
the implementation.

Against base binary SHA-256
`d2bda569d41560a1e00607163a0fe5f258d2c46257b8ae362231d0e02fa093f7`,
candidate binary SHA-256
`d4d4930db2bb3e8939ae9228ea979dbe25c34d2bf17ddd5aad136d7b6c6d957d`
cut the independent JetStream class-field raytrace case to 0.862964x base.
Complete one-block candidate and same-window base external previews retained
identical coverage at 5/5 JetStream, 11/14 Kraken, and 23/26 SunSpider.
Common-case candidate/base suite geometric means were 0.972917x, 0.998275x,
and 0.999306x. Candidate/QuickJS-NG still failed B5 at 7.776865x, 4.702112x,
and 6.167400x; the improved raytrace case itself remained 11.793273x.
Candidate external raw/report SHA-256 are
`c432e480e5a3f9b7f28422b208e6255d6095db323ca6cf0d8d85036b62d85dec`
and `eca416d406b986635b09c66de587387b1327d9df64d41e5fa4be409baae8100e`;
base raw/report SHA-256 are
`62f34e1a24e4e0993bb7d6b152aa3513808869aa60872e943c24310352027a5e`
and `bfbd9b2a358aeacb14f9e630ec332d7d7ce689708534d87cbd83343bbecc3e64`.

The complete three-role broad development diagnostic produced 225/225
eligible exact-checksum measurements, zero non-OK samples, and 600 linearity
samples with all 75 engine/case pairs passing. Candidate/base was 1.003467x
overall; family ratios ranged from 0.997485x for binding through 1.057264x for
the single string case, which was also the worst case. Candidate/QuickJS-NG was
0.194702x overall, but allocation still failed B4 at 1.138514x. The strict
analyzer correctly rejected the receipt-less dirty development binaries, so
this is regression evidence rather than a fixed-hardware claim. Broad raw
SHA-256: `d1db2d4998b95ad988670acc4b31da7219d28f300824c5f4055629ce7e36b962`.
This unit improves a class-heavy external program while preserving both other
external suites and the internal regression guard; B4 and B5 remain active.

The exact-SHA hosted workflows for
`eaf64f20f4790681a38dfbc382696eb52330ed2e` then completed successfully. CI
run `29617126163` passed comparison, Test262 subset, and full-check jobs.
Coverage run `29617310831` retained 42,671 Rust passes, one failure, zero
timeouts, and one actionable gap; its burndown and comparison-case SHA-256
values are
`0a9594104963a6815964c986c5779d09088d4a88761688f9fa9c476ef762a900`
and `3ba9a1a20f4cd430f9f33471ca97652a67a12602740b26453f2e06fd63a58329`.

Performance Preview run `29617126173` retained 25/25 broad cases, all 225/225
eligible measurements, and all 75 engine/case linearity pairs. The hosted
runner's 1.007516x candidate/base result was inconclusive; candidate/QuickJS-NG
was 0.366549x overall, but allocation still failed B4 at 2.822703x. The other
family ratios were 0.615636x array, 0.173452x binding, 0.087795x builtin,
0.528594x call, 0.188521x control, 0.170350x property, and 0.521216x string.
Hosted broad raw/report SHA-256 are
`48ecceb438c6359b17346f048e99305ebde22d46a4c53a1c361d6644e33ec467`
and `c76237ea82ad301bbb51c99929da9ec91accaade2238e3e2d2c8a53e7c2f15a4`.

The same exact-SHA artifact retained comparable coverage at 5/5 JetStream,
7/14 Kraken, and 23/26 SunSpider, but qjs-rust lost every comparable case.
Suite diagnostic ratios remained far from B5 at 10.625742x, 6.170478x, and
11.060351x QuickJS-NG. Hosted external raw/report SHA-256 are
`33e631640f11669044b630bc663e7d7849ef60a85f6077cabf853d020cc1c848`
and `59bd814501b7b4f0cf40c4be3f96b291bd44f075b84484a40026658861421f6f`.
The target JetStream class-field raytrace duration nevertheless fell from
10,044,283,046 ns at the exact base artifact to 8,697,197,004 ns here, a
0.865886x cross-artifact ratio, while its candidate/QuickJS-NG ratio improved
from 16.940592x to 14.608166x. This confirms that the mechanism generalized to
the independent external workload; it does not mask the much larger remaining
engine-wide deficit. The external results, rather than internal-only wins,
therefore continue to drive the next structural bottlenecks. B4 and B5 remain
active.

The forty-seventh v2 unit follows a fresh external bitops profile into the
general global-binding read path. A declared top-level `var` or function
previously re-hashed its source name and consulted the realm/global object on
every read even though declaration instantiation already provided a stable
shared realm cell. Such reads now use the declaration's indexed slot backed by
that cell. Writes remain on the complete global path so writable descriptors,
strict/sloppy behavior, `with`, direct eval, global-object mutations, and
accessor/deletion deoptimization keep their existing checks. The same change
made every top-level declaration install its realm cell at frame entry. It also
closed two older string-append paths that mutated the realm value table
directly: both now refresh an existing shared cell after copy-on-write. Focused
tests prove the read opcode/write opcode split and visibility after direct
eval, global-object assignment, `Object.defineProperty`, and a nested string
append. No workload name, source path, iteration count, checksum, or expected
benchmark value appears in the implementation.

Against base binary SHA-256
`d4d4930db2bb3e8939ae9228ea979dbe25c34d2bf17ddd5aad136d7b6c6d957d`,
candidate binary SHA-256
`989f3cd0c827db801b23ebf95ef8eea326c144a34e05313c1327ef2b24132926`
improved alternating outer-wall probes of six independent SunSpider sources.
Candidate/base medians were 0.804612x for `bitops-bitwise-and`, 0.979102x for
`bitops-nsieve-bits`, 0.936710x for `3d-morph`, 0.963054x for
`access-nsieve`, 0.994970x for `math-partial-sums`, and 0.994814x for
`string-validate-input`. The 9-run probes were independently repeated with 21
runs for the three closest or initially variable cases. These focused probes
use the pinned upstream source and the external preview's raw CLI mode, but are
development evidence rather than a complete suite claim; the exact-SHA hosted
preview remains authoritative.

The complete one-block three-role broad development diagnostic produced 1,473
OK samples: 498 calibration, 225 startup, 75 warmup, 75/75 eligible
measurements, and 600 linearity samples. Candidate/base was 0.987170x overall;
family ratios ranged from 0.955193x call through 1.001039x property, and the
worst individual candidate/base ratio was 1.011043x. Candidate/QuickJS-NG was
0.192872x overall, but allocation still failed B4 at 1.136663x. The strict
analyzer correctly rejected the receipt-less dirty development binaries, so
this is regression evidence rather than a fixed-hardware claim. Broad raw
SHA-256: `fc5533c3541b3fdc108c85e3744a2cd90b976e83bc19ef8e09188e3158dbfc33`.
This unit improves several structurally different external programs while
keeping the internal family guard neutral; B4 and B5 remain active.

The exact-SHA audit for
`e73773d4481310490b9f6d12e39bfa9fa759c2fe` deliberately separates workflow
success from goal success. CI run `29620670097` passed all three jobs, and
Performance Preview run `29620670106` completed all 225/225 broad measurements
and all 75 linearity pairs. On the hosted runner, candidate/base was
0.991388x overall. The binding family improved to 0.969603x and the top-level
function-call case to 0.800568x, while the aggregate remained below the
campaign's practical effect threshold and several unrelated cases varied in
both directions. Candidate/QuickJS-NG was 0.368849x overall, but allocation
still failed B4 at 2.168137x; the other family ratios were 0.590612x array,
0.169022x binding, 0.100850x builtin, 0.610915x call, 0.246021x control,
0.144766x property, and 0.527411x string. Hosted broad raw/report SHA-256 are
`0c22db41a60128cda6b315342bb2eac6bf8715d54442fde9d7d5b64926903fe7`
and `e356af5fa81c678438bbba5fe895fe239fb5de36a00408147fa825671e67ec94`.

The same artifact is the authoritative warning against treating an internal
win as general engine speed. It retained 5/5 JetStream, 8/14 Kraken, and 23/26
SunSpider comparable cases, but qjs-rust still won none. Suite diagnostic
ratios were 10.710538x, 7.155944x, and 11.051522x QuickJS-NG: effectively no
suite-level closure of the external deficit. Some individual ratios improved,
including `string-validate-input` from 52.765730x to 36.587812x and the
class-field raytrace from 14.608166x to 13.369248x, while the targeted
`bitops-bitwise-and` ratio moved from 20.896859x to 23.019352x on the new
hosted run. Cross-run absolute durations moved for both engines and are not a
fixed-hardware comparison, so those mixed ratios outrank the favorable local
probes. External raw/report/manifest SHA-256 are
`65ad40f0cbc57fe71bef0fcad441428fa89e452782a683f19d68ab329eff24d6`,
`2fe13bab16b5600e7093d4829aeb2698bcafdf4a91cb8d78c186b08b5a33c965`,
and `fbcd37039908f72342effd1c2d9d3b12156f180bec745e95ff9a8bae2d56a93a`.
B4 and B5 remain active, and the next optimization must again target a general
external-profiled mechanism rather than an internal workload shape.

Coverage run `29620811076` also exposed why a green workflow is not sufficient
evidence. It completed successfully, but Rust moved from 42,671 pass / 1 fail
to 42,668 pass / 4 fail, creating three new actionable gaps in the direct-eval
spread family. The global-read change made a pre-existing bad cell route
observable: an unresolved assignment in direct eval could attach a
same-named function local to the realm cell, so the later top-level indexed
read observed the local assignment. The immediate correctness follow-up now
resolves a direct-eval caller or deoptimized binding cell before considering
the realm cell, in both regular and direct VM initialization, and covers plain
and spread eval with a shadowing function `var`. All four exact upstream
`eval-spread*` cases, 114 focused eval/call cases, 1,389 runtime tests, and the
full 5,139-case curated subset pass locally. The regressed coverage artifact's
burndown/comparison SHA-256 are
`8ef8c44a2ea9d4b676f5c0797c76350504f4ace9efc6829a6b7ac7a04548aaaa`
and `9961943568f0540bda862cf368c7175e105bf7ac4bd811dc955c5fc49186a696`;
the next exact-SHA coverage artifact must restore the former one-gap baseline
before this optimization unit is accepted.

The exact correction commit
`e56aee3422bbba6e9f9b53ffaf0ea5d28f4241b4` closed that requirement. CI run
`29622215672` passed all jobs, and coverage run `29622349540` restored 42,671
Rust passes, one failure, zero timeouts, and one actionable gap across all
42,672 configured cases. The restored burndown/comparison SHA-256 values are
`a44f50aaf14636af13027f8228b49f53012d51f664b4379250e740d87360f1bc`
and `3ba9a1a20f4cd430f9f33471ca97652a67a12602740b26453f2e06fd63a58329`.
This is the semantic metric closure; workflow success alone was not counted.

Performance Preview run `29622215689` also completed at that exact correction
SHA. Hosted broad candidate/base was 0.987593x and candidate/QuickJS-NG was
0.355157x overall; allocation still failed B4 at 2.538944x. The other family
ratios were 0.577693x array, 0.169475x binding, 0.092127x builtin, 0.502671x
call, 0.188787x control, 0.172372x property, and 0.522510x string. Broad
raw/report SHA-256 are
`78bc381763394af7626c20453a949b7827e20e03282172f425bda6b8e2e79340`
and `e7fbfbd8264fd0b70c9eadc5708e39e0ad97b0b952826797706580c42e3155b3`.

The same exact artifact retained 5/5 JetStream, 7/14 Kraken, and 23/26
SunSpider comparable cases and again won none. Suite diagnostic ratios were
10.169012x, 6.016440x, and 10.481689x QuickJS-NG. In particular,
`string-validate-input` measured 52.310699x, which confirms that the external
string-accumulation deficit remains real despite the favorable internal broad
ratio. External raw/report SHA-256 are
`7f59bb6353e9ad02c65d5687bf2bbf765bb3f1ca4a3146f8abb449bc70e55b89`
and `5d957b9a0527bd17e5c5b0eb9bb9ae4f4bc54d216f415018763e3d71cf9e4ee7`.

The forty-eighth v2 unit follows the independent SunSpider string profile into
general primitive-string accumulation. A dynamic string `+` followed by a
binding store previously copied the whole left string even when its extra
`Rc<String>` owners were only engine-internal slot, upvalue, realm, and global
data-property mirrors. The VM now recognizes that ordinary bytecode store
shape at execution time, temporarily drops only exact-pointer internal
mirrors, and lets `Rc::unwrap_or_clone` retain the allocation and its capacity.
Any real JavaScript alias keeps an `Rc` alive and still forces copy-on-write;
RHS objects keep the full observable ToPrimitive path; immutable, accessor,
non-writable, module, `with`, and direct-eval bindings remain on their existing
paths. Moving an owned string through ToString now also avoids an unnecessary
copy. The numeric primitive fast path remains first, after the first complete
broad diagnostic caught and rejected an intermediate dispatch-order
regression. No workload name, source path, iteration count, checksum, or
expected benchmark value appears in the implementation.

On the same macOS host, exact base binary SHA-256
`2f9fafddea0f28046b4461672c8800f9aef8a3d30525544e935cd0f442974b53`
and final candidate binary SHA-256
`be9652fb29b39b0638bc9a55eba055bd477492430030852d8fc7be121eb263b4`
retained identical comparable coverage at 5/5 JetStream, 11/14 Kraken, and
23/26 SunSpider. Common-case candidate/base geometric means were 1.003227x,
0.999240x, and 1.008344x, so this is not a suite-wide speedup. The independently
sourced `string-validate-input` case nevertheless fell to 0.724812x base, and
its candidate/QuickJS-NG ratio fell from 9.318717x to 6.821436x. The candidate
still lost all comparable cases; suite ratios remained 7.849188x, 4.723542x,
and 6.182158x QuickJS-NG. Candidate external raw/report SHA-256 are
`66dd73dea64a967bded6a550ad9abbbf5b69fb7896ca797248687123eeb650cd`
and `4ab9e6475fcc855facdf6f344b26c792dc771aff3882d614bdde4d41a9b8e280`;
base raw/report SHA-256 are
`dbe7b03155fb5ec7df2b529b9af23649ac6a03ae8112c031722e659496310a7a`
and `947e157fca4ec43303ac8c71f732bc81e86031739d7808c8ef253cf0c25b2885`.

The final one-block broad diagnostic produced 75/75 eligible measurements,
600/600 passing linearity samples, and 1,462 OK samples. Candidate/base was
0.999098x overall; family ratios ranged from 0.996075x array through
1.003763x string, and the worst individual ratio was 1.029102x. Candidate/
QuickJS-NG was 0.192541x overall, but allocation still failed B4 at 1.142756x.
The strict analyzer correctly rejected the receipt-less dirty development
binaries, so this is regression evidence rather than a fixed-hardware claim.
Broad raw SHA-256 is
`47368de161f9f66180c1aed665a3b50e632818626c98a8affdcc0f778f6c3c75`.
This unit closes one profiled external mechanism without trading away the
internal families; B4 and B5 remain active.

The trusted hosted closure for commit `39fb188bd7bc015b03eb569a9830304d32b81a4e`
reinforces the external-first interpretation. Performance Preview run
`29626227250` measured broad candidate/base at 1.0075x and candidate/QuickJS-NG
at 0.3564x, but the same exact artifact measured qjs-rust at 10.604x JetStream,
5.939x Kraken, and 10.044x SunSpider versus QuickJS-NG, with zero qjs-rust wins.
The external comparable inventory was 5/5, 7/14, and 23/26. Broad summary and
external report SHA-256 are
`f5760ccff29505ba2bf72a5be668df08467d6e293fa036b60cbd6b008ba2155d`
and `36f62381d149150b4e5b084c6c890ce77d413f80967e850a4a3d55499ecfaadd`.
Coverage run `29626336016` independently retained 42,671 passes, one failure,
zero timeouts, and one actionable gap. Its burndown/comparison SHA-256 are
`d58b953981f0da104efb995fa28764b13113a5e5c3ddc08d39f2bc43feeb1e22`
and `3ba9a1a20f4cd430f9f33471ca97652a67a12602740b26453f2e06fd63a58329`.
The favorable internal score therefore remains a regression guard, not proof
of general engine speed.

The forty-ninth v2 unit follows the worst comparable hosted SunSpider global
loop into the general realm-binding mechanism. Top-level `var` reads
previously re-hashed the binding name to prove that a cached upvalue was the
realm cell on every access, while writes cloned the opcode name and traversed
the full global-binding path. The VM now classifies its first 128 local slots
once at frame creation, directly reads initialized realm cells, and directly
updates existing own writable global data properties while keeping the realm
cell and local mirror coherent. Deleted globals retain the uninitialized
marker and deopt through observable global-object lookup; accessors, missing
properties, lexical/module bindings, caller locals, direct eval, and strict or
sloppy non-writable writes retain their existing semantic paths. Store helpers
also borrow the immutable bytecode name so the common path performs no name
clone. The implementation contains no benchmark name, source path, iteration
count, checksum, or expected result.

Against the exact `39fb188b` local base, the independent one-block external
inventory retained 5/5 JetStream, 11/14 Kraken, and 23/26 SunSpider comparable
cases. Common-case candidate/base geometric means were 1.003942x, 0.994949x,
and 0.970081x respectively. `bitops-bitwise-and` improved to 0.715605x base;
the best and worst unrelated SunSpider ratios were bounded by that result and
1.068998x. Candidate/QuickJS-NG suite ratios moved from 7.849188x, 4.723542x,
and 6.182158x to 7.792788x, 4.699712x, and 6.101560x. The candidate still lost
all comparable cases, so this is measured structural progress rather than B5
completion. Candidate external raw/report SHA-256 are
`c6a55a4838cbf68e30820bc2fc6314828ccb2287c0d6dd8ea73bc0489b195ee4`
and `a6aa7cad1f73ebb90ca8722a99a3be7a884b1e0b3e98030f217e88081840a5ef`.

The accompanying one-block broad diagnostic retained all 25 cases and found a
0.983771x candidate/base geometric mean. The call family improved to
0.942244x, led by the general top-level call shape at 0.701536x; every other
family was between 0.983683x and 1.000439x, and the worst individual ratio was
1.002788x. Candidate/QuickJS-NG was 0.189843x overall, while allocation still
failed B4 at 1.122560x. Because the dirty development binaries have no trusted
receipts, the strict report correctly rejected claim generation; raw SHA-256
is `6734a73d0ebee06f538607a0958447daf2b6658d5487f20a3676af5c4dca12e1`.

The exact pushed closure is commit
`a0e77626616b8f86e517c73f06f6178db0a286fc`. Performance Preview run
`29628011436` retained 25/25 broad cases, 75/75 passing linearity probes, and
three valid blocks. Candidate/base was 0.990962x overall, with binding at
0.957254x and call at 0.954900x; candidate/QuickJS-NG was 0.354227x overall,
but allocation still failed B4 at 2.654359x. Broad raw/report SHA-256 are
`f94dcbd348e398ef64e17d9372e52694e60005da18dd07ebb23aad10155fa719`
and `a60efa1bd35478ee567576877489051b75d7a4a4e2d2d89b7c7ab1e061e2ac79`.

The independently sourced result remains the controlling interpretation:
the same exact artifact retained 5/5 JetStream, 7/14 Kraken, and 23/26
SunSpider comparable cases, lost every one, and measured 10.850x, 6.362x, and
10.206x QuickJS-NG. Thus the internal improvement is not evidence of general
engine leadership. External raw/report SHA-256 are
`aa220f95dd2f101448f3a04eceaad8928186fea2a1a561b3d7cb3219f9ba4b53`
and `77b6590b5c801b51e0bee7ce0de6e812f41eeba37bcfe026345da5b1a5a944d6`.
CI run `29628011449` passed, and coverage run `29628122466` retained 42,671
passes, one failure, zero timeouts, and one actionable gap. Its
burndown/comparison SHA-256 are
`00e6508fb2210d6018f113f70ceee69bbe5e93bb1f5af06b7d453b3b6e42a637`
and `3ca541ca3392c4e5198bff5fa938a7b8e9b8cd6cb709b65b28b207cc64d18dd8`.

A bounded realm-eval bytecode-cache prototype was evaluated before the next
runtime unit. It improved a synthetic repeated-eval diagnostic by roughly
12--16%, but did not improve the independently sourced Date workload used to
test the hypothesis. The prototype was discarded rather than counted as
progress: an internal diagnostic alone is not sufficient evidence under the
external generalization contract.

The fiftieth v2 unit follows the independent SunSpider Date cliff into the
general native-call mechanism. Every ordinary Date getter and `valueOf`
previously cloned a complete call environment before dispatch and applied it
again afterward, even though these functions only read their Date receiver.
The VM now sends all such environment-free Date natives through its existing
identity-based fast native dispatcher. Replaced functions still use their
actual callable identity, invalid receivers still throw, argument expressions
are still evaluated, and the getters ignore rather than convert extra
arguments as required. The implementation contains no workload name, source
path, iteration count, checksum, or expected result.

Against the exact unit-49 local base, the independent one-block external
inventory retained 5/5 JetStream, 11/14 Kraken, and 23/26 SunSpider comparable
cases. `date-format-tofte` improved to 0.851757x base and
`date-format-xparb` to 0.616619x base. Common-case candidate/base geometric
means were 0.998801x, 1.012140x, and 0.967962x respectively; candidate/
QuickJS-NG suite ratios were 7.864687x, 4.742293x, and 5.926553x, with zero
candidate wins. This closes a real external mechanism but remains far from
B5. External raw/report SHA-256 are
`c675f35e88a5a1e64ebc0a98d6a5d94e8b1b4d4feef201fc3fcdca31111e1f33`
and `d9750ada24eb9fd8eef8c0036b75896acbcba9be7784777ed08acb7656119cec`.

The accompanying one-block broad diagnostic retained all 25 cases, 75/75
eligible measurements, and 600/600 successful linearity samples. Candidate/
base was 1.008617x overall and candidate/QuickJS-NG was 0.191653x; allocation
still failed B4 at 1.134601x. A single unrelated `plain_function_call` sample
showed 1.114594x base, so the complete six-case call family was independently
rerun for three blocks. That confirmation measured 1.003226x candidate/base
overall and 1.002650x for `plain_function_call`, rejecting the apparent cliff
as one-block noise rather than silently excluding it. Broad and call-rerun raw
SHA-256 are
`a7cb68592f9e1a966c9238123245945ae5c9bb0ca63ff96eab0d87c3ca0d97a9`
and `e451838b3a38acdb74a2b227e765860e1e9dfdcdbbf3a506c87671379af2adb7`.
Because the development binaries lack trusted receipts, these are regression
diagnostics rather than hosted claims.

The exact pushed closure is commit
`769e2dcb3cf2944d807856c732a3c24e9a133245`. Performance Preview run
`29629531518` retained 25/25 broad cases, 75/75 passing linearity probes, and
three valid blocks. Candidate/base was 1.005411x and candidate/QuickJS-NG was
0.353097x overall; allocation still failed B4 at 2.630826x. Broad raw/report
SHA-256 are
`a1ec049a7fd5e25e3011502543ff9c0d2169d86a174acb4cbf4b667661a71161`
and `b23f02272533192d8d6cc1bab2dd025dd2ae21556003451149c4fa2d4f27f074`.

The exact external artifact retained 5/5 JetStream, 6/14 Kraken, and 23/26
SunSpider comparable cases and still lost every one; suite diagnostic ratios
were 11.166x, 6.488x, and 10.243x QuickJS-NG. Nevertheless, the two independent
Date cases confirmed the mechanism on the hosted runner:
`date-format-tofte` fell from 1,121.565 ms at the exact unit-49 base to
966.720 ms (0.861938x), and `date-format-xparb` fell from 659.281 ms to
421.213 ms (0.638923x). External raw/report SHA-256 are
`1cdbcc643d819b9a06ce4e47fecd2c7b165954bbc6ec85c624ad58e5dc7b9241`
and `ead9c8dcc35bfe6efe6a1561098ce300eba815f669706aa5575eb72acf39f520`.
CI run `29629531527` passed, and coverage run `29629626617` retained 42,671
passes, one failure, zero timeouts, and one actionable gap. Its
burndown/comparison SHA-256 are
`f5c7e5b5787b504ccd3013dd8b2486372c4dbe90aea7f011ce8fd5a7ca098cb9`
and `3ca541ca3392c4e5198bff5fa938a7b8e9b8cd6cb709b65b28b207cc64d18dd8`.
The external aggregate remains the campaign verdict; the Date improvement is
accepted as a general mechanism slice, not as B5 completion.

The fifty-first v2 unit continues from the independent bitwise profile into a
general top-level binding write. A hoisted script `var` starts with an empty
compatibility local but already owns an exact shared realm cell. The previous
indexed fast store was incorrectly gated on that compatibility local already
being initialized, so the first assignment could never admit the fast path
and every later loop update repeated name hashing and global-object/realm
synchronization. The VM now attempts the exact-cell write before consulting
the local mirror. It still admits only an existing own writable data property;
missing, accessor, deleted, immutable, module, direct-eval, and non-writable
cases retain their observable paths. No workload name, source path, iteration
count, checksum, or expected result appears in the implementation.

Against the exact unit-50 local base, the independent one-block external
inventory retained 5/5 JetStream, 11/14 Kraken, and 23/26 SunSpider comparable
cases. Common-case candidate/base geometric means were 1.010112x, 0.999584x,
and 0.980121x. `bitops-bitwise-and` improved to 0.718379x base; every other
common case was bounded by 0.942692x and 1.043184x. Candidate/QuickJS-NG suite
ratios were 7.910x, 4.743x, and 5.831x, with zero candidate wins, so B5 remains
open. Candidate external raw/report SHA-256 are
`5b339d3a7d4a38312df2f9ab16872a65d23c3ee2c5d9be8ed844678c15214e64`
and `1dabe5a61b0a6493283172324ec2b90b9706c5f7b66bed6c03e5f2656f95607c`;
base raw/report SHA-256 are
`850580124798146c5739fb28525a2f160bdae75b365fa749ac8a9adde6bd1c7a`
and `13c8a2823c034749fe37b33ccee7a8bdcf6fb16e8d7780b537e65b52cfd6d60d`.

The accompanying one-block broad diagnostic retained all 25 cases, 75/75
eligible measurements, and 600/600 successful linearity samples. Candidate/
base was 0.981309x overall. The call family improved to 0.921300x, led by the
general top-level call shape at 0.605039x; the other families ranged from
0.982964x through 1.027620x, which was also the worst individual ratio.
Candidate/QuickJS-NG was 0.187216x overall, while allocation still failed B4
at 1.146695x. The receipt-less development run is regression evidence rather
than a hosted claim; broad raw SHA-256 is
`5262e49fb51499f66a35527a6918a241a69e2f05ad7542e0e1fc85e1fdfb00c2`.
Exact pushed performance and coverage artifacts remain required before
accepting this unit.

The exact pushed closure is commit
`473c5185763820f699dd2eb12d03558a86c8ed70`. Performance Preview run
`29630951841` retained 25/25 broad cases, 75/75 passing linearity probes, and
three valid blocks. Candidate/base was 0.996413x overall and candidate/
QuickJS-NG was 0.352509x. Allocation remains the controlling internal gap:
the object, array, and closure allocation cases measured 2.995x, 3.457x, and
1.888x QuickJS-NG. Broad raw/report SHA-256 are
`4af4c6d0282b75eef1a5cd87655fa0e874483de2c2025dfb559b06484098a9ba`
and `b1f56074726d075f892525a15f996cb825d7b8551f2cde416e49b785c8869731`.

The exact external artifact retained 5/5 JetStream, 7/14 Kraken, and 23/26
SunSpider comparable cases, lost every one, and measured 11.139x, 6.581x,
and 10.063x QuickJS-NG. External raw/report SHA-256 are
`65590d7bb66c2e2c27156899635ecbb0c06017767d9343f4a3eb93f9ce9eeff7`
and `bf84482525ae633877dd6de6cc72929fac56e92b60e34c217c4f450e8df551a4`.
CI run `29630951843` passed, and coverage run `29631049852` retained 42,671
passes, one failure, zero timeouts, and one actionable gap. Its
burndown/comparison SHA-256 are
`c6d4bdadcee2e8224b3809dac66cd92e02aabd415e29ae18119eb6ffe550fdff`
and `3ca541ca3392c4e5198bff5fa938a7b8e9b8cd6cb709b65b28b207cc64d18dd8`.
The hosted external result, rather than the internal bitwise win, remains the
campaign verdict.

Three broader realm-binding representations were then prototyped and rejected
before accepting any fifty-second runtime unit. Eagerly storing every realm
binding in an upvalue cell improved the focused top-level binding diagnostic,
but common-case external candidate/base geometric means regressed to 1.009x,
1.012x, and 1.003x for JetStream, Kraken, and SunSpider. A hybrid cell/value
dual table was slower even in the focused diagnostic because every ordinary
builtin lookup paid a failed cell-table probe before its value-table lookup.

The final prototype kept a single realm table and lazily promoted only shared
bindings from direct values to cells. It preserved all 1,389 runtime tests and
improved the focused six-million-iteration diagnostic by roughly 14--15%, but
the adjacent same-host external comparison rejected it decisively. JetStream
regressed to 1.074133x base with all 5/5 cases slower; Kraken regressed to
1.024859x with 10/14 cases slower; SunSpider regressed to 1.006060x with
17/26 cases slower. `bitops-bitwise-and` improved to 0.866410x base, but a
single local win does not compensate for suite-wide generalization failure.
Candidate external raw/report SHA-256 are
`13cbf7081da455d4fb3828dd7ad7976ae394d13ef64c2ecfc7754f77a1bd990b`
and `5156523f7ad9358d69c71f1e9b6ec527cb96c29055e3a42a347978f808b240d2`;
the adjacent exact unit-51 base raw/report SHA-256 are
`d9f98f7a2697e77f6bfbcac7f6ddd1e1f80445507558066c03e52240e6f52b9c`
and `1b185f387e410a6b47c2a1d786818867550d3f0e6d483f28e312d1bc30c6f126`.
All three prototypes were discarded without a runtime commit. This is an
intentional application of the external anti-overfitting contract: internal
benchmark speedups alone are not T018 progress.

The fifty-second v2 unit instead comes directly from independent external
profiles. macOS sampling of Kraken `ai-astar` and `audio-oscillator` placed
`apply_env` at the top of both stacks (44 and 57 samples respectively), with
the time below it dominated by caller-local snapshots, string clones, and
`HashMap` insertion/hashing. Ordinary native functions do not dynamically
inherit their caller's lexical environment: user callbacks already carry
their exact closure/upvalue cells, and global state is shared by the realm.
The VM therefore gives frame-independent native calls a realm-only `CallEnv`
instead of reconstructing every active caller slot and name map. Direct eval
is explicitly excluded and retains the active dynamic-name view. The focused
correctness test covers both coercion and array callbacks mutating captured
locals, plus direct eval mutating a caller local. No workload name, source
path, iteration count, checksum, or expected result appears in the runtime
implementation.

Against the exact unit-51 local base, the independent one-block external
inventory retained full 5/5 JetStream, 14/14 Kraken, and 26/26 SunSpider
coverage. Common-case candidate/base geometric means were **0.955718x**,
**0.886864x**, and **0.996554x** respectively. JetStream improved in all 5/5
cases; Kraken improved in 11/14, led by `audio-oscillator` at 0.299309x and
bounded by a 1.028269x worst observed ratio; SunSpider split 14/26 faster and
12/26 slower, with a 1.107301x worst single-block observation. Candidate/
QuickJS-NG remains far from B5 at 7.617x, 5.480x, and 10.169x, so this is a
general structural improvement rather than a target-completion claim.
External raw/report SHA-256 are
`4fa259b01a1642b0e60b35319bf5965970995a4ee17c10b1ee3f78924e203d9a`
and `0bd046453dee13903dde12663c35a0ee9ae2ed57fbbcd9ada8b127362c140fd4`.

The accompanying receipt-less one-block broad diagnostic retained all 25
cases. Its manual protocol-equivalent normalization measured candidate/base
at 1.005854x overall, with 11/25 cases faster and all individual ratios
between 0.990890x and 1.050665x. Candidate/QuickJS-NG was 0.186030x in this
diagnostic. The strict report tool correctly rejected the run as unverified
because development binaries had no build receipts; the complete raw JSONL
is retained only as regression evidence with SHA-256
`59244edb2b9eb5ddf39ab0ec0ad54bf7c5578dc060d7fd6c627fe540c4ff8236`.
Exact pushed performance and coverage artifacts remain required before this
unit is closed.

The pushed unit-52 closure is commit
`562c3ec57def00389f638951714713abc2b18bc2`. CI run `29635023959`, Performance
Preview run `29635023955`, and Test262 Coverage run `29635121251` all completed
successfully. The downloaded performance artifact binds the candidate to that
exact revision and contains a complete, verified 25/25-case broad run with all
225/225 measurements eligible and all three blocks valid. Hosted
candidate/base was **0.964125x** with a 95% confidence interval of
[0.960787x, 0.971370x]; candidate/QuickJS-NG was 0.343608x with a 95%
confidence interval of [0.342418x, 0.345492x]. Broad raw/report SHA-256 are
`c234eaa735a3cc6a98a3737778338a84924491e0ffcee68bc600f6379f1551a8`
and `28cf2d593d13f784b0119dec58a7396bfe1ecbf735f82b135b72836899875b2c`.

The same hosted artifact does **not** close external coverage: the fixed
15-second informational preview completed JetStream 5/5 but only Kraken 7/14
and SunSpider 23/26. All ten missing comparisons are explicit qjs-rust
timeouts (`ai-astar`, five additional Kraken audio/imaging cases, and three
SunSpider string/regexp cases), while QuickJS-NG completed them. The partial
comparable-case ratios of 10.824x, 6.400x, and 10.007x therefore remain
diagnostic only and must not be presented as complete suite results. External
raw/report SHA-256 are
`36e0575a7778a6e3ae83e6a38c10b2da8a690cdf92ba0b23b952cb7f930e4f03`
and `403244805fce920e12b1048bef24004a43fa4678da0f9e6036f225a936417574`.
The independent same-host local inventory above remains the complete 45/45
external comparison for this unit.

Coverage recorded 42,671/42,672 configured qjs-rust passes and one actionable
gap,
`test/language/expressions/class/private-static-setter-multiple-evaluations-of-class-realm.js`;
the comparison artifact confirms this is the sole QuickJS-NG-pass/qjs-rust-fail
case. Coverage burndown/comparison SHA-256 are
`25df7fae86a217e2a2e957c47c133a9b2c416ff3455cc7f339c524c515dd4bd2`
and `44cdb54200fbbebbb9353ed4f93c3c39b80a7135d20c05ed0eb6f25806bf401b`.
Unit 52 is closed as a broad structural performance improvement, not as B5 or
100% Test262 completion.

The fifty-third v2 unit removes the same compatibility-frame tax from the
generic binary-coercion fallback. A post-unit-52 macOS sample of Kraken
`ai-astar` attributed 4,054/7,150 nested samples in the slow binary path to
`apply_env`, with another 1,455 samples constructing `current_env`; caller
local snapshots and name-map reconstruction dominated both stacks. Binary
coercion hooks execute as ordinary functions and therefore already carry their
closure/upvalue cells, while globals are shared through the realm. The VM now
uses a realm-only `CallEnv` for this fallback and never snapshots or writes
back the caller compatibility environment. The fallback is `#[inline(never)]`
so its cold body does not inflate the main bytecode dispatch loop. Native
Error `instanceof` stays on the same realm-only semantic path. The focused
test proves that `valueOf` can still mutate both a captured lexical binding
and a realm global. No workload identity, source path, iteration count,
checksum, or expected benchmark result appears in the implementation.

Against the exact unit-52 same-host base, the complete one-block external
inventory retained 5/5 JetStream, 14/14 Kraken, and 26/26 SunSpider coverage.
Candidate/base geometric means were **0.952120x**, **0.904433x**, and
**0.932400x** respectively. The worst observed case was only 1.052829x base;
`ai-astar` fell to 0.231100x and `3d-raytrace` to 0.390563x. A final clean
single-case A* run took 13.33 seconds versus roughly 57.24 seconds for the
unit-52 binary. Candidate/QuickJS-NG nevertheless remains far from B5 at
7.248x, 4.929x, and 9.418x. External raw/report SHA-256 are
`ed666cc08f8e0b6319b0d863118704815f30b7435ef989ac595749fec3156f7d`
and `77fdd85a06986df5f28642c2658e11d3271d6c8cd9fd77baf000c70194a444ca`.

The accompanying receipt-less one-block broad diagnostic completed all 25
cases and all 75/75 eligible measurements. Manual protocol-equivalent
normalization measured candidate/base at 1.007021x overall with 17/25 cases
faster, and candidate/QuickJS-NG at 0.187913x. The call and allocation family
ratios were 1.024921x and 1.024799x; all other families were at or below
0.999821x. The worst case, `dynamic_method_call`, was 1.179716x and remained
1.171988x in an independent five-block focused rerun; `closure_allocation_call`
was 1.098205x and 1.097293x respectively. These measured layout side effects
remain below the 1.25 per-case guardrail but are explicit debt for later units,
not evidence that the final critical-family requirement is met. Broad and
focused raw SHA-256 are
`080f51470c22473f0f7fdf062668c45c77a89edde05cea625614c2e6bd086132`
and `e2b4b43a2590f26e792c502e3877b0214416cc9d8ba56b6d9af0266898528ea9`.
The strict report tool correctly rejects both development runs as unverified
because they have no build receipts. Exact pushed performance and coverage
artifacts remain required before unit 53 is closed.

The pushed unit-53 closure is commit
`188033532d183934b3e898f5480aa887ed0c9346`. CI run `29637597458`, Performance
Preview run `29637597433`, and Test262 Coverage run `29637684611` all completed
successfully. The downloaded performance artifact binds the candidate to that
exact revision and contains a complete, verified 25/25-case broad run with all
225/225 measurements eligible and all three blocks valid. Hosted
candidate/base was **0.999398x** with a 95% confidence interval of
[0.999037x, 1.002281x]; candidate/QuickJS-NG was 0.348981x with a 95%
confidence interval of [0.346488x, 0.353847x]. Broad raw/report SHA-256 are
`aeafc5568565038168f470e64bfbc558ce1692c9542cd2285840e59e92439bad`
and `72654f2c5d0ede4c7909e79164b9f8d6becc0ae8222525979abdad0b33f1fefe`.

The hosted external preview remains incomplete under its fixed 15-second
limit: JetStream completed 5/5, Kraken 7/14, and SunSpider 23/26. The ten
missing candidate results are explicit timeouts, so no incomplete-suite ratio
is promoted to an aggregate claim. On the unchanged comparable inventory,
however, all three diagnostic geometric means improved over unit 52:
JetStream moved from 10.824x to **9.993x** (0.923173x unit 52), Kraken from
6.400x to **6.028x** (0.941909x), and SunSpider from 10.007x to **9.166x**
(0.915984x). This agrees directionally with the complete same-host 45/45 local
comparison and is accepted as generalization evidence, not B5 completion.
External raw/report SHA-256 are
`fce00a26b28013cecd4af0d6999b1a3a207871de94f6718e786985084d6cbd20`
and `9fd71f44b1859767eda82165a79e48eb98d61f1c7dde4e07bdd8be7e379fd9ba`.

Coverage remained 42,671/42,672 configured qjs-rust passes with zero timeout
and the same sole actionable private-static-setter gap. Coverage
burndown/comparison SHA-256 are
`44ca082b6f7adb754ce4458c929e88c09506fcae46c1f46c896e7c8527631240`
and `44cdb54200fbbebbb9353ed4f93c3c39b80a7135d20c05ed0eb6f25806bf401b`.
Unit 53 is closed as an externally corroborated general runtime improvement,
not as a complete external run, B5, or 100% Test262 completion.

A post-unit-53 prototype extended the exact named-property inline cache from
primitive results to weakly held object results. The mechanism was semantic
and workload-independent: it reused only the same receiver identity and
property revision, did not retain the returned object, and invalidated after
receiver mutation. It also reduced the sampled `ai-astar` property-lookup
stack and improved that case to 0.971996x of the unit-53 base. The complete
same-host external inventory nevertheless rejected it: JetStream was
**1.009939x** base with all 5/5 cases slower, Kraken was **1.005941x** with
only 4/14 faster, and SunSpider was **1.008835x** with 10/26 faster. The
worst one-block observation was 1.112912x. Candidate/QuickJS-NG remained
7.259x, 4.976x, and 9.554x. External raw/report SHA-256 are
`88090e5668fd6a1d8d667d9d611e1ec8d08c00b12d412e67e0b0880561946858`
and `a89aad7fed7558559c4fe70387c4fb9ae09efc7e70154e76df5cffab5e9af6a0`.
The prototype was reverted without a runtime commit: one improved external
case does not outweigh three suite-wide regressions under the anti-overfitting
contract.

The fifty-fourth v2 unit follows the independent SunSpider regexp/string
profile into general UTF-16 regexp preparation. macOS sampling of
`string-unpack-code` showed that every non-Unicode pattern and input code unit
allocated a temporary Rust `String` only to extract its first `char`. The
runtime now maps a code unit directly to the existing internal character
representation: ordinary code units retain their scalar value and lone
surrogates retain the established sentinel encoding. `push_code_unit` shares
the same conversion helper, and the focused test covers both ASCII and lone
surrogate round trips. The implementation contains no suite or case identity,
source path, iteration count, checksum, or benchmark-specific result.

Against the exact unit-53 same-host base, the complete one-block external
inventory retained 5/5 JetStream, 14/14 Kraken, and 26/26 SunSpider coverage.
Candidate/base geometric means were **1.006037x**, **1.008965x**, and
**0.726930x** respectively, with 2/5, 6/14, and 12/26 cases faster. The small
unaffected-suite movements are within the adjacent one-block host drift: the
immediately preceding rejected property-cache prototype measured 1.009939x
and 1.005941x against the same base. The affected independent cases improved
by an order of magnitude: `regexp-dna` fell to 0.075346x base,
`string-tagcloud` to 0.059895x, and `string-unpack-code` to 0.049642x.
Independent three-run direct timings were 3.52--3.55 seconds, 2.62--2.63
seconds, and 4.40--4.51 seconds respectively, versus unit-53 one-block
durations of 46.68, 42.81, and 89.21 seconds. Candidate/QuickJS-NG remains far
from B5 at 7.317x, 4.976x, and 6.765x. External raw/report SHA-256 are
`2531f93d84e9cbba106c32f10879c89697b01c79c587e3e0f4acef42eaa4c321`
and `b41aab481a0b0d3f28b13ba45a47ad849fd7df35f426cd9c0b10ab93b0d97c32`.

The receipt-less one-block broad diagnostic completed all 25 cases and all
75/75 eligible measurements. Manual protocol-equivalent normalization
measured candidate/base at **1.006793x** overall with 10/25 cases faster and
candidate/QuickJS-NG at **0.189055x**. The string family was 0.983693x base;
unaffected families ranged from 1.001059x to 1.026644x in this single-block
run. The raw JSONL SHA-256 is
`9007098682c619de1113da5c1ab37315c188717087729f70fc9cdcb9ae3d3d8a`.
All 1,392 runtime tests, the full workspace and benchmark-tool gates, the
5,139-case Test262 subset, and 205 QuickJS-NG comparison fixtures passed.
Exact pushed performance and coverage artifacts remain required before unit
54 is closed; this local result is accepted as external generalization
evidence, not B5 completion.

The pushed unit-54 runtime is commit
`549486dfd6973fe0fd2efedcaf860b160365e3be`; the trusted harness revision is
`c3d65ad3d36362f105c31404bc7137f37dedcf01`. CI run `29640021376`,
Performance Preview run `29640021343`, and Test262 Coverage run `29640099734`
all completed successfully. The downloaded performance artifact contains a
complete verified 25/25-case broad run with 225/225 eligible measurements,
passing linearity, and three valid blocks. Hosted candidate/base was
**0.983314x** with a 95% confidence interval of [0.982050x, 0.999283x];
candidate/QuickJS-NG was **0.344955x** with a 95% confidence interval of
[0.335483x, 0.349178x]. Broad raw/report SHA-256 are
`17379b13dda3513f99225bcb6750b1f5e31a88ffdf0800db109faa210f70106e`
and `57bb53ae67ac9375f49582120460e90a1a92da584afecc1bdbb09f2f593a9612`.

The hosted external preview completed all 5/5 JetStream and, for the first
time in this campaign, all 26/26 SunSpider cases; Kraken remained partial at
7/14. On the unchanged unit-53 comparable inventories, candidate duration
geometric means improved to 0.987196x, 0.975203x, and 0.990279x respectively.
The three newly completed SunSpider cases are exactly the profiled mechanisms:
`regexp-dna`, `string-tagcloud`, and `string-unpack-code`. Because their large
remaining QuickJS-NG gaps now enter the aggregate, the complete SunSpider
candidate/QuickJS-NG ratio is **10.979076x**, not a B5 pass; JetStream was
9.925689x and partial Kraken was 5.869854x. External raw/report SHA-256 are
`19cafd80bd7038845d7a73f94b0f4779e24147e0c13c38d9bffc2a9dd5898a3b`
and `dd8582196f826522518a4ab1ee89bb2deedb42d6d899b4ae72a96132616c0384`.

Coverage remained 42,671/42,672 configured qjs-rust passes with zero timeout
and the same sole actionable private-static-setter gap. Coverage
burndown/comparison SHA-256 are
`efb8ed34e77a5341952f0850fdd63b4dca90393d17d9275922c0f2f5adf48309`
and `44cdb54200fbbebbb9353ed4f93c3c39b80a7135d20c05ed0eb6f25806bf401b`.
Unit 54 is closed as a general regexp allocation improvement that increased
external coverage, not as B5 or 100% Test262 completion.

### Unit 55: lazy regexp search starts

Runtime commit `a5e39997fb640425a2902a2b08e204bc3eecd7ca` removes the
`Vec<usize>` that `regexp_match` previously allocated and filled with every
candidate start position before attempting a non-anchored match. It now walks
the same inclusive range lazily and stops at the first match. This is a general
engine allocation and memory-traffic reduction: it is independent of workload
identity, expected output, iteration count, and benchmark path.

The first broad A/B attempt was rejected because its nominal base executable
came from a stale main-worktree `target/release/qjs`. The accepted same-path
rebuild binds candidate SHA-256
`30d55d46f68bd358be52072a6d81a9e6095b621778075c546ba749becc1befd7`
and unit-54 base SHA-256
`e26e0b9478797e027ac36f0db213ecc6bb3037a06c73b23a58587e4705d509c1`.
The final source rebuild reproduced the candidate hash byte for byte. Its exact
25/25-case, 50/50-eligible two-role broad run measured candidate/base
**1.003574x**. All family ratios stayed below 1.026x; the largest was string at
1.025469x. The raw SHA-256 is
`3e018eb528d3a0eb0d8450cbe37478f98c82fb658528187fafd1ebbbd54c5600`.
A separate five-block call-family audit measured 0.993581x overall, with all
four cases between 0.965410x and 1.009943x; raw SHA-256 is
`91e5adc5a1057a1dc6098d3f823578f5a7448095d0ab6b6e41e55e9c772e4311`.

Two formal 60-second external runs each completed 44/45 cases. Both timed out
only on Kraken `imaging-gaussian-blur`; an isolated run completed in 48.73 s,
so this remains a real near-limit external hotspot rather than being hidden or
reclassified. On the second run, unit-55/unit-54 duration geometric means on
the comparable inventories were 0.996167x for JetStream, 0.999839x for Kraken,
and 0.918827x for SunSpider. Raw/report SHA-256 are
`dc2e67eb2546906396c4983a73f693275a0950bd28e1a7b5b33d62c243e264a9`
and `bb7d799e253f3206c53debd545174844e781182b1b55e5ec5931b3778b7996c5`.

A separately labelled 90-second diagnostic completed all 5/5 JetStream,
14/14 Kraken, and 26/26 SunSpider cases. Relative to unit 54, complete-suite
duration geometric means were 0.998322x, 0.994281x, and 0.916457x. The directly
affected SunSpider ratios were 0.585879x for `regexp-dna`, 0.502985x for
`string-tagcloud`, and 0.457737x for `string-unpack-code`. Candidate/QuickJS-NG
remains **7.231091x**, **4.971088x**, and **6.322610x** respectively, so the
external evidence confirms useful generalization but also a large remaining
performance gap. Diagnostic raw/report SHA-256 are
`911c89f960c2685e079802a73393ab76c121316dbc715642182cc39ac81d318f`
and `a09bb528d3f8b93cec46be0fe55e9e71e1c1a46f1fcf7c12774dab000d27f45a`.

Local correctness gates passed 1,392 runtime tests, the full repository check
including all 198 benchmark-tool tests and 5,139 Test262 subset cases, and all
205 QuickJS-NG comparison fixtures.

The pushed unit-55 runtime and documentation are commits `a5e39997` and
`9f88688c`. CI run `29642617673`, Performance Preview run `29642617675`, and
Test262 Coverage run `29642717096` all completed successfully. The hosted
three-block broad artifact was complete and healthy, but it did not confirm
the locally neutral movement: candidate/base was **1.025615x** with a 95%
confidence interval of [1.024365x, 1.028105x]. Binding was **1.100406x** and
call was **1.047362x** base; candidate/QuickJS-NG remained **0.370231x**.
Broad raw/report SHA-256 are
`94a7ff86ec7d85ed21eb1c0945bd19b46cf226682c9fb06a4f44659ed872c938`
and `c165359d95310b49fd33cd3b2a371488c9544e7899a122b4417a2e51a11270d5`.

The hosted external preview completed 5/5 JetStream, 7/14 Kraken, and 26/26
SunSpider cases. Candidate/QuickJS-NG was **10.394x**, **6.815x**, and
**10.913x** respectively, with zero qjs-rust wins; Kraken
`imaging-gaussian-blur` remained the sole timer-limited case from the complete
local 90-second inventory. External raw/report SHA-256 are
`7bfee955fdfaab3b42a2f578a7fb9a359f04376d0b4a8c0dc0e9c0d56f634c1a`
and `16e4a11c475c0b34ae451f0ce8a6a431f7726be7426f14547ef17195afa5992b`.
Coverage stayed 42,671/42,672 with the same private-static-setter gap;
burndown/comparison SHA-256 are
`5f0bc2e506fdec3476e7aa5f8bd5460a1a6888a24f83ff4fd5aabc7981323266`
and `44cdb54200fbbebbb9353ed4f93c3c39b80a7135d20c05ed0eb6f25806bf401b`.
Unit 55 is closed as fully verified evidence for a general lazy-regexp
mechanism, but its hosted broad regression means it is not counted as an
internal broad improvement and does not advance B4 by itself.

### Unit 56: read-only global captures through realm cells

Runtime commit `35c3f33e` lets statically known, read-only top-level `var`
references flow through shared realm cells and ordinary local/upvalue slots.
Writers remain name-addressed global operations, nested readers reuse the same
cell, and any function containing direct `eval` stays on the dynamic name path.
Numeric-loop admission was generalized to consume authoritative realm slots,
including reordered calls, rather than adding a workload-specific exception.
The implementation contains no benchmark identity, source path, iteration
count, checksum, or expected-result specialization.

Two prototypes were rejected before acceptance. The first made broad
candidate/base **2.3137x**, including `global_read` at 167.94x, because the
general numeric-loop path did not understand the new slot representation. A
second fixed stable reads but left reordered calls at 57.18x. Neither result is
counted. The final implementation also fixed two correctness boundaries found
by the full gates: direct-eval functions are explicitly deoptimized, and the
string-append path releases its realm borrow before synchronizing a captured
global. Both have focused regression tests.

The final candidate binary SHA-256 is
`fc01c52272370d48ee71caa6963ce88a0cc649b8c983c8ee04db82c1518decdf`.
Its complete 25/25-case, 50/50-eligible broad A/B measured candidate/base
**0.998691x**. Binding improved to **0.974028x**; the other seven families
ranged from 0.998004x to 1.010294x. Broad raw SHA-256 is
`2f7a32bc659db059cf0f0152600ae56cfe3c2454dda5baaa03b2745faf0657ef`.
An independent five-block call audit measured **0.996854x** overall:
`plain_function_call` 0.993997x, `closure_allocation_call` 0.997818x, and
`empty_loop` 0.998752x. Its raw SHA-256 is
`dc7bd42642c4a2dd1d2c8ecc0410e609fc67905344c56ee641613dc039ef3857`.

The exact final external binary completed all 45/45 cases at the formal
60-second timeout, including Kraken `imaging-gaussian-blur` at 39.2945 s.
Against the unit-55 common local inventory, duration geometric means were
1.001862x for JetStream, 1.002953x for Kraken, and 0.988179x for SunSpider;
the newly completed Gaussian case is the material generalization result.
Candidate/QuickJS-NG is still **7.344830x**, **4.912244x**, and **6.277423x**
for the three suites, with zero qjs-rust wins. External raw/report SHA-256 are
`55dea2ad176657d2eeacbc161ea36badba91fa84e1fef65f45243e5897b91534`
and `0433284651053a783219241b06029123b67e834536a6c43f96fca7ce0f2d20c1`.
All 1,395 runtime tests, the full workspace and 198 benchmark-tool tests,
5,139 Test262 subset cases, and 205 QuickJS-NG comparisons pass.

The pushed runtime and goal-contract documentation are commits `35c3f33e` and
`f3b7d1a6`. CI run `29646643893`, Performance Preview run `29646643882`, and
Test262 Coverage run `29646745719` all completed successfully. The hosted
three-block broad run was complete with 225/225 eligible measurements and
75/75 passing linearity probes. Candidate/base was **0.985002x** with a 95%
confidence interval of [0.982929x, 0.985761x]; binding was 0.973778x, call was
0.979214x, and candidate/QuickJS-NG was 0.354042x. Broad raw/report SHA-256 are
`438365cf7d51d6dcd2d6ddcd13c77be66c68ed46538f20d8f14e93c32391c2d8`
and `9bb019f2f67857a3c5aa74d9dec880c863d2971ba48c2aacd744b324fbc7e80f`.

Under the hosted preview's fixed 15-second external timeout, JetStream and
SunSpider remained complete at 5/5 and 26/26 while Kraken expanded from the
unit-55 hosted 7/14 to 9/14. The five remaining Kraken timeouts are
`ai-astar`, `audio-beat-detection`, `audio-fft`, `imaging-gaussian-blur`, and
`imaging-desaturate`; all complete locally with the formal 60-second timeout,
so none is hidden or reclassified. Hosted candidate/QuickJS-NG was
**10.194558x**, **6.791710x**, and **11.031447x**, with zero qjs-rust wins.
External raw/report SHA-256 are
`3b926d7caabf7b7a030cd0e6528232d14f6f99782aba2fd3173badf826345178`
and `da3ad19b43acd2773ebc718841bb884734eba5f20ab4175484f2d78199ef9b4a`.

Coverage remained 42,671/42,672 with zero timeout and the same sole
private-static-setter actionable gap. Coverage burndown/comparison SHA-256 are
`e9d23b6e280470f1d7b2743950b64920af18f53d2e06dc21a2aca54a4cbc7a6a`
and `94a97687babcdfba02efff77997af0a59fa7ddfb08a2370d03fbb933de069e69`.
Unit 56 is closed as a fully verified general environment-representation
improvement. The goal remains open because every comparable hosted external
case still loses and external throughput remains roughly 6.8--11.0x slower
than QuickJS-NG on the hosted inventory.

### Unit 57: copy-on-write eval conflict metadata

Runtime commit `06c96adf` changes per-function `catch_bindings` and
`direct_eval_var_conflicts` metadata from owned `HashSet<String>` values to
shared `Rc<HashSet<String>>` values. Ordinary call frames now clone two
reference-counted pointers instead of cloning both hash tables; the uncommon
paths that discover new catch or direct-eval conflicts preserve isolation with
`Rc::make_mut`. This is a general call-environment representation improvement:
it applies independently of benchmark identity, source path, iteration count,
checksum, or expected result. A focused test verifies the copy-on-write
boundary between parent and child frames.

An earlier prototype that reset both sets to empty in child frames was
rejected. Although it helped closure and dynamic-method calls, its five-block
focused aggregate was **1.016x** base and `top_level_function_call` regressed
about 22.7%. The accepted representation keeps the inherited semantics and
eliminates the common clone cost instead of assuming that the metadata is
irrelevant.

The accepted candidate and frozen unit-56 base binary SHA-256 values are
`4d50f538db4c3b71ca19fa1d3f0c41a72073d61663318090d801f89b7d678539`
and `fc01c52272370d48ee71caa6963ce88a0cc649b8c983c8ee04db82c1518decdf`.
A five-block affected-and-control audit measured **0.970893x** overall:
`closure_allocation_call` 0.929553x, `dynamic_method_call` 0.944619x,
`plain_function_call` 1.002990x, and `top_level_function_call` 1.008922x.
Its raw SHA-256 is
`43862921ba129750d3d7f9aa4361b20d221d373ef5436bdc36bec8595c5abf5f`.

The complete 25/25-case, 50/50-eligible one-block broad run measured
candidate/base **0.998799x**. Allocation was 0.983529x and call was 0.995145x;
the apparent one-block string, empty-loop, and plain-call regressions were
audited separately rather than ignored. The broad raw SHA-256 is
`c65be083bfdd7c95d9b4d94e76ae5eb8dfe4f053fcd1b08854f3d34500a16c36`.
That independent five-block control rerun measured **0.994921x** overall,
with `object_allocation` 1.012008x, `plain_function_call` 0.993994x,
`empty_loop` 0.992101x, and `string_concat` 0.981817x. Its raw SHA-256 is
`638356598cd4aa32d787d735412715fc63266e32c033239da119c2a375d51692`.

The exact 60-second external run completed all 45/45 cases. Relative to the
exact unit-56 local inventory, candidate duration geometric means were
1.010063x for JetStream, 0.983659x for Kraken, and 0.984789x for SunSpider;
10/14 Kraken and 20/26 SunSpider cases improved. Candidate/QuickJS-NG remains
**7.443087x**, **4.861760x**, and **6.273050x**, respectively, with zero
qjs-rust wins. This satisfies the per-unit external-generalization audit but
is far from the campaign target. External raw/report SHA-256 are
`700b34a4e2e85ed8640d66e869f3eb1c6e76054d1a98b3286776e1443c688912`
and `f7eeffe1be41d80e6c5197b09d0ffd178b44641d7d219b8277852a270d65b134`.

Local correctness gates passed all 1,396 runtime tests, 67 focused eval/catch
Test262 cases, the staged gate including 49 function Test262 cases, all 205
QuickJS-NG comparisons, and the full repository check including 5,139 Test262
subset cases. Hosted CI, broad, external, and coverage artifacts are still
required before closing this unit. The goal remains open: the general call-path
improvement is real, but external suites are still 4.86--7.44x slower than
QuickJS-NG and have no qjs-rust wins.

The pushed runtime and local-evidence commits are `06c96adf` and `27be05d4`.
CI run `29649223133`, Performance Preview run `29649223064`, and Test262
Coverage run `29649333891` all completed successfully. The hosted three-block
broad artifact was complete with 225/225 valid measurements, 75/75 passing
linearity probes, and three valid blocks. Candidate/base was **0.993178x**
with a 95% confidence interval of [0.991214x, 0.994007x], while
candidate/QuickJS-NG was **0.356623x**. Call was 0.950335x base; the other
families ranged from 0.920431x builtin to 1.043543x binding. Broad raw/report
SHA-256 are
`785adae20fca07c0c8b5ec16cd11fb48cc522aee1b8195e0f0cffb8328beed12`
and `04926a78c782b491df60794f4b05e8ae9e5ee67f0c72e0dce7b054e57ab9a5e1`.

The hosted fixed-15-second external preview completed 5/5 JetStream, 9/14
Kraken, and 26/26 SunSpider cases. Candidate/QuickJS-NG was **10.778x**,
**7.940x**, and **11.382x**, with zero qjs-rust wins. QuickJS-NG wall times
moved enough that these cross-engine ratios are worse than unit 56, so the
same-engine candidate-duration comparison is recorded separately rather than
mislabeling reference-runner drift as a runtime regression. On the common
hosted inventories, unit-57/unit-56 candidate duration was 0.968086x for
JetStream, 1.022976x for Kraken, and 0.975161x for SunSpider; the exact local
45/45 run independently measured 1.010063x, 0.983659x, and 0.984789x. The
Kraken direction therefore remains runner-sensitive, while two suites and the
complete local inventory confirm external generalization. Hosted external
raw/report SHA-256 are
`6464ac4180d253a9b537bb6e550c6967380ade84ca9b3640bcdfb131d6be1f47`
and `d0c2733247e17128f758ac00beb3570d56d4337cbc4a9458c2f7a617a43b0e3d`.

Coverage remained 42,671/42,672 with zero timeout and the same sole
private-static-setter actionable gap. Coverage burndown/comparison SHA-256 are
`579364989fc544ecefe302bf9094393fedae9084c9a34f115459049a11f9cdae`
and `94a97687babcdfba02efff77997af0a59fa7ddfb08a2370d03fbb933de069e69`.
Unit 57 is closed as a fully verified general call-environment representation
improvement, not as B5 progress. The campaign remains open because every
hosted external comparable case still loses and the suite-level diagnostic
ratios remain 7.9--11.4x QuickJS-NG.

### Unit 58: rejected property-hashing experiment

This unit is deliberately recorded as a rejection, not performance progress.
Runtime commit `0000faf8` replaced SipHash-backed hot object-property and shape
maps with keyed `AHashMap`. Local Apple-silicon measurements initially looked
promising: the complete one-block broad run was 0.997777x base, a five-block
focused audit was 0.990289x, and `dynamic_method_call` was 0.950558x. A local
external inventory measured unit-58/unit-57 candidate durations at 0.930623x,
0.988313x, and 0.986173x for JetStream, Kraken, and SunSpider. Those external
numbers compared separate runs, however, so they are now classified as
diagnostic rather than acceptance evidence. Their raw/report SHA-256 are
`337d9270c725c56bfe0abf4e341a09ae06270b472dbf8e72342ac67797ccec97`
and `ed8b737bffdc91718c14d1ad8c5308845333ab35c7178100bb231f13309eccd8`.

Two repetitions of hosted Linux run `29651613063` contradicted the local
result. The unconditional candidate/base broad ratio was **1.013641x** with a
95% confidence interval of [1.013640x, 1.016890x] on the first attempt and
**1.018547x** with [1.016832x, 1.018626x] on the second. Binding regressed to
1.04987x/1.04957x base, builtin to 1.04048x/1.03955x, and array to
1.02012x/1.01928x. The raw/report SHA-256 pairs are
`c749ae5beb38fc25584fe32f7bdb82643ce4eae98d729e4fe209ce12ef5516a3` /
`8a605622b2c2f3ae10c4fede2bade8afcfd5aa2b62c95f02937dea4972f8cc3d`
and
`07c7c7b087cdb934b6def22fae4cd425c84bc43e9247b30079a4bfe12ab16b0e` /
`a3b189cac909bbd64e7a7ba84c9aa94c1cb87e18740ce2d5bd35299a0b83bf`.

Commit `7a0af070` then limited AHash to builds with the AES target feature. That
retained local ARM gains, including a 0.787307x direct candidate/base result on
JetStream `gaussian-blur`, but hosted run `29653501977` was still **1.002847x**
base with [1.000751x, 1.006010x]. Call, control, and property regressed to
1.053123x, 1.020426x, and 1.005379x. Its external preview still had zero
qjs-rust wins and was 9.887x, 5.692x, and 10.394x QuickJS-NG. Broad raw/report
SHA-256 are
`d406191e627f618e34911aa652cca3921a777e8ed15051c9c6cb86f14d60391f`
and `8eb942700c49bc8e6e4fb7df02f279b9071341b03eb9f7a071d1ce3231ffb5d4`;
external raw/report SHA-256 are
`e8f978e05a926b70003d8ee09d8acd7cf32e120332afb5eebc344ef72e15ea56`
and `02374816acce24565efbbec7bdf9484593e56a9288cb3065bdcfba49ed277a18`.

Commit `0fdf1b53` therefore fully reverted both hashing commits and the new runtime
dependency. `Cargo.lock`, `qjs-runtime/Cargo.toml`, and object storage are byte
for byte identical to the closed unit-57 state. Local verification passed the
complete repository check, including all 198 benchmark-tool tests and 5,139
Test262 subset cases. Hosted CI run `29655047865` and coverage run
`29655165753` succeeded; coverage remained 42,671/42,672 with one failure and
zero timeout. Coverage burndown/comparison SHA-256 are
`d8537e57ba916b730e3eb768a4eb33639bc5c4cfe5153f96eda45f63750ff33d`
and `94a97687babcdfba02efff77997af0a59fa7ddfb08a2370d03fbb933de069e69`.

Hosted Performance Preview run `29655047930` also succeeded. Its restored
candidate binary SHA-256,
`bc35d9cb2f0ac0a6bcf07c1bcf8b9f94b13ac4c6991feedd38b940f557e2590a`,
exactly matches the unit-57 hosted binary. The broad candidate/gated-base ratio
was 0.984800x with a 95% confidence interval of [0.981700x, 0.988875x], and
candidate/QuickJS-NG was 0.346714x. The old two-role external protocol still
reported zero qjs-rust wins and 10.089x, 5.858x, and 10.616x QuickJS-NG for
JetStream, Kraken, and SunSpider; it cannot establish a candidate/base external
direction. Broad raw/report SHA-256 are
`0d33542f438784d8645b6555dbb7135bbbd3c50a613203ac64fd557869b189b1`
and `a6442b284a6343f1c78033c7ec1231ba3f64c6984c1c159772592536f73edaf0`;
external raw/report SHA-256 are
`021425144e661807ba859da5820e94c71245272c646d9cb20b92f34475185019`
and `d22e2406dc4137189a2ec91daf5f7bf972bbab9652801c00b1fc02b73e43a328`.

A separate realm-binding `AHashMap` prototype was also rejected before commit.
Its focused eight-case run was 0.991944x overall, but
`dynamic_method_call` regressed to 1.025535x and a seven-block direct external
JetStream Gaussian comparison regressed to 1.019049x. The focused raw SHA-256
is `b32d5f7a21bcd1ca11c35bc1aa815598dd16fe7a940a2640a29e1916909be1ad`.
Together these failures demonstrate why a portable, same-run external base is
mandatory: an architecture-specific internal win is not a general engine
optimization. Unit 58 contributes rejection evidence only; B3, B4, and B5
remain open.

### Unit 59: rejected user-call environment writeback shortcut

This unit is also recorded as a rejection, not performance progress. Commit
`63dbd768` skipped the environment writeback after a user-function call when
the callee could not mutate any name-addressed binding. The implementation was
a general call-frame mechanism with no benchmark IDs or checksum branches, but
mechanism-level generality alone is not sufficient: acceptance still requires
portable broad and external evidence without trading one workload family for
another.

Hosted Performance Preview run `29657953046` compared the exact candidate
binary `2075c3fceca53cfa09fb68342fdbc3b089888c27e223a991f0ac03dbca8f2578`
against the exact protocol-baseline binary
`bc35d9cb2f0ac0a6bcf07c1bcf8b9f94b13ac4c6991feedd38b940f557e2590a`.
The unconditional broad candidate/base ratio was **0.998752x**, but its 95%
confidence interval of [0.997096x, 1.005531x] crossed 1.00x. The focused hosted
call shapes were mixed: `function_call_two_args` improved to 0.919801x and
`top_level_function_call` to 0.972439x, while `array_index_of` regressed to
1.046024x and `closure_allocation_call` to 1.041305x. Broad raw/report SHA-256
are `e4d31669504e9e71d8ed6f65ca0226cdfc847c8e21053d72669840a029ef06b3`
and `6fc7d3a5faa29ec412b36e361b0dada2b60116d877341af102ff7602958ffd1d`.

The same hosted run did show small same-run external candidate/base gains:
0.997402x across 5/5 comparable JetStream cases, 0.989929x across 6/14 Kraken
cases, and 0.998013x across 26/26 SunSpider cases. Those results are evidence
about this Linux runner, not a sufficient acceptance result. An independent
five-block Apple-silicon audit did not reproduce either hosted call win:
`function_call_two_args` was 1.00194x and `top_level_function_call` was
1.00404x, while `array_index_of` and `closure_allocation_call` were 1.00057x
and 0.99641x. Its raw SHA-256 is
`dbfe162ed4a0b9a738ba88babfe701e432bfb8df5d232b501a5bbb34c5a44dc9`.
Hosted external raw/report SHA-256 are
`855bbbc4c8c30eb3249ab6034295b66cbe832d960f8983947add9a87fdb56d16`
and `e40aebaa1ecb4dee0ebca66f36d0fcc626a61c3d435126c1757fdf81bafcd42d`.

Commit `36ae2dea` therefore reverted unit 59. Hosted CI run `29659400697`,
coverage run `29659492656`, and Performance Preview run `29659400691` all
succeeded. Coverage remained 42,671/42,672 with one actionable gap and zero
timeout. The restored candidate binary again has SHA-256
`bc35d9cb2f0ac0a6bcf07c1bcf8b9f94b13ac4c6991feedd38b940f557e2590a`.
The reverse broad comparison was 0.997875x with [0.991715x, 1.008582x]; the
reverse external ratios were 1.014675x, 1.001979x, and 1.011904x for
JetStream, Kraken, and SunSpider. Revert broad raw/report SHA-256 are
`2bb7d7b63434a9c237b7f8fcdebe956d80b67eb688c354d2ede5a4181d4e1d7a`
and `6ded17c0893a6506af7a739a648953bf36712f98fc4be70766eaf2de36d060f3`;
external raw/report SHA-256 are
`d628310f9eaf3f3aa1a4d15113b7507484f8cd7322e65c1fa0760a8369ea8113`
and `ba19d4de8c746554cb5f9fa88b4f03cbcf2b5a6a18b4ed5afebfc3effdc13243`.
Unit 59 is closed as rejection evidence: a narrow hosted win that is not
portable and introduces other regressions cannot count as general engine
optimization.

### Unit 60: accepted small-object property storage with recorded broad tradeoff

Commit `5a0c9cff` replaces the always-hashed property table for ordinary objects
with a general two-tier representation. Objects retain up to eight own
properties in an insertion-ordered small vector and promote to the existing
SipHash-backed dynamic map on the ninth property. Large and adversarial-key
objects therefore retain the existing hash-flooding protection; shaped object
literals retain their shaped representation. The mechanism contains no
benchmark identity, source-path, iteration-count, or checksum condition. It
preserves descriptor behavior, numeric-key-before-string enumeration,
delete/reinsert ordering, and the existing `ObjectData` size bound.

The complete hosted three-role Performance Preview run `29660991986` compared
candidate revision `5a0c9cff7ebcfe12abbe918fc0940fdc9621dde4` with exact base
revision `36ae2dea5b03eaafe610a2eb6a57f560b30ba4c7`. Their executable
SHA-256 values are
`ed47cbf63c66bb4b9b844abeb2a3b3b1c91d70c22bdd3603b6947a0a6ebb1192`
and
`bc35d9cb2f0ac0a6bcf07c1bcf8b9f94b13ac4c6991feedd38b940f557e2590a`;
the pinned QuickJS-NG executable remains
`8614a5a91e3476db1a1300b0969387b85e0716a836f799cf243a80d4d1f27699`.
All 75 linearity diagnostics passed and all three blocks were valid.

The hosted broad result records a real tradeoff rather than an across-the-board
win. Candidate/base was **1.008574x** overall with a 95% confidence interval of
[1.004889x, 1.010198x]. The directly relevant allocation family improved to
0.979045x base, while call and string improved to 0.994284x and 0.953584x.
Array and property were effectively flat at 0.998750x and 0.999531x. Binding,
control, and builtin moved to 1.022082x, 1.001362x, and 1.134355x; in
particular, the unrelated `math_abs` and `array_index_of` cases measured
1.209817x and 1.063600x base. These regressions remain follow-up liabilities
and are not hidden by the aggregate.

Candidate/QuickJS-NG was **0.359801x** overall. Seven families remain below
QuickJS-NG, but allocation is still 2.726752x, so B4 is not complete despite
the overall ratio being below 0.50x. Broad raw/report SHA-256 are
`6a53496552b88499f36340964ace32ce1ba2a73c9850295e1babefc0d90459e0`
and `1a0f12d45d9831f2c14725916bce9a1f7d87e629108dccd8afff367575025ae0`.

The mandatory same-run external evidence moved in the desired direction on
all three independent corpora:

| Neutral shell port | Comparable | Candidate / base | Candidate wins vs base | Candidate / QuickJS-NG |
| --- | ---: | ---: | ---: | ---: |
| JetStream 3 JavaScript subset | 5/5 | 0.920256x | 4/5 | 9.829701x |
| Kraken 1.1 | 7/14 | 0.972240x | 7/7 | 6.702010x |
| SunSpider 1.0 | 26/26 | 0.995071x | 16/26 | 11.185109x |

An independent Apple-silicon one-block diagnostic also moved all three suites
in the same direction: 0.907655x, 0.989059x, and 0.991116x base over 5/5,
13/14, and 26/26 comparable cases. Its external raw/report SHA-256 are
`0ca30ac9b6048284b8dd4b42c18ddc8a41fe3696a52ac3ff2e1d38c5b037c3ec`
and `c8b61f62ab9fb09a55caf390e5014f876caa4058c37516f5aa4dc90e4e0b9e25`.
The corresponding local broad diagnostic was 0.987344x base, but it had only
one block and no formal build receipts, so it is not used to override the
hosted broad regression; its raw SHA-256 is
`e29ddba79a4b5f1e47cb8d9ce6477dcb69acbdb99179845ebbd91227034da109`.

Hosted external raw/report SHA-256 are
`5bf9a8b5ab0d02476711aa17196c591b0d17a1854dd9613274eebfd5499f9fc1`
and `5d74ccbee15fc4fee2272a797853865d7ed4b3623088179e323d535af07bd7df`.
Every external comparison against QuickJS-NG still loses, so this unit is not
B5 progress and makes no official upstream-suite score claim.

Focused small-storage tests cover promotion, numeric enumeration, and removal
ordering. The complete local gate passed 1,399 runtime tests, all 198 benchmark
tool tests, and 5,139 Test262 subset cases; QuickJS-NG comparison smoke tests
also passed. Hosted CI run `29660991977` and Test262 Coverage run
`29661093222` succeeded. Coverage remained 42,671/42,672 with one actionable
gap and zero timeout; burndown/comparison SHA-256 are
`7e772d8063ea61126d77e01d9bca883db866fdc038f9914d10897156b112e311`
and `94a97687babcdfba02efff77997af0a59fa7ddfb08a2370d03fbb933de069e69`.

Unit 60 is accepted as B3 campaign progress because a general representation
change produced repeatable, same-direction improvements on all three external
corpora and improved the sole remaining above-ceiling broad family. It is not
accepted as an internal-benchmark win: the hosted broad regression is an
explicit cost, and subsequent work must recover the unrelated binding and
builtin movement without giving back the external gains. The campaign remains
open with zero external wins against QuickJS-NG.

### Unit 61: accepted prepared native RegExp replacement state

Commit `49be51dd`, merged as `e5b1a262`, removes repeated setup work from the
builtin global `RegExp.prototype[Symbol.replace]` path. A prepared matcher now
retains the normalized pattern, capture indices, property cache, top-level
alternatives, match options, and input code units across the global exec loop;
the native replacement path also avoids constructing an unobservable temporary
match array. The fast path is guarded by the exact current-realm RegExp
prototype, no own `exec`, and the unchanged intrinsic native `exec` function.
Own overrides, mutated prototype methods, RegExp-like objects, and observable
flag behavior therefore remain on the original protocol. The mechanism has no
benchmark identity, source path, expected checksum, or iteration-count branch.

The work was selected from external profiles rather than broad-micro case
shapes. Local sampling of `string-unpack-code` attributed repeated global-match
time to rebuilding RegExp metadata and input code-unit views. Interleaved
five-block Apple-silicon A/B measurements then produced 0.389327x base for
`string-unpack-code` and 0.621166x for `regexp-dna`. A separate one-block
25-case broad diagnostic was 1.003189x base; its unrelated case movement was
treated as noise screening only, not acceptance evidence.

The complete hosted three-role Performance Preview run `29664385576` compared
candidate revision `e5b1a262f49bce6395e583df4790bf4c5cf3f2b3` with exact base
revision `168f24c4929269419fb08f5c8d6189f50297e07c`. Their executable SHA-256
values are
`95568ece30bdf4a71d01a151aec9c28b165377e538d9ebe07f297c07d9a89086`
and
`ed47cbf63c66bb4b9b844abeb2a3b3b1c91d70c22bdd3603b6947a0a6ebb1192`;
the pinned QuickJS-NG executable remains
`8614a5a91e3476db1a1300b0969387b85e0716a836f799cf243a80d4d1f27699`.
All 75 linearity diagnostics passed and all three blocks were valid.

Hosted broad candidate/base was **0.990363x** overall with a 95% confidence
interval of [0.985495x, 0.993295x]. Binding and call improved to 0.960404x and
0.990026x base; array, builtin, property, and control were 0.993865x,
0.999190x, 0.998480x, and 1.000035x. Allocation was 1.006612x with an interval
crossing 1.00x. The unrelated single-case string family measured 1.025165x,
also with an interval crossing 1.00x; that movement remains visible rather than
being attributed to the RegExp mechanism. Candidate/QuickJS-NG improved to
**0.341383x** overall, but allocation remains **2.487731x**, so B4 is not
complete. Broad raw/report SHA-256 are
`82a081385bbf880fcc4a54b7b7646a03257721199f31bcdac4b7b9218cd6a4f7`
and `bfa45b77a18e7725d31142184897b29124042edaae91c2d8ce9a5c5ff3e08dfe`.

The mandatory same-run external evidence isolated large improvements on three
independent SunSpider string/RegExp workloads while leaving the other corpora
effectively flat:

| Neutral shell port | Comparable | Candidate / base | Candidate wins vs base | Candidate / QuickJS-NG |
| --- | ---: | ---: | ---: | ---: |
| JetStream 3 JavaScript subset | 5/5 | 0.999252x | 4/5 | 8.946845x |
| Kraken 1.1 | 7/14 | 0.999160x | 5/7 | 5.708726x |
| SunSpider 1.0 | 26/26 | 0.867078x | 17/26 | 9.017390x |

`regexp-dna` fell from 4,797.347 ms to 1,282.558 ms (0.267347x base),
`string-unpack-code` from 3,964.257 ms to 788.363 ms (0.198868x), and
`string-tagcloud` from 1,598.544 ms to 804.460 ms (0.503245x). The third case
was not the profiling input and is useful generalization evidence for the same
ordinary RegExp replacement mechanism. External raw/report SHA-256 are
`4282778e0c3f625cbbb3871d7e2607b6fd993087889138e4055946381ef4825a`
and `69aa4fe80b53f5d373bf8d0ed427d3fbdb0783740bd4cf37913206e1834a23a0`.

Focused tests cover own and prototype `exec` overrides, named captures, global
empty matches, and UTF-16 positions. The complete local repository gate passed
1,399 runtime tests, all 198 benchmark-tool tests, and 5,139 Test262 subset
cases; QuickJS-NG comparison smoke tests also passed. Branch CI run
`29663692582`, main CI run `29664385589`, and Test262 Coverage run
`29664490206` all succeeded. Coverage remained 42,671/42,672 with one
actionable gap and zero timeout; burndown/comparison SHA-256 are
`a936ab324bf07a438589b179d9c4b342014044b6c251d0857841c008ce5e9ab5`
and `94a97687babcdfba02efff77997af0a59fa7ddfb08a2370d03fbb933de069e69`.

Unit 61 is accepted as B3 progress because an externally profiled, general
runtime mechanism produced a statistically separated broad improvement and
large same-run gains on multiple external workloads without degrading the
other two external corpora. It is not B5 progress: every comparable external
case still loses to QuickJS-NG, and no official upstream-suite score is
claimed. The next external profile identified repeated full-input UTF-16
conversion while emitting replacement segments; that is tracked as a separate
optimization unit rather than being folded into this evidence after the fact.

### Unit 62: accepted reusable UTF-16 RegExp replacement input

Commit `be9db2db`, merged as `88e1a232`, retains one UTF-16 code-unit view of
the input while emitting a native RegExp replacement. Unmatched segments and
the `$\`` / `$'` substitutions now slice that prepared view instead of decoding
the complete input again for each global match. Replacement length accounting
also receives the already-computed input length. The change applies to the
ordinary replacement mechanism and contains no benchmark identity, source
path, expected checksum, or iteration-count condition.

The work was selected from an independent `string-tagcloud` sample after Unit
61. Interleaved local A/B checks against the Unit 61 executable showed the
expected same-direction reductions on `string-tagcloud`, `regexp-dna`, and
`string-unpack-code`; the separate one-block 25-case broad diagnostic was
1.002467x base and was treated only as noise screening. Focused coverage adds
an astral-character global replacement and verifies `$\``, `$&`, and `$'`
UTF-16 slicing in the same result.

The complete hosted three-role Performance Preview run `29666107766` compared
candidate revision `88e1a2324a6b01dfb80abf561ec787497344ada6` with exact base
revision `9a0edd0761d0eafa6a6e6fa529af2ada505a0843`. Their executable SHA-256
values are
`0dfcf5aa4a81b6e43f2e7412df82e41d76acb5cf4277c7557a59d94a42f2a1bb`
and
`95568ece30bdf4a71d01a151aec9c28b165377e538d9ebe07f297c07d9a89086`;
the pinned QuickJS-NG executable remains
`8614a5a91e3476db1a1300b0969387b85e0716a836f799cf243a80d4d1f27699`.
All 75 linearity diagnostics passed and all three blocks were valid.

Hosted broad candidate/base was **1.007638x** overall with a 95% confidence
interval of [1.000553x, 1.010157x]. The binding and builtin families regressed
to 1.015581x and 1.028162x base; allocation, array, control, call, property,
and string were 1.015459x, 1.005068x, 1.001232x, 0.999956x, 0.999975x, and
0.994072x. The RegExp replacement path is absent from this portfolio, but the
statistically separated broad movement remains a campaign cost rather than
being dismissed as unrelated runner noise. Candidate/QuickJS-NG moved to
**0.346338x** overall, and allocation remains **2.592431x**, so B4 is not
complete. Broad raw/report SHA-256 are
`57434e9dae099519d4efbe28e184b3a859e8071b22147e652b2aeb91a4768866`
and `6d2e2c8beb9899536c558325512da315def7ab511f185f40cba69055a1a6c1dd`.

The mandatory same-run external evidence confirmed the prepared-input gain on
all three independent RegExp replacement workloads while the other two
corpora remained effectively flat:

| Neutral shell port | Comparable | Candidate / base | Candidate wins vs base | Candidate / QuickJS-NG |
| --- | ---: | ---: | ---: | ---: |
| JetStream 3 JavaScript subset | 5/5 | 0.993752x | 3/5 | 8.966690x |
| Kraken 1.1 | 7/14 | 1.003210x | 4/7 | 5.682961x |
| SunSpider 1.0 | 26/26 | 0.962590x | 9/26 | 8.608818x |

`regexp-dna` fell from 1,277.797 ms to 1,019.770 ms (0.798069x base),
`string-tagcloud` from 797.625 ms to 550.729 ms (0.690461x), and
`string-unpack-code` from 788.978 ms to 451.626 ms (0.572418x). SunSpider's
absolute candidate/QuickJS-NG comparable-case ratio improved from Unit 61's
9.017390x to 8.608818x, but every external case still loses to QuickJS-NG.
External raw/report SHA-256 are
`97124b66df7277cdde0b0ab66f4bddf47687a8a11455ed4e2b39873084177971`
and `c9d68241a55715702e233cb6597c920ed5d5ab5a7ce873b0d32b6d864a6f389e`.

The complete local repository gate passed 1,399 runtime tests, all 198
benchmark-tool tests, and 5,139 Test262 subset cases; QuickJS-NG comparison
smoke tests also passed. Branch CI run `29665620218`, main CI run
`29666107762`, and Test262 Coverage run `29666207650` all succeeded. Coverage
remained 42,671/42,672 with one actionable gap and zero timeout;
burndown/comparison SHA-256 are
`6d6bf67895979ee84af2b5f498ec816dd009ad3575fd40992b0f7ec9644a14e6`
and `94a97687babcdfba02efff77997af0a59fa7ddfb08a2370d03fbb933de069e69`.

Unit 62 is accepted as B3 external-generalization progress because an
externally selected, benchmark-independent runtime mechanism produced large,
same-direction gains on three workloads without a material cross-corpus
regression. It is not broad-target or B5 progress: the hosted broad regression
is explicit debt, allocation remains above QuickJS-NG, and the external suites
remain 5.68x to 8.97x slower. Subsequent units must recover the broad movement
while preserving these external gains.

### Unit 63: rejected direct-eval capture scan

Commit `79d236c1`, merged as `8d3100fa`, replaced the value-cloning
`binding_snapshot` used by direct-eval captured-function repair with a visible
binding scan. The API itself is benchmark-independent and preserves frame
shadowing, but the isolated unit does not qualify as a general performance
improvement.

The complete hosted three-role Performance Preview run `29667880164` compared
candidate revision `8d3100fa07995dfdb1fcd01b6a932ecaf449e345` with exact base
revision `9a98ea3c66c5231dabfc71d19d28dcb4f6b56963`. Their executable SHA-256
values are
`faae28959686dccbea81fb8bb3a34cc8f9bbb85c69e769790958000760ca1b0c`
and
`0dfcf5aa4a81b6e43f2e7412df82e41d76acb5cf4277c7557a59d94a42f2a1bb`;
the pinned QuickJS-NG executable remains
`8614a5a91e3476db1a1300b0969387b85e0716a836f799cf243a80d4d1f27699`.
All 75 linearity diagnostics passed and all three blocks were valid.

Hosted broad candidate/base regressed to **1.016034x** overall with a 95%
confidence interval of [1.015676x, 1.019484x]. The call and binding families
regressed to 1.052319x and 1.032314x base; allocation and property were
1.007020x and 1.004619x. `captured_read` reached 1.124465x,
`function_call_two_args` 1.083704x, `function_call_reordered` 1.072784x,
`plain_function_call` 1.071010x, and `method_call` 1.070150x. These paths do not
execute the new scan, so their coherent movement is a portable code-layout
cost, not a mechanism win. Candidate/QuickJS-NG was **0.354517x** overall, but
allocation remained **2.597257x**, so B4 remained incomplete. Broad raw/report
SHA-256 are
`1ac39bed374b1af6a282b3b0fc9d008727465558cd7b43cbf84ce9d917b3043f`
and `8541643184404911c313f9b4016ad66dfe2109e240a17a4563078f3fe841b75a`.

The same-run external matrix also failed to establish generalization:

| Neutral shell port | Comparable | Candidate / base | Candidate wins vs base | Candidate / QuickJS-NG |
| --- | ---: | ---: | ---: | ---: |
| JetStream 3 JavaScript subset | 5/5 | 1.010x | 1/5 | 9.104x |
| Kraken 1.1 | 6/14 | 1.004x | 1/6 | 5.734x |
| SunSpider 1.0 | 26/26 | 0.999x | 14/26 | 8.610x |

SunSpider `date-format-tofte` measured 0.945x base, but its eval strings are
call-only expressions such as `a()`: they contain no hoisted or written
binding, so the changed capture-repair helper is not invoked. The other date
eval workload, `date-format-xparb`, was 0.999x base, while the unrelated
`regexp-dna` moved to 1.074x. The target-looking result therefore cannot be
causally attributed to the new scan. A same-binary A/A preview from run
`29667307939` further demonstrated runner/layout sensitivity: candidate and
base shared executable SHA-256
`0dfcf5aa4a81b6e43f2e7412df82e41d76acb5cf4277c7557a59d94a42f2a1bb`,
yet broad candidate/base reported 0.9923x with a confidence interval excluding
1.00x. External raw/report SHA-256 for Unit 63 are
`9247663a7b0adbe2a1fe64c3a8118dc326e47ad303153fae1bc09fce13f6599a`
and `fb72c089da12065df3afc0bbfe13dac5a67f77f3d822c28070ac4d3268342522`.

The complete local repository gate passed 1,400 runtime tests, all 198
benchmark-tool tests, and 5,139 Test262 subset cases; QuickJS-NG comparison
smoke tests also passed. Branch CI run `29667445280`, main CI run
`29667880160`, and Test262 Coverage run `29667960402` all succeeded. Coverage
remained 42,671/42,672 with one actionable gap and zero timeout;
burndown/comparison SHA-256 are
`24ec1f95b1d49b895b94d05b548fec536fbc32791b69b91a6dbc44dc41242bf5`
and `94a97687babcdfba02efff77997af0a59fa7ddfb08a2370d03fbb933de069e69`.

Unit 63 is rejected for B3, B4, and B5. Its visible-binding iterator is carried
forward only as scaffolding for the next unit, which makes the iterator serve
the actual general native-call and direct-eval environment paths. That broader
unit must pass its own hosted broad and external acceptance run; otherwise the
scaffolding is reverted rather than counted as progress.

### Unit 64: accepted dynamic-environment snapshot reduction

Commit `fc0cce0a`, merged as `7453dea3`, makes the Unit 63 visible-binding
iterator serve the general dynamic-environment paths for which it was retained.
`Vm::apply_env` now consumes a sequential visible-binding buffer instead of
first building a name-to-value `HashMap`, and direct eval collects only visible
names when values are not needed. Reverse frame traversal still preserves the
innermost active shadow, and explicit deopt cells remain visible only when a
frame binding does not shadow them. The implementation is independent of
benchmark identity, source path, iteration count, and expected output.

The change was selected from the independent SunSpider date-format profile,
not from a broad-micro case. An exact-base ten-block local external A/B measured
`date-format-tofte` at 0.865776x base and `date-format-xparb` at 0.963753x;
`string-tagcloud`, which was not the profiling target, remained neutral at
1.002251x. A fresh short candidate profile still placed `apply_env` and the
remaining sequential binding collection among the top frames, while the exact
base profile showed the removed `snapshot_locals` work below both direct eval
and `apply_env`. This supports the mechanism's causal relationship but is not
used in place of the hosted comparison.

The complete hosted three-role Performance Preview run `29669068340` compared
candidate revision `7453dea3c339a6b93f3f2ec767866c762f4ea699` with exact base
revision `8d3100fa07995dfdb1fcd01b6a932ecaf449e345`. Their executable
SHA-256 values are
`bc711e20bd1206c21ac9436885f585a2dc4a632966ade9d61be81f8fe9ce8498`
and
`faae28959686dccbea81fb8bb3a34cc8f9bbb85c69e769790958000760ca1b0c`;
the pinned QuickJS-NG executable remains
`8614a5a91e3476db1a1300b0969387b85e0716a836f799cf243a80d4d1f27699`.
All 75 linearity diagnostics passed and all three blocks were valid.

Hosted broad candidate/base improved to **0.979391x** overall with a 95%
confidence interval of [0.979057x, 0.981504x]. Call, binding, allocation,
property, array, and string were 0.946087x, 0.968429x, 0.982383x, 0.996781x,
0.997303x, and 0.984163x base. Control was effectively neutral at 1.001469x.
Builtin regressed to **1.028617x**, driven by `array_index_of` at 1.057915x;
that broad case does not execute the changed dynamic-environment mechanism, so
the regression is recorded as portable code-layout debt rather than hidden or
claimed as a semantic trade. Several direct-call cases similarly improved even
though they do not execute this mechanism, so their movement is not counted as
causal evidence. Candidate/QuickJS-NG was **0.345553x** overall, slightly
better than Unit 62's pre-scaffolding 0.346338x, but allocation remained
**2.559545x** and B4 is therefore incomplete. Broad raw/report SHA-256 are
`a60a3f1c0d7016a6ddb44566805f0cca6e9efe91b087a6e3f1188c55677da684`
and `b4d6145978d5c2a5971b3198bdb409ca0ccc6b9098dbd0434f9161bb2a187af7`.

The mandatory same-run external evidence generalized in all three corpora:

| Neutral shell port | Comparable | Candidate / base | Candidate wins vs base | Candidate / QuickJS-NG |
| --- | ---: | ---: | ---: | ---: |
| JetStream 3 JavaScript subset | 5/5 | 0.983272x | 4/5 | 8.934282x |
| Kraken 1.1 | 7/14 | 0.995930x | 4/7 | 5.703890x |
| SunSpider 1.0 | 26/26 | 0.984351x | 15/26 | 8.458902x |

Both independently profiled eval workloads moved in the expected direction:
`date-format-tofte` fell from 844.677 ms to 731.238 ms (0.865701x base), and
`date-format-xparb` fell from 193.271 ms to 188.700 ms (0.976349x).
`string-tagcloud` also improved to 0.983944x base. No external comparable case
regressed beyond 1.023051x, all three suite-level candidate/base geometric
means improved, and comparable coverage did not decrease. External raw/report
SHA-256 are
`357d3ca02f446a91feb3c9770a2a16224a49fae09d547939f407e7d4b1037e6c`
and `96dacea4db2c2a28795c75a3e5947be131fc28a74774acb3e8692676557b7a11`.

The complete local repository gate passed 1,400 runtime tests, all 198
benchmark-tool tests, and 5,139 Test262 subset cases; QuickJS-NG comparison
smoke tests also passed. Branch CI run `29668579145`, main CI run
`29669068333`, and Test262 Coverage run `29669147736` all succeeded. Coverage
remained 42,671/42,672 with one actionable gap and zero timeout;
burndown/comparison SHA-256 are
`b194cfd73aac52655cad6d1a60c0315dfc5b7718d967dd16b37e8f0b5b9dd25d`
and `94a97687babcdfba02efff77997af0a59fa7ddfb08a2370d03fbb933de069e69`.

Unit 64 is accepted as B3 external-generalization progress: an externally
profiled, general environment mechanism produced repeatable gains on both eval
workloads, remained positive across all three external corpora, recovered the
pre-scaffolding broad position, and preserved correctness. It is not B4 or B5
progress: allocation and every external suite still lose materially to
QuickJS-NG. The next environment unit should remove the remaining temporary
binding buffer or replace direct-eval value copying and name-based writeback
with shared slot/upvalue cells; it must preserve the external gains and recover
the builtin code-layout debt rather than specializing a benchmark source.

### Unit 65: rejected streamed dynamic-environment writeback

The uncommitted Unit 65 prototype attempted the smaller of those follow-ups:
it moved direct frame values through a callback instead of allocating the
temporary visible-binding `Vec`, froze cell-backed values before callbacks,
and avoided direct-parameter shadow sets outside parameter prologues. The
prototype changed only the general environment representation and
`Vm::apply_env`; it contained no workload identity, source, checksum, or
iteration matching. Focused visible-binding and direct-eval tests passed, as
did all-target clippy, but the mechanism added 188 lines while removing 79.

An exact local A/B used Unit 64 binary SHA-256
`c42b6774060e58d8a510371dd62b48086877c984cffe3cd9bba4f164e7039381`
and prototype SHA-256
`7ad50da46a6894f4645906a3c13f59045aeddcc942c44ae8fc2f2cf17091f157`.
After warmup, twenty paired external blocks produced robust median
candidate/base ratios of 0.999855x for `date-format-tofte`, 0.998219x for
`date-format-xparb`, and 0.999425x for `string-tagcloud`. Geometric means moved
more because of noisy long-tail samples, but the medians show no repeatable
general gain.

A six-block diagnostic broad slice agreed. Formal measurement samples gave
median candidate/base ratios of 1.000491x for `array_index_of`, 1.004113x for
`dynamic_method_call`, 1.000402x for `many_locals_call`, 1.002385x for
`plain_function_call`, and 1.000056x for `top_level_function_call`. The raw
SHA-256 is
`c68e7424d4e025d920bac2bdf5fcee17ebd3675186812d69c2d4a629475efa82`.
The report correctly rejected this deliberately incomplete case selection as
an identity/completeness mismatch, so these values are local diagnostics, not
a portfolio claim.

Unit 65 is rejected without a runtime commit or hosted preview. It neither
improved the independent external workloads nor the selected broad paths by a
material amount, and accepting its added representation complexity would
violate the campaign's general-performance and anti-overfitting rules. The
next unit must attack the structural cost that streaming could not remove:
direct eval still copies values into a name-based environment and writes them
back by name. Shared slot/upvalue cells are the intended mechanism; acceptance
still requires complete broad and mandatory external evidence.

### Unit 66: rejected redundant live-cell value-copy removal

Runtime commit `11d9e4df` attempted the shared-cell follow-up without changing
the dynamic-environment contract: when `frame_call_env` was already going to
overlay a visible slot with its live upvalue cell, its first pass stopped
cloning the cell's current `Value` into a temporary name binding. The mechanism
was general and contained no workload identity, source path, iteration count,
or checksum matching. An initial single-pass prototype incorrectly changed
lexical-shadow ordering; focused tests caught two failures, and the pushed
version retained the original two-pass ordering. All 1,400 runtime tests, 65
selected eval/block/function Test262 cases, the complete repository gate, and
the branch CI run `29672096329` passed before integration.

Hosted Performance Preview run `29672508646` compared candidate merge
`4e9c9c7c` and exact base `51063ac8`. Candidate and base executable SHA-256
were `ee866f6232be90f1e71e449a666c05448dae4311e0ad641f0f047f2a22c51e4f`
and `bc711e20bd1206c21ac9436885f585a2dc4a632966ade9d61be81f8fe9ce8498`;
the pinned QuickJS-NG executable was
`8614a5a91e3476db1a1300b0969387b85e0716a836f799cf243a80d4d1f27699`.
The broad artifact was complete with 225/225 valid measurements, 75/75 passing
linearity probes, and three valid blocks. It rejected the candidate clearly:
candidate/base was **1.034108x**, with a 95% confidence interval of
[1.031907x, 1.037882x]. Call, binding, and allocation regressed to 1.067149x,
1.064953x, and 1.052465x base; `captured_read`, `function_call_two_args`, and
`captured_write` reached 1.207745x, 1.166919x, and 1.136323x. The isolated
`array_index_of` improvement to 0.950038x cannot offset those general losses.
Candidate/QuickJS-NG was 0.359263x overall, but allocation remained 2.683857x,
so the candidate was neither B4 progress nor an acceptable intermediate unit.

The mandatory same-run external evidence was mixed rather than compensating:
candidate/base geometric ratios were 0.998699x for all 5/5 JetStream cases,
0.997804x for 7/14 comparable Kraken cases, and 1.001515x for all 26/26
SunSpider cases. The selected `date-format-tofte` path did reproduce the local
gain at 0.963264x base; `date-format-xparb` was 0.991150x and
`string-tagcloud` 0.997592x. Nevertheless, the three external diagnostic ratios
were still 8.953760x, 5.700817x, and 8.499701x QuickJS-NG with zero qjs-rust
wins. A single 3.7% eval-workload win therefore cannot justify a repeatable
3.4% broad regression.

Broad raw/report SHA-256 are
`9e3ae12898c0f486e0541417eb1ec6f67a60731106a19f7b676c50dfad0c7c21`
and `b00cc11d4a2928178e9f589c02c93d0a7156c19ec191d45bed68366ff1ad3bf0`;
external raw/report SHA-256 are
`3b53eeb569b61b78460d0cd8ae47489e667ff740ac4a24f5767a75c9959dd1d0`
and `50bed5d82d66710398cba0225d0eed341e763f9f00ba0d9b6b3c249a95f83bae`.
Main CI run `29672508650` and Test262 Coverage run `29672600574` succeeded.
Coverage remained 42,671/42,672 with one actionable gap and zero timeout;
burndown/comparison SHA-256 are
`82c7c42e8cfa32e4dd0d75e06e37d98cbf823b44059ddcd0153d20e72f4ab3b4`
and `94a97687babcdfba02efff77997af0a59fa7ddfb08a2370d03fbb933de069e69`.

Commit `d65d5cfd` restores the exact base runtime. Before that revert was pushed,
the full local gate again passed 1,400 runtime tests, all 198 benchmark-tool
tests, 5,139 Test262 subset cases, and all 205 QuickJS-NG comparisons. Unit 66
is rejection evidence only: even a benchmark-independent mechanism is not
campaign progress when complete broad and external evidence disagree. The next
call-frame unit should eliminate allocation without adding per-op stack
branches or trading call throughput against object, array, and closure
allocation.

### Unit 67: accepted bounded bytecode operand-stack reuse

Runtime commit `2c9e7da3`, merged as `adc52017`, gives each compiled `Bytecode`
one shared, single-entry pool for its VM operand-stack allocation. Ordinary
execution takes the cleared buffer and returns it on frame exit; generator
suspension moves the live stack into the snapshot and returns any unused
acquired buffer. Fresh buffers start with capacity 64, and capacities above 256
are deliberately not retained so one unusual frame cannot permanently inflate
the compiled function. The mechanism applies to every bytecode invocation and
contains no benchmark identity, source path, iteration count, checksum, or
expected result. A focused unit test fixes cleared reuse and bounded rejection,
including clones of the same bytecode.

The local screening evidence was directionally positive but deliberately not
used as acceptance evidence. A focused nine-case call/binding/allocation run
had an approximate candidate/base case geometric mean of 0.9909x, and a direct
alternating JetStream raytrace sample measured 0.9591x base. The local complete
portfolio attempt was only 24/25 comparable because one `captured_write` block
failed local linearity; its raw all-measurement diagnostic was 0.9938x base.
That incomplete result justified sending the general mechanism to hosted
measurement, not accepting it.

Trusted-main Performance Preview run `29676062273` compared exact candidate
`adc5201782e3594c856a82c763a2c01846b6c67e` with exact base
`5f7f7a16a78f9b9aeaa635019ce30908168bae68`. Candidate, base, and pinned
QuickJS-NG executable SHA-256 were
`04c98f4a10f7254e41c15ef2e2ceb81832fdd7808af6f544cfc7494947a1c288`,
`bc711e20bd1206c21ac9436885f585a2dc4a632966ade9d61be81f8fe9ce8498`,
and `8614a5a91e3476db1a1300b0969387b85e0716a836f799cf243a80d4d1f27699`.
All 225/225 formal measurements were valid, all 75 engine/case linearity
diagnostics passed, and all three blocks remained valid. As expected for the
three-block hosted preview, health is `non-claim`/`inconclusive`, not a
fixed-hardware public claim.

Hosted broad candidate/base was **0.996099x** overall with a 95% confidence
interval of [0.993897x, 1.001591x]. The interval crosses 1.0, so the 0.39%
point improvement is recorded as near-neutral rather than a broad win. Call
was 0.996625x base and control 0.989373x; builtin moved to 0.890904x, driven by
unrelated code-layout movement in `math_abs` and `array_index_of`, and is not
claimed as causal. Allocation, binding, property, array, and string were
1.012187x, 1.020030x, 1.004207x, 1.007278x, and 1.005126x base. In particular,
`captured_read` and `function_call_two_args` were 1.088225x and 1.090019x;
those internal regressions remain debt rather than being hidden by the overall
geometric mean. Candidate/QuickJS-NG was **0.360613x** overall, but allocation
remained **2.756890x** and call remained 0.568853x, so B4 is still open.

The mandatory same-run external evidence generalized modestly in all three
corpora and is the reason this near-neutral broad unit is retained:

| Neutral shell port | Comparable | Candidate / base | Candidate wins vs base | Candidate / QuickJS-NG |
| --- | ---: | ---: | ---: | ---: |
| JetStream 3 JavaScript subset | 5/5 | 0.982408x | 5/5 | 9.663311x |
| Kraken 1.1 | 7/14 | 0.993973x | 5/7 | 6.727101x |
| SunSpider 1.0 | 26/26 | 0.995835x | 15/26 | 9.061978x |

Every JetStream case improved against the exact base. Kraken's two losing
comparable cases were at most 1.003281x base, and SunSpider's largest
regression was 1.011216x; comparable coverage did not decrease. These are
neutral shell-port diagnostics, not official suite scores, and they also make
the remaining external deficit unambiguous: qjs-rust still loses every
comparable external case to QuickJS-NG by large suite-level geometric means.

Broad raw/report SHA-256 are
`763dd57962a2456a996a487e5ab6f916670db31565d75f68432f8daf587dbf42`
and `9ec4e97372641d523081495fb2b8bb9c9e0b52584ae1e66105d8e5f11676c3e4`;
external raw/report SHA-256 are
`094957ef8038102713a5b82fdc08c7aeeeda9e36064111db295344388266af61`
and `3e9e994a7a8073923b0cd8366bae7f1284e00a24ef071665d2277cc5af38a57d`.
The complete local gate passed 1,401 runtime tests, all 198 benchmark-tool
tests, 5,139 Test262 subset cases, and all QuickJS-NG comparisons. Branch CI
run `29674623516`, main CI run `29676062272`, Performance Preview run
`29676062273`, and Test262 Coverage run `29676164904` all succeeded. Coverage
remained 42,671/42,672 with one actionable gap and zero timeout;
burndown/comparison SHA-256 are
`c1893e0880725baaef7f27f76adc6b714b8906caa8cefefe8e0489e88fce0111`
and `94a97687babcdfba02efff77997af0a59fa7ddfb08a2370d03fbb933de069e69`.

Unit 67 is accepted only as small B3 external-generalization progress: a
bounded, benchmark-independent allocation-reuse mechanism kept the complete
broad portfolio near neutral and improved all three independent external
suite-level diagnostics. It is not B4 or B5 progress, and its internal binding
and allocation regressions constrain the next unit. The next general unit
should target the remaining per-call frame/local/upvalue allocations or the
object/array/closure allocator itself, while preserving the external gains and
recovering the binding family; benchmark-specific stack sizing or case-aware
pooling remains forbidden.

### Unit 68: rejected single-entry function-locals reuse

Branch commit `fc445cef` tested one benchmark-independent allocation change:
each compiled function retained one cleared local-slot `Vec` for a later
sequential invocation, while recursive overlap allocated normally and
capacities above 256 slots were discarded. `FunctionBytecodeResult` returned
the storage on drop, including errors and derived-constructor consumers. The
implementation contained no workload identity, iteration count, checksum, or
source-path condition. It was motivated by an independent JetStream `hash-map`
sample in which VM frame construction accounted for 83 of 1,821 run-time
top-of-stack samples (4.56%), alongside substantial allocator/free traffic.

Correctness screening passed 1,403 runtime tests, the 114-case touched
Test262 slice, the complete local pre-push gate including 5,139 Test262 cases,
and all QuickJS-NG comparisons. Branch CI run `29678450308` was fully green:
`check`, `compare-qjs`, and `test262-subset` all succeeded. Correctness was not
the rejection reason.

The first complete external same-run diagnostic kept full 5/5 JetStream,
14/14 Kraken, and 26/26 SunSpider coverage. Candidate/base geometric means
were 0.993134x, 1.006689x, and 0.986631x respectively. JetStream improved in
4/5 cases and SunSpider improved at suite level, but Kraken regressed 0.67%; in
particular `ai-astar` and `stanford-crypto-ccm` were 1.055087x and 1.058075x
base. The external raw/report SHA-256 were
`65c64f6913db35eb86ca17f18d53eaadd2397e03bcbc4d3a30dd2dafcfc885ee`
and `30dd9f0d13113c1f441b5b5e0a6024899666c4e4f37012ac8baf6f6f57eb538b`.
This direct-binary run did not carry hosted build receipts, so it is retained
as diagnostic rejection evidence rather than trusted acceptance evidence.

A subsequent clean-source preview rebuilt exact candidate `fc445cef`, exact
base `57d3b888`, and pinned QuickJS-NG with the hosted fixed recipe. Executable
SHA-256 were `30116c6a1ef0131f4a71ab40347ca0237ed689461eb238191b05526ae16bc7ac`,
`36b5269a1c6e4edae6ed23e03dd7897d87a6d731ec14f1a87ee78a2180b83fb6`,
and `cfd8386c3c29b1125a878b8fb82f9627820f2dcc16d2a691c5f8c16ad0b047a0`.
All 75 linearity diagnostics passed and all 225 formal measurements executed,
but every portfolio block was correctly invalidated: candidate
`captured_read` and base `property_write` were timer-limited in all three
blocks on the local M1 host. Therefore the report contains no valid complete
comparison and cannot support a performance claim.

The invalid-block raw measurements remain useful only for deciding whether to
spend a hosted run. Their all-case candidate/base diagnostic geometric mean
was 0.997127x. The mechanism's own expected beneficiaries moved in the wrong
direction: call was 1.001988x and allocation was 1.006378x base. String was
1.065892x, while array, binding, builtin, control, and property were
0.986259x, 0.994131x, 0.980709x, 0.977606x, and 0.996161x. Verified raw/report
SHA-256 were
`2db5eb0ebf9a3ccdcc2943afce71759ff7fdfef4690ad843d563064c1efb2ff0`
and `2d80803055c6b7d4342e5c042c64c2e4762e8721cecd7bbc52cc4a7b67f0e9af`.

Unit 68 is rejected and its runtime commit is not merged. The result shows
that removing one local-slot allocation is too small to offset the added pool
bookkeeping and does not explain most of the external frame-construction hot
path. The next unit must profile and reduce a broader frame/call mechanism
(initialization, value cloning/dropping, environment lookup, or dispatch), or
target another independently dominant external subsystem. It must not extend
this pool merely to make the internal call cases look better.

### Unit 69: rejected direct-call argument moves

Unit 69 tested whether VM-owned arguments could enter slot-backed leaf frames
without the existing shallow `Value` clone and matching drop. This was a
benchmark-independent call-frame mechanism motivated by the same independent
JetStream `hash-map` profile: `Value::clone` and `Value::drop` together
accounted for 123 of 1,821 sampled run-time stacks, while the VM already owned
the arguments it had popped. Neither implementation contained workload names,
iteration counts, checksums, or source-path conditions.

The first implementation generalized direct-call argument storage to borrowed,
empty, one-value, and owned-vector variants. It passed `cargo check` and all
1,401 runtime tests, but its exact release binary
`506c074454df35bf06177ebb4ed7f6de90931da39dd6b4d07255652128299746`
failed the first external screen. Seven alternating JetStream `hash-map`
samples produced a 3,379,502,917 ns candidate median and a 3,308,020,208 ns
base median, or **1.021609x** candidate/base. The generalized enum and iterator
dispatch cost more than the removed clones, so this shape was discarded.

The second implementation preserved the original borrowed multi-argument path
and moved only the VM's hottest zero/one-argument shape. It also passed
`cargo check` and all 1,401 runtime tests. Exact candidate and base release
binary SHA-256 were
`a87dc2b1f4fb615d1b445feb1bf89807829c6cbcf523386484dcfbed57cb85b2`
and `1917121ac8a121c6214ffef0c7107f4d71064ad0719ca56a2959ff69d6f5d20f`.
Seven alternating, post-warmup JetStream `hash-map` samples yielded medians of
3,356,192,166 ns and 3,387,047,500 ns, or **0.990890x** candidate/base. That
0.91% direction was smaller than the observed run-to-run spread and therefore
was not treated as an external win. The generated hash-verified bundle SHA-256
was `aa98bc1975d8824840df5c31a397b5dab27d514e1854ff5681ceea8ec4bf2c20`.

A separate three-block, randomly ordered black-box diagnostic then measured all
six internal call-family cases against the exact same base and pinned
QuickJS-NG. Candidate/base median ns/op ratios were 0.995354x for
`plain_function_call`, 0.999063x for `method_call`, 1.001234x for
`function_call_two_args`, 1.005297x for `function_call_reordered`, 1.022224x
for `top_level_function_call`, and 1.009039x for `dynamic_method_call`.
Four of six shapes regressed, including the two slower general call shapes.
This selected-case run is diagnostic only, not a complete portfolio claim; its
raw JSONL SHA-256 is
`aa0f202bb788ef162a3a19dff8c1f5325477a8ebe7ae320ec6652300405415aa`.

Unit 69 is rejected with no runtime commit. The experiment establishes that a
single shallow argument clone is not a dominant enough cost to justify added
frame state or dispatch. The next unit must remove a larger end-to-end call or
allocation mechanism demonstrated by external profiles; specializing internal
call cases or extending argument-shape variants is explicitly out of scope.

### Unit 70: rejected realm-owned empty direct-call metadata

Runtime commit `43fa6d56` tested a general allocation reduction identified by
the independent JetStream `hash-map` profile. Every direct leaf frame created
three fresh empty reference-counted maps/sets for catch bindings, direct-eval
conflicts, and module imports. The candidate instead shared canonical empty
metadata owned by the realm and retained copy-on-write mutation semantics. It
contained no benchmark identity, source path, iteration count, checksum, or
expected result. A focused test verified sharing, mutation isolation, and the
unchanged frame contract; 1,402 runtime tests, the touched Test262 slice, all
5,139 complete-gate Test262 cases, and all QuickJS-NG comparisons passed.
Branch CI run `29682077926` and main CI run `29682201219` were fully green.

Local external screening strongly favored the mechanism. Eight alternating
post-warmup JetStream `hash-map` samples measured a 0.922703x candidate/base
median. The complete one-block external preview retained 5/5 JetStream, 13/14
Kraken, and 26/26 SunSpider comparable cases; candidate/base suite diagnostics
were 0.954x, 0.992x, and 0.984x. A complete 25-case, three-block local broad
diagnostic measured 0.984973x base overall, with every family below 1.0x and
only `property_read` slightly above base at 1.001423x. These receipt-less local
measurements were screening evidence, not acceptance evidence. Their broad raw
SHA-256 was
`e6d6046d197e59e41ad1d38fa4e4cb3ba47077696878f418bfe5fa17466f64b8`;
external raw/report SHA-256 were
`f8505e0ae107fe446cb895c77311f1be9d159f5b1f5a615e3403c4837f336672`
and
`19303eb0277aa15cc958fd592a873c467798ffd675a3450aec3238d0c3c39081`.

Trusted-main Performance Preview run `29682201213` compared exact candidate
`43fa6d5634bf5f396c3ff8fd52dbda1c0b69148d` with exact base
`e749a6fa07cdec94febec292dead22c3b3d113c6`. Candidate, base, and pinned
QuickJS-NG executable SHA-256 were
`62555e5b7068a3906ab51f6f3123c915a7ff46cf02a6e82897e58c583e19d0eb`,
`04c98f4a10f7254e41c15ef2e2ceb81832fdd7808af6f544cfc7494947a1c288`,
and `8614a5a91e3476db1a1300b0969387b85e0716a836f799cf243a80d4d1f27699`.
All 225/225 formal measurements and all 75 linearity diagnostics were valid,
with 3/3 complete blocks.

The hosted broad result contradicted the local screen: candidate/base was
**1.022354x** overall with a 95% confidence interval of
[1.019425x, 1.025640x]. Call was **1.086263x** and binding was 1.023243x;
`plain_function_call`, `method_call`, and `function_call_reordered` each
regressed about 15%. Candidate/QuickJS-NG remained 0.354328x overall, but the
allocation family was still 2.780764x, so the campaign target was not reached.
The same hosted run did confirm external generalization: candidate/base was
0.975x for JetStream with 5/5 wins, 1.002x for Kraken, and 0.993x for
SunSpider. Candidate/QuickJS-NG remained far from B5 at 8.815x, 5.792x, and
8.595x respectively, with no coverage decrease attributable to the change.

Hosted broad raw/report SHA-256 are
`06ab59205e86a16959f500d37d3a6afc7a71eeedb57da54e5a5f9d76149379bc`
and
`6c6d6512b582437f33f68b0beb130d24c18c3af99774163ba77f21c3dfd449aa`;
hosted external raw/report SHA-256 are
`92bbedb1ef5ec63b19b85762377a5a15efc1d2b5aa43b1318b46f3a8269e647f`
and
`f13e0ea7c866179fef52810bf1650014fbc19e76aaf8d4b290ad55ceff1b61a5`.

Unit 70 is rejected despite its genuine external improvement. Broad-micro is
not allowed to dictate the optimization design, but it remains the campaign's
mandatory regression guard; a complete hosted 2.24% overall loss and 8.63%
call-family loss cannot be hidden by the external aggregate. The runtime
change is reverted. The next unit keeps the externally profiled direction but
must isolate a mechanism whose benefit survives both the broad guard and the
external preview.

### Unit 71: accepted direct leaf-getter call path

Runtime commit `662f63e1` removes the caller-environment snapshot and writeback
from a general property-access mechanism: an ordinary, Proxy-free prototype
lookup may invoke a directly resolved leaf user getter with the callee's own
slot frame. Accessors reached through proxies or exotic objects, non-leaf user
functions, and every dynamically unresolved case retain the observable generic
path. The implementation contains no workload identity, source-path, iteration,
checksum, or expected-result condition. Focused tests cover the receiver,
prototype lookup, abrupt completion, captured accessors, and the guarded
fallbacks. The full local gate passed 1,402 runtime tests, 198 benchmark-tool
tests, 5,139 curated Test262 cases, all QuickJS-NG comparisons, formatting,
Clippy, and file-size checks.

Trusted-main Performance Preview run `29685294595` compared exact candidate
`662f63e1d407c2c763dde42b1ed7f0094e18d181` with exact base
`255e9796fa505c4e98ee28418878566ebd4af8a7`. All 225/225 formal broad
measurements were eligible, all 75 N/2N checks passed, and all three blocks
were valid. Candidate/base was **0.991664x** overall with a 95% confidence
interval of [0.990401x, 0.992370x]. Call was 0.975402x and binding was
0.995658x; allocation was 1.009048x and control was 1.026165x. The latter
small family movement is retained visibly rather than hidden by the aggregate.
Candidate/QuickJS-NG was 0.344189x overall, but allocation still failed B4 at
2.784770x. The hosted profile remains intentionally
`inconclusive`/`non_claim`, not a fixed-hardware public claim.

The mandatory external preview independently accepted the mechanism's
direction. JetStream's five-case candidate/base geometric mean was
**0.970663x**, with four candidate wins and one base win; `hash-map`, the
profiled beneficiary, measured **0.882188x**, while `cdjs`,
`raytrace-public-class-fields`, and `stanford-crypto-aes` also improved.
SunSpider remained neutral at 0.999567x over all 26 cases. Kraken was
1.004750x over the six cases comparable to the exact base and retained seven
candidate/QuickJS-NG comparable cases; it is an incomplete diagnostic, not a
suite score. qjs-rust still trails QuickJS-NG badly on these ports at 8.729700x
JetStream, 5.803031x Kraken, and 8.735423x SunSpider. This unit is therefore
accepted as measured general-engine progress, not as evidence that the external
performance goal is close to complete.

The standard CI run `29685294592` passed every executed job. Test262 Coverage
run `29685411920` passed all 16 Rust shards and aggregation; its baseline-shard
and cache-save skips were expected cache hits rather than failures. Hosted
evidence bindings:

- benchmark run ID: `fee7ab26-e56f-4823-8081-c971165b5a66`;
- artifact SHA-256:
  `b98c367e70f2b5234100bf29821c3cbcfe45784cd5dde1d730292c02de28da7e`;
- broad raw/report SHA-256:
  `d1d42afba6508e7f94536dea639c884bab2348baf54e62197f98f4ae2bf1a03a`
  and
  `444765779b7c57b764ae818b0564231d7c8f147722c5dc00dc4d846b07a4917f`;
- external raw/report SHA-256:
  `9a2a97032eb70886fa4648c946c5d2bd855c5b16817bc405129b0a7beb3ac955`
  and
  `b36518672870dbdac3a475dca0dad5f747874bd6467f175e683da53d47b3abba`;
- candidate/base/QuickJS-NG executable SHA-256:
  `f4931ee4609bb3a137486144887c6e82151d6cba9efea699d6d7f98c865e4396`,
  `04c98f4a10f7254e41c15ef2e2ceb81832fdd7808af6f544cfc7494947a1c288`,
  and
  `8614a5a91e3476db1a1300b0969387b85e0716a836f799cf243a80d4d1f27699`.

### Unit 72: rejected native-getter shortcut

Runtime commit `a64ee6f8` called an already resolved frame-independent native
getter without rebuilding the caller compatibility environment. The focused
typed-array getter probe improved to 0.2537x candidate/base, but that narrow
result did not generalize. Trusted-main Performance Preview run `29687435417`
completed all 225/225 formal measurements, all 75 linearity checks, and all
three blocks. Candidate/base was **1.008305x** overall with a 95% confidence
interval of [1.005640x, 1.014772x]. Builtin was 1.046763x, call was 1.023254x,
and string was 1.028092x. The mandatory external preview was also neutral to
worse: candidate/base was 1.001049x for JetStream, 1.002609x for Kraken, and
1.000579x for SunSpider.

This is exactly the kind of internal-only win prohibited by the general
optimization acceptance rule. The mechanism improved its motivating probe but
made the complete broad guard significantly worse and produced no independent
external benefit, so commit `85c4696b` reverted it. The reverse Performance
Preview run `29689512453` recovered the broad result to 0.989672x with a 95%
confidence interval of [0.987800x, 0.993055x]. Its external candidate/base
ratios were 0.997334x, 0.995028x, and 0.998796x respectively, confirming that
the rejected shortcut had not carried an external gain. CI run `29689512443`
and Test262 Coverage run `29689606829` passed after the revert; the full local
gate passed 1,402 runtime tests, 198 benchmark-tool tests, and all 5,139 curated
Test262 cases.

Rejected-candidate broad raw/report SHA-256 are
`6b0ac4cc6b9b267eee53834d003a2f842b1f8993d23e4d1118cf7e2a9bbe06ca`
and
`8081741939a8fa782cd7405b9df6a8b4477b44a84e15dca0c14b3fa55a0f0072`;
its external raw/report SHA-256 are
`f3a6804f8327603daeb1eea2bf8fa0d913112345096508e15c3a0d8c67ed07c7`
and
`ccbdb1f3db6ada52fc549649d1bc2207dcbfd2ed2ff649faf26850c066660c04`.
Reverse-run broad raw/report SHA-256 are
`f2d6b7199e209d19880557b5074ca20e13c32db602a7c8418c2cc0e1e671bc5c`
and
`c1b406a3e405ffda2247fd908d6162d76b41d0249392fc9c3f67cd2c47b6ab29`;
reverse-run external raw/report SHA-256 are
`8efeddafc87a986c0ce41a1d20770a59e10aa7483d8757fe743ea992882db313`
and
`3dbf270dbeede6c9a7e7235198b9ce0a248aeebdcabc9ceef4dfc868a794213c`.

### Unit 73: accepted metadata-free plain JSON parsing

Runtime commit `c4b41d80` makes the ordinary no-reviver `JSON.parse` path build
values directly. Only a callable reviver requests the source strings and
recursive `JsonNode` child tree needed by the source-context extension. This
is a general parser allocation change: it is independent of input size,
property names, suite identity, source path, iteration count, and checksum.
Focused coverage verifies that plain parsing discards metadata while reviver
parsing retains exact primitive source text. The full local gate passed 1,403
runtime tests, 198 benchmark-tool tests, all 5,139 curated Test262 cases,
formatting, Clippy, agents, and file-size checks. Main CI run `29691436176` and
Test262 Coverage run `29691553101` both passed.

The external result provides the causal acceptance evidence. Trusted-main
Performance Preview run `29691436194` measured exact candidate
`c4b41d8076ed4c0b9c9898ff6710c87d6e64c713` against exact base
`85c4696bf1ac3af7a6f8f6b6feb82d7010663ef2`. Kraken
`json-parse-financial` fell from 156.529 ms to 108.779 ms: **0.694944x**
candidate/base, or about 30.5% faster. It reached 1.055857x
candidate/QuickJS-NG. The distinct `json-stringify-tinderbox` path remained
neutral at 0.998733x candidate/base. Kraken's six-case comparable diagnostic
geometric mean improved to **0.946306x**; JetStream was 0.998683x and
SunSpider was 1.000436x, with no coverage loss caused by the change.

The hosted broad run was physically complete: 225/225 measurements, 75/75
linearity checks, and three valid blocks. It reported 1.003668x candidate/base
with a 95% confidence interval of [1.002444x, 1.009864x], while retaining the
profile's explicit `inconclusive`/non-claim classification. No broad case calls
`JSON.parse` (the harness only uses `JSON.stringify` to emit its result), so
that cross-case shift has no execution-path connection to this commit. The
same-host external controls agree: both unrelated complete suites were within
0.14%, while the exact parser workload moved by 30.5%. Unit 73 is therefore
accepted as externally demonstrated general-engine progress rather than being
rejected on an unrelated hosted-runner shift. Candidate/QuickJS-NG remained
0.345121x on broad overall, but allocation still failed B4 at 2.767643x; the
external suite diagnostics remained 8.752580x JetStream, 5.343628x Kraken, and
8.721159x SunSpider, so B4/B5 remain open.

Hosted broad raw/report SHA-256 are
`7471f410a224d7534eb0901246a83961bee946d4597b2907726000bfc9a84eb2`
and
`801735c0c8b3593e9b2aa998368ee8b6b6b280d8f808a71e910e4f4416084925`;
hosted external raw/report SHA-256 are
`30b67c7db74d8f0b1bec4b9669654341ae2dcec7cfbd73727b891e882077bc7c`
and
`91ae97056a57092bac9819477e74d5157320474ecc139c08d33426181b8f0a29`.
Candidate/base/QuickJS-NG executable SHA-256 are
`efd2be5bc2bfc3d8c0b048d74091747dc1310d05c122b62419a8fb6a5e2dea68`,
`f4931ee4609bb3a137486144887c6e82151d6cba9efea699d6d7f98c865e4396`,
and
`8614a5a91e3476db1a1300b0969387b85e0716a836f799cf243a80d4d1f27699`.

### Unit 74: rejected direct slots for closure-producing calls

Runtime commit `42685690` allowed a closure-producing function to enter the
general indexed-slot call path, retaining an all-`None` local-upvalue table so
captured locals could be promoted lazily when a nested closure was created.
The change contained no benchmark identity or input-shape condition, and its
focused local probes suggested about 3% improvements for closure allocation
and dynamic method calls. The full local gate passed 1,404 runtime tests, 198
benchmark-tool tests, all 5,139 curated Test262 cases, formatting, Clippy,
agents, file-size checks, and QuickJS-NG comparisons. Main CI run
`29692851062` and Test262 Coverage run `29692980217` also passed.

The trusted complete evidence did not confirm the focused result. Performance
Preview run `29692851054` compared exact candidate
`42685690f7e89d03671e2a905673ff008e87f897` with exact base
`c4b41d8076ed4c0b9c9898ff6710c87d6e64c713`. All 225/225 broad measurements,
75/75 linearity checks, and three blocks were valid, but candidate/base was
**1.016134x** overall with a 95% confidence interval of [1.014143x,
1.055291x]. `captured_read` regressed to 1.157411x, `captured_write` to
1.088178x, and the motivating `closure_allocation_call` case was effectively
unchanged at 1.000398x. Candidate/QuickJS-NG remained 0.352113x overall, with
the allocation family still above the campaign threshold.

The mandatory external preview supplied no offsetting generalization benefit.
Candidate/base was 0.999050x for JetStream, 1.001361x for Kraken, and
1.014847x for SunSpider. SunSpider `string-unpack-code` was 1.308688x, while
the intended closure-related direction did not produce a repeatable suite-wide
gain. These are informational neutral-shell ratios, not official suite scores,
but they independently agree that the unit is not general performance
progress. Commit `c8f02e33` therefore reverts Unit 74.

Hosted broad raw/report SHA-256 are
`c85ee7ca801a22c04ab1dcdcf10a20767daade112ef32241b2ea24b73c3b8203`
and
`0c3660ddf24ee5e95fed4da8f72368f8f67a3848f619e425318fb3665d8925ef`;
hosted external raw/report SHA-256 are
`469fe76f84cb9dfe0d32c28669ba1c2b357836cbbf305e3fa9ff2c60c96fe979`
and
`4cd1b4248d276a37515779dc3acdd7ca8d678d2a21cb0c6da9a57890d10427ea`.
Candidate/base/QuickJS-NG executable SHA-256 are
`34d2b53b311d0af6817d0b8f4984df27840ceaae62222773f58c93ea1d784282`,
`efd2be5bc2bfc3d8c0b048d74091747dc1310d05c122b62419a8fb6a5e2dea68`,
and
`8614a5a91e3476db1a1300b0969387b85e0716a836f799cf243a80d4d1f27699`.

### Unit 75: accepted streaming JSON serialization

Runtime commit `28cbf049` makes `JSON.stringify` append directly to one output
`String`. Recursive array/object serialization no longer constructs a fresh
`String` for every node, collects object members in a temporary vector, joins
temporary fragments, or allocates a second quoting buffer. The same mechanism
handles all serializable values and preserves replacer, pretty-printing,
omission, escaping, astral-pair, and lone-surrogate behavior. It contains no
benchmark identity, source path, iteration count, input-size threshold,
checksum, or expected-result condition.

The full local gate passed 1,403 runtime tests, all 198 benchmark-tool tests,
all 5,139 curated Test262 cases, formatting, Clippy, agents, and file-size
checks; `compare-qjs.sh` also passed. Main CI run `29695787335` and Test262
Coverage run `29695892451` passed. Performance Preview run `29695787364`
completed successfully. Its skipped steps are expected conditional paths:
QuickJS-NG was restored from a validated cache, PR/fork-only jobs do not run on
a trusted main push, and already-populated caches are not saved again. The
actual three-engine measurement and evidence upload steps passed.

The hosted external result supplies the causal acceptance evidence. Exact
candidate `28cbf0490ef605e76514101ddb3411a13b10124d` was measured against exact
base `a4ac3833be2a35383c023900bb21081a2343848d`. Kraken
`json-stringify-tinderbox` fell from 411.400 ms to 213.864 ms:
**0.519846x** candidate/base, or 48.0% lower wall time. Its ratio to pinned
QuickJS-NG improved to **1.903608x**. The distinct JSON parser case remained
near the previous level; all 5 JetStream and all 26 SunSpider cases remained
comparable, and Kraken retained the same 7/14 candidate/base coverage. The
diagnostic candidate/base geometric means were 1.003349x JetStream,
0.911213x Kraken, and 1.000409x SunSpider. Those unrelated complete suites are
effectively neutral around the large exact-path improvement, so this is
accepted as externally demonstrated general-engine progress.

The hosted broad run was physically complete: 225/225 measurements, 75/75
linearity checks, and three valid blocks. It measured 0.990982x
candidate/base with a 95% confidence interval of [0.981362x, 1.023412x], and
0.343968x candidate/QuickJS-NG. That internal aggregate does **not** complete
this goal. The mandatory external preview still measured qjs-rust at
**8.722909x** QuickJS-NG for JetStream, **5.032326x** for the incomplete
Kraken comparison, and **8.713730x** for SunSpider; QuickJS-NG won every one of
the 38 comparable external cases. Unit 75 therefore closes only one general
serialization bottleneck. The campaign remains open until the broad critical
families and independent external workloads both demonstrate the required
general performance, rather than an internal-portfolio-only score.

Hosted broad run ID is `67af037c-60e4-4048-8952-e71c572becf6`; broad
raw/report SHA-256 are
`d155f202821204c0505a6942994c6098b15c1bf173d4c9def8e4e7358c195763`
and
`c2fea3437f7ee8b46ba8a7f7cd0407208200e820b5af1f08035de1bea12c28a5`;
hosted external raw/report SHA-256 are
`69cd3364c16fe93a147b945f8652262e4fda28a7143166598684d0aab3c2c8a5`
and
`a49ca465e14e69ca512b695e29bcea22f6ce27b43eb98ca203930ecff2136e27`.
Candidate/base/QuickJS-NG executable SHA-256 are
`0c9e51565c2972e01ed168173f9a3712bdf4594a9bd00fff2a23f631aa470a16`,
`efd2be5bc2bfc3d8c0b048d74091747dc1310d05c122b62419a8fb6a5e2dea68`,
and
`8614a5a91e3476db1a1300b0969387b85e0716a836f799cf243a80d4d1f27699`.

### Unit 76: accepted preclassified direct-call realm upvalues

Runtime commit `0189845f` classifies realm-backed received-upvalue slots once
when a function is created. Direct calls reuse that fixed mask instead of
repeating module-import and sloppy-global classification for every invocation;
the empty-import path also bypasses an otherwise useless lookup. This is a
function/environment representation change used by every eligible direct call.
It contains no benchmark identity, source path, recursion check, iteration
count, input-size threshold, checksum, or expected-result condition.

The full local gate passed 1,404 runtime tests, all 198 benchmark-tool tests,
all 5,139 curated Test262 cases, formatting, Clippy, agents, and file-size
checks; `compare-qjs.sh` also passed. Isolated branch CI run `29698934914`,
main CI run `29699035809`, and Test262 Coverage run `29699134087` passed.
Performance Preview run `29699035799` completed successfully. Its skipped
steps are expected conditional paths on a trusted main push and cache hit; the
actual three-engine builds, measurements, reports, and artifact uploads passed.

The independent external preview confirms the motivating general call-path
effect. SunSpider `controlflow-recursive` fell from 157.097 ms to 146.323 ms,
or **0.931418x** candidate/base (6.9% lower wall time). Across complete suites,
candidate/base geometric means were 0.992738x for all five JetStream cases and
0.994731x for all 26 SunSpider cases. Kraken's eight mutually comparable cases
were 0.996665x. No external case improved enough to approach the campaign
target: candidate/QuickJS-NG remained 9.288036x on JetStream, 6.102195x on the
incomplete Kraken comparison, and 8.700169x on SunSpider, with QuickJS-NG
winning 38 of 39 comparable cases.

Kraken `audio-oscillator` is explicitly excluded from that comparison because
it lies on the 15-second timeout boundary, not because it establishes a
coverage regression. Candidate capability sampling timed out at 15.0018 s;
base capability sampling completed at 14.9353 s, but two of its three formal
samples also timed out at 15.0014 s and 15.0018 s. This threshold instability
must not be reported as stable candidate or base coverage.

The hosted broad diagnostic was physically complete with 225/225 measurements,
75/75 passing linearity checks, and three valid blocks. It measured 0.989345x
candidate/base with a 95% confidence interval of [0.982844x, 0.994661x], and
0.360526x candidate/QuickJS-NG. Its health is deliberately `inconclusive`:
three preview blocks are a non-claim cohort, not the fixed-hardware 30-block
claim protocol. The internal call family improved to 0.957789x candidate/base,
while the binding family measured 1.045998x; individual captured-read/write
ratios were 1.087975x and 1.187982x. Those loop-body cases execute call setup
only once and conflict in direction with the external recursive-call result,
so they are retained as an explicit regression watch rather than used to
reject the externally confirmed mechanism. The next unit must remeasure them;
this unit is not evidence that the campaign goal is complete.

Hosted broad run ID is `013d2508-899f-4ef0-9312-cf323ad45000`; broad
raw/report SHA-256 are
`5ae6db5df6a969878d40989b27a73d99edb395b3d68c73a346647aeaba1b6f5f`
and
`25daba07d6c6c8dd4ae7542b3eae6d3e32e96f925896901bec00a61c24b13d36`;
hosted external raw/report SHA-256 are
`861b530f808824c24afa0180ce1b16cac90a826ecf06f602f8248afe89884811`
and
`6f0d3677205dfb44c605cfd9bc79f783b340f833b58e0ba5cc52ba67a321a863`.
Candidate/base/QuickJS-NG executable SHA-256 are
`479cb674ff8c2e029c74f1a249b15dd961d5ac58202c84b3b97a2c7d735e3e45`,
`0c9e51565c2972e01ed168173f9a3712bdf4594a9bd00fff2a23f631aa470a16`,
and
`8614a5a91e3476db1a1300b0969387b85e0716a836f799cf243a80d4d1f27699`.

### Unit 77: accepted shared empty direct-call environment metadata

Commit `625b19fde62e664b0408217673ca9dd3c3df77fc` made the direct-call
environment reuse realm-owned canonical empty name sets for local-lexical and
local-declaration metadata, while passing the function's actual module-import
metadata through directly. Copy-on-write preserves mutation semantics. This
removes three empty-container allocations from every eligible direct leaf call;
it is a general runtime representation change with no benchmark identity,
source path, recursion shape, iteration count, input threshold, checksum, or
expected-result condition.

The authoritative external preview is the acceptance evidence. Against Unit 76
base `87a1063710c049772a72d72c671128cbac0aa222`, all five JetStream subset
cases improved, for a 0.972380x candidate/base diagnostic geometric mean.
SunSpider improved on 18 of 26 cases, for a 0.987360x suite geometric mean;
the motivating but independently sourced `controlflow-recursive` case fell
from 134.483 ms to 119.009 ms, or **0.884932x** (11.5% lower wall time).
Kraken's eight mutually comparable cases measured 0.998421x, with the same
capability coverage as base: six cases timed out for both engines, and no new
candidate-only timeout appeared. These multi-suite effects establish that the
optimization generalizes beyond the internal micro portfolio.

The hosted broad diagnostic was physically complete with 225/225 measurements,
75/75 passing linearity checks, and three valid blocks. Its 1.001172x
candidate/base ratio has a 95% confidence interval of [0.997016x, 1.001172x]
and is treated as neutral; it is not allowed to override the external evidence.
The binding family improved to 0.967602x, while the call family measured
1.015452x and remains a regression watch. Candidate/QuickJS-NG was 0.359719x
on the internal portfolio, but the external suites remain much slower:
9.097435x on JetStream, 6.270528x on the incomplete Kraken comparison, and
8.862341x on SunSpider, with QuickJS-NG winning all 39 comparable cases. The
campaign goal is therefore still far from complete.

The isolated branch full check passed 1,405 runtime tests, all 198
benchmark-tool tests, all 5,139 curated Test262 cases, formatting, Clippy,
agents, and file-size checks; `compare-qjs.sh` also passed. Isolated branch CI
run `29700684387`, main CI run `29701341940`, Test262 Coverage run
`29701453804`, and Performance Preview run `29701341938` all completed
successfully. A redundant main-worktree local recheck was interrupted after the
macOS loader stalled before starting a test binary; the exact branch and main
remote gates provide the completed verification rather than concealing that
local infrastructure anomaly.

Hosted broad run ID is `86d58dd9-f282-4f32-b840-a4c72059e52c`; broad
raw/report SHA-256 are
`803c007875c1e2ea1fd8bf8c4ce10c065d6c7534fa7e13af73db65fb6d61dc01`
and
`ebaad42ddaa0d1968ad77e78071845ee6d8d68a34eba11589f980dc3c60e39af`;
hosted external raw/report SHA-256 are
`7318a565f92fab54aafcc473300b86d06265fa8f863047a0d77f55241c7932e5`
and
`438c6b2db031195307bb2c868e02d3f9e8a2a5c91a17959558002c2b0f655959`.
Candidate/base/QuickJS-NG executable SHA-256 are
`9a61fef261b458b5f8b805ae235c7bd41bedd6953d2a4af31ee138ebde42161e`,
`479cb674ff8c2e029c74f1a249b15dd961d5ac58202c84b3b97a2c7d735e3e45`,
and
`8614a5a91e3476db1a1300b0969387b85e0716a836f799cf243a80d4d1f27699`.

### Unit 78: accepted compile-time direct-call parameter slots

Commit `b99d5e43a81bba86f5189c940d1408110ae0c52e` records the local slot for
each positional parameter when function bytecode is compiled and seeds direct
VM frames from that indexed vector. Previously every eligible invocation
walked the parameter AST and repeated a string-keyed local-slot lookup for each
argument. Duplicate sloppy parameters retain their positional entries and are
seeded in order; the numeric-leaf parameter map deliberately selects the last
duplicate position. The existing bytecode vector is reused, so the function
object hot header does not grow. The change contains no benchmark identity,
source path, recursion check, input threshold, checksum, or expected result.

Independent external evidence is the acceptance criterion. A local 31-sample
alternating run of the fixed SunSpider `controlflow-recursive` source measured
76.551 ms candidate versus 81.147 ms Unit 77 base, or 0.943369x. The hosted
x86 preview confirmed the direction at 134.480 ms versus 138.896 ms, or
**0.968206x** (3.2% lower wall time). A fresh five-second sample of the
amplified external source also removed the prior parameter-name hashing and
`HashMap` lookup symbols from the top-of-stack hot list, matching the intended
general mechanism.

All three hosted external suite diagnostics were neutral or faster against the
Unit 77 base. JetStream's five cases measured 0.986562x candidate/base with
four candidate wins; Kraken's seven mutually comparable cases measured
0.994085x with five candidate wins; and all 26 SunSpider cases measured
0.997293x with 17 candidate wins. Kraken capability was symmetric: the same
seven cases timed out for candidate and base, including `audio-oscillator` on
the 15-second boundary, so the smaller comparable cohort is not a candidate
coverage regression.

The hosted broad diagnostic was physically complete with 225/225 measurements,
75/75 passing linearity checks, and three valid blocks. It measured 0.984370x
candidate/base with a 95% confidence interval of [0.981254x, 0.985314x]. Its
health remains deliberately `inconclusive` because three blocks are a preview,
not a claim cohort. The binding family improved to 0.855519x, but call and
builtin measured 1.047157x and 1.061698x. The unrelated `array_index_of` case
also moved to 1.126158x despite this unit not touching builtin execution; these
internal movements remain explicit regression watches rather than a reason to
select a favorable internal-only story.

The campaign target is still far away. Candidate/QuickJS-NG was 0.355666x on
the internal portfolio, but external candidate/QuickJS-NG geometric means were
8.287110x on JetStream, 4.852732x on the incomplete Kraken comparison, and
8.323470x on SunSpider. QuickJS-NG won all 38 comparable external cases.

The isolated branch full check passed 1,406 runtime tests, all 198
benchmark-tool tests, all 5,139 curated Test262 cases, formatting, Clippy,
agents, and file-size checks; `compare-qjs.sh` also passed. Isolated branch CI
run `29702791971`, main CI run `29703032108`, Test262 Coverage run
`29703142411`, and Performance Preview run `29703032123` all completed
successfully. A redundant main-worktree check was interrupted when the local
macOS filesystem left Clippy compiler processes in prolonged uninterruptible
I/O; the exact isolated local gate plus isolated and main remote gates are the
completed verification evidence. GitHub's most recent 100 runs contained no
failure, cancellation, or timeout at acceptance time.

Hosted broad run ID is `a2d0ab7a-2cf5-46bf-b78b-92d4247babfd`; broad
raw/report SHA-256 are
`22f19414f665568ee02b33fdb6c3c79e8c25c8e4590841e5bcd5115c2c2da664`
and
`0bf5297fa542bc97fa5826631b4470f4219d30a128e3f00aa9e65fceb814f8ae`;
hosted external raw/report SHA-256 are
`8070a3a839f6ab6929c21231b994c75c53a425d54f2acd3dc1c22d0cfceb0017`
and
`de98371b746adc45514b39b0c5572a00ef6caf35fa028d8bbcff6f8d82215fc1`.
Candidate/base/QuickJS-NG executable SHA-256 are
`69359825cb8d010bd4a0ad11343de409f31781865b1371834739260f81154338`,
`9a61fef261b458b5f8b805ae235c7bd41bedd6953d2a4af31ee138ebde42161e`,
and
`8614a5a91e3476db1a1300b0969387b85e0716a836f799cf243a80d4d1f27699`.

### Units 79-80: rejected direct-call representation prototypes

Two local-only prototypes were rejected before integration. Unit 79 cached the
complete direct-leaf eligibility predicate on each function, but alternating
external screens measured 1.01947x candidate/base over 31 pairs and 1.01303x
over 41 pairs. Unit 80 replaced small direct-frame local vectors with inline
storage; both the four-slot and eight-slot variants regressed the fixed
external workload, at 1.01127x and 1.01357x respectively. Neither prototype
was committed or pushed. These rejections prevent metadata caching or inline
storage from being presented as progress merely because they appear cheaper
in isolation.

### Unit 81: accepted fixed small direct-call argument slices

Commit `619f98fc3df0286cf6653530dee7557979786079` avoids constructing a heap
`Vec<Value>` for eligible two- and three-argument direct leaf VM calls. The VM
pops those values into fixed stack arrays and passes a slice to the existing
direct-call implementation. The zero/one-argument path is unchanged, and the
new fixed-array helper is deliberately out of line because placing all shapes
in the main opcode dispatch body measurably slowed unrelated calls. The change
is selected only by ordinary call arity and the existing semantic direct-leaf
predicate; it contains no benchmark identity, source path, recursion test,
input-size threshold, checksum, or expected-result condition.

The mandatory external preview supplies the acceptance evidence. Against Unit
78 base `8a1f174f0f6689b5f9bf1b30cb011bd6a930e490`, all five JetStream subset
cases were comparable and measured 0.993956x candidate/base, with three
candidate wins. SunSpider's complete 26-case comparison measured 0.990886x,
with 15 candidate wins; the independently sourced `controlflow-recursive`
case fell from 131.858 ms to 125.358 ms, or **0.950703x**. The effect was not
confined to that case: `raytrace-public-class-fields` measured 0.981578x,
Kraken `imaging-darkroom` 0.973209x, SunSpider `crypto-md5` 0.965192x, and
`math-spectral-norm` 0.937482x. Kraken's seven mutually comparable cases were
near neutral at 1.003129x with four candidate wins. Capability was symmetric:
the same seven of fourteen Kraken cases completed for candidate and base, so
no candidate-only timeout or coverage loss was hidden in the aggregate.

The hosted broad diagnostic was physically complete with 225/225 valid
measurements, 75/75 passing linearity checks, and three valid blocks. It
measured 0.986032x candidate/base with a 95% confidence interval of
[0.982679x, 0.987919x]. Health remains deliberately `inconclusive` because
three preview blocks are a non-claim cohort. The motivating call family
improved to 0.929429x; allocation was 0.993758x and string was 0.980306x.
Binding, builtin, property, control, and array measured 1.004388x, 1.020559x,
1.017020x, 1.004152x, and 1.001539x respectively and remain explicit
regression watches. In particular, `captured_write` was 1.091107x and
`array_index_of` was 1.041356x even though this unit does not alter those
operations. The accepted external multi-suite improvement and the 0.929429x
call-family result outweigh those small unrelated movements, but the watches
must be remeasured by the next unit.

The campaign remains far from complete. Candidate/QuickJS-NG was 0.347249x on
the internal portfolio, while the external diagnostic geometric means were
8.186116x on JetStream, 4.828001x on the incomplete Kraken comparison, and
8.170998x on SunSpider. QuickJS-NG won all 38 comparable external cases.

The isolated full local gate passed 1,407 runtime tests, all 198 benchmark-tool
tests, all 5,139 curated Test262 cases, formatting, Clippy, agents, and
file-size checks; `compare-qjs.sh` also passed. Isolated branch CI run
`29705061727`, main CI run `29705376840`, Test262 Coverage run `29705469813`,
and Performance Preview run `29705376810` all completed successfully. The
redundant main-worktree pre-push hook was interrupted only after the SSH remote
closed its idle connection during a third full local rebuild; the already
completed exact isolated gates and both remote CI layers are the authoritative
verification, and no CI job was skipped or ignored.

Hosted broad run ID is `90518955-a23d-413c-88b9-2c0a92c157a0`; broad
raw/report SHA-256 are
`c49720cb97e48d924a70f8198a707d43f5421ab6308e5849b447b44f7f87e59d`
and
`6171d0030ddce38eb8153e0b2eb0872c0db5f238d57ae7dd393d0d8f506b9c8f`;
hosted external raw/report SHA-256 are
`3cfc396c632eb579d56e4ae1324d44528dbf004b288c8894e0d3918b84246933`
and
`ac6d27454583d37973a732a46751baf9b83b96bfbd6a56ca4e92273a3bf8d4ed`.
Candidate/base/QuickJS-NG executable SHA-256 are
`1492ef71536f52b3694e04e0c5f9c3fd66943ee1c8b32224fb2fb1735b6837e9`,
`69359825cb8d010bd4a0ad11343de409f31781865b1371834739260f81154338`,
and
`8614a5a91e3476db1a1300b0969387b85e0716a836f799cf243a80d4d1f27699`.

### Unit 82: rejected bounded direct-call frame-storage pools

Runtime commit `d4d84b3bff5846582e9373b5ea8faf16b1a614ce` tested a
general allocation mechanism identified by independent SunSpider profiling.
It replaced the single retained operand stack with a bounded per-bytecode pool,
added a bounded pool for cleared direct-leaf local slots, and returned direct
leaf locals to the owning bytecode as soon as evaluation completed. Both pools
were capped at 64 retained frames and discarded allocations above 256 entries;
all values were cleared before reuse. The implementation contained no suite or
case identity, source path, recursion shape, iteration count, checksum, or
expected-result condition. Focused tests covered clearing, capacity bounds,
sequential reuse, and distinct live recursive frames.

Local screening favored the mechanism's independently motivated path. Forty-one
alternating fixed-source `controlflow-recursive` pairs measured 0.930967x by
median and 0.931467x by paired geometric mean. A complete one-block external
preview measured 0.990080x candidate/base on all five JetStream cases,
0.997636x on thirteen mutually comparable Kraken cases, and 0.990174x on all
26 SunSpider cases; `controlflow-recursive` was 0.935986x. The complete local
25-case broad diagnostic was near neutral at 1.001850x, with call at 1.002115x.
Its raw SHA-256 was
`16ab81c63df5ab862b49e13783753d52ccce2cf4f14218fba6645c5da5e1dbc2`.
These local measurements were screening evidence only.

Trusted-main Performance Preview run `29707728246` compared the exact
candidate against base `ca50e57d03e9bf2b95ecead5b6c5fef07a83d619` and
pinned QuickJS-NG `f7830186043e4488f2998759d60a514faf07cbc9`. Candidate,
base, and QuickJS-NG executable SHA-256 were
`8d97453c0052560b9ac71d8c73d679122a2f5038ad5bcaeb4e668a846bf54da0`,
`1492ef71536f52b3694e04e0c5f9c3fd66943ee1c8b32224fb2fb1735b6837e9`,
and
`8614a5a91e3476db1a1300b0969387b85e0716a836f799cf243a80d4d1f27699`.
All 225/225 broad measurements were valid, all 75 linearity checks passed, and
all three blocks were valid.

The hosted external preview reproduced and strengthened the local direction.
Candidate/base geometric means were **0.984034x** on JetStream with four of
five candidate wins, **0.995107x** on Kraken's seven mutually comparable
cases with three candidate wins, and **0.989483x** on all 26 SunSpider cases
with eighteen candidate wins. `controlflow-recursive` improved to **0.911533x**
(125.117 ms to 114.048 ms), while independently different allocation and
object-heavy cases also moved: JetStream `hash-map` was 0.957666x and
SunSpider `access-binary-trees` was 0.939958x. Capability remained symmetric;
no candidate-only failure or missing case manufactured those ratios.

The broad regression guard nevertheless rejected the unit. Candidate/base was
**1.009971x** overall with a 95% confidence interval of
[1.008544x, 1.013162x]. Call regressed to **1.042732x** and binding to
**1.019466x**; the direct-call cases `plain_function_call`, `method_call`,
`captured_read`, and `function_call_reordered` measured 1.070732x, 1.071304x,
1.079916x, and 1.070390x. Array and string were also above base at 1.015816x
and 1.018169x. Allocation, builtin, property, and control measured 0.975432x,
0.976982x, 0.986815x, and 0.998252x, but those wins cannot hide a significant
general call-path regression. This is exactly the campaign's two-sided
anti-overfitting rule: external workloads are mandatory, while broad-micro
remains a regression guard rather than an optional score to discard.

Candidate/QuickJS-NG remained far from the external completion boundary at
8.089199x on JetStream, 4.848733x on incomplete Kraken, and 8.159375x on
SunSpider; QuickJS-NG won all 38 comparable cases. The internal portfolio was
0.354123x candidate/QuickJS-NG, but that result cannot accept a candidate that
regresses its exact base.

The full local gate passed 1,409 runtime tests, 198 benchmark-tool tests, all
5,139 curated Test262 cases, formatting, Clippy, agents, file-size checks, and
all QuickJS-NG comparisons. Isolated branch CI run `29707633388`, main CI run
`29707728255`, and Test262 Coverage run `29707816149` were fully green. The
performance run ID was `dddc69a5-ecdf-4359-80a4-689e637c2059`; broad
raw/report SHA-256 were
`ea724078e4f8e9a28d6b7efa17ae5961119275bbfa441a3c7f18cfe3aef85b17`
and
`867a6dcb0744e4186a2f3043b5f03c444c34db4c5cb20ec406e9e224ddf3fcc3`;
external raw/report SHA-256 were
`96227ad7f8804f80075eb07907f2ce2f0f077dafa7d9eb24106abda4b2ca8816`
and
`86691e38d61d4a3d9aa6e245c26f16253b9687441606bf6e1ea95dc954cb64cb`.

Commit `1be491b75412a3d967739c2d4921d4069e3b7305` reverts Unit 82. Its
runtime tree is byte-for-byte identical to the accepted Unit 81 state at
`ca50e57d03e9bf2b95ecead5b6c5fef07a83d619`. Unit 82 therefore contributes
rejection and profiling evidence only; B3, B4, and B5 remain open. The next
unit must reduce externally observed frame/environment overhead without adding
per-call pool bookkeeping that regresses the already-fast direct-call cohort.
Revert CI run `29708634404` and Test262 Coverage run `29708721088` both
completed successfully.

### Unit 83: rejected declaration-free direct-eval writeback skip

An uncommitted Unit 83 prototype tested the next independently profiled
environment cost. Before direct eval, the caller VM exposes ordinary locals as
shared upvalue cells. For eval scripts with no `var`/function declarations, no
lexical declarations, and no direct binding writes, the prototype marked the
subsequent name-based whole-frame `apply_env` as redundant and refreshed only
realm-backed slot caches. Calls executed by the eval still mutated captured
cells and the shared realm in place. The guard used compiled semantic metadata,
not source text, suite identity, workload name, or expected output. Two focused
tests verified repeated reads and call side effects through caller cells; all
57 global/eval tests passed.

The mechanism did not survive the external-first screening rule. Exact release
candidate and accepted Unit 81 base binaries had SHA-256
`b1ba4faa51a320f321cd388440e446f97d07c80b7bdab721b35f3eec1ab82942`
and
`f448ee1b769ead47036010068b70064c9aab3e1a9c4e3e1c0ad32a85abc07313`.
An initial eleven-pair alternating SunSpider `date-format-tofte` run was only
about 0.994x by median. A fresh, already-warm 21-pair alternating run reversed
that small movement: candidate median was 418.319 ms, base median was 417.080
ms, or **1.002971x** candidate/base. Raw timing SHA-256 was
`ded5ca4a7523d7b1c9e68c6f7be786a3272c8d5f01d0c55ca4ac271068251cd0`.
A candidate profile confirmed that constructor-call environment application,
current-environment construction, hashing, and allocation still dominated; the
removed return-stage walk was not a material end-to-end bottleneck.

Unit 83 is rejected without a runtime commit, broad run, or hosted preview. A
semantically valid branch is not campaign progress when the independent
external workload is neutral. The next unit must remove the larger environment
construction or object/allocation mechanism itself instead of adding another
post-call shortcut.

### Unit 84: rejected lazy caller-name collection for direct eval

An uncommitted Unit 84 prototype targeted the remaining pre-evaluation name
scan rather than Unit 83's post-evaluation writeback. The current direct-eval
path always clones and hashes every visible caller binding, although that set
is consumed only by declaration validation, binding initialization, and
name-based writeback. The prototype used compiled hoist/write metadata to skip
that collection for read/call-only eval code. Ordinary reads still used the
supplied caller frame and functions invoked by the eval still mutated shared
upvalue cells. It contained no suite identity, source-text match, case name,
iteration threshold, checksum, or expected result. A focused test covered an
eval expression that both reads a local and invokes a closure that mutates it;
all 55 selected global/eval tests passed.

The independent external screen again found no material end-to-end benefit.
Exact release candidate and accepted Unit 81 base executable SHA-256 were
`c5a46bb513880c5a7dd21cd5db22f9c54fed5129c6cd5e46604cd5d8d1ccc279`
and
`6d97786319139b4ccabcfcf3fe687418a403b98ff51bde29b246396c3f86db55`.
After three warmups per binary and case, 21 alternating pairs measured paired
median candidate/base ratios of **1.001154x** for SunSpider
`date-format-tofte`, **1.000096x** for `date-format-xparb`, and **1.009086x**
for the independently different `string-tagcloud`. Candidate/base median wall
times were 416.464/416.198 ms, 107.934/108.179 ms, and 422.994/421.292 ms.
Raw timing-record SHA-256 was
`dbed25a65057aaa697370d08f630db7f7a6bc52bcb97ab0678f567b3b2e7a40c`.

Unit 84 is rejected without a runtime commit, broad run, or hosted preview,
and the prototype was removed. Eliminating an allocation visible in a sampled
profile is not sufficient when three independent external cases remain neutral
or regress. The next unit should target work that occupies a much larger share
of an external execution, especially general object representation/allocation
or the environment construction itself, rather than another small eval scan.

A temporary hit diagnostic on the restored accepted runtime then executed all
26 original SunSpider sources. None of the three existing counted-loop engines
(`NumericLoopPlan`, `ControlLoopPlan`, or `NumericMutationLoopPlan`) ran even
once. The diagnostic code was removed after the inventory. This zero-hit result
explains the internal/external split more directly than another micro-profile:
the broad portfolio's historically fast trace cohort is real, but the current
trace grammar does not generalize to even one complete SunSpider workload.
The next trace unit must therefore add a reusable semantic loop family selected
from external code and retain broad regression checks; adding another internal
case-specific pattern would not address B5.

### Unit 85: guarded simple numeric recurrence loops

Unit 85, runtime commit `7fa2cca7e233861091b612e970f9168377baf6b1`,
starts from an original external source shape rather than adding another broad
micro case. It recognizes a straight-line counted loop whose numeric
accumulator is updated from itself, the counter, a stable local, or a numeric
constant, followed by an incrementing or decrementing numeric counter. The
plan supports arithmetic, remainder, exponentiation, shifts, and bitwise
recurrences. It is selected from bytecode structure only: there is no source
text, suite or case identity, variable name, iteration threshold, checksum, or
expected-result check.

Runtime admission is deliberately semantic. The counter and accumulator must
be numbers; local slots must remain authoritative or stable realm bindings;
global sinks must be writable ordinary data properties; and accessors,
non-number coercion, `with`, or direct-eval state deopt before the first
iteration is consumed. The plan commits the final accumulator, counter, loop
result, upvalue/realm/module mirrors, and sloppy-global bookkeeping once at
loop exit. Focused tests cover arbitrary local and global names, arithmetic
and bitwise operators, increment and decrement, a dynamic accumulator limit,
zero iterations, string-addition deopt, and observable global accessor reads.

The motivating independent case is original SunSpider
`bitops-bitwise-and`, whose 600,000-iteration global recurrence was not
recognized by any previous loop plan. With exact release candidate/base
executable SHA-256
`8b701934d4bfdda3c6094c7b85d93d1a470142580fbbdfdda9b49818782dc12d`
and
`6d97786319139b4ccabcfcf3fe687418a403b98ff51bde29b246396c3f86db55`,
three warmups and 21 alternating pairs measured median wall times of
5.719/213.990 ms for candidate/base. The pinned QuickJS-NG median was 24.587
ms, giving **0.02673x base** and **0.23261x QuickJS-NG**. This is the first
external case in the v2 campaign with a directly caused qjs-rust win over
QuickJS-NG, but remains a focused local diagnostic rather than an acceptance
claim.

The complete 25-case local broad screen used one block and seed `20260724`.
All 75 formal measurements were valid. Candidate/base geometric mean was
**1.00058x**, while candidate/QuickJS-NG was **0.18542x**. Family
candidate/base ratios were allocation 1.00676x, array 0.99567x, binding
1.00475x, builtin 0.99614x, call 0.99640x, control 0.99847x, property
0.99944x, and string 1.01791x. This single-block run is an anti-regression
screen, not a stability claim; the hosted multi-block artifact decides whether
the unit is retained. Raw JSONL SHA-256 was
`b8394728cfc1f3c68e17cbf2e7d6e7e03363b681da6f28dad812e6e10e049335`.

Local `scripts/check.sh`, all 1,413 runtime tests, the staged 65-case Test262
slice, and `scripts/compare-qjs.sh` passed. Branch CI `29711304789` and main CI
`29712265686` passed every job. Main Test262 Coverage `29712373798` passed all
16 shards and the aggregate. Its commit-bound artifact reports 42,671 Rust
passes, one failure, zero timeouts, and one actionable QuickJS-NG gap, so this
performance change introduced no authoritative conformance regression.
Burndown JSON SHA-256 was
`881765e1ddae74a5359c0210dafb639c08098b3185a9882cf90c3fd900ff7e9c`.
Trusted-main Performance Preview run `29712265669` compared the exact candidate
against base `1be491b75412a3d967739c2d4921d4069e3b7305` and pinned QuickJS-NG
`f7830186043e4488f2998759d60a514faf07cbc9`. Candidate, base, and QuickJS-NG
executable SHA-256 were
`046945d520437da4a541fead5c6244876634dc0f55477c2228835680946ed330`,
`1492ef71536f52b3694e04e0c5f9c3fd66943ee1c8b32224fb2fb1735b6837e9`,
and
`8614a5a91e3476db1a1300b0969387b85e0716a836f799cf243a80d4d1f27699`.
All 225/225 broad measurements were valid, all 75 linearity probes passed, and
all three requested blocks were valid.

The hosted external result reproduced the motivating win:
`bitops-bitwise-and` fell from 319.491 ms on the exact base to 6.208 ms on the
candidate, versus 25.022 ms on QuickJS-NG. Its ratios were **0.01943x base**
and **0.24810x QuickJS-NG**. SunSpider's 26-case candidate/base geometric mean
therefore improved to **0.86273x**, but candidate/QuickJS-NG remained
**7.32254x**, with QuickJS-NG winning the other 25 cases. The independent
JetStream and Kraken directions were neutral to adverse at **1.00545x** and
**1.00280x** candidate/base, and still **8.36038x** and **5.87786x**
candidate/QuickJS-NG. Comparable coverage was unchanged: 5/5 JetStream, 9/14
Kraken, and 26/26 SunSpider cases.

The broad regression guard rejects Unit 85. Candidate/base was **1.00333x**
overall with a 95% confidence interval of [0.98588x, 1.00583x], but four
critical families were above the exact base: control **1.02625x**
([1.02611x, 1.02627x]), allocation **1.01297x** ([1.01297x, 1.02465x]),
array **1.01007x** ([1.00704x, 1.02162x]), and property **1.00084x**
([1.00021x, 1.00181x]). In particular, the independently shaped
`branch_arithmetic`, `array_write`, and `closure_allocation_call` cases
regressed to 1.05301x, 1.03044x, and 1.03419x. A single external 51x
candidate/base speedup cannot hide statistically clear regressions in general
control, array, allocation, and property work.

Broad raw/report SHA-256 were
`4391594126151a6b3f294b0db2a9245df69493f0565849a640bf6e44be94fa24`
and
`daf05d354da8c41c2e95f5eced595003d8c5ea17e33ebc1a8058f280d230a234`;
external raw/report SHA-256 were
`4dad12d57197023358ac722942eedbec7f7f96955c35814146e0eb6a5496feee`
and
`613a12c11304681e15846f34124b5b61d2a24a0649d0ab4e2fb2b2f4051c5867`.
Commit `8710d404248ecf2fb53046ba74bf250f360ef2f7` reverts Unit 85, restoring
the runtime tree byte-for-byte to accepted Unit 81. Unit 85 contributes a
useful loop-shape prototype and rejection evidence only. The next unit must
reduce a general mechanism shared by multiple independent external cases
without paying a new per-loop dispatch cost on unrelated control paths.

### Unit 86: stream top-level RegExp match priority

Unit 86, runtime commit `60eabcdb9cac449fc6ecd88f38d1a826f0d1d2eb`,
was selected from independent external profiles rather than an internal broad
case. Samples from original external `string-tagcloud` and `regexp-dna`
executions both concentrated in the native RegExp matcher, especially
`match_pattern` and simple-atom repetition. The implementation adds a
top-level first-match path that streams greedy or lazy simple-repetition
boundaries into the remaining pattern and stops at the first complete match in
ECMAScript backtracking priority. Nested and general matching retain the
existing all-state fallback. Selection depends only on RegExp structure and
matching semantics; it contains no benchmark identity, source path, fixed
input, iteration count, checksum, or expected result.

Focused tests cover greedy and lazy first-match and capture priority. Local
`scripts/check-touched.sh`, `scripts/check.sh`, all 1,408 runtime tests, all
5,139 selected Test262 cases, and `scripts/compare-qjs.sh` passed. Branch CI
`29716125491` passed all jobs before integration. Candidate and accepted Unit
81 base release executable SHA-256 were
`42d9a8f340a44ca1be80d905cd2400ac70c5d4746b33e78b42be5f85a98870f1`
and
`6d97786319139b4ccabcfcf3fe687418a403b98ff51bde29b246396c3f86db55`.

After three warmups, 11 alternating local pairs measured candidate/base
ratios of **0.89508x** for external `string-tagcloud` and **0.65990x** for
external `regexp-dna`. A complete one-block external screen reproduced the
direction at **0.886x** and **0.685x**, while the independently shaped
`string-validate-input` case also improved to **0.917x**. The external raw and
report SHA-256 were
`b30b06964606b4e6384b5360879c7ca4858a60789e0cfc56442b0568de9d6458`
and
`5592bebc0e7fb1880131b7fd1631607da1cac1a73971eab91834d49e84350b02`.
Complete-suite candidate/base diagnostic means were 1.001x for JetStream,
1.001x for Kraken, and 0.972x for SunSpider.

The complete three-block local broad guard recorded 225/225 valid
measurements. Candidate/base was **1.00158x** overall; family ratios were
allocation 0.99371x, array 0.99613x, binding 1.00385x, builtin 1.00074x, call
1.00471x, control 1.00200x, property 1.00628x, and string 0.99844x.
Candidate/QuickJS-NG was 0.18552x overall. Raw JSONL SHA-256 was
`02bcaa56ab77f14ef3e4f2d0ed044948d3b0ba0005d6ff3719273a656fbd084e`.
These local broad offsets are small but do not prove neutrality; trusted-main
Performance Preview `29716273894` is the acceptance authority. It compared the
exact candidate against accepted base
`8710d404248ecf2fb53046ba74bf250f360ef2f7` and pinned QuickJS-NG
`f7830186043e4488f2998759d60a514faf07cbc9`. Candidate, base, and QuickJS-NG
executable SHA-256 were
`c18151cc0e7af31c9ee0ef30f24b8f7ffd45ef1c9e20f2f9aa98a6f9e8711c64`,
`1492ef71536f52b3694e04e0c5f9c3fd66943ee1c8b32224fb2fb1735b6837e9`,
and
`8614a5a91e3476db1a1300b0969387b85e0716a836f799cf243a80d4d1f27699`.
All 225/225 broad measurements, 75/75 linearity probes, and three requested
blocks were valid.

The hosted external result reproduced the general RegExp benefit across three
independent cases: `regexp-dna` was **0.642x**, `string-tagcloud` **0.870x**,
and `string-validate-input` **0.916x** candidate/base. SunSpider improved to
**0.974x** over all 26 comparable cases, while Kraken was neutral at 0.997x
over 7/14 and JetStream was slightly adverse at 1.006x over 5/5. qjs-rust
remained far from the external completion boundary at 7.975x, 4.865x, and
8.199x QuickJS-NG for SunSpider, Kraken, and JetStream respectively, with zero
qjs-rust wins against QuickJS-NG.

Unit 86 is nevertheless rejected by the two-sided broad regression guard.
Candidate/base was 0.98139x overall with a 95% confidence interval of
[0.98139x, 0.98577x], but the critical `string` family regressed to
**1.05709x** with a wholly adverse interval of **[1.04694x, 1.07083x]**. A
real external win cannot override a statistically clear critical-family
regression, just as an internal win cannot override neutral external evidence.
Broad raw/report SHA-256 were
`8791ada02999d38ee3d75232edf874b0d8127e3b587c90a9c0edace82a9cd49e`
and
`b093d8cda811117fe1a516d83e7d4bc48b67f34f38cd45e00e4df2884acd9d97`;
external raw/report SHA-256 were
`174155f1b4c93d70e068c50beec824368d5abe38aff7495695717acc09f16d09`
and
`4b52d7b4f23d83bacdc1c795db7c98e7e98a4e871154df07a84f432e2b0181e8`.

Main CI `29716273925` passed all jobs. Test262 Coverage `29716402425`
passed all 16 shards and aggregation; its commit-bound artifact reports 42,671
Rust passes, one failure, zero timeouts, and one actionable QuickJS-NG gap.
Burndown JSON SHA-256 was
`d2e7e348d2f243a8cff46163f8e1ce286c7d38722ffcb660e0e54bc2901bd293`.
Commit `78c03c19474ea3d93b3904e70f398e3a7f348499` reverts Unit 86 and restores
the three matcher files byte-for-byte to accepted Unit 81. A cumulative
follow-up may reintroduce the semantic first-match stream only if an additional
general external-profiled change clears both the external and broad guards
against that accepted base.

### Unit 87: reuse prepared RegExp input slices

Unit 87 reintroduces the Unit 86 first-match stream as commit
`e58c57ea` and adds commit `c804af0ff35af2a12ae19b0e5ab764227e0c006f`.
The additional mechanism was selected from post-Unit-86 external profiles:
`string-tagcloud` placed 62 top-of-stack samples, about 21% of the sample, in
`input_slice -> string_code_units`; `regexp-dna` independently placed 23
samples there. The prepared native global-RegExp path already decodes its input
once for matching, but result materialization decoded the complete input again
for every match and capture merely to extract a short substring.

`PreparedInput::slice` now materializes matcher-indexed substrings directly
from that existing view. Non-Unicode entries retain one UTF-16 code unit,
including lone-surrogate sentinels; Unicode entries retain one scalar value.
Whole matches, captures, and named-group values in the strict original-native
global replace path share it. Custom `exec`, RegExp-like objects, and all
observable fallback protocol remain unchanged. Focused tests cover Unicode
substrings and both halves of an astral character in non-Unicode code-unit
mode. The implementation has no benchmark identity, source path, input
constant, iteration count, or expected output.

The clean complete local external screen compared cumulative candidate SHA-256
`29edf05c3bd4abb3c13fde69dd27d528923504791a1e250f4cb44c3425660a84`
against accepted Unit 81 base
`6d97786319139b4ccabcfcf3fe687418a403b98ff51bde29b246396c3f86db55`.
JetStream was 0.998x candidate/base over 5/5 cases, Kraken 0.994x over
13/14, and SunSpider **0.919x** over 26/26. The independently improved cases
were `regexp-dna` **0.554x**, `string-tagcloud` **0.625x**,
`string-validate-input` **0.900x**, and `string-unpack-code` **0.396x**.
No comparable coverage was removed. External raw/report SHA-256 were
`0d1903bf96ffc683da150361ec64e7de943720978ce41d8c0b26b588e8cccac5`
and
`ff12b07add054b1847cba14d29577e0a7f34bfbb5b8c96081354ad2c12f75668`.

A complete clean-commit local broad screen produced 1,621 protocol records,
225/225 valid formal measurements, all 25 cases, and three blocks. Local
direct binaries have no trusted build receipts, so the report validator
correctly rejected `provenance_status=unverified`; these numbers are screening
evidence only. Recomputed paired case medians gave 1.00117x candidate/base
overall and 0.18407x candidate/QuickJS-NG. Candidate/base family ratios were
allocation 0.99364x, array 0.99383x, binding 1.00478x, builtin 0.99788x,
call 0.99827x, control 1.00163x, property 1.01962x, and string 0.99628x.
The changing small family directions across two complete local runs reinforce
that only a receipt-bound hosted report may accept or reject the cumulative
candidate. Local raw JSONL SHA-256 was
`5e5edd8e7ddadda532ea7467d0bc3c7c7e3d94b750f316f3f2d219ab5a1dc46c`.

`scripts/check-touched.sh`, `scripts/check.sh`, all 1,409 runtime tests, all
198 benchmark-tool tests, the 5,139-case Test262 subset, and
`scripts/compare-qjs.sh` passed. Branch CI `29719854934` and main CI
`29720061660` passed every job. Main Test262 Coverage `29720196801` passed all
16 shards and aggregation; its commit-bound artifact reports 42,671 Rust
passes, one failure, zero timeouts, and one actionable QuickJS-NG gap.
Burndown JSON SHA-256 was
`35495a77398ec22119ec2b00c5624cea4ef1173203dc7b948bbeaa9261ec6812`.
Trusted-main Performance Preview `29720061573` accepted Unit 87 after two
complete attempts over the exact same candidate, base, and QuickJS-NG
executables. Candidate, base, and QuickJS-NG binary SHA-256 were
`5a7ad0f43fe96a8195e685911c721f39db4e3e95fa5b06c7d5b683d9083a36b4`,
`1492ef71536f52b3694e04e0c5f9c3fd66943ee1c8b32224fb2fb1735b6837e9`,
and `8614a5a91e3476db1a1300b0969387b85e0716a836f799cf243a80d4d1f27699`.
The first broad attempt measured **0.98690x** candidate/base overall with a
95% confidence interval of [0.98307x, 0.98892x]; the independent rerun measured
**0.98851x** with [0.98477x, 0.98952x]. Both attempts had 225/225 valid
measurements, all 75 linearity probes passing, and all three blocks valid.

The external direction also reproduced. The first and second attempt suite
ratios were 1.004x and 1.005x on JetStream, 0.996x and 0.992x on the mutually
comparable Kraken cases, and **0.940x** and **0.942x** on all 26 SunSpider
cases. The independently selected RegExp/string cases reproduced large gains:
`regexp-dna` was 0.536x then 0.535x, `string-tagcloud` 0.741x then 0.714x,
`string-unpack-code` 0.538x then 0.543x, and `string-validate-input` 0.903x
then 0.928x candidate/base. Comparable coverage did not decrease.

The broad `builtin` family was a repeatable regression watch at 1.00630x
([1.00440x, 1.01061x]) and 1.00597x ([1.00562x, 1.00641x]); the second
attempt also placed control at 1.00102x and string at 1.00419x, with both
confidence intervals crossing 1.00x. These cases do not execute the changed
RegExp path, the portfolio overall improved on both attempts, and the maximum
repeatable family movement was 0.63%, so this is retained as a small
code-layout/hosted-runner regression watch rather than classified as a
material general-path regression. The next unit must remeasure `builtin`; a
larger or compounding regression is not acceptable.

Unit 87 is accepted as general-engine progress because the implementation is
semantic rather than workload-specific, the motivating benefit reproduced
across four independent external cases on two hosted attempts, overall broad
performance improved on both attempts, correctness remained green, and no
coverage was removed. It does not approach campaign completion: the rerun was
still 8.291x QuickJS-NG on JetStream, 4.861x on incomplete Kraken, and 7.703x
on SunSpider, with QuickJS-NG winning all 38 comparable external cases.
First-attempt broad raw/report SHA-256 were
`8c713f5fb204c06a86f1f9231762f060ff9df53383d7ed2e6126a899f7a5063b`
and `9b08d5f64690e4128ccb439d34ae091285ac453600026ed88bc4b5531f9afdc2`;
external raw/report SHA-256 were
`7ce61fb4bf36b611f2e43f4256b801d5237cb8f442ceab54b1074bfb81580025`
and `565fc3e8b20c74f612eaf821a1d23422587d5f4e63a43a1aee46f0ec38d693d0`.
Rerun broad raw/report SHA-256 were
`b0e0bfcd9f331edb91ba713e1282cdeb829fa38464214bf63b6f1891e47928ed`
and `c6555bdc0ce1c7a6b71999b9850ae4cda90f6de9df2cac154418f4ac18311005`;
external raw/report SHA-256 were
`8f1db47353a8be9d39be72449237fd3126c1b9559deff8b635ea0006193d0ebc`
and `bdeabc41c79c3904748c4fb3c21d912b4d5af7bda09e72216ebe7d18790ec547`.

### Unit 88: rejected exact-once RegExp step

Unit 88 tested a general exact-once matcher path for simple RegExp programs.
It classified a conservative capture-free subset and executed its first
matching step directly instead of constructing temporary match-state vectors.
The implementation contained no benchmark name, source path, fixed iteration
count, or expected output, and focused tests plus the local full correctness
gate passed. Branch and trusted-main CI also passed, but correctness alone was
not sufficient to accept a performance unit.

The first trusted-main Performance Preview attempt for commit `cd5ab038` was
not a performance conclusion. QuickJS-NG `array_read` was timer-limited in
block 2, invalidating the entire block and leaving only 150/225 formal
measurements. The workflow correctly failed instead of publishing a partial
broad comparison. The failed-attempt broad raw/report SHA-256 were
`031eb96878c8718c04b849d7442b09f7a680c754dc4e20d80649eaead8df45a8`
and `c2031fc0bd00190b754fd9971d97c61567edaf4137e951a5e2b9f051e56ec8fb`.
This was a benchmark-host interruption, not a code or correctness failure, so
only the failed job was rerun against the same commit.

The rerun completed all 225 measurements, all 75 linearity probes, and all
three blocks. It found a material **1.02089x** candidate/base broad regression
with a 95% confidence interval of [1.01089x, 1.03140x]. Call regressed to
**1.05498x** and binding to **1.04906x**; `captured_read` was 1.16769x,
`function_call_two_args` 1.12522x, `captured_write` 1.08429x, and the plain,
method, and reordered-call cases were about 1.070--1.071x. These cases never
execute the changed RegExp matcher, so the result exposed a material general
hot-path/code-layout cost rather than an allowed RegExp tradeoff.

The external rerun did confirm a real focused benefit: `regexp-dna` improved
to 0.47287x candidate/base, while suite geometric ratios were 0.98727x for
JetStream, 0.98945x for Kraken, and 0.96239x for SunSpider. However, QuickJS-NG
still won every comparable external case, with qjs-rust/QuickJS-NG at 8.218x,
4.820x, and 7.418x respectively. A large isolated RegExp win cannot compensate
for a statistically clear regression across unrelated call and binding paths,
so Unit 88 was rejected under the general-optimization rule.

Rerun broad raw/report SHA-256 were
`98c543f7d1567d0928b12ce1acb8ae1c06c57de043be495785b1957cedde4db1`
and `b743b4315da3cdfd97e8379657cebfe147fea408f3800ca22a65eb288a239b73`;
external raw/report SHA-256 were
`cc744cc42a3538848d4f7ab5f4b836ea4bff86a5f64ad4d6394c6ab30b111136`
and `2d0f7c788e65a13625ee90e679fcf3ba14c8b9effad5f5318cd9a9a28cb9edb5`.
Candidate, base, and QuickJS-NG executable SHA-256 were
`f8537a5bca5184824c512a4706e58afdb50e28acdde1cdda7d39e40455b1e80f`,
`5a7ad0f43fe96a8195e685911c721f39db4e3e95fa5b06c7d5b683d9083a36b4`,
and `8614a5a91e3476db1a1300b0969387b85e0716a836f799cf243a80d4d1f27699`.

Commit `ac509225` reverts Unit 88. Main CI `29728886300`, Test262 Coverage
`29729084748`, and Performance Preview `29728886296` all passed after the
rollback. The rollback candidate executable exactly matches accepted Unit 87
and measured 0.97737x versus the rejected Unit 88 base, recovering the inverse
call and binding losses. Rollback broad raw/report SHA-256 were
`cfddef31b8e3bb1bbe2d793023d1ce28de989cfd39bb3bf0ec10fb363f7c80e6`
and `8618746a33b34fbadb90fbfd1e8a87b8bb2afeb4d97b5d8e176db48fb8b1eb44`;
external raw/report SHA-256 were
`da3cf5524580cdb10caa462815234d3191961aaf4bba81d21f0fd2aa31f55bc0`
and `8a88701529d2de6bf4f54edb131439d3b33a362d57ee619bcb3436bbfe9fe8b4`.

### Unit 89: rejected repeated-eval bytecode cache

An uncommitted Unit 89 prototype added a bounded per-realm cache for successful
`eval` compilations. Its key included source, direct/indirect mode, strict and
function/method/constructor/field-initializer parser context, and the visible
private names. Syntax failures were not cached, and each hit returned a fresh
bytecode clone so eval-local mutation and template-object identity remained
per evaluation. Focused tests covered current local values, strict/sloppy
separation, distinct private environments, and fresh tagged-template arrays.
Formatting, focused tests, Clippy, and a release build passed.

The mechanism came from the external `date-format-tofte` profile, where
repeated same-source evals spent time parsing and compiling. Sixteen balanced
targeted pairs initially measured 0.97028x candidate/base for that case and
0.99331x for `date-format-xparb`. The complete one-block external screen did
not reproduce a material suite-level improvement: JetStream was 0.99144x,
Kraken 1.00507x, and SunSpider 0.99842x; `date-format-tofte` itself was
0.99698x in that noisier complete run. Coverage did not decrease. External
raw/report SHA-256 were
`80eb3dae7966984bc11b3efa22098d9db87ca84ab3eb441de27e53e153c47c51`
and `51b44fe0349d70cfd1c82b9cbd0d964f02b4a3b20a5161e57e4aae58edcccc45`.

A complete local three-block broad screen had 225/225 eligible measurements
and all 600 linearity samples successful, but the fixed-host execution was too
unstable to clear the regression guard. Its provisional overall ratio was
1.01288x and its three block ratios were 0.99845x, 1.04988x, and 0.99129x.
The middle block drove `function_call_two_args` to 1.70695x and
`captured_read` to 1.62129x even though the surrounding blocks were near one;
the cache cannot run in those cases. Without a repeatable external benefit or
a clean broad result, the prototype was rejected rather than pushed. Its raw
JSONL SHA-256 was
`f09b43216eda38c416db0c4299d7ab3e9c9dabb6db0b38c3815f02ebd408e456`;
candidate/base executable SHA-256 were
`52c91b328f0eb0313c5bd8486221d67b2df0a9de817eb2833801d5e2628ae0c6`
and `29edf05c3bd4abb3c13fde69dd27d528923504791a1e250f4cb44c3425660a84`.
The isolated worktree is retained only for diagnosis; none of its four runtime
files entered `main`.

### Unit 90: rejected shared class-instance metadata

Unit 90 tested a general allocation optimization for class construction. Commit
`178e5093` shared immutable instance-element metadata behind `Rc<Vec<_>>`
instead of cloning the full vector into every constructed instance. The
mechanism contained no benchmark identity or workload-specific branch, and its
focused class-field tests plus the local correctness gate passed.

Trusted-main Performance Preview `29736942043` did not reproduce useful
general-engine progress. Broad candidate/base was 1.0097x. The external suite
ratios were 0.998x for JetStream, 1.010x for Kraken, and 1.009x for SunSpider;
the intended `raytrace-public-class-fields` workload itself was 0.996x. The
candidate still trailed QuickJS-NG by 8.45x, 4.99x, and 8.01x on the three
external suite diagnostics. A representation change with no repeatable target
benefit and small broad/external regressions is not campaign progress, so the
optimization was rejected.

Commit `40aa0424` reverts Unit 90 while retaining the independent realm fix
described below. Main CI `29740806181`, Test262 Coverage `29740972027`, and
Performance Preview `29740806114` all passed. The rollback comparison moved in
the counterintuitive direction: broad candidate/base was 1.0206x with a 95%
confidence interval of [1.0015x, 1.0262x], while external JetStream, Kraken,
and SunSpider were 1.020x, 1.003x, and 1.006x. This inverse result is runner
variance, not evidence that the rejected metadata clone was beneficial.
Rollback broad raw/report SHA-256 were
`fc352215353b3d5128819f463016a48b604e2900007b7927faef5fa034f6aab7`
and `6776382f4a4a1834f8ca2e95996d3e99c860059167edeed105b81cb7a0d60bff`;
external raw/report SHA-256 were
`95e62175ba433582a461eb53c3fc1cd5ebadc00289a70912430d4e304acffdcf`
and `d076cd3c5fdad39e5c04a0614ab74c074f501a6d0bd2db8ab5d8686bd943bd4b`.

### Unit 91: close the dynamic-function realm correctness regression

The coverage audit around Unit 90 exposed a correctness failure unrelated to
its metadata experiment:
`private-static-setter-multiple-evaluations-of-class-realm.js` overflowed the
stack. The first bad revision was `0a3fb27c`, whose cached dynamic-Function
realm identity had copied a temporary per-function snapshot. Later realm
changes were therefore stale during repeated class evaluation.

Commit `55ec2ad9` replaces that snapshot with an immutable presence hint and
reads the live realm marker when the function executes. A focused regression
test exercises the repeated dynamic realm transition. The local full gate and
main CI `29739923449` passed. Test262 Coverage `29740129617` then reported
42,672/42,672 configured cases passing, zero failures or timeouts, and zero
actionable gaps; its burndown SHA-256 was
`43f67fc820b3c0e0dc203afa905039f2fb20e7e3a866862d6341a1744e5da0da`.
The post-rollback Coverage run `29740972027` independently preserved the same
42,672/42,672 result; its burndown SHA-256 was
`cf3feb1d134344ee467b67b994e57791264d29dbf25d22f7ca3f27c2f0cab9d8`.
Correctness closure is recorded separately and is not counted as a performance
win.

### Unit 92: retain canonical numeric compound-assignment keys

Unit 92 starts from an independent macOS `sample` profile of SunSpider
`bitops-nsieve-bits`. The profile attributed a material part of the hot path to
`coerce_property_key` and `number_to_js_string`: computed compound assignments
such as `array[index] &= value` converted a canonical numeric array index into
a newly allocated string before both the read and write-back.

Commit `b83b97cb` adds an internal `ToPropertyKeyForAccess` bytecode operation.
It performs observable object-key coercion exactly once, but retains primitive
canonical numeric indices as numbers so dense-array GetProp/SetProp paths can
consume them without string formatting. The implementation is independent of
benchmark identity, source path, iteration count, and checksum. A focused test
also verifies that an object key with `Symbol.toPrimitive` is still coerced
exactly once and produces the correct write-back value.

The first local complete broad screen executed all 225 formal measurements and
all 600 linearity samples successfully. Because the candidate and base were
local binaries outside the receipt-backed build workflow, the reporter
correctly rejected their provenance instead of issuing a formal report. A
provisional replay using the frozen median-log aggregation was 0.99405x
candidate/base overall; no family indicated a material regression. Its raw
JSONL SHA-256 was
`6d4113aaf28b8395c14d67b7813eb997124883a18dd34b71dd4515960b83a03c`.

A separate three-block local external run used exact candidate, base, and
QuickJS-NG executable SHA-256 values
`08393a0bc24a4d852d50cfee3191c112c6859754a6a8b3f03b226d5de20a758e`,
`64eab52425912a4dc3af1c26269a4b0340cf0c40bbbfb8d6524bda993c5ef6f3`,
and `cfd8386c3c29b1125a878b8fb82f9627820f2dcc16d2a691c5f8c16ad0b047a0`.
JetStream was neutral at 0.99846x candidate/base; Kraken was 0.99196x and
SunSpider was 0.97929x. The mechanism-shaped results repeated in every block:
Kraken `audio-fft` was 0.925--0.937x, `audio-beat-detection` was
0.930--0.935x, SunSpider `bitops-bitwise-and` was 0.851--0.872x, and
`bitops-nsieve-bits` was 0.868--0.876x. The same run exposed a real local
tradeoff: `audio-dft` was 1.030--1.052x. All 399 formal measurements were
successful; the only capability failures were symmetric candidate/base
timeouts for Kraken `imaging-gaussian-blur`. External raw/report SHA-256 were
`6bdfd1c5a4762ff9abb1971c43e73cdb6e9657876714720ed27381ba5e8c237e`
and `f0d22c7e6709e189602090f614938ddc963653402f918187f465d95f8b1f5a2b`.

Trusted-main Performance Preview `29743460693` was deliberately evaluated
across all three attempts rather than selecting the most favorable result:

- Attempt 1 produced no performance conclusion. Candidate `captured_write`
  was timer-limited in broad blocks 0 and 2, invalidating both complete blocks
  under the frozen portfolio-wide policy. All 225 measurement processes and
  600 linearity samples otherwise completed with status `ok`; external
  measurement never started. Raw/report SHA-256 were
  `33a58e807d954b7851c7f8861366cb1a7b0eb257fb8fbc250ba78531e712f816`
  and `e50d0420fd95a19de24cd93e82730b74274bd04f6c8a66d0aa81eec14b80783e`.
- Attempt 2 completed all three broad blocks and reported 0.97190x
  candidate/base with a 95% confidence interval of [0.96892x, 0.98033x]. That
  attractive aggregate was not credited: large movement in unaffected binding
  cases and a 1.0543x `branch_arithmetic` result showed substantial hosted
  runner/code-layout noise. External candidate/base was 1.00127x for
  JetStream, 0.99397x for Kraken, and 0.99407x for SunSpider.
  `bitops-bitwise-and` and `bitops-nsieve-bits` were 0.95568x and 0.95995x.
  Broad raw/report SHA-256 were
  `66c49db7b7df3a1e791d8f18aefc1185ab47b77b24fc568a0183108e31783b59`
  and `78d6dd6e0cf3c6b6a632f4993e60c239862379c2b3f1b039479d4b894adecc5a`;
  external raw/report SHA-256 were
  `d66594b8866ada6f4ae95daf96a825972695d1d421ec80d16f398b327bd70080`
  and `d7dfdd4be592fe05e3fd15e1f710cef4cb7553f6a943d7006ebf9186082ed816`.
- Attempt 3 independently completed all three broad blocks and converged to a
  neutral 0.99974x overall, with a 95% confidence interval of
  [0.99786x, 1.00469x]. External candidate/base was 1.00579x for JetStream,
  0.99902x for the 7/14 mutually comparable Kraken cases, and 0.99966x for
  SunSpider. `bitops-nsieve-bits` again improved to 0.93316x; the noisier
  `bitops-bitwise-and` result was 0.99474x. Broad raw/report SHA-256 were
  `167c8855cf1f9a167723fe38bea993fedb9c36a20f655731ba479df0d728e9ad`
  and `6264137bc7f6f99d81ef6b4263073cf13d7577621d1b02c2867ed306b16334b5`;
  external raw/report SHA-256 were
  `07cc91aa6f79841b5f1c51965c6fc831c58abf6cb0396119727752cbf2b9407b`
  and `2b22a257446149feab9bb41f832c7ce639709d06061fbc2f6ddc3f4e2ed2c9bc`.

Unit 92 is accepted as a modest general-engine step, not as the large broad win
suggested by attempt 2. The changed mechanism's `bitops-nsieve-bits` benefit
repeated in the local three-block run and both successful hosted attempts,
while the independent hosted broad repeat was neutral and no external suite
showed a repeatable material regression. Main CI `29743460727` passed, and
Test262 Coverage `29743678092` reported 42,672/42,672 configured cases passing,
zero failures or timeouts, and zero actionable gaps. Its burndown SHA-256 was
`5d4ccef310aca9aae2004cf32baab8380a9d8863e8e5b595d5dd4072c1055cef`.

This unit does not materially close T018. Attempt 3 remained at 0.34918x
candidate/QuickJS-NG on broad v2, with allocation still 2.835x QuickJS-NG.
External candidate/QuickJS-NG remained 8.470x for JetStream, 4.975x for the
comparable Kraken subset, and 7.942x for SunSpider. B3 therefore continues with
larger external-profile-driven structural bottlenecks; B4--B6 remain open.

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

## 2026-07-21 Scalar Numeric-Leaf Adapter Recovery

The Realm-correctness units exposed a compiler-layout regression in three
call-shaped numeric-leaf cases. Keeping only the scalar `number_binary`
adapter inline, and spelling its five always-Number arithmetic operations in
that adapter, restores the intended layout without inlining the wider
`direct_number_binary` loop helper. The operations are the same Rust `f64`
expressions as the canonical helper; comparison, bitwise, shift, and power
operations still use the existing fallback.

Against exact base `00e72f743205cdd6c9afdbbf4c5ee8643b3cf639`, the complete
25-case, three-block candidate/base diagnostic produced these median ratios:

| Scope | Candidate / base |
| --- | ---: |
| overall | 0.93182x |
| call family | 0.88909x |
| `function_call_two_args` | 0.48873x |
| `captured_read` | 0.52959x |
| `captured_write` | 0.68833x |
| worst case (`plain_function_call`) | 1.01380x |
| worst family (`control`) | 1.00407x |

The full run had 147/150 eligible formal records. The only capacity miss was
the candidate's three `branch_arithmetic` records: each formal window was
1.45-1.47 seconds, but an 18.4 ms median process-start sample exceeded that
case's former 1% ceiling. A same-binary supplemental run with only the startup
ceiling raised to 2% made all 6/6 candidate/base records eligible and measured
`1.00681x`. The checked-in capacity settings also lower the exact-number-capped
captured/two-argument windows; they do not change workloads, checksums,
linearity, result validation, or performance acceptance thresholds.

The same candidate/base binaries passed the three-suite focused external
guard: JetStream `hash-map 0.98483x`, Kraken `json-parse-financial 0.98747x`,
and SunSpider `controlflow-recursive 1.00057x`, with all three engine
capabilities still `ok`.

Evidence bindings:

- candidate binary SHA-256:
  `8fbe9ac99123e2dc19e80e5165162aafdd3222d6a6102f14159d21a762ed4b28`;
- base binary SHA-256:
  `ded5c6e3c3d3b136b574a3f49e153a2b9fcf85a4df230bd295d5e70ce861640d`;
- full diagnostic raw SHA-256:
  `cb3a90807d8efea4ff7b25bfa58956162170b620a2cf2b840229568d9420e411`;
- full analysis SHA-256:
  `f66b89390f07ea963b451af92030e443ca54b4181ba6311a8694463b0e0e3f3c`;
- supplemental raw SHA-256:
  `1ac99cbc172c0e61ba6c265b3d2d5d6f7a146d3f80fa5e8c64ecbb6d031ccb3f`;
- external report SHA-256:
  `e3b6fe05f6f26c7fa1af3beb713558d4075c5f3709e0dd2757bc6eb7432a8357`;
- external raw SHA-256:
  `fdaf6fa183ce067c99234c91072ac87317bb8be7fdf6ae12d35f886203bda110`.

### Hosted confirmation at `a9a752ec`

The informational hosted Performance Preview run `29893708879` independently
confirmed the scalar numeric-leaf recovery on Linux x86-64. Its exact candidate
was `a9a752ec9f589e15017745a1a0cd7306a8ee304e`, exact base was
`00e72f743205cdd6c9afdbbf4c5ee8643b3cf639`, and pinned QuickJS-NG was
`f7830186043e4488f2998759d60a514faf07cbc9`.

The complete hosted broad diagnostic measured `0.95112x` candidate/base and
`0.31888x` candidate/QuickJS-NG overall. Binding improved to `0.85562x`
candidate/base and call to `0.94471x`; the strict every-case contract still had
eight failures:

| Case | Candidate / QuickJS-NG |
| --- | ---: |
| `top_level_function_call` | 9.45135x |
| `dynamic_method_call` | 7.46955x |
| `array_write` | 4.92542x |
| `array_allocation` | 3.38184x |
| `object_allocation` | 2.52945x |
| `closure_allocation_call` | 1.75505x |
| `string_slice` | 0.53327x |
| `local_read` | 0.51566x |

External suites remained essentially neutral candidate/base and far from the
QuickJS-NG target: JetStream was `1.00443x` / `8.08784x`, Kraken was
`1.00009x` / `4.78708x`, and SunSpider was `0.99908x` / `7.49692x`
(candidate/base / candidate/QuickJS-NG). This confirms that the accepted unit
recovered broad call/binding layout without yet changing the external
dispatch, object, or allocation bottlenecks.

The matching Test262 Coverage run `29893867661` preserved the correctness
floor: 42,672/42,672 configured cases passed, with zero failures, timeouts,
not-run cases, or actionable QuickJS-NG gaps. Evidence SHA-256 values:

- hosted broad report:
  `535c82136fc454ec059386fc9ffea68ffefb8c9f0aff3836650acecefaaa3a1a`;
- hosted broad raw:
  `959a27047ef3966740adbf0cd8866349970b3a7a887a022e216dc2cfa38edeb3`;
- hosted external report:
  `c0bdbadbfb5b43a5f26c5dbc5f2193118ff2a2130bbbb2cca5758843a37db125`;
- hosted external raw:
  `3ca84d2a7d8713f871aacb871731a0983c136af198baac6ca432297c77f56610`;
- Test262 burndown:
  `7e834737f584f43f2024cddc9bb3e6697865c9cbd8366ed8e4ebd270b4db35ad`.

### 2026-07-22 string R4 accepted; literal-allocation R4 rejected

Commit `55c22dec0a914ca87b2b7c670da8c70d3988dad5` (integrated on `main` as
`b569784173a973a1740d3685eaf9e75d852695ed`) corrected `slice`, `substring`,
and `substr` to index UTF-16 code units and changed the already fail-closed
numeric-loop `.slice(...).length` term to compute the selected code-unit range
without allocating a temporary substring. The admission proof still requires
an authoritative primitive-string slot, numeric arguments, and the unchanged
intrinsic `String.prototype.slice`; method replacement, getters, object
coercions, and dynamic scope stay on the ordinary path.

The formal 20-million-iteration runner classified the candidate as
`timer_limited`, so the result is not a formal portfolio claim. A separate
role-rotated three-block 40M/80M confirmation retained correct operations,
checksums, exits, and near-linear duration. Its median ratios were
`0.07476x` candidate/base and `0.04677x` candidate/QuickJS-NG; incremental
slopes were `0.07211x` and `0.04660x`. Independent external controls stayed
within the unit guard: JetStream `string-fasta 0.990x`, Kraken
`json-stringify-tinderbox 1.025x`, and JetStream `hash-map 0.990x`
candidate/base. Candidate binary, raw, and summary SHA-256 values are
`60bfe9a48c30c52f59753c77cc64fbff684e0dd9204e12adaa863ad8532b3b43`,
`495813494142bbbe3e0c01851f2bad417c6e5bfb8bbd6b17e7d8ee59b90acadb`,
and `a94327407ed57821da8a64cd9ad5b0d21c7fe9462a7fae3b915c7d52f77c1c5c`.
The integrated unit passed 1,422 runtime tests, the complete 5,141-case local
Test262 subset, and all QuickJS-NG comparison fixtures before push.

Hosted Performance Preview `29903567325` completed against exact base
`57ae2dee63fddd3bb3e15bdfc3e5e7da8f4f6259`, but correctly produced **no
formal performance conclusion**: the optimized candidate reached the frozen
20-million iteration ceiling in all three `string_slice` blocks at roughly
0.318 seconds, below the 0.6-second eligibility floor. Candidate coverage was
therefore 24/25 while base and QuickJS-NG retained eligible `string_slice`
records, making the portfolio comparison input incomplete. This is a harness
capacity boundary, not an execution failure or a regression.

The retained diagnostic samples measured median `string_slice` cost at
15.8701 ns/op for the candidate, 50.1272 ns/op for the exact base, and 95.1165
ns/op for pinned QuickJS-NG: diagnostic ratios of 0.31660x candidate/base and
0.16685x candidate/QuickJS-NG. Across the 24 formally common cases, the
diagnostic geometric means were 1.00550x candidate/base and 0.31797x
candidate/QuickJS-NG; `top_level_function_call` remained the worst strict-goal
gap at 9.86565x QuickJS-NG. The separate 40M/80M confirmation above remains
the admissible direction check for the timer-limited string mechanism.
Hosted raw, placeholder report, status, and manifest SHA-256 values are
`c00b16a8a09bd0d41b1d69bed750c6e683d1ad5e3fd364c4cd5bb410763daf95`,
`c1aa3862d7f75bb0b68c76e7e32d109917e922c0bcc8e4b919d72db64630c120`,
`adbc36d63ba49e88ae19939da0cf36d5b03097edf93a56a83f52ba0eed395f70`,
and `d5315473824f62466e2bb127329489d27eb4b26605f94b62bfe68ce0e2f98572`.

The companion allocation experiment on
`agent/allocation-hotpath/alloc-r4` at
`d5ecfb3e8762a0295a17a5ef05d9bf721ca479e0` is explicitly rejected and was
not integrated. Although its dead-literal loop replacement produced
`0.05091x` object-allocation and `0.03259x` array-allocation ratios against
QuickJS-NG, independent review found two blockers:

- counter/accumulator aliasing changed a result from `12` to `17`, and
  limit/accumulator aliasing changed `606` to `405`;
- the planner matched the internal allocation cases' exact counted-loop,
  completion-temporary, constant-literal, and `acc +=` bytecode grammar, while
  three external controls remained neutral (`1.020x`, `1.003x`, `0.993x`).

The second point violates this task's generalization rule even if the first is
repaired. The experiment is retained only as an upper-bound diagnostic and a
source of conservative escape/authoritative-slot tests. Any successor must use
general CFG, def-use, alias, and virtual-object analysis with an independent
external effect; it must not reuse the absolute-offset matcher or its
single-expression substitute interpreter.

### 2026-07-22 static numeric-index stores accepted

The three-commit array R5 unit (`e113e76c`, `56553683`, and `bd973aff` on
`main`) lowers simple assignments through a side-effect-free numeric literal
key to `SetPropIndex`. It preserves receiver-before-RHS evaluation, leaves the
assignment result on the operand stack, uses the existing dense-array and
integer-indexed TypedArray guards, and falls back to complete `[[Set]]`
semantics for proxies, special descriptors, custom prototypes, frozen arrays,
and other receivers. Detached/out-of-bounds TypedArrays, numeric-versus-BigInt
coercion, and strict/sloppy failure behavior have focused coverage.

The first build inserted the new opcode between existing hot enum variants.
Although the new compiler lowering could be disabled without changing the
Kraken result, that build regressed `audio-fft` by about 4.3%. Appending the
opcode to the enum instead preserved every existing discriminant and removed
that layout effect. The final three-block focused diagnostic measured:

| Case | Candidate / base | Candidate / QuickJS-NG |
| --- | ---: | ---: |
| `array_write` | 0.70280x | 1.58543x |
| `array_read` control | 0.99012x | 0.05589x |
| `property_write` control | 1.01748x | 0.15809x |

Role-reversed direct-bundle screening then bounded Kraken `audio-fft` at
1.022x and `audio-dft` at 0.985x candidate/base by median process CPU time.
These external numbers are diagnostic controls, not a formal portfolio claim;
they show that preserving opcode layout removed the earlier over-3% regression.
Candidate, base, QuickJS-NG binary SHA-256 values are
`46536489762d8f02dd9ccf898aa5e43d55b4d717c7fbbfbcccee68c1b7b8b7d5`,
`3ec36e1bafa4d0bfc9353e3a20c0656c6965d0cc55dae78bac555856facf3f6f`,
and `cfd8386c3c29b1125a878b8fb82f9627820f2dcc16d2a691c5f8c16ad0b047a0`;
focused raw SHA-256 is
`9b30cfff49f7a6f2c507ce670779974ae0b2e92905372b211a092f6fa51f2285`.
The strict target remains open because `array_write` is still 1.585x slower
than QuickJS-NG locally.

### 2026-07-22 virtual-object scalar replacement accepted

Commits `bb631582`, `13ad60ce`, and `c704cc71` replace provably non-escaping
dense object and array literals with frame-owned scalar slots. Admission is a
general CFG, def-use, and alias proof: candidates fail closed on escaping
values, unknown control flow, suspension, unsupported coercion, or more than
the 128 runtime-authority slots. The lowering retains original bytecode
offsets and executes through the existing `Vm::run_completion` dispatch; it
does not add a benchmark-specific matcher or a second executor.

Two independent reviews found and drove fixes for four correctness or
architecture hazards before integration:

- generator start and resume now refresh the selected execution stream and
  clear frame-owned virtual state;
- the virtual increment back edge uses the same loop-plan dispatch as the
  original `Jump`, avoiding an unrelated empty-loop regression;
- the scalar bank is lazy, so cold calls do not pay an allocation, and it only
  grows so a later small plan cannot truncate higher live slots;
- runtime authority is a bounded `u128` mask rather than a per-operation
  collection walk.

The final role-rotated focused run
`67bb3f9e-7bc2-40a6-96a8-d3d7a9a469d6` retained 20/20 eligible formal
samples and 32/32 valid linearity samples. It measured:

| Case | Candidate ns/op | QuickJS-NG ns/op | Candidate / QuickJS-NG |
| --- | ---: | ---: | ---: |
| `object_allocation` | 73.0989 | 175.0023 | 0.41812x |
| `array_allocation` | 76.5649 | 160.1440 | 0.47711x |

Every individual block stayed below 0.427x and 0.485x respectively, and all
linearity medians remained inside the frozen 0.85--1.15 interval. Because the
run intentionally covered only two of 25 broad cases, these values are focused
admission evidence rather than a complete-portfolio claim. Its raw SHA-256 is
`00f56449f6ec0fc8109da34c9e5d11df326087b33f610b67737b77fe41e5ccc8`.

After integration, the main worktree passed 1,478 runtime tests, all 198
benchmark protocol tests, the complete 5,143-case local Test262 subset, and
all 207 QuickJS-NG comparison fixtures. Hosted broad, external-suite, and
full Test262 confirmation remains asynchronous; only that exact-main evidence
can close the allocation cases in the strict every-case contract.

The exact-main hosted closure for `17b89142c8978d6e8ed3d5ec2ffe4726d0e3eaf5`
completed after that local acceptance. Test262 Coverage run `29923200787`
reported all **42,672/42,672** configured Rust cases passing, with zero
failures, timeouts, not-run cases, or actionable QuickJS-NG gaps. Performance
Preview run `29922926945` retained all 25 broad cases, but was correctly
classified `inconclusive`/`non_claim`. Object and array allocation measured
0.8564x and 1.1640x QuickJS-NG on the hosted runner, while `property_read` and
`array_read` regressed to 7.6165x and 5.0757x QuickJS-NG. The latter two
regressions exposed a real compatibility bug: scalar replacement wrote
`undefined` into the local aliases used by the precompiled numeric-loop
planner, so the planner could no longer prepare its fast path. The hosted
external preview remained far from the campaign goal at 8.170x for the
JetStream subset, 4.796x for comparable Kraken cases, and 7.425x for
SunSpider. Broad raw/report SHA-256 values are
`e7856247c021fa3330abde2a8968691c624007474a740b6da0f07c038c6bc20e` and
`1888fb6902cef9fa59c3157a4b5bcde2530f135e7aac825655b51fa1f46a9165`;
external raw/report values are
`6ef2c2971ed2dd3b96dfedc142ab3d4d16f25f35db7202a9350466354ae5c129` and
`6ceb16e6a229d2a708e9e4ef9b7f2764786a516744b4fda93c4f7c0585a7d293`.

### 2026-07-22 function-literal scalar replacement and loop compatibility

Commits `20090831` and `2a544b11` extend the same general escape analysis to
provably non-escaping function literals. Eligible literals are represented by
frame-owned scalar metadata and called directly through the existing VM;
ordinary function allocation and property materialization remain the fallback.
Admission is fail-closed and does not inspect benchmark identities, source
paths, checksums, or iteration counts.

Two independent correctness reviews caught creation-context loss in the first
implementation. A named function's immutable inner-name binding and direct
`eval` frame bindings must remain observable to nested calls. The final runtime
therefore selects full function SRA only when there are no deopt bindings,
immutable function-name bindings, or active `with` environments; otherwise it
uses a same-length data-only object/array stream or original bytecode. It also
rejects any virtual candidate whose allocation or use overlaps a numeric,
control, or numeric-mutation loop plan. This restores the exact precompiled
loop plan instead of trying to reconstruct its aliases after lowering. Mixed
functions can still scalar-replace an independent data literal while the
context-sensitive function remains materialized.

The final focused three-role run retained 45/45 eligible samples and all 120
linearity checks. Its median candidate/QuickJS-NG ratios were 0.09003x for
`property_read`, 0.05722x for `array_read`, and 0.41950x for
`closure_allocation_call`. A separate allocation rerun measured 0.45564x for
`object_allocation`, 0.50413x for `array_allocation`, and 0.41626x for
`closure_allocation_call`; these remain local directional evidence because the
array result is just above the strict 0.50x line and hosted exact-main evidence
is authoritative. A role-reversed candidate/base run measured medians of
1.01063x, 0.99464x, and 1.02414x for the same three allocation cases. The
closure movement is below the predeclared 3% rejection line but remains a
code-layout risk to verify on hosted Linux. Raw SHA-256 values are
`9d17e1aa21de17d3a618ca89a498ddad5feb81db42384c9b970f39bdf0014d2c`,
`289f70ea3789df667c0ba703c24de4ac0acca3441a72b432fb760a089fbf7469`,
`e354b01e7c28e0b53e65f68484a16940ccba91ebd5d384802c726edcc7eca32d`, and
`7e22e4bcce7c643912d2442cbf6acc791c1fed033f485bd7e5f84d2fd3f8b828`.

Before integration, the combined branch passed 1,489 runtime tests, workspace
Clippy, all QuickJS-NG comparisons, the touched 65-case eval/function Test262
slice, and the full repository check including all 5,143 curated Test262 cases.
Branch CI `29929244064` and exact-main CI `29929669969` passed.

The exact-main hosted closure for
`a716aa442c43fa160c60fd62d47f5d01949a9500` confirms both the compatibility
repair and the remaining structural gap. Test262 Coverage run `29929967481`
reported all **42,672/42,672** configured Rust cases passing, with zero
failures, timeouts, not-run cases, or actionable QuickJS-NG gaps. Its burndown
SHA-256 is
`3899c0f2a6f212c60a06acf79e29abac1d5477ba18b9d9a6314f97e56a90a934`.

Performance Preview run `29929668597` retained all 25 broad cases and measured
an informational candidate/QuickJS-NG geometric mean of 0.2756x. The repaired
`property_read` and `array_read` cases measured 0.2757x and 0.1990x, while
`property_write` measured 0.1438x. This closes the loop-plan compatibility bug,
but not the every-case contract: `top_level_function_call` remains 9.8544x,
`dynamic_method_call` 7.4783x, `array_write` 2.5116x, `object_allocation`
1.1130x, `array_allocation` 1.6318x, and `closure_allocation_call` 0.9426x
QuickJS-NG. The external preview is likewise still structural rather than
incremental: the comparable JetStream subset is 8.060x, Kraken 4.785x, and
SunSpider 7.555x QuickJS-NG. Broad raw/report SHA-256 values are
`0ad2a05ed3f0b29d2eb8c290e1fd20d0607afba5a73bb4aa6744ac3b1c685422` and
`0b6ee86c177f94b52f1cba5acf136e38b62c5aa589611597129ad0a8b3e043f2`;
external raw/report values are
`de4ff4b70df0669e9650204806604d9f7a56bf393cd6a50300d3876aefbc439d` and
`1242413c9af1723684793ec7d650a6e8c1dec08f5482f55f5b091c5bc3b0fb48`.
The preview remains `inconclusive`/`non_claim` because it ran on variable
GitHub-hosted hardware, so it is prioritization evidence rather than a public
fixed-hardware claim. B4 and B5 remain open.

### 2026-07-22 top-level call measurement-capacity repair

Trusted-main Performance Preview `29942288216` produced no performance
conclusion after the guarded top-level numeric-loop unit. Candidate and base
both reached the existing 50,000,000-iteration ceiling in about 234 ms, below
the frozen 500 ms formal window, so `top_level_function_call` was
`timer_limited` in all three blocks. The other 24 cases were present for both
roles and all 75 three-role linearity checks passed.

The measurement ceiling for that case is therefore raised to 130,000,000, and
its capacity-bound formal window becomes 250 ms with a 4% startup ceiling.
This leaves the workload, operation count, checksum model, warmup, timeout, and
analysis policy unchanged. Its maximum triangular checksum is
8,450,000,065,000,000, below JavaScript's maximum safe integer. The 250 ms
window still requires at least 25x startup amortization and matches the policy
already used for cases whose exact checksum bounds cap safe iterations. This
is a capacity repair only and is not counted as a runtime performance
improvement.

Follow-up trusted-main Performance Preview `29945099786` completed the repaired
portfolio with 225/225 eligible formal measurements, all 75 linearity checks,
and 3/3 valid blocks. Candidate and base runtime binaries were byte-identical,
and candidate/base was 0.999766x with interval [0.993341x, 1.004445x], which
confirms that this commit changed measurement capacity rather than runtime
performance. The current hosted candidate/QuickJS-NG baseline is 0.212540x
overall, but the strict every-case target remains open: `dynamic_method_call`
is 7.3041x, `array_write` 2.1976x, `array_allocation` 1.3244x,
`object_allocation` 0.9337x, `closure_allocation_call` 0.9094x, and
`local_read` 0.5923x QuickJS-NG.

The mandatory external preview remains the dominant structural gap. Its
comparable candidate/QuickJS-NG diagnostics were 8.022x for the five-case
JetStream 3 JavaScript subset, 4.834x for seven Kraken cases, and 7.218x for
all 26 SunSpider cases. Broad raw/report SHA-256 values are
`3de273d065c56a53fc29126436aa30363fa59330af67345ea891e81031f25acc`
and `275054d48db7c3d167b9ccb126ce92bd16a15e5698ca8c6a2bdf552641fb11cc`;
external raw/report values are
`8aacd413c578dfa78887dfcdc83466de4c77665135a92df7d48238d0847dcd38`
and `09c7f079b8c64e1f3da73f7ee645f546ce9889254b45d1bc57612c382de013d1`.
The artifact digest is
`77abb9859a48653796b89ac5e8f370a5b6ad00944473590d62bb88355d8c77f8`.

### 2026-07-22 NewClass-only cold opcode payload accepted locally

This runtime unit moves `NewClass`'s immutable, variable-sized definition
behind `Rc<ClassDefinition>` while leaving `NewFunction` inline. The measured
64-bit `Op` layout falls from 216 to 96 bytes. Compiler emission, VM execution,
binding-write caches, closure-name scans, deferred computed keys, private
elements, and global-name traversal retain the same inputs and order. The
`NewClass` variant remains in its original enum position, and the ownership
graph adds no back edge from child bytecode to its parent definition.

The complete 25-case, two-role, three-block screen retained 150/150 eligible
formal measurements and all 50 role/case linearity medians. Candidate/base was
0.996210x overall with a 95% bootstrap interval of
[0.967402x, 1.004527x]. Its raw SHA-256 is
`a6560e735a57a7c2d9640f738fcb1fcf148dbbb6740469d6350cbcacd8d1853f`.
A separate eight-case, seven-block control run retained 112/112 eligible
measurements and all 16 linearity medians; its aggregate candidate/base result
was 0.988805x with interval [0.962167x, 1.023236x]. Its raw SHA-256 is
`96acb4992520343202b5d9ff5db7279957e4f24eb53596b18c3c91dadb03d68b`.
Both intervals cross 1.0, so these runs establish a neutral local regression
screen, not a throughput win.

This is narrower than T011's rejected experiment, which boxed both
`NewFunction` and `NewClass`, reduced `Op` to 80 bytes, but measured 1.0392x
overall and 1.1796x for `many_locals_call`. Here `many_locals_call` measured
1.002954x with interval [0.982683x, 1.051018x], so that regression did not
recur. The branch passed all 1,534 runtime tests and `git diff --check`.

The raw runs are receipt-less two-role diagnostics and correctly remain
`claim_eligible: false`.

Hosted Performance Preview `29948189616` compared exact candidate `089e4920`
with parent `2e7ac71c`. It retained 225/225 formal measurements, 3/3 valid
blocks, and all 75 linearity checks. Candidate/base was 0.984050x overall with
interval [0.982250x, 0.992158x], while `object_allocation` and
`array_allocation` improved to 0.801767x and 0.800388x. The unit nevertheless
failed its predeclared standalone no-regression gate:
`closure_allocation_call` was a stable 1.036896x with interval
[1.031255x, 1.056660x]. The external candidate/base suite diagnostics were
neutral at 1.002667x for JetStream, 0.998406x for Kraken, and 1.001564x for
SunSpider. CI `29948189614` and Test262 Coverage `29948452652` passed.

The immediately following cold-accessor layout unit at `fb3d7645` supplied
the missing cumulative evidence. Performance Preview `29948718553`, with
`089e4920` as its exact base, again retained 225/225 measurements, 3/3 valid
blocks, and all 75 linearity checks. It measured 0.992252x overall
[0.989354x, 0.993247x], 0.984654x for the allocation family, and 0.964732x
for `closure_allocation_call`; its largest internal regression was
`dynamic_method_call` at 1.018922x. All three external suite geometric means
improved: JetStream 0.985069x, Kraken 0.991548x, and SunSpider 0.994892x.
SunSpider `crypto-sha1` was a one-run 1.054829x diagnostic without a per-case
interval and remains a repeat-watch item rather than a rollback signal. CI
`29948717681` and Test262 Coverage `29948984533` passed.

Multiplying the two exact-parent comparisons gives a directional cumulative
point estimate of 0.976426x overall, 0.798864x for `object_allocation`,
0.794915x for `array_allocation`, and 1.000327x for
`closure_allocation_call`. The corresponding external suite estimates are
0.987696x, 0.989967x, and 0.996448x. These cross-run products do not replace a
same-host direct comparison, so `NewClass` is not credited as a standalone
accepted performance unit. The current pair is retained for ROI because the
successor removes the only stable >3% internal regression while preserving
the allocation gain; a direct current-HEAD versus `2e7ac71c` A/B remains the
final retain-or-revert audit.

NewClass broad raw/report/status SHA-256 values are
`0006a3f5e2154319aaa9f05c88461812a7a30668567673ef0daf1a5ba67affeb`,
`3c8ab9c119b55f0235abbab7d295898dd49bba60c17a9374576b1dc350449c60`,
and `4913c9f4ad08eaf4542b771a9079f08510332cb6e5004da26fff904ff339ce28`;
external raw/report values are
`ca1176babb17c1ca11d1fd43fdb5ec5be91dce44227da088eecb81e8730f8ae9`
and `0fa2900e56f604029594cae7fc799b76c66542e7ecc8cb6f6c66cea921ca3b5e`.
Its artifact digest is
`0448ce08702564b5da28819b924005b78c93b57bd2f92ea8727ab4b2856e4908`.
The Property unit broad raw/report/status values are
`8104130fd38bee11b393c51bdb89211b381554aafc416df03d815957b10c1e4d`,
`925ba557f3802907eee8e34f90807b916cf3e2fe9978c710ed887256a751f41e`,
and `3b93c414c8583d85c502cbb2c96ea7a716b77f5a122a8341d22467d907d7b7a6`;
external raw/report values are
`a1bebe13313a85ca51f8ff054cc44b5a60056d5be4bfa15f3ea95f330d0bba07`
and `335c599108285e36e2b64f2ccc6a776a2a6ae998121900b73529b5539241d10b`.
Its artifact digest is
`3ed6be7438b8037003012c34a1fc4d4eb8299283fbe4f1c973dd792458eeda8e`.
Both hosted reports remain informational `inconclusive`/`non_claim` evidence
on variable GitHub hardware and do not close a strict T018 case.

### 2026-07-22 target-specific present-own dense array reads

Commit `eabf2456` makes the existing numeric-index fast read depend only on the
target array element. A present own dense data element shadows every prototype,
so unrelated holes, cold descriptors, named properties, and custom or Proxy
prototypes no longer force the read through numeric-to-string coercion. A hole
or a special descriptor at the target index still falls back to ordinary
property lookup. The target descriptor check formats the canonical decimal
index into a fixed stack buffer and performs a borrowed hash lookup, avoiding
a heap allocation on the cold-state guard itself.

Focused coverage exercises filled `new Array(length)` storage, unrelated
holes/descriptors, custom/null/Proxy prototypes, inherited hole lookup,
target accessors, object-key `ToPropertyKey` side effects, and the transitional
storage state where a dense write clears a hole while the same-index descriptor
remains present. The branch passed 1,544 runtime tests, the complete
5,148-case local Test262 subset, and all 218 QuickJS-NG comparisons.

The final exact seven-block SunSpider `bitops-nsieve-bits` A/B measured
0.709604x candidate/base with a 95% bootstrap interval of
[0.706253x, 0.711776x], a 29.04% improvement. Internal controls remained
neutral: `array_dynamic_read` was 0.997496x
[0.987594x, 1.001215x], `array_read` 0.995710x
[0.995420x, 0.999442x], and `property_dynamic_read` 0.999118x
[0.996793x, 1.009020x]. Candidate/base binary SHA-256 values are
`af2854e1241f6d23979126dc0e393c2a7008ccbe000a9ede467aeb79db1fe266`
and `88c0fb379c3546a5820409943aacfdd0578081b9cef66adbd3dfc6db620473c8`.
Internal raw, external raw, and external report SHA-256 values are
`4f804db18b24c32f52c39c0d8f16f00a0a36d4b356eb951d640fc873b393d257`,
`197b825f9fbfb397f5ba82ef10084437e6d28e92444f5a23691d2323eb55010c`,
and `2daa9c75fb5bee728097915bddb2e02f7d0baa669134aed155cda30499823a58`.
This closes a general coercion bottleneck but leaves nsieve well above the
strict 0.50x QuickJS-NG boundary; typed dense numeric mutation remains the
next structural step. Hosted broad/external and exact-main Test262 evidence is
asynchronous and must confirm the local retain decision.

### 2026-07-22 selected numeric loop terms

Commit `ef814c65` retains the selector/phi extension for the existing numeric
loop executor. The compiler now accepts selected numeric loop terms without
adding another VM or weakening the executor's runtime guards. The final exact
seven-block capacity run measured `dynamic_method_call` at 0.023551x
candidate/base with a 95% interval of [0.023490x, 0.023811x], and 0.080488x
candidate/QuickJS-NG with [0.080348x, 0.080961x]. The six control cases had a
1.001470x candidate/base geometric mean with [0.999853x, 1.002492x]. All
147/147 formal measurements were eligible and every N/2N check passed. Raw
and manifest SHA-256 values are
`0c25436c78ddb9a81976d4e873c9129dcea36304ee16ddd6f5b723d3c144184c`
and `57fa5fb0a48f09fe25b06547181a5447c65026fff2611868170f8d5157842919`.

This is a general bytecode-shape improvement: receiver selection and numeric
phi state are represented in the prevalidated loop plan, while any observable
coercion, non-authoritative slot, or unsupported operation still declines to
the normal VM. The branch passed the complete local correctness gate before
integration; its remote CI also completed successfully.

### 2026-07-22 dynamic-method measurement-capacity repair

The retained hash-bound selector/phi capacity run shows that the checked-in
`dynamic_method_call` contract is too small for the optimized candidate. Its
repeated N/2N samples were linear, and scaling those samples to the old
50,000,000-iteration ceiling puts every one below the 500 ms formal window.
The base and QuickJS-NG roles calibrated normally, while the other six cases
remained complete. This isolates a case-specific measurement-capacity limit
rather than a broad runner failure.

The measurement ceiling for `dynamic_method_call` is therefore raised to
130,000,000, and its capacity-bound formal window becomes 250 ms with a 4%
startup ceiling. The workload, operation count, checksum model, warmup,
timeout, and analysis policy remain unchanged. Its maximum triangular
checksum is 8,450,000,065,000,000, below JavaScript's maximum safe integer.
The 250 ms window still requires at least 25x startup amortization and matches
the existing `top_level_function_call` capacity contract. This is a benchmark
capacity repair only and is not counted as a runtime performance improvement.

### 2026-07-22 dense numeric mutation loops

Commit `1e70253e` retains a prevalidated dense-Number mutation executor within
the existing numeric mutation subsystem. It leases writable dense elements
once, keeps loop-carried numeric state unboxed, executes supported numeric and
bitwise operations without per-iteration `Value` or environment allocation,
and publishes locals only after completed iterations. A failed entry guard
declines before mutation. A mid-loop element, index, or storage guard failure
publishes only completed iterations, releases the lease, and returns to the
original loop header so the ordinary VM replays the current iteration exactly.

The exact seven-block capacity run measured `array_write` at 0.047137x
candidate/base with a 95% interval of [0.046923x, 0.047556x], and 0.070210x
candidate/QuickJS-NG with [0.069951x, 0.070387x]. All 21 formal measurements
and all 24 N/2N samples were eligible. The same exact binaries measured
SunSpider `bitops-nsieve-bits` at 0.403949x candidate/base
[0.402582x, 0.412155x] and 1.960948x candidate/QuickJS-NG
[1.951412x, 1.969571x]; unrelated internal and external controls had no stable
greater-than-3% regression. Capacity raw/manifest SHA-256 values are
`6d349a01ccc95320d4a4c5c9f71465238b753cfc6f654f4db5081159dbd21836`
and `99ed0b1b6c58323bb9e595d4eb8077260d85241f4a7c87ea728f32cfa4648dbb`.
External raw/report values are
`be3e8370bf7476785cbb3547bc3d052317d1ce627cb7bbcbd48796265b2f777f`
and `84f3d911b38065f5e8cbad3969a698649309a9cfde149775f42211dbfceb1277`.

The unit passed focused coverage, the complete runtime suite, the 5,148-case
local Test262 subset, all 218 QuickJS-NG comparisons, `check-touched`, and the
full local check before integration. The remaining nsieve gap therefore is not
evidence that the dense mutation mechanism failed; it identifies the outer
predicate scan as the next profile target.

### 2026-07-22 array-write measurement-capacity repair

The frozen dense-mutation seven-block screen made the checked-in
`array_write` contract too small. The optimized candidate reached the
20,000,000-iteration ceiling in about 246--250 ms, below the 500 ms formal
window, so all seven candidate measurements were correctly classified as
`timer_limited`. The five control cases remained eligible, and all 18
role/case N/2N groups across the three roles passed, isolating the problem to
this case's capacity.

The measurement ceiling for `array_write` is therefore raised to 50,000,000,
and its capacity-bound formal window becomes 250 ms with a 4% startup ceiling.
The workload, operation count, checksum model, warmup, timeout, and analysis
policy remain unchanged. Its maximum triangular checksum is
1,250,000,025,000,000, below JavaScript's maximum safe integer. A hash-bound
derived-manifest run with the same three binaries retained all 21 formal
measurements and all 24 linearity samples; its raw and manifest SHA-256 values
are `6d349a01ccc95320d4a4c5c9f71465238b753cfc6f654f4db5081159dbd21836`
and `99ed0b1b6c58323bb9e595d4eb8077260d85241f4a7c87ea728f32cfa4648dbb`.
This is a benchmark capacity repair only and is not counted as a runtime
performance improvement.

### 2026-07-22 exact-main hosted closure and next hotspot

Performance Preview `29958012732` is the first complete hosted artifact after
the selector, dense mutation, and both capacity repairs. Its head is
`2c6dcfff`; the exact base is `1e70253e`. Because `2c6dcfff` changes only the
measurement contract, both roles correctly produced the same runtime binary
SHA-256,
`97829fd1b27f7fc065fa73e8317a2bc83c01fc0d7831ece64124011ce01e40e0`.
All 225 measurements, 3/3 blocks, and 75 N/2N groups were valid. The hosted
candidate/QuickJS-NG broad ratio was 0.1635x. Twenty-one of 25 cases met the
strict 0.50x boundary; the remaining cases were `local_read` at 0.5461x,
`object_allocation` at 1.0917x, `array_allocation` at 1.4454x, and
`closure_allocation_call` at 1.1000x. The report remains
`inconclusive`/`non_claim` because the GitHub runner is variable hardware, not
because evidence was missing. Broad raw/report SHA-256 values are
`ac34a31d6a514635b959275aa13a205a9247709e8f3eca40e635578b9ed691fb`
and `c4a3c0af70159de1b59251607c9d80ed8a73059e88bb2e5461ea21e67b2a60c2`.

The same artifact confirms that external performance, not broad v2, is now the
dominant campaign risk. Candidate/QuickJS-NG diagnostic geometric ratios were
7.7515x for all five JetStream ports, 4.8895x for 9/14 comparable Kraken
ports, and 7.0550x for all 26 SunSpider ports. Five Kraken candidate cases
timed out. No external case met the 0.50x goal; even the only QuickJS-NG win,
`json-parse-financial` at 0.9664x, remained above it. Hosted
`bitops-nsieve-bits` was 4.5456x QuickJS-NG. External raw/report SHA-256 values
are `c6ad11a23e5e11514de3325ed183eec2d52247f6d585115784ab174063a6fd77`
and `51b215e6d212be7c64c4c506b5c452b64ad0ef9f9db5cd30583f3a689a16d625`.

Test262 Coverage `29958265493` independently closed correctness at the same
`2c6dcfff` head: 42,672/42,672 configured cases passed, with zero failures,
timeouts, not-run cases, or actionable gaps. Its burndown SHA-256 is
`c827878cfeb2dd70746e0c24ae54cb2f0ae2faf4e68b66eb59d3f84a4ba85754`.

A separate loop-plan ownership prototype removed per-frame plan-vector clones
but missed its predeclared setup gate: one-plan `numeric`, `control`, and
`dense` ratios were 0.891639x, 0.950275x, and 0.964203x, for a 0.934837x
geometric mean versus the required at-most-0.90x. It was rejected without a
commit. Result/workload SHA-256 values are
`5d62fa02852221ee00039dee4a3ffad432d833d4d7cf7cb9ed5913f85e849ae7`
and `0820d20cca5db747e6c6930ab021832ba989bc762f603179cb0d0aa02cb210fb`.
This also rejects a unified plan index as the immediate priority unless a new
profile shows repeated misses are hot.

The post-dense nsieve decomposition used exact candidate binary SHA-256
`dcef12c705a1f0a862b74b7318dd7868a8e08e0822ae73c65a50bff14b84588a`
and preserved checksum `-1286749544853`. Thirty amplified exact rounds took
52.33 ms each. Replaying only the same 160,000-step outer dense bit predicate
scan took 28.64 ms per round, or 54.7% of the whole case; initialization was
about 0.60 ms. A fixed 400,000-mutation diagnostic grew from 16 ms with one
plan to 21 ms with 12,500 plans, placing the real workload's 7,837 active
inner-plan setup cost at only a few milliseconds. The next retained experiment
must therefore attack the general dense numeric predicate scan, fail closed at
the first true predicate, and return to ordinary bytecode for the observable
body. Plan ownership, object shapes, and benchmark-specific matching are not
supported by this profile.

### 2026-07-22 common-range `ToUint32` conversion

Agent commit `fab5a2b5` replaces the expensive floating-point modulo in the
common `ToUint32` range with an equivalent checked conversion. Values in
`[-2^32, 2^32)` are safely within `i64`: Rust's checked-range float conversion
truncates toward zero, and narrowing the resulting integer to `u32` keeps the
low 32 bits required by ECMA-262. NaN, infinities, the exact upper boundary,
and all larger magnitudes retain the preceding `trunc().rem_euclid(2^32)`
path. This is a shared conversion primitive used by ordinary VM bitwise
operators and the prevalidated numeric loop executors; it contains no loop,
workload, source-path, or checksum identity.

The exact seven-block external gate used candidate, base, and QuickJS-NG
binary SHA-256 values
`053041633aff3067b7231516e4796895182854bf8c10c4ba15864b0f903a2274`,
`dcef12c705a1f0a862b74b7318dd7868a8e08e0822ae73c65a50bff14b84588a`,
and `cfd8386c3c29b1125a878b8fb82f9627820f2dcc16d2a691c5f8c16ad0b047a0`.
Candidate/base paired ratios with 95% bootstrap intervals were:

- SunSpider `bitops-nsieve-bits`: 0.856362x [0.848521x, 0.860219x];
- `bitops-3bit-bits-in-byte`: 0.845193x [0.835896x, 0.850157x];
- `bitops-bits-in-byte`: 0.957382x [0.936965x, 0.964626x];
- `bitops-bitwise-and`: 0.991787x [0.982219x, 0.999090x];
- `access-nsieve`: 0.989637x [0.988540x, 0.993571x].

The unrelated `hash-map` and `json-parse-financial` controls had interval
upper bounds of 1.01657x and 1.00620x. External manifest/raw/report SHA-256
values are
`5f4ed9bf8c2bc23742bc37e9038d3e71c324908f19a78cd70e9ff051955a4598`,
`861d53e14e57375014a395fd2db4178175d9486f3ceea9ab66cfe4a590b245d8`,
and `3cc1adf387c528031561e5e8f27226858024c5ef320a99bb27c5a2c3967a6746`.

The seven-block internal gate measured `dynamic_method_call` at 0.016279x
[0.016170x, 0.016364x] and `branch_arithmetic` at 0.868276x
[0.866463x, 0.871576x]. `local_read`, `empty_loop`, `array_read`, and
`array_write` ranged from 0.998160x to 1.003764x; every interval remained below
the predeclared 1.03x regression boundary. Internal raw SHA-256 is
`1028f7dd6ef9bf6b926c1d811b33739f85f66f90d53897ca1a8b4c1936a982e7`.

Correctness coverage includes explicit expected results at NaN, infinities,
signed zero, fractional values, both sides of the signed and unsigned 32-bit
boundaries, adjacent representable values around `2^32`, `2^53`, and the
largest finite magnitudes. A deterministic 100,000-pattern raw-f64 differential
test supplements those independent expectations. The focused conversion
tests, all 1,574 runtime tests, formatting, workspace clippy, branch-scope
validation, and `check-touched` passed before integration. The integrated full
Test262 and QuickJS-NG comparison gates remain mandatory before push.

### 2026-07-22 dense bit predicate scans

Agent commit `b1fb0b88` extends the numeric-mutation subsystem with a fail-closed
dense bit predicate plan. It recognizes the general bytecode relation
`array[index >> shift] & (base << (index & mask))` around a counted-loop
predicate; the shift, mask, and base remain plan data rather than workload
identities. Eligible loops lease fully dense Number storage once and scan
packed words with integer bit operations. The executor stops before the first
true predicate, releases the array borrow, publishes only completed counter
progress, and hands the observable true body back to ordinary bytecode. Holes,
special descriptors, non-Number elements, non-authoritative locals, direct
`eval`, unsupported numeric ranges, aliasing counter/limit slots, or any
failed entry guard retain the normal VM path. A zero-progress runtime mismatch
removes only that frame's plan so repeated deopts do not become a new hot path.

The first scalar scan prototype was rejected against its predeclared at-most
0.70x gate: exact seven-block `bitops-nsieve-bits` measured 0.784x base. The
packed-word refinement used the same frozen base and QuickJS-NG binaries and
measured 30.601 ms versus 57.398 ms base, or 0.53314x, with a paired 95%
interval of [0.532742x, 0.538585x]. Its six controls stayed below the 1.03x
regression boundary. A standalone element-construction probe was inconclusive
because all roles were dominated by constructing the 20,000-element input;
the exact scalar-plan-to-word-plan movement, 45.036 ms to 30.601 ms, is the
reliable executor-level diagnostic.

After rebasing onto the accepted common-range `ToUint32` unit, the combined
exact seven-block gate used candidate, frozen base, and QuickJS-NG binary
SHA-256 values
`7764eb7d0ba94d2cc90c12d7ca07ee64ce385b897df127c2cbc1d9640b551639`,
`dcef12c705a1f0a862b74b7318dd7868a8e08e0822ae73c65a50bff14b84588a`,
and `cfd8386c3c29b1125a878b8fb82f9627820f2dcc16d2a691c5f8c16ad0b047a0`.
`bitops-nsieve-bits` measured 26.223 ms candidate, 57.000 ms base, and
29.376 ms QuickJS-NG: 0.460x base and 0.893x QuickJS-NG. Paired 95%
intervals were [0.456426x, 0.460999x] and [0.884138x, 0.901987x]. Control
candidate/base point estimates were 1.006x for JetStream `hash-map`, 1.002x
for Kraken `json-parse-financial`, 0.992x for `access-nsieve`, 0.843x for
`bitops-3bit-bits-in-byte`, 0.959x for `bitops-bits-in-byte`, and 0.982x for
`bitops-bitwise-and`; the largest paired interval upper bound was 1.01057x.
Manifest/raw/report SHA-256 values are
`5f4ed9bf8c2bc23742bc37e9038d3e71c324908f19a78cd70e9ff051955a4598`,
`a53828c47ee6b37990881bfe05b86703a7b217d5f676f0ba08da772db47c805c`,
and `0398bf4728b8134e398684d954e8e63cef8f85d437bee195e101394f8a86ce03`.

The branch passed all 1,587 runtime tests, the related 65-case Test262 slice,
formatting, workspace clippy, file-size checks, `check-touched`, and branch
scope validation. Three independent review passes found no remaining P0--P2
issue after fixes for counter/limit aliasing, zero-progress deopt suppression,
and virtual-object lowering ranges. The mechanism therefore clears its local
generalization gate, while B5 remains open because 0.893x QuickJS-NG is still
above the final 0.50x every-case boundary.

### 2026-07-22 dense-predicate hosted closure

Main commit `7a9616af` closed the dense-predicate unit on hosted hardware.
Performance Preview `29963227131` retained all 25 broad cases and measured an
overall 0.9626x candidate/base ratio [0.9610x, 0.9672x] and 0.1522x
candidate/QuickJS-NG [0.1495x, 0.1546x]. The four broad cases still above the
strict 0.50x QuickJS-NG boundary were `local_read` at 0.5222x,
`object_allocation` at 1.1066x, `array_allocation` at 1.4176x, and
`closure_allocation_call` at 1.0912x. Hosted `bitops-nsieve-bits` moved to
0.412x candidate/base and 1.475x candidate/QuickJS-NG. Broad raw/report
SHA-256 values are
`f1c665d90d81e22403c1c597305330fb9538f84a6585d0b6c822aa97d2605404`
and `19e40359695933057594d35a7ce5103760b360fd84ade906d039fcd945086fad`.

The same artifact kept the external campaign open: diagnostic geometric
candidate/QuickJS-NG ratios were 7.543x for the five JetStream ports, 4.784x
for the 9/14 comparable Kraken ports, and 6.690x for all 26 SunSpider ports.
External raw/report SHA-256 values are
`477db87ccd0bfcc6e51d16625a7bca6e2c35aef7d70f6904471cd410749e96b2`
and `558bbf1ba8013bb79bd17a0c7ca086888d59b6876505526cd05f8ce401bc728f`.
CI run `29963227127` passed. Test262 Coverage `29963409223` independently
passed all 42,672/42,672 configured cases with zero failures, timeouts,
not-run cases, or actionable gaps; its burndown SHA-256 is
`32801f80331b523999ec14080dc102b588877353ffe043d39179f96871102171`.

An all-`Stable` fixed-shape numeric-loop kernel was also measured and rejected
without a commit. Its exact seven-block candidate/base ratios were 1.00086x
for `local_read` [0.99887x, 1.00606x], 1.00215x for `property_read`, and
1.00211x for `array_read`; the remaining controls were neutral. The mechanism
therefore added code without measurable movement. Candidate, base, and
QuickJS-NG binary SHA-256 values were
`821b818cc928bb3872f2112517202d11b48e2707ddae723e93f6ac5d4b3d96a2`,
`7764eb7d0ba94d2cc90c12d7ca07ee64ce385b897df127c2cbc1d9640b551639`,
and `cfd8386c3c29b1125a878b8fb82f9627820f2dcc16d2a691c5f8c16ad0b047a0`.
Raw, manifest, and workload SHA-256 values are
`102d6695686a130d23af05b32d5fe855f6c427d195100c45ffdd06e05552ab0f`,
`869042a8adbd0bf6e3cafd05adbc3ef6c0d7b6f3c927b7d8a3eadf75d45fdbe0`,
and `e4c79e512f11bd3716cba0f2a85d650cc120594b6f06370acd92f74952c3248c`.

### 2026-07-22 multi-array numeric loop regions

Agent commit `ef35008d` generalizes the computed-index dense mutation plan from
one read/modify/write receiver to a bounded straight-line numeric region: up
to eight distinct dense Array receivers, 32 ordered stores, 256 Number
instructions, and 64 numeric locals. The translator records receiver identity
and preserves ordinary expression results. The executor leases each distinct
array once, forwards same-iteration staged stores to later loads in source
order, preflights every current-iteration access, and commits all stores only
after the final potentially failing instruction. Aliased receiver slots,
holes, non-Number elements, special properties, frozen arrays, borrow
conflicts, non-authoritative locals, direct `eval`, and unsupported bytecode
all fail closed. A failed guard publishes only earlier complete iterations,
releases every lease, and restarts the ordinary VM at the loop header for an
exact replay of the first failed iteration.

The common one-receiver/one-store path remains allocation-free. Programs up
to 64 instructions use inline registers; larger admitted regions allocate an
exact-size register vector. A unique store with no later array access is sunk
after the Number program, while regions requiring store-to-load forwarding
retain the transactional instruction. The final code-generation repair marks
the shared binary-operation helper always-inline. This restores the old
single-RMW hot loop's inlining while retaining one generic, monomorphized
instruction executor for both Single and Multi access; release assembly
changed from two hot `bl apply_binary` sites to zero.

The performance screen was deliberately strict: both targets needed a
candidate/base interval upper bound at most 0.95x, their geometric mean at
most 0.90x, and every unrelated control at most 1.03x. Four earlier frozen
seven-block candidates were rejected rather than relaxing it. They measured
strong target movement but regressed `bitops-nsieve-bits`: `a88316cb` was
1.108x [1.105x, 1.133x], `b76bfdbb` was 1.094x [1.083x, 1.098x],
`284484c0` was 1.035x [1.031x, 1.039x], and `59752e2d` was 1.061x
[1.057x, 1.068x]. Those iterations respectively exposed whole-register-file
initialization, remaining Single-path allocation/abstraction, and the
out-of-line binary helper. Each was kept out of history until the shared
legacy control recovered.

The accepted exact seven-block Gate5 used candidate, base, and QuickJS-NG
binary SHA-256 values
`ece6efd42c04b3f4e2772af74becc7ea0bdbe34d8dd603eb42b75a6041c3dc99`,
`7764eb7d0ba94d2cc90c12d7ca07ee64ce385b897df127c2cbc1d9640b551639`,
and `cfd8386c3c29b1125a878b8fb82f9627820f2dcc16d2a691c5f8c16ad0b047a0`.
Candidate/base paired estimates with 95% intervals were:

- Kraken `audio-fft`: 0.490253x [0.487265x, 0.492119x];
- SunSpider `access-fannkuch`: 0.763370x [0.754279x, 0.763899x];
- target geometric mean: 0.611755x [0.606245x, 0.613070x];
- JetStream `hash-map`: 0.992550x [0.992020x, 0.996946x];
- Kraken `json-parse-financial`: 1.003025x [1.001779x, 1.009403x];
- SunSpider `bitops-nsieve-bits`: 0.993481x [0.987597x, 0.996616x];
- Kraken `audio-dft`: 1.000543x [0.996702x, 1.021939x].

The target candidate/QuickJS-NG ratios remain 1.632896x for `audio-fft` and
2.341656x for `access-fannkuch`, so this unit materially reduces two external
gaps but does not close B5. Manifest, raw, external-report, and paired-report
SHA-256 values are
`335eb1b4cbf23f2d94a55a3363b25e863ec9fabdb1c1dc8a92fffcb4c32e9c40`,
`c71cde426f78959bcd76369d8d1b52f013e24e1aaa6be81a6cc65ae55883497c`,
`e37c555dc4d05c067a406c60b927f4cc2d835d34a5a7e48f0afc4b0918399042`,
and `47a13fb7e637ac2d479b0025f876399ea637dcfba1381a0935864877efe020bd`.

Coverage includes multiple arrays, multiple ordered stores, same-array
forwarding, runtime alias fallback, entry and mid-loop deoptimization,
out-of-bounds preflight, store/load/replay ordering, sunk-store assignment
results, and conflicting leases. The staged touched gate and commit hook each
passed workspace formatting, clippy, file-size checks, all 1,597 runtime
tests, and the related 65-case Test262 slice. Independent reviews found no
remaining P0--P2 issue after fixes for integer progress overflow, register
remapping, zero-allocation Single dispatch, and sunk-store replay ordering.

### 2026-07-22 multi-array hosted closure

Main commit `aafdd827` closed the multi-array unit on trusted hosted state.
CI run `29967261430` passed. Test262 Coverage `29967443565` independently
passed all 42,672/42,672 configured cases with zero failures, timeouts,
not-run cases, or actionable gaps; its burndown SHA-256 is
`b546b6989a32a312b7268ae1b2f11b915853c19beb6b7cfed5c81333f0c5f0a0`.

Performance Preview `29967261448` completed with all three blocks and all
75 N/2N groups valid. The non-fixed hosted runner measured broad-v2 at
1.02616x candidate/base [1.01018x, 1.02616x] and 0.15648x
candidate/QuickJS-NG [0.15551x, 0.15680x]. This run also exposed the explicit
tradeoff behind retaining the unit: several already-leading call micros moved
backward, while the external bottlenecks moved materially. Kraken `audio-fft`
changed from a base timeout to a runnable candidate at 3.184x QuickJS-NG, and
SunSpider `access-fannkuch` measured 0.683x candidate/base. Both remain above
the final 0.50x QuickJS-NG boundary, so the campaign stays open and the exact
local Gate5 remains the controlled acceptance evidence for this mechanism.
Broad raw/report SHA-256 values are
`6f9682e6f58d2394a857eb0a30083a202cb12c2ef2721d14f378379c48c9f7a5`
and `f09f6cf96f285e80ba9cd2e51081c242e35102b0c5094d9b6633aaf420b71876`;
external raw/report SHA-256 values are
`b8a4f81452d6b097a0b5956f0ebfe4b03d4fe8645923aa3f8c24ffe494c98fba`
and `8c9953769f853b7390dc3e8ff859dfd76bc91ca56bf941bdce4c6c300bf9ba76`.

### 2026-07-22 read-only dense numeric reductions

The dynamic dense region now admits straight-line Number programs with dense
loads and scalar writes but no array stores. It resolves up to eight local
Array receivers, validates holes, special properties, and storage/length
consistency, then holds shared immutable element leases for the complete
region. Shared leases deliberately allow receiver aliases and frozen arrays;
borrow conflicts, non-Array receivers, non-Number elements, out-of-bounds
indices, non-authoritative locals, captures, or direct `eval` fail closed. A
failed current iteration publishes no scalar writes. Earlier complete scalar
iterations are published only after all leases drop, and ordinary bytecode
replays the failed iteration from the loop header. The existing writable
Single and Multi paths are unchanged.

Temporary path instrumentation was removed before measurement. It proved that
the mechanism, rather than unrelated code layout, covered the pinned targets:
JetStream AES recorded 12,600 region entries, 126,000 accelerated iterations,
and 2,520,000 dense loads; Kraken AES recorded 33,248 entries, 340,224
iterations, and 6,804,480 loads. Both had zero bailout. Gaussian blur,
`audio-fft`, `json-parse-financial`, and `bitops-nsieve-bits` recorded no
read-only-region hit under the same instrumentation.

Exact-binary discipline rejected two tempting shortcuts. The first successful
seven-block screen used release SHA-256
`4100a7c5e281d4a69cc91764909351d98636a6b94dad412e4b769f9d1b7a2030`,
but post-review test hardening changed the final release identity, so that run
was not used to accept the commit. The first seven-block screen of the final
`326b4be49746e9b2220d3f599b35ee7f32fc8b94125118ab440b9b90ff307a9b`
binary kept both targets below 0.84x base, but `json-parse-financial` and
`bitops-nsieve-bits` had wide control intervals above 1.03x. That run is
durably retained as a failed gate; its raw, report, and paired-report SHA-256
values are
`ea6c8078c564bedcc379b650e54c7b5299c34f170dd1ae1b799cfbe5d54c5958`,
`3a3c49517164df55f5c1f93e868fbe42ab8461233e94e48ac151467559944b54`,
and `61f7f0e04253ca5ad76a49389782b65855d70da714e6c95dc5147db38f3160ab`.

A predeclared 21-block precision confirmation then retained the same six
cases, order, three binaries, thresholds, and all 378 measurement samples.
Candidate, base, and QuickJS-NG SHA-256 values were
`326b4be49746e9b2220d3f599b35ee7f32fc8b94125118ab440b9b90ff307a9b`,
`ece6efd42c04b3f4e2772af74becc7ea0bdbe34d8dd603eb42b75a6041c3dc99`,
and `cfd8386c3c29b1125a878b8fb82f9627820f2dcc16d2a691c5f8c16ad0b047a0`.
Paired candidate/base estimates with 95% intervals were:

- JetStream `stanford-crypto-aes`: 0.829944x [0.827313x, 0.831558x];
- Kraken `stanford-crypto-aes`: 0.809532x [0.806625x, 0.812977x];
- target geometric mean: 0.819674x [0.817546x, 0.821712x];
- JetStream `gaussian-blur`: 0.998374x [0.995020x, 1.001739x];
- Kraken `audio-fft`: 1.003798x [0.993454x, 1.007382x];
- Kraken `json-parse-financial`: 1.005040x [1.003566x, 1.006793x];
- SunSpider `bitops-nsieve-bits`: 0.994412x [0.990522x, 1.004535x].

The target candidate/QuickJS-NG estimates remain 2.790117x and 2.801455x,
so B5 remains open. The 21-block manifest, receipt, raw, external report, and
paired-report SHA-256 values are
`d8c58ba36efe5e02215420851f94c19989e5fd80dcaf2c4f98e67d4d28b66bc2`,
`10bf8f41d9c9a8af70ddc27cbc0d65457fb132c9fcc560eaebea214a4b897a35`,
`e0428311e75f43081a61f6ab6cc3cec7f11591b8bc812a7efcf22d34f9e36caf`,
`25c61837eeab2f11d46c41bbd3eb82729f1838b246f9eafc4650ade64dffaef9`,
and `ce96ffda381d355c8df92e88fc6b435265e86408fbab71254e4addb6dc3c00c4`.

Coverage includes aliased and distinct five-array AES-shaped rounds, comma
sequencing, frozen arrays, unrelated outer captures, rejected captured
receivers and direct `eval`, holes with observable prototype getters, borrow
conflicts, and zero- and mid-progress replay after non-Number or out-of-bounds
loads. The branch passed 8/8 focused read-only tests, the immutable-lease test,
16/16 existing mutable dense regressions, all 1,606 runtime tests, the related
65-case Test262 slice, formatting, workspace clippy, file-size checks,
`check-touched`, and branch-scope validation. Three independent final reviews
reported no remaining P0--P2 issue.

### 2026-07-22 read-only dense hosted closure

Main commit `3cbc57a7` closed the read-only dense reduction unit on trusted
hosted state. CI run `29970185890` passed. Test262 Coverage
`29970329829` independently passed all 42,672/42,672 configured cases with
zero failures, timeouts, not-run cases, or actionable gaps; its burndown
SHA-256 is
`f5cf8465e42842e7a797030a7d4cd62367929bc83cdbe70efee571a7b79cffee`.

Performance Preview `29970185871` completed all three hosted blocks. The
informational 25-case micro portfolio measured 1.053555x candidate/base
[1.050604x, 1.059878x] and 0.162611x candidate/QuickJS-NG
[0.161718x, 0.162639x], but classified the result as inconclusive on variable
GitHub hardware. The intended external AES targets did move in the same
direction as the exact local gate: JetStream AES measured 0.824x base and
4.626x QuickJS-NG, while Kraken AES measured 0.813x base and 4.541x
QuickJS-NG. Unrelated hosted allocation and dynamic-array rows moved much
more than their source paths justify, so this run is closure evidence rather
than the acceptance gate; the exact local 21-block result above remains the
mechanism decision. Broad raw/report SHA-256 values are
`d3a2b1e335380b53d3defa63e876fbeed476930cebc0d82de8c30ae20f7b692b`
and `9b4b5e60041d7a2c5303ffc4555e1b414afcb685f6784e9f2dbc13cfc567cb07`;
external raw/report SHA-256 values are
`0f897ecac27874beb402b8d32438172264371b02678473266155518df33231f4`
and `067924546e151a28b27344b6c7c92b9a00ac5652f9fe02aaf34dccaa80c1d308`.

### 2026-07-22 invariant dense sources and Array length

Agent commit `5824d1dc` extends read-only dense numeric regions beyond local
array receivers. A non-global direct-leaf frame may now resolve
`this.<name>` once when `<name>` is an own ordinary data property containing
an Array; a fused local `array.length` may supply the loop bound. Named
sources are rejected for Proxy, accessor, inherited, symbol-primitive,
typed-array, or module-namespace behavior, and both forms remain unavailable
to writable regions. Captured or lexical `this`, direct `eval`,
non-authoritative slots, holes, special indexed properties, non-Number
elements, out-of-bounds reads, or borrow conflicts fail closed. A progressed
failure publishes only earlier complete scalar iterations, drops all leases,
and replays the failed iteration from the ordinary header; named sources and
the Array length are resolved again on the next entry.

Temporary instrumentation on the pinned Kraken `audio-dft` case recorded
10,240 region entries, 10,475,520 accelerated iterations, 41,902,080 dense
loads, and zero bailout. The final source patch SHA-256 is
`e8dd66e5ca971d403ffd306bffc27db7cabff605abc93f3d6f6e2f7c3b1e561b`;
candidate, base, and QuickJS-NG executable SHA-256 values are
`a4acc55ffdd01272e6df5be8a6a697da07d16c0d77935971e2a146a6212f4602`,
`326b4be49746e9b2220d3f599b35ee7f32fc8b94125118ab440b9b90ff307a9b`,
and `cfd8386c3c29b1125a878b8fb82f9627820f2dcc16d2a691c5f8c16ad0b047a0`.

The predeclared seven-block gate required the `audio-dft` candidate/base
interval upper bound to be at most 0.60x and four unrelated control upper
bounds to be at most 1.03x. Final paired candidate/base estimates with 95%
intervals were:

- Kraken `audio-dft`: 0.216137x [0.214294x, 0.223564x];
- JetStream `gaussian-blur`: 1.002355x [0.983471x, 1.008128x];
- Kraken `audio-fft`: 1.005669x [0.999845x, 1.007458x];
- Kraken `json-parse-financial`: 0.990914x [0.990337x, 0.995326x];
- SunSpider `bitops-nsieve-bits`: 0.994169x [0.976582x, 1.000244x].

The target candidate/QuickJS-NG estimate is 0.878929x
[0.865767x, 0.903991x]: this unit turns the previous `audio-dft` deficit into
a lead, but it does not reach the final 0.50x boundary. Manifest, raw,
external-report, paired-report, and summary SHA-256 values are
`70ff53124f08970475037be4021d01149a268efe2fb280dd82ad9e07a3991ffc`,
`76ccbb2ea6d2d605ae0efc04b3adaa02f83b6a5ea7284babfbd2404cbd68c8cb`,
`d3680757baf6bb9bfe532b86ba5a9aee0c61ca5a12671f219bdb30d33a472fdc`,
`7f60b1093f67db50a306d66e3dc8f859cdaa578a8e58db91ebbfc52587ff7d87`,
and `3c37e853ef0fc9e400fddd4649a11464c55688b4119dd143ed22c9bf0da0e841`.

Exact-binary review discipline rejected an earlier passing candidate rather
than accepting its speed. Independent review found that an unused
`this.tick;` expression could leave an unresolved named source for `Pop` to
discard, skipping an observable getter in accelerated iterations. The final
translator rejects discarded `DirectThis` or `ArraySource` values, and the
regression proves ten generic getter calls with no compiled region. Two
earlier invocations also stopped before any timing sample because one corpus
SHA was mistyped in the first manifest; the corrected manifest changed only
that validated source hash and retained the same cases, seed, blocks, and
thresholds. The final branch passed all 1,614 runtime tests, all 5,148 curated
Test262 cases, the 65-case touched slice, formatting, clippy, file-size and
branch-scope checks. The reviewer that found the P1 and a separate final
reviewer found no remaining P0--P2 after the repair.

### 2026-07-22 invariant dense sources hosted closure

Main commit `330602b3` closed the invariant-source unit on trusted hosted
state. CI run `29972324830` passed. Test262 Coverage `29972462375`
independently passed all 42,672/42,672 configured cases with zero failures,
timeouts, not-run cases, or actionable gaps; its burndown SHA-256 is
`69de8dca08da1a6426e75dba22a3579e87c89ead7733c6a88d0f18c8d9f4ed86`.

Performance Preview `29972324836` completed all three hosted blocks. The
informational 25-case micro portfolio measured 1.000504x candidate/base
[0.998863x, 1.007477x] and 0.159378x candidate/QuickJS-NG
[0.158126x, 0.161083x], and correctly classified the variable-host result as
inconclusive. The admitted external suites reported 5/5 JetStream cases,
11/14 Kraken cases, and 26/26 SunSpider cases. The local exact-binary gate
above remains the mechanism acceptance evidence; the hosted run supplies
post-integration conformance and build closure. Broad raw/report SHA-256 values
are
`fb9709d44ea6f31205d901895b7a50d32275f7441a8d55a9543b58b4bbde3e09`
and `c00f1cf34e9d0aef1f88b267f82750185876aaf1d11b2e6d6c9b58706e34841f`;
external raw/report SHA-256 values are
`e99187fe21990699671aeba2dc948be5ef2f14528d06e6ae436949778dfa04f6`
and `57eaa7f5d00c20d2a873ca28bae98b728c0cbf34ae3e41c92e52f2a1dc94c40c`.

### 2026-07-22 rejected virtual scalar allocation regions

A general virtual-scalar prototype was evaluated and removed without commit.
It first proved that a loop-local object or Array did not escape through
identity, capture, property-key coercion, calls, or unsupported control flow,
then represented Number-only fields and elements as transactional scalar state
inside the existing numeric-region machinery. The prototype matched structure
and safety properties rather than benchmark names, and retained ordinary
bytecode replay on every failed guard.

The predeclared discovery rule formed a high-reuse set `H` from external cases
with at least 10,000 candidate iterations. Across the fixed 45-case external
corpus, all 44 measurable cases recorded exactly zero compiled entries and zero
accelerated iterations. The only non-completing case, Kraken
`imaging-gaussian-blur`, timed out again in an isolated 15-second retry after
15.018748 seconds with exit status -9 and empty output. Therefore `H` was empty:
the mechanism had no demonstrated external reuse and was rejected before any
timing gate, despite being semantically general.

The rejected candidate executable SHA-256 is
`04798f7d7f28195b16b35f4f0944860e2e61d896932a447a97d42cfc46cec7d3`.
Manifest, raw, external-report, and summary SHA-256 values are
`a8ddeded582573bc676bf3f7bbbaf2625f6dfa7742f07bcdd6aaa26366f4e6c4`,
`bb9ea6845837813d1bac55476848c80d968e6d43aced1bd37060ea6e06852f07`,
`477856a3a7962c4dc81f122125e0de4599b495eb75655a060d18f3a9285fc13b`,
and `3191ac4bc14225f8c437a21def2a24322daf091e85d8e23bf262ba5d8da3010e`.
The isolated retry bundle and result SHA-256 values are
`54a60944b49ce59f85524db09ea4eba6f2137036e19487a45844d728b843bd38`
and `3205d31d0bd4236c869816fb993ce1a8c2aaabccb116d9a970a162a20ae2b8e1`.
This negative result redirects ROI toward bytecode shapes already proven hot
in admitted workloads; it is not evidence against a future allocation model
rewrite with independently demonstrated coverage.

### 2026-07-22 postfix countdown dense regions

Agent commit `bba359fb` extends the existing dynamic dense numeric region to
the exact bytecode semantics of `while (local--)`. The plan recognizes
`LoadLocal`, `ToNumeric`, `Dup`, decrement, assignment to the same local,
`JumpIfFalse`, and `Pop`; it accepts only positive safe-integer Numbers at the
accelerated entry and leaves BigInt, non-finite, fractional, negative, and
larger-than-safe values to ordinary bytecode. The body may not write or capture
the counter, and direct `eval`, non-authoritative slots, unsupported control
flow, holes, non-Number elements, or failed array leases continue to fail
closed.

Each fast iteration stages the old counter, makes the decremented value visible
to the body, and commits the counter and array stores only after the complete
iteration succeeds. A mid-body deopt restores the old value before replaying
the header, preventing a double decrement. A clean exit still performs the
observable final `0 -> -1` update and skips the bytecode exit `Pop`. The
existing less-than control path remains distinct and unchanged.

Temporary instrumentation on pinned Kraken `imaging-desaturate` proved 200
region entries, 21,359,800 accelerated iterations, 64,079,400 dense loads,
64,079,400 committed stores, and zero deopt or bailout. The process returned
Boolean `true` with exit status zero. The trace, aggregate, and evidence
manifest SHA-256 values are
`761382d9882b280ae12347700459d81a12754980e52a79c664c66d00ac8551bb`,
`3bdeed3278f33d974a93cc265131955620e32381456dbea3627c6dd23ff9d219`,
and `6fd551ad1744c5c7291c717a5e76eddfc9c4be94d11b6a440e22473e2a9fcd5f`.
The final production patch SHA-256 is
`c289f21cf69810020742782c15d46975a78e323a529562182409b781d3dfdb51`;
candidate, base, and QuickJS-NG executable SHA-256 values are
`085e5346a0e7d42d1ddf122a61786be5daa7b9a212a20b426f4f0e279879b327`,
`a4acc55ffdd01272e6df5be8a6a697da07d16c0d77935971e2a146a6212f4602`,
and `cfd8386c3c29b1125a878b8fb82f9627820f2dcc16d2a691c5f8c16ad0b047a0`.

The predeclared seven-block gate required the target candidate/base interval
upper bound to be at most 0.50x and six unrelated control upper bounds to be at
most 1.03x. Final paired estimates with 95% intervals were:

- Kraken `imaging-desaturate`: 0.169857x base [0.168010x, 0.172582x]
  and 0.425978x QuickJS-NG [0.424679x, 0.436486x];
- JetStream `gaussian-blur`: 0.989485x base [0.982999x, 1.012773x];
- Kraken `audio-dft`: 1.012521x base [0.998715x, 1.022641x];
- Kraken `audio-fft`: 1.006229x base [0.992373x, 1.007871x];
- Kraken `json-parse-financial`: 1.006163x base [1.001847x, 1.014894x];
- SunSpider `access-fannkuch`: 0.998539x base [0.989274x, 1.000700x];
- SunSpider `bitops-nsieve-bits`: 0.986930x base [0.919044x, 0.998935x].

The target medians were 8.940103 seconds for base, 1.515023 seconds for the
candidate, and 3.539484 seconds for QuickJS-NG. Manifest, raw, external-report,
and paired-report SHA-256 values are
`421b3b3cfe521fe29db350912f0193944f03331e33f9723e5da23cfa6578347e`,
`4fe0b8600f1be82ed3bb9052f8555e723556cc13839ae669d02a783754b034af`,
`17b526b0c62e96e7884680335c3a276d7e762df835f75f03c85620ac65c7aa50`,
and `56f5c7705c4acedfbfa513ee1dd89e9bd41996310b1d722ad7ac3e65fd7cde58`.

Coverage includes the exact countdown shape, post-decrement value visibility,
the final `-1`, zero and mid-progress replay, holes, getters, prototype hazards,
Proxy and frozen arrays, captured counters, direct `eval`, body writes, and all
rejected Number forms. The branch passed 8/8 focused countdown tests, the
existing less-than control test, all 1,623 runtime tests, the related postfix
decrement Test262 case, formatting, clippy, file-size checks, the 65-case
touched Test262 slice, and the complete `check.sh` gate. Branch-scope validation
passed, and two independent final reviews reported no remaining P0--P2 issue.
This closes `imaging-desaturate` below the final 0.50x QuickJS-NG boundary;
B5 remains open until every admitted benchmark reaches the same boundary.

### 2026-07-22 postfix countdown hosted closure

Main commit `0f1e147d` closed the postfix-countdown unit on trusted hosted
state. CI run `29975064583` passed. Test262 Coverage `29975194172`
independently passed all 42,672/42,672 configured cases with zero failures,
timeouts, not-run cases, or actionable gaps; its burndown SHA-256 is
`15bc10937515649895736d9639132ed154423c76f7464d64df441708fff19400`.

Performance Preview `29975064579` completed all three broad blocks, all 225
measurements, and all 75 N/2N groups. The variable hosted runner measured
0.953803x candidate/base [0.949188x, 0.953803x] and 0.155544x
candidate/QuickJS-NG [0.153824x, 0.156196x]; the workflow correctly retained
the result as inconclusive rather than replacing the exact local gate. The
external inventory completed 5/5 JetStream cases, 11/14 Kraken candidate/base
comparisons, 12/14 Kraken candidate/QuickJS-NG comparisons, and 26/26
SunSpider cases. Diagnostic suite geometric ratios were 0.997605x base and
6.963502x QuickJS-NG for JetStream, 1.002429x base and 3.086192x QuickJS-NG
for Kraken, and 0.992223x base and 6.371524x QuickJS-NG for SunSpider.
`imaging-desaturate` remained ahead of QuickJS-NG at 0.689456x on that runner,
while `audio-oscillator` remained a major 8.819397x deficit.

Broad raw/report SHA-256 values are
`1605e55cd626ed9f8dbe9cb9d96e3dac4e8efeda5eef7191a23d2d4e54502519`
and `1f08a4507d57d10f8eee248b24e8c38f30f1c6b533d89e40ffc4720215d512e7`.
External raw/report and summary SHA-256 values are
`760723072c83eda8e622397fd21ac2d0bb5bf7b38b0679573cfb15bac37291f7`,
`8cfba7c725d5ab91e3ad779d32be2485c035025fce0f0c28b3083d4f0e0ff707`,
and `8610551730bd73926a9c36d12b191e1ef1e89ac23f362fad6822c8a64988f701`.

### 2026-07-22 own-data dense regions and exact shared reductions

The next runtime unit extends the dynamic dense numeric region in two related
ways. Writable regions may resolve ordinary own-data Array and Number sources
from either direct `this` or an authoritative local once per region, and may
execute the exact native `Math.round` while the global `Math` object and its
own `round` data property still resolve to the native identity. Proxy,
accessor, inherited, symbol-primitive, typed-array, and module-namespace
owners fail closed. Replaced or inherited `Math.round`, direct `eval`, captured
or non-authoritative locals, holes, borrow conflicts, aliases between readable
and writable arrays, and non-Number elements retain ordinary bytecode. A
zero-progress stable lease failure suppresses only the current invocation;
progressed failures publish only complete earlier iterations and replay the
failing iteration from the original header.

Pure read-only regions additionally recognize one to eight ordered
multiply-add reduction lanes followed by the ordinary counter increment. The
generic reduction kernel preserves source operand and lane order, publishes
all accumulators and counter shadows only after a complete iteration, and
deoptimizes before exposing partial lane work. The highest-value two-lane
strided form uses checked integer Array indices when counter and strides are
exact non-negative indices. When both lanes have the same compiled sample
receiver, stride, and current coefficient index, it loads that immutable
sample once, advances one checked coefficient recurrence, and still performs
the two separate multiply-then-add operations in source order. It does not use
FMA, reassociate arithmetic, skip iterations, or key behavior to a benchmark
name, input size, checksum, or source path. Fractional, out-of-range,
overflowing, sparse, accessor-backed, captured, or otherwise unsupported
states fall back to the existing floating-index or ordinary bytecode paths.

The dense compiler, invariant resolution, and compact legacy executor were
split into semantic submodules as part of the same unit so the new guards did
not push the original source file past the first-party size limit. The compact
legacy representation remains the executor for existing simple dense regions;
the reduction is an optional read-only plan, not a second general VM.

#### Rejected candidates and measurement discipline

A general last-write-wins publication prototype was evaluated and removed. It
deduplicated repeated staged writes without recognizing a benchmark, and its
focused correctness tests passed, but exact same-host A/B evidence made Kraken
`audio-dft` **5.81% slower**: new/old was 1.058090x with a 95% interval of
[1.032014x, 1.114149x]. `gaussian-blur` and `bitops-nsieve-bits` also had
interval uppers above the 1.03 control ceiling. The paired-report SHA-256 is
`c52ddff5195305657c9c1d97350fc2b5e898786b337d8773cf548686e484713c`;
the prototype was reverted rather than hidden inside the final candidate.

The first two-lane strided reduction candidate was also rejected by its hard
retention gate. Its frozen executable SHA-256 was
`9990f1bc7ade452b1e602f426a2d7180f9d78767cda62629a810f8c3e1a93e08`.
All base and control gates passed, but `audio-dft` candidate/QuickJS-NG was
0.502310x [0.501167x, 0.510874x], above the required 0.50 interval upper
bound. Raw, external-report, external-summary, paired-report, and
paired-summary SHA-256 values are
`12e64a5128cdedf907aea63561b7512637d2c8b83720eb0b645a1e3a698dea3d`,
`d85eaf80b6b4b84eedb80b6b9e266efc43fd3c03c8e6829398e017e5da3efd75`,
`3503faea71ba27dedc9dac59c19f2b61b14b7f644ff8ab906424b1b3214c232d`,
`1d78cd80a1489e7bdcfc20d6920512b5dc76ac20c402d220ba3ac7905b3c63d0`,
and `8df10dd4b71f24dec15e9f7637e3dfa9f33ac171b1c2816b7e19284517062517`.
The exact-index and shared-sample tuning runs were used only to select the
successor; they remain explicitly non-claim discovery evidence and were not
substituted for the final single-shot gate.

#### Accepted single-shot gate and path proof

The final candidate, preceding base, and pinned QuickJS-NG executable SHA-256
values are
`9b987cbcba67fa564237ddbcc94ceb244a13c907f7ee6a2064cc7e855729cf93`,
`085e5346a0e7d42d1ddf122a61786be5daa7b9a212a20b426f4f0e279879b327`,
and `cfd8386c3c29b1125a878b8fb82f9627820f2dcc16d2a691c5f8c16ad0b047a0`.
A fresh preregistration froze one seven-block run, exact source/binary pins,
the two targets, five unrelated controls, and every interval threshold before
measurement. All 168/168 rows completed in that one formal invocation. Paired
estimates with 95% intervals were:

- Kraken `audio-dft`: 0.552633x base [0.550112x, 0.557935x] and
  **0.491273x QuickJS-NG [0.477951x, 0.495005x]**;
- Kraken `audio-oscillator`: 0.467067x base [0.465550x, 0.467774x];
- JetStream `gaussian-blur`: upper control bound 1.008600x;
- Kraken `audio-fft`: upper control bound 0.961257x;
- Kraken `json-parse-financial`: upper control bound 0.998499x;
- SunSpider `access-fannkuch`: upper control bound 1.001892x;
- SunSpider `bitops-nsieve-bits`: upper control bound 1.008186x.

Thus `audio-dft` now crosses the final 0.50x QuickJS-NG boundary and
`audio-oscillator` is cut by more than half relative to the exact preceding
base. This accepts the general mechanisms in this unit; it does not imply B5
completion because oscillator and many other admitted external cases remain
above 0.50x QuickJS-NG.

Preregistration SHA-256 is
`75cfb9b2c25844d9c408ab668ac087355bef536931dbb8940aeb9ec4094e4f45`.
Manifest, raw, external-report, external-summary, paired-report, and
paired-summary SHA-256 values are
`114a64feea29d270245077e7939fcfb6b54e0a126bcf0dc2e9437db90c0e478b`,
`213e232edc380d05bcf73c6b1613e28007ba2c232a0728e6090632631667323f`,
`d191e540b58f413cb69f63d3b77989fcf38142f3f43a0b0cb1172b8595b5a613`,
`93141db34fef001921a8f0d1fea81da17b1a2b61425efa7e5028aaec6db8074b`,
`531d5f1d19e4621bbcf7ccda35566168a157943682bdb776fe8a127ca83b80e9`,
and `723394492c05f29f1455522366e4fc6070c41b84b914cae12ef4780f533b048c`.
Independent reconstruction from the 168 raw rows reproduced every paired
estimate and confidence interval with a maximum floating-point difference of
`7.55e-15`; no second formal output exists.

Compile-time-only, release-only, runtime-gated instrumentation was then built
in a fresh Cargo target and removed with reverse patches before verification.
Per-iteration proof counters stayed invocation-local and merged into TLS only
once at exit; both targets also had a post-marker overflow guard. On the
exact `audio-dft` bundle it recorded 10,240 dispatcher entries, all 10,240 on
the exact shared-sample kernel, 10,475,520 committed iterations, and
10,475,520 loads at each of the first coefficient, shared sample, and second
coefficient sites. Exact-nonshared selection, floating fallback, load failure,
deopt, replay, decline, and lease failure were all zero. On the exact
`audio-oscillator` bundle it recorded 1,500 attempts, 999 handled entries, 501
invocation-local suppressions, 8,182,809 committed iterations and stores,
12,278,309 physical dense loads, 4,087,309 native round operations, and zero
decline or deopt. Both processes exited zero and wrote exactly the two expected
external-success lines to stdout.

The authoritative v5 proof binary SHA-256 is
`be0d40896bf5ecdce69c206cd22f9fc712e0c3d22ee4d45a9655f61461471d75`;
DFT bundle/stdout/stderr SHA-256 values are
`57fcc07e83044e5552d83e4bf13a1e10af6e71357dddf28ea3c64de8dbd0bbda`,
`f5bc4f369844bf414bcaa550808d7e5406037ad4da77bde3ae30fcaa7701bdfc`,
and `7c1662c8af7275d52c3b65aa72e7e1556a77c704846e5366ed3b190c66f1f53b`.
Oscillator bundle/stdout/stderr SHA-256 values are
`b302c0457bc183da4106148640e17964033829769da144f537d2d7180a44c1df`,
`f5bc4f369844bf414bcaa550808d7e5406037ad4da77bde3ae30fcaa7701bdfc`,
and `a46f1fdc7a9d003130c025377794efae902236169589303c8c5e393646eb3421`.
The preceding v4 proof is retained as rejected protocol evidence because its
oscillator hot loop accessed TLS for each operation; none of its binaries or
outputs is used for the accepted path claim.
Post-proof source, staged/unstaged diff, and frozen-binary sentinels all
returned to their pre-proof SHA-256 values, and every proof-only symbol is
absent from the source tree.

Focused coverage exercises native-round identity and edge values including
NaN, infinities, negative zero, adjacent half ties, own-data owner hazards,
lease suppression, and one- through three-lane reductions. The implementation
caps plans at eight lanes. Coverage also includes exact-index overflow,
operand order, sample aliasing, sparse/prototype access, zero- and mid-progress
replay, direct `eval`, capture guards, and borrow conflicts. The restored
candidate passes formatting and all 1,662 `qjs-runtime` release tests. The
staged touched gate passed formatting, clippy, the source-size guard, all 1,662
runtime tests, and its 65 selected Test262 cases. The complete workspace check
also passed, including all 5,148 curated Test262 cases, and all 218 current
QuickJS-NG comparison fixtures passed. Trusted-main hosted closure remains the
post-push step.

#### Trusted-main hosted closure

Main commit `5d6bb73080095812678247e2e22cc6bc6a497d2e` closed the
own-data/shared-reduction unit on trusted hosted hardware. Test262 workflow
`29998583152` completed all **42,672/42,672** pinned cases with zero failures,
timeouts, not-run cases, or actionable gap. The downloaded burndown JSON
SHA-256 is
`10eee6df4b84210d2bfe70b4f01404a55f85b869c75313b9575a1db541bcb276`.

Performance Preview workflow `29998334159` completed successfully with exact
candidate, base, and QuickJS-NG source revisions `5d6bb730`, `0f1e147d`, and
`f7830186`; their hosted executable SHA-256 values are respectively
`d9a93a5acb7a3a4b738c75ab67de96b2c4c060bc6a34a65e135ada304cf46d46`,
`10279d035c94c74adbb1bc19cc7d446f38bdd5372027db73420c700b34eba446`,
and `8614a5a91e3476db1a1300b0969387b85e0716a836f799cf243a80d4d1f27699`.
All 225/225 broad measurements, all three blocks, and all 75 linearity checks
completed. The three-block profile is intentionally informational and its
health verdict remained `inconclusive`/`non_claim`: broad candidate/base was
1.017537x [1.017495x, 1.020194x], while candidate/QuickJS-NG was 0.162868x
[0.162849x, 0.164279x]. These hosted broad ratios do not replace the exact
same-host seven-block acceptance gate above.

The external inventory completed all five JetStream ports, 12/14 comparable
Kraken cases, and all 26 SunSpider cases. Candidate/base diagnostic geometric
means were 1.002209x, 0.881338x, and 0.992857x; candidate/QuickJS-NG means were
7.181513x, 2.693966x, and 6.579981x. The targeted directions reproduced more
strongly than the local gate: Kraken `audio-dft` was 0.444368x base and
0.377408x QuickJS-NG, while `audio-oscillator` was 0.439941x base. JetStream
`gaussian-blur` remained 1.001763x base and 6.601032x QuickJS-NG, identifying
fixed TypedArray backing access as the next high-ROI path. `ai-astar` and
`imaging-gaussian-blur` timed out for both qjs-rust roles and remain capability
gaps rather than comparable performance results.

Downloaded raw/report, external raw/report/summary, and status SHA-256 values
are respectively
`bae227308d9c2ec9bbf5efde49caa2568e0ed105ab11850c9b6fc41b18722e17`,
`30b51c95fbc6a50f99c4c8c564c8177cdac14bd169e9a00135cfe9aafb413bec`,
`fbc8eafb3645dd8465d49b2ff72fee3079b9195f1cec74e65b950c652b9e23d4`,
`5ea31647c8e345a174881a103ba238da1311c67ee3621d829e37d5b923086435`,
`882caebdf326c3cd7d1f3e5a52ca37324ae9527f2a3b02c2c452f409b8cccbeb`,
and `de89869b44fd3c2261a81dc4ac5074899d149a9d02022dde203c72f751bbe1ea`.

### Fixed Number TypedArray dense backing lease

The next unit attacks the fixed Number TypedArray backing traffic exposed by
hosted `gaussian-blur`. A compiled dense numeric region now resolves its fixed,
in-bounds Number TypedArray views once, acquires one stable mutable byte lease
per distinct non-shared backing, lowers typed loads, stores, arithmetic, and
control to a compact typed program, and executes forward `< limit` and exact
descending `>= 0` loops without repeating observable property or backing
lookups per element. Aliased views, shared or resizable backing, detached or
immutable buffers, unsupported element kinds, borrow conflicts, invalid
geometry, non-number live-ins, and any unsupported operation fail closed to
ordinary bytecode. Stores remain iteration-atomic: pending bytes publish only
after every load and operation in the iteration succeeds, and a deopt replays
the first uncommitted iteration through the ordinary VM.

The supporting ArrayBuffer and TypedArray state is explicit rather than a
benchmark-local shortcut. Stable fixed-buffer leases share the same detach,
resize, immutable, and shared-backing guards used by ordinary runtime
operations. Integer-indexed `set`, `defineProperty`, integrity-level, proxy,
Reflect, DataView, resizable-buffer, growable-shared-buffer, and immutable
ArrayBuffer behavior was tightened and covered at the ordinary runtime layer.
The mechanism is independent of benchmark names, source text, sizes, and
checksums.

The forward Gaussian form initially exceeded the dense compiler's live-out
limit because repeated stores appended one `LocalWrite` per bytecode store,
even when every write targeted the same local slot. The compiler now coalesces
live-out publication by local slot with last-write-wins ordering. This is a
general correctness-preserving representation change: a focused test compiles
and executes more than 64 repeated writes to one slot, while a Gaussian-shaped
multi-store case proves that unsafe publication patterns remain suppressed.
The repaired forward plan has 97 operations, two receivers, four stores per
iteration, and 33 unique live-outs; the descending plan has 124 operations,
three receivers, one store, and 28 live-outs.

To keep first-party files reviewable, the stable ArrayBuffer and TypedArray
`ObjectRef` inherent methods were token-equivalently moved from `object.rs` to
private child modules. Three independent reviews found no P0-P2 issue in the
whole unit and two token-normalized split reviews found no semantic, visibility,
feature-gate, or API drift. The final files are 1,910 lines for `object.rs`, 129
for `array_buffer_methods.rs`, and 26 for `typed_array_methods.rs`, all below
the source-size guard.

#### Rejected reverse-only candidate and repaired path proof

The first frozen candidate was not accepted even though its 21-block timing
gate passed. Its source and release binary SHA-256 values were
`932b69aef4594870281ae09f14bc4031e052fd877a92078060cc0fd082a5dbbb`
and `59d1c572dc418cf82565b62122298b9021d18cb558145e28a05791c21630d6a8`.
The exact combined Gaussian proof observed only the 1,250 descending regions,
718,750 iterations, 3,593,750 loads, and 718,750 stores; all 1,250 forward
regions stayed on ordinary bytecode. The preregistered requirement was 2,500
regions covering both directions, so protocol verdict SHA-256
`0e1c7b7715e7b80942886144bb245f0257299f4e788c6412d48f78870a6ad8a8`
rejected it despite Gaussian candidate/base upper 0.540061 and six clean
controls. Timing alone was not allowed to hide missing mechanism coverage.

After the unique-local compiler repair, the exact bundle produced 2,500/2,500
attempts, hits, normal exits, and lease sets; 6,250 leased views and backings;
1,437,500 iterations; 4,312,500 loads; and 3,593,750 stores. Declines,
suppressions, lease failures, and deopts were all zero. The 1,250 forward and
1,250 reverse regions each executed 718,750 iterations. The authoritative
pre-split source diff, release binary, proof binary, and path receipt SHA-256
values are
`1582e1b9b5de15066b31eefabdc70a19d3739c24916ae2dd57b8a0622fe33726`,
`59801e9ef58081a17702e392c5acebb13345f6209282ac718cd602156fe62ca1`,
`5f2d17bbdb8ffb69a3396e15479451111f47f8486b428e45c89bb5392aa92dc1`,
and `e322cf5d6fe39c167e2b840a3224b221a851d528b67d92dfb0b19fb942d96620`.

One attempted formal invocation was stopped when unrelated same-host profiling
was discovered. No raw timing file, report, or numeric result was produced or
used. Contamination verdict SHA-256
`bd7349988ea7350d1ef69c14cf7169a2e7888c001d2421b30f0fa5f99b84ef2f`
preserves that rejection. A fresh preregistration and seed were then used for
the isolated run below; the stopped invocation was never retried in place.

#### Accepted pre-split performance gate

The fresh isolated 21-block invocation completed all 462/462 rows: 21
capability probes and 441 measurements, with no error, timeout, truncation,
missing row, duplicate key, identity mismatch, or order drift. Median paired
whole-block log effects with 20,000 bootstrap draws produced:

- JetStream `gaussian-blur`: 0.190134x preceding base
  [0.189688x, 0.191482x] and 0.633386x QuickJS-NG
  [0.631898x, 0.644209x];
- Kraken `audio-dft`: 0.990976x base [0.979970x, 0.999511x];
- Kraken `audio-fft`: 0.985489x base [0.981525x, 0.990693x];
- Kraken `audio-oscillator`: 0.990542x base [0.989024x, 0.995501x];
- Kraken `imaging-desaturate`: 0.973103x base
  [0.970536x, 0.976498x];
- Kraken `json-parse-financial`: 0.997738x base
  [0.993051x, 1.004524x];
- SunSpider `bitops-nsieve-bits`: 0.999675x base
  [0.992072x, 1.002693x].

Thus Gaussian retains an approximately 5.26x improvement over the exact
preceding base and all six unrelated controls remain under the preregistered
1.03 upper bound. Gaussian is still above the campaign's final 0.50x
QuickJS-NG boundary, so this accepts the mechanism without claiming B5
completion. Raw, external report, external summary, paired report, paired
summary, formal audit, and protocol verdict SHA-256 values are respectively
`d56dfd2c56135ddbdcd6705463a922ceabd882f118d60e369987c46995e651af`,
`c8355b37750ea1623ab4b93b1321cc902823fbb72cf05369057c0a475e2d1890`,
`7bfef52d09f2b3c0883c308681d0f41488b57ec9b4c87e876a9d3a4773b7f267`,
`2d945b92d8a5e02e5e88f7c609e2947454133bbe5149940b42a1c95dcb1be731`,
`408050e4961ef7c04ef3421e4012cac7582e27e911461072089a5ce44c680dfa`,
`1f9d15e111f51e5a18c2e4ca7a8302c7d6e0321c9a5b703d73b2bc5794b5183b`,
and `45b509f3171872566c8f6f3b45aa2faf3386ec6ae564efc51206e42a71073153`.
Independent reconstruction matched every paired estimate and interval with
maximum absolute difference zero.

#### Final source-layout equivalence closure

The final staged source diff and release binary SHA-256 values after the
token-equivalent file split are
`503755aebf1169f0bbe439099562dde0ed0c19b91b66ccb9bffb2d27eb692e99`
and `cf3969a96b5897248bfa12a633d4c8f692fac8fd11774b87054129f0e31690d6`.
A post-preregistration proof reproduced every combined-path count above; its
proof binary and receipt SHA-256 values are
`0f4c9275069a8fb55186472e4ec805057d677b0e442bfe162ccd4c5875119a5b`
and `02380c8eb43a3a9b82656245d74dbbc93baa04fce2c9968fb7733ce3415665bb`.

One seven-block final/pre-split equivalence invocation then completed all
168/168 rows. Every preregistered final/pre-split 95% upper bound was at most
1.03:

- `gaussian-blur`: 0.990905x [0.976301x, 1.010137x];
- `audio-dft`: 0.993802x [0.980025x, 1.023387x];
- `audio-fft`: 0.994995x [0.990382x, 1.007792x];
- `audio-oscillator`: 0.993171x [0.989366x, 1.006249x];
- `imaging-desaturate`: 0.998168x [0.991705x, 1.003325x];
- `json-parse-financial`: 1.000068x [0.994647x, 1.012162x];
- `bitops-nsieve-bits`: 0.996471x [0.993607x, 1.003464x].

Gaussian final/QuickJS-NG was 0.636052x [0.635545x, 0.641217x], below the
0.70 retention ceiling. Raw, report, summary, paired report, paired summary,
formal audit, and final protocol verdict SHA-256 values are respectively
`18a0a5c2441344bb09dde6a0acb7db8b10668c769024713f8e6ba435f3d3701b`,
`89fe1fd7fac49da6311b7f8569d0b3e1a5e56e20ebaa7c30ab6abad5d83dbfd2`,
`6d0dee88404e1add75b7385d43b4d5af38963064b5879498c48686eb583dbc63`,
`3bb8d462768a587b33756d1e8317719bf9f60ceff1769a9cc97815404cd726f8`,
`7396c3f4e8d950175421722406c95902c0b8a855d339c6328bfa29d583abfeb6`,
`4046637effc16d895b038c7c1c8e996c2ead0c2ae680e045c62a96bd2843bc87`,
and `e73444ee5ef5615d21ff9e0bb80cd5c28b8b0b32fd9042d727711fbb475853b2`.
Two independent raw reconstructions matched the generated external and paired
reports with maximum absolute difference zero.

Commit-eligible verification passed `check-touched` with 1,698 runtime tests
and 65 selected Test262 cases, the full repository gate with all 5,148 curated
Test262 cases, the all-features runtime suite with 1,709 tests, and all 218
QuickJS-NG comparison fixtures. The branch source commit is `55795330`.

Exact `audio-oscillator` instrumentation then identified the next high-ROI
slice: 1,500 dense attempts but only 999 hits. The remaining 501 invocations
were suppressed because `new Array(N)` has a logical length of `N` while its
dense vector initially materializes only the first zero or one elements. The
ordinary-Array mechanism that closes this gap is recorded below.

#### Trusted-main performance evidence for the fixed TypedArray unit

Main commit `c8da917bdaa98981cc2c0882f660066378abc766` passed hosted CI workflow
`30015374254`. Full pinned Test262 workflow `30015693597` completed all 42,672
configured cases with 42,662 passes, 10 failures, zero timeouts, and an
`actionable_gap` of 8; the same run recorded QuickJS-NG at 42,602 passes and 70
failures. The downloaded burndown JSON SHA-256 is
`3090757a4cb1e9bf8f7161b7a7e6266d2201bbdda5c2b3284ef020778f993a35`.
The 10 failures were a pinned-Test262 metadata mismatch: those tests omitted
the immutable TypedArray factory exclusions required by the runtime semantics,
while the runtime's immutable indexed-descriptor and rejected-write behavior
was correct. Official upstream commit
`250f204f23a9249ff204be2baec29600faae7b75` adds those exclusions. Main commit
`c119c897` backports exactly the corresponding metadata for 11 affected tests
without skipping their mutable-factory assertions. Hosted Test262 workflow
`30022997362` then ran all 42,672 configured cases at exact commit `c119c897`:
all 42,672 passed, with zero failures, timeouts, not-run cases, or actionable
gaps. Its burndown JSON SHA-256 is
`81b9c6dc85ce1dc49caef332e5e7f1aa68758167c2da149214226a3280f9537e`.

Performance Preview workflow `30015374367` completed successfully. Its
informational three-block internal portfolio measured candidate/base at
0.9924x [0.9885x, 0.9924x] and candidate/QuickJS-NG at 0.1624x. The external
inventory reproduced the intended Gaussian direction at 0.15156x base and
0.97952x QuickJS-NG, while `audio-oscillator` remained 1.01171x base and
3.97017x QuickJS-NG, confirming the ordinary Array tail as the next target.
The status, external report, and external summary SHA-256 values are
`a3edb3434adaa2500b3bac4585c3db50ea1bbec032068e72caf20b18ac35cb45`,
`8dd126c1a028ef167b0d1f087320e07a1575f4f62d4995cfb3127464a7af10fc`,
and `00f355355354a7a66b6509d3cff1542224c248674d2bee3c006a4582100e7ab6`.

### Ordinary Array implicit-hole-tail append lease

A dynamic numeric region may now lease one extensible ordinary Array for
append-only materialization of an existing implicit hole tail. Compilation
admits only a single store at the exact induction index of a forward `<`
loop whose counter advances by exactly one. Runtime requires that the index
equal the materialized dense prefix length and remain below both the existing
logical length and the one-million-element dense-storage cap. Every other
receiver is fully dense and read-only, the writer cannot alias a reader, all
element borrows must be stable, the writer has no explicit holes or indexed
descriptors, and its effective realm prototype chain has no indexed hazard.
A non-writable length is intentionally allowed below the existing logical
length because this creates an indexed property without extending `length`.
Non-extensible, sealed, frozen, described, aliased, borrowed, custom-prototype,
and overflow states, plus arrays with explicit holes or non-tail sparse state,
fail closed to ordinary bytecode.

Stores remain iteration-atomic. The append adapter stages exactly one Number
write and pushes only after the complete iteration has validated; a deopt
publishes complete earlier iterations and replays the first uncommitted one.
For fresh tails, one opportunistic `try_reserve_exact` is bounded by
`min(ceil(loop endpoint), logical length, dense cap) - current prefix`; NaN,
negative, exhausted, and allocation-failure cases reserve nothing or continue
through normal `Vec::push`. Fully dense regions never execute this reserve
path. Store-only stride, shifted-index, and multi-store loops are rejected at
compile time instead of repeatedly probing an inapplicable lease.

The first independent performance review found two issues before freeze: the
compact legacy executor could retry a structurally impossible store-only plan
at every backedge, and fresh 8,192-element tails incurred repeated geometric
growth. The compiler now rejects non-append store-only shapes, exact append
structural failure suppresses only the current invocation, zero-progress
deopt remains `Declined` for replay, and the bounded reserve removes repeated
growth. A second full review by three independent reviewers was clean with no
P0-P2 findings. Production fast-path code contains no benchmark/source/
checksum/expected-result detection and no branch on known benchmark input
sizes; all size-dependent logic is generic bounds and reservation logic.

#### Exact path proof and accepted formal gate

The exact Kraken `audio-oscillator` bundle was run with a separate release
proof binary using per-invocation local counters. It recorded exactly 1,500
attempts and 1,500 writable hits: 999 fully dense entries plus 501 append
entries. The append path made 4,103,691 pushes; the combined regions executed
12,286,500 committed iterations and stores, 16,382,000 dense loads, 8,191,000
native round operations, and 1,500 sunk-store hits. Append failure,
single-receiver hit, suppression, decline, progressed deopt, and zero-progress
deopt were all zero. The production staged diff at proof freeze had SHA-256
`2800dbd124222d96f117007bbe8206cf661211c6c9e648425415cf3ccf80bb26`,
with no unstaged tracked diff or proof symbol. Proof receipt, proof binary,
instrumentation patch, and bundle SHA-256 values are
`e81c8f7eaadced13805cf6bec14cacb72a5549a9a5ae3a50c46bf3639af468f9`,
`0a91421a76d2a09b85eff8a9de338332f2127d180d5fb0a3d90267dc144fcb60`,
`79021bcc0b174610e54d5c4106800f27c35461e08808df1301a3d408548c895f`,
and `b302c0457bc183da4106148640e17964033829769da144f537d2d7180a44c1df`.

A separate three-block discovery run was used only to freeze the formal block
count and thresholds. Preregistration SHA-256
`f31daff239fe7b4e95882a61db5c4543f12efe272ca96fd71ad7095ad4086d44`
then pinned one seven-block invocation, seed 20260725, exact candidate/base/
QuickJS-NG binary SHA-256 values
`b914394dad9a3a54db5df36508e4b6be7d4082e15e68fd89961aef8b6e21d0a5`,
`cf3969a96b5897248bfa12a633d4c8f692fac8fd11774b87054129f0e31690d6`,
and `cfd8386c3c29b1125a878b8fb82f9627820f2dcc16d2a691c5f8c16ad0b047a0`,
plus the target upper bounds of 0.20x base and 0.50x QuickJS-NG. It also pinned
six preregistered non-target controls with 1.03x base upper bounds. No build,
test, profile, or other benchmark overlapped the single formal invocation.

All 168/168 rows completed: 21 capability probes and 147 measurements, with
zero missing or duplicate keys, errors, nonzero exits, timeouts, truncation,
identity drift, source drift, or schedule drift. Median paired whole-block log
effects with 20,000 bootstrap draws produced:

- Kraken `audio-oscillator`: **0.154588x base [0.152603x, 0.159224x]** and
  **0.418002x QuickJS-NG [0.408291x, 0.426792x]**;
- JetStream `gaussian-blur`: base upper bound 1.022352x;
- Kraken `audio-dft`: base upper bound 1.003201x;
- Kraken `audio-fft`: base upper bound 1.005371x;
- Kraken `imaging-desaturate`: base upper bound 1.014987x;
- Kraken `json-parse-financial`: base upper bound 1.001058x;
- SunSpider `bitops-nsieve-bits`: base upper bound 0.994521x.

Thus `audio-oscillator` is approximately 6.47x as fast as the exact preceding
base, using 84.5% less wall time, and approximately 2.39x as fast as QuickJS-NG,
using 58.2% less wall time. Its entire confidence interval is below the
campaign's final 0.50x boundary, and all six control gates pass.
Independent reconstruction from the raw rows matched every estimate and bound
with maximum absolute difference zero. Raw, external report, external summary,
paired report, paired summary, formal audit, and protocol-verdict SHA-256 values
are respectively
`9ac6d4347bde7d53cd5b3e10107d4ddf5342c97a6fe016c922fa5d49e203cd10`,
`8a3a282dd83731f895bf0bde581e8f1dbcb0ba9f4b7c45c2f4cadaf37019b5ca`,
`9722d193f29970b0e14fbba8aad5499c13f4a956d6698e1a26c0427c30091503`,
`c7858758afeebad1769104c4b80f926572950df9ab38f2410a81e63e0f5f8553`,
`01b521e04970afaf52f1967662d500904374745df42367d46397d71b366d8c6d`,
`1e4ba15353e91f73b4ba85c4d6e674afaa134491d03280c258691d3b473d91e6`,
and `45d22112d1eafe9c95e4eead1b336d2923bcc242b361b76ad230e7f0aa352fbd`.

Focused hole-tail tests passed 15/15, the runtime release suite passed all
1,715 tests, and 14 focused Array semantics Test262 cases passed. The staged
touched gate passed formatting, workspace Clippy, the source-size guard, all
1,715 runtime tests, and 65 selected Test262 cases. The final three-reviewer
snapshot was clean. The full repository gate passed all 5,159 curated Test262
cases, and all 218 QuickJS-NG comparison fixtures passed. Branch commit
`36905b7c` passed CI `30023617310`; it integrated on main as `b420c81e`, whose
CI `30024220377` also passed.

Hosted Test262 workflow `30024516032` closed the append unit at exact main
commit `b420c81e`: all 42,672 configured cases passed, with zero failures,
timeouts, not-run cases, or actionable gaps. The burndown JSON SHA-256 is
`c427b71c4cb87eae12922e0c4b9975d09e5001963938d4fd04c7ad80d976b5bf`.

Performance Preview workflow `30024220328` failed closed on attempt 1 because
QuickJS-NG's `plain_function_call` block 0 was `timer_limited`; candidate and
base measured all 25 cases, but the common set was only 24/25. The failed
status, raw rows, and partial report SHA-256 values were
`bac9f0bf6f0ca6f75997c0a6e42081fdf93b30412d05945b65b69d1e922686d1`,
`7cdc055ce047077268eb462c145686fcb4bde37f670234bde9387d88b1efafce`,
and `6ed790a998a3162798fcdc1f7b8c4493063fe4f2f0b40bcae3fe5c06c0db8b00`.
Attempt 2 then completed successfully with all 25 internal cases, all three
roles, all three blocks, and passing linearity. Its informational hosted
candidate/base aggregate was 0.9862x [0.9828x, 0.9873x]. The external preview
remained partial at 43/45 comparable cases because `ai-astar` and
`imaging-gaussian-blur` timed out for qjs-rust; `audio-oscillator` measured
0.133x base and 0.514x QuickJS-NG on that variable runner. The attempt-2
status, raw, report, external raw, and external report SHA-256 values were
`69dac8c08c6c3cc2472f804fcbd4987f9188cc64f080a753197cf4a90c34a35a`,
`dda35829d33235e02dfdf1728cd74e1c3bebe1a7120d6b99381994d30113c18b`,
`7777c728168e2f24511376adee95ba997b52f46661b14a6d401ef1cd710a867c`,
`652dbb5ed9e4f3f5b020a271b5d942a1a843cb73b8f8b887c94bd50abff8654d`,
and `7fefeb25b52f5696b2b75a5107845f89e8c4afa2d3c9794b0bfb1eb445024380`.
The hosted profile is explicitly `inconclusive`/non-claim and does not replace
the isolated local seven-block acceptance gate above.

This closes the local `audio-oscillator` performance gate, not B5 or the
correctness campaign. The same formal matrix still places Gaussian at
0.641878x, FFT at 1.542264x, JSON parse at 0.765637x, and nsieve at 0.886772x
QuickJS-NG. Subsequent profiling below supersedes the earlier
transition-shape/full-frame hypothesis: the next ROI work targets the repeated
entry boundary around an already-general dense numeric loop, not another
benchmark-local array case.

### Immutable loop-plan zero-clone screen (rejected)

A follow-up prototype replaced the three per-frame owned loop-plan vectors
with slices borrowed from the immutable `Bytecode` `OnceCell` caches. Numeric
and mutation selected plans were borrowed instead of cloned; control plans
were changed to execute by reference. Numeric-mutation zero-progress
suppression moved to a normally empty frame-local sparse vector and was carried
through generator snapshots. Three independent reviewers converged cleanly
after fixing two review findings: suppression lookup now filters by backedge
before consulting the sparse set, and control execution no longer implicitly
copies its selected plan. The final uncommitted diff SHA-256 was
`bf14d1a2c55b4db8024ef333fc1378fa70162cb075278f472bbf9b7b614d8de6`.

The predeclared acceptance rule required the seven-block FFT candidate/base
95% interval upper bound to be at most 0.95x; a one-block seven-case direction
screen had to justify paying for that formal run. Exact candidate, base, and
QuickJS-NG executable SHA-256 values were
`6daf3f3d8d5c00a7aa9b02f373b9c4930831f83d40d440f059b6a5b512d45bb0`,
`b914394dad9a3a54db5df36508e4b6be7d4082e15e68fd89961aef8b6e21d0a5`,
and `cfd8386c3c29b1125a878b8fb82f9627820f2dcc16d2a691c5f8c16ad0b047a0`.
The screen measured `audio-fft` at **1.006835x base**, so it showed no target
benefit and could not reach the required five-percent win. Controls were
Gaussian 1.047032x, DFT 1.024812x, oscillator 0.997340x, desaturate 1.023437x,
JSON parse 1.047257x, and nsieve 0.999289x base. The formal seven-block and
25-case runs were therefore not started, and the runtime change was rejected
without a commit.

The frozen screen manifest, raw rows, and report SHA-256 values are
`4f3eb760b567d44eaf8fca20690c978a3d6a4d011b1dfb3c18898be5b1767768`,
`359aacaab1df3aafaeac1680c64e90f794ce80fd484b1f120e047dc84644c1ae`,
and `32fa41b85c68d8d56ac456ba399a35b9406cb098fbed6b8c1aa683c140fa4e06`.
The compact rejection record is
`target/benchmarks/rejected/loop-plan-zero-clone-plan1/external-screen-result.json`.
This confirms the earlier setup-only rejection on a real external workload:
plan ownership is not the current FFT bottleneck. The next FFT unit must start
from a profile of the dense/register execution path instead of adding frame or
locals pooling.

### `audio-fft` nested-dense ROI selection

A two-run macOS sample of the exact `b420c81e` base executable
(`b914394dad9a3a54db5df36508e4b6be7d4082e15e68fd89961aef8b6e21d0a5`)
used the unmodified external bundle with SHA-256
`07fd2de1a2d39f430923300a61bda3a67f34b0e68b855fde10162f36c387147a`.
The two sample artifacts have SHA-256 values
`5234dc93de2c8707fb54cdfadbe8224474b3585de614d47ac336595c4b52501a` and
`90cec2ddc00d756a92c56c9c0a0df6ccf027076d22d19462e39003db44a21c7b`.
Pooled self samples assigned 46.04% to `Vm::run_completion`, 15.68% to the
dense register program, 6.96% to `Value::drop`, 4.01% to `Value::clone`, and
3.50% to fast numeric binary operations. Plan search itself was only 0.35%,
confirming that changing immutable plan ownership could not provide material
FFT benefit.

Static path counting and region-isolation timing identified the butterfly as
about 86% of `FFT.forward`; the final spectrum/square-root loop was about 12%,
and initial reordering was under 2%. Each of 1,000 forward calls enters the
existing dense inner plan 1,023 times but executes only 5,120 butterflies,
averaging five useful iterations per entry. A general two-level region that
fuses one numeric outer loop around one existing dense inner loop can reduce
the target's total entries from 1,023,000 to 10,000 without absorbing the full
function or any benchmark identity. Because takeover occurs after one seeded
ordinary butterfly per entry, the path proof must report exactly 10,000 nested
entries, 1,023,000 completed outer iterations, 5,110,000 native inner commits,
10,000 seeded ordinary inner iterations, 5,120,000 total butterflies, and zero
steady-state bailouts. The direction screen is preregistered to require
`audio-fft` candidate/base at most 0.80x while keeping all six controls at most
1.03x; failure rejects the unit before a formal run.

## Notes

Broad v2 is still a first-party micro portfolio, not a substitute for an
admitted external macro suite. T017's external-corpus audit remains the path to
broader public claims. The immediate purpose is to make optimization robust to
code-shape changes and multiple runtime subsystems before chasing the 2x goal.

2026-07-20: opened `T019-object-layout-rewrite.md` to attack the persistent
`allocation` critical family gap (2.56x-7.56x across recorded units, never
reached QuickJS-NG parity) as a narrower alternative to a full GC/arena
rewrite. `third_party/quickjs-ng/quickjs.c` confirms QuickJS-NG itself uses
refcounting plus a periodic cycle-collecting mark/sweep pass, not a
tracing/moving GC, so the gap is attributed to `JSObject`'s flat C layout
rather than to memory-management strategy. T019 shrinks `ObjectData`/
`ArrayData`/`PropertyStorage` layout in separately measured slices; land its
units the same way as any other T018 structural-bottleneck unit. S1 (box
`PropertyStorage::Dynamic`) landed with a measured, non-regressing 1.2% win
on `object_allocation`/`array_allocation`; S2/S3 (bitset-pack scattered
`Cell<bool>` fields) were attempted and rejected — Rust's default struct
layout already absorbs single-byte fields into existing alignment padding,
so consolidating them saves zero bytes here. Full writeup in
`tasks/T019-object-layout-rewrite.md`.

2026-07-20: re-measuring the full 25-case portfolio against QuickJS-NG (not
the stale initial-baseline table above) surfaced a much larger, previously
uncatalogued outlier: **`top_level_function_call` at 9.897x QuickJS-NG** —
worse than every case T019 targets. `dynamic_method_call` (3.854x) and
`array_write` (2.446x) are also larger than the `allocation` family cases.
Root cause for `top_level_function_call`: `Vm::store_local_slow`
(`crates/qjs-runtime/src/bytecode/vm_bindings.rs`) re-fetches `globalThis`
from the realm binding map, calls `has_own_property`, and clones the
binding name up to three times on *every* write to a hoisted top-level
`var`, not just once. This is VM dispatch / environment-sync work, not
object layout — it belongs to a future T018 unit, not T019. Deliberately
not attempted without a dedicated session's verification budget: this code
sits in the realm/globalThis-sync territory that took many sessions to
stabilize historically (see the `Parity progress` memory entries on realm
semantics). Next session should prioritize this over continuing T019.

2026-07-20: accepted a general dispatch mechanism fix for `dynamic_method_call`
(3.854x QuickJS-NG). Profiling isolated the cost to `NamedPropertyCache`
(`crates/qjs-runtime/src/bytecode/ir.rs`): its single-entry cache thrashed on
every access because the benchmark's receiver alternates between two
distinct object shapes (`a.f()`/`b.f()` behind a ternary), so every call
rebuilt the cache from a full hashed shape lookup instead of hitting. Changed
the single `Option<NamedPropertyCacheEntry>` to a small fixed `[Option<_>; 2]`
round-robin polymorphic cache — a strict capability superset (a miss still
falls back to the existing slow path exactly as before; the only change is
that up to two receiver identities/shapes can be remembered instead of one).
Local three-role `benchmark.sh` A/B (3 blocks, 25 cases): `dynamic_method_call`
candidate/base **0.887x** (11.3% faster), candidate/QuickJS-NG improved from
3.854x to 3.404x; no other case moved outside noise. `method_call`
(monomorphic, unaffected by the fix) initially showed a 3-block 1.137x
reading; a 5-block/20-sample re-isolation converged to 1.008x, confirming
that was measurement noise from too few samples, not a regression. Added a
focused `named_property_cache_remembers_two_alternating_receivers` unit test.
Mechanism is general (any polymorphic call site benefits, not just this
benchmark shape); does not close the case's gap to QuickJS-NG, which remains
open for further work.
