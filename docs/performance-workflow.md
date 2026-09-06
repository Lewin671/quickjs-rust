# Comparing qjs-rust with QuickJS-NG

The objective is a generally faster, correct engine. No single microbenchmark
or geometric mean establishes that result. Keep the three performance lanes
separate, and inspect resource measurements before a production-performance
claim:

| Lane | Question | Acceptance use |
| --- | --- | --- |
| Broad, 25 cases | Do the specializing tiers still accelerate their recognized workloads? | Specializer regression coverage |
| Generic sentinels, 6 cases | How fast are dynamic calls and property access? | Mandatory controls for every optimization |
| External, 45 cases | How long do pinned application-like scripts take from process spawn to exit? | Generalization and whole-process latency |
| Resource lanes | What do startup, peak RSS and binary size cost? | Independent production-readiness evidence |

A faster valid specialization is useful. It does not establish that ordinary
calls got faster. Likewise, external process-wall measurements include parsing,
startup and shutdown; they must never be labeled VM-only throughput. The
external stdout sentinel proves the adapter reached the end; correctness also
relies on each upstream workload's assertions/validation hook and Test262, not
on the sentinel alone.

## One complete local comparison

Build candidate, comparison base and the pinned NG reference separately. Use
verified build receipts matching the measurement manifests, as described in
[benchmarking.md](benchmarking.md#running). Do not reuse a receipt after
rebuilding an executable. The checked-in manifests describe macOS arm64; a
Linux run needs matching prepared manifests and receipts, as used by the hosted
preview. Both internal lanes must use the same build recipes and profile.

```sh
python3 -m tools.benchmark.compare \
  --candidate /path/to/candidate/qjs \
  --candidate-receipt /path/to/candidate-receipt.json \
  --base /path/to/base/qjs \
  --base-receipt /path/to/base-receipt.json \
  --quickjs-ng /path/to/quickjs-ng/qjs \
  --quickjs-ng-receipt /path/to/quickjs-ng-receipt.json \
  --blocks 30 \
  --output-dir target/comparison/run-001
```

The runner validates receipts before measurement, then runs the lanes
sequentially on one host. It builds nothing. External sources are fetched into
the hash-checked cache when absent. Existing output directories are refused.
`status.json` records the active phase and preserves failures; completed raw
files remain available even if a later phase fails. `summary.md` keeps separate
per-case lane tables. `summary.json` provides engine identities and decision
readiness and a sealed inventory of all artifact hashes. Readiness is only permission to evaluate a unit's gates, not a
performance claim or an accepted optimization.

Use `--blocks 3` for quick diagnostic feedback. Use a predeclared 30- or
60-block run for decisions. Do not repeat experiments until a favorable
interval appears, discard outliers, or pool unrelated runs. The frozen A/A
noise calibration and hardware-qualification work remains necessary before
turning this into a public claim or required CI performance gate.

## Evidence replay and opportunity queue

Keep the generated filenames together in the evidence directory:
`raw.jsonl`, `manifest.json`, `report.json`, `sentinel-raw.jsonl`,
`sentinel-manifest.json`, `sentinel-report.json`, `external-raw.jsonl`,
`external-manifest.json`, `external-report.json`, and `summary.json`.

```sh
./scripts/performance-decision.sh queue \
  --summary target/comparison/run-001/summary.json \
  --broad-report target/comparison/run-001/report.json \
  --sentinel-report target/comparison/run-001/sentinel-report.json \
  --external-report target/comparison/run-001/external-report.json \
  --output target/comparison/opportunity.json
```

The CLI reconstructs reports from raw data before trusting their metrics. It
checks the sealed same-directory artifact inventory, exact case inventories,
frozen measurement semantics, engine revisions,
executable hashes, reference pin, internal linearity/coverage, and consistent
host metadata. External replay additionally checks adapter flags, capability
results, the seeded role order, complete block identities, nonoverlapping
monotonic timers, and consistent workload/executable hashes. A content hash
alone is not evidence that two files belong to the same experiment.

Archived internal manifests are temporarily copied under `benchmarks/` to
resolve their frozen protocol inventory against this checkout, then removed.
Replay requires the matching measurement protocol and the current analyzer;
legacy report formats cannot silently pass. External schema-2 reports can be
reconstructed separately with:

```sh
./scripts/external-performance-preview.sh report \
  --input target/comparison/run-001/external-raw.jsonl \
  --output target/comparison/replayed-external-report.json
```

Older external raw files lacking effective block/timeout and host metadata, or
bundles lacking a sealed inventory, need a fresh run. Sealing prevents accidental
file replacement/mixing; it is not authentication against a malicious producer. The historical plans remain structurally readable through
`check-unit`; new queues use schema 2 and bind engine identities. A historical
plan's missing raw profile is not grandfathered into a new decision.

## Profiles before implementation

Profile the exact queue candidate executable on an identified workload. Keep
the actual sampler command and version, raw stacks/counters, and the exact
workload source. Import those existing artifacts into a portable receipt:

```sh
python3 -m tools.benchmark.profile \
  --input /path/to/current.sample \
  --binary /path/to/base/qjs \
  --workload /path/to/profile.js \
  --base-sha <full-queue-candidate-sha> \
  --opportunity-id external/sunspider-1.0/3d-cube \
  --tool 'sample <recorded-version>' \
  --command-json '["sample","<recorded-pid>","10","-file","/path/to/current.sample"]' \
  --output-dir target/profiles/current-case
```

The output supplies the receipt filename and SHA-256 for the plan's
`profile_evidence.source` and `sha256`. `source` is relative to `--profile-root`.
The receipt copies and hashes both raw profile and workload. Validation checks
those files and requires the exact queue candidate binary hash, not merely a
matching commit message or revision. An instrumented build with a different
binary hash is supplemental diagnosis; it cannot replace the exact-binary
sampling receipt. The `shared_cost` interpretation and `inclusive_fraction`
remain human-reviewed claims: inclusive stack counts overlap and must not be
summed as independent removable costs.

```sh
./scripts/performance-decision.sh validate-unit \
  --unit tasks/performance-units/<unit>.json \
  --queue target/comparison/opportunity.json \
  --profile-root target/profiles/current-case
```

For multiple receipts, place their directories under one profile root and use
relative paths such as `case-a/receipt.json` in the plan. `check-unit` checks
only plan structure; `validate-unit` checks actual profile artifacts too.

## Acceptance after implementation

Measure the candidate against the same base executable represented by the
queue, then evaluate the frozen plan:

```sh
./scripts/performance-decision.sh decide --mode promotion \
  --unit tasks/performance-units/<unit>.json \
  --queue target/comparison/opportunity.json \
  --profile-root target/profiles/current-case \
  --summary target/comparison/run-002/summary.json \
  --broad-report target/comparison/run-002/report.json \
  --sentinel-report target/comparison/run-002/sentinel-report.json \
  --external-report target/comparison/run-002/external-report.json \
  --test262-burndown /path/to/exact-candidate-burndown.json \
  --require-retained --output target/comparison/decision.json
```

Each watched comparison needs at least 30 complete paired blocks, a 95%
confidence interval and at most 3% relative half-width. The entire interval
must be within the target or control threshold to pass; an interval crossing
the threshold is inconclusive. A precise interval wholly beyond the threshold
rejects the change. Three samples cannot certify a 3% improvement.

`fast` checks declared targets/controls plus all six sentinels. `promotion`
also checks **every** broad and external case against the frozen control
regression ceiling, requires complete comparisons with NG, and requires the
exact candidate's zero-gap Test262 burndown covering the complete pinned Git
inventory, with consistent result counts. Improving one target cannot hide
a tenfold regression elsewhere. Migration `stage` uses the same evidence and
sentinel controls with its predeclared cumulative regression budget; it still
produces `advance`/`abort`, not a final performance claim.

`retained` means this optimization passed its unit gates on this experiment.
It never means the whole engine has surpassed NG. That broader conclusion
requires the separately reported NG comparisons, representative coverage,
resource evidence, conformance and calibrated hardware/noise policy.
