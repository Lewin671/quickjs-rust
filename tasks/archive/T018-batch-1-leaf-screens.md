# T018: Batch 1 leaf screens before integration

- Status: historical-campaign. Leaf handoff evidence, not a promotion decision.
- Implementation: present on the two named leaf tips; integration state is in
  [T037](../T037-integration-batch-1.md).
- Verified at: global tip `92ee2968`, class tip `7c15dde5`; measurements below
  name their own revisions and do not establish current-tree performance.
- Evidence: original handoff records below; local artifacts are not retained.
- Unresolved: formal decisions are pending in T037.
- Next action: follow T037.

## Global-sinks handoff records

| Plan | Owner | Base | Historical outcome |
| --- | --- | --- | --- |
| `global-numeric-binding-sinks` | Brief 02 | `e54e33eb` 2026-10-09 | rejected; runtime implementation withdrawn. Plan `4feb5f7b`; two working-tree screens with `lockf /tmp/qjs-lead/host-timing.lock ./scripts/perf-loop.sh --plan tasks/performance-units/global-numeric-binding-sinks.json` (5 pairs). Screen 1 PASS: cycles/base targets bitwise-and 0.1144 and top-level call 0.3038; ai-astar 0.9968, audio-dft 0.9856, sentinel max 1.0237. Screen 2 FAIL after callee/cell and computed-alias guards: targets 0.1124 and 0.3084; ai-astar 0.9844, audio-dft 0.9468; prototype-method sentinel 1.0430 exceeds 1.03 (other sentinels 0.9993, 0.9976, 1.0155, 1.0058, 1.0120). Both runs warned only about high load and unreliable wall time; no residual-build warning. Rejected source patch SHA-256 `e914c6c36301c4454288453eaa53e3c6fae926e1fd4918c310a53c38b682d7f7`, binary `efcf25a1896eb2fa30ce1e980683ad074b906185d076c0fca7f6fc4377c47f0a`, base binary `c3da30270e9454fcbfaa66af45fc78a654ab9c1da73528a4c63a402cb96fd501`. Candidate staged gate passed 2227 runtime tests and 65 focused Test262 cases; 101 typed-loop counter tests passed. Shape: proved hoisted/canonical sinks, scalar-only deferred exit publication, parameter-only per-write calls, out-of-line callee guard; no hot dispatch arm changed. Wide executor offset moved 0x200 to 0x400 and call_leaf 0x580 to 0x980; no causal claim. At N=1000, property_read and array_read had 1000 declines (virtual receiver slots), property_write one GetNamedTyped deopt then 999 declines (virtual object); dynamic_method_call had 1000 declines (object sources seeded as scalars); captured_write had one Call(0) decline and 14069 compact ops. None was blocked by global sinks. No formal run or promotion; new work requires a new profile/plan and lead-owned layout review. |
| `global-numeric-binding-sinks-repin` | Brief 02 reopen | `e54e33eb` 2026-10-09 | screen 1 FAIL at restored 0x200 (prototype 1.2143, ai-astar 1.2049); lead approved diagnostic isolation without charging an attempt. 30 fixed-N AB/BA pairs, no quiet wrapper, equal-length binary paths: cycles/base b/c/d/A-A ai-astar 1.008877/1.209968/1.207256/0.996548; prototype 1.040981/1.209020/1.210464/0.999875; capturing 1.022930/1.214232/1.021828/1.000228; heterogeneous 1.003762/1.196773/1.199725/0.999253; bitwise 0.118406/0.118404/0.118086/1.001729. Instructions approximately unchanged on controls; median P-core user-time shares >99.99%; wrapper/direct nice 0, QoS 17, relative priority 0. Prefix/tail bisection selected wide 0x400, independent call_leaf 0x580, budgeted probe; speculative evaluator/cache-get pins and guard refactor not retained. Small residuals remain beyond narrow A/A median intervals, not claimed zero. Diagnostics fixed: intended global route 1 entry/0 declines; scoped test-only residency bound preserves <=100 assertion and production bound. Final full gate passed 2228 runtime tests, 23 perf-counter diagnostics, 5170 subset cases; 102 typed counter tests and layout tests pass. Screen 2 PASS: `lockf /tmp/qjs-lead/host-timing.lock env PYTHONPATH=. PYTHONDONTWRITEBYTECODE=1 python3 target/profiles/global-numeric-binding-sinks-isolation/final-screen.py` execs perf-loop with frozen plan and --require-quiet; no competing build/engine processes; starting load 4.7617. Cycles/base targets 0.1144/0.3114; controls ai-astar 0.9934, audio-dft 1.0110; sentinels 0.9861/1.0088/0.9986/1.0024/1.0074/0.9932. Implementation `d5eaf51a`, layout `23d1c834`, dependent diagnostic expectation/budget tests in following commit; final evidence index SHA-256 `8de84065dfb96139f7863ad91c20c687ab4fb1b6483a20925758a1cd6dda42eb`, candidate binary `91683e74524557253166b1443ba7d14b9480eb09d6ee06bac58266b80688be17`. Estimated cycles/NG 0.5142/1.1294 and wall/NG 0.6742/1.1276 are cross-run estimates, no direct NG rerun. No formal comparison or promotion. Next: lead integration re-pin and batched formal decision; scripts/check.sh:63 issue remains out of scope. |

## Brief 03 handoff: class-field-batch-install

- Status: screen passed, recommended for lead's formal batch; not retained.
  Plan `e0027e5e` remains byte-identical, including the rank-9 priority override.
- Implementation `9ed0aea0`; fix `cd32549b` skips declined segments until their boundary.
  The 512-field class test fails with 131328 inspections without the skip guard,
  and passes with 512 after restoration. Re-pin `ac6f830d` changes only CLI order;
  stable at iteration 2, executor offsets 0x200/0xc40.
- Verified at `ac6f830d`: staged gates, pre-push `check.sh` (2236 runtime tests, 5170 Test262 subset),
  and `check-touched.sh --base e54e33eb --explain` (52 Test262 slice cases).
  CI run 38218871886 passed. No dependencies, wide dispatch arms, or typed-loop changes.
- Attempt 1 at `ac6f830d` vs `e54e33eb`: **PASS**, five pairs; target instructions
  0.9294. All target cycle pairs below 1 and all control medians below 1.03.
  Attempt 2 unused because attempt 1 passed. Diagnostic, not formal evidence.
- Screen command (JSON/log under `target/class-field-batch-install/`): `lockf /tmp/qjs-lead/host-timing.lock python3 -m tools.benchmark.screen --candidate target/release/qjs --base target/perf-loop/rev-e54e33eb9371c5cb4627bd8e590a648085a7fa26/qjs --plan tasks/performance-units/class-field-batch-install.json --pairs 5 --settle 300 --json target/class-field-batch-install/screen-attempt1.json --symbols 15`.
- Load 43.22 before / 29.25 after; no build-process warning. Exact warning:
  `warning: load exceeded 5.00; the wall column is unreliable, counter ratios are still usable`.
  Wall is not payoff evidence; recursive-sentinel spread needs formal checking.

Screen candidate/base cycles, median [min, max]:

| Case | Cycles |
| --- | ---: |
| `external/jetstream3-js-subset/raytrace-public-class-fields` | 0.933631 [0.924223, 0.944993] |
| `broad/object_allocation` | 0.990583 [0.958177, 1.022992] |
| `external/sunspider-1.0/access-binary-trees` | 0.994729 [0.992097, 1.023086] |
| `external/jetstream3-js-subset/cdjs` | 0.997894 [0.984290, 1.004744] |
| `external/jetstream3-js-subset/hash-map` | 1.002979 [0.997915, 1.008144] |
| `sentinel/recursive_call_tree` | 1.022566 [0.878189, 1.111040] |
| `sentinel/prototype_method_call` | 0.990542 [0.988211, 0.995821] |
| `sentinel/polymorphic_call_site` | 0.994112 [0.989757, 1.004491] |
| `sentinel/capturing_closure_call` | 0.995313 [0.991813, 1.041679] |
| `sentinel/heterogeneous_property_read` | 1.013363 [1.006291, 1.026097] |
| `sentinel/string_key_map_churn` | 1.004825 [0.993897, 1.099161] |

Micro command: `lockf /tmp/qjs-lead/host-timing.lock env PYTHONPATH=. python3 target/class-field-batch-install/micro-after.py`.
Same class bodies/paths; N=1m/2m, five samples per engine/size, plain release builds.
Base remeasured beside candidate and pinned NG `f7830186`; (median 2N - median N)/N cycles/instance:

| Fields | Constructor stores | Base | Candidate | NG | Candidate/base | Candidate/NG |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| 2 | no | 879.30 | 802.03 | 754.24 | 0.912120 | 1.063351 |
| 2 | yes | 980.42 | 914.05 | 843.84 | 0.932298 | 1.083193 |
| 3 | no | 1014.17 | 871.01 | 974.93 | 0.858842 | 0.893411 |
| 3 | yes | 1132.53 | 1026.81 | 1077.69 | 0.906650 | 0.952787 |
| 8 | no | 2500.77 | 1418.84 | 2362.48 | 0.567363 | 0.600575 |
| 8 | yes | 2844.30 | 1767.54 | 2564.69 | 0.621433 | 0.689186 |

- Binary SHA-256: candidate `0e48236253f8baa1aa24915e9ccab4f7dc5406a153618c86ea081a65b67393c8`;
  base `6b40aa1944af1096657641cab5b2f80319cc35c3765a79da169cb5c698af09c4`.
  Numbers/revisions above remain available without the local artifacts.
- Limits: only whole runs fitting Small storage with configurable descriptors are batched;
  other receivers/storage use sequential installation. Full portfolio performance,
  complete Test262 parity, and promotion are unverified. Frozen broad object_allocation
  permits virtual/folded objects; it does not guard every physical allocation path.
- Next: lead may include this unit in the formal batch from `e54e33eb`; re-pin on integration
  and enforce formal/promotion gates. High load and recursive-sentinel spread are not formal proof.
