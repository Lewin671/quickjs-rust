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

| `global-numeric-binding-sinks-repin` | Brief 02 reopen | `e54e33eb` 2026-10-09 | stopped by lead rule after screen 1 FAIL; second attempt not used; runtime/layout implementation withdrawn; complete candidate saved locally as `target/profiles/global-numeric-binding-sinks-repin/stopped/candidate.patch` (measured source HEAD `91b34bb6`, patch SHA-256 `9d60aa0136242e644f0e611ac25b6eb23cabadbb54690cae9117c645326a35d0`, binary `b5a3c0013cb92aa7764d775a89aeb815ab1d073fe43e4df13a16847d2c3d5218`). Valid plan SHA-256 `c16a173da34875227f7301d733d81bf7a393df4a773dc60336acfd66a4737647` was verified before restoring runtime; initial invalid zero-fraction receipt entry was corrected in `91b34bb6`. New evidence index SHA-256 `c212315b732d98feffb6343fd6ea688f7ee8f01dd45823e1066ddab7b33accc8`; counter A/B instructions/cycles 1.00088091/0.99611123, same execution counters; release 0.99722425/1.03898673; branch-miss counts unavailable, discarded-work fraction is not a count. `lockf /tmp/qjs-lead/host-timing.lock python3 target/profiles/global-numeric-binding-sinks-repin/quiet-exec.py ./scripts/perf-loop.sh --plan tasks/performance-units/global-numeric-binding-sinks-repin.json`: no competing build/engine processes; load 4.79 before/3.26 after; 5 pairs. Cycles/base: bitwise-and 0.1133, top-level call 0.3130, ai-astar 1.2049, audio-dft 1.0031; sentinels recursive 1.0003, prototype 1.2143, polymorphic 1.1657, capturing 1.2303, heterogeneous 1.2013, churn 1.1100. Prototype instructions 0.9975; other control instructions approximately 1.0. Re-pinned offsets: VM 0xc40, wide 0x200, call_leaf 0x580, probe 0x650 (3968 bytes); probe added to budgeted callees. Candidate gates: 2227 runtime tests, 101 typed counter tests, 65 focused Test262 cases, 6 layout tests passed. Subsequent candidate full check FAILED its perf-counters diagnostics stage: `a_declining_same_region_plan_no_longer_blocks_the_typed_program` observed 101 declined edges against <=100; `supplying_loop_plans_externally_does_not_change_which_plan_claims_a_site` expected 0 global-loop entries and observed 1. Full correctness not established. `scripts/check.sh:63` also masks a non-final per-plan validation failure; that script is outside this unit boundary. Estimated cycles/NG (cross-run multiplication, not an NG rerun): bitwise 0.5093, top call 1.1352; wall estimates 0.6372/1.1374. No formal comparison/promotion. Next: lead-authorized fixed-N 30-pair AB/BA comparing exact baseline, rejected unpinned binary and saved restored-layout binary, plus an executors-only pin to isolate the new probe slot; record CPU residency and raw branch-miss counters before changing layout again. |
