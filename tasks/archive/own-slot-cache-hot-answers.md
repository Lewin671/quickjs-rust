# OwnSlot cache hot answers

- Status: closed-rejected.
- Implementation: reverted; no runtime diff is committed.
- Verified at: e54e33eb plus the two recorded uncommitted attempts, 2026-10-12; five focused tests each, two failed diagnostic screens. Plan commit 2e879910 validated; final documentation checks below.
- Evidence: this record; local artifacts under target/ are not retained.
- Unresolved: cdjs instruction regression; no formal promotion evidence.
- Next action: no attempt remains; exclude A from the formal batch. B starts independently at e54e33eb.

## Goal and scope

Deliver successful OwnSlot reads through the existing guarded hot path.
[The frozen plan](../performance-units/own-slot-cache-hot-answers.json) owns cases,
thresholds and the two-attempt budget. Lead selected this leaf after T036 pricing
on design branch 6a7bf946: hash-map 3,510,015, cdjs 28,022 and raytrace 78,441
successful OwnSlot reads; tree has none. The 0.9999 target ceiling reflects cdjs's
small reach, not a claimed 5% gain. All targets still need improved median cycles
and every paired target below 1 for a diagnostic pass.

Base e54e33eb, branch agent/own-slot-cache-hot-answers/codex. Runtime boundary:
`bytecode/named_property_cache.rs`, its tests and `compact_fn/property.rs`.
No typed-loop, bindings, class-field, equality, layout-order or budget changes.
Only this plan and its index row are owned in performance-units.

Unlike the rejected T033 prototype pre-probe, this serves an already guarded
own answer; it never assumes a new receiver misses a prototype method name.
Use a borrowed out-of-line helper to limit hot executor and probe growth.

## Verification and result

All measurements use lockf /tmp/qjs-lead/host-timing.lock and wait for competing
build processes to finish. Screens are diagnostic, never formal retention.
A control cycle failure at unchanged instructions is reported with symbol
positions/sizes as placement; it does not justify a second attempt or repinning.
The required full gate, focused negative tests and compare-qjs run before handoff.

### Attempt 1, 2026-10-12 (uncommitted runtime diff after 2e879910)

Borrowed OwnSlot helper added as a fourth hot match case. Five-pair diagnostic
screen: FAIL; exact failure `target external/jetstream3-js-subset/cdjs: a pair
did not improve (max 1.0280)`. cdjs also adds 0.46% instructions. All control
medians are inside 1.03 on both counters; no cycle-only control failure.
High load warning concerned wall time; no competing-build warning.

| Case | Instructions/base | Cycles/base |
| --- | ---: | ---: |
| external/jetstream3-js-subset/hash-map | 0.932424 | 0.948260 |
| external/jetstream3-js-subset/cdjs | 1.004575 | 0.997320 |
| external/sunspider-1.0/3d-raytrace | 0.985222 | 0.980249 |
| external/sunspider-1.0/access-binary-trees | 1.004292 | 0.998956 |
| external/kraken-1.1/ai-astar | 1.000019 | 0.999208 |
| sentinel/recursive_call_tree | 0.999969 | 1.011976 |
| sentinel/prototype_method_call | 1.000798 | 1.001189 |
| sentinel/polymorphic_call_site | 1.000040 | 1.003783 |
| sentinel/capturing_closure_call | 0.999755 | 1.000703 |
| sentinel/heterogeneous_property_read | 0.999935 | 1.001796 |
| sentinel/string_key_map_churn | 0.999898 | 0.986802 |

Base binary SHA-256 6b40aa1944af1096657641cab5b2f80319cc35c3765a79da169cb5c698af09c4;
candidate 8690199fa8087983e29590fa96e3708b592895a6aeae1406a1e1c9ec0177e316.
The exact command (run from this worktree):

```sh
lockf /tmp/qjs-lead/host-timing.lock python3 -m tools.benchmark.screen --candidate target/release/qjs --base /Users/qingyingliu/Code/js-engine/quickjs-rust/target/perf-loop/rev-e54e33eb9371c5cb4627bd8e590a648085a7fa26/qjs --plan tasks/performance-units/own-slot-cache-hot-answers.json --pairs 5 --settle 60 --symbols 12 --json target/own-slot/screen-1.json
```

`llvm-nm -n -C --defined-only` offsets relative to 0x100000000, with size bytes:
call_leaf 8580/2784 → 8580/2784; VM 14070/6656 → 14070/6672;
wide 19070/20048 → 19080/20224; probe 3f650/3968 → 3f730/3968.
New borrowed helper: 36a9f0/224. No layout-order edits.
Attempt 2 moves the OwnSlot check into the original hot match's fallback arm,
so SharedSlot/prototype dispatch keeps its previous cases. This addresses the
positive instruction delta in cdjs, not a placement-control failure.

### Attempt 2: rejected, two-attempt budget exhausted

Original hot match cases preserved; only its fallback calls the borrowed helper.
Five-pair diagnostic gate FAIL. Exact output:

```text
target external/jetstream3-js-subset/cdjs: median cycles 1.0163 > 0.9999
target external/jetstream3-js-subset/cdjs: a pair did not improve (max 1.0200)
target external/sunspider-1.0/3d-raytrace: a pair did not improve (max 1.0214)
```

| Case | Instructions/base | Cycles/base |
| --- | ---: | ---: |
| external/jetstream3-js-subset/hash-map | 0.933503 | 0.944497 |
| external/jetstream3-js-subset/cdjs | 1.005804 | 1.016335 |
| external/sunspider-1.0/3d-raytrace | 0.985832 | 0.988713 |
| external/sunspider-1.0/access-binary-trees | 1.003083 | 0.996669 |
| external/kraken-1.1/ai-astar | 1.000219 | 1.009766 |
| sentinel/recursive_call_tree | 0.999911 | 0.996837 |
| sentinel/prototype_method_call | 0.999978 | 0.999174 |
| sentinel/polymorphic_call_site | 1.000032 | 1.007687 |
| sentinel/capturing_closure_call | 1.000038 | 1.000291 |
| sentinel/heterogeneous_property_read | 1.000042 | 0.997813 |
| sentinel/string_key_map_churn | 0.999578 | 0.998512 |

No competing-process warning. All control medians pass both instruction and
cycle ceilings; no placement-only control failure to chase. cdjs's consistently
positive instruction delta is material: this implementation is not justified
for all three selected targets. Native release recompilation also changes many
unrelated symbol sizes; this screen does not localize every additional instruction.
No runtime changes should enter the lead's formal batch from this unit.

Second command is attempt 1's command with `--max-load 1000 --require-quiet`,
`screen-2.json` output. This removes load-average settling after the explicit
competing-process check; wall time is unused. Candidate binary SHA-256:
c270267c2617a53271b09aaab6de69531106ac4a446aaf25f7c04c66e73f0a43.
Offsets/sizes: call_leaf 8580/2784 → 8580/2784;
VM 14070/6656 → 14070/6672; wide 19070/20048 → 19080/20224;
probe 3f650/3968 → 3f720/3968; new helper 1065d0/224.

Multiplying formal e54/NG paired cycles by this diagnostic median gives only
an estimate, not new NG evidence: hash-map 1.2821, cdjs 1.3709, raytrace 1.3357.
None reaches 1.0. The baseline formal cycle ratios are 1.357461, 1.348961 and
1.351027 (NG f7830186; brief01 bundle, SHA-256
febc1b2f4904f113eca4ca4850630012e4e08ded0f85829ace3c91c9f48b6955).

Five named-property-cache tests passed in both attempts, including the two new
negative guard tests. Full check and compare-qjs were queued on the timing lock but cancelled before
execution when the budget closed; they were not run on either rejected candidate.
Runtime and added tests were restored to e54e33eb; the branch is documentation only. No attempt remains and no formal
comparison was run. A is explicitly rejected before starting B.

Final check-context, staged check-touched and branch-scope validation passed.
The original order-file SHA-256 remained
0629ec01a397a9d37130fffe9a6d939eb35ff31f67bafe00ab9138292f9edfd2.
