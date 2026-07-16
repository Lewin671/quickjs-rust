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

- [ ] B1 freeze broad v1 workload, exact case/family inventory, manifest,
  protocol hashes, hosted preview contract, and documentation.
- [ ] B2 record the first complete three-role local baseline and identify the
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

## Notes

Broad v1 is still a first-party micro portfolio, not a substitute for an
admitted external macro suite. T017's external-corpus audit remains the path to
broader public claims. The immediate purpose is to make optimization robust to
code-shape changes and multiple runtime subsystems before chasing the 2x goal.
