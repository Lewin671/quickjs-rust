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

- Status: screened, ready for lead's formal batch; not formally retained. The
  frozen plan (`e0027e5e`) is unchanged, including its rank-9 priority override.
- Implementation: present (`9ed0aea0`); `cd32549b` removes rescanning after a
  decline until the constant segment's boundary. The 512-field class inspection
  test fails with 131328 visits if the skip guard is disabled, and passes with
  512 after restoration; an execution test checks all sequential definitions.
  The lead-authorized re-pin is isolated in `ac6f830d` (only CLI order file),
  stable on iteration 2; executor offsets are 0x200/0xc40.
- Verified at `ac6f830d`: staged gates for both continuation commits, the full
  `check.sh` pre-push gate (2236 runtime tests, 5170 Test262 subset), and
  `check-touched.sh --base e54e33eb --explain` with 52 selected Test262 cases.
  CI run 38218871886 passed at that exact revision. All prior semantic tests
  remain present; no dependency, wide dispatch, or typed-loop changes were made.
- Screen attempt 1, `ac6f830d` vs `e54e33eb`: **PASS**, five pairs; target cycles
  median 0.933631 [0.924223, 0.944993], instructions 0.9294; all controls below
  the frozen 1.03 median ceiling. Attempt 2 was not used because attempt 1 passed.
- Screen command: `lockf /tmp/qjs-lead/host-timing.lock python3 -m
  tools.benchmark.screen --candidate target/release/qjs --base
  target/perf-loop/rev-e54e33eb9371c5cb4627bd8e590a648085a7fa26/qjs --plan
  tasks/performance-units/class-field-batch-install.json --pairs 5 --settle 300
  --json target/class-field-batch-install/screen-attempt1.json --symbols 15`.
  The local JSON is diagnostic, not formal decision evidence.
- Screen load was 43.22 before / 29.25 after. No build-process warning occurred.
  Exact warning: `warning: load exceeded 5.00; the wall column is unreliable,
  counter ratios are still usable`. Wall times are not used as payoff evidence;
  recursive sentinel's cycle spread is broad and needs the formal gate.

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

After micro command: `lockf /tmp/qjs-lead/host-timing.lock env PYTHONPATH=.
python3 target/class-field-batch-install/micro-after.py`. Same class bodies and
paths as the preimplementation micro; N=1m/2m, five samples per engine/size,
plain release builds. Base was remeasured beside candidate and pinned NG
`f7830186`. Values are (median 2N - median N)/N cycles per instance:

| Fields | Constructor stores | Base | Candidate | NG | Candidate/base | Candidate/NG |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| 2 | no | 879.30 | 802.03 | 754.24 | 0.912120 | 1.063351 |
| 2 | yes | 980.42 | 914.05 | 843.84 | 0.932298 | 1.083193 |
| 3 | no | 1014.17 | 871.01 | 974.93 | 0.858842 | 0.893411 |
| 3 | yes | 1132.53 | 1026.81 | 1077.69 | 0.906650 | 0.952787 |
| 8 | no | 2500.77 | 1418.84 | 2362.48 | 0.567363 | 0.600575 |
| 8 | yes | 2844.30 | 1767.54 | 2564.69 | 0.621433 | 0.689186 |

- Binary identities: candidate SHA-256
  `0e48236253f8baa1aa24915e9ccab4f7dc5406a153618c86ea081a65b67393c8`;
  base `6b40aa1944af1096657641cab5b2f80319cc35c3765a79da169cb5c698af09c4`.
  Local artifacts are under `target/class-field-batch-install/`; numbers and
  revisions above remain available without those artifacts.
- Unresolved: only Small storage fitting the whole run with configurable
  existing descriptors is batched. Other storage/receivers keep sequential
  installation. Full portfolio performance, complete Test262 parity, and
  formal promotion have not been verified. Broad object_allocation remains the
  frozen portfolio control; its source permits virtual/folded object work and
  does not establish neutrality of all physical allocation paths.
- Next action: lead may include this unit in the formal batch from `e54e33eb`;
  re-pin on integration and enforce the plan's formal/promotion gates. High
  screen load and the recursive-sentinel spread must not be treated as formal
  proof of absence of regressions.
