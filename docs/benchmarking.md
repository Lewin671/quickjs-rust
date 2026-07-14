# Performance Benchmarking

This repository treats performance evidence as a versioned experiment, not as
a single score. The authoritative throughput path is `scripts/benchmark.sh`;
independent resource evidence uses `scripts/resource-benchmark.sh`. Neither
builds an engine, downloads a corpus, or edits a submodule. Build candidate,
base, and the pinned QuickJS-NG reference separately, then pass their executable
paths to the runner. QuickJS-NG remains a black-box reference, never a Cargo or
FFI dependency.

## Current Series Contract

`benchmarks/manifest.json` freezes the schema, series and suite identities,
profile, expected case set, family and critical membership, workload SHA-256,
validated operations/checksum models, timeouts, warmups, and measurement
limits. Loading fails closed on unknown or missing fields, unsupported schema,
duplicate case IDs, escaping workload paths, hash mismatches, and invalid
values. A workload edit therefore requires an explicit manifest hash update
and creates reviewable evidence.

The measurement manifest also freezes a protocol ID, canonical repository-
relative file IDs, and an aggregate SHA-256 over only measurement semantics:
the workload, adapters, process containment, shared result records, runner,
schema, CLI, and shell entrypoint. Reporting and statistics are deliberately
excluded. `benchmarks/analysis.json` independently freezes analysis schema,
bootstrap, linearity, and run-health policy, compatible measurement
schema/protocol, and an
analysis-only protocol inventory/hash. Analysis changes can therefore re-read
old raw evidence without changing or invalidating its measurement identity.
The measurement manifest separately pins the QuickJS-NG identity, source
repository, and full git revision.

The profile only names the comparable platform series. Identity-specific build
recipes freeze every build dimension separately for `qjs-rust` and
`quickjs-ng`: build mode, exact toolchain identifier, target, exact ordered
feature/flag lists, LTO, strip, allocator, and host-feature policy. The current
macOS arm64 recipes are explicit contracts chosen for this series, not values
inferred by the runner. A toolchain or recipe change requires a new series (or
an explicit reviewed manifest revision), even if the produced binary appears
equivalent. The QuickJS-NG recipe explicitly enables `BUILD_QJS_LIBC` because
the shell workload relies on normal qjs host facilities such as `scriptArgs`.

The `core-black-box-v4` series deliberately supersedes v3. It gives the raw
schema the explicit `throughput/wall_ns_per_operation` lane identity and moves
health interpretation to analysis-v2, so v3 evidence cannot be silently read
under the new policy. Measurement and analysis contracts remain independent,
and protocol file identities remain portable. v3 and v4 records are never
pooled. The v4 suite is a first-party subset derived from
the same behavior families as
the repository's historical QuickJS micro probe. Its T016 matrix covers plain
calls, method calls, captured reads, captured writes, and calls with many
locals, alongside property and array reads. It reports deterministic operation
counts and correctness checksums but contains
no clock. Python measures `perf_counter_ns` around a fresh shell process, so the
metric is **amortized black-box throughput**, including startup, parsing,
realm/setup, execution, and shutdown. It is not VM-only execution time.

For each engine/case, the runner measures zero-iteration startup/setup, then
calibrates the iteration count against a safety-adjusted target. The target is
`ceil(max(min_window_ns, median_startup_ns / startup_max_fraction) *
calibration_safety_factor)`. The checked-in hosted manifest sets the explicit
factor to 1.25, leaving enough headroom for a formal block to run up to 20%
shorter than the calibration target before it crosses the evidence boundary.
This comfortably covers the roughly 8% hosted-runner variation that exposed
the previous boundary condition. The factor is schema-bounded to 1 through 4;
both ratio fields allow at most 18 significant digits and 18 decimal places,
and JSON decimals are parsed directly into exact fractions without an
intermediate binary float. Round-up-to-integer nanoseconds makes runner and
raw-evidence replay deterministic.

When a calibration sample is short of the target, the next iteration count is
`ceil(iterations * target_ns / duration_ns)`. Progress is always at least one
iteration, a single step is capped at 16x, and the manifest's `max_iterations`
is the final cap. Runner execution and raw validation call the same integer-only
progression helper, so proportional scaling cannot drift between production
and replay.

The safety factor changes only when calibration stops. Formal measurement
eligibility is unchanged and still requires both conditions:

- the outer window is at least the manifest's 500 ms minimum; and
- median startup/setup is at most 1% of the outer window.

The manifest caps calibration. A measurement that misses either condition is
recorded as `timer_limited`, not promoted to a precise comparison. Failures
(including a process-spawn error bound to its captured stderr), timeouts,
malformed output, operation mismatches, and checksum mismatches are
durable records and make the run non-zero. Output is capped at 64 KiB per
stream. No outlier is automatically deleted.

After successful calibration and warmup, every role/case emits two dedicated
diagnostic samples, `linearity_n` and `linearity_2n`. The runner chooses N so
2N is exact and within `max_iterations`; these samples occur before all formal
measurement blocks and never inherit measurement eligibility. They are not
derived from, or included in, formal block durations. A missing, duplicate,
malformed, or failed diagnostic makes the raw comparison input incomplete.

The legacy `scripts/microbench.sh` uses an internal millisecond `Date.now()`
loop. It remains a useful quick probe, but its quantization and fixed engine
order make it unsuitable for a gate or public performance claim.

## Running

Builds are deliberately outside measurement:

```sh
cargo build --release -p qjs-cli
./scripts/benchmark.sh --dry-run --blocks 3
./scripts/benchmark.sh \
  --candidate target/release/qjs \
  --candidate-receipt /path/to/candidate-receipt.json \
  --base /path/to/base/qjs \
  --base-receipt /path/to/base-receipt.json \
  --quickjs-ng /path/to/quickjs-ng/qjs \
  --quickjs-ng-receipt /path/to/ng-receipt.json \
  --blocks 30 --seed 20250713
./scripts/benchmark-report.sh \
  --analysis-manifest benchmarks/analysis.json \
  --input target/benchmarks/run-YYYYMMDDTHHMMSSZ.jsonl \
  --output target/benchmarks/report.json
```

Use `--case ID` (repeatable) or `--filter TEXT` for a focused run. `--output`
must name a new file; otherwise the runner writes under ignored
`target/benchmarks/`. Candidate/base default to adapter `qjs-rust-raw`
(`--raw FILE ARGS`) and identity `qjs-rust`; the reference defaults to adapter
`qjs-file` (`FILE ARGS`) and identity `quickjs-ng`. `--ROLE-adapter` controls
only argv protocol, while `--ROLE-identity` independently selects a
manifest-known build recipe. The reference role must retain the pinned
QuickJS-NG identity.

Each optional build-receipt sidecar is strict schema version 1 and is bound to
the executable SHA-256. Its recorded `receipt_sha256` is the SHA-256 of
canonical semantic JSON (`sort_keys=true`, compact separators, UTF-8), not the
hash of source-file whitespace. The analyzer recomputes it from the embedded
receipt, so raw evidence cannot claim an unverifiable sidecar-file hash. It
records engine identity; source repo, full lowercase
40-hex revision, and dirty state; profile ID; build mode; toolchain; target;
exact feature and flag arrays; LTO/strip/allocator; and host features. Unknown
or duplicate fields, binary/profile mismatches, and any recipe difference fail closed. A
QuickJS-NG receipt must additionally match the manifest's pinned identity,
repository, and revision. Missing or dirty receipts still permit local
measurement but record `provenance_status=unverified|dirty` and force
`claim_eligible=false`.

Receipt shape (values must describe the actual build):

```json
{
  "schema_version": 1,
  "engine_identity": "qjs-rust",
  "source": {"repo": "https://example/repo.git", "revision": "<40-hex-sha>", "dirty": false},
  "profile_id": "macos-arm64-release-v1",
  "build": {
    "build_mode": "release",
    "toolchain": "rustc 1.95.0 (59807616e 2026-04-14); cargo 1.95.0 (f2d3ce0bd 2026-03-21); LLVM 22.1.2",
    "target": "aarch64-apple-darwin",
    "features": [],
    "flags": [],
    "lto": "off",
    "strip": "none",
    "allocator": "system",
    "host_features": "target-default"
  },
  "binary_sha256": "<64 lowercase hex characters>"
}
```

Every raw sample records the lane identity, manifest and workload hashes,
binary path/hash and
best-effort version probed from a disposable run-private executable copy, build
receipt/hash, complete argv, role, adapter ID,
engine identity, profile,
runner-repository commit/dirty state, host data, UTC start, duration, phase,
block/order, iterations, validated result, exit status, and bounded
stdout/stderr. The runner repository is never presented as an engine's source
revision. Three-role measurements use a
seeded 3x3 Latin-square rotation; two roles alternate; one role is stable.
Every block contains the frozen case set. Missing or unsupported cases are not
silently reduced to a dynamic intersection.

The run header records both the full manifest portfolio and selected cases.
Focused `--case`/`--filter` runs keep the same series identity but set
`portfolio_complete=false` and can never support a claim. Run-end coverage
reports manifest total, selected total, complete cases per role, and their
common complete set. Failed calibration or warmup emits an explicit
`not_run/ineligible` measurement for every affected planned block.

The runner and M3 health report never sign a final performance claim. A sample's
`measurement_eligible=true` means only that this measurement-phase record has
valid output and timing quality for later analysis; diagnostics and `not_run`
records are false. Run start and run end always carry `claim_eligible=false`.
Run end may set `comparison_input_complete=true` only for the exact
candidate/base/QuickJS-NG role triple, the full portfolio, clean recipe-validated
receipts, every eligible block, both linearity points for every role/case, and
an otherwise successful run. A structurally complete run may instead end with
`comparison_input_complete=false` and `status=failed`; every planned
measurement still has an exact durable record. Single- and two-role runs can
never set readiness. M6
must establish fixed-hardware noise controls before a later artifact may issue
any claim.

stdout and stderr are drained concurrently while retaining at most 64 KiB of
raw bytes per stream; overflow is discarded while draining and invalidates the
sample. Invalid UTF-8 is replaced and cropped to the same encoded bound. On
POSIX, each engine runs in its own session and timeout kills the whole process
group; non-POSIX hosts fall back to killing the direct process and should not
be used for claim-grade runs until equivalent containment is implemented.

Before startup calibration, the run copies every engine executable and each
unique workload into a private directory under `target/benchmarks/snapshots/`.
Each copy is hashed while written and must match the already validated binary
or workload SHA-256; a mismatch fails closed. File modes restrict engine copies
to owner read/execute and workload copies to owner read, but these permissions
are not an immutability guarantee for code running as the same user.

Version metadata is bounded and best-effort: timeout, execution failure, or no
recognized output records `null` and does not block measurement. The runner
first probes a disposable executable copy, records its hash before and after
the probe, and only then creates a separately hash-verified measurement copy.
The probe copy is never used by a sample. Thus a version handler that rewrites
its own file cannot alter the later measurement executable; if it changes the
mutable source instead, creation of the measurement copy fails closed. Sample
argv use only the unprobed measurement copy and hash-verified workload copy.
JSONL records the mutable source path, ephemeral probe and measurement paths,
and their bound hashes. The directory is removed when the run
finishes, so recorded snapshot paths are provenance evidence, not durable
artifacts. Current adapters cover self-contained qjs shell executables. A
future engine that needs adjacent libraries, resources, or configuration must
define and verify a bundle receipt/snapshot contract; it may not silently read
mutable neighboring files.

## Analysis and Claims

`scripts/benchmark-report.sh` consumes one raw JSONL file, its measurement
manifest, and a compatible `--analysis-manifest`, then creates a deterministic
`quickjs-benchmark-report` JSON artifact.
It refuses existing output paths and publishes through a same-directory atomic
link only after validation. Structurally invalid input exits non-zero without a
report. The parser rejects duplicate JSON keys, unknown/missing fields, wrong
record order or identity, any role/case/block record intersection smaller than
the frozen physical plan, duplicate records, forged runner states, and
inconsistent setup/iteration contracts. It accepts exact durable
`failed`/`timeout`/`invalid`/`timer_limited`/`not_run` states so reporting can
classify experiment health instead of hiding failed attempts. It never deletes
an outlier, retries a sample, or dynamically intersects cases.

Validation also replays the seeded measurement plan and requires the physical
JSONL order, block/order labels, roles, and manifest case order to match it
exactly. For every role/case it binds the three zero-iteration startup samples,
calibration progression and final iteration count, warmup count/iterations,
N/2N diagnostics, and all formal blocks into one iteration contract. A
successful sample must have integer exit status zero, null error, untruncated
streams, adapter-exact argv, and a stdout result that parses under the same
strict result contract as the runner and matches the recorded fields. The
analyzer independently recomputes every formal sample's minimum window and
median-startup fraction; an `eligible` label is never trusted by itself.

Raw JSONL stores canonical repository-relative measurement protocol file IDs,
not checkout paths. Report input identity contains only SHA-256 and byte
length—never the current input path or filename—so identical evidence bytes
produce identical reports after rename or analysis in another worktree. The
report separately identifies the raw measurement contract and the exact
analysis manifest/protocol used for this interpretation.

Report coverage keeps structural and runner readiness separate:
`physical_plan_complete=true` means every seeded planned record was present;
`comparison_input_complete`, `runner_end_status`, and the embedded runner
coverage are copied only after validator recomputation. A failed but physically
complete run is therefore never presented as a complete comparison.

For each case/block, the three roles form an atomic triple. Any non-eligible
role invalidates that triple, and any bad triple invalidates that block across
every case and role. Raw records remain in the input evidence, not in the
report. The report preserves invalid-trigger summaries, aggregate statistics,
and coverage; statistics use only the resulting shared whole-block set. A
missing planned record is a structural error, never an invalid block.

For a fixed case, the analyzer computes ns/op, pairs candidate and comparator
by shared valid block, and calculates
`log(candidate_ns_per_op / comparator_ns_per_op)`; the case effect is the
median block log effect. A family is the equal-case mean of its fixed case log
effects, exponentiated back to a ratio. The deterministic paired bootstrap
jointly resamples shared block IDs across every fixed case. The independent
analysis manifest freezes 20,000 draws, seed 20250713, 95% confidence, and
linearity bounds 0.85..1.15; cases are never resampled. Case, family, and
overall ratios and confidence intervals are reported separately for
candidate/base and candidate/QuickJS-NG. Every case, family, and overall result
also records the multiplicative relative half-width
`max(upper/estimate - 1, estimate/lower - 1)` over positive values.

Linearity health subtracts the median of three startup samples from both
diagnostic durations, converts each to per-op cost, and reports the normalized
2N/N ratio for every role/case. Non-positive adjusted durations are
`inconclusive`; ratios outside the frozen bounds are `fail`. An executed setup
or linearity failure makes overall health `invalid`; it cannot be relabeled as
an analyzable success. This is experiment health, not a regression gate: M3
reports always carry `claim_eligible=false` even when complete and healthy.

Analysis-v2 freezes 30 initial blocks, 30 extension blocks, 60 maximum blocks,
a 3% critical-family relative-half-width limit, and at most 10% invalid whole
blocks. Thirty blocks therefore permit at most three invalid blocks; sixty
permit at most six, while the first 30 must independently remain within its
three-block budget. A healthy 30-block experiment is `healthy` when every
critical family is within 3% for both candidate/base and candidate/QuickJS-NG;
otherwise it is `extension_required` with exact requested IDs 30 through 59.
A healthy but still-wide 60-block experiment is `inconclusive`. Other block
counts are smoke evidence and always `inconclusive`/non-claim.

The 3% comparison uses a fixed numerical-boundary tolerance (`rel_tol=1e-12`,
`abs_tol=1e-15`) after computing the multiplicative half-width. This prevents
binary floating-point representation of exactly 3% (for example,
`0.030000000000000027`) from spuriously requesting extension; materially wider
intervals remain wide.

`extension_required` does not append to the existing JSONL. Run a new complete
60-block experiment under the same frozen contracts. Safe append/resume
semantics remain a later M6 concern. Within a run, outliers are retained and
retry policy is `never`: the runner neither fills holes nor changes the seeded
order.

## Independent Resource Lanes

`benchmarks/resources.json` measurement-v1 and
`benchmarks/resource-analysis.json` analysis-v1 are independent of the
throughput raw schema and protocol hashes. One resource JSONL run selects
exactly one frozen lane:

- `fresh_process_latency/wall_ns_per_process` records nanoseconds;
- `peak_rss/bytes` records normalized bytes; and
- `binary_size/bytes` records logical executable bytes.

The measurement and analysis inventories bind their own runner, validator,
statistics, CLI, and shell entrypoints. Shared snapshot, adapter, canonical
receipt, planning, and strict result helpers are explicitly listed in the
resource inventory, so changing a local runtime dependency changes the
resource protocol hash. It does not silently alter the reviewed throughput
protocol.

Select a lane with `--lane fresh|rss|size`. A single-role invocation is useful
only as smoke evidence and can never become comparison input. Report-grade
input requires exactly candidate, base, and QuickJS-NG, with clean receipts
bound to all three binary hashes, the resource profile, exact build recipes,
and the pinned reference revision. Evidence and reports always carry
`claim_eligible=false`; no resource performance conclusion or gate is allowed
before M6 fixed-hardware A/A baselines.

```sh
# Plan-only smoke; no engine or submodule is touched.
./scripts/resource-benchmark.sh --lane fresh --dry-run

# Report-grade example. Repeat with --lane rss and --lane size, using a new
# evidence and report path for each lane.
./scripts/resource-benchmark.sh --lane fresh \
  --candidate /path/to/candidate/qjs \
  --candidate-receipt /path/to/candidate-receipt.json \
  --base /path/to/base/qjs \
  --base-receipt /path/to/base-receipt.json \
  --quickjs-ng /path/to/quickjs-ng/qjs \
  --quickjs-ng-receipt /path/to/ng-receipt.json \
  --blocks 30 --seed 20250713 \
  --output target/benchmarks/resource-fresh.jsonl
./scripts/resource-benchmark-report.sh \
  --input target/benchmarks/resource-fresh.jsonl \
  --output target/benchmarks/resource-fresh-report.json
```

Fresh-process latency starts one new direct shell process for every planned
sample and times from immediately before spawn until the direct child is
reaped with `perf_counter_ns`. It has no calibration phase and no benchmark
warmup phase. The OS page cache is allowed to warm naturally across blocks;
this is deliberately **not cold-disk startup**. The fixed one-iteration probe
and checksum are correctness guards, while the metric includes process launch,
runtime initialization, parse/evaluation, and shutdown. It is never collected
in the same execution as RSS.

Peak RSS has a separate POSIX execution path. It starts one new session and
uses one dedicated reaper thread calling `os.wait4(pid, 0)` to reap that exact
direct child and obtain its `rusage`. It never computes a
`RUSAGE_CHILDREN` delta and never calls `Popen.wait`, `poll`, or `communicate`,
which could consume the wait status first. stdout and stderr are drained
concurrently, retained at the encoded 64 KiB bound, and validated against the
fixed workload identity, operation count, and checksum. Timeout kills the
whole process group and still reaps the direct child through `wait4`.

`ru_maxrss` units are part of the profile contract: macOS reports bytes;
Linux reports KiB and is multiplied by 1024. Unknown platforms, mismatched
profile/unit pairs, machine architecture mismatches, and hosts without `wait4`
fail closed. The current profile freezes `darwin` plus `arm64` separately from
the RSS unit. M6 will additionally freeze the concrete hardware model, power
policy, and other fixed-host controls. RSS is explicitly a
**single direct process** metric, not a process-tree aggregate. After the direct
child is reaped, the runner checks the just-created process group; a surviving
descendant is killed and the sample becomes invalid. Claim-grade engines must
not spawn children. This both enforces the metric boundary and prevents a
background child from escaping containment or retaining output pipes.

Binary size performs no engine execution. It measures `stat().st_size` of the
run-private executable snapshot after rechecking that snapshot's SHA-256
against the validated engine hash. It reports logical bytes for the main
executable only: adjacent libraries, resources, filesystem allocation, and
inferred strip state are out of scope. Each role is measured once. A complete
three-role report gives exact candidate/base and candidate/QuickJS-NG ratios;
it does not fabricate bootstrap confidence intervals.

Dynamic resource lanes replay an exact seeded physical plan at 30 blocks, or a
new complete 60-block run when extension is required. Any bad role sample
invalidates the entire lane block. There is no retry, outlier deletion, or
dynamic intersection. Candidate ratios use paired log effects over shared
valid blocks and the independently frozen 20,000-draw shared-block bootstrap.
The same 3% multiplicative-width tolerance and 10% whole-block loss policy as
throughput applies, including the independent first-30 loss budget in a
60-block cohort. Binary size is healthy only when all three exact values are
present under clean verified provenance; otherwise it is invalid.

Resource JSONL validation rejects duplicate/unknown/missing fields, wrong
types, record order, plan identity, argv, units, output/checksum, receipt,
snapshot identity, forged status/error combinations, or recomputed coverage.
Timeout, spawn, nonzero-exit, truncation, descendant, and malformed-output
states remain durable records while the runner completes the physical plan.
Reports distinguish physical-plan completeness, comparison-input readiness,
runner end status, and statistical health. Raw samples remain only in input
evidence; reports retain input SHA-256/byte length, invalid-trigger summaries,
coverage, health, and comparisons. The digest contains no input path, and the
report writer atomically refuses overwrite.

Future gating must use a frozen portfolio and predeclared practical threshold
`delta`: a regression exists only when the lower 95% confidence bound of
candidate/base exceeds `1 + delta`. A “beats QuickJS-NG” claim requires the
upper 95% bound of candidate/NG below `1` for every predeclared critical family.
It also requires the pinned NG SHA, exact profile and platform series, complete
expected-set coverage, and no Test262 conformance regression. Report
measured/common/total counts even when a run is invalid. Timing, peak RSS, and
exact binary bytes remain separate lanes.

## Rust-Native Lifecycle Diagnostics

M4 adds a separate Criterion diagnostic at the engine's natural public Rust
boundaries. It calls only `qjs_parser::parse_script` and
`qjs_runtime::compile_script`; it does not expose or reach into the private VM,
realm, or evaluator. There is no public realm-construction API today, so realm
construction is deliberately not benchmarked. It can be added only when a
natural production public API exists, never through a benchmark-only API.

Two repository-owned, versioned fixtures exercise functions, closures,
properties, arrays, and control flow without claiming to represent an external
suite. The stable Criterion KPI keys are:

- `lifecycle/parse/{small-v1,medium-v1}`, which parses source anew in every
  timed iteration;
- `lifecycle/compile/{small-v1,medium-v1}`, which parses once before timing and
  times only compilation of `&Script`; and
- `lifecycle/parse_and_compile/{small-v1,medium-v1}`, which times both phases
  in each iteration.

Input size is reported separately through Criterion `Throughput::Bytes` (553 B
and 1,644 B for the current fixtures), so non-semantic byte changes do not
silently rename a KPI. Before benchmarking, each fixture must match its frozen
byte length and FNV-1a 64-bit fingerprint. FNV-1a is only a version-drift
sentinel, not a security hash: changing fixture content requires a new `v2`
fixture ID plus new length and fingerprint rather than silently rewriting v1.

Fixture I/O and cloning are outside the timer; `include_str!` embeds inputs,
and `std::hint::black_box` retains inputs and outputs. All three phases use
Criterion `iter_with_large_drop`, so destruction is deferred outside measured
iterations. The combined phase returns both `(Script, Bytecode)`, ensuring
neither output's teardown is included. Normal runs freeze 50 samples, a
two-second warmup, five-second measurement time, 95% confidence, and a 2%
noise threshold. Run the normal diagnostic or a quick smoke from any working
directory with:

```sh
./scripts/lifecycle-bench.sh
./scripts/lifecycle-bench.sh --quick
```

Formal runs use Criterion's standard `target/criterion` artifact directory as
the only long-term output boundary. Quick runs are forced to the isolated
`target/criterion-smoke` home and automatically add `--discard-baseline`, so a
smoke can neither read nor overwrite a formal baseline. The wrapper rejects
options through a fail-closed allowlist: positional filters; `--quick`,
`--list`, `--help`, `--verbose`, `--quiet`, `--noplot`, `--exact`, and
`--ignored`; exact short forms `-v`, `-n`, and `-h`; plus only the equals forms
`--color={auto,always,never}` and `--format={pretty,terse}`. All other long
options, short options, clusters, and future unknown options are rejected, so
sampling, statistics, profiling, plotting-backend, output, and baseline
identity cannot be overridden. Repository tooling does not parse Criterion's
uncommitted internal JSON layout and does not add `cargo-criterion`. These
Rust-native lifecycle measurements are not pooled with the externally timed
candidate/base/QuickJS-NG lanes and cannot support a performance claim or CI
gate before M6 establishes fixed-hardware A/A noise envelopes.

Criterion is a dev/bench-only dependency under its Apache-2.0 OR MIT license.
It is pinned to exactly 0.7.0 because that release supports Rust 1.80 while the
workspace supports Rust 1.85; current Criterion 0.8.2 requires Rust 1.86.
Default features are disabled and only `cargo_bench_support` is enabled, so
Rayon, Plotters, HTML reports, and async integrations are not added. This has
no production runtime or library dependency impact.

## Series Identity and Governance

A comparable series freezes manifest hash, lane identity, corpus/workload hashes, expected
set, family weights and critical set, engine commits/binary hashes, target,
release/LTO/strip/allocator settings, host feature policy, OS/kernel, CPU and
governor/power policy. A profile or hardware change starts a new series. Before
gating, run same-binary A/A shadows to set a noise ceiling. The report now
implements the frozen 30-to-60 and portfolio-whole-block health interpretation,
but does not turn health into a regression or superiority claim.

Hosted PR runners produce visible, informational previews: three-block ratios
for candidate/base and candidate/QuickJS-NG with raw evidence and deterministic
reports. Their variable hardware makes them non-gating and ineligible for a
performance claim. Stable regression evidence still belongs on a fixed
self-hosted sentinel for performance-sensitive PRs and fixed nightly or
release hardware for the full portfolio. A macOS claim needs dedicated Mac
hardware; other hosts are supporting evidence only.

## External Corpus Admission

`benchmarks/external-corpora.json` is the strict, deny-only v1 governance
registry for external candidates. Validate it from any working directory with:

```sh
./scripts/external-corpus-audit.sh
./scripts/external-corpus-audit.sh --require-admitted sunspider-1.0
```

The default command emits a deterministic structural summary. V1 permits only
`blocked` and `excluded`; it has five blocked source-pinned candidates and two
excluded evidence-backed decisions. Octane deliberately has no source pin.
`--require-admitted ID` consults only the default checked-in trust root and
always exits 2. It cannot be combined with `--registry`: a custom registry is
structural audit input and can never authorize a runner. Validation only reads
metadata. It never downloads a corpus, initializes a submodule, or runs a
benchmark.

Real admission is not obtained by filling v1 fields with plausible strings.
It requires a separately reviewed v2 schema plus a content-hashed audit bundle
binding source-pin evidence, a per-file license inventory and NOTICE decision,
a repository-owned adapter, a neutral timing protocol and phase boundary, and
an expected-case manifest with source hashes. Generated/downloaded assets do
not belong in `tests/`, and `third_party/` remains read-only. An upstream
top-level license never substitutes for a per-file inventory.

Admission tiers:

- The current QuickJS-derived first-party subset is the only runnable layer;
  the external registry records governance state, not first-party admission.
- V8 benchmark suite v7 (`bench-v8`) and the QuickJS-NG Web Tooling Benchmark
  fork are blocked candidates pending per-workload license, capability, and
  timing audits. Web Tooling documents `qjs --stack-size 2048 --script
  dist/cli.js`; qjs-rust does not expose those shell flags, so it is not
  runnable under the current neutral adapter.
- SunSpider and Kraken are historical, per-case evidence only. SunSpider is
  the preferred first v2 review because its small shell-oriented
  boundary is clearest, but it remains blocked until the per-file license
  inventory and NOTICE disposition close. The QuickJS-NG
  benchmark repository's runner, including its Node `benchmark.js` path, is
  opponent-owned and cannot be reused as a neutral referee; only an audited
  corpus/phase-boundary port is admissible.
- A future JetStream 3-derived shell subset may use its `cli.js` selection
  mechanism after capability audit. JetStream mixes JS, Wasm, and multiple
  workload classes, so a subset must never be presented as an official score.
- Octane is excluded because its publisher retired it as unrepresentative of
  real-world JavaScript performance. Speedometer is excluded because it measures
  browser end-to-end web-app responsiveness, including DOM and asynchronous
  phases, rather than a pure JavaScript shell. A registry entry or successful
  audit is never headline evidence: only a complete frozen measurement and
  analysis protocol on qualified hardware can support a performance claim.

## CI Layering and Gate Activation

`benchmarks/performance-policy.json` is a fail-closed v2 policy. Validate the
checked-in trust root from any working directory with:

```sh
./scripts/performance-policy-audit.sh
./scripts/performance-policy-audit.sh --require-gate nightly
```

The audit cross-checks all four current measurement/analysis protocol hashes,
the pinned QuickJS-NG repository/revision, and an aggregate hash over the full
hosted control/audit chain: workflow, Rust setup action, preview orchestrator,
renderer, admission/failure-evidence helper, both audit wrappers and both audit
validators, plus the external-corpus registry. It also requires that registry
to remain non-claim and zero-admitted.
It reports `claim_eligible=false`, no fixed hardware, no evidence entries, and
all `nightly`, `release`, and `pr_sentinel` gates disabled. Every
`--require-gate` request therefore exits 2. A custom `--policy` is structural
input only and cannot be combined with `--require-gate`.

`.github/workflows/performance-smoke.yml` declares `pull_request_target` and
`push`, both filtered to `main`. For a same-repository PR targeting `main`, the
base-owned workflow, setup action, and `base_owned_harness` compare the explicit
PR head SHA against the explicit PR base SHA. Fork previews are explicitly
unsupported and skipped. This is an integrity boundary for cooperative
same-repository PRs, not a malicious code sandbox: candidate compilation and
execution share the runner, and a hosted artifact is not designed to resist a
malicious candidate.

Every push to `main` also runs one actual three-engine comparison. A merge
creates that push naturally, so there is no separate merge-event run; a direct
push follows the same path. The pushed `github.event.after` revision owns the
workflow, setup action, and `main_push_head_owned_harness`, and is both harness
and candidate. `github.event.before` is checked out as the base. Executable
admission requires event `push`, ref `refs/heads/main`, matching workflow/event
repository identities, full lowercase before/after/workflow SHAs, a non-zero
before and after, and `github.sha == github.event.after`; unchanged or malformed
identities fail closed. The after-owned harness is necessary because the first
before revision predating this path cannot implement it.

Both paths use read-only contents permission, no secrets, no write permission,
no slowdown threshold, and no performance gate. Harness mode/revision plus the
candidate, base, and pinned QuickJS-NG revisions are recorded in pending,
failure, and successful provenance. PR numbers isolate and supersede stale PR
runs, while each main push gets a distinct workflow-run-bound concurrency group so no
push is canceled by a later one.

`scripts/performance-preview.sh` initializes only the manifest-pinned
QuickJS-NG revision and prepares all three engines on one `ubuntu-latest` host.
The workflow restores compact content-addressed caches containing only final
candidate/base `qjs` and fixed-revision QuickJS-NG executables. Benchmark rows,
receipts, reports, summaries, and conclusions are never cached: every run makes
fresh receipts bound to the current candidate/base revisions and validated
binary digests, then repeats all measurement and evidence generation.

Rust keys bind tracked workspace manifests, the lock/toolchain files, the full
`crates/` tree, hosted image identity, OS release/kernel/libc, actual
Rust/Cargo/compiler/linker paths, versions and executable digests, every
effective build-affecting Cargo/Rust/C/linker environment value, target, and
the exact hosted release recipe. Documentation- and workflow-only edits do
not force compilation. QuickJS-NG keys bind its fixed repository/revision,
OS/architecture, compiler target, actual C/CMake/Make identities, relevant
build environment, and make recipe. Candidate and base share the Rust content
namespace, so identical build inputs reuse one binary across roles/revisions.

Restored executables are untrusted input. Exact-key metadata, input identity,
regular executable mode, size, and SHA-256 must validate before use; missing or
invalid entries rebuild. Logs and artifact `build-cache.json` expose each
role's hit/rebuild status and key. `pull_request_target` uses exact restore-only
actions and cannot save candidate-influenced state. Only a trusted `main` push
may save new immutable executable caches; prefix restores are not used. Before
storing a rebuilt miss locally, the orchestrator revalidates that role's source tree
immediately after compilation, so a build that dirties tracked provenance
cannot create a ready entry. Before a remote save, an always-run step
independently revalidates each atomically stored
entry. Thus a valid build can be reused even when later hosted
measurement/report health fails, while partial or malformed entries are never
saved. Cache backend restore/save errors are non-fatal: they degrade to
rebuild/no-save without changing benchmark failure semantics.

It rejects repository Cargo config overrides, forces the recorded Cargo release
profile and generic CPU flags, and verifies that every source tree remains at
the requested clean revision before and after builds and again after
measurement. Allocator selection for all engines is recorded as
source-controlled and not independently verified. Before candidate build or
execution, the orchestrator removes GitHub command-file, runtime-token, and
OIDC-token environment variables. After measurement it revalidates all three
source trees and reruns both selected-harness audits before summary generation.
These checks limit accidental or cooperative drift; they do not turn candidate
code into an adversarial sandbox. The script generates a Linux-hosted dynamic manifest and three
verified receipts bound to the actual source SHAs, binary hashes, toolchains,
targets, and build flags. It then runs the complete seven-case portfolio for
three blocks, validates raw JSONL, and creates the deterministic report.

The measurement step is bounded below the 45-minute job timeout. A pending
summary/status is written before setup/audit and replaced on success or
failure. Failure status records the active phase, including candidate/base/
QuickJS-NG build, measurement, post-measure validation, and summary. The
always-run publisher creates fallback Markdown and machine-readable status even
for a pre-orchestrator failure; artifact absence is an error. Available
raw/report/manifest/receipt/status evidence is retained for 14 days.

The Step Summary defines ratio as candidate wall ns/op divided by comparator:
above 1 means higher ns/op and below 1 means lower ns/op. It shows both overall
ratios, 95% CIs, direction/percentage, valid blocks, and health. A success
summary is emitted only for the expected non-claim report: overall
`inconclusive`, block health `non_claim`, linearity `pass`, all three blocks
valid, and both candidate comparisons present. A higher ratio never fails the
job; missing, malformed, incomplete, or unhealthy comparison evidence does and
does not receive a ratio conclusion. The output is informational, non-gating,
and not a fixed-hardware claim. The policy freezes the aggregate hosted
implementation hash, direct QuickJS-NG pin, three roles, seven cases, three
blocks, artifact retention, no threshold, no gate, and claim ineligibility.
Any future fixed-hardware claim or gate is scoped to trusted merged commits,
not hosted PR artifacts.

Gate activation remains future work, in this order:

1. Qualify and content-hash a fixed-hardware fingerprint.
2. Produce at least 20 independent same-binary, randomized-order A/A shadow
   reports for nightly/release and at least 30 for a PR sentinel, retaining
   report content hashes.
3. Freeze a noise envelope bound to the current four protocol hashes.
4. Demonstrate and freeze a false-positive budget before any PR sentinel.
5. Review a content-hashed evidence bundle and only then consider enabling
   one gate. A policy field or hosted preview result cannot substitute for that
   evidence.

## Roadmap

- **M2 (complete):** strict complete-block analysis/reporting, the T016
  call/binding matrix, and dedicated N/2N linearity health.
- **M3 (complete):** 30-to-60 bootstrap reporting, portfolio-whole-block
  health, fresh-process latency, direct-child RSS, and binary-size lanes.
- **M4 (complete):** Criterion diagnostics at the public parser/compiler
  boundaries; realm construction remains waiting on a natural public API.
  Private VM internals stay private, and results are diagnostic only.
- **M5 (governance mechanism complete):** strict deny-only registry and
  fail-closed audit command; zero external corpora are admitted. Each future
  admission requires its own reviewed v2 audit bundle.
- **M6 (policy infrastructure ready, calibration incomplete):** establish the
  qualified fixed-hardware fingerprint, A/A shadows, and noise envelopes.
- **M7 (policy infrastructure ready, all gates disabled):** enable conservative
  nightly/release gates, then a self-hosted PR sentinel only after the
  false-positive budget is demonstrated.
