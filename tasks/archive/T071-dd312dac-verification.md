# T071: dd312dac narrowed-source verification

- Status: historical-campaign; narrowed-source correctness and diagnostic evidence.
- Implementation: present in `dd312dac`; eval-only capture/declaration experiment and unnecessary parent receiver slots removed.
- Verified at: `dd312dac`, 2026-10-23; fast/full, 2,403 runtime tests, 117 focused and 5,170 curated Test262 cases, 260 NG fixtures, module and five-area comparisons pass; final-version triage complete.
- Evidence: [narrowing attempts and triage](T071-narrowing-evidence.md), [original work](T071-lexical-context-gaps.md), [prior review](T071-review-verification.md).
- Unresolved: no known new correctness regression; final triage unclear and ordinary instruction growth is not fully flat. T074 scope gaps remain outside this task.
- Next action: follow the active [T071 record](../T071-lexical-context-review.md).

## Scope narrowed by lead, 2026-10-24

Remove eval-only outer capture and declaration shadowing instead of point-fixing
the eval environment model. Compiler capture selection, call/env/global eval
and frame binding sources are restored to `6e899d24`. Removed dynamic_names,
call-entry fork_cells, outer-binding provenance, owns_eval_var and declaration
shadowing helpers. Removed tests/fixtures requiring original bug 4 to be fixed.
[T074](../T074-direct-eval-scope-model.md) lists every known scope gap/constraint;
its next action is an architecture design reviewed before implementation.

Keep deferred keys lexical for this/arguments/new.target/home/super, with class
private authority and the shared derived-this cell. Only real arguments owners
reserve the canonical binding before parameter/body lexicals; lexical thunks
forward that cell, including eval-created keys. No arbitrary eval-only names
are captured. No new dependency or ordinary call/frame executor changes.

Keys use existing receiver preparation and do not manufacture a parent this
slot. This fixes both the review's unfilled module slot and the discovered
module eval/static-key receiver overwrite. The original combined module fixture
is retained; reordering keys to hide the overwrite was rejected. With no extra
parent receiver slot, baseline try/catch cleanup suffices; the exemption is
removed. Existing derived-this and source-arguments cleanup tests still pass.

## Coverage and boundaries

- Deferred-key escaping arrows, mutable arguments/shadows, target, static and
  instance super, private environments and derived-this before/after super.
- P1-a live inherited eval map: var/function declarations, generators, nested
  rebinding. P1-b outer const/TDZ cell integrity; main's missing TypeError stays
  T074. P1-c module/global receiver and absent arguments, including the combined
  module eval/static-key regression (two key forms).
- Original `outer(8)()` remains unfixed; no new test expects its wrong result.
  The key eval test's field initializer uses explicit `value`; its eval-only
  field capture belonged to the removed bug-4 experiment.
- Full before scan at `6e899d24`: five areas, 9,226 configured Rust passes,
  26 feature skips, no Rust fail/timeout/not-run or NG-pass/Rust-nonpass. Details,
  intermediate gates/failures, exact probes and pre-final triage are archived.

The `9adf5662` triage was unclear, with 105 valid pairs and near-flat ordinary
instructions; its full 21-row table, limits, timings and artifact identities
remain in the linked evidence. Receiver repair changed engine source after
that measurement; those numbers are not final-version performance evidence.

## Final correctness verification at dd312dac

Fast/full gates exit 0: formatting, all clippy/features/workspace tests,
2,403 runtime tests, 732 benchmark Python tests, 35 script tests, plan/context/
size/hook/shell checks, 117 focused and 5,170 full curated Test262 cases.
Scope validator against `66e500a7` passes. Call/env/global eval, frame bindings
and try/catch cleanup are byte-identical to main `6e899d24`; static capture
selection matches main (test added only). No dependencies or cross-crate APIs.

Queue `20261023T173428Z-t071-narrow-final-reference`, pinned `dd312dac`, exits
0 in 349 s: all 260 script fixtures match NG; combined module outputs are
byte-identical: `undefined:undefined:undefined:undefined` then `function:function`.
Five complete filtered --all scans before (`6e899d24`) and after (`dd312dac`):

| Area (`test/`) | Rust pass before / after | NG pass before / after |
| --- | ---: | ---: |
| language/expressions/class | 4,049 / 4,049 | 4,049 / 4,049 |
| language/statements/class | 4,351 / 4,351 | 4,349 / 4,349 |
| built-ins/eval | 10 / 10 | 10 / 10 |
| language/eval-code | 347 / 347 | 347 / 347 |
| annexB/language/eval-code | 469 / 469 | 469 / 469 |

Each side has 9,226 configured Rust passes and 26 feature skips; zero Rust
failures, timeouts, not-run or NG-pass/Rust-nonpass. NG's two pre-existing
statement grammar failures are unchanged. These are complete filtered ranges,
not full-suite scans, so no burndown record is appended.

Artifact SHA-256 (results remain here after local cleanup):
- Fast log: `e46de237808a5fc526542b472e1a64cb010d4a5d9518b773ae7c5f26ec67c9a3`.
- Full log: `e60950309fe325911884e52fc2585978ce2e9584d3c07c9a6330a6aa89486f57`.
- Reference log: `deb442a5a62fb8dcbb65dc7956ed19fade7c7425657b1736568b55a72aad5467`.
- Both module outputs: `8b839ca4974d1086d3d9105322e1372606c8d146d0053541eb6b65527c295ac8`.

Final-version quick triage queue `20261023T174036Z-t071-narrow-final-triage`
fixes candidate `dd312dac`, base `6e899d24`, the same five development targets
and all fixed controls. It starts only after all correctness checks pass,
with an iMac 02:45-05:00 exclusion and 30-minute start margin. The earlier
pre-final `9adf5662` run remains archived and is not substituted for this result.

## Final-version quick triage at dd312dac

Queue `20261023T174036Z-t071-narrow-final-triage`, base `6e899d24`, exit 0,
verdict **unclear**. 105/105 valid pairs; zero invalid or heavy samples; one
light background sample below the disturbance threshold. Budget met, zero plan
attempts consumed, no obvious regressions. Possible wall regressions:
graphlib/paths, moment/dates, polymorphic_call_site, prototype_method_call,
string_key_map_churn and top_level_function_call.

Ordinary instructions are not exactly flat: binary-trees +0.27%, raytrace
+0.35%, cdjs +0.37%, property_dynamic_read +1.15%. Development targets range
+0.15% to +0.92%. These costs remain unadjudicated; do not claim codegen
neutrality or performance acceptance. No tuning or acceptance chain follows.
One final-version trial ran only after correctness passed. The earlier trial
on rejected `9adf5662` is preserved separately, not substituted or pooled.

Time: iMac 2026-10-24 01:41:37-01:46:01 CST, outside 02:45-05:00.
End-to-end 324.16 s includes 60.71 s queue quieting; command 263.43 s,
base build 0.86 s, candidate build 71.06 s, collection 179.27 s.
Ratios below are candidate/base, lower better. Five natural-layout pairs;
95% intervals are exploratory, unadjusted, without layout robustness or a
complete-portfolio claim. All 21 rows are retained:

| case | wall ratio [95% exploratory interval] | instructions [95% exploratory interval] |
| --- | ---: | ---: |
| development/lodash/transform (target) | 0.9930 [0.9575, 1.0357] | 1.0036 [1.0029, 1.0038] |
| development/immutable/persistent (target) | 0.9681 [0.9200, 0.9901] | 1.0092 [1.0091, 1.0092] |
| development/diff/compare (target) | 0.9807 [0.9290, 1.0534] | 1.0026 [1.0025, 1.0026] |
| development/graphlib/paths (target) | 1.0124 [0.9936, 1.0531] | 1.0078 [1.0078, 1.0080] |
| development/moment/dates (target) | 1.0202 [1.0180, 1.0518] | 1.0015 [1.0015, 1.0016] |
| sentinel/capturing_closure_call | 0.9809 [0.9323, 1.0164] | 1.0000 [1.0000, 1.0001] |
| sentinel/heterogeneous_property_read | 1.0205 [0.9784, 1.0484] | 1.0000 [0.9999, 1.0001] |
| sentinel/polymorphic_call_site | 1.0619 [0.8585, 1.2496] | 1.0000 [0.9999, 1.0001] |
| sentinel/prototype_method_call | 0.9709 [0.9500, 1.0509] | 1.0008 [1.0007, 1.0008] |
| sentinel/recursive_call_tree | 0.9795 [0.9613, 1.0017] | 1.0000 [0.9999, 1.0000] |
| sentinel/string_key_map_churn | 1.0151 [0.9806, 1.2691] | 1.0000 [0.9999, 1.0001] |
| broad/property_dynamic_read | 0.9933 [0.9855, 0.9963] | 1.0115 [1.0114, 1.0115] |
| broad/object_allocation | 0.9955 [0.9732, 1.0045] | 1.0000 [0.9999, 1.0000] |
| broad/string_slice | 0.9399 [0.8099, 0.9831] | 0.9999 [0.9998, 1.0000] |
| broad/top_level_function_call | 1.0214 [1.0100, 1.0945] | 1.0000 [0.9999, 1.0001] |
| external/kraken-1.1/json-stringify-tinderbox | 0.9986 [0.9787, 1.0040] | 1.0003 [0.9989, 1.0030] |
| external/kraken-1.1/json-parse-financial | 0.9794 [0.9323, 0.9805] | 1.0052 [1.0051, 1.0053] |
| external/jetstream3-js-subset/raytrace-public-class-fields | 1.0080 [0.9898, 1.0116] | 1.0035 [1.0035, 1.0036] |
| external/sunspider-1.0/access-binary-trees | 0.9902 [0.9782, 1.0007] | 1.0027 [1.0021, 1.0028] |
| external/kraken-1.1/audio-dft | 1.0000 [0.9841, 1.0395] | 1.0002 [0.9987, 1.0006] |
| external/jetstream3-js-subset/cdjs | 0.9915 [0.9855, 1.0078] | 1.0037 [1.0032, 1.0041] |

Immutable final triage SHA-256:
- Request: `8186e64ce87696900bf1c7415b66072ac12c68b748ff571185d1a1eb499b251a`.
- Result: `ee3f5bf2a350e2e805040d4ee0530addacac516decc88b4ae0d8a5561b6c95fa`.
- Markdown: `c4b8bf0a2585046604b01b0f4123bb6c9e94f7526e56280c7dfe76b53467378f`.
- Base image: `cbb471c7047917f02e13d0ecf83516d9d6e57c050f2170ddde3b5e6e17ed735e`.
- Candidate image: `0a0a9b012b87336ce1c82856eae3a1460653b62f13a757ff86ed18c7763ac4bb`.

Final cleanup after gates: `cargo clean` removed 8,906 files / 6.0 GiB;
`git checkout -- target` restored the tracked 4 KiB fixture. No later Rust
rebuild. Final documentation-only fast/context gate and branch scope pass;
no origin push or merge. No generated build artifacts enter the commits.
