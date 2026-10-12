# Performance-unit plans

Which frozen plan belongs to which task, and what happened to it.

Each `<unit>.json` here freezes one performance unit before implementation:
its base revision, queue binding, profile receipts, target and control
cases, thresholds and attempt budget. How to write, validate, screen and
decide a plan is in
[docs/performance-workflow.md](../../docs/performance-workflow.md). The
field meanings and decision states are in
[docs/benchmarking.md](../../docs/benchmarking.md#unit-plans-and-decisions).

## Rules

- A plan is frozen once committed. Its outcome is recorded in the index
  below and in the owning task, never by editing the JSON. The one
  permitted edit is advancing `migration.current_stage` in a migration
  plan, which the stage decision reads.
- `tools/benchmark/performance_decision.py` accepts schema 1 (leaf plans
  without `unit_kind`) and schema 2 (`unit_kind` of `leaf` or `migration`),
  so older plans stay byte-identical and keep their SHA-256 bindings.
- Evidence a plan or task cites under `target/` or `/tmp` is a local
  artifact and is not retained; the hashes identify it.
- Add a row when a plan is committed and update its outcome when the task
  records one.

## Index

Tasks are named by id; a task file is `tasks/T0NN-*.md` or, once closed,
under `tasks/archive/`. T018 and T021 entries are in those tasks' archived
logs, under a dated heading that names the unit. The plan JSON has no date
field: Base is `base_sha` and that commit's date. Outcomes are quoted from
task files only. "not recorded" means no task file names the plan; it says
nothing about whether the code exists.

| Plan (`<name>.json`) | Task | Base | Recorded outcome |
| --- | --- | --- | --- |
| `ascii-string-literal-span-copy` | T021 | `0c31864b` 2026-07-28 | rejected |
| `base-class-constructor-direct-slots` | not recorded | `d3f9c179` 2026-07-28 | not recorded |
| `borrowed-fast-native-predispatch` | T021 | `620bb67b` 2026-07-30 | rejected |
| `box-shaped-pair-storage` | T019 | `32a00b0e` 2026-07-29 | rejected |
| `boxed-string-virtual-index-properties` | T021 | `620bb67b` 2026-07-30 | rejected |
| `cached-direct-leaf-numeric-call-graph` | T021 | `0745c018` 2026-07-30 | rejected |
| `capture-free-regexp-program` | T021 | `af4a65a1` 2026-07-29 | rejected at promotion, after a retained fast screen |
| `chunked-compound-string-accumulator` | T021 | `620bb67b` 2026-07-30 | rejected |
| `compact-generic-bytecode-core` | T021 | `50a20d60` 2026-07-29 | rejected |
| `compact-hot-dispatch` | T021 | `6cac3f50` 2026-07-28 | rejected |
| `compact-hot-op-error-abi` | T026 | `2093eeab` 2026-08-03 | rejected |
| `compact-named-property-access` | not recorded | `adb77520` 2026-08-14 | not recorded |
| `compilation-graph-static-property-names` | T029 | `13e5d229` 2026-08-03 | rejected at promotion |
| `contiguous-direct-call-frame-stack` | T021 | `620bb67b` 2026-07-30 | rejected |
| `default-data-property-storage` | T018 | `e66d0303` 2026-07-29 | rejected |
| `dense-index-numeric-leaf` | T021 | `6cac3f50` 2026-07-28 | rejected |
| `dense-primitive-predicate-scan` | T021 | `60e28ecf` 2026-07-31 | rejected |
| `direct-construction-realm-context` | T021 | `32a00b0e` 2026-07-29 | rejected at source audit; never screened |
| `direct-eval-selected-bindings` | T021 | `4d14ee5a` 2026-07-28 | retained (fast); promotion inconclusive |
| `direct-leaf-class-construction` | not recorded | `adb77520` 2026-08-14 | not recorded |
| `direct-leaf-frame-cold-state` | T021 | `6cac3f50` 2026-07-28 | rejected |
| `direct-leaf-lazy-loop-plan-state` | T021 | `8a49ac9b` 2026-07-28 | rejected |
| `direct-leaf-tail-frame-reset` | T021 | `af4a65a1` 2026-07-29 | rejected |
| `direct-readonly-upvalue-frame-sharing` | not recorded | `e66d0303` 2026-07-29 | not recorded |
| `direct-slot-capture-closures` | T021 | `7d0f6bdb` 2026-07-29 | rejected |
| `discarded-binary-branch-superinstruction` | T028 | `e11b5d19` 2026-08-03 | rejected |
| `discarded-dynamic-string-reuse` | T018 | `7fffa88d` 2026-07-27 | fast targets and controls met; promotion inconclusive |
| `discarded-member-assignment-value-transfer` | T018 | `46ceb4f6` 2026-07-29 | rejected on a diagnostic screen; no formal decision |
| `exact-simple-regexp-atom` | T021 | `32856150` 2026-07-29 | retained (fast); promotion inconclusive |
| `for-in-ordinary-key-cache` | T021 | `da94a017` 2026-07-27 | retained on a local fast gate only; promotion not run |
| `frame-independent-native-construction` | T021 | `14d9f2f6` 2026-07-30 | retained (fast); promotion inconclusive |
| `frame-verified-direct-local-opcodes` | T027 | `bea6aacf` 2026-08-03 | rejected |
| `frameless-string-equality` | not recorded | `9c58edba` 2026-09-22 | not recorded |
| `general-call-activation-cost` | not recorded | `adb77520` 2026-08-14 | not recorded |
| `guarded-math-unary-call-opcode` | T021 | `10ba7595` 2026-07-30 | retained (fast and promotion) |
| `idempotent-dynamic-cell-overlay` | T021 | `620bb67b` 2026-07-30 | retained (fast and promotion) |
| `immediate-object-slot-read-cache` | T021 | `6cac3f50` 2026-07-28 | rejected |
| `incremental-frame-deopt-cell-overlay` | T021 | `60e28ecf` 2026-07-31 | rejected |
| `inline-regexp-match-state-list` | T021 | `10ba7595` 2026-07-30 | rejected |
| `isolated-class-field-data-install` | not recorded | `f0b3ea87` 2026-07-28 | not recorded |
| `lazy-weak-object-refcount` | T019 | `32a00b0e` 2026-07-29 | rejected |
| `native-callback-direct-leaf-bridge` | T021 | `32a00b0e` 2026-07-29 | rejected |
| `number-only-local-assignment-program` | T021 | `60e28ecf` 2026-07-31 | rejected |
| `number-only-numeric-leaf-program` | T021 | `0c31864b` 2026-07-28 | accepted; promotion inconclusive |
| `numeric-recursive-call-cluster` | T021 | `6cac3f50` 2026-07-28 | rejected |
| `ordinary-bytecode-value-only-exit` | T021 | `32a00b0e` 2026-07-29 | rejected |
| `owned-number-binary-operands` | T021 | `50a20d60` 2026-07-29 | rejected |
| `packed-typed-loop-scalars` | T030 | `e68abfc9` 2026-08-03 | rejected |
| `primitive-string-prototype-read-cache` | T021 | `6cac3f50` 2026-07-28 | rejected |
| `realm-object-arena` | T023 | `32a00b0e` 2026-07-29 | rejected |
| `realm-regexp-program-cache` | T033 | `93f98a4a` 2026-09-22 | in the batch rejected at promotion; no separate decision recorded |
| `realm-single-code-unit-strings` | T031 | `a8e9d253` 2026-09-05 | retained (promotion) |
| `realm-string-prototype-direct-read` | T021 | `0745c018` 2026-07-30 | retained (promotion) |
| `regexp-choice-stack-capture-undo` | T025 | `c62314aa` 2026-08-02 | migration; stage 2 advance, stage 3 abort |
| `regexp-cow-capture-snapshots` | T021 | `14d9f2f6` 2026-07-30 | rejected |
| `regexp-first-continuation-repetition` | T021 | `7d0f6bdb` 2026-07-29 | rejected |
| `regexp-literal-shared-blueprint` | T021 | `14d9f2f6` 2026-07-30 | rejected |
| `shaped-prototype-slot-reads` | T032 | `5702c789` 2026-09-06 | retained (promotion, 60 blocks) |
| `shared-functional-replace-input` | T021 | `60e28ecf` 2026-07-31 | retained (promotion) |
| `shared-instance-field-key-install` | T021 | `6cac3f50` 2026-07-28 | rejected |
| `shared-slot-promotion-compaction` | T021 | `6cac3f50` 2026-07-28 | rejected |
| `small-property-storage-threshold-12` | not recorded | `fe8d287e` 2026-07-29 | not recorded |
| `string-wrapper-shared-data-buffer` | T018 | `e66d0303` 2026-07-29 | rejected |
| `transition-shape-object-storage` | T021 | `50a20d60` 2026-07-29 | rejected |
| `typed-loop-branchy-dense-element-transform` | T021 | `60e28ecf` 2026-07-31 | rejected |
| `typed-loop-branchy-nested-dense-read` | T021 | `4023b3dd` 2026-07-31 | retained (promotion) |
| `typed-loop-computed-index-write` | T018 | `1e534b1f` 2026-07-27 | retained (fast); promotion inconclusive |
| `typed-loop-dense-object-number-read` | T021 | `620bb67b` 2026-07-30 | rejected |
| `typed-loop-dense-tail-append` | T021 | `60e28ecf` 2026-07-31 | withdrawn at preflight; never screened |
| `typed-loop-entry-expanded-numeric-helper-bodies` | T021 | `620bb67b` 2026-07-30 | rejected |
| `typed-loop-entry-prepared-numeric-helper-graph` | T021 | `14d9f2f6` 2026-07-30 | rejected |
| `typed-loop-fixed-index-nested-dense-read` | T021 | `60e28ecf` 2026-07-31 | rejected |
| `typed-loop-isolated-prepared-numeric-helper-graph` | T021 | `14d9f2f6` 2026-07-30 | rejected |
| `typed-loop-numeric-object-fields` | T021 | `930eaeb8` 2026-07-29 | rejected |
| `typed-loop-register-file-compaction` | T021 | `7d0f6bdb` 2026-07-29 | retained (fast); promotion not claimed |
| `typed-loop-same-region-plan-decline` | not recorded | `adb77520` 2026-08-14 | not recorded |
| `typed-loop-scratch-reuse` | T021 | `6cac3f50` 2026-07-28 | accepted; no decision artifact recorded |
| `typed-loop-sloppy-global-numeric-sinks` | T018 | `c8c24648` 2026-07-27 | retained (fast); promotion inconclusive |
| `typed-loop-unbound-numeric-native-call` | not recorded | `32a00b0e` 2026-07-29 | not recorded |
| `wide-compact-tier` | not recorded | `adb77520` 2026-08-14 | not recorded |
| `wide-tier-interpreter-exits` | T033 | `93f98a4a` 2026-09-22 | screen pass; rejected at batched promotion |
| `wide-tier-math-calls-and-field-thunks` | T033 | `93f98a4a` 2026-09-22 | screen attempt 1 failed; nothing later recorded |
| `class-field-batch-install` | Brief 03 (lead-owned integration) | `e54e33eb` 2026-10-11 | screen attempt 1 PASS at `ac6f830d`; second attempt unused; recommended for lead formal batch, not retained; see handoff below |

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
| `global-numeric-binding-sinks` | Brief 02 | `e54e33eb` 2026-10-09 | rejected; runtime implementation withdrawn. Plan `4feb5f7b`; two working-tree screens with `lockf /tmp/qjs-lead/host-timing.lock ./scripts/perf-loop.sh --plan tasks/performance-units/global-numeric-binding-sinks.json` (5 pairs). Screen 1 PASS: cycles/base targets bitwise-and 0.1144 and top-level call 0.3038; ai-astar 0.9968, audio-dft 0.9856, sentinel max 1.0237. Screen 2 FAIL after callee/cell and computed-alias guards: targets 0.1124 and 0.3084; ai-astar 0.9844, audio-dft 0.9468; prototype-method sentinel 1.0430 exceeds 1.03 (other sentinels 0.9993, 0.9976, 1.0155, 1.0058, 1.0120). Both runs warned only about high load and unreliable wall time; no residual-build warning. Rejected source patch SHA-256 `e914c6c36301c4454288453eaa53e3c6fae926e1fd4918c310a53c38b682d7f7`, binary `efcf25a1896eb2fa30ce1e980683ad074b906185d076c0fca7f6fc4377c47f0a`, base binary `c3da30270e9454fcbfaa66af45fc78a654ab9c1da73528a4c63a402cb96fd501`. Candidate staged gate passed 2227 runtime tests and 65 focused Test262 cases; 101 typed-loop counter tests passed. Shape: proved hoisted/canonical sinks, scalar-only deferred exit publication, parameter-only per-write calls, out-of-line callee guard; no hot dispatch arm changed. Wide executor offset moved 0x200 to 0x400 and call_leaf 0x580 to 0x980; no causal claim. At N=1000, property_read and array_read had 1000 declines (virtual receiver slots), property_write one GetNamedTyped deopt then 999 declines (virtual object); dynamic_method_call had 1000 declines (object sources seeded as scalars); captured_write had one Call(0) decline and 14069 compact ops. None was blocked by global sinks. No formal run or promotion; new work requires a new profile/plan and lead-owned layout review. |

| `global-numeric-binding-sinks-repin` | Brief 02 reopen | `e54e33eb` 2026-10-09 | screen 1 FAIL at restored 0x200 (prototype 1.2143, ai-astar 1.2049); lead approved diagnostic isolation without charging an attempt. 30 fixed-N AB/BA pairs, no quiet wrapper, equal-length binary paths: cycles/base b/c/d/A-A ai-astar 1.008877/1.209968/1.207256/0.996548; prototype 1.040981/1.209020/1.210464/0.999875; capturing 1.022930/1.214232/1.021828/1.000228; heterogeneous 1.003762/1.196773/1.199725/0.999253; bitwise 0.118406/0.118404/0.118086/1.001729. Instructions approximately unchanged on controls; median P-core user-time shares >99.99%; wrapper/direct nice 0, QoS 17, relative priority 0. Prefix/tail bisection selected wide 0x400, independent call_leaf 0x580, budgeted probe; speculative evaluator/cache-get pins and guard refactor not retained. Small residuals remain beyond narrow A/A median intervals, not claimed zero. Diagnostics fixed: intended global route 1 entry/0 declines; scoped test-only residency bound preserves <=100 assertion and production bound. Final full gate passed 2228 runtime tests, 23 perf-counter diagnostics, 5170 subset cases; 102 typed counter tests and layout tests pass. Screen 2 PASS: `lockf /tmp/qjs-lead/host-timing.lock env PYTHONPATH=. PYTHONDONTWRITEBYTECODE=1 python3 target/profiles/global-numeric-binding-sinks-isolation/final-screen.py` execs perf-loop with frozen plan and --require-quiet; no competing build/engine processes; starting load 4.7617. Cycles/base targets 0.1144/0.3114; controls ai-astar 0.9934, audio-dft 1.0110; sentinels 0.9861/1.0088/0.9986/1.0024/1.0074/0.9932. Implementation `d5eaf51a`, layout `23d1c834`, dependent diagnostic expectation/budget tests in following commit; final evidence index SHA-256 `8de84065dfb96139f7863ad91c20c687ab4fb1b6483a20925758a1cd6dda42eb`, candidate binary `91683e74524557253166b1443ba7d14b9480eb09d6ee06bac58266b80688be17`. Estimated cycles/NG 0.5142/1.1294 and wall/NG 0.6742/1.1276 are cross-run estimates, no direct NG rerun. No formal comparison or promotion. Next: lead integration re-pin and batched formal decision; scripts/check.sh:63 issue remains out of scope. |
