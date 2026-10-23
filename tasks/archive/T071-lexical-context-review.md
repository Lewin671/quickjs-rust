# T071: Deferred class-key context ordinary-code cost review

- Status: closed 2026-10-24 — parked by the lead, not integrated (decision below).
- Implementation: present in `a7581aa9`; actual-key lowering, root-slot finalization and retained arguments cell replace global AST planning.
- Verified at: `a7581aa9`, 2026-10-23; fast gate, 2,410 runtime tests, ordinary IR/frame fingerprints, fast/full, 117 focused/5,170 curated Test262, 261 NG fixtures, module and five-area checks pass; sole iMac triage complete with likely loss.
- Evidence: [dd312dac correctness and triage](T071-dd312dac-verification.md), [flat correctness iterations](T071-flat-correctness-iterations.md), [earlier narrowing](T071-narrowing-evidence.md), [original work](T071-lexical-context-gaps.md), [phase probe](../diagnostics/t071-flat/README.md).
- Unresolved: 11/21 instruction point estimates exceed +0.2%; ordinary x86 runtime costs remain despite base-identical IR. General eval scope gaps remain [T074](../T074-direct-eval-scope-model.md).
- Next action: none here; deferred-key lexical context is part of [T074](../T074-direct-eval-scope-model.md)'s design.

## Lead decision (2026-10-24)

Three repair rounds each traded one problem for another: eval-capture regressions (removed), then +0.3–1.2% ordinary
instructions, now base-identical IR but x86 runtime instructions still up on 11/21 cases (string_key_map_churn +3.1%).
The defects are rare, pre-existing and outside Test262; ordinary code must not pay for them. Per the architecture-first rule,
the lead parks this candidate and folds deferred-key lexical context into T074's design (how nested thunks and direct eval
name enclosing frame context under the slot/cell model), to be reviewed before code. The branch is retained for reuse.

## Scope and implementation

Keep deferred keys lexical for this/arguments/new.target/home/super, class
private authority and shared derived-this. No parser or general eval scope
changes. The call binding model stays slot-indexed with shared upvalue cells.
Receiver preparation still adds no synthetic parent this slot; retain the
combined module eval/static-key regression without reordering.

Delete the global per-function AST demand pass. Ordinary functions retain
baseline slot order, flags, bytecode and frame plans. Actual deferred-key
lowering records compiler-only demand; lexical arrows return that demand to
their owner. Finalize the rare owner's implicit arguments slot before its
parameter/body slots and remap the root operations and parent capture indices
once, removing that now-hoisted binding from previously emitted try cleanup
lists. Child bytecode retains its own indices. No new runtime frame metadata,
additional ordinary-function compilation, or second AST analysis traversal.

Only eval sources that actually introduce keys and require the enclosing
arguments binding may lower a second time after checking actual cell presence.
The existing dynamic eval view passes that single cell back to its declaring
frame through a private frame-metadata alias. An explicit call-context-cell variant
in the existing frame-binding value enum lets later dynamic views retain it; only the already-forked eval view overlays its arguments entry.
Ordinary dynamic value forks perform no alias lookup. Caller dynamic maps
retain their identity and see later added names. No
caller-name snapshot or general declaration ownership model is added.
Ordinary eval bytecode and cache admission are preserved.

The five complete baseline IR/frame fingerprints cover plain parameters,
one and two nested arrows, direct eval, and a method arrow using this. Slot
permutation coverage includes rest/destructuring/object-rest locals, with,
class method/private/public-field captures, catch cleanup, per-iteration
cells, generator delegation, pre-key arguments assignment and escaped arrows.
An incidental no-key block-arguments eval-shadow fix from the removed prepass
is reverted with ordinary IR; its independent baseline gap is recorded in
T074 item 13, without asserting the wrong result in a test.

## Instruction attribution

Arm64 attribution uses identical public parse/compile/eval boundaries in
release diagnostic examples built from exact `6e899d24` and `dd312dac`.
A parent reads live proc_pid_rusage instruction counts while the child waits
at phase barriers (no Rust FFI or production instrumentation). Three paired,
interleaved samples per input. The runtime example uses the standard allocator,
not the CLI recipe; these medians are attribution, not timing/promotion evidence:

| Case | Base compile | dd compile | Ratio | Base execute | dd execute | Ratio |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| immutable/persistent | 183,273,896 | 186,010,839 | 1.014934 | 21,017,401,142 | 20,998,890,125 | 0.999119 |
| graphlib/paths | 32,559,871 | 32,807,817 | 1.007615 | 8,997,040,159 | 9,005,204,412 | 1.000907 |

Checksums match (`2fd9ba00`, `e845d023`). Parse ratios vary 1.008982 and
1.003145 despite unchanged parser source. Thus this local split demonstrates
compile overhead and small execution differences but does not quantitatively
explain every iMac delta. Bytecode provides independent structural evidence:
`function make(x){return()=>x+1}` changes x slot 0 -> 1, introduces arguments
at slot 0 instead of slot 1, marks arguments hoisted and changes its initial
route Frame -> FrameOrUndefined. Plain direct-eval functions gain a slot that
base never emitted; nested arrows also change received-upvalue routes.

## Final verification

Exact engine source is `a7581aa9`. The final comparison is pinned to that
source even if later commits only record results. No rejected-source timing
was run. Reproductions and rejected cell-map ownership handling are archived;
current code retains the inherited dynamic map and only forwards arguments.

After the typed retained-cell refinement, three paired arm64 samples at
`a7581aa9` give these medians (same diagnostic API boundaries and allocator):

| Case | Base compile | Final compile | Ratio | Base execute | Final execute | Ratio |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| immutable | 182,689,679 | 182,687,219 | 0.999987 | 21,015,567,573 | 20,993,391,119 | 0.998945 |
| graphlib | 32,587,014 | 32,626,540 | 1.001213 | 8,996,657,891 | 8,992,074,830 | 0.999491 |

Checksums still match. Parse ratios are 1.011020/0.998382; unchanged parser
source and separate process layouts limit attribution. Final compile growth
is 0.00%/0.12%, execution -0.11%/-0.05%; this local result does not replace
the queued x86_64 whole-process triage or establish performance acceptance.

Final queue `20261023T192337Z-t071-flat-final-code`, exact `a7581aa9`, exits 0
in 355 s. All 261 script fixtures and the combined module outputs match NG:
`undefined:undefined:undefined:undefined`, then `function:function`.
Before is exact `6e899d24` from the archived record; after is `a7581aa9`:

| Area (`test/`) | Rust pass before / after | NG pass before / after |
| --- | ---: | ---: |
| language/expressions/class | 4,049 / 4,049 | 4,049 / 4,049 |
| language/statements/class | 4,351 / 4,351 | 4,349 / 4,349 |
| built-ins/eval | 10 / 10 | 10 / 10 |
| language/eval-code | 347 / 347 | 347 / 347 |
| annexB/language/eval-code | 469 / 469 | 469 / 469 |

Each side: 9,226 configured Rust passes, 26 feature skips, zero Rust
fail/timeout/not-run or NG-pass/Rust-nonpass. NG's two existing grammar
failures are unchanged. Filtered complete ranges, not a complete-suite scan;
no burndown entry. No dependencies, cross-crate API or ordinary frame fields.

Recorded SHA-256 identities (conclusions and numbers retained after cleanup):
- Fast log: `4690361f4642aab058ec5e8d786efa2492fa39d3b4c7e21d8eb000d7b509f79f`.
- Full log: `985f1404947cab5caf129b36cd2010b34f5599e3fb0b5d76120d4fbdd4d3ca30`.
- Final reference log: `eef8ce8fff36303d483bbb7935fd9e2984ccb5781158a05dc936438e76408b8b`.
- Both module outputs: `8b839ca4974d1086d3d9105322e1372606c8d146d0053541eb6b65527c295ac8`.
- Initial phase raw counts: `bf1992a18e208544a3f98dac3f4073d0e6838d499b2eadd4d3134b461ab45d1e`.
- Final phase raw counts: `d1d35df8ed390049e3a9aa849093f993fd8c5ee77313334face5f5d60d828966`.
- Base probe image: `8be0f5ba1b683c05c8233ce52e0a6a300dbb684af251d8b807b3b4c3bba350d4`.
- dd probe image: `b5bbbac9c07cf7672c98c0d3a1cb995cd452ada95bf371d3e49ec0629dc57f2f`.
- Final probe image: `1d630adead8ec161b50ea760a662747ff48e5e0db025bd307b45766b0bf9eef0`.

## Sole final-source iMac triage: target missed

Queue `20261023T210010Z-t071-flat-triage`, exact `a7581aa9` versus
`6e899d24`, exits 0; verdict **likely loss**, not performance acceptance.
All 105 pairs valid; zero invalid/heavy samples and no outer host intruders.
Budget met, zero plan attempts consumed. Policy, producer, all 21 case IDs and
prepared workload hashes match the dd trial; base image is byte-identical.
Fetched request/raw checksums validate. No second triage or tuning follows.

**11/21 instruction point estimates exceed +0.2%; 10/21 meet +/-0.2%.**
Range is -0.1685% to +3.0875%. All five development targets miss the target.
String-key churn crosses the tool's 1.03 instruction loss threshold, with
95% interval [1.030719, 1.030990]. Possible regressions: lodash/transform,
capturing_closure_call, heterogeneous_property_read, polymorphic_call_site,
prototype_method_call, recursive_call_tree, string_key_map_churn and
property_dynamic_read. These flags use the tool's joint wall/instruction
heuristic; the stricter +/-0.2% instruction target is checked separately.

Ordinary IR/frame-plan identity and arm64 phase counts did not establish x86
neutrality. Ordinary runtime/cell representation or machine-code costs need
further attribution; this trial does not identify a specific instruction
site. The actual-key typed cell remains in the retained source. Do not claim
that deleting the AST prepass removed every ordinary-code cost.

Submitted at iMac 2026-10-24 05:00:10 CST, admitted 05:35:40 after another
30-block queue job; request created 05:36:41, completed about 05:41:02.
All timing is after 05:00. Stages: 321.52 s including 60.68 s quieting,
260.84 s command, 0.83 s base build, 71.29 s candidate build, 176.26 s
collection; waiting behind the other job is excluded from this budget.
Five natural-layout pairs; 95% intervals below are exploratory/unadjusted,
without layout robustness or a complete-portfolio claim. All 21 rows follow;
ratios are candidate/base and the final column checks the instruction point.

| Case | Wall ratio [95% interval] | Instructions [95% interval] | +/-0.2% |
| --- | ---: | ---: | :---: |
| development/lodash/transform (target) | 0.9952 [0.9446, 1.0708] | 1.002357 [1.001220, 1.002407] | no |
| development/immutable/persistent (target) | 1.0095 [0.9540, 1.0145] | 1.002993 [1.002869, 1.003050] | no |
| development/diff/compare (target) | 0.9677 [0.9637, 1.0514] | 1.002274 [1.002251, 1.002362] | no |
| development/graphlib/paths (target) | 1.0111 [0.9914, 1.0467] | 1.006052 [1.005857, 1.006196] | no |
| development/moment/dates (target) | 1.0077 [1.0066, 1.0186] | 1.005639 [1.005629, 1.005640] | no |
| sentinel/capturing_closure_call | 1.0417 [0.9972, 1.3839] | 1.004624 [1.004532, 1.004883] | no |
| sentinel/heterogeneous_property_read | 1.0381 [0.9821, 1.0510] | 1.000073 [0.999939, 1.000161] | yes |
| sentinel/polymorphic_call_site | 1.0031 [0.9826, 1.1556] | 1.004843 [1.004701, 1.004941] | no |
| sentinel/prototype_method_call | 1.0031 [0.9638, 1.1642] | 1.003094 [1.003050, 1.003111] | no |
| sentinel/recursive_call_tree | 0.9864 [0.9739, 1.1863] | 1.000070 [0.999996, 1.000227] | yes |
| sentinel/string_key_map_churn | 1.0252 [0.9648, 1.0777] | 1.030875 [1.030719, 1.030990] | no |
| broad/property_dynamic_read | 0.9959 [0.9595, 1.1348] | 0.998315 [0.998258, 0.998453] | yes |
| broad/object_allocation | 0.9837 [0.9646, 1.0095] | 0.999970 [0.999865, 1.000098] | yes |
| broad/string_slice | 1.0116 [0.8653, 1.0357] | 1.010243 [1.010081, 1.010324] | no |
| broad/top_level_function_call | 0.9878 [0.9122, 1.0139] | 1.003468 [1.003439, 1.003536] | no |
| external/kraken-1.1/json-stringify-tinderbox | 0.9960 [0.9800, 1.0045] | 1.000223 [0.999960, 1.001059] | yes |
| external/kraken-1.1/json-parse-financial | 0.9872 [0.9462, 1.0170] | 1.000172 [0.999889, 1.000274] | yes |
| external/jetstream3-js-subset/raytrace-public-class-fields | 1.0099 [0.9818, 1.0204] | 1.000078 [1.000024, 1.000088] | yes |
| external/sunspider-1.0/access-binary-trees | 0.9649 [0.9295, 0.9960] | 1.001120 [1.000008, 1.001179] | yes |
| external/kraken-1.1/audio-dft | 1.0047 [0.9609, 1.0194] | 0.999939 [0.998208, 1.000408] | yes |
| external/jetstream3-js-subset/cdjs | 1.0008 [0.9926, 1.0089] | 1.001992 [1.001509, 1.002130] | yes |

Final triage SHA-256:
- Request: `2ad5fd57169ccd990264a87923d6ea670f67171b6f42c09f65bfc04d975c138b`.
- Result: `1aa672bc1757ae5886999ab00113f32d210b9fd7b2ca852d62c5e698239f6557`.
- Markdown: `da2aca503a8b15ca9155127817c0f67631fc07a44301c2b3f846c1c3287a14b0`.
- Base image: `cbb471c7047917f02e13d0ecf83516d9d6e57c050f2170ddde3b5e6e17ed735e`.
- Candidate image: `7a70654852fd2d58a639370c5fbe49b14391f2b15404109dd3883ea72a9e0812`.

Local cleanup after correctness and phase diagnostics: `cargo clean` removed
24,214 files / 9.8 GiB; `git checkout -- target` restored the tracked 4 KiB
fixture. No later Rust rebuild. Source remains `a7581aa9`; subsequent commits
record results only. No generated artifacts in commits, origin push or merge.
