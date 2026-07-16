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
