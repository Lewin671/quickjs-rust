# T056 M2 plan inputs: ordinary entry before the shared driver

- Status: historical-campaign; feedback-1 measured pricing complete; original two-target draft is not ready to freeze.
- Implementation: no runtime on this plan branch; diagnostic indexed-entry prototype `9dafdebe` and counter audit `232196d5` stay on their separate scratch branch.
- Verified at: exact M versus `9dafdebe`, 2026-10-22 UTC; full prototype gate, cohort/instruction/profile receipts and diagnostic limits below. Plan-only checks apply to this evidence delta over `93ff53ab`.
- Evidence: this record, the proposal and [earlier exact-M receipts](T064-successor-plan-inputs.md); raw artifacts are workspace-local, not retained in Git.
- Unresolved: graphlib cold/self/lexical contexts remain unpriced; wall/layout/RSS acceptance and strict kernel decoder coverage are unverified. Projections are conditional, not measured wall gains.
- Next action: lead reviews the measured immutable margin and graphlib shortfall; do not freeze or automatically retarget this draft. Preserve old budgets and the separate diagnostic prototype.

## Decision and scope

Start with **M2.1 indexed entry and typed ActivationContext**, within the
existing **five-stage migration**, not a leaf or a new driver-first order.
[T056](../T056-interpreter-core-v2.md#ordered-migration) and
[frame activation](../../docs/design/frame-activation.md) own the stage contracts.
This record supplies measurements and the proposed numeric contract, not replacement architecture.

The lead's brief reports K isolation stopped after outlining raised object-heavy
instructions 1.4–1.6x, and directs effort to M2. This is the brief's routing
instruction, not a new measured verdict on the frozen K plan. Its failed
receipts and old M1 accounts remain untouched. No previous candidate result
selects the M2 targets. No held-out program was inspected.

Choose **graphlib/paths and immutable/persistent** for final cumulative payoff.
Graphlib combines high entry cost with the tight floor band; immutable has
substantial entry **and** Vm cost. Js-yaml, diff, moment and meriyah stay
controls after comparing reach to their actual bands, rather than inheriting
M1's target list. Every other policy case is a control: **84**, including all
six sentinels and eight other development programs. Held-out remains nine
acceptance-only programs. A lower-ranked graphlib target requires the proposal's
explicit priority override; validation does not approve that override.

M2.1 removes the largest directly attributable *first* cost among independent
entry, Vm and compact-driver work. Across the two targets, equal-case entry
reach is **10.31%**, versus **7.92%** Vm/prepared-call and **2.90%** sampled
compact-boundary wrappers. Dynamic consumers have smaller reach and require
entry/context first. Driver-first would leave name assembly intact and repeat
T021's ownership-first mistake; M2.3 and M2.4 must follow their prerequisites.
The unseparated instructions inside wide `run` are not a fifth removable bucket.

## Exact input identities

M = `9eba7a1d6ae8b67397d20e0d6c93ac79037f5e30`; executable SHA-256
`f5822aa32e4c7a3def2aaf5b8eff2107ba005a76252afb25f06077fe25f2e8eb`.
The branch starts at `d1e80fec`; no runtime/build differences from M.
Queue = `tmp/qjs-lead-briefs/current-main-9eba7a1d-queue.json` under the workspace,
SHA-256 `f6666a68cb8e1841418d6f5012e6f7810b0c25ebf982b45ef6d5564855e0ae13`.
The selection formal is `20261022T041140Z-current-main-9eba7a1d-formal`,
workers-4, 30 blocks, summary SHA-256
`205e882999604170609e6e46e98e6c83091604bb9f9b6b2da0b322f817a41bc5`.
[T040's standing](../T040-imac-general-performance.md#current-measured-standing-2026-10-21)
records development Rust/NG geomean 3.071 and js-yaml 4.34x, immutable 4.01x,
moment 3.65x, diff 3.62x and meriyah 3.34x. These are opportunity sizes, not
M2's removable fractions.

Fresh existing receipt job **J0** = `20261022T050553Z-t064-successor-exact-profiles`;
its 27-window manifest/hashes and collection protocol are recorded in
[T064 inputs](T064-successor-plan-inputs.md#receipt-manifest), not duplicated here.
Re-read those exact copied `profile.sample` files for the narrower M2 buckets.
J0 lacked graphlib. Historical [M2 diagnostics](T056-m2-design-inputs.md) and
[unit 0 part 2](T056-unit0-part2-evidence.md) justified filling that gap, but
supply no current target fraction.

After `imac-bench.sh status` showed all prior jobs done, submitted **J1** =
`20261022T072707Z-t056-m2-plan-graphlib-exact` via the workspace script:
`run --timing --ref 9eba7a1d --name t056-m2-plan-graphlib-exact -- '<collector>'`.
It reused J0's collector with only the case list changed to graphlib. No SSH,
cancellation, local/release build, benchmark screen or formal timing was run.
The runner's `--timing` reserves a quiet host for sampling; elapsed metadata is
not performance evidence. J1 is **done 0**, three independent processes,
`sample <pid> 10 1 -mayDie -file <raw.sample>`, every process/sampler exit 0,
every manifest checksum valid, empty `intruders.log`. Each process finished by
sampler return. Queue regenerated from the same sealed formal is byte-identical.

Artifacts are under `outputs/imac-bench/<job>/` in the workspace. J1 hashes:

| Artifact | SHA-256 |
| --- | --- |
| `command.sh` | `142a1d63b1031d64ee39d3d5f96415fe70d1ab8b4755b75f1622fd53365ef820` |
| `intruders.log` | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |
| `out/selection.json` | `f88ced45fdc724e970bd2129548ccacadf280548845c98d0adfea38fbbfe7423` |
| `out/host.json` | `b6c7c3b696ba08e08e0d05a23f81c0c653baf544b61cab2753254a0c56fb2976` |
| `out/profiles.json` | `2258c027df3e3b65f9440c598c2dfbd4b29100e27ffd95a1dd688a242a2a96c5` |

J1 receipt paths are `out/profiles/development--graphlib--paths/<repeat>/receipt/receipt.json`.
Workload SHA-256, identical in all three:
`87d777c0409dea06f66052bfa763379f6370cdad05a97301f79dd7e8fbddadaf`.

| Repeat | Receipt SHA-256 | Copied profile SHA-256 |
| --- | --- | --- |
| 0 | `1f3e44ea5e2249e9697bca3ccfa8f17ebf1704a7e683e906a7b5092f29fc5282` | `6c41b2986ee228860ff7e0c5f419a2eb1dd7f60f1ac3d9ef43b91aaafa4d7dd0` |
| 1 | `04b8e574dcfa6228a190ac9e56380d1125acb92f488a069e67c5f47af8d6ee10` | `56491813c009dc1659a7634a40ce4835bb2357e1fb69002b05a3e4d6134e2592` |
| 2 | `45b6a49b61a9d64eefb4fd9789b3576b97bc89af2f169bfdce6ebff5a3f961dc` | `777a695ac8067caea42aaa206f49844c9cdf191c2c1f8f379a52c3d318014829` |

`tools.benchmark.profile.verify_profiles` content-verified **30/30** J0/J1
receipts and copied assets against the queue's exact M image. The proposal
binds all three windows of each target (six receipts); its profile root is
workspace `outputs/imac-bench/`, allowing both jobs without copying/re-importing.

## Attribution method and limitations

Parse only the first main-thread call tree; depth is the count's column in
`^([ +!:|]*)(\d+) (.+)$`. Stop at a non-tree line or the next root. Subtract
direct children from each node to get self samples; assert nonnegative and
sum(self) = thread total. Each self sample is charged once to the nearest
semantic engine frame. Recognize `qjs_runtime` symbols, skipping `_RIN*` and
`_RNvX*` generic/trait helpers and `5value(?:8teardown|.*5Value)` or
`6string.*Clone5clone` glue; recognize `CallEnv` drop/Clone explicitly before
those skips. Allocator/hash/mem*/Value glue is thus charged to its semantic
caller. This is a deliberately narrower partition than J0's broad env/Vm/call
groups; do not add the two methods' percentages.

Classify that owner by the first matching row below (Rust mangled substrings):

| Bucket | Matching expression / special case |
| --- | --- |
| E: entry/name plumbing | `function_env\|new_function_frame\|FrameBindings\|apply_env\|current_env\|callee_env\|frame_call_env\|refresh_realm_backed\|pop_arguments\|Bytecode10local_slot\|can_seed_slot_backed_call\|cell_received_upvalue_index\|insert_frame_cell\|initial_slots`, or explicit CallEnv drop/Clone |
| D: residual dynamic/slot work | `DynamicBindings\|CallEnv[0-9]+get$\|load_local_slow\|store_local_slow\|clear_local\|load_global` |
| V: Vm/prepared-call | `new_with_globals\|with_frame_bytecode\|eval_function_bytecode\|run_prepared_bytecode_call\|eval_direct_call_bytecode` |
| C: visible compact-boundary wrappers | `activation.*(?:inline_entry\|clear_window\|call_from_activation\|exit_to_interpreter\|try_run_in_caller_env)\|frame_stack.*run_completion` |
| Remainder | wide-run, Vm-run and all other owners; no payoff credited |

Also compute E's inclusive **union** by charging a self sample once if any
ancestor matches E (CallEnv drop included; Clone-only special case excluded).
The plan's `inclusive_fraction` uses this union, **not** E+V. Never sum union
and owner columns. Pool counts, not percentages, across all three windows.

E is an entry-related **reach envelope**, not proven M2.1 savings: it contains
required parameter/this/cell work and nonordinary entries. D mixes M2.1's
link-time local classification and M2.2 consumers; V includes setup retained
until later stages. C includes real slow-callee/exit work and misses inlined
transitions inside `run`. Neither wrapper-inclusive callee time nor entire
wide/Vm dispatch self time is credited. Cohort counters and release PC audits
are required to sharpen this; no sample zero proves absence of a path.
The short suite samples are particularly weak for coverage. J0 moment windows
end before the whole process, as its collection record explains.

Local derivation script `tmp/qjs-lead-briefs/m2-analyze.py` SHA-256
`f52dbf17b5d3fed2caa7525ea054440bd1f5dd144302d4bcde86cf0368bed2a7`;
summary `m2-reach.json` SHA-256
`92df7e27ce48b01b043d8a0cdb5ce7d168f2d4d7f74224807127516f2df9ede9`.
These are unsealed local derivatives, not receipts. The algorithm, masks,
counts and conclusions here are the durable evidence.

## Fresh candidate reach and price

Percentages are of pooled sampled main-thread time. B is the policy wall band;
P = min(.97, 1/B). E/V/D/C are disjoint under the method above, but each still
contains semantic work that remains. Ranks are queue lane-local.

| Candidate | Rank / Rust:NG | Samples | E count / % | V % | D % | C % | B | P / required gain | Role |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| development/graphlib/paths | 6 / 2.8875 | 3639 | 408 / 11.21 | 5.11 | 2.45 | 2.97 | 1.050127431 | .952265383 / 4.77% | target |
| development/immutable/persistent | 2 / 4.0096 | 10695 | 1006 / 9.41 | 10.73 | 2.95 | 2.82 | 1.076242823 | .929158345 / 7.08% | target |
| development/js-yaml/document | 1 / 4.3374 | 12432 | 908 / 7.30 | 3.10 | 1.61 | 2.21 | 1.050127431 | .952265383 / 4.77% | control |
| development/diff/compare | 4 / 3.6178 | 18244 | 2287 / 12.54 | 2.07 | 5.74 | .88 | 1.099109904 | .909827121 / 9.02% | control |
| development/moment/dates | 3 / 3.6504 | 24836 | 1363 / 5.49 | 2.17 | 2.71 | 1.84 | 1.050127431 | .952265383 / 4.77% | control |
| development/meriyah/parse | 5 / 3.3434 | 7310 | 296 / 4.05 | 2.97 | 2.11 | 2.12 | 1.071894924 | .932927265 / 6.71% | control |
| external/jetstream3-js-subset/hash-map | 7 / 1.5026 | 1489 | 11 / .74 | .00 | .00 | 3.49 | 1.050127431 | .952265383 / 4.77% | control |
| external/jetstream3-js-subset/raytrace-public-class-fields | 24 / 1.0691 | 2165 | 2 / .09 | .00 | .00 | 2.63 | 1.050127431 | .952265383 / 4.77% | control |
| sentinel/heterogeneous_property_read | 1 / 1.4232 | 675 | 0 / .00 | .00 | .00 | .00 | 1.050127431 | .952265383 / 4.77% | mandatory control |
| broad/property_read | 1 / 5.1528 | 1305 | 0 / .00 | .00 | .00 | .00 | 1.050127431 | .952265383 / 4.77% | control |

Lodash/transform is **not ranked in this queue**: historical-base comparison
is absent. Its old 25.5% env diagnosis is not a current receipt or eligibility.
Big.js, matter-js and papaparse remain unprofiled controls, not rejected targets
based on invented current reach. No broad/sentinel target is selected by a
large NG gap when its ordinary entry path is unobserved.

Why these two: required removal of **E+V** is graphlib **29.24%**, immutable
**35.17%**, versus js-yaml **45.90%**, diff **61.73%**, moment **62.33%** and
meriyah **95.58%**. This is a reach/payoff comparison, not permission to delete
those fractions. Immutable's wider band is affordable from its much larger V
share. Js-yaml has a tighter band but less entry/Vm reach and 20.83% string/RegExp
ownership in J0. Diff's high E alone does not excuse its much stricter price.

Target E counts by repeat: graphlib **136/1207, 134/1217, 138/1215**;
immutable **326/3572, 341/3568, 339/3555**. E inclusive union is respectively
**216/1207, 189/1217, 212/1215** and **383/3572, 415/3568, 407/3555**.
Pooled E unions: **16.96%** graphlib and **11.27%** immutable. Example concrete
owners: graphlib `function_env` 100, CallEnv drop 66, FrameBindings get/cell 95,
Vm construction 110 samples; immutable 265, 204, get/cell 127, construction 776.
These concrete owners explain the entry-first choice without treating the
wide loop's large self share as dispatch overhead.

## Expected payoff estimate, not a forecast

Use delta = reach × eligible-cohort fraction × net cost removed within that
cohort. No eligible-cohort fraction has yet been counted. For planning only,
if M2.1 removes **50–70% net of E**, graphlib gains **5.61–7.85%**, exceeding
its 4.77% price, while immutable gains **4.70–6.58%**, short of its 7.08% price.
M2.1 would need **42.58%** of graphlib E, **75.31%** of immutable E to pay alone.
**Do not turn M2.1 into a leaf or claim that it pays every target.**

For the whole migration, use only E+V, deliberately crediting neither D/C nor
unseparated wide/Vm run time. Reach is **16.32%** graphlib, **20.14%** immutable.
A **40–60% net reduction** gives conditional gains **6.53–9.79%** and
**8.06–12.08%**, respectively; ratios .9347–.9021 and .9194–.8792, both beyond
the required .9523/.9292 at this planning range. Example basis: 80% eligible
coverage × 50–75% net elimination yields 40–60%; the design deletes whole name
assembly/reconciliation and then nested-Vm setup, rather than one probe. These
are explicit assumptions, not measured coverage, speedups or lower bounds.
If counters show the necessary ~29%/~35% E+V elimination implausible, stop for
lead review before freeze; do not alter targets after a candidate measurement.

The existing five stages provide bounded implementation space for this thesis;
only M2.5 pays. T021's frame movements/window-base additions and T024's 0.9906
preamble result forbid using stable storage or dispatch shrinkage as a payoff
proxy. T027's 0.984055 direct-local result forbids reviving a local-op recognizer.
The [T021 log](T021-single-vm-frame-stack-log.md), [T024](T024-general-register-core.md)
and [T027](T027-frame-verified-direct-local-opcodes.md) were read before this draft.

## Proposed budget, resources and code-layout risk

The proposal owns numeric fields: fixed M, five stages, current stage 1,
cumulative stage cap **min(1.025, B_i)** per policy case, instruction-blind,
workers-4 v6 policy SHA-256
`be123f7c5cfd46aa863467256eb8bf6947ff64f72541efc371f05a36c5535a31`,
both targets memory-sensitive, **K=6 × 5 pairs**, **60 formal blocks** after
passing each stage screen, **30-pair held-out** at final acceptance plus all
mandatory completions. Intermediate stages use `decide --mode stage`; only
stage 5 runs ordinary payoff/promotion. No stage changes the cumulative base.

At most **two valid failed screens/stage aborts total**, shared across M2;
a second valid failure closes this shape. No borrowing/reset from M1/K.
Upper budget: seven valid full screen sets and six canonical formal bundles;
v6 mandatory two-look continuations through cap 240 remain, with no optional
extra look or rerun to get favorable data. Incomplete evidence follows the
existing completion/load-only recovery rules. No attempt is spent here.

Manual resource ceiling: **per-case median paired process max-RSS candidate/M
≤1.05 for every one of the 86 selection cases**, including all ten development
cases, at every stage. Complete exact-image/host/case/block-bound raw cohorts
and the existing replayed resource report are mandatory; a generic probe or
lane RSS cannot substitute. Missing/excess coverage blocks advance/promotion.
This stricter all-selection-case ceiling is proposed, not tool-enforced.

M2 does **not** have a kernel firewall. Entry/context changes first avoid a
premature dispatch/typed-service rewrite, but fat LTO may still change typed
and wide relative PCs, spills or helper inlining. Keep admission/scalar banks,
object layout and recipe fixed. Before each future screen compare exact linked
M/candidate kernel bytes, instruction offsets from entry, branch/table targets,
stack spills, sizes and hot-callee graph; use reproducible A/A when attributing
drift. Counts, entry addresses and page residues alone are insufficient.
Stages 3/4 intentionally change wide driver code; separate those changes from
accidental typed-loop drift and price both on the complete portfolio.

When diagnosing coupling, `scripts/check-kernel-isolation.sh --ref <candidate>
--full --output "$BENCH_OUT/kernel-diagnostic"` runs through a **non-timing**
queued job. It diagnoses A/A plus H8/C8 perturbations; separately compare M and
candidate fingerprints. Its failure is **not** an M2 rejection or prerequisite;
its pass proves neither neutral timing nor M/candidate equality. Do not revive
K's service-yield outlining, add padding, select a favorable placement, alter a
golden or change the recipe to conceal drift. All six matching layouts,
instruction alerts, every case/lane/portfolio, formal precision and final
held-out acceptance remain binding. No kernel diagnostic build was run here.

## Validation and handoff

Validation command from this worktree (workspace abbreviated as W):

```sh
./scripts/performance-decision.sh validate-unit \
  --unit tasks/proposals/T056-m2-frames-activation.json \
  --queue "$W/tmp/qjs-lead-briefs/current-main-9eba7a1d-queue.json" \
  --profile-root "$W/outputs/imac-bench"
```

Exit **0**, `status=valid`, `priority_mode=override`, base M; unit SHA-256
`3b7357cd979b8886d127812f5dc9981546d283ad49e53ae7458a5f0dfabbc7c2`; queue SHA-256 as above.
Validation stdout SHA-256
`d182d962300342fb5a9f5298de40af34ff49afd16d37b5d910e68b13bd82040a`. All six declared
receipts pass exact executable/opportunity/asset checks.

Structure/provenance validation is not approval, a measurement or a check of
the manual RSS/attempt ledger/native writer inventory. The draft remains in
`tasks/proposals/`; no `performance-units/` freeze/index or runtime was edited.

Open review items: count current callee/caller-ordinary coverage; audit and
freeze the complete native-writer exclusion list; review the conservative net
savings assumptions and two-failure/six-formal/all-case-RSS budget; resolve
T056/T064's outdated resume blocks and M1 closure on the lead branch. The brief
authorizes this proposal, not an automatic implementation of a blocked task.

Verification at `d1e80fec` plus this planning delta: `check-context.py`,
`git diff --check` and the staged fast gate passed. The initial context check
required indexing this new archive; adding its link to `tasks/README.md` resolved it. No runtime/parser/lexer source,
script, Cargo file or executable example changed, so no Test262 slice matched
and `check.sh` was not rerun for this proposal. No local build was needed;
`df` showed 69 GiB free before work, and `target/debug` is absent at handoff.
No freeze, implementation, benchmark timing, push or merge.

## Measured cohort and pricing prototype

Lead feedback 1 requests one non-timing pricing round before any freeze. The
40–60% net-removal assumption above remains historical planning arithmetic;
**coverage is now counted, but an unmeasured elimination fraction is not a
payoff forecast**. The proposed JSON remains unchanged and unfrozen.

The scratch branch is `agent/t056-m2-pricing/codex`, based on main `9be6373d`
(which closes/abandons T064 K). Baseline cohort overlay `0ce6f3162908760d452e7492b6d44d62fc685f02`
is based on the same runtime as queue M; the indexed-entry prototype is
`9dafdebe401b27786fb05605872c08b1c9fc9fb9`. No prototype code is on this plan
branch. The scratch ownership boundary is runtime sources, its architecture
paragraph and diagnostic task/index; branch-scope validation passed.

**Counter method.** The agents/perf-counters build records actual bytecode
activations at the general, direct-leaf, compact-inline, successful typed-loop
closed-form and virtual-function-literal boundaries. Native-origin guards
separate a native's immediate JS callback from subsequent JS-to-JS calls.
Proxy/bound wrappers count separately from the body they forward to. Folded
helper graphs are not dynamic activations; generator invocation is counted,
resume is not. Existing `ordinary_call_attempts` is not used as the denominator:
it double-counts some forwarded direct-leaf calls and omits some numeric wide
inline successes. New counters partition their own explicit denominator.

Admission follows frame-activation's two ordinary rules: no own direct eval or
with, no active deopt unless bypassed, no parameter/body eval split, and no
transitively eval/with-capable descendant. The transitive fact walks functions,
class constructors/methods, private elements, static blocks and deferred keys,
once at compilation. Generator/async and construction are later-stage
populations. Parameter shape/captured cells/module/lexical state do not narrow
the full ordinary count; they may narrow the pricing prototype. Reason order
is generator, async, construction, own eval, own with, retained deopt,
descendant dynamic, ordinary; primary reasons partition activations.

Collection job `20261022T075213Z-t056-m2-cohort-complete` is **done 0**:
five requested development programs plus all six sentinels, all checksums valid,
no timing flag or result. It used the canonical release recipe with only
`agents,perf-counters` as the feature set. Counter metadata changes are not an
exact canonical-image performance comparison. Copied raw stdout/stderr and
identities are under the workspace's `outputs/imac-bench/<job>/out/`.
`identity.json` SHA-256 `a8e0ba7b4fdec0d9a018ec55959421ae850eb47dbe85820b27d52029dfc07266`;
`counts.json` SHA-256 `523fcf3d522d6d3eea48a94868fb6709f5ee3d77fb9b4c26f65cf5831346db2c`.

| Case | Activations | JS-to-JS | Native callbacks | Full ordinary | Construction excluded | Ordinary / JS-to-JS |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| graphlib/paths | 1,000,770 | 925,385 | 75,385 | 1,000,622 | 148 | 99.98401% |
| immutable/persistent | 5,429,466 | 4,917,207 | 512,259 | 5,299,633 | 129,833 | 97.35962% |
| moment/dates | 7,758,058 | 7,645,258 | 112,800 | 7,590,056 | 168,002 | 97.80253% |
| meriyah/parse | 2,303,861 | 2,301,202 | 2,659 | 2,303,860 | 1 | 99.99996% |
| diff/compare | 2,462,190 | 2,397,132 | 65,058 | 2,462,174 | 16 | 99.99933% |
| sentinel/recursive_call_tree | 5 | 5 | 0 | 5 | 0 | 100% |
| sentinel/prototype_method_call | 3,000,069 | 3,000,069 | 0 | 3,000,005 | 64 | 99.99787% |
| sentinel/polymorphic_call_site | 3,000,004 | 3,000,004 | 0 | 3,000,004 | 0 | 100% |
| sentinel/capturing_closure_call | 3,000,008 | 3,000,008 | 0 | 3,000,008 | 0 | 100% |
| sentinel/heterogeneous_property_read | 5 | 5 | 0 | 5 | 0 | 100% |
| sentinel/string_key_map_churn | 5 | 5 | 0 | 5 | 0 | 100% |

The last column uses **JS-ordinary / JS-to-JS**, excluding ordinary native
callbacks from its numerator. All other primary exclusion counts (eval, with,
retained deopt, descendant dynamic, generator, async, missing), plus Proxy and
bound dispatches, are **zero in every counted case**. This is coverage of
these programs, not removal of those fallback cohorts from the architecture.
For all activations, graphlib ordinary is 99.98521%, immutable 97.60873%.
The three five-call sentinels provide little entry-path coverage despite their
large nominal iteration count; the call-heavy sentinels mostly take existing
closed-form tiers. They remain mandatory controls, not evidence that the new
entry executes millions of times.

**Native writer audit, closed conservatively.** `global::native_global_eval`
can replace the lent `*env`; coercion can reach
`conversion::sync_to_primitive_method_env` and write existing caller cells;
callback completion can restore an immutable caller name. An Eval-only writer
list would therefore be unsound. Constructor new.target changes/restoration
also remain on legacy routing. `$262.evalScript` uses a separate frame and
shared realm cells; installation writes and dynamic Function's fresh global
snapshot are not writes to the lent activation. Property/realm effects remain
observable and are not optimized away.

The frozen scratch exclusion is every `NativeFunction` variant except eight
proved non-coercing activation-read-only natives: `ArrayIsArray`, `ArrayBufferIsView`,
`NumberIsFinite`, `NumberIsInteger`, `NumberIsNaN`, `NumberIsSafeInteger`,
`ObjectIs`, `IsHtmlDda`. The source enum has **592** variants; the **584**-member
conservative exclusion includes all unproved callback/coercion paths, not a
claim that every member writes on every invocation. A default exclusion also
covers future variants. Enum source SHA-256
`f709e944ee98a554e51a43c1975067f2cd588bdc4d1de29433c19f69e88237bf`.
All native boundaries, including the eight readers, remain legacy in this
small prototype; **no native environment saving is credited**.

| Case | Observed native dispatches | Conservative writer exclusions |
| --- | ---: | ---: |
| graphlib | 161,604 | 161,603 |
| immutable | 830,677 | 639,073 |
| moment | 4,652,876 | 4,652,876 |
| meriyah | 157,000 | 47,788 |
| diff | 1,675,476 | 1,674,996 |
| recursive / polymorphic / capturing sentinels (each) | 3 | 2 |
| prototype / heterogeneous sentinels (each) | 67 | 66 |
| string-key-map sentinel | 1,027 | 1,026 |

These are dispatches through `call_native_function`; already-specialized native
fast paths do not build that boundary. This is a separate native denominator,
never subtracted from ordinary JS activations. The conservative exclusion set
is complete; the minimal actual-writer set is not claimed.

**Implemented pricing scope.** The prototype preserves all existing direct-leaf
tiers, then replaces name assembly for a narrower general-call population:
plain identifier parameters (defaults allowed), static shared captures, no
own arguments/name/immutable-name/lexical-this/lexical-arguments/cold lexical
state, retained with chain, synthetic/foreign realm, super, suspension, class
creation or lexical-this child. It uses a typed ActivationContext guard and
borrowed services alternative in FrameEnvironment. Caller binding storage is
moved aside and restored, never cloned into a name/value mirror; arguments,
this and received cells use the existing positional metadata. Parent locals
captured by ordinary closures receive their required cell vector.

Nested Vms, argument vectors, caller env exports and native boundaries remain.
This is a partial M2.1 price, not an implementation or price of all five stages.
The first exercising tests caught the old direct-leaf initializer's omitted
parent-cell vector when admitting static closures; the correction is included
before collection. Five focused tests now prove new-route reach, escaping
shared cells, callback/throw restoration, transitive eval exclusion and exact
recursion accounting. `check.sh` passed on the prototype delta: 2,335 default
runtime tests, agents/perf-counters checks, 723 benchmark-tool tests, 35 script
tests, context/size/hooks and all 5,170 curated Test262 cases. Full log and
staged fast-gate hashes will be recorded with final collection identities.

**Pricing collection and provenance.** Job
`20261022T075812Z-t056-m2-pricing-instructions-complete` is **done 0**, without
`--timing`. It built canonical agents/fat16/abort `9dafdebe`, plus its separate
agents/perf-counters image. Exact M was reused and its hash checked. Each of
the same 11 cases has three alternating instruction pairs, validated stdout,
and three independent `sample` windows per role (10 s development, 2 s
sentinel, 1 ms interval). All 66 instruction records and 66 profile receipts
are retained; 33/33 receipts per role pass executable/source/opportunity and
copied-profile/workload hash verification. `/usr/bin/time -l` is used solely
for retired instructions; elapsed, user time and cycles do not decide or
support a speed claim. No wall screen, formal bundle or acceptance candidate
was collected. The non-timing queue does not certify a quiet/timing-valid host.

Canonical prototype executable SHA-256:
`d62dc9059d0e46a72c6d8e649fe3ac99b046bc086fc3a23fc00ef3dfbbbc254b`;
its counter image:
`17e485ab95130075a443fa6e7bd9452c04939d218329c0f6eb4f315b3c32f367`.
Rust is 1.95.0 / LLVM 22.1.2 on x86_64-apple-darwin. No recipe or object-layout
change, dependency, unsafe or FFI was introduced. Existing tier priority is
retained; source admission has no workload/name/checksum/path recognizer.

The first collector jobs are retained: `20261022T074651Z-t056-m2-cohort-m`
failed before a case because its workload directory was absent;
`20261022T075714Z-t056-m2-pricing-instructions` failed at import (the real helper
is `parse_macos_time`). Neither produced a candidate result to select or omit.
The complete jobs repair those collector errors, not an adverse measurement.

A perf-counters-only decline audit at
`232196d5f5fdeb8b3d938aff6c39168ffc0433d4`, job
`20261022T081053Z-t056-m2-pricing-decline-audit` (**done 0**), explains the narrow
prototype's coverage without replacing the measured canonical `9dafdebe`
image. All 11 cases retain exactly the same activation/cohort totals and
prototype entry counts; every decline is partitioned by the additional
counters. Six focused counter tests and the staged fast gate passed at that
audit delta. It is not a second runtime-price implementation attempt.

| Case | Prototype entries / all activations | Narrow prototype declines by primary reason |
| --- | ---: | --- |
| graphlib | 847 / 1,000,770 (0.084635%) | cold home/private context 33,503; self/name 17,203; lexical context 2,002; arguments 1 |
| immutable | 187,601 / 5,429,466 (3.455239%) | self/name 25,008; arguments 25,009 |
| moment | 12,151 / 7,758,058 (0.156624%) | arguments 43,273 |
| meriyah | 0 / 2,303,861 | self/name 10,400; arguments 2,602 |
| diff | 320 / 2,462,190 (0.012997%) | cold 37,440; lexical 23,690; arguments 5,441; self/name 5,378 |
| capturing sentinel | 4 / 3,000,008 | none |
| other five sentinels | 0 | none (already answered by existing tiers) |

Construction calls bypass this narrower ordinary-call test and were excluded
separately above. Existing leaf/compact/closed-form calls are not narrow
prototype declines. Therefore neither ordinary/total nor entries/total is a
cost fraction. In particular, graphlib's 99.98% semantic cohort does **not**
mean that this prototype removes 99.98% of its entry cost. Most of its general
calls still need cold or self/lexical context support. Count-based scaling
would erase exactly the unimplemented work the review asked to price.

**Per-symbol attribution.** Use the earlier nearest-semantic-owner algorithm
on the new base/prototype receipts, including all windows. Add
`activation_context|ActivationContext|FrameEnvironment|try_indexed_entry` to E,
recognize ActivationContext/FrameEnvironment trait owners before skipping
trait glue, and add `with_frame_environment|eval_indexed_entry` to V. Thus new
guard, borrowed-environment and indexed-wrapper costs are charged, not lost
under a new symbol. No entire wide/Vm run body is credited to entry work.

| Case | Samples M / prototype | E share M → prototype | V share M → prototype | E+V share M → prototype |
| --- | ---: | ---: | ---: | ---: |
| graphlib | 3,661 / 3,652 | 10.63% → 8.95% | 5.27% → 5.12% | 15.90% → 14.07% |
| immutable | 10,660 / 8,951 | 9.11% → 5.07% | 9.98% → 6.32% | 19.09% → 11.40% |
| moment | 24,886 / 24,880 | 5.36% → 5.38% | 2.28% → 2.10% | 7.65% → 7.48% |
| meriyah | 7,274 / 7,400 | 4.36% → 4.12% | 3.22% → 3.22% | 7.57% → 7.34% |
| diff | 18,088 / 18,101 | 12.99% → 12.67% | 2.07% → 2.02% | 15.06% → 14.70% |

Concrete owner counts (each column has its own process-sample denominator):

| Owner | graphlib M / prototype | immutable M / prototype |
| --- | ---: | ---: |
| `function_env` | 102 / 72 | 264 / 94 |
| `FrameBindings` get/insert/cell family | 77 / 73 | 140 / 69 |
| Vm construction (`new_with_globals*`, `with_frame_*`) | 114 / 114 | 720 / 378 |
| CallEnv drop | 69 / 60 | 215 / 73 |
| new typed context/borrow/entry owners | 0 / 1 | 0 / 42 |
| new indexed Vm wrapper | 0 / 2 | 0 / 54 |

These are sampled owner counts, not instructions per symbol or independently
additive inclusive fractions. Sampling variance and normalized denominators
matter: graphlib's apparent E+V-share drop cannot override its flat whole
instructions. Six sentinel E/V samples are zero under this narrow partition;
that is not proof that their setup performs no work.

**Measured projection and payoff decision.** Let R be the median of the three
whole-process prototype/M instruction ratios, and fM/fP the pooled E+V sample
fractions. A diagnostic wall-cost projection at unchanged process wall time
per retired instruction gives attributable removal `D = fM - R*fP`. Credit
`max(0, min(1-R, D))`: entry/Vm credit cannot exceed the observed net instruction
saving. This is a transparent inference, **not** measured wall time, a lower
bound, a confidence interval or a new acceptance rule. It assumes that the
sampled owner distribution represents cost and that the wall/instruction
conversion does not change; kernel drift below weakens the latter assumption.

No additional gain is credited for unimplemented native/argument work,
M2.2–2.5 or a larger ordinary cohort. That is the supported projection for the
full-M2 proposal using this prototype's measured scope, not a forecast that
future stages cannot improve it. In particular, do not multiply by
ordinary/prototype call-count ratios: per-call costs and declined mechanisms
differ, and no later-stage price was measured.

| Case | Instruction R (all-three range) | E+V cost D | Credited projection | Required payoff gain | Clears with margin in this model? |
| --- | ---: | ---: | ---: | ---: | --- |
| graphlib | .996932 (.996851–.997512) | 1.866% | **0.307%** | 4.773% | **No**, short by 4.467 pp |
| immutable | .846112 (.844461–.847388) | 9.448% | **9.448%** | 7.084% | **Yes conditionally**, +2.364 pp |
| moment | .995377 (.995148–.995620) | .202% | .202% | 4.773% | No |
| meriyah | .999954 (.999688–1.011330) | .237% | .0046% | 6.707% | No |
| diff | .998905 (.998809–.998914) | .380% | .110% | 9.017% | No |

Immutable's E+V removal envelope using **every combination** of the three
base shares, three prototype shares and three instruction ratios is
**8.510–10.359%**, still above 7.084% within this model. This is an observed
sensitivity envelope, not a statistical interval. Whole instructions fall
15.389%, but only 9.448% is credited to the priced owner classes; no 15.4%
wall-speed claim is made. For reference, the uncapped whole-instruction proxy
could tolerate at most about 9.82% worse wall/instruction conversion before
missing .929158; that conversion was not measured.

None of the other measured cases clears its payoff in this projection. All
six sentinel instruction medians are within 0.009% of unity: recursive
1.000020, prototype method 1.000019, polymorphic 1.000009, capturing .999995,
heterogeneous .999993, string-key-map 1.000084. Meriyah's third pair is retained
at 1.011330; no pair, window or case was dropped. Js-yaml and the other policy
cases were not priced in this review and receive no claim from this table.

**Conclusion for the lead:** M2.1 now has measured cost support on immutable;
graphlib does not clear with this prototype. **Do not freeze the original
two-target plan as written.** A graphlib projection still requires pricing its
cold/self/lexical contexts, rather than counting them as already removed.
A revised target/scope decision belongs to the lead, not automatic retargeting
from these candidate results. Wall, K=6, resources and final promotion remain
unverified; no performance attempt budget is spent by this diagnostic round.

**Typed/wide relative-PC risk.** Invoking the requested
`check-kernel-isolation.sh --ref 9dafdebe --full` through non-timing job
`20261022T075715Z-t056-m2-pricing-kernel-diagnostic` returns **exit 2**:
`K0 error: unknown tail alignment` on r0a. No complete A/A, H8 or C8 matrix
exists, and this is not a static pass or a performance rejection. The pricing
job's strict fingerprint also records this decode error. No decoder or golden
was changed to turn it into a pass.

Supplemental non-timing job `20261022T080212Z-t056-m2-raw-kernel-diagnostic`
is **done 0**. It uses structural role selection and raw-byte-checked Apple
otool instructions. Compare the whole symbol extent/raw hash plus the
contiguous instruction prefix **before the first owned data table**; assert
that every prefix byte decodes contiguously and equals the image. This
avoids claiming an interpretation of the unsupported tail. It is an
observational drift diagnostic, not a replacement kernel CFG/relocation gate.

| Role | M → prototype symbol bytes | Trusted-prefix instructions | Relative-PC observation |
| --- | ---: | ---: | --- |
| typed `Vm` | 13,792 → 13,888 | 2,937 → 2,956 | boundaries/tables/stack differ; first changed subsequent PC +0x242 → +0x243 |
| typed `WideLoopFrame` | 13,696 → 13,696 | 2,920 → 2,920 | prefix boundaries/tables/stack equal; full bytes differ, so no isolation claim |
| wide activation `run` | 24,640 → 24,960 | 5,141 → 5,193 | boundaries/tables/stack differ; first changed subsequent PC +0x1b3 → +0x1b0 |

Thus entry-only changes **did not avoid typed/wide codegen coupling**. This is
a concrete reason not to turn an instruction-price projection into a wall
claim. Complete layout/wall pricing remains necessary before acceptance.

Artifacts below are SHA-256, relative to each named job directory; all are
workspace-local raw evidence, not committed build outputs:

| Job / artifact | SHA-256 |
| --- | --- |
| pricing `command.sh` | `b413be01be02b8bcf5b455da92284e1dfe95df670b62efb77a4f575e51654fbd` |
| pricing `out/identity.json` | `4d607aee363a24a6d002115d071e26ed75a72fe69f5e507a44ab6cf2deb29dc7` |
| pricing `out/counts.json` | `f5eaa2036cfce82e22adc521319d9ef8dfe0d04c2578c160b89fea5a42f361b2` |
| pricing `out/instructions.json` | `54fdcb64c4b30b30334cb9a275a67ca0fcdf745057adb2aeec7b72662ee29c99` |
| pricing `out/profiles.json` (all 66 receipts) | `0a662e93d0bead257148ebd573ff331dd4a2803750ff50589ca88949ffadcb22` |
| pricing `out/kernel-difference.json` | `c4df4a02f343773a5ebc5aaa66ddf567fb0fb7c7f61f7a5c4e30a37bbf1fb020` |
| strict checker `out/kernel-diagnostic/result.json` | `86ebb5f3b0cdcdf2e622136791632426f2324d4135cd660b8f643027437dff1c` |
| raw-layout `command.sh` | `3a8585cdf24dd66f733f06cae937a564e8562e0799218734b83a9ab68daa24a1` |
| raw-layout `out/comparison.json` | `9a85bef14be27ff69508e0c565f9899c7a62396002d99faf04d2bd84977cc486` |
| decline-audit `out/identity.json` | `d87ad36f6e961e7915e625f38bdeeb0bd67d80a61ab0bd412635d5e9ed95a774` |
| decline-audit `out/counts.json` | `2b77877bcb68cdfe05ebb639e6dd7a35e077396af500e2204270716c02816cfe` |

The local derivation uses the exact masks and formula above:
`tmp/qjs-lead-briefs/m2-price-analysis.py` SHA-256
`5a3e4cfbfa1d7126954b661d1d84b22e5ecb7686776b0bcb2f8ea7122ff49a9b`;
final `m2-price-analysis.json` SHA-256
`f3667c477ad6462edfcc4c0997fff05b433905b294224b0e0280dc1bc04e03d7`.
These are derivatives, not sealed performance decisions.

Canonical prototype full-gate log SHA-256
`f5edc1413698a0777c41a0fbc19464a70d540bb736dbd4263c551afea2a77988`;
its staged fast gate
`97cfa95dd90614f253260c8cb6d1f225f4a398794413d58a48e062e088e4468f`.
Counter-audit six-test log SHA-256
`4906ffc7476094b5d5f6c5687e4fe65ebcf2b000961fcc2daa172a30727cd3d8`;
its fast gate
`65519b0230cbcc8f8cfe79715105bb4b4cab67c0674b62a40280ba9fcd08c73f`.
Full correctness applies to the measured `9dafdebe` runtime, not an asserted
HEAD-wide new acceptance result. This plan branch changes only this record;
context/whitespace/staged fast gate and unchanged draft validation are checked
before committing. Disk stayed above 8 GiB (local observed minimum 63 GiB);
local pricing/counts/plan `target/debug` directories are removed before handoff.
No freeze, push, merge, wall timing, layout-panel or formal decision.
