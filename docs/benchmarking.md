# Performance benchmarking reference

What the measurement tools and their artifacts mean. This is reference
only. The procedure for one performance unit, with every command, is in
[performance-workflow.md](performance-workflow.md). Rules that hold across
revisions are in [performance-knowledge.md](performance-knowledge.md).
Dated measurements and the earlier, longer text of this document are in
[the archive](../tasks/archive/benchmarking-history.md).

Inventories, schema versions, protocol hashes and thresholds are defined by
the files under `benchmarks/` and the loaders under `tools/benchmark/`.
This document names those files instead of copying them; when the two
disagree, the files are right.

## Lanes and inventories

| Lane | Defined by | Metric |
| --- | --- | --- |
| Broad | `benchmarks/manifest.json`, workload `benchmarks/workloads/broad-micro.js` | wall ns per declared operation |
| Generic sentinels | `benchmarks/generic-sentinels-manifest.json`, workload `benchmarks/workloads/generic-sentinels.js` | wall ns per declared operation |
| External | `benchmarks/external-preview.json` (sources fetched into a hash-checked cache) | whole-process wall time |
| Resources | `benchmarks/resources.json`, `benchmarks/resource-analysis.json`, workload `benchmarks/workloads/resource-probe.js` | fresh-process latency, peak RSS, binary size |
| Lifecycle | Criterion bench behind `scripts/lifecycle-bench.sh` | parse and compile time at the public Rust API |

Records from different lanes or series are never pooled, and no lane yields
a headline engine score.

**Broad** has 25 cases in eight families (call, binding, property, array,
control, builtin, string, allocation). Each case is a small loop over one
kind of operation. Each manifest entry declares the operations per
iteration and a closed-form checksum. What broad measures depends on which
tier answers each loop, and that changes with the engine, so this document
makes no statement about its tier coverage. The last formal standing is in
[T018](../tasks/T018-broad-performance.md). Read
the [execution counters](#execution-counters) for a case before using it
as evidence about a specializer or as a control for the generic path.

**Generic sentinels** are six cases that withhold the static facts a
specializer needs: recursion, prototype dispatch on a rotating receiver,
callees and closures selected from run-time tables, reads across several
storage shapes, and computed string keys. Each checksum must match the
manifest's closed form and QuickJS-NG's result for the same iteration
count. They are the controls for ordinary calls and property access.

**External** has 45 cases in three suites: SunSpider 1.0, Kraken 1.1 and a
JetStream 3 JavaScript subset. Results are never an official score of any
suite, and an incomplete suite has no suite score. The metric includes
process start, parsing, execution and shutdown.

**Resources** are three independent lanes selected with
`scripts/resource-benchmark.sh --lane fresh|rss|size`. Peak RSS is a
single direct-process value, not a process tree. Binary size is the logical
size of the main executable.

**Lifecycle** times `qjs_parser::parse_script` and
`qjs_runtime::compile_script` on two repository fixtures. It is diagnostic
and is never pooled with the lanes above.

## Engines compared

A full run has three roles: `candidate`, `base` (both qjs-rust) and
`quickjs-ng`. The reference is the revision pinned by `reference_engine` in
`benchmarks/manifest.json` and by `benchmarks/performance-policy.json`. It
is a black-box executable, never a Cargo or FFI dependency.

`build_recipes` in the manifest freezes, per engine identity, the build
mode, toolchain, target, feature and flag lists, LTO, strip, allocator and
host-feature policy. The checked-in recipes describe macOS arm64. A
toolchain or recipe change is a new series.

A build receipt binds one executable's SHA-256 to a clean source revision
and a recipe. `tools/benchmark/local_receipt.py` writes local receipts and
refuses a toolchain that differs from the recipe or a reference checkout
that is not clean at the pin. A run without clean, recipe-matching receipts
still measures but is recorded as unverified and cannot be comparison
input. The loader is `tools/benchmark/receipts.py`.

## How a sample is taken

Internal lanes (broad, sentinels):

- The workload contains no clock. Python brackets one fresh engine process
  with `perf_counter_ns`, so the metric is amortized black-box throughput:
  startup, parsing, setup, execution and shutdown are all inside the
  window.
- Per engine and case, the runner measures zero-iteration startup, then
  calibrates the iteration count so the window meets the case's minimum
  duration and startup stays under the case's allowed fraction. A sample
  that misses either condition is recorded as `timer_limited`, never
  promoted.
- After warmup it takes paired N and 2N linearity diagnostics. Their
  ratio, after subtracting startup, must fall inside the bounds in
  `benchmarks/analysis.json`.
- Formal blocks rotate the three roles in a seeded order. Every block
  contains the whole frozen case set. There is no retry, no outlier
  deletion and no dynamic intersection of cases.
- Each process also records `instructions` and `cycles` where the host
  exposes them per process. On macOS they are read from the exited child
  (`tools/benchmark/counters.py`).
- Engines and workloads run from hash-checked private copies under
  `target/benchmarks/snapshots/`. Each stream is capped at 64 KiB. On
  POSIX a timeout kills the engine's process group.

External cases are whole-process samples of a generated bundle. QuickJS-NG
runs with `--script`. Stdout must match the adapter's exact sentinel
contract. That proves the script reached its end; correctness rests on each
workload's own validation and on Test262.

Analysis (`tools/benchmark/analysis.py`, constants in
`benchmarks/analysis.json`): for each case the effect is the median over
shared valid blocks of the log ratio of candidate to comparator. A family
is the equal-case mean of its case effects. Confidence intervals come from
a paired bootstrap that resamples block IDs. One invalid role sample
invalidates the whole block for every case. A 30-block run whose intervals
are too wide reports `extension_required`; the response is a new complete
60-block run, never an append.

## Entry points

| Command | Does | Writes |
| --- | --- | --- |
| `scripts/perf-compare.sh` | builds candidate and base at commits, writes receipts, runs all three lanes | a sealed bundle directory |
| `scripts/performance-decision.sh` | `queue`, `check-unit`, `validate-unit`, `decide` | queue and decision JSON |
| `scripts/perf-loop.sh` | builds base and working tree, runs the counter screen | stdout only |
| `python3 -m tools.benchmark.screen` | counter A/B of two executables | stdout; optional `--json` |
| `scripts/benchmark.sh` | one internal lane over given executables; builds nothing | raw JSONL |
| `scripts/benchmark-report.sh` | validates one raw JSONL and analyzes it | report JSON |
| `scripts/external-performance-preview.sh` | `audit`, `fetch`, `run`, `report` for the external lane | raw, report, summary |
| `scripts/resource-benchmark.sh`, `scripts/resource-benchmark-report.sh` | one resource lane and its report | raw JSONL, report JSON |
| `scripts/lifecycle-bench.sh` | Criterion lifecycle diagnostic | `target/criterion` |
| `scripts/external-corpus-ab.py` | amplified diagnostic A/B over cached SunSpider and Kraken | stdout only |
| `scripts/microbench.sh` | legacy in-script `Date.now()` probe | stdout only; never evidence |
| `scripts/performance-preview.sh` | hosted preview orchestrator | hosted artifacts |
| `scripts/performance-policy-audit.sh`, `scripts/external-corpus-audit.sh` | validate the policy and the corpus registry | a summary on stdout |

`scripts/benchmark.sh` takes `--manifest` to select the lane (default
broad) and `--case ID` or `--filter TEXT` for a focused run. A focused run
or a run with fewer than three roles is smoke evidence and can never be
comparison input. `--output` is optional: without it the runner writes
`target/benchmarks/run-<UTC timestamp>.jsonl`. In both cases the file is
created exclusively, so an existing path is an error. Report writers and
`performance-decision.sh` also refuse to overwrite.

## Bundle layout

`scripts/perf-compare.sh --output-dir <dir>` produces:

| File | Content |
| --- | --- |
| `manifest.json`, `raw.jsonl`, `report.json` | broad lane: the manifest copy, every sample, the analysis |
| `sentinel-manifest.json`, `sentinel-raw.jsonl`, `sentinel-report.json` | sentinel lane, same shapes |
| `external-manifest.json`, `external-raw.jsonl`, `external-report.json` | external lane |
| `summary.json` | engine identities, `promotion_metric`, `decision_readiness`, and the seal |
| `summary.md` | per-lane, per-case tables for reading |
| `status.json` | the active phase, or the failure and the phase it happened in |

`summary.json` carries a `bundle` object with an experiment id and the
SHA-256 of each of the nine lane files (`tools/benchmark/bundle.py`).
`queue` and `decide` require the report paths to be those files in the
summary's own directory and recompute every hash. The seal prevents
accidental mixing or replacement of files. It is not authentication.
Before trusting a metric, both commands also rebuild each report from its
raw file (`tools/benchmark/performance_evidence.py`).

`decision_readiness` is `ready_for_unit_gates` or `inconclusive`. It is
permission to evaluate a plan's gates, not a result. Every bundle, report
and summary carries `claim_eligible: false`: nothing these tools emit is a
fixed-hardware performance claim.

Raw files hold one JSON record per line; `tools/benchmark/records.py` and
`tools/benchmark/raw_validation.py` define and validate them. Report input
identity is a hash and a byte length, never a path, so the same evidence
bytes give the same report in any checkout. Files under `target/` are
local artifacts and are not retained; regenerate before reuse.

## Unit plans and decisions

A unit plan is `tasks/performance-units/<unit>.json`. Its schema is
`load_unit` in `tools/benchmark/performance_decision.py`, which rejects
unknown or missing keys. The index of existing plans is
[tasks/performance-units/README.md](../tasks/performance-units/README.md).

| Field | Meaning |
| --- | --- |
| `base_sha` | the revision the unit starts from; must equal the queue's candidate |
| `queue` | the queue's candidate revision and the queue file's SHA-256 |
| `priority` | `opportunity_ids`, and either `mode: "queue"` with a `rank_ceiling` or an override with its reason |
| `mechanism` | summary, generality argument and semantic risks |
| `profile_evidence` | per receipt: path under `--profile-root`, SHA-256, covered opportunities, the shared cost named, its inclusive fraction |
| `fast_gate` | `target_ids`, `control_ids`, `target_max_candidate_over_base`, `control_max_candidate_over_base`, `max_attempts` (1 or 2) |
| `promotion_gate` | required completeness of broad, external and Test262 evidence |
| `unit_kind`, `migration` | schema 2 only; see below |

Schema 1 plans have no `unit_kind` and are leaf units. Schema 2 adds
`unit_kind` (`leaf` or `migration`). Both are accepted, so older plans keep
their bytes and their SHA-256 bindings. A migration plan adds `stages` (2
to 12), `current_stage`, `cumulative_target_ids` and
`stage_max_candidate_over_base` (1.0 to 1.25).

The **opportunity queue** (`queue`) lists, per lane, the cases whose
candidate/QuickJS-NG ratio exceeds `--target-ratio`, ranked by that ratio.
It embeds the bundle's engine identities and evidence hashes. It is derived
data and is not stored in the repository.

A **profile receipt** (`tools/benchmark/profile.py`) copies and hashes a
raw profile and its workload and records the sampler, its command and the
sampled binary's SHA-256. Validation requires that hash to equal the
queue candidate's executable hash.

**Decision metric.** `decide` judges cycles when every comparable case in
every lane has cycle evidence, and wall time otherwise, as on hosted Linux
runners. Cycles exclude scheduling and frequency noise but also exclude
time off-CPU. A precise wall-time regression beyond the control ceiling
that cycles do not show therefore makes the decision `inconclusive` and is
listed in `wall_time_divergence`.

A watched comparison is precise when it has at least 30 valid blocks and a
relative half-width of at most 3%. `decide` requires the bundle's base
revision to equal the plan's `base_sha` and its base executable to equal
the queue's candidate executable.

| Mode | Watches | States |
| --- | --- | --- |
| `fast` | the plan's targets and controls plus the six sentinels | `retained`, `rejected`, `inconclusive` |
| `promotion` | the same, plus every broad and external case at the control ceiling, complete QuickJS-NG comparisons, and a zero-gap Test262 burndown for the candidate | `retained`, `rejected`, `inconclusive` |
| `stage` | a migration's cumulative targets and its controls at the stage budget | `advance`, `abort`, `inconclusive` |

- `retained` / `advance`: every watched interval lies wholly inside its
  limit and nothing is missing.
- `rejected` / `abort`: a precise interval lies wholly beyond its limit, or
  in promotion mode the Test262 gap is not zero.
- `inconclusive`: anything else, such as an interval crossing a limit, an
  imprecise comparison or missing evidence.

`fast` and `promotion` are refused for a migration until `current_stage`
equals `stages`. `abort` closes one stage's implementation, not its
mechanism family. The campaign's targets against QuickJS-NG are not
enforced by these tools; they are acceptance criteria in T018.

## Execution counters

Wall time cannot tell an accelerated workload from a folded one. The
`perf-counters` cargo feature (`qjs-cli` forwards it to `qjs-runtime`)
compiles in counters and traces that can. Without the feature every
counting site expands to nothing. A `perf-counters` build adds work to the
paths it observes and is never used for timing.

After evaluation the CLI prints a `QJS_PERF_COUNTERS` line to stderr,
followed by one `name value` line per counter. The counters are the fields
documented in `crates/qjs-runtime/src/diagnostics.rs`. The ones used most:

- `ordinary_call_attempts`, `closed_form_leaf_evaluations`,
  `direct_leaf_frames`, `generic_call_frames`, `native_calls`,
  `nested_vm_constructions`: how calls were really answered. A workload
  that claims N calls must report about N attempts.
- `named_property_reads` / `_writes`, `computed_property_reads` /
  `_writes`: property operations performed.
- `loop_backedges`, `loop_plan_entries`, `declined_loop_plan_edges`:
  backward edges taken, edges where an accelerator ran a region, and edges
  where both accelerators (numeric-mutation, then typed loop) declined.
- `executed_ops` and the `dispatched_*_ops` families that partition it:
  bytecode instructions the interpreter dispatched, which separates "runs
  more instructions" from "runs each instruction more slowly".
- `compact_function_ops`, `compact_standalone_activations`,
  `compact_direct_calls`, `compact_caller_env_calls`: work done by the
  compact tiers.

The same build reads these environment variables. A level is matched
exactly, not as a threshold.

| Variable | Output on stderr |
| --- | --- |
| `QJS_TL_TRACE` (any value) | typed-loop lines: `TLOK` / `TLFAIL` per compiled or failed region, `TLGIVEUP` per instruction a compile pass gave up on, `TLRUN` per entry with its outcome, `TLDEOPT` per deoptimization, `TLENCLOSE` when a region is declined because it encloses a special numeric-mutation plan, `TLCLAIM` per backward edge an accelerator claimed, `TLEDGE` per backward edge both declined |
| `QJS_TL_TRACE=2` | also the bytecode of a region a pass gave up on |
| `QJS_TL_TRACE=3` | also each compiled program as the builder emitted it (`TLOP`) |
| `QJS_TL_TRACE=4` | also the program that runs, after register packing, hoisting and copy forwarding (`TLFINAL`, `TLHOIST`, `TLFOP`) |
| `QJS_TL_NO_FORWARD` (any value) | none; disables copy forwarding (`typed_loop/forward.rs`) so one binary can compare both |
| `QJS_CF_TRACE` (any value) | wide compact tier lines: `CFOK` / `CFDECLINE` per body compiled or declined, `CFVM` per general-path frame built, `CFEXIT` per exit to the interpreter, `CFNATIVE` per loop handed back to the tier, `CFLOOP` per typed-loop program the tier ran |
| `QJS_CF_TRACE=2` | also each compiled body (`CFLIST`) |
| `QJS_CF_TRACE=3` | also the bytecode of each `CFVM` body (`CFVMCODE`) |
| `QJS_IC_TRACE` (any value) | `ICUPDATE` per named-property cache update: key, storage kind, occupied entries |

`python3 -m tools.benchmark.tl_trace --binary <perf-counters qjs> --case
<spec>` summarizes a `QJS_TL_TRACE=1` run as two histograms:
deoptimization sites by frequency, and interpreted regions weighted by
`TLEDGE` count and joined with their last give-up line.
`scripts/perf-loop.sh --trace <spec>` builds the diagnostic binary and
runs it. A histogram of `CFVM` lines ranks the callees that still pay for
a full interpreter frame.

## Pinned code layout

On macOS the `qjs` binary links its functions in the order listed in
`crates/qjs-cli/hot-functions.order` (`crates/qjs-cli/build.rs`; other
hosts ignore the file). The workspace builds with v0 symbol mangling
(`.cargo/config.toml`), whose names carry no signature hash, so the list
keeps naming a function whose signature changes.

`tools/benchmark/order_file.py` generates the list by sampling every case.
`tools/benchmark/layout_pin.py` rewrites its head so that the two
instantiations of the typed-loop executor (the interpreter's and the wide
tier's) start at fixed offsets modulo 4 KiB, with the listed callees after
the wide one in fixed-size slots padded with standard-library filler. The
header comments record the pin and each slot budget. The offsets and callee
lists are constants in `layout_pin.py`.
When to regenerate, re-pin or re-scan is in
[the workflow](performance-workflow.md#re-pinning-the-code-layout).

## Hosted workflows

`.github/workflows/performance-smoke.yml` runs an informational
three-engine preview on a GitHub-hosted Linux runner through
`scripts/performance-preview.sh`:

- on `pull_request_target` for a same-repository pull request to `main`,
  with the base-owned harness comparing the head against the base (fork
  previews are skipped);
- on every push to `main`, comparing the pushed revision against the one
  before it;
- on manual dispatch from `main`, with an optional `base_sha` input.

The script runs one stage per invocation, and the workflow gives each stage
its own job:

1. `build` validates both sources, builds or restores the three executables,
   and records their identity (`tools/benchmark/preview_identity.py`):
   revisions, toolchains, and each executable's SHA-256.
2. The measuring stages run in parallel, one runner each: `external`,
   `sentinel`, and the broad portfolio as `broad-1` and `broad-2`. A lane job
   checks out only the harness revision, downloads the recorded executables,
   refuses any that do not match the record or the event it was started for,
   and measures all three engines on its own runner at three blocks. Every
   case ratio is therefore still a same-host comparison; ratios from
   different lanes come from different hosts and are never combined.

   The broad lane is sharded because nearly all of its time is per-case
   setup. The cases are dealt between the shards in portfolio order
   (`BROAD_SHARDS` in `tools/benchmark/hosted_preview.py`), each shard
   measures its cases under the unchanged protocol, and the publisher joins
   them only when every shard was admitted and together they cover the
   frozen portfolio exactly once. The lane's overall ratio is the geometric
   mean of the case ratios, as it is unsharded; it has no interval, because
   that bootstrap resamples blocks shared by cases measured on one host.
3. The publish job always runs. It reads each stage's `<stage>-status.json`,
   admits a stage's evidence only when that stage succeeded no earlier than
   the build, its evidence parses and renders, and it measured the recorded
   executables; writes the summary and a `status.json` that indexes every
   evidence file by SHA-256; publishes; and only then fails if the broad or
   external lane has no admitted evidence. A sentinel lane that runs out of its 600-second
   measurement deadline is recorded as incomplete and reported, not failed.

It has read-only permissions, no threshold and no gate. A slower result
never fails a job; missing or malformed evidence does. The combined evidence
artifact is retained for 14 days; the per-stage artifacts that carry
executables and evidence between jobs for one day. Three-block hosted
results can show a direction. They cannot retain or reject a unit, and
durations from separate hosted runs are not comparable.

The published summary is one document rendered by
`tools/benchmark/preview_summary.py` from the lanes' validated machine
summaries, written for a reader who wants to know whether the change made
the engine faster or slower:

- It opens with that answer as a sentence, naming the two builds by what the
  run compared ("this commit" and "the commit before it", or "this pull
  request" and "its base"). When the two executables are byte-identical it
  says the engine did not change and that every difference is noise.
- One row per workload group -- each external suite, then the sentinels
  ("interpreter basics"), then the broad lane ("micro-operations") -- with
  both comparisons in words: `1.5% slower`, `1.20× faster`. No cell needs a
  sign convention or a ratio direction to be read.
- A group is called out only beyond `GROUP_NOISE` (2%) and a single test is
  listed only beyond `TEST_NOISE` (7%). Those are what identical builds show
  on the hosted runners; the measurements behind them are in
  [T017](../tasks/T017-performance-benchmark-system.md). They decide wording
  only: nothing in the workflow gates on them.
- A program without a complete comparison is explained, a lane without
  evidence is named with its reason, and the per-test tables, the exact
  ratios, health and provenance are folded away.

`benchmarks/performance-policy.json` is the fail-closed policy for that
path: protocol hashes, the reference pin, the hash of the hosted
implementation files, and the `nightly`, `release` and `pr_sentinel`
gates, all disabled. `scripts/performance-policy-audit.sh` validates it;
`--require-gate <name>` exits 2 while the gate is disabled. A change to a
file listed under `hosted_implementation` requires updating that hash.

### Gate activation

No gate is enabled and no fixed hardware is configured. The prerequisites
are the `activation_prerequisites` of each gate in the policy file, which
`tools/benchmark/performance_policy.py` validates; enabling a gate takes, in
this order:

1. A qualified, content-hashed fixed-hardware fingerprint.
2. Independent same-binary, randomized-order, content-hashed A/A shadow
   reports: at least 20 for `nightly` and `release`, at least 30 for
   `pr_sentinel`.
3. A frozen noise envelope bound to the current protocol hashes.
4. For `pr_sentinel`, a demonstrated and frozen false-positive budget.
5. Review of the content-hashed evidence bundle. A policy field or a hosted
   preview result does not substitute for that evidence.

`benchmarks/external-corpora.json` is a deny-only governance registry: it
admits no external corpus for claims. The external preview executes pinned
sources without changing any registry decision.
`scripts/external-corpus-audit.sh` validates it.

## Measurement artifacts

These are properties of the method, not of a revision.

- **Startup dilution.** When a case does little more work than process
  start, a one-run-per-binary ratio is mostly startup and is pulled toward
  1.0. The internal lanes avoid this by calibrating the window; whole
  process external ratios of short cases do not.
- **Amplification.** `scripts/external-corpus-ab.py BASE CAND` repeats each
  cached SunSpider and Kraken source text until a process runs for
  `--target` seconds, keeping every copy at top level because scope takes
  part in tier selection. It needs `--base-adapter quickjs-ng` when the
  base is QuickJS-NG. It has no receipts and no frozen inventory, so it
  ranks cases and is never quoted as a result. A ratio whose range spans
  1.0 is no evidence.
- **Path sensitivity.** The same script, or byte-identical binaries, can
  time differently from different filesystem paths. Compare only numbers
  taken through the same harness along the same paths.
- **Tier change at small iteration counts.** A loop run for few iterations,
  or with an empty body, can execute on a different tier than the workload
  of interest. Increments between N and 2N on the same body shape measure
  an operation; subtraction from an empty loop measures the tier change.
- **Timer-limited samples.** A case that cannot reach its minimum window
  within its iteration cap is reported, not compared.
- **Diagnostic builds.** Counter and trace output changes timing. Nothing
  measured on a `perf-counters` build is a timing result.
