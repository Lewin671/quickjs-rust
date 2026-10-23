# T071: Review verification of lexical context ownership

- Status: historical-campaign; owner correctness review and requested verification complete, performance adjudication pending.
- Implementation: present in runtime `2fc8efb8` (review cells/ownership `6b4db3ba`, real arguments `17f9ebb7`, final one-pass demand scan `2fc8efb8`); fixture correction `da9251f7`.
- Verified at: final runtime `2fc8efb8`, 2026-10-23; fast/full gates (2,412 runtime tests, 5,170 Test262 subset cases), 261 fixtures, independent module comparison and all five complete area scans pass.
- Evidence: this record and [original T071](T071-lexical-context-gaps.md).
- Unresolved: none in requested correctness findings; triage on earlier `6b4db3ba` is likely loss, and final runtime performance remains unmeasured.
- Next action: lead performance adjudication; no more owner timing or acceptance chain without a new request. No push or merge.

## Review scope and design

Base for this review: `71f081ea`; same owner branch/worktree as original T071.
Only real function arguments owners may materialize an implicit slot; arrows,
keys, eval and module/static/field contexts inherit or omit it. Capture
selection must distinguish referenced outer bindings from owned block/catch
slots. Eval var/function declarations may shadow received outer cells without
conflicting with or mutating those outer bindings. Keep indexed shared cells
without adding per-call name snapshots. Call setup and dynamic frame cell
attachment need ownership checks, so the original brief's one diagnostic
triage will be run on the final candidate (no acceptance chain).

Stages: focused reproductions and bytecode probes (including `6e899d24`);
compile-time ownership/capture fixes; declaration regressions and any needed
cold eval fix; fast/full gates and before/after comparisons; update evidence.

## Verification

Baseline job `20261023T141436Z-t071-review-before`, at `71f081ea`, repeats all
five original areas with `find-qjsng-gaps.sh --filter <area> --all`.
Lead reports performance job `20261023T135507Z-t071-lead-triage` neutral,
instructions within +/-0.2% everywhere; this is lead-supplied evidence, not
an owner rerun or a claim about the eventual repaired binary.

## Confirmed findings and repair

1. Module/global direct eval reproduces the spurious arguments TDZ. Compilation
   now carries explicit arguments ownership: only actual functions materialize
   arguments slots; arrows/keys/static blocks use lexical/no ownership. This
   also covers nested arrows, field contexts and function eval arguments.
2. Exact base `6e899d24` probe: captures/cells/received slots all empty,
   `creates_capturing_closures=false`. Reviewed `71f081ea`: captures
   `[("value", 0)]`, parent cell slots `[0]`, received slots `[0]`, flag true.
   Capture discovery now retains received cells, not every owned local spelling.
   A compiler regression asserts no capture/cell/flag for block and catch shapes.
3. The simple reported eval expression returns 5, but a sibling reader proves
   sloppy eval var/function declarations mutate the captured outer binding.
   A prior-read probe at `71f081ea` also reproduces the var-conflict SyntaxError.
   Received cells are not own lexical conflicts. Eval declarations now allocate
   distinct cells in the activation's variable map, with original-cell identity
   provenance for already-created closures. Each eval-capable function forks
   inherited mappings once at call entry; generator resumes retain that map.
   Parameter/body shadows split maps and cells, preserving existing tests.

The direct-eval declaration helper was extracted into `global/eval_bindings.rs`
because `global.rs` was already 1,996 lines; no dependency or public API added.
Cold declaration metadata reuses the existing optional Rc context handle.
The eight-way var/function, sloppy/strict, first/prior-read matrix passes, with
outer binding unchanged; additional tests cover pre-existing closures, deeper
shadowing, bare var, owned lexical conflicts, outer const shadows, suspension,
module/global eval/static/field boundaries and real enclosing arguments.

Historical probe sources and builds are temporary under target; their exact
results and revisions above are retained here, not inferred from HEAD.

## Jobs and fixture correction

- Baseline `20261023T141436Z-t071-review-before`: exit 0, 353 s at
  `71f081ea`; all five original areas pass the same 9,226 Rust cases, no
  timeouts or NG-pass/Rust-fail gaps. NG's two class-statement syntax failures
  remain reference-side baseline differences. Log SHA-256:
  `de14b2e97adea657c77d00749656f38269a7b63d9abda81faf6c3aef1dae0882`.
- `20261023T143955Z-t071-review-reference`: exit 1, 20 s before area scans.
  The new script fixture reused `outer` for a loop block function and a later
  factory; sloppy Annex B rebinding made the factory return a callable instead
  of an array. Renamed the loop factory `makeCase`; no runtime code changed.
  Corrected local outputs are the eight var/function cases followed by
  `5:7:5:1|5:5:1`; the module outputs four `undefined` values.
- `20261023T143957Z-t071-review-triage`: exit 0 at runtime `6b4db3ba`,
  base `6e899d24`, original five lodash/immutable/diff/graphlib/moment targets.
  The command is one single-quoted queue argument, with "$BENCH_OUT/triage".
  This is the only owner diagnostic triage; no acceptance chain.

## First repaired runtime gates

At runtime `6b4db3ba`, `check-touched.sh --staged --explain` and the full
`check.sh` both exit 0. The full gate used the requested external TEST262_DIR,
ran all three clippy configurations, feature and workspace tests (2,409
runtime), 732 benchmark-tool tests, 35 script tests, plan/context/size/hook
checks and all 5,170 curated Test262 cases. The focused fast gate ran 117 cases.
The fixture-only `da9251f7` commit changes no runtime Rust code.

- Full log SHA-256: `6963aeb18f4765f9037a6f1f48724c26810f9d4f260eff0f2b97100d723f8294`.
- Fast log SHA-256: `6633ac94f47e9bf48d2088f2451b4b4c6d0cb2946d21e5ff5f3bc6a14f5f03e8`.
- Base capture probe log: `9a3801137af2498f92c231ff5e25b71221aa840e9053ad6a63909780cb5aa101`.
- Reviewed capture probe log: `3e33092c749ce8ebc44a6c58c6208e28328a1b5a5bffa4db7196b5c369859fd5`.
- Reviewed prior-read probe log: `ecf696d90b17bb2f0ee04db40844d5fa9e3ab1df01d9067ad5be592441446aac`.

## Sole performance diagnostic

Job `20261023T143957Z-t071-review-triage`, base `6e899d24` versus runtime
`6b4db3ba`: **likely loss**, 105/105 valid pairs, zero invalid/heavy samples
and no intruder events. Command 261.1 s; end-to-end including 60.6 s quieting
321.7 s; build base 0.85 s, candidate 71.23 s, collection 176.20 s. Budget met.
No plan attempt was spent, no acceptance chain run, and the lead's older neutral
triage is not a claim about this repaired binary.

All ratios below are candidate/base (lower is better), natural layout with
five pairs and exploratory 95% intervals. They establish no formal precision,
complete-portfolio or layout robustness claim. The recursive-call-tree wall
flag is obvious in the diagnostic while its instruction count is nearly
unchanged; this does not identify its cause. Possible flags also include
capturing-closure calls, prototype-method calls and object allocation. The
largest instruction increase in the fixed controls is binary trees (1.0237).

| case | wall ratio [95% exploratory interval] | instructions [95% exploratory interval] |
| --- | ---: | ---: |
| development/lodash/transform (target) | 0.9861 [0.9234, 1.0588] | 1.0005 [1.0002, 1.0009] |
| development/immutable/persistent (target) | 0.9854 [0.9138, 1.0074] | 1.0044 [1.0043, 1.0045] |
| development/diff/compare (target) | 0.9505 [0.9388, 1.0000] | 0.9975 [0.9975, 0.9976] |
| development/graphlib/paths (target) | 1.0192 [1.0117, 1.0481] | 1.0038 [1.0037, 1.0039] |
| development/moment/dates (target) | 1.0023 [0.9861, 1.0259] | 1.0018 [1.0018, 1.0018] |
| sentinel/capturing_closure_call | 1.0086 [0.9705, 1.0642] | 1.0000 [1.0000, 1.0001] |
| sentinel/heterogeneous_property_read | 0.9917 [0.9494, 1.0133] | 1.0000 [0.9998, 1.0001] |
| sentinel/polymorphic_call_site | 0.9970 [0.9151, 1.0377] | 1.0000 [0.9999, 1.0001] |
| sentinel/prototype_method_call | 1.0214 [0.9069, 1.3731] | 1.0007 [1.0006, 1.0011] |
| sentinel/recursive_call_tree | 1.1462 [1.1144, 1.1655] | 1.0001 [1.0000, 1.0002] |
| sentinel/string_key_map_churn | 1.0041 [1.0019, 1.0195] | 1.0000 [0.9998, 1.0002] |
| broad/property_dynamic_read | 0.9739 [0.9305, 0.9788] | 1.0049 [1.0047, 1.0049] |
| broad/object_allocation | 0.9942 [0.9726, 1.0880] | 1.0000 [1.0000, 1.0001] |
| broad/string_slice | 1.0011 [0.9841, 1.0079] | 1.0000 [1.0000, 1.0001] |
| broad/top_level_function_call | 0.9887 [0.9138, 1.0404] | 1.0000 [0.9999, 1.0003] |
| external/kraken-1.1/json-stringify-tinderbox | 1.0035 [0.9873, 1.0359] | 1.0005 [1.0001, 1.0013] |
| external/kraken-1.1/json-parse-financial | 0.9923 [0.9612, 1.0039] | 0.9999 [0.9998, 1.0000] |
| external/jetstream3-js-subset/raytrace-public-class-fields | 1.0187 [1.0046, 1.0228] | 1.0133 [1.0132, 1.0133] |
| external/sunspider-1.0/access-binary-trees | 1.0068 [0.9937, 1.0079] | 1.0237 [1.0236, 1.0239] |
| external/kraken-1.1/audio-dft | 0.9949 [0.9730, 1.0397] | 1.0002 [1.0001, 1.0003] |
| external/jetstream3-js-subset/cdjs | 1.0102 [0.9938, 1.0232] | 1.0135 [1.0132, 1.0139] |

Identities:

- Request SHA-256: `e4c306db1b71ef508720e4b7e90ca1cfa4e70da4774c5c7a61a4174dab0ece3a`.
- Result SHA-256: `17e7be2ddea54587171bdd8a7d31a18c199ca006533a7cb01f686539bebfd005`.
- Base image: `cbb471c7047917f02e13d0ecf83516d9d6e57c050f2170ddde3b5e6e17ed735e`.
- Candidate image: `119921e8043766c41c7b2d2014a1a15b54eb10c2a4b96bdc62ca5b703fc7283f`.

Raw remote artifacts are fetched under workspace `outputs/imac-bench/<job>/`;
this record retains their results independently. Raw local logs/probe builds
are temporary and do not carry unrecorded evidence.

## Final real-owner arguments-cell refinement

A positive control after `da9251f7` found `function f(value) { return eval(
"class C { #p; [(#p in {}, arguments[0])]() {} } typeof new C().method;"); }
f('method');` throws ReferenceError in `6b4db3ba`. Suppressing phantom slots
must not remove the real function binding when its only reader is eval code.

Ordinary function compilation scans argument demand once before lowering,
stopping at ordinary-function and class-body boundaries, and reserves the slot
before parameter/body lexicals. It does not retry lowering nested functions
or clone compiler/runtime frames or binding values. Its slot is a
function-var binding (not a lexical conflict), and shared dynamic cells preserve
reassignment for escaping arrows created in an eval's deferred key. Modules,
eval code, arrows/static/field/key thunks still do not invent a binding.
Two new tests cover direct reads, later reassignment and body lexical/function
shadows; the syntax scan also has explicit owner-boundary tests. The original body-scope var-arguments
regression also passes; newest full/fast gates and exact references are recorded below.

The one diagnostic measurement above predates this compiler/cell refinement.
Its adverse result remains retained; there will be no second owner timing run
or acceptance chain under the original one-triage instruction. Final performance
is therefore unverified and is a lead decision, not a neutrality claim.

## Newest runtime local gates

Final engine code `2fc8efb8` replaces the intermediate `17f9ebb7` compiler
retry with a bounded syntax pass. All 2,412 runtime tests pass, including 13
new review tests. The final fast gate runs 65 selected Test262 cases; the
review's earlier function/context changes ran 117. The full gate exits 0 with
all 5,170 curated cases and all other standard stages. Newest local logs:

- Full SHA-256: `e0f4300fda9a724c0be31e0743a59afe8e6a9d6750657642f9cdacf5f0c1fe2c`.
- Fast SHA-256: `74bf1498a7a94d3e7d46c6d01abdf005f6f9d2153e8f464747410f21dfde9cb2`.

Intermediate `da9251f7` references (`20261023T144210Z-t071-review-reference-final`)
and `17f9ebb7` references (`20261023T152223Z-t071-review-arguments-reference`)
both completed successfully: 261 fixtures, one independent module comparison
and the complete 9,226-case five-area scans. Exact newest job `20261023T154307Z-t071-review-scan-reference` at `2fc8efb8`
completed in 356 s with exit 0. No extra timing job was submitted after the one measured candidate.

## Exact final reference results

Job `20261023T154307Z-t071-review-scan-reference` checked out
`2fc8efb8bb22ab1e4f046b53a13ea12bc0edf34a`. All 261 comparison fixtures
pass. The independent `.mjs` file prints
`undefined:undefined:undefined:undefined` on both Rust and NG; output files
compare byte-for-byte. The real-owner arguments fixture returns
`undefined:undefined:undefined:8:function:changed` on both engines, including
the positive eval-created key and reassigned arguments-cell cases.

Each area ran `find-qjsng-gaps.sh --filter <area> --all` (exact requested range):

| Area under test/ | Before `71f081ea` Rust pass | Final `2fc8efb8` Rust pass |
| --- | ---: | ---: |
| language/expressions/class | 4,049 | 4,049 |
| language/statements/class | 4,351 | 4,351 |
| built-ins/eval | 10 | 10 |
| language/eval-code | 347 | 347 |
| annexB/language/eval-code | 469 | 469 |
| Total configured | 9,226 | 9,226 |

No Rust failure, timeout or not-run case; no NG-pass/Rust-nonpass gap.
The same two reference-side statement syntax failures and 26 configured
feature skips remain as in the baseline. No unfiltered full scan or burndown
entry is claimed. No runtime code changed after this verified revision.

- Final reference log SHA-256: `6ffa5e3024ee83994d524716d06d482b3d546268f6da12f4ef4875e07b2a1891`.
- Identical module output SHA-256: `f4331078fecf05dc8449e34579b8587bae4bcf8cf1d3951ed619185a4ae88f52`.

## Handoff and cleanup

`cargo clean` removed 22,498 files / 11.1 GiB, including both historical
probe source/build trees. `git checkout -- target` restored the tracked fixture
(target directory 4 KiB). Final scope and document-only fast checks run before
the evidence commit, without rebuilding cleaned binaries. No origin publication, merge or
performance acceptance/promotion was performed.
