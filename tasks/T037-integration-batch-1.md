# T037: Integration batch 1

- Status: active. Integration complete; strict placement neutrality and formal
  decisions remain pending. No unit is retained by this integration.
- Implementation: present on `integration/batch-1`, from `e54e33eb`.
  Two screened runtime leaves and docs-only mixed-call evidence are integrated.
  The separate check-plan fix is not merged here.
- Verified at: `7095a4aa`, 2026-10-12, complete check.sh and compare-qjs after
  each merge; `20c3cd22`, staged gate, fixed-point pin and 15-pair checks below.
- Evidence: conclusions, hashes and counts below; raw target/ and /tmp logs
  are local artifacts, not retained. Historical leaf screens are in
  [the archive](archive/T018-batch-1-leaf-screens.md).
- Unresolved: prototype instructions are 0.997522 of the queue base,
  significantly below 1.0; the requested strict equality is not established.
  Prefix/tail isolation did not remove it. No formal bundle or complete
  candidate Test262 parity exists yet.
- Next action: explain the prototype instruction difference, then run the
  ready commands below on a quiet host with the exact candidate burndown.
  Do not treat the passing cycle ceiling as a strict placement acceptance.

## Scope and integration

Base: `e54e33eb9371c5cb4627bd8e590a648085a7fa26`.
Worktree: `../quickjs-rust-worktrees/wt-quickjs-rust-integration-batch-1`.
The main checkout's typed_loop diff was left unchanged; its SHA-256 before
and after is `a9f3e61d1ebfa859912749623bc04d3d249f5dafd7a3fad727cb42fd93d43783`.

| Source tip | Merge | Scope check | Source CI |
| --- | --- | --- | --- |
| class fields `7c15dde5` | `bd592be6` | 7 files, reported paths, PASS | 38220390353 success |
| global sinks `92ee2968` | `2382d9dc` | 16 files, reported paths, PASS | 38237290482 success |
| mixed-call docs `6a7bf946` | `7095a4aa` | 7 docs/tasks files, PASS | 38233484486 success |

Only the three docs/tasks commits of the mixed-call branch were integrated;
no temporary diagnostic runtime changes exist in its diff. Runtime conflicts
were absent. The order-file conflict provisionally kept the global-sinks pin;
`20c3cd22` independently re-pins the merged tree, stable after two relinks.
Frozen plan JSON files were not edited. The live global plan is the repin plan.

## Review findings

No runtime correctness blocker or benchmark-shape recognizer was found.
Global admission uses operations, binding/property/canonical-cell identities;
unknown observers disable deferral, completed writes publish before continuation,
and call-time callee guards reject captureful replacements. Protected slot/cell
ownership remains intact; no name-keyed per-call snapshot was introduced.
Class batching accepts resolved string keys and primitive constant-return
initializers only, ends before observers/private/symbol elements, and checks
receiver/storage/descriptors before mutation. Duplicate keys keep insertion
positions; indexed-key counts and both revisions update. Proxy, Array,
TypedArray, lazy String-index and failing descriptor paths retain sequential
exceptions and prior definitions. Global mirroring is synchronized before any
observer. The conservative Small-storage/all-configurable boundary can miss
opportunities but does not weaken semantics. The quadratic declined-segment
scan was fixed in `cd32549b`; the 512-field regression proves 512 inspections.
I would not approve the pre-fix scan or interpreting either leaf screen as
promotion. Current runtime code has no identified blocker; strict placement
neutrality below remains unapproved. No dependency or public API was added.

## Placement check

| Symbol | Queue base mod 4 KiB | Merged mod 4 KiB |
| --- | ---: | ---: |
| VM executor | 0xc40 | 0xc40 |
| Wide executor | 0x200 | 0x400 |
| call_leaf | 0x580 | 0x580 |

At `20c3cd22`, 15 AB/BA pairs/case, fixed sentinel N of 3844778,
8388608 and 3844778; external cases are whole processes. Equal-length binary
paths, warmup of both executables/case, no quiet-exec wrapper, RUSAGE_INFO_V6
cycles/instructions. The shared lock covered each complete run; no competing
build/engine process was present at the start or before/after any pair.
P-core user-time shares were >=99.999% on the primary run. Medians below:

| Case | Cycles candidate/base | Instructions candidate/base |
| --- | ---: | ---: |
| `external/kraken-1.1/ai-astar` | 1.003438 | 0.999962 |
| `sentinel/prototype_method_call` | 1.016025 | 0.997522 |
| `sentinel/capturing_closure_call` | 1.003885 | 0.999980 |
| `sentinel/heterogeneous_property_read` | 1.006699 | 0.999996 |
| `external/sunspider-1.0/bitops-bitwise-and` | 0.116519 | 0.078169 |
| `external/jetstream3-js-subset/raytrace-public-class-fields` | 0.945835 | 0.929610 |

All four cycle medians and their 95% upper bounds are below 1.03. The three
non-prototype instruction intervals include 1.0. Prototype's instruction
CI is [0.997487, 0.997567], so literal instruction equality fails. The target
raytrace cycle CI [0.940251, 0.952860] also does not prove the formal 0.95 ceiling;
this check is diagnostic, not a decision.

Order isolation: retain final pin head/use baseline ranking tail, then use
baseline pin head re-derived on merged code/use final tail. Both hold the
requested executor/call_leaf offsets. Each variant received 15 AB/BA pairs
on all six cases, not selective reruns; neither removes instruction drift.

| Prototype order variant | Cycles/base | Instructions/base |
| --- | ---: | ---: |
| `head-final-tail-base` | 1.012752 | 0.997476 |
| `head-base-tail-final` | 1.020169 | 0.997509 |

The original final order/binary was restored byte-identically after isolation.
The wide executor and call_leaf sizes differ from base (12144->12656 and
2784->2880 bytes); the other measured pinned callees have equal sizes. This
bounds the investigation; it does not attribute the instruction delta to a
specific operation or claim the codegen band is the whole cause.
The durable rule lives in [performance knowledge](../docs/performance-knowledge.md#codegen).

Primary binary SHA-256: base
`6b40aa1944af1096657641cab5b2f80319cc35c3765a79da169cb5c698af09c4`, candidate
`6878520ff5fc8fc480e9fd8086c2b5caeb4ba609236ea423cd5a6d573a6fc54b`.
Local commands: `lockf /tmp/qjs-lead/host-timing.lock env PYTHONPATH=.
PYTHONDONTWRITEBYTECODE=1 python3 /tmp/qjs-lead/batch1-placement.py merged`;
`lockf /tmp/qjs-lead/host-timing.lock bash /tmp/qjs-lead/batch1-bisect.sh`.
Raw records include argv, counts, pair order, hashes and bootstrap intervals.

## Gates and separate script fix

All three merges passed staged gates, check.sh and 220 QuickJS-NG fixtures.
Runtime tests: class merge 2236; global and docs merges 2246 each. Every full
gate passed 5170 Test262 cases, 23 perf-counter diagnostics, 21 agents tests,
375 benchmark-tool tests and 35 script tests. Focused slices: class 52,
global 65; docs-only merge selects none. Re-pin staged gate passed 13 CLI tests.
An initial fast gate raced local submodule checkout (`error: Test262 allowlist
validation failed`); populated checkouts and a complete rerun resolved it.

Separate branch `agent/check-plan-validation/codex`, one commit `578a8f6b`:
explicit return propagates each plan's error from the conditional stage;
Bash glob enumeration avoids a failed rg process substitution. Three tests
prove first failure exits 17/stops, last failure exits 17, all-valid exits 0;
they run with PATH=/usr/bin:/bin. Before fix the first failure returned 0.
Initial CI 38238700110 exposed `scripts/check.sh: line 63: rg: command not found`
and still printed `check: ok performance unit plans`; its new tests failed
with FileNotFoundError. The corrected one-commit branch passed local full gate
(2218 runtime, 5170 Test262, 375 benchmark-tool, 38 script tests), scope check
and CI 38239774599. It is pushed and intentionally separate from this batch.
All 87 distinct frozen plans at current e54e33eb-descended local branch tips
were checked individually; none fail. Base alone has 82; this batch has 85.
No frozen plans were changed. core.bare remains false; user identity unchanged.
Remaining conditional-stage helper error propagation is a separate audit;
no claim is made that this one fix audits every function in check.sh.

## Ready formal commands

Not executed: 30-block perf-compare, promotion decisions, full portfolio gates
and complete candidate Test262 parity. Concurrent host work excludes a formal
run now. Resolve strict neutrality above first. Fetch the complete, zero-gap
burndown for the exact candidate to target/batch1-burndown/burndown.json.
The worktree's target/perf-loop/rev-e54e33eb9371c5cb4627bd8e590a648085a7fa26
is a symlink to the same directory in the untouched main checkout; its qjs
hash was verified against the original queue bundle's candidate hash.
Both live plans passed validate-unit with the queue and profile roots below.

```sh
cd /Users/qingyingliu/Code/js-engine/quickjs-rust-worktrees/wt-quickjs-rust-integration-batch-1
task_main=/Users/qingyingliu/Code/js-engine/quickjs-rust
task_candidate=$(git rev-parse HEAD)
task_run=target/comparison/batch1-formal-$task_candidate
task_queue=$task_main/target/comparison/brief01-e54e33eb-20261011-queue.json
lockf /tmp/qjs-lead/host-timing.lock ./scripts/perf-compare.sh \
  --base e54e33eb9371c5cb4627bd8e590a648085a7fa26 --candidate "$task_candidate" \
  --blocks 30 --output-dir "$task_run"
for task_unit in global-numeric-binding-sinks-repin class-field-batch-install; do
  case "$task_unit" in
    global-*) task_profile=/Users/qingyingliu/Code/js-engine/quickjs-rust-worktrees/wt-quickjs-rust-global-numeric-binding-sinks-codex/target/profiles/global-numeric-binding-sinks-repin ;;
    class-*) task_profile=/Users/qingyingliu/Code/js-engine/quickjs-rust-worktrees/wt-quickjs-rust-class-field-batch-install-codex/target/class-field-batch-install ;;
  esac
  ./scripts/performance-decision.sh decide --mode promotion \
    --unit "tasks/performance-units/$task_unit.json" --queue "$task_queue" \
    --profile-root "$task_profile" --summary "$task_run/summary.json" \
    --broad-report "$task_run/report.json" --sentinel-report "$task_run/sentinel-report.json" \
    --external-report "$task_run/external-report.json" \
    --test262-burndown target/batch1-burndown/burndown.json \
    --require-retained --output "$task_run-$task_unit-decision.json"
done
```
