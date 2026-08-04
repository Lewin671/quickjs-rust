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

The current `broad-black-box-v2` series supersedes both `core-black-box-v4` and
`broad-black-box-v1`; records from different series are never pooled. Broad v2
retains the seven historical T016 cases as a trace cohort and the same 18
holdouts that vary loop placement, call arity and expression shape, dynamic
access, writes, allocation, control flow, strings, and builtins. Its 25 critical
cases cover eight families: call (6), binding (5), property (3), array (3),
control (2), builtin (2), string (1), and allocation (3). Tests freeze the exact
case inventory and these family counts so a later optimization cannot silently
narrow the portfolio.

Broad v1's `property_write` and `array_write` holdouts repeatedly overwrote a
fixed set of slots and checked only their final values. A semantics-preserving
loop summary could therefore replace all remaining iterations with the final
state, producing constant-time samples that no longer measured sustained write
throughput. Broad v2 chains each round's property/element values into the next
round and accumulates every round into a triangular checksum. The declared
operation count remains three named-property updates or four dense-element
updates per iteration. This makes a terminal-state-only shortcut fail the
checksum while leaving a genuinely general optimizer free to accelerate the
state recurrence. The protocol ID, workload hash, suite ID, and series ID all
changed so v1 evidence cannot be mistaken for v2 evidence.

### Generic-path sentinels

`benchmarks/generic-sentinels-manifest.json` (series `generic-sentinels-v1`,
workload `benchmarks/workloads/generic-sentinels.js`) is a separate six-case
suite that exists because broad v2's anti-shortcut reasoning does not go far
enough. Broad v2 defeated *terminal-state* shortcuts, but every one of its
cases still names its callee statically and holds its receiver fixed, so a
partial evaluator can fold the measured operation away in full rather than
accelerate it. At `c823043e` the broad portfolio therefore reported a
candidate/QuickJS-NG geometric mean of 0.158 while the external macro suites
reported 1.643 on the same binaries: `plain_function_call` completed
20,000,000 nominal calls in 0.04 s against QuickJS-NG's 0.90 s, and six
different cases all reported exactly 4.3 ns/op. Those are folding artifacts,
not throughput.

Broad v2 is consequently a **specializer coverage suite**. It is still the
right instrument for proving a recognizer did not regress, and it must not be
used as a neutrality control set for units that change the ordinary
interpreter, because its cases do not execute that path.

The sentinels keep the same host contract — closed-form checksums, declared
operation counts, no clock — but withhold the static facts a specializer needs,
using ordinary dynamism rather than artificial barriers:

| Case | Family | Denies the specializer |
| --- | --- | --- |
| `recursive_call_tree` | call | 127 real calls per iteration through a binary recursion whose `value + 1` result can only be established by induction |
| `prototype_method_call` | call | prototype-dispatched callee reading `this.step` on a receiver whose identity rotates through a 64-element pool |
| `polymorphic_call_site` | call | callee identity rotates through a runtime-built table |
| `capturing_closure_call` | call | closures read a captured cell, selected from a runtime-built table |
| `heterogeneous_property_read` | property | three storage shapes hold the three read names at different positions |
| `string_key_map_churn` | property | computed string-key read and write against an object past every small-storage threshold |

Every case is verified two ways: its checksum must match the manifest's
closed form, and it must match QuickJS-NG's checksum for the same iteration
count. If a future optimization genuinely accelerates ordinary calls or
ordinary property access, these cases move — that is what makes them a
holdout rather than a second recognizer target.

The workload reports deterministic operation counts and correctness checksums
but contains no clock. Python measures `perf_counter_ns` around a fresh shell
process, so the metric is **amortized black-box throughput**, including startup,
parsing, realm/setup, execution, and shutdown. It is not VM-only execution time.
The campaign target in `tasks/T018-broad-performance.md` is an overall
candidate/QuickJS-NG geometric-mean wall-ns/op ratio at or below 0.50, with
every critical family at or below 1.00. Those are optimization acceptance
criteria, not an enabled CI gate or a public fixed-hardware claim.

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
`ceil(iterations * target_ns / duration_ns)`. A first sample that reaches the
target is immediately confirmed at the same iteration count before any warmup
or formal block starts. If that confirmation is short, it becomes the next
calibration input instead of being silently treated as a valid measurement.
Progress is always at least one iteration, a single step is capped at 16x, and
the manifest's `max_iterations` is the final cap. Runner execution and raw
validation replay the same integer-only progression and confirmation sequence,
so calibration cannot drift between production and evidence replay.

The safety factor changes only when calibration stops. Formal measurement
eligibility is unchanged and still requires both conditions:

- the outer window is at least that case's versioned `min_window_ms`; and
- median startup/setup is at most that case's versioned
  `startup_max_fraction` of the outer window.

Most cases retain a 500 ms window and 1% startup ceiling. Cases whose exact
checksum bounds cap the safe iteration count use explicit 250-350 ms windows
and 2-7% ceilings; even the widest ceiling still amortizes startup by more
than 14x. `branch_arithmetic` retains a 500 ms minimum but permits a 2% startup
fraction so an occasional process-start outlier does not invalidate an
otherwise 1.4-second measurement at its iteration cap. These are per-case
evidence-capacity settings, not relaxed result, linearity, checksum, or
candidate/base acceptance rules.

The manifest caps calibration. A measurement that misses either condition is
recorded as `timer_limited`, not promoted to a precise comparison. Failures
(including a process-spawn error bound to its captured stderr), timeouts,
malformed output, operation mismatches, and checksum mismatches are
durable records and make the run non-zero. Output is capped at 64 KiB per
stream. No outlier is automatically deleted.

After successful calibration and warmup, every role/case emits eight dedicated
`linearity` diagnostics in the fixed scale order N, 2N, 2N, N, N, 2N, 2N, N.
The runner chooses N so 2N is exact and within `max_iterations`. Four
alternating-order pairs make the reported ratio the median of four paired 2N/N
per-op ratios; the balanced order limits monotonic hosted-runner frequency drift
without conditionally retrying or deleting any observation.
These samples occur before all formal measurement blocks and never inherit
measurement eligibility. They are not derived from, or included in, formal
block durations. A missing, duplicate, reordered, malformed, or failed
diagnostic makes the raw comparison input incomplete.

The legacy `scripts/microbench.sh` uses an internal millisecond `Date.now()`
loop. It remains a useful quick probe, but its quantization and fixed engine
order make it unsuitable for a gate or public performance claim.

## Performance Priority And Decision Gate

Performance evidence answers two different questions: whether a measurement is
valid, and whether the selected work is the best current use of effort. The
existing protocol answers the first. T022 supplies the second without turning
the informational GitHub preview into a fixed-hardware CI claim.

Static architecture/task text is never the source of the current highest-ROI
item. It can describe a completed migration or a still-valid constraint, but a
new optimization begins with the exact parent revision's `summary.json`,
`report.json`, and `external-report.json` artifact bundle:

```sh
./scripts/performance-decision.sh queue \
  --summary /path/to/summary.json \
  --broad-report /path/to/report.json \
  --external-report /path/to/external-report.json \
  --output target/performance-opportunity.json
```

The output contains two ranked lists rather than a fabricated grand score:

- `external` ranks complete candidate/base/QuickJS-NG rows still above the
  campaign target (`0.50x` by default); and
- `broad` ranks broad cases still above that target.

Those lists identify workloads to profile, not implementation tactics. A
highest-ratio external case must first be tied to a shared runtime cost in a
current profile. The desired P0 item is a mechanism evidenced in more than one
independent workload; a one-case leaf path is lower priority unless a written
override explains why it unblocks the P0 work.

Each optimization that claims campaign progress then adds a plan under
`tasks/performance-units/`. The plan freezes its base revision, queue SHA-256,
target/control cases, profile receipts, semantic risks, target improvement,
control regression ceiling, and maximum two attempts. Validate it before code:

```sh
./scripts/performance-decision.sh validate-unit \
  --unit tasks/performance-units/<unit>.json \
  --queue target/performance-opportunity.json
```

After measurement, classify the plan using the exact candidate/base preview
bundle. The fast mode checks only the predeclared targets and controls; the
promotion mode additionally requires all 25 broad cases, complete external
comparisons, and the exact zero-gap Test262 burndown for the candidate commit.

```sh
./scripts/performance-decision.sh decide \
  --mode promotion \
  --unit tasks/performance-units/<unit>.json \
  --queue target/performance-opportunity.json \
  --summary /path/to/summary.json \
  --broad-report /path/to/report.json \
  --external-report /path/to/external-report.json \
  --test262-burndown /path/to/burndown.json \
  --require-retained \
  --output target/performance-decision.json
```

`retained` means the frozen gates passed; `rejected` is valid negative evidence
that closes that mechanism after its attempt budget; `inconclusive` means the
artifact is incomplete/noisy and cannot be counted as progress. Hosted preview
artifacts update the queue and audit a result; same-host evidence remains the
decision-making receipt. Once fixed-hardware or approved same-host promotion
infrastructure exists, make this decision check required in the repository
ruleset rather than relying on convention alone.

### Staged migrations

The gate above is correct for a leaf fast path and wrong for an architectural
change. A leaf unit must pay for itself immediately, and the one-or-two-attempt
budget is what stops a recognizer from being retuned until the noise agrees. An
architectural migration cannot meet that gate at its first commit: its early
stages move real execution onto a new representation before any of it is
faster, so they measure neutral or slightly negative by construction. Judging
them by the leaf gate is what closed the contiguous frame stack, the realm
object arena, transition-shape storage, and default data-property storage —
each after a single implementation, each recorded as "do not retry this
mechanism".

Schema 2 therefore adds `unit_kind`. Plans without it are `leaf` and behave
exactly as before, so existing frozen plans keep their SHA-256 bindings. A
`migration` plan additionally declares its stage budget:

```json
{
  "schema_version": 2,
  "unit_kind": "migration",
  "migration": {
    "stages": 8,
    "current_stage": 1,
    "cumulative_target_ids": ["external/sunspider-1.0/controlflow-recursive"],
    "stage_max_candidate_over_base": 1.10
  }
}
```

`./scripts/performance-decision.sh decide --mode stage` then classifies one
stage as:

- `advance` — every cumulative target and declared control is inside
  `stage_max_candidate_over_base`. The stage has earned the right to continue;
  it has **not** made a performance claim.
- `abort` — a target or control regressed past that budget. This closes that
  stage's implementation shape, explicitly **not** the mechanism family it
  belongs to.
- `inconclusive` — the evidence is missing or incomplete, exactly as elsewhere.

`retained` and `rejected` are unavailable in stage mode, and `--mode fast` or
`--mode promotion` is refused until `current_stage == stages`. The migration as
a whole is judged only at its end state, by the ordinary payoff gate.

A migration keeps **one** `base_sha` across every stage. The decision tool
already requires the preview summary's base to equal the plan's `base_sha`, so
this single rule is what makes every stage measurement cumulative against the
migration base rather than against the scaffolding commit before it. Comparing
a stage to its immediate parent remains useful for diagnosis, but it is not
what the gate reads.

The stage budget is bounded on both sides: below 1.0 it would be a leaf gate
wearing a migration's name, and above 1.25 it would license an unbounded
regression. `stages` is capped at 12, because a program longer than that is not
one reviewable unit of work.

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

Pass `--manifest benchmarks/generic-sentinels-manifest.json` to run the
generic-path sentinels instead of the broad portfolio. The two series are never
pooled: they answer different questions, and their absolute ratios are not
comparable.

The hosted Performance Preview runs **both** lanes. The broad lane keeps its
existing summary, now labelled for what it measures, and the sentinel lane
appends its own section with a geometric mean per comparison and a per-case
table. `preview prepare` admits either frozen inventory and nothing else, so a
preview still cannot quietly measure a narrowed or edited portfolio. The
sentinel lane runs *after* the broad lane's summary is durable and every one of
its steps may fail without costing the preview its conclusion; a failure prints
a section saying no ordinary-interpreter reading was produced rather than
silently omitting one.

Three details are load-bearing rather than incidental. The lane writes its own
receipts: `preview prepare` refuses to overwrite, so reusing the broad lane's
paths would fail every run and reduce the section to "did not complete"
permanently. The lane runs **last** and is **admitted only when both the step and the job
demonstrably have room**. `tools/benchmark/preview_admission.py` compares
elapsed time against each budget (`QJS_PREVIEW_STEP_BUDGET_SECONDS`,
`QJS_PREVIEW_JOB_BUDGET_SECONDS`), subtracts a reserve for the fallback write
and the publisher, and admits only when the tightest remainder covers the
lane's deadline (`QJS_SENTINEL_TIMEOUT_SECONDS`, 600). Refusing prints a
section saying so, with the numbers. The decision lives in Python rather than
in shell arithmetic because it is logic, and logic belongs where it can be
tested — its boundary cases are covered directly.

The job budget matters separately from the step's. A job carries setup, cache,
and publication steps the orchestrator never observes, so its clock can be far
ahead of the step's; step-only arithmetic would admit a lane into a job about
to expire and lose the publisher. The workflow records `QJS_PREVIEW_JOB_STARTED_AT`
as its first step so the orchestrator can see that clock at all.

The **whole** admitted pipeline shares one deadline — preparation, measurement,
reporting, and rendering each run against the remaining time. Wrapping only the
measurement would leave the advertised deadline untrue: a report that started
just inside the limit could still overrun the step.

Two weaker fixes were tried and are recorded because both are tempting.
Bounding the lane alone does not work: the deadline starts only after
compilation and the broad lane, so a slow prefix still lets the step expire
mid-lane, after which the always-run publisher replaces the broad lane's
durable conclusion with a failure notice. Raising the step timeout alone does
not work either — nothing bounds the prefix, so it only moves the cliff. And
consulting the step alone is not enough, because the job has its own deadline
and its own head start. Measuring what is actually left, against every deadline
that can kill the run, is what makes the guarantee hold. Running last is what
makes that arithmetic honest: only the fallback write and the publisher follow,
so the reserve does not have to cover the external corpus work as well.

Isolating the lane in its own non-fatal workflow step would remove the
arithmetic entirely, and is left for a change that can thread the lane's
inputs out of the orchestrator. And the sentinel section enforces the same
health invariants as the broad one: complete comparison input, three roles, the
complete frozen six-case inventory, 3/3 valid non-claim blocks, and agreeing
health/linearity status. It additionally requires **both** comparison maps to
carry exactly the frozen inventory, because `coverage` reports how many cases
were measured and does not prove the maps contain all of them. A report can carry comparisons built from whatever
measurements survived a timer-limited block, so a renderer that read ratios
without those checks would publish a degraded number as ordinary-interpreter
cost -- exactly the class of mistake the sentinels exist to stop.

`benchmarks/performance-policy.json` pins the sentinel manifest and protocol
under `protocols.sentinel_measurement`, declares the hosted portfolio as
`complete-broad-25-case-and-generic-sentinels-6-case`, and requires the
sentinel protocol hash in every gate's activation prerequisites. The
repository cross-check additionally requires the sentinel manifest to name the
same pinned QuickJS-NG as the broad one, and requires the analysis policy to
admit the sentinel series. Without those, a sentinel manifest could name a
different reference revision -- which `prepare` would stamp straight into the
lane's receipts -- and still pass `performance-policy-audit.sh`, because the
protocol aggregate covers the protocol *file list* rather than the manifest's
own fields.

### Execution counters

Wall time cannot tell an accelerated workload from a folded one. The
`perf-counters` cargo feature adds execution counters that can, reporting how
many calls, property operations, and loop-plan probes the engine really
performed:

```sh
cargo build --release -p qjs-cli --features perf-counters \
  --target-dir target/perf-counters
./target/perf-counters/release/qjs benchmarks/workloads/broad-micro.js \
  plain_function_call 100000 2>&1 >/dev/null
```

A counter-enabled build is a **diagnostic build and must never be used for
timing** — the counters add work to the paths they observe. Nothing is
compiled in without the feature: every counting site expands to nothing.

`static_property_name_identity_hits` and
`static_property_name_text_fallbacks` diagnose the compilation-graph property
name path. Equal static ordinary names in one root compilation, including its
nested and capture-recompiled functions, retain one immutable `Rc<str>`.
Small object storage scans those identities before falling back to text. The
fallback is required: eval or `Function` bodies, modules, separately compiled
scripts, dynamic computed keys, host objects, and cross-realm values may hold
equal text under different identities. Count one fallback lookup, rather than
every candidate string it examines, so
`identity_hits / (identity_hits + text_fallbacks)` reports the fraction of
eligible Small-storage resolutions that avoided byte comparison.

This is what the two suites report for a nominal 100,000 iterations:

| Case | Suite | Claims | Real calls | Real property ops | Declined plan edges |
| --- | --- | ---: | ---: | ---: | ---: |
| `plain_function_call` | broad | 100,000 calls | 5 | 8 | 0 |
| `method_call` | broad | 100,000 calls | 5 | 9 | 0 |
| `dynamic_method_call` | broad | 100,000 calls | 5 | 9 | 0 |
| `property_read` | broad | 300,000 reads | 4 | 11 | 0 |
| `property_write` | broad | 300,000 writes | 4 | 15 | 0 |
| `recursive_call_tree` | sentinel | 12,700,000 calls | 12,700,004 | 10 | 100,000 |
| `prototype_method_call` | sentinel | 100,000 calls | 100,069 | 300,136 | 100,063 |
| `heterogeneous_property_read` | sentinel | 300,000 reads | 5 | 400,074 | 100,064 |
| `string_key_map_churn` | sentinel | 200,000 ops | 5 | 304,107 | 102,047 |

The broad portfolio performs five calls where it claims a hundred thousand:
one loop-plan entry runs the whole loop and the callee never executes. That is
the concrete form of the folding problem, and it is why a broad case cannot
serve as a control for generic-path work.

The last column is a second finding the counters make visible. Every case a
loop engine cannot claim pays the full four-engine probe chain on *every*
backward edge and enters none of them — about one declined chain per
iteration across all four sentinel loops. That measurement, not a raw probe
count, is what a loop-dispatch-table change would have to justify itself
against.

`executed_ops` counts the bytecode instructions the interpreter actually
dispatched. It answers a question wall time and per-path counters cannot:
whether a slow workload runs *more* instructions than the reference engine or
merely runs each one more slowly. Divide standard-build wall time by this
count to get nanoseconds per dispatched instruction, and compare that against
the reference engine's total time over the same count:

The `dispatched_*_ops` counters partition `executed_ops` into constant loads,
local and global bindings, named and computed properties, calls/construction,
stack traffic, numeric operations, branches/returns, and the remaining general
opcode path. They are deliberately operation-family counts rather than one
field per opcode: the diagnostic is meant to select a shared handler boundary,
not to turn one benchmark's exact bytecode stream into an optimization rule.
The three `dispatched_*_local_ops` fields split the local-binding family, while
the corresponding `authoritative_*_hits` fields report how often dispatch
completed on the direct slot path instead of calling the dynamic/cell-aware
fallback.

| Case | Ops per iteration | Our ns/op | QuickJS-NG ns/op |
| --- | ---: | ---: | ---: |
| `recursive_call_tree` | 20.6 per call | 15.6 | 2.05 |
| `prototype_method_call` | 28.0 | 13.8 | 2.36 |
| `heterogeneous_property_read` | 28.0 | 9.5 | 2.14 |

Instruction counts are at parity -- `callTree` compiles to 41 opcodes and
dispatches about 20.6 per activation, which is what a comparable stack
codegen emits. The gap is entirely per-instruction cost. Two controls bound
where that cost is *not*: inflating the instruction word from 96 to 168 bytes
costs 2.85% geometric mean and nothing at all on the two call-heavy cases, and
a standalone 104-variant `match` dispatch loop over 96-byte words runs at
1.3-1.4 ns per operation on the same machine. Dispatch and instruction-word
size together account for at most 1.4 ns of a 9.5-15.6 ns budget, so the
remainder is inside the opcode handler bodies.

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
    "flags": ["-Cllvm-args=-align-all-functions=4"],
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
receipts, every eligible block, all eight linearity diagnostics for every role/case, and
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

Linearity health subtracts the median of three startup samples from every
diagnostic duration, converts each paired N/2N observation to a per-op ratio,
and reports the median paired ratio for every role/case. Non-positive adjusted durations are
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

## Diagnostic A/B Over the Cached Corpus

`scripts/external-corpus-ab.py BASE CAND` compares two engine binaries over the
SunSpider and Kraken files already in `target/benchmarks/external-cache/`. It is
a **diagnostic** runner for choosing what to optimize next, not a claim-grade
lane: it has no receipts, no frozen inventory, and no governance. Nothing it
prints may be quoted as a series result.

It exists because the obvious form of that comparison does not work. Process
start is 5.2 ms for QuickJS-NG and 5.4 ms here, while most SunSpider cases do
12-35 ms of work, so a one-run-per-binary ratio is mostly startup:

- Dilution biases every short case toward 1.0. On 2026-08-02 `access-nbody`
  read 1.92 that way and 3.06 amplified.
- What remains is dominated by scheduling. The same case measured 1.92 and
  3.69 against the same two binaries an hour apart.

So the runner repeats each case's source *text* until a process runs for at
least `--target` seconds. Repeating the text keeps every copy at top level,
which is deliberate: scope participates in tier selection, so wrapping the body
in a function would measure a different program. Per-copy time was verified
constant across 1, 4 and 16 copies on `access-nbody`, `crypto-aes`,
`string-base64`, `access-binary-trees`, `string-tagcloud` and `crypto-md5`.

```sh
./scripts/external-corpus-ab.py third_party/quickjs-ng/build/qjs target/release/qjs \
  --reps 3 --label ours/NG
./scripts/external-corpus-ab.py /tmp/base-qjs target/release/qjs --reps 11 --only fannkuch
```

Read a ratio whose `[min, max]` spans 1.0 as no evidence. Three repetitions
ranks cases; take a suspected regression to `--reps 11` before believing it.

Fourteen of the forty cached cases are skipped against QuickJS-NG because
**NG** exits non-zero on them, not this engine.

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

Real claim-grade admission is not obtained by filling v1 fields with plausible strings.
It requires a separately reviewed v2 schema plus a content-hashed audit bundle
binding source-pin evidence, a per-file license inventory and NOTICE decision,
a repository-owned adapter, a neutral timing protocol and phase boundary, and
an expected-case manifest with source hashes. Generated/downloaded assets do
not belong in `tests/`, and `third_party/` remains read-only. An upstream
top-level license never substitutes for a per-file inventory.

Admission and execution tiers:

- The current QuickJS-derived first-party subset remains the only claim-candidate
  runnable layer. The external registry records governance state, not
  first-party admission.
- Every admitted hosted preview mode runs the non-claim external preview
  described
  by `benchmarks/external-preview.json`. It runs 45 explicitly listed cases from
  71 files at three full upstream revisions, verifies every SHA-256 before
  generating temporary bundles, and uploads hashes/results rather than source.
  Same-repository pull requests use the base-owned harness, while trusted
  `main` pushes and manual dispatches use the exact `main` harness revision.
  This execution-only layer does not change any `blocked` registry decision.
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

### External hosted preview

The external preview compares the current qjs-rust candidate, its exact
comparison-base revision, and the pinned QuickJS-NG executable on the same
host. The candidate/base ratio is the authoritative per-change generalization
diagnostic; comparing qjs-rust durations across separate hosted runs is only
diagnostic because runner drift is not controlled across runs. QuickJS-NG runs
with `--script` so historical
implicit globals and top-level script semantics match qjs-rust. Python brackets
each fresh shell process with `perf_counter_ns`; the metric therefore includes
startup, parsing, execution, and shutdown. A seeded three-role Latin-square
rotation gives every engine each order position once across the three
measurement blocks.

External raw sample schema v2 records both `timer_started_ns` and
`timer_finished_ns` from that same monotonic clock. `duration_ns` is required
to equal their difference, and adjacent samples can therefore be audited for
overlap without mixing the monotonic timer with the diagnostic UTC
`started_at` timestamp.

The frozen suite inventory is:

- **SunSpider 1.0:** all 26 upstream cases;
- **Kraken 1.1:** all 14 upstream cases, with each data file bound separately;
- **JetStream 3 JavaScript subset:** `cdjs`, `hash-map`, `gaussian-blur`,
  `stanford-crypto-aes`, and `raytrace-public-class-fields`, each invoking one
  upstream `Benchmark.runIteration` plus its validation hook.

Capability probes run before measurement. Stdout health is byte-exact for each
shell adapter: qjs-rust candidate/base runs must emit the two sentinel lines
produced by the explicit host `print` and raw completion value, while QuickJS-NG
`--script` runs must emit exactly one sentinel line. Missing or additional bytes,
including blank lines, invalidate that role. A failed, invalid, or timed-out
engine remains visible for that frozen case and is excluded only from comparisons
that require that role; the other supported roles are still measured. The report
may show a diagnostic geometric mean over explicitly comparable cases, but an
incomplete suite has no suite score. JetStream output is always named **JetStream 3
JavaScript subset** and never an official JetStream score. All hosted output
keeps `claim_eligible=false`. The GitHub Step Summary shows both the three-suite
overview and every named external case with candidate, base, and QuickJS-NG
median wall time plus candidate/base and candidate/QuickJS-NG ratios. The
internal portfolio is likewise
rendered as a 25-case table instead of only an overall ratio.

Run the same preview locally after building the candidate, base, and reference
shells:

```sh
cargo build --release -p qjs-cli
make -C third_party/quickjs-ng BUILD_QJS_LIBC=y
./scripts/external-performance-preview.sh audit
./scripts/external-performance-preview.sh run \
  --cache-root target/external-preview/corpora \
  --work-root target/external-preview/work \
  --output-dir target/external-preview/evidence \
  --candidate target/release/qjs \
  --base /path/to/base/qjs \
  --quickjs-ng third_party/quickjs-ng/build/qjs
```

The output directory receives `external-raw.jsonl`,
`external-report.json`, `external-summary.md`, and the exact manifest. Corpus
files and generated bundles stay outside that directory; bundles are removed
after the run. Before each engine process starts, the runner flushes the current
suite, case, phase, block, and role to standard error so a long run exposes its
active sample without changing the final machine-readable JSON on standard
output. Evidence files are still published atomically only after the complete
run. Use `--blocks 1 --timeout-seconds 1` only for harness smoke tests, not for a
performance reading.

## CI Layering and Gate Activation

`benchmarks/performance-policy.json` is a fail-closed v2 policy. Validate the
checked-in trust root from any working directory with:

```sh
./scripts/performance-policy-audit.sh
./scripts/performance-policy-audit.sh --require-gate nightly
```

The audit cross-checks all four current measurement/analysis protocol hashes,
the pinned QuickJS-NG repository/revision, and an aggregate hash over the full
hosted control/audit chain: the trusted Cargo codegen config, workflow, Rust
setup action, preview orchestrator, renderer, admission/failure-evidence
helper, both audit wrappers and both audit validators, plus the external-corpus
registry. It also requires that registry to remain non-claim and zero-admitted.
It reports `claim_eligible=false`, no fixed hardware, no evidence entries, and
all `nightly`, `release`, and `pr_sentinel` gates disabled. Every
`--require-gate` request therefore exits 2. A custom `--policy` is structural
input only and cannot be combined with `--require-gate`.

`.github/workflows/performance-smoke.yml` declares `pull_request_target`,
`push`, and `workflow_dispatch`. PR and push events are filtered to `main`; a
manual dispatch is admitted only when the selected ref is exactly
`refs/heads/main`. For a same-repository PR targeting `main`, the
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

A manual run uses the selected `main` revision as the trusted workflow,
candidate, and same-revision base. Its internal candidate/base lane is therefore
an A/A noise observation, while the external phase still compares that exact
qjs-rust revision with the pinned QuickJS-NG executable across JetStream 3,
Kraken, and SunSpider. Dispatch admission requires event `workflow_dispatch`,
ref `refs/heads/main`, matching workflow/event repository identities, and
`github.sha` equal to the selected revision. Trigger it from the Actions page
with **Run workflow** and branch `main`, or with:

```sh
gh workflow run performance-smoke.yml --ref main
```

Before a trusted main push or manual comparison, a separate job prepares the pinned
QuickJS-NG executable cache. It computes the same content-addressed build
identity as the measurement job, restores an exact entry when available, and
builds plus saves the pinned reference only on a miss. The comparison job uses
`always()` on that dependency: a cache-backend or preparation failure never
suppresses the benchmark, because the existing orchestrator can still rebuild
the reference as a fallback. The preparation job never produces or caches
measurements. Every `main` update and every manual dispatch therefore runs a
fresh benchmark even when every executable is reused.

Both paths use read-only contents permission, no secrets, no write permission,
no slowdown threshold, and no performance gate. Harness mode/revision plus the
candidate, base, and pinned QuickJS-NG revisions are recorded in pending,
failure, and successful provenance. PR numbers isolate and supersede stale PR
runs, while each main push or manual dispatch gets a distinct workflow-run-bound
concurrency group so no trusted run is canceled by a later one.

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
may save new immutable executable caches; its reference preparation job saves
QuickJS-NG before measurement, while the measurement job retains a validated
fallback save for any cache miss. Prefix restores are not used. Before
storing a rebuilt miss locally, the orchestrator revalidates that role's source tree
immediately after compilation, so a build that dirties tracked provenance
cannot create a ready entry. Before a remote save, an always-run step
independently revalidates each atomically stored
entry. Thus a valid build can be reused even when later hosted
measurement/report health fails, while partial or malformed entries are never
saved. Cache backend restore/save errors are non-fatal: they degrade to
rebuild/no-save without changing benchmark failure semantics.

It rejects unrecognized repository Cargo config overrides. The only accepted
source-local config is the exact trusted `.cargo/config.toml`, which aligns
functions to 16-byte boundaries; absence remains valid for a transition base.
Hosted builds override source config with the equivalent recorded alignment
flag plus the generic CPU flag, force the recorded Cargo release profile, and
verify that every source tree remains at
the requested clean revision before and after builds and again after
measurement. Allocator selection for all engines is recorded as
source-controlled and not independently verified. Before candidate build or
execution, the orchestrator removes GitHub command-file, runtime-token, and
OIDC-token environment variables. After measurement it revalidates all three
source trees and reruns both selected-harness audits before summary generation.
These checks limit accidental or cooperative drift; they do not turn candidate
code into an adversarial sandbox. The script generates a Linux-hosted dynamic manifest and three
verified receipts bound to the actual source SHAs, binary hashes, toolchains,
targets, and build flags. It then runs the complete 25-case portfolio for
three blocks, validates raw JSONL, and creates the deterministic report.

The first main push that introduces this trusted config is a deliberate
transition: its older base may omit the file, but both candidate and base are
built with the harness-owned encoded alignment flag. That hosted
candidate/base comparison is therefore alignment-normalized A/A evidence, not
evidence for the alignment change itself. Same-repository PRs based on an older
harness cannot introduce the config because the base-owned script rejects it;
land the trust-root transition from `main`, then use later hosted runs normally.

The measurement step is bounded below the 45-minute job timeout. A pending
summary/status is written before setup/audit and replaced on success or
failure. Failure status records the active phase, including candidate/base/
QuickJS-NG build, measurement, post-measure validation, and summary. The
always-run publisher creates fallback Markdown and machine-readable status even
for a pre-orchestrator failure; artifact absence is an error. Available
raw/report/manifest/receipt/status evidence is retained for 14 days.

The Step Summary defines ratio as candidate wall ns/op divided by comparator:
above 1 means higher ns/op and below 1 means lower ns/op. It shows both overall
ratios, 95% CIs, direction/percentage, valid blocks, and health when the
expected non-claim report has overall `inconclusive`, block health `non_claim`,
linearity `pass`, all three blocks valid, and both candidate comparisons
present. If a complete 3/3-block hosted run instead fails linearity, the job
succeeds as explicitly inconclusive, preserves the raw evidence, and emits no
ratio direction. A higher ratio never fails the job; missing, malformed, or
incomplete evidence still does. The output is informational, non-gating, and
not a fixed-hardware claim. The policy freezes the aggregate hosted
implementation hash, direct QuickJS-NG pin, three roles, 25 cases, three
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
