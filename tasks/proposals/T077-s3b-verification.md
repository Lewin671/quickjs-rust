# S3b frozen verification matrix and exit

Part of the [frozen brief](T077-s3b-brief.md). This freezes obligations, not test
results. Audit source `71294a20`; rebind to accepted S2 before coding. All 13
independent [T074 snippets](../T074-direct-eval-scope-model.md) are additional
acceptance cases; configured Test262 standing alone does not establish them.

## T074 permanent discriminators

Each row requires lowest-layer route/identity assertions plus a permanent runtime
test and Rust/NG comparison fixture. Use the exact independent snippet in T074;
wrap outputs/exceptions explicitly, never use a main wrong result as expectation.
New script fixtures use `s3b-scope-<row>-*.js`; module/context fixtures go under
`tests/fixtures/compare-modules/`. Avoid amalgamating independent name scopes.

| Item | Frozen result and additional assertions |
| --- | --- |
| 1 Eval-only capture | `outer(8)()` = 8 after outer return; sibling ordinary/eval writes use one ptr_eq cell, no retained slot value. |
| 2 Live parent additions | Var/function and suspended-generator variants = 5; nested rebinding = `[2,2]`. Same parent holder before/after child entry and later outer declaration; child without eval also sees later shadowing. |
| 3 Outer const/TDZ | Const eval write throws TypeError; uninitialized lexical read/write throws ReferenceError. TDZ remains uninitialized after failed Assign; later Init succeeds once. |
| 4 Module key context | Private-brand/String(this) key completes; module this initialized undefined. Module/global scripts and direct eval preserve absent arguments and global receiver; no derived-this error. |
| 5 Own eval declaration | Var/function inner/outer = `[2,1]`; ordinary parameters, preexisting closures, prior reads, same own redeclaration and strict eval isolation. Own cell identity stable, parent cell untouched. |
| 6 Deletion | Eval-created var delete = true, later typeof undefined; static var delete = false. Delete own reveals parent on fresh lookup; preselected reference retains old cell. |
| 7 Mapped arguments | Eval arguments[0] write makes parameter 2; reverse parameter write visible; duplicate parameters, missing/excess actuals, delete, accessor/nonwritable descriptors sever only the specified mappings. Strict/non-simple unmapped. |
| 8 With initializer | Result `[1,2]`. Proxy HasProperty/get/unscopables trace in spec order; selected assignment reference precedes RHS effects and is consumed once, including throws and mutations. |
| 9 Same-frame const | Eval Assign throws TypeError, both static/cell/dynamic routes; Init remains distinct, including immutable class/self/import policies. |
| 10 Key suspension | Generator = `['key','function']`, async prints function. Same enclosing activation/lease/lexical owner before and after yield/await; private/derived/finally/completion state survives. |
| 11 Lexical arguments | Escaping arrow returns 3 after initialization; call before Init throws ReferenceError despite implicit arguments availability. Parameter/body/block shadow boundaries remain distinct. |
| 12 Global nested arrows | `typeof arguments` = undefined in a fresh realm; ordinary read throws unresolvable ReferenceError. Arrows with parameter-eval declarations select that own scope, never invent TDZ. |
| 13 Exited block | Later direct eval returns typeof outer arguments = object; active cursor excludes exited block and any stale lexical cell, including abrupt exit. |

## T071 exact fixture and test recovery manifest

Read historical sources with `git show`; do not cherry-pick removed runtime
experiments. Base already has `class-deferred-key-call-context.js` and all
`derived-eval-*.js`; keep them. The following exact fixture names at `a7581aa9`
(flat iterations) and `dd312dac`/`2fc8efb8` (review) must be audited/recovered as
tests, even where absent at the implementation base. Revalidate against NG;
historical passing counts are not current results.

| Fixture / source (relative to tests/fixtures/) | Mandatory coverage / oracle |
| --- | --- |
| compare-qjs/arguments-context-boundaries.js | Global/eval/nested arrows/static/fields absent arguments, real eval-only arguments owner, escaping arrow reassignment. Review output `undefined:undefined:undefined:8:function:changed`. |
| compare-qjs/class-key-lexical-context.js | Private brands, lexical this/arguments/new.target, static/instance super, escaped arrows, field keys, argument shadows and derived-this before/after super. Compare full output with NG. |
| compare-qjs/class-key-slot-finalization.js (`a7581aa9`) | Default/destructuring/rest plus with, private fields, generator/yield*, try cleanup reading arguments after catch, direct arguments assignment, escaped ordinary function seeing later eval var, reassignment. Compare full output with NG. |
| compare-qjs/eval-scope-boundaries.js | Live inherited var/function declarations and nested rebinding; const/TDZ integrity. Strengthen previously permissive baseline checks to T074's required errors. |
| compare-qjs/direct-eval-outer-cells.js and eval-declaration-shadowing.js (`2fc8efb8`) | Review capture/declaration discriminators, prior reads/parameters/preexisting closures; use approved own-holder behavior, reject map-fork/value-writeback experiments. |
| compare-modules/deferred-key-arguments.mjs | Combined module fixture in original order: `undefined:undefined:undefined:undefined`, then `function:function`; eval/static key must not overwrite module receiver. |

Historical runtime tests: recover `tests/direct_eval/{outer_bindings,declarations}.rs`
from `2fc8efb8`, `tests/direct_eval/scope_boundaries.rs` from `dd312dac`, and
their registration. The four
scope-boundary tests are `inherited_eval_maps_observe_declarations_after_closure_creation`,
`nested_eval_maps_share_later_rebindings`, `eval_only_assignment_does_not_mutate_an_outer_const_cell`,
and `eval_only_assignment_does_not_initialize_an_outer_tdz_cell`.
Existing `tests/{parameters,closure_state,functions,classes,class_eval,class_fields,
class_constructors,class_super,modules}.rs` and `tests/direct_eval/derived_this.rs`
remain required: defaults/TDZ, rest/mapped arguments, named self/class immutability,
class private/home/receiver/target, field early errors, module live cells and
derived binding order. Include `block-lexical-native-call.js` and
`default-parameters.js` plus S3a's native slice/recycler/constructor regressions.

`compare-qjs.sh` runs scripts only; it does **not** execute .mjs fixtures.
Run each recovered/new module with Rust and NG module entry modes (Rust CLI
`--module <file.mjs>`; NG `-m <file>`, TZ=UTC), compare exit status/stdout including job
completion; record commands and hashes. Add module runtime tests so full check
also protects them. Register new test modules centrally through the core owner.

## Stress modes (new modes are implementation obligations)

New mode names below are frozen debug controls, **not existing commands**.
Capture once per RuntimeServices; reject/ignore debug forcing in production
timing binaries. Same semantic outputs and cell identities in every mode.

| Mode / configuration | Coverage |
| --- | --- |
| `QJS_VERIFY_BINDINGS=1` | At promotion/routes/Init/Assign assert exactly one authoritative slot or cell, matching metadata and received order; no observable read replay. Debug full runtime + all permanent fixtures. |
| `QJS_FORCE_EVAL_EXPOSURE=1` | Promote all user-visible active declaratives at scope entry, including TDZ; compare default lazy exposure. Never promote compiler temps. Debug full runtime + fixtures; include closures with no eval and early/late eval. |
| `QJS_POISON_RELEASED_SCOPES=1` | Poison/clear released private slots, generations and existing recyclable vectors, force reuse via churn; escaped leases/cells, arguments mappings and saved references survive. No poisoning of live cells. Debug full runtime + fixtures. This does not introduce S3c pooling. |
| S2 `QJS_DISABLE_IC=1` | Complete configured exact/NG run with semantic property paths; capture modes must not rely on ICs. |
| S2 `QJS_VERIFY_IC=1` | Debug fixtures/runtime: cached vs pure uncached outcomes without replaying hooks, with native/eval/callback mutations. |
| Combined binding modes + IC off | All permanent T074/T071 cases and lowest-layer identity tests; each individual mode also passes the complete runtime suite. |
| default / `agents` / `perf-counters` | Runtime unit tests and crate/CLI builds; both aarch64 and x86_64 production targets for changed layouts and borrowed ABI. Agent/jobs preserve context and can-block identity. |
| Ordinary debug and release | S3a depth probe at accepted S2 base and candidate: every base-success maximum/landmark succeeds for forEach/map/reduce/call/apply/Reflect.apply/yield*/callback recursion. No arbitrary new depth guard. |

Stress cases cross strict/sloppy and same/cross realm where valid: promotion
before/after callback writes and throw, eval on cold/warm cache, empty eval and
early/late declarations, with getter/Proxy/unscopables mutating scope, mapped and
unmapped arguments, catch/finally/loop exit, returned closures, private/derived
and new.target ownership. Suspend generator/yield*, async/await, computed class
keys and module TLA; resume after outer return and storage churn. Test pending
throw/return/finally and native re-entry restoration. No callbacks under borrows.

## Frozen exit and evidence

1. All rows above pass; every method and consumer in the inventory has a typed
   replacement/deletion. No CallEnv/alias, execution name/value projection,
   apply-back, empty-env factory or caller-env equality proof remains. Current
   tiers use callee Context; dormant later-stage machinery is explicitly scoped.
2. Staged fast gate and `./scripts/check.sh` pass with default/agents/perf-counters
   coverage; all NG script fixtures and separate module comparisons pass.
   Run focused eval/class/parameter/arguments/with slices during commits, then
   `./scripts/find-qjsng-gaps.sh --exact --all --no-recommend --out-dir <dir>`
   on the frozen production binary with default ICs and again IC-off.
   Zero configured fail/timeout/not-run, zero gaps/lost passes/new exclusions;
   record a burndown entry for **each** complete scan, never partial scans.
3. Ten development programs reproduce source hash/stdout/full counter maps twice
   per base/candidate. Count new typed entries, exposure/cells/leases, old env
   construction/fork/writeback/equality events (must be zero), S3a probes/copies
   (remain zero). Separate pool requests from actual allocation and retained
   owners. No instrumentation in production timing binaries.
4. Lead freezes candidate SHA/binary/corpus/recipe and collects formal development,
   external, equal-case selection and sealed held-out against accepted S2 using
   T077 precision/health rules: **all four <1**, with cumulative rebuild-baseline
   and NG ratios reported. Per-case/lane and RSS 1.10 worst/1.05 geomean are
   reports under T077, not replacement vetoes. Missing/unhealthy evidence fails
   acceptance; no held-out profiling/tuning or T022 5%/finite-attempt substitution.
5. Record source/binary/commands/hashes, correctness totals, deletions, counter
   numbers, aggregate results/health and unresolved issues in durable diagnostics
   and T077 resume block. Update actual env/architecture/suspension docs at
   implementation, preserving S3c/d/e ownership. Lead alone accepts/integrates.

This design submission runs context/diff/staged documentation checks only; no
runtime, complete exact or wall/RSS standing is claimed at the design revision.
