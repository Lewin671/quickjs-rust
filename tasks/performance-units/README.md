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
