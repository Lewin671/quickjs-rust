# OwnSlot hot answers, hash-map target

- Status: active; diagnostic screen passed, awaiting lead formal batch.
- Implementation: present; predecoded OwnSlot hot answers and four focused tests. Temporary diagnostic hooks are absent.
- Verified at: e54e33eb base / eb80d95d plan plus the candidate diff below, 2026-10-12; full gate, 5170 Test262 subset cases, 220 NG comparisons, seven cache tests and screen attempt 2 passed.
- Evidence: [site diagnosis](archive/e54-named-site-miss-diagnosis.md); candidate checksum, commands and both screen results below. Raw target artifacts are local, not retained.
- Unresolved: no formal promotion evidence; placement/inlining changes still need the lead's batch. The frozen sampler inclusive label is corrected below.
- Next action: lead runs the formal batch for this candidate; B remains held with its second attempt unused.

## Scope and predecessor

Base e54e33eb, branch agent/own-slot-cache-hot-answers-hash-map/codex, isolated
worktree created by the agent script. Reopens own-slot-cache-hot-answers, frozen
in 2e879910 and rejected/reverted in 86e1a462 on its own branch. The lead corrected
its target selection: hash-map is the sole target; cdjs, 3d-raytrace, trees,
ai-astar and the six sentinels are controls. A fresh exact-base sampler receipt
and the opcode-counter diagnosis precede the replacement plan.

Production boundary: named_property_cache.rs, compact_fn/property.rs and their
tests; this plan, task and index rows. Diagnostic-only VM read/call hooks were
restored before this documentation commit. No typed-loop, vm_bindings, vm_ops,
class-field, order-file or layout budget changes. B stays held at 644e1de4 with
one unused attempt; this branch neither imports nor changes B.

## Diagnosis established before the reopen

Hash-map's object methods are already hot: 2,069,997 / 2,070,029 method-site reads.
Its OwnSlot cold answers number 3,510,015. The all-cold method price multiplied
by all method calls overstated its gap share. In trees, both recursive itemCheck
sites miss every time on different receivers despite one identical layout and
an already-cached holder. CDjs combines that receiver-proof issue with 596,719
primitive-string localeCompare reads outside the object hot path, and a VM
cached-prototype path that never remembers its own-miss proof. The full record
ranks every observed read site and lists the resolved-call sites and admissions.

Old A adds ~39 instructions to Exact/literal reads and ~10 to prototype reads;
its cdjs mix explains ~46M of the ~49M added instructions. This is real work,
not a cycle-placement diagnosis. Keep the old three hot-entry dispatch cases
and predecode OwnSlot eligibility in the existing hot-index byte; only a marked
OwnSlot calls the borrowed helper. All cache mutation/probe paths must maintain
the hint; identity, layout and live data-slot guards remain mandatory.

## Frozen replacement plan

[The plan](performance-units/own-slot-cache-hot-answers-hash-map.json) owns gates
and attempts. Targets/controls changed by the lead, never by screen selection.
Fresh exact-base profile plus the new diagnostic record replace the old evidence.
The new metadata hint must be proved by focused negative tests and family pricing
before a five-pair diagnostic screen. No formal run is authorized in this session.

Sampler attribution correction: the frozen plan's 1246/1248 wording labels the
whole compact caller stack as wide. Summing wide call-graph branches gives
1228/1248 (98.40%); flat wide remains 632/1248 (50.64%). Receipt bytes and opcode
counts are unchanged. The frozen plan is not edited; this corrects the label and
fraction in the record, without changing selection, gates or attempt budget.

## Screen attempt 1, 2026-10-12

Predecoded hot-index codes, bounds-miss OwnSlot branch, helper reborrows cache.
Seven focused tests, full check (2222 runtime tests, feature lints/tests, 5170
Test262 subset), 220 NG fixtures passed. Initial clippy failed at two sites with
`error: operator precedence might not be obvious`; parentheses fixed both,
and the final full gate passed. Runtime source was not committed yet.

| Case | Instructions/base | Cycles/base |
| --- | ---: | ---: |
| external/jetstream3-js-subset/hash-map | 0.930543 | 0.951985 |
| external/jetstream3-js-subset/cdjs | 0.997512 | 0.993970 |
| external/sunspider-1.0/3d-raytrace | 0.977213 | 0.989306 |
| external/sunspider-1.0/access-binary-trees | 0.999458 | 1.003304 |
| external/kraken-1.1/ai-astar | 0.999899 | 0.972593 |
| sentinel/recursive_call_tree | 1.000021 | 0.985086 |
| sentinel/prototype_method_call | 1.000031 | 1.002516 |
| sentinel/polymorphic_call_site | 1.000095 | 1.002451 |
| sentinel/capturing_closure_call | 0.999999 | 1.005403 |
| sentinel/heterogeneous_property_read | 1.000003 | 1.001109 |
| sentinel/string_key_map_churn | 0.999929 | 0.996442 |

Diagnostic FAIL, exact output:

```text
target external/jetstream3-js-subset/hash-map: median cycles 0.9520 > 0.95
```

All controls pass both counters. No build warning. Candidate binary SHA-256
6330be0e29c7a41e40eb8f0862bbe13b3b58494d86bc0b50396db92a45370c05.
Offsets/sizes (hex relative to0x100000000 / bytes): wide19070/20048→19080/20000;
VM14070/6656→14070/6672; call_leaf8580/2784 unchanged; probe3f650/3968→3f650/3952;
new helperda610/320. No layout/order budget changes.

Attempt2 removes the release/reborrow on the successful OwnSlot route by moving
the existing Ref guard into the helper. This is per-hit work, not a placement
retry; no control cycle failure occurred. One screen remains.

## Screen attempt 2: PASS, 2026-10-12

Only the successful helper's borrow ownership changed from attempt 1: transfer
its existing Ref guard rather than release and reacquire the cache borrow.
No source-shape recognition, layout order or pin-budget changes. Seven focused
cache tests, full check (2222 runtime tests, feature lints/tests, 5170 Test262
subset cases), and all 220 NG fixtures pass. Both permitted attempts are used;
this passing screen authorizes the lead's formal batch, not promotion.

Candidate executable SHA-256:
`b6d04b595a582b4430f05cf9bc9d832f3ecad843da555ef66284b55fa60b13b6`.
Base executable SHA-256:
`6b40aa1944af1096657641cab5b2f80319cc35c3765a79da169cb5c698af09c4`.

| Case | Instructions/base | Cycles/base |
| --- | ---: | ---: |
| external/jetstream3-js-subset/hash-map | 0.920282 | 0.925080 |
| external/jetstream3-js-subset/cdjs | 0.992937 | 0.979777 |
| external/sunspider-1.0/3d-raytrace | 0.970159 | 0.959206 |
| external/sunspider-1.0/access-binary-trees | 0.995649 | 0.988811 |
| external/kraken-1.1/ai-astar | 0.999803 | 0.994891 |
| sentinel/recursive_call_tree | 1.000120 | 1.002530 |
| sentinel/prototype_method_call | 0.999988 | 0.998308 |
| sentinel/polymorphic_call_site | 0.999995 | 0.995855 |
| sentinel/capturing_closure_call | 1.000119 | 0.998734 |
| sentinel/heterogeneous_property_read | 0.999911 | 0.998419 |
| sentinel/string_key_map_churn | 0.999937 | 0.994933 |

Hash-map cycles range 0.916222–0.941810: every pair improves, median <0.95.
All controls also pass on instructions. Load 5.25→4.19, no competing-build
warning. Diagnostic estimates obtained by multiplying the sealed e54 paired
cycles/NG ratio by this screen median: hash-map 1.2558, cdjs 1.3217, raytrace
1.2959, trees 1.3114. These are cross-run projections, not measured candidate/NG
ratios; no demonstrated combination reaches NG parity yet.

`nm -n -C --defined-only` offsets (relative to 0x100000000) / byte sizes:
wide 19070/20048→19080/20128 (+80); VM 14070/6656→14070/6672 (+16);
call_leaf 8580/2784 unchanged; probe 3f650/3968→3f6e0/3952 (-16);
new borrowed helper da6a0/288. The Ref transfer also changes caller codegen;
do not attribute the entire second-screen delta exclusively to borrow work.
The screen reports native-family inlining differences despite unchanged sources.
No cycle-only control failure occurred, and no placement retry was made.

Before screen 1, matched family pricing showed the rejected A's unnecessary
OwnSlot helper detours were removed. Exact/literal remaining whole-packet
penalties were +12.781/+13.012 instructions, vs old A +39.012/+39.245;
SharedSlot -4.044, prototype hot -0.250, OwnSlot numeric -142.968. Prototype
cold retained +11.849. These are attempt-1 costs, not exclusive helper prices
or attempt-2 measurements; remaining cold work was disclosed before screening.
The cdjs instruction control now improves 0.706%, reversing old A's +0.58%.

Reproduction (from this worktree; wrapper waits for build processes before
execing tools.benchmark.screen; every timing command takes the host lock):

```sh
lockf /tmp/qjs-lead/host-timing.lock env PYTHONPATH=. python3 target/reopen-screen/quiet.py --candidate target/release/qjs --base /Users/qingyingliu/Code/js-engine/quickjs-rust/target/perf-loop/rev-e54e33eb9371c5cb4627bd8e590a648085a7fa26/qjs --plan tasks/performance-units/own-slot-cache-hot-answers-hash-map.json --pairs 5 --settle 60 --max-load 1000 --require-quiet --symbols 12 --json target/reopen-screen/screen-2.json
lockf /tmp/qjs-lead/host-timing.lock cargo test -p qjs-runtime bytecode::named_property_cache::tests
lockf /tmp/qjs-lead/host-timing.lock ./scripts/check.sh
lockf /tmp/qjs-lead/host-timing.lock ./scripts/compare-qjs.sh
```

No formal 30-block comparison, complete Test262 scan or full-burndown record.
The method-miss fixes are proposals only; no diagnostic hook or B code is part
of this candidate. Diagnosis fd29cc1a and frozen plan eb80d95d were pushed before
runtime implementation. Staged fast gate passed, including 65 selected Test262
cases; check-context passed. Remote CI must be green before integration.
