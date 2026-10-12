# Wide method resolution

- Status: blocked.
- Implementation: present; correctness verified, diagnostic cycle gate failed.
- Verified at: e22f8942, 2026-10-12; its unchanged runtime tree passed the full check, 5170 Test262 subset cases, 220 NG fixtures and staged fast gate (65 focused Test262 cases); screen 1 failed. Frozen plan f86b59bc validated before runtime edits.
- Evidence: [pricing record](archive/wide-method-resolution-pricing.md); local raw counters/screens not retained.
- Unresolved: unchanged-instruction control cycle failure (placement); no formal promotion evidence.
- Next action: lead reviews the layout-owner result before authorizing the unused second attempt; exclude this candidate from the current formal batch.

## Scope and selection

[The frozen plan](performance-units/wide-method-resolution.json) owns cases,
thresholds and the two-attempt budget. Base e54e33eb; branch
agent/wide-method-resolution/codex, independent of A. A was rejected with both
attempts spent and runtime reverted, recorded/pushed as 86e1a462 on
agent/own-slot-cache-hot-answers/codex before B's worktree was created.

Allowed: compact_fn/property.rs, wide/activation.rs, activation/exits.rs,
resume code and their focused tests; this task, pricing record, plan/index row.
Forbidden: typed_loop/**, vm_bindings.rs, vm_ops.rs, class-field code, order file
and layout_pin budgets. Other owners' plans and index rows are untouched.

B selects method resolution because the measured corpus-weighted lookup proxy
is larger in every target than native-loop overhead. It removes only the second
hot probe after a miss, retaining all cold ordinary/exotic eligibility and
own/prototype proofs. It does not repeat T033's rejected prototype pre-probe,
inline clone/drop, or frame-stack rewrite. Native-backedge action/view work is
priced separately and is not implemented here. Shared upvalue cells and slots
are unchanged.

All measurements use the host timing lock, reject/discard competing builds,
and use instructions and cycles. No repin is authorized. A control cycle-only
failure must be reported as placement with symbol positions and sizes, and
must not consume an attempt chasing layout.

## Attempt 1: duplicate hot probe removed

Plan f86b59bc was frozen, validated and pushed before runtime edits. The two
wide read arms call an out-of-line hot-miss reader; the original reader still
performs its hot probe for other callers. Both specialize one shared body,
preserving all ordinary/exotic guards, own shadow lookup and prototype proofs.
Native-backedge/action/view code, class-field initializer code and cache internals
are unchanged. No layout-order or budget edit.

Focused property contract tests and the wide getter/Proxy receiver test passed.
`lockf /tmp/qjs-lead/host-timing.lock bash -c './scripts/check.sh ... &&
./scripts/compare-qjs.sh ...'` passed: workspace 2221 runtime tests plus other
crates, default/agents/perf-counters lints and tests, 5170 Test262 subset cases,
benchmark/script/plan/context/size gates, and all 220 NG comparison fixtures.
Checks apply to the uncommitted runtime diff after f86b59bc on 2026-10-12.
The initial focused compile emitted an unused-Result warning in the new test;
set_prototype now uses expect and all lint gates pass without that warning.

`llvm-nm -n -C --defined-only` offsets relative to 0x100000000 / size bytes:
call_leaf 8580/2784 → 8580/2784;
VM 14070/6656 → 14070/6672;
wide 19070/20048 → 19080/20048;
probe 3f650/3968 → 3f5f0/3968;
get_prop_named 32d10/2096 → 32cb0/2096;
new get_prop_named_after_hot_miss 42c370/1440. Thus wide and probe sizes stay
fixed, VM grows 16 bytes through code generation; placement still changes.
Candidate binary SHA-256:
3662d5ae9eaa25a8170dbff2ba4a8dca4d6abe6153e4bcde6edfd85ccb0c7b37.
Base binary is the exact e54 executable sealed by brief01 (hash in pricing).

All screens use this quiet-start wrapper (no priority/QoS changes):

```python
import os,sys
from tools.benchmark.measure_lock import wait_for_builds
while wait_for_builds(60):
    print('still waiting; no screen samples started',flush=True)
os.execv(sys.executable,[sys.executable,'-m','tools.benchmark.screen',*sys.argv[1:]])
```

Exact screen command, from this worktree:

```sh
lockf /tmp/qjs-lead/host-timing.lock env PYTHONPATH=. python3 target/method-pricing/screen-quiet.py --candidate target/release/qjs --base /Users/qingyingliu/Code/js-engine/quickjs-rust/target/perf-loop/rev-e54e33eb9371c5cb4627bd8e590a648085a7fa26/qjs --plan tasks/performance-units/wide-method-resolution.json --pairs 5 --settle 60 --max-load 1000 --require-quiet --symbols 12 --case file:target/method-pricing/lookup-4.js --case file:target/method-pricing/lookup-8.js --case file:target/method-pricing/move-4.js --case file:target/method-pricing/move-8.js --json target/method-pricing/screen-1.json
```

A pre-screen wait for qjs(62212) completed before samples. Full formal comparison,
exact-candidate complete Test262 burndown, and promotion were not run.

### Screen 1 result: blocked on placement; no second attempt spent

| Case | Instructions/base | Cycles/base |
| --- | ---: | ---: |
| external/jetstream3-js-subset/hash-map | 0.985928 | 0.976470 |
| external/jetstream3-js-subset/cdjs | 0.995648 | 0.983764 |
| external/sunspider-1.0/access-binary-trees | 0.996197 | 1.047667 |
| external/jetstream3-js-subset/raytrace-public-class-fields | 0.997158 | 0.992333 |
| external/kraken-1.1/ai-astar | 0.999863 | 1.007864 |
| sentinel/recursive_call_tree | 0.999948 | 1.013861 |
| sentinel/prototype_method_call | 0.999687 | 1.012734 |
| sentinel/polymorphic_call_site | 1.000213 | 1.013103 |
| sentinel/capturing_closure_call | 0.999993 | 0.986079 |
| sentinel/heterogeneous_property_read | 0.999990 | 1.007776 |
| sentinel/string_key_map_churn | 1.000016 | 1.060676 |
| file:target/method-pricing/lookup-4.js | 0.968834 | 0.971437 |
| file:target/method-pricing/lookup-8.js | 0.962433 | 0.967449 |
| file:target/method-pricing/move-4.js | 1.000001 | 0.997343 |
| file:target/method-pricing/move-8.js | 0.999995 | 0.997904 |

Exact diagnostic FAIL output:

```text
target external/jetstream3-js-subset/hash-map: a pair did not improve (max 1.0021)
target external/sunspider-1.0/access-binary-trees: median cycles 1.0477 > 0.9999
target external/sunspider-1.0/access-binary-trees: a pair did not improve (max 1.1975)
control sentinel/string_key_map_churn: median cycles 1.0607 > 1.03
```

String-key churn instructions are 1.0000 while cycles are 1.0607. Per the lead's
rule, report this as placement and do not spend an attempt chasing it. The
symbol offsets/sizes above are the requested evidence; they do not prove which
function caused it. Load average was 83.97→61.44 and cycle spreads are wide;
residency/background effects remain possible. No competing-build warning was
emitted during the screen. No repin or layout budget changes were made.
One of two attempts used; the unused attempt is held for the lead's layout
investigation, not repeated until a favorable draw appears. This unit is not
retained or rejected by a formal decision. The implementation is committed for
review, with the failed diagnostic gate explicit; no formal batch is recommended
at this standing.

Matched lookup4/8 instruction ratios .9688/.9624, matched move controls 1.0000.
Using the baseline raw medians from the pricing record and screen ratios to
estimate the changed 8-minus-4 increment gives 34.899 fewer instructions and
4.803 fewer cycles per lookup; move changes are .0066 instructions / .0514 cycles.
These are estimates across runs, not newly paired absolute per-operation prices.
Subtracting the matched move delta yields about 34.89 instructions / 4.75 cycles
for resolution. This is about 10.3% of the calibrated lookup instruction excess,
not the whole 338.5. This cheap leaf pays at the instruction level in all targets.

Method-only cycle projection (4.75 cycles times the measured method frequency,
subtracted from formal e54 cycles, divided by NG) is hash-map 1.347,
cdjs 1.344, trees 1.323; it excludes other cold named-read reach and layout effects.
No target reaches 1.0. A direct multiplication of the screen median by formal
e54/NG paired cycles instead gives 1.326 / 1.327 / 1.389; the tree value includes
its failed cycle gate and is not a supported payoff claim. The baseline formal
cycle ratios are 1.357461 / 1.348961 / 1.326270 (brief01, NG f7830186).
Instruction reductions are 1.41% / 0.44% / 0.38%; the instruction signal is the
reason to preserve this candidate for review despite the cycle block.

Implementation candidate e22f8942 is pushed; the runtime tree matches the
correctness-checked and screened working tree. Branch scope validation passed
against e54e33eb; core.bare=false and Git author identity remained unchanged.
Plan CI f86b59bc passed; candidate CI is pending at this recording.
