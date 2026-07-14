# T017: Performance Benchmark System

## Goal

Build a long-lived, reproducible candidate/base/QuickJS-NG benchmark platform
whose evidence can eventually support conservative regression gates and public
claims without coupling the Rust engine to the reference implementation.

## Scope

- Allowed paths: `benchmarks/`, `tools/benchmark/`, benchmark scripts and docs.
- Forbidden paths: engine semantics and `third_party/` outside a separately
  approved milestone.
- Owner boundary: serialize manifest/schema changes; external corpora are one
  independently reviewed admission unit each.

## Parallel Assignment

- Base sha: `fd8d5ba6e5e9ab00cd449591f9101839407024fc`
- Branch: detached task worktree (no branch created)
- Worktree: `/Users/qingyingliu/.codex/worktrees/61c8/quickjs-rust`
- Owner id: benchmark-system
- Integration owner: main agent

## References

- `AGENTS.md`
- `docs/architecture.md`
- `docs/benchmarking.md`
- `docs/harness.md`
- Pinned QuickJS-NG source SHA recorded by the repository gitlink

## Acceptance Criteria

- [x] M0 records external-suite license/capability/timing admission gates and
  blocks unaudited candidates.
- [x] M1 provides a strict versioned manifest, hashed first-party workload,
  independent shell adapters, seeded balanced plan, calibrated external wall
  timing, protocol identity, build-receipt provenance, bounded process-group
execution, validated raw JSONL, and failure/timeout records without dependencies.
- [x] M1 records per-sample measurement eligibility and complete three-role raw
  input readiness only; it never emits a final performance claim.
- [x] M1 probes version metadata on a disposable hash-verified copy, then
  executes separate unprobed, hash-verified snapshots of every binary and
  unique workload, preventing probe self-modification or post-validation
  source replacement from silently changing measurement inputs.
- [x] Manifest, planner, runner failure/timeout, and statistics primitives have
  Python unit tests; Rust-only smoke does not require initialized submodules.
- [x] M2 wires complete-block analysis/reporting and adds the T016 call/binding
  matrix and N/2N linearity evidence.
- [x] M2 separates portable measurement-v3 evidence from analysis-v1 policy,
  allowing an analysis-only revision to reinterpret unchanged raw bytes.
- [x] M3 adds bounded bootstrap decisions, run health, fresh-process/RSS/size lanes.
  - [x] Measurement-v4 names the throughput lane; analysis-v2 implements
    portfolio-whole-block invalidation, strict durable failure-state analysis,
    30-to-60 critical-family width decisions, and frozen retain/never policy.
  - [x] Resource measurement-v1 and analysis-v1 add separately versioned
    fresh-process latency, direct-child peak-RSS, and exact binary-size lanes.
- [x] M4 adds dev-only Criterion benches for public parse, compile, and
  parse-plus-compile lifecycle boundaries, with versioned fixtures and a quick
  smoke wrapper. Realm construction waits for a natural public API.
- [x] M5 provides a strict deny-only external-corpus v1 registry and a
  fail-closed audit command. V1 cannot represent admission; each future corpus
  requires a separately reviewed v2 content-hashed audit bundle.
- [x] CI policy infrastructure provides a deny-only v1 policy and hosted
  smoke-only workflow. It cross-checks current protocols but cannot configure
  fixed hardware, store evidence, enable gates, or make claims. The policy
  binds the workflow's exact path, bytes, and SHA-256.
- [ ] M6 establishes fixed-hardware A/A shadow baselines.
- [ ] M7 enables fixed-hardware nightly/release gates and, if justified,
  self-hosted PR sentinels.

## Verification

```sh
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s tools/benchmark/tests -v
bash -n scripts/benchmark.sh scripts/benchmark-report.sh \
  scripts/resource-benchmark.sh scripts/resource-benchmark-report.sh \
  scripts/lifecycle-bench.sh scripts/external-corpus-audit.sh \
  scripts/performance-policy-audit.sh
./scripts/external-corpus-audit.sh
./scripts/performance-policy-audit.sh
./scripts/benchmark.sh --dry-run --blocks 3 --case plain_function_call
./scripts/benchmark.sh --candidate target/release/qjs --blocks 1 \
  --case plain_function_call --output target/benchmarks/smoke.jsonl
./scripts/benchmark-report.sh --analysis-manifest benchmarks/analysis.json \
  --input target/benchmarks/full.jsonl \
  --output target/benchmarks/report.json
./scripts/resource-benchmark.sh --lane fresh --dry-run
./scripts/resource-benchmark-report.sh \
  --input target/benchmarks/resource-full.jsonl \
  --output target/benchmarks/resource-report.json
./scripts/lifecycle-bench.sh --quick
./scripts/check.sh
```

## Notes

The throughput lane is amortized black-box throughput, not VM-only execution.
Resource lanes remain separate raw/report contracts and never pool metrics.
Criterion lifecycle output remains a Rust-native diagnostic under
`target/criterion`; it never enters the black-box comparison protocol or a CI
threshold before M6. Criterion 0.7.0 is an exact, dev-only Apache-2.0 OR MIT
dependency because it supports the workspace Rust 1.85 floor; 0.8.2 requires
Rust 1.86. Default features are disabled and only `cargo_bench_support` is
enabled. Timed iterations defer output teardown. Fixture v1 length plus FNV-1a
drift sentinels require any content revision to become v2. Quick smoke uses an
isolated discarded Criterion home. The wrapper uses a fail-closed allowlist for
positional filters and explicitly documented display/run flags, rejecting all
other long options, short options, and clusters.

`scripts/microbench.sh` remains a non-authoritative quick probe. Do not add a
CI performance gate before M6 demonstrates the noise envelope. All M3 reports
keep `claim_eligible=false`.

M5 records five blocked source-pinned candidates and two excluded
evidence-backed decisions with zero admitted entries; Octane deliberately has
no pin. The registry is governance metadata, never headline evidence.
SunSpider is the preferred first v2 review after its per-file license inventory
and NOTICE disposition close; QuickJS-NG Web Tooling is blocked on shell flags,
bench-v8 and Kraken on neutral-referee plus license audits, JetStream 3 on a
truthful shell-subset boundary, and Octane/Speedometer remain excluded for
retired and browser-system-boundary reasons respectively. A future v2 admission
must bind a content-hashed audit bundle covering source-pin evidence, per-file
licenses and NOTICE, a repository-owned adapter, a neutral timing protocol,
and a case manifest with source hashes.

CI layering is ready without claiming M6 or M7 completion. GitHub-hosted CI is
smoke-only: policy audits, dry-run plans, and listing six lifecycle IDs. The
deny-only performance policy has no fixed-hardware fingerprint or evidence and
keeps nightly, release, and PR-sentinel gates disabled. Activation requires a
qualified fingerprint, 20 independent content-hashed same-binary randomized
A/A reports for nightly/release (30 for PR), a protocol-bound noise envelope,
and an additional demonstrated false-positive budget for the PR sentinel.
The validator compares the hosted workflow byte-for-byte with its allowed v1
definition and policy hash; extra triggers, jobs, actions, uploads, or commands
are rejected rather than inferred safe from selected text fragments.
