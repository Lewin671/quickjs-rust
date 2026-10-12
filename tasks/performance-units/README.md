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

| `global-numeric-binding-sinks` | Brief 02 | `e54e33eb` 2026-10-09 | rejected; runtime implementation withdrawn. Plan `4feb5f7b`; two working-tree screens with `lockf /tmp/qjs-lead/host-timing.lock ./scripts/perf-loop.sh --plan tasks/performance-units/global-numeric-binding-sinks.json` (5 pairs). Screen 1 PASS: cycles/base targets bitwise-and 0.1144 and top-level call 0.3038; ai-astar 0.9968, audio-dft 0.9856, sentinel max 1.0237. Screen 2 FAIL after callee/cell and computed-alias guards: targets 0.1124 and 0.3084; ai-astar 0.9844, audio-dft 0.9468; prototype-method sentinel 1.0430 exceeds 1.03 (other sentinels 0.9993, 0.9976, 1.0155, 1.0058, 1.0120). Both runs warned only about high load and unreliable wall time; no residual-build warning. Rejected source patch SHA-256 `e914c6c36301c4454288453eaa53e3c6fae926e1fd4918c310a53c38b682d7f7`, binary `efcf25a1896eb2fa30ce1e980683ad074b906185d076c0fca7f6fc4377c47f0a`, base binary `c3da30270e9454fcbfaa66af45fc78a654ab9c1da73528a4c63a402cb96fd501`. Candidate staged gate passed 2227 runtime tests and 65 focused Test262 cases; 101 typed-loop counter tests passed. Shape: proved hoisted/canonical sinks, scalar-only deferred exit publication, parameter-only per-write calls, out-of-line callee guard; no hot dispatch arm changed. Wide executor offset moved 0x200 to 0x400 and call_leaf 0x580 to 0x980; no causal claim. At N=1000, property_read and array_read had 1000 declines (virtual receiver slots), property_write one GetNamedTyped deopt then 999 declines (virtual object); dynamic_method_call had 1000 declines (object sources seeded as scalars); captured_write had one Call(0) decline and 14069 compact ops. None was blocked by global sinks. No formal run or promotion; new work requires a new profile/plan and lead-owned layout review. |

| `global-numeric-binding-sinks-repin` | Brief 02 reopen | `e54e33eb` 2026-10-09 | screen 1 FAIL at restored 0x200 (prototype 1.2143, ai-astar 1.2049); lead approved diagnostic isolation without charging an attempt. 30 fixed-N AB/BA pairs, no quiet wrapper, equal-length binary paths: cycles/base b/c/d/A-A ai-astar 1.008877/1.209968/1.207256/0.996548; prototype 1.040981/1.209020/1.210464/0.999875; capturing 1.022930/1.214232/1.021828/1.000228; heterogeneous 1.003762/1.196773/1.199725/0.999253; bitwise 0.118406/0.118404/0.118086/1.001729. Instructions approximately unchanged on controls; median P-core user-time shares >99.99%; wrapper/direct nice 0, QoS 17, relative priority 0. Prefix/tail bisection selected wide 0x400, independent call_leaf 0x580, budgeted probe; speculative evaluator/cache-get pins and guard refactor not retained. Small residuals remain beyond narrow A/A median intervals, not claimed zero. Diagnostics fixed: intended global route 1 entry/0 declines; scoped test-only residency bound preserves <=100 assertion and production bound. Final full gate passed 2228 runtime tests, 23 perf-counter diagnostics, 5170 subset cases; 102 typed counter tests and layout tests pass. Screen 2 PASS: `lockf /tmp/qjs-lead/host-timing.lock env PYTHONPATH=. PYTHONDONTWRITEBYTECODE=1 python3 target/profiles/global-numeric-binding-sinks-isolation/final-screen.py` execs perf-loop with frozen plan and --require-quiet; no competing build/engine processes; starting load 4.7617. Cycles/base targets 0.1144/0.3114; controls ai-astar 0.9934, audio-dft 1.0110; sentinels 0.9861/1.0088/0.9986/1.0024/1.0074/0.9932. Implementation `d5eaf51a`, layout `23d1c834`, dependent diagnostic expectation/budget tests in following commit; final evidence index SHA-256 `8de84065dfb96139f7863ad91c20c687ab4fb1b6483a20925758a1cd6dda42eb`, candidate binary `91683e74524557253166b1443ba7d14b9480eb09d6ee06bac58266b80688be17`. Estimated cycles/NG 0.5142/1.1294 and wall/NG 0.6742/1.1276 are cross-run estimates, no direct NG rerun. No formal comparison or promotion. Next: lead integration re-pin and batched formal decision; scripts/check.sh:63 issue remains out of scope. |
