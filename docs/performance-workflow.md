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

Broad cases are answered by whichever tier admits each loop, and that
changes with the engine. Do not assume which tier a broad case runs on, or
use one as a neutrality control, until
its [execution counters](benchmarking.md#execution-counters) show it runs
the path under test. The six sentinels are mandatory controls for every
unit. External times include startup, parsing and shutdown.

- A **diagnostic screen** compares two executables on hardware counters in
  minutes. It writes no decision artifact and cannot retain or reject a unit.
- A **formal run** is one sealed three-engine bundle from
  `scripts/perf-compare.sh` at 30 or 60 blocks. `queue` and `decide` read
  that bundle.
- A **leaf unit** is one mechanism that must pay for itself within its
  attempt budget. A **migration** (`"unit_kind": "migration"`) has stages
  judged only against a regression budget; the payoff gate applies at the
  last stage.

## Prerequisites

- macOS arm64 with the toolchain pinned in `rust-toolchain.toml`. The
  checked-in recipes in `benchmarks/manifest.json` describe that host, and
  `perf-compare.sh` refuses a `rustc`/`cargo` that differs from them.
- A built reference engine at `third_party/quickjs-ng/build/qjs`, with the
  submodule clean at the pinned revision:

  ```sh
  cmake -S third_party/quickjs-ng -B third_party/quickjs-ng/build \
    -DCMAKE_BUILD_TYPE=Release -DBUILD_QJS_LIBC=ON
  cmake --build third_party/quickjs-ng/build --target qjs
  ```

  The frozen recipe records `-DBUILD_QJS_LIBC=ON`, so pass it; the pinned
  `CMakeLists.txt` defines no option of that name.
- A committed candidate. `perf-compare.sh` builds candidate and base from
  commits in clean worktrees and caches each executable as
  `target/perf-loop/rev-<sha>/qjs`. Keep that cache between the queue run
  and the decision run: `decide` requires the decision's base executable to
  be byte-identical to the queue's candidate.
- Serialized timing. `compare`, `screen` and `front_end` hold
  `target/.perf-measure.lock`; `compare` (up to 300 s) and `screen`
  (`--settle`, default 30 s) then wait for running `cargo`, `rustc`,
  linker, C compiler and `qjs` processes and warn if any remain. The lock
  is per checkout: measurements from different worktrees are not queued
  against each other, so run them one at a time.

## 1. Regenerate the opportunity queue

A queue from an older revision does not establish priority. Build one from
a formal bundle whose candidate is the revision you start from:

```sh
./scripts/perf-compare.sh --base <earlier-commit> --candidate <start-sha> \
  --blocks 30 --output-dir target/comparison/<run>
./scripts/performance-decision.sh queue \
  --summary target/comparison/<run>/summary.json \
  --broad-report target/comparison/<run>/report.json \
  --sentinel-report target/comparison/<run>/sentinel-report.json \
  --external-report target/comparison/<run>/external-report.json \
  --output target/comparison/<run>-queue.json
```

If no bundle exists for the starting revision, run the comparison first;
`--base` is then any earlier commit, normally the previous formal base.
The output directory and queue file must be new, and the reports must be
the files in the summary's own sealed directory. The queue ranks, per lane,
cases whose candidate/QuickJS-NG ratio exceeds `--target-ratio` (default
0.5). It names workloads to profile, not tactics.

## 2. Profile, then freeze the plan

Profile the queue's candidate executable (`target/perf-loop/rev-<sha>/qjs`)
on a ranked workload (`python3 -m tools.benchmark.bundles` writes each
external case to `target/bundles/<suite>--<case>.js` as a run executes it),
then import the sampler output into a receipt:

```sh
python3 -m tools.benchmark.profile \
  --input /path/to/case.sample --workload /path/to/case.js \
  --binary target/perf-loop/rev-<sha>/qjs \
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

`./scripts/perf-loop.sh --plan tasks/performance-units/<unit>.json` builds
and caches the plan's base, builds the working tree, screens the plan's
targets, controls and the six sentinels, and prints PASS or FAIL with the
largest function-size changes. `--trace <case>` adds the typed-loop trace
histograms from a `perf-counters` build, `--case <spec>` adds cases, and
`--base <ref>` instead of `--plan` screens without a verdict.

### Screen gate

Cases and thresholds come from the frozen plan's `fast_gate`, never from
screen results. The screen judges cycles. It passes when every target's
median ratio is at most `target_max_candidate_over_base` with every pair
below 1.0, and every control's and sentinel's median is at most
`control_max_candidate_over_base`. Anything else fails.

Each implementation attempt gets one screen, and a failed screen consumes
one of the plan's `fast_gate.max_attempts` (the validator accepts 1 or 2).
No tool counts attempts: record every screen result as one line in the
unit's task file. When the budget is spent the mechanism is closed; new work
needs a new profile and plan. Only a passing screen spends a formal run.

An instruction ratio that moves opposite to cycles usually means a layout or
inlining change: check the function-size list and re-pin the layout (below).

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
  --test262-burndown /path/to/exact-candidate-burndown.json \
  --require-retained --output target/comparison/<run2>-decision.json
```

The base is the plan's frozen `base_sha`, not the commit preceding the
candidate. `--mode fast` checks the declared targets and controls plus the
six sentinels. `--mode promotion` also holds every broad and external case
to the control ceiling and needs complete QuickJS-NG comparisons and the
candidate's zero-gap Test262 burndown. Do not rerun until a favorable
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
unit alone. Units whose targets overlap go in separate batches.

### Migration stages

Keep `base_sha` fixed, so every stage is measured against the migration
base. Advance `migration.current_stage` in the plan (the one permitted edit
to a committed plan) and judge with `decide --mode stage` and the arguments
above. `advance` permits the next stage and is not a performance claim.
`abort` closes that stage's implementation shape, not the mechanism family;
record which, in those words. `--mode fast` and `--mode promotion` are
refused until `current_stage == stages`.

## Diagnostic helpers

- `python3 -m tools.benchmark.screen --candidate B --base A --case <spec>`:
  counter A/B. `--case` is repeatable and accepts `sentinel`, `broad`,
  `external`, `sentinel/<id>`, `broad/<id>`, `external/<suite>/<case>` and
  `file:<path>` (default: the six sentinels). `--aa` (without `--base`)
  screens the candidate against itself for the host's noise floor.
- `python3 -m tools.benchmark.front_end --binary B --reference
  third_party/quickjs-ng/build/qjs`: lexing, parsing and compiling cost
  alone; `--base A` compares two builds.
- `./scripts/external-corpus-ab.py A B --reps 3`: amplified ranking of
  short cases ([benchmarking.md](benchmarking.md#measurement-artifacts)).
- Execution counters and the `QJS_TL_TRACE` / `QJS_CF_TRACE` traces on a
  `perf-counters` build ([benchmarking.md](benchmarking.md#execution-counters)).

## Re-pinning the code layout

`crates/qjs-cli/hot-functions.order` pins the hot functions' link order and
the typed-loop executor's address. Re-pin in the commit that adds a hot
function or changes the executor, a pinned callee or a pinned function's
size, so every formal candidate carries a current file. Repeat the last two
commands until the file stops changing:

```sh
cargo build --release -p qjs-cli
python3 -m tools.benchmark.layout_pin --binary target/release/qjs
cargo build --release -p qjs-cli
```

When the hot set changes, rebuild the ranking with
`python3 -m tools.benchmark.order_file --binary target/release/qjs` (it
samples every case and applies the pin) and relink. Add a hot executor
callee to `CALLEES` in `tools/benchmark/layout_pin.py`.

After editing the executor itself, run
`./scripts/layout-scan.sh --offsets 0x0,0x200,0x400` and adopt the centre
of the fast window by setting `DEFAULT_OFFSET` in `layout_pin.py`. The scan
rewrites `hot-functions.order` and relinks `target/release/qjs` for each
offset, and restores the order file when it exits: do not build, commit or
measure from that checkout while it runs. Its default canaries are the six
sentinels, `ai-astar`, `imaging-desaturate`, `access-nsieve` and
`access-nbody`.
