# T072 measurement evidence

Owner and acceptance contract: [T072](../../archive/T072-live-property-proof.md).
Base `6e899d24c167d1393c077bcd69a01b61571dbcd1`; first design
`80b2eeccf8b014754a089e2245c9856acf811858`; final frozen candidate
`6c122407146614e5d480b642551d18e782c430f5`. All timing is the qualified
iMac x86_64 canonical agents/fat16-abort build. Local arm64 checks establish
correctness only. Raw assets are external queue artifacts, not retained Git inputs.
No held-out case selected a target or design; no final decide command is run.

## Collection ledger

- `20261023T125302Z-t072-base-queue`: exact-base versus itself, 30 blocks,
  workers 4; exit 0 / 2077 s, `ready_for_unit_gates`, all 86 cases complete,
  no measurement issue or imprecise case; two light background samples,
  no DISTURBED. Both Rust binaries are SHA-256
  `cbb471c7047917f02e13d0ecf83516d9d6e57c050f2170ddde3b5e6e17ed735e`.
- `20261023T125356Z-t072-exact-profiles`: three complete unamplified process
  samples per selected target; exit 0 / 82 s, all stdout checks pass,
  no DISTURBED, **invalid identity**: collector passed literal `HEAD` to
  `qjs_build_revision` (whose documented input is a SHA), reusing an old
  `rev-HEAD` cache. Receipts label the source correctly but contain binary
  `b3132b40...`, not the queue's `cbb471c7...`. No sample is used in the plan.
- `20261023T125820Z-t072-bind-export`: exit 0; queue generation/export
  succeeds, but local `validate-unit` rejects the imported profiles with
  `profile does not describe the exact queue candidate executable`.
  The uncommitted plan draft is removed from the plan directory, not frozen.
- `20261023T133819Z-t072-exact-profiles-r2`: exit 0 / 82 s replacement;
  all 15 complete windows/stdout checks pass, no DISTURBED; resolves
  the complete commit SHA before cache access and asserts the queue binary
  hash before sampling. The healthy baseline is not rerun.
- `20261023T133841Z-t072-bind-export-r2`: exit 0; checks every receipt's
  source/hash against the queue before exporting content-verifiable assets.

`validate-unit` passes with all 15 content-verified receipts. Queue SHA-256
`c6063619ce1daee74f4d7c48254b2844d3d48d76102e9f0f6e9c2876fe7dbb06`;
plan SHA-256 `476cb44858ba0ca74de9ba94a3b200676fb1d4c069cc865c9f47c6a581d381b4`.
Freeze commit `f2ac7ec3` contains no runtime change. The subsequent diagnostic
import preserves `168ce2c0`'s outcome/receiver partitions and actual Hash/Eq
counting; its collector includes all ten development programs so every target
and the non-Object populations are measured before/after. Run only through
the queue, without timing:
`python3 -m tools.benchmark.named_property_counts --output "$BENCH_OUT/counts"`.
Fresh profile inclusive union (cache/handler/storage frames, recursion once):
meriyah 2240/6642, moment 6902/25941, diff 4475/14331, js-yaml 1892/10507,
matter-js 1684/7798. Inclusive reach includes necessary work; it is not a
removable price. All repeats are bound; targets remain the T068 rule above.
The healthy queue's ten-development wall geomean is 2.633808 times NG;
runner free disk at binding is 57,973,440,512 bytes. Neither is a candidate gain.

Local full gate passes on unchanged base runtime: 2,382 runtime tests,
21 agents tests, 24 diagnostic tests, 732 benchmark-tool tests, 35 script
tests and 5,170 curated Test262. Two extra property mutation/reentrancy
smokes pass; the reentrant getter/setter smoke also agrees with NG. Their
raw logs are local artifacts, not retained. No new complete exact scan is
claimed. Initial design/selection commit: `f101bcb5` (docs-only fast gate).

## First design

Diagnostics commit `689db691` passes its fast gate, including 65 selected
Test262 cases. The first hook invocation stopped on missing worktree Test262
inputs; rerun with the brief's read-only `TEST262_DIR` passes without any
corpus edit. `20261023T135308Z-t072-base-counts` exits 0 / 112 s; every one
of ten development stdout checks and both outcome/receiver partitions pass.

`ReadReceiver` borrows lazily once; revision-guarded by-value hits need no
table borrow. Cache slot guards validate receiver/layout, shape or stored
key against that same view. The data slot supplies both the cloned result
and installation metadata without another name lookup. Mutable slot proofs
retain descriptor checks and update the value revision; consuming the write
view releases its borrow before intrinsic identity capture. Prototype holder
upgrades and layout checks happen once; ordinary prototype installation uses
one lookup per holder and declines exotic storage. Existing inline hot paths
stay in place; the cold compact handler no longer repeats the hot probe.
No unsafe, dependency, binding, call-context or representation change.

Focused coverage includes all four storage forms, dynamic swap-removal,
read-only/accessor replacement, shared-key mismatch, receiver/parent shadowing,
weak ownership, namespace fallback, pending String indices and `globalThis`
capture. JS reentrancy tests cover both compact-eligible and forced-VM get/set,
strict/sloppy writes and changing Object/Array/Function/TypedArray/Proxy/null
receivers. Default clippy and the focused cache/reentrancy tests pass locally.

## First result and final design

Candidate `80b2eeccf8b014754a089e2245c9856acf811858` passes the full gate
(including 26 diagnostic tests), all 257 fixtures and all ten counter stdout/
partition checks. Jobs: `20261023T140356Z-t072-design1-counts` (0 / 112 s),
`20261023T140357Z-t072-design1-reference` (0),
`20261023T140449Z-t072-design1-triage` (0 / 257 s command, 317.196 s total),
`20261023T140542Z-t072-design1-export` (0). Triage has 110/110 valid pairs,
no invalid/heavy/DISTURBED samples; budget met. Target wall geomean
**1.003921464**, instructions **0.974620415**: no K=6 attempt is spent.
Target wall/instructions: meriyah 0.9066/0.8883, moment 1.0534/1.0227,
diff 1.0607/1.0116, js-yaml 0.9609/0.9381, matter-js 1.0477/1.0199.
Obvious control losses: string-key churn (instructions 1.0310), binary trees
(instructions 1.0382); complete tables will be retained at closure.

Counters establish removed work: scoped compares/hashes per handler entry
fall from 1.553/0.936 to 0.710/0.227 in meriyah, 0.374/0.369 to 0.025/0.024
in js-yaml, 7.056/0.077 to 5.249/0.033 in diff, 4.732/0.418 to 4.093/0.361
in moment, and 0.102/0.102 to effectively zero in matter-js. This does not
establish a speedup; the wall condition fails.

`20261023T141421Z-t072-design1-profiles` (0) samples all three regressed
targets three times using the exact triage image `50895e7e...`; checksums and
identity pass, no DISTURBED. Matter's pooled 8,149 samples assign 463 self to
`read_live`, 324 to `ReadReceiver::slot`, 317 to the cold named-get handler;
their **self**, not inclusive, sum is 13.548%. Diff has 325/269/135 of
14,386; moment 508/355/101 of 26,506. These identify the next implementation
cost, not its removable price.

The second and final design makes selected Exact/LiteralShape/OwnSlot hits
consume their guarded value or live slot directly before a cold handler is
entered. One out-of-line helper keeps generic Value cloning out of executor
arms. Existing SharedSlot/prototype hot paths remain; cold misses retain the
shared storage view. Writes derive strict/readonly completion and intrinsic
capture from the live descriptor, releasing the borrow before observation.
This is a hit-proof continuation, not layout tuning or another name memo.
No target, policy, threshold, attempt budget or frozen JSON changes.

The final implementation keeps hot own cloning in one non-inlined helper.
A mapped `ReadProof` retains the actual slot borrow until consumed. Stores
produce a `WriteOutcome` from the live descriptor and actual stored name;
the owning view drops its borrow before consuming deferred identity capture.
The newly validated installation becomes the hot way immediately. Additional
tests cover hot Exact/OwnSlot/literal reads, live updates, readonly/accessor
replacement, wrong receivers/layouts, namespace rejection and hot intrinsic
capture. No Array/Function representation or loop executor changes are added.

Queue status and iMac clock checked first (2026-10-23 20:52 CST, outside
02:45-05:00). Local free disk 49 GiB. The wrapper's private runner refs are
measurement transport, not publication to origin. No other owner's jobs
are cancelled. Raw artifacts are local; hashes and conclusions will be
recorded here before handoff.

## Design 1: complete quick-triage result

Target wall geomean **1.003921463976**, instructions **0.974620414657**. Tool label `likely loss`; collection budget met: `True`.
Pairs: 110 valid, 0 invalid, 0 heavy samples. Natural layout, five pairs per case; 95% intervals below are exploratory, unadjusted and do not cover layout bias. This is not formal or K=6 evidence.

- Base executable: `cbb471c7047917f02e13d0ecf83516d9d6e57c050f2170ddde3b5e6e17ed735e`.
- Candidate executable: `50895e7e4a65828da7f41515431524e9e8b1bc92a6dea8fc67530506abbeba61`.
- Request SHA-256: `a5e2dfc89e35dd240010efbc486acc590a97eb4f57eb38b044bccb6946c0afad`.
- Result SHA-256: `ba28510e7b2d36b9a8bddacb9e1724d5e6522ab5b0853e7f5bc16653cbbb185d`.
- Reference fixtures: 257; log SHA-256 `469fcd8ad0e7c2538a0aa4a4574e6ca848bb6b16e42eb9f5e5c4e449eec6a1c7`.

| Case | Wall point [95% interval] | Instructions point [95% interval] |
| --- | ---: | ---: |
| `development/meriyah/parse` (target) | 0.906556 [0.840098, 0.911147] | 0.888303 [0.887875, 0.888833] |
| `development/moment/dates` (target) | 1.053355 [1.017435, 1.061923] | 1.022738 [1.022682, 1.022752] |
| `development/diff/compare` (target) | 1.060736 [0.964848, 1.082338] | 1.011606 [1.011485, 1.011631] |
| `development/js-yaml/document` (target) | 0.960947 [0.946770, 0.977678] | 0.938145 [0.938101, 0.938182] |
| `development/matter-js/simulate` (target) | 1.047665 [1.037671, 1.134667] | 1.019931 [1.019923, 1.020019] |
| `sentinel/capturing_closure_call` | 0.999995 [0.920307, 1.032412] | 1.004553 [1.004359, 1.004648] |
| `sentinel/heterogeneous_property_read` | 1.025979 [0.982005, 1.042885] | 1.000027 [0.999866, 1.000116] |
| `sentinel/polymorphic_call_site` | 0.968615 [0.876083, 0.987383] | 1.004761 [1.004656, 1.004847] |
| `sentinel/prototype_method_call` | 1.002253 [0.957435, 1.029554] | 1.003068 [1.003020, 1.003213] |
| `sentinel/recursive_call_tree` | 0.961248 [0.940499, 1.001605] | 1.000126 [1.000023, 1.000185] |
| `sentinel/string_key_map_churn` | 1.058258 [1.029285, 1.209746] | 1.030950 [1.030921, 1.031099] |
| `broad/property_dynamic_read` | 1.034907 [0.824695, 1.060221] | 1.003269 [1.003150, 1.003372] |
| `broad/object_allocation` | 0.993823 [0.956124, 1.024885] | 0.999948 [0.999836, 1.000063] |
| `broad/string_slice` | 1.111020 [0.972373, 1.171504] | 1.002972 [1.002740, 1.003126] |
| `broad/top_level_function_call` | 0.991685 [0.970960, 1.026216] | 1.003495 [1.003448, 1.003542] |
| `development/lodash/transform` | 0.998310 [0.990772, 1.010851] | 1.008929 [1.008038, 1.009001] |
| `external/kraken-1.1/json-stringify-tinderbox` | 0.992982 [0.975047, 1.004035] | 1.000246 [0.998883, 1.002407] |
| `external/kraken-1.1/json-parse-financial` | 0.951702 [0.930914, 0.983783] | 1.004817 [1.004509, 1.004914] |
| `external/jetstream3-js-subset/raytrace-public-class-fields` | 1.026566 [1.015524, 1.046443] | 1.016196 [1.016152, 1.016225] |
| `external/sunspider-1.0/access-binary-trees` | 1.018986 [0.998706, 1.031129] | 1.038231 [1.038010, 1.038338] |
| `external/kraken-1.1/audio-dft` | 0.999742 [0.980408, 1.014504] | 1.000431 [1.000332, 1.001573] |
| `external/jetstream3-js-subset/cdjs` | 1.019028 [1.005168, 1.025390] | 1.026608 [1.025761, 1.027308] |

Obvious diagnostic losses: `sentinel/string_key_map_churn`, `external/sunspider-1.0/access-binary-trees`.

Stage seconds: `{"analysis": 2.6521011530421674, "build_base": 0.8510867450386286, "build_candidate": 71.05657923407853, "build_quieting": 0.03093184600584209, "collection": 172.30468615703285, "command_total": 256.5907945970539, "end_to_end": 317.19591546058655, "prepare_workloads": 0.1827785619534552, "queue_quieting": 60.60827040672302, "seal_and_replay": 8.052835517912172, "settle_and_launch": 1.1310273179551587, "validate_and_freeze": 0.15839781996328384}`.

| Role | Job | Exit | Log SHA-256 |
| --- | --- | --- | --- |
| base-counts | `20261023T135308Z-t072-base-counts` | `done 0` | `75c15de67a40e92f16230eebed11975d35ac0c94260c10e3bc1d82537f41b983` |
| counts | `20261023T140356Z-t072-design1-counts` | `done 0` | `3a0df96c4aed6b82641f4e45fde3c0c3330547c474b3b95d514a43bb29f92843` |
| reference | `20261023T140357Z-t072-design1-reference` | `done 0` | `76f4549efcf790881934f2db0dce668ea7609e9b4426109e9354a95e86781712` |
| triage | `20261023T140449Z-t072-design1-triage` | `done 0` | `ee7edf1ce571f4e66afa485e160562aef3a37867f9f6ae2e07168e52ac2037e7` |

## Design 2: complete quick-triage result

Target wall geomean **0.949774025573**, instructions **0.913469163516**. Tool label `likely loss`; collection budget met: `True`.
Pairs: 110 valid, 0 invalid, 0 heavy samples. Natural layout, five pairs per case; 95% intervals below are exploratory, unadjusted and do not cover layout bias. This is not formal or K=6 evidence.

- Base executable: `cbb471c7047917f02e13d0ecf83516d9d6e57c050f2170ddde3b5e6e17ed735e`.
- Candidate executable: `f6d5220992f3dd7d05afaad4af2e169a01258294e7995347d28f81e4ca74a5e3`.
- Request SHA-256: `6bb2be8a83002acdd85635aa0323015aaddc9f69db2f19fe674434fe202c55f1`.
- Result SHA-256: `97d1ebd0f4e2a0f047b05721c915e3425509964b99f2e0f86e26aa4f56470724`.
- Reference fixtures: 257; log SHA-256 `469fcd8ad0e7c2538a0aa4a4574e6ca848bb6b16e42eb9f5e5c4e449eec6a1c7`.

| Case | Wall point [95% interval] | Instructions point [95% interval] |
| --- | ---: | ---: |
| `development/meriyah/parse` (target) | 0.855555 [0.814924, 0.876912] | 0.822301 [0.821810, 0.822496] |
| `development/moment/dates` (target) | 1.076441 [1.054268, 1.081783] | 0.997068 [0.997033, 0.997101] |
| `development/diff/compare` (target) | 1.001428 [0.964955, 1.071064] | 0.982423 [0.982382, 0.982524] |
| `development/js-yaml/document` (target) | 0.930393 [0.897361, 0.946716] | 0.905043 [0.904986, 0.905063] |
| `development/matter-js/simulate` (target) | 0.900693 [0.886512, 0.965509] | 0.872461 [0.872430, 0.872525] |
| `sentinel/capturing_closure_call` | 1.014492 [0.983498, 1.094047] | 1.000055 [0.999948, 1.000117] |
| `sentinel/heterogeneous_property_read` | 0.988211 [0.940818, 1.021229] | 0.999971 [0.999898, 0.999984] |
| `sentinel/polymorphic_call_site` | 0.992106 [0.908275, 1.038245] | 1.000068 [0.999959, 1.000146] |
| `sentinel/prototype_method_call` | 0.996308 [0.982914, 1.027428] | 1.000097 [1.000044, 1.000127] |
| `sentinel/recursive_call_tree` | 0.974751 [0.954968, 1.016651] | 0.999989 [0.999838, 1.000076] |
| `sentinel/string_key_map_churn` | 1.002447 [0.988638, 1.013943] | 0.999923 [0.999857, 1.000149] |
| `broad/property_dynamic_read` | 0.990027 [0.964251, 0.998707] | 0.996126 [0.996081, 0.996172] |
| `broad/object_allocation` | 0.999097 [0.972576, 1.012009] | 1.000027 [0.999933, 1.000100] |
| `broad/string_slice` | 0.993129 [0.791526, 1.013608] | 1.000467 [1.000278, 1.000586] |
| `broad/top_level_function_call` | 1.012707 [0.997180, 1.020549] | 1.000002 [0.999944, 1.000013] |
| `development/lodash/transform` | 0.963465 [0.951480, 0.994569] | 0.985794 [0.984746, 0.986720] |
| `external/kraken-1.1/json-stringify-tinderbox` | 0.981677 [0.967198, 0.988868] | 1.000157 [0.999778, 1.000550] |
| `external/kraken-1.1/json-parse-financial` | 0.980955 [0.954762, 1.014266] | 1.004165 [1.003939, 1.004192] |
| `external/jetstream3-js-subset/raytrace-public-class-fields` | 1.024419 [1.020873, 1.040499] | 1.010119 [1.010085, 1.010167] |
| `external/sunspider-1.0/access-binary-trees` | 1.017839 [0.992572, 1.042617] | 1.044673 [1.044595, 1.045024] |
| `external/kraken-1.1/audio-dft` | 0.998317 [0.962750, 1.039040] | 0.999280 [0.999115, 1.000167] |
| `external/jetstream3-js-subset/cdjs` | 1.020521 [1.011182, 1.038217] | 1.021628 [1.020707, 1.022036] |

Obvious diagnostic losses: `development/moment/dates`, `external/sunspider-1.0/access-binary-trees`.

Stage seconds: `{"analysis": 2.6012365110218525, "build_base": 0.8538243359653279, "build_candidate": 71.39746749401093, "build_quieting": 0.029960704036056995, "collection": 173.7137321140617, "command_total": 258.39542118494865, "end_to_end": 319.01619839668274, "prepare_workloads": 0.1759841569000855, "queue_quieting": 60.623090505599976, "seal_and_replay": 8.061155706061982, "settle_and_launch": 1.2230148219969124, "validate_and_freeze": 0.15983520005829632}`.

| Role | Job | Exit | Log SHA-256 |
| --- | --- | --- | --- |
| base-counts | `20261023T135308Z-t072-base-counts` | `done 0` | `75c15de67a40e92f16230eebed11975d35ac0c94260c10e3bc1d82537f41b983` |
| counts | `20261023T143013Z-t072-design2-counts` | `done 0` | `3baa03df1173df83423942e68fa1842a01fb6c29c0a6496b310e184f1aa6a759` |
| reference | `20261023T143015Z-t072-design2-reference` | `done 0` | `561238bd033431b47de3bffc0d636fdb3c9244d2a4fab27af9e5e10ef4199646` |
| triage | `20261023T143016Z-t072-design2-triage` | `done 0` | `ccce6f8c875805c14cab79be8ce66f8e7ff8faa6e888599ecf31e65ce3a393e3` |

## Diagnostic counters before and after

These binaries use `perf-counters` and are never timed. All ten stdout checks and both terminal-outcome/receiver partitions pass in each collection. Entry counts remain identical across all three sources. Scoped comparisons/hashes count actual text Eq/Hash work while a named handler is active; pointer guards are not text comparisons. Hot handlers enter only after success, as documented by the reused T065 instrumentation.

| Program | Stage | Entries | Compares / entry | Hashes / entry | Hot % | Probe % | Own lookup % | Non-Object % |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| `development/big.js/arith` | base | 661841 | 0.348594 | 0.033815 | 15.615 | 3.944 | 0.036 | 72.638 |
| `development/big.js/arith` | design 1 | 661841 | 0.344278 | 0.033815 | 15.705 | 3.944 | 0.036 | 72.638 |
| `development/big.js/arith` | design 2 | 661841 | 0.344278 | 0.033815 | 15.765 | 3.884 | 0.036 | 72.638 |
| `development/immutable/persistent` | base | 9344130 | 1.562699 | 0.231268 | 46.920 | 23.096 | 2.594 | 16.678 |
| `development/immutable/persistent` | design 1 | 9344130 | 1.506214 | 0.213394 | 47.366 | 24.883 | 0.807 | 16.678 |
| `development/immutable/persistent` | design 2 | 9344130 | 1.506214 | 0.213394 | 66.176 | 6.073 | 0.807 | 16.678 |
| `development/js-yaml/document` | base | 14364546 | 0.373919 | 0.368843 | 41.605 | 3.604 | 34.447 | 20.196 |
| `development/js-yaml/document` | design 1 | 14364546 | 0.025283 | 0.024424 | 44.626 | 35.157 | 0.009 | 20.196 |
| `development/js-yaml/document` | design 2 | 14364546 | 0.025283 | 0.024424 | 78.984 | 0.800 | 0.009 | 20.196 |
| `development/graphlib/paths` | base | 4816956 | 0.290082 | 0.041985 | 45.878 | 40.831 | 2.552 | 7.843 |
| `development/graphlib/paths` | design 1 | 4816956 | 0.171325 | 0.020656 | 48.296 | 42.684 | 0.699 | 7.843 |
| `development/graphlib/paths` | design 2 | 4816956 | 0.171325 | 0.020656 | 90.336 | 0.644 | 0.699 | 7.843 |
| `development/lodash/transform` | base | 2901542 | 0.019659 | 0.014100 | 4.240 | 47.083 | 0.003 | 48.643 |
| `development/lodash/transform` | design 1 | 2901542 | 0.018702 | 0.014095 | 4.244 | 47.083 | 0.003 | 48.643 |
| `development/lodash/transform` | design 2 | 2901542 | 0.018702 | 0.014095 | 51.326 | 0.001 | 0.003 | 48.643 |
| `development/diff/compare` | base | 16823970 | 7.056019 | 0.076624 | 5.105 | 54.315 | 4.369 | 12.740 |
| `development/diff/compare` | design 1 | 16823970 | 5.249127 | 0.032941 | 13.354 | 58.683 | 0.001 | 12.740 |
| `development/diff/compare` | design 2 | 16823970 | 5.249127 | 0.032941 | 48.609 | 23.428 | 0.001 | 12.740 |
| `development/moment/dates` | base | 27761662 | 4.732291 | 0.417536 | 22.634 | 38.397 | 0.364 | 13.185 |
| `development/moment/dates` | design 1 | 27761662 | 4.093260 | 0.360542 | 24.270 | 38.397 | 0.364 | 13.185 |
| `development/moment/dates` | design 2 | 27761662 | 4.093260 | 0.360542 | 55.587 | 7.080 | 0.364 | 13.185 |
| `development/meriyah/parse` | base | 8629184 | 1.552740 | 0.936179 | 4.805 | 10.162 | 67.989 | 8.242 |
| `development/meriyah/parse` | design 1 | 8629184 | 0.710171 | 0.227424 | 8.449 | 75.398 | 2.753 | 8.242 |
| `development/meriyah/parse` | design 2 | 8629184 | 0.710171 | 0.227424 | 80.404 | 3.442 | 2.753 | 8.242 |
| `development/matter-js/simulate` | base | 45415974 | 0.102279 | 0.101960 | 35.545 | 52.182 | 10.175 | 2.093 |
| `development/matter-js/simulate` | design 1 | 45415974 | 0.000470 | 0.000235 | 36.122 | 61.775 | 0.004 | 2.093 |
| `development/matter-js/simulate` | design 2 | 45415974 | 0.000470 | 0.000235 | 97.591 | 0.307 | 0.004 | 2.093 |
| `development/papaparse/csv` | base | 4377829 | 5.197151 | 0.481668 | 18.014 | 10.246 | 0.005 | 61.062 |
| `development/papaparse/csv` | design 1 | 4377829 | 3.935589 | 0.481643 | 18.014 | 10.246 | 0.005 | 61.062 |
| `development/papaparse/csv` | design 2 | 4377829 | 3.935589 | 0.481643 | 28.260 | 0.000 | 0.005 | 61.062 |

| Stage | Diagnostic executable SHA-256 | Identity SHA-256 |
| --- | --- | --- |
| base | `4e87683e988e67399554f63ebd12e4fc5a6e3d927f5cfbfb6d6a8be88ca5ce99` | `2ebb3204fbbfa0ef29ab8c75f45f22081f507378d3aacaceb57ac3de24aa2e40` |
| design 1 | `d39a21b3956639ebe47b89509571d519c47ee1f74e958fd88f4a003a5de25b5d` | `4f817ee9639f076bfcfbf409efc586acefbefeea1d9943a2cdac08a6f6f96e07` |
| design 2 | `eaf5cd21b0f50e9310856132723449be2c563d8c4b96986514c42e035fb7d4a0` | `7600fa967511b145cdfa2e0208131a196a7b2af719641e5b872390303845ea2e` |

## Final candidate and screen ledger

Final candidate `6c122407` passes its commit hook (2,395 runtime tests and 65 selected Test262 cases), full gate (2,395 runtime, 21 agents, 26 diagnostic, 732 benchmark-tool, 35 script tests and 5,170 curated Test262), all 257 NG fixtures, and all ten diagnostic stdout/partition checks. New tests prove hot own/literal cache guards, current values, descriptor changes, namespace rejection and actual-key intrinsic capture; earlier compact/VM reentrancy coverage remains.

Design 2 meets the prospective screen trigger: wall 0.949774025573 <=0.97, correctness checks pass. Moment wall 1.076441 / instructions 0.997068 and binary-trees instructions 1.0447 remain reported diagnostic losses; the tool label is not relabeled as a pass.

- `20261023T143036Z-t072-design2-export`: exit 0; authenticated result/counter/reference export.
- `20261023T143906Z-t072-screen-attempt1`: exit 1 after 1 s preflight; caller omitted `--workers 4`, so default workers 1 fails the frozen schedule check before building/timing. No valid attempt consumed.
- `20261023T144103Z-t072-screen-attempt1-r2`: exit 1 / 1,134 s; explicit workers 4 and content-checked trigger; natural collection aborts on a load-valid short increment. Attempt 1 closes incomplete; one of two attempts is charged, one remains unused. No candidate or policy change.

## Screen health failure and collection stop

All twelve variants built and passed static audits: base six at `6e899d24`,
candidate six at `6c122407`. Natural executable hashes equal the triage images:
base `cbb471c7047917f02e13d0ecf83516d9d6e57c050f2170ddde3b5e6e17ed735e`,
candidate `f6d5220992f3dd7d05afaad4af2e169a01258294e7995347d28f81e4ca74a5e3`.
There are 68 natural-case collection starts in the log; shard 3 stops at
`broad/local_read`. No complete natural panel, six-layout panel or qualifying
screen verdict exists. No subset geomean is substituted for the missing run.
Three light intruder samples are below the qualified disturbance threshold;
no DISTURBED warning and no load-invalid pair explains the failure.

The failed raw packet calibrates N=23,068,672. All five pair indexes are
load-valid. Candidate N/2N wall measurements and increments are:

| Pair | N wall ns | 2N wall ns | Increment ns | Instruction increment |
| --- | ---: | ---: | ---: | ---: |
| 0 | 265227103 | 520914425 | 255687322 | 4547405685 |
| 1 | 501623129 | 521621365 | 19998236 | 4544952953 |
| 2 | 266976091 | 524236151 | 257260060 | 4547328958 |
| 3 | 264039974 | 524948408 | 260908434 | 4547340790 |
| 4 | 266188241 | 519016033 | 252827792 | 4547244308 |

Pair 1's 19,998,236 ns increment is below the unchanged 50,000,000 ns minimum.
Its N process uses 490 ms user CPU and 501.623129 ms wall; 2N uses 510 ms user
CPU and 521.621365 ms wall. The other four candidate increments are about
253-261 ms. This observation does not establish a root cause or attribute it
to the property optimization, OS scheduling, code placement or clock drift.

The [qualified recovery contract](../../../docs/layout-policy-artifacts.md#load-only-pair-recovery)
says: “A valid short/nonpositive increment fails instead of being replaced.”
`pair_recording.py` likewise makes short/nonpositive accepted increments fatal.
No sample is dropped, substituted, recollected or relabeled load-invalid.
The attempt remains incomplete; no formal, held-out, peak RSS or host-resource
run is launched without the required qualifying screen. One unused screen
attempt is reserved for the lead's review of a new justified proposal; this
record does not reset the budget or make a promotion/rejection decision.
The known process/layout-variance campaign [T038](../../T038-audio-dft-variance.md)
is a research route, not a diagnosis of this sample.

- `20261023T150847Z-t072-screen-export`: exit 1; correctly refuses to export
  a nonexistent complete screen from the failed job.
- `20261023T151036Z-t072-screen-failure-diagnosis`: exit 0; retains the
  failed raw packet/JSONL, both six-image identities, original log and load log.
- Screen job log SHA-256: `533ad53e337d3b9d28668cf127e31156f898a46fe78c6796656e9311fb2fbe0c`.
- Failed packet SHA-256: `f416ccec4ad97983d64c89d5e556e9521f6834af778580df9a8d70524ad62090`.

## Complete exact conformance at the final runtime

The unchanged harness runs `find-qjsng-gaps.sh --exact --all --no-recommend`
from a scratch copy under this worktree, with copied pinned Test262
`4249661388e5d3f92a85186213da140a6481490f` test/harness directories and pinned
NG `f7830186043e4488f2998759d60a514faf07cbc9`. Original reference directories
are not edited. The local arm64 release/agents build contains runtime
`6c122407`; subsequent commits change documentation only. Eight case workers,
30-second timeout; no timing claim or held-out input.

Every one of the 53,572 pinned corpus paths occurs exactly once; its path set
is checked against the original corpus. NG configuration skips 10,900 paths.
All **42,672 configured Rust cases pass**, with fail/timeout/not-run/actionable
gap counts zero. NG passes 42,602 and has its standing 70 failures, no timeout.
`test262-aggregate.py --require-complete-parity` passes. The complete scan's
`local-exact-agents` record for `6c122407` is appended to the frozen burndown
JSONL; no prior record changes.

| Asset | SHA-256 |
| --- | --- |
| Local release/agents CLI | `254dd9ff8c88f98bc597aa59238a4aae5f15bf7c8fb952348a8ddda9baecdafa` |
| Local NG CLI | `cfd8386c3c29b1125a878b8fb82f9627820f2dcc16d2a691c5f8c16ad0b047a0` |
| Complete cases | `2263183d8df546cb37338205c107cc4614a7ddee069a711be6181c0e27c2f61a` |
| Scan summary | `cffdf394963ec66c41d006192593e9975e6b082a55483748550b7a9fb866e80f` |
| Burndown entry | `d49efc0cc709e1662fb6bcdad233ac0d7d15c8b8892d55ae6876c48f041905a5` |
| Exact scan log | `f94abdab46017877c8ce1cb25e8446f264e76ded375c6e98f74b8f8ffaf4b84c` |

## Handoff boundary

The implementation remains present on the owner branch at `6c122407`; it is
not promoted or integrated. No origin push, merge, new dependency, unsafe,
benchmark/policy/recipe change or call/environment rewrite occurs. The unused
attempt is not a license for favorable reruns. The lead should review the
valid-short-increment failure before authorizing any new acceptance path.
Remote branch CI is unrun because branch publication was forbidden.

Task-external stale text: the T069 row of
[the plan index](../../performance-units/README.md) still says lead review is
pending although the T069 task opens with its promotion decision. This unit
updates only its own index row.

## Final repository verification

Final `TEST262_DIR=<shared pinned corpus> ./scripts/check.sh` passes after
the complete conformance record and handoff documentation are prepared:
all feature/default clippy and tests, 2,395 runtime tests, 732 benchmark-tool
tests, 35 script tests, context/size/hooks and 5,170 curated Test262. Runtime
source remains exactly `6c122407`; subsequent changes are records only.
Final full-gate log SHA-256: `c806bb8391c28bbc5ca723db3e41a6dbd2f887212afeeab2de47287e5fdb078e`.
The original shared Test262 and NG tracked trees also report clean status.

Branch descent/path validation passes against frozen base `6e899d24`; every
changed file stays in the declared property/diagnostic/test/document boundary.
Final cleanup runs `cargo clean` (65,416 files / 7.5 GiB removed), then
`git checkout -- target` restores the tracked rejected-screen fixture.
The worktree is clean and free disk is 43 GiB. Cleaned binaries/raw scratch
are not rebuilt; only this handoff note follows the final full gate, with
the documentation fast hook. All T072 remote jobs are finished. No origin push
or merge occurs.
