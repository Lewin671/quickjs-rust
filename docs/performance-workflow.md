# Performance workflow

How to select, measure and decide one performance unit. This is the only
procedural document for performance work. Tool and artifact meanings are in
[benchmarking.md](benchmarking.md), rules that hold across revisions in
[performance-knowledge.md](performance-knowledge.md), campaign targets in
T018 and T022. Priority comes from an opportunity queue that is generated
from a fresh formal run and is not stored in the repository (step 1).

## Terms
| Lane | Inventory | What a run measures |
| --- | --- | --- |
| Broad | 25 cases of `benchmarks/workloads/broad-micro.js`, listed in `benchmarks/manifest.json` | ns per declared operation of small single-operation loops |
| Sentinel | 6 cases, `benchmarks/generic-sentinels-manifest.json` | ns per operation of calls and property access whose callee, receiver or key varies at run time |
| External | 45 cases in 3 suites, `benchmarks/external-preview.json` | whole-process wall time of pinned SunSpider, Kraken and JetStream 3 subset scripts |
| Held-out | 9 cases, `benchmarks/held-out-preview.json` | whole-process wall time of pinned third-party npm libraries that never select work |
| Development | 10 cases, `benchmarks/development-preview.json` | whole-process wall time of pinned third-party npm libraries that may select work |

Broad cases are answered by whichever tier admits each loop, and that
changes with the engine: do not assume a tier until its
[execution counters](benchmarking.md#execution-counters) show it runs the
path under test. The six sentinels are mandatory controls for every unit.
External and both npm lanes' times include startup, parsing and shutdown;
held-out is acceptance only, while development cases may be profiled and ranked.

- A **diagnostic screen** reports dual metrics and a screening verdict;
  retaining a unit still requires a complete formal decision.
- A **formal run** is one sealed three-engine bundle from `scripts/perf-compare.sh`
  at 30 or 60 blocks; `queue` and `decide` read that bundle.
- A **leaf unit** is one mechanism that must pay for itself within its attempt
  budget. A **migration** (`"unit_kind": "migration"`) has stages judged only
  against a regression budget; the payoff gate applies at the last stage.

## Prerequisites

- macOS with the toolchain pinned in `rust-toolchain.toml`; `perf-compare.sh`
  refuses a `rustc`/`cargo` that differs from the recipe. The checked-in
  recipes in `benchmarks/manifest.json` describe an arm64 host; on another
  platform `perf-compare.sh` measures with copies that name the live host's
  profile, targets and reference compiler (`tools/benchmark/host_manifest.py`).
  Cases, policy and protocol are unchanged. Bundles from different hosts are
  never compared: a queue, its plans and their decisions all come from one host.
- Python 3.10 or newer as `python3`.
- A built reference engine at `third_party/quickjs-ng/build/qjs`, with the
  submodule clean at the pinned revision:

  ```sh
  cmake -S third_party/quickjs-ng -B third_party/quickjs-ng/build \
    -DCMAKE_BUILD_TYPE=Release -DBUILD_QJS_LIBC=ON
  cmake --build third_party/quickjs-ng/build --target qjs_exe
  ```

  The frozen recipe records `-DBUILD_QJS_LIBC=ON`, so pass it; the pinned
  `CMakeLists.txt` defines no option of that name.
- A committed candidate. `perf-compare.sh` builds candidate and base from
  commits in clean worktrees and caches each executable as
  `target/perf-loop/compiler-fat16-abort-v1/rev-<sha>/qjs`. Keep that cache between the queue run
  and the decision run: `decide` requires the decision's base executable to
  be byte-identical to the queue's candidate.
- Serialized timing. `compare`, `screen` and `front_end` hold
  `target/.perf-measure.lock`; `compare` (up to 300 s) and `screen`
  (`--settle`, default 30 s) then wait for running `cargo`, `rustc`,
  linker, C compiler and `qjs` processes and warn if any remain. The lock
  is per checkout: measurements from different worktrees are not queued
  against each other, so run them one at a time.
- Lanes side by side, where qualified. `perf-compare.sh --concurrent-lanes`
  measures broad, sentinel and external at once, one process each; the summary
  says so in `lane_schedule`. It still holds the lock, and nothing else may
  run. `--workers N` shares the cases of all three lanes among N processes, a
  case's three engines always in the same one (`lane_schedule` is
  `workers-N`). Use either only on a host with an idle core per process whose
  [T017](../tasks/T017-performance-benchmark-system.md) records a passing
  qualification of that schedule; a queue and the decisions made from it
  use one schedule.

## 1. Regenerate the opportunity queue

Build the opportunity queue from a formal bundle at the starting revision:

```sh
./scripts/perf-compare.sh --base <earlier-commit> --candidate <start-sha> \
  --blocks 30 --output-dir target/comparison/<run>
./scripts/performance-decision.sh queue \
  --summary target/comparison/<run>/summary.json \
  --broad-report target/comparison/<run>/report.json \
  --sentinel-report target/comparison/<run>/sentinel-report.json \
  --external-report target/comparison/<run>/external-report.json \
  --development-report target/development/<lane-run>/development-report.json \
  --output target/comparison/<run>-queue.json
```

If no bundle exists for the starting revision, run the comparison first;
`--base` is then any earlier commit, normally the previous formal base.
Use new output paths and reports from the summary's sealed directory. The queue
ranks, per lane, cases whose candidate/QuickJS-NG ratio exceeds `--target-ratio`
(default 0.5) and names workloads to profile, not tactics; a
`--development-report` from the same host and executables, beside its own raw and
manifest files, adds the development channel.

## 2. Profile, then freeze the plan

Profile the queue's candidate executable (`target/perf-loop/compiler-fat16-abort-v1/rev-<sha>/qjs`)
on a ranked workload (`python3 -m tools.benchmark.bundles` writes each
external case to `target/bundles/<suite>--<case>.js` as a run executes it),
then import the sampler output into a receipt:

```sh
python3 -m tools.benchmark.profile \
  --input /path/to/case.sample --workload /path/to/case.js \
  --binary target/perf-loop/compiler-fat16-abort-v1/rev-<sha>/qjs \
  --base-sha <full-queue-candidate-sha> \
  --tool 'sample <version>' --opportunity-id external/sunspider-1.0/3d-cube \
  --command-json '["sample","<pid>","10","-file","/path/to/case.sample"]' \
  --output-dir target/profiles/<run>/<case>
```

A profile of an instrumented build is supplemental; the receipt must
describe the exact queue candidate binary. Write
`tasks/performance-units/<unit>.json`
([field reference](benchmarking.md#unit-plans-and-decisions)) with
`base_sha` equal to the queue's candidate, the queue file's SHA-256, each
receipt's path relative to `--profile-root` and its SHA-256, the target and
control cases, both thresholds and `max_attempts`. Validate before writing
runtime code, commit the plan, and add its row to
[the plan index](../tasks/performance-units/README.md):

```sh
./scripts/performance-decision.sh validate-unit \
  --unit tasks/performance-units/<unit>.json \
  --queue target/comparison/<run>-queue.json \
  --profile-root target/profiles/<run>
```

`check-unit --unit <plan>` checks structure only, without a queue.

## 3. Implement and screen

New units use schema 3 and freeze `layout_gate`: policy path and SHA-256,
mechanism class, memory-sensitive wall targets, K=6 and measurement budget.
Schema 1/2 plans stay readable for historical decisions, never for a new unit.

```sh
./scripts/perf-loop.sh --plan tasks/performance-units/<unit>.json \
  --output target/layout-attempts/<new-attempt>
```

This requires a committed clean candidate. It builds/audits both revisions'
six variants before timing, screens natural placement, and completes the
remaining five matching placements when the rule requires a panel. Natural
samples are reused unchanged; completion never reruns variant zero. The
output preserves raw samples, binary identities and the verdict. Run this
through the site's serialized timing queue. Legacy `--base`, `--trace` and
extra `--case` remain diagnostic helpers.

### Screen gate

The standing rule is in [T022](../tasks/T022-performance-priority-controller.md#layout-robust-decisions).
The policy's measured bands apply only on its host/toolchain/recipe; a mismatch
requires recalibration. Every calibration/formal A/A automatically follows the
[power/continuation procedure](layout-policy-artifacts.md#precision-power-and-continuation), including diagnostic bands/K=6 at the cap. Every screen
covers all broad, sentinel and external cases, with every non-target a
control. It reports `pass`, `control event`, `wall event`, `reject`, or `incomplete`,
with per-case dual metrics and independent lane/portfolio gates.
An event is incomplete evidence, not a rejection or a passing screen.
For an explicit panel (both output paths must have equal-length names):

```sh
python3 -m tools.benchmark.layout_variants --revision <base> --output target/layout/base000
python3 -m tools.benchmark.layout_variants --revision <candidate> --output target/layout/cand000
python3 -m tools.benchmark.layout_panel \
  --base-matrix target/layout/base000/identity.json \
  --candidate-matrix target/layout/cand000/identity.json --output target/layout/panel
```

Use every offset, never select a winning layout. The builder refuses other
platforms. `--natural` measures only variant zero; `--complete` on that same
output appends variants 1..5 and writes `complete.json`. Keep its original
`panel.json` and raw files. `--held-out --pairs 30` with a fresh output
measures mandatory promotion acceptance after candidate/targets are frozen.
Never select work or change targets from held-out results.
A valid failed screen consumes the plan's attempt. Record attempts/events and
completed panels in its task. Resolve missing panels/instruction alerts within
the frozen budget; no tool counts attempts. Layout collectors retain load-invalid pairs and append
replacements under the [load-only recovery contract](layout-policy-artifacts.md#load-only-pair-recovery).
Cap failures and formal DISTURBED jobs are replaced whole, never pooled.
Only a passing screen spends a formal run.

## 4. Formal run and decision

Commit the candidate, then measure it against the plan's `base_sha` and
evaluate the frozen plan:

```sh
./scripts/perf-compare.sh --base <plan base_sha> --candidate <commit> \
  --blocks 30 --output-dir target/comparison/<run2>
./scripts/performance-decision.sh decide --mode promotion \
  --unit tasks/performance-units/<unit>.json \
  --queue target/comparison/<run>-queue.json \
  --profile-root target/profiles/<run> \
  --summary target/comparison/<run2>/summary.json \
  --broad-report target/comparison/<run2>/report.json \
  --sentinel-report target/comparison/<run2>/sentinel-report.json \
  --external-report target/comparison/<run2>/external-report.json \
  --layout-panel target/layout/panel/panel.json \
  --held-out-panel target/layout/held-out/panel.json \
  --test262-burndown /path/to/exact-candidate-burndown.json \
  --require-retained --output target/comparison/<run2>-decision.json
```

The base is the plan's frozen `base_sha`, not the commit preceding the
candidate. `--mode fast` checks the full policy portfolio. `--mode promotion` additionally
requires held-out evidence, complete QuickJS-NG comparisons and the candidate's
zero-gap Test262 burndown. Panel and held-out raw samples are replayed and
bound to the formal bundle's exact natural binary hashes and source revisions.
The canonical formal bundle keeps all precision/health/linearity requirements;
its raw paired ratios use the same case-median/lane statistic as qualification. Do not rerun until a favorable
interval appears, drop outliers, or pool runs; `--blocks 3` bundles are
diagnostic. Decision states are defined in
[benchmarking.md](benchmarking.md#unit-plans-and-decisions). Record the
result in the owning task and in the plan index. `rejected` is evidence to
keep; `inconclusive` means complete the evidence; `retained` means this
unit passed its gates on this run, not that the engine beats QuickJS-NG.

### Batched promotion

Units that passed their screens and share one `base_sha` may share one
formal run: stack them on one candidate, measure once, and run `decide` per
plan against the same bundle. Every unit must pass its own gates; if any is
`rejected` or `inconclusive`, split the batch and remeasure each remaining
unit alone, after any explicitly authorized [unstable-base re-decision](../tasks/T022-performance-priority-controller.md#unstable-frozen-base-cases-lead-amendment-2026-10-12). Units whose targets overlap go in separate batches.

### Migration stages

Keep `base_sha` fixed, so every stage is measured against the migration
base. Advance `migration.current_stage` in the plan (the one permitted edit
to a committed plan) and judge with `decide --mode stage` and the arguments
above. `advance` permits the next stage and is not a performance claim.
`abort` closes that stage's implementation shape, not the mechanism family;
record which, in those words. `--mode fast` and `--mode promotion` are
refused until `current_stage == stages`.

## Re-pinning the code layout

macOS uses the native order file (`hot-functions.order` on arm64,
`hot-functions-x86_64.order` on x86_64). Build with the canonical recipe,
not plain cargo release; audit actual raw symbols and linked addresses:

```sh
binary="$(python3 -m tools.benchmark.build_cache build-rust \
  --source . --target-dir target/layout-build)"
python3 -m tools.benchmark.layout_pin --binary "$binary" --preserve-offsets
python3 -m tools.benchmark.build_cache build-rust --source . --target-dir target/layout-build
python3 -m tools.benchmark.layout_audit --binary "$binary" --strict
```

Repeat pin/relink until stable; add changed hot callees to `layout_targets.py`.
`layout-scan.sh --offsets 0x0,0x200,0x400` restores the native order on exit;
its canary wins cannot adopt production layout. Keep baseline placements
while comparing implementations; production placement requires the whole
portfolio. Do not build/measure concurrently in that checkout.
