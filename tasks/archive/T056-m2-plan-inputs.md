# T056 M2 plan inputs: ordinary entry before the shared driver

- Status: historical-campaign; draft planning inputs, not an approved freeze.
- Implementation: none. The [proposal](../proposals/T056-m2-frames-activation.json), this record and its task-index link are added; no runtime, object layout or recipe change.
- Verified at: `d1e80fec` plus this planning delta, 2026-10-22 UTC; exact-M profile identities and validation below. Runtime/build diff from M to this branch base is empty.
- Evidence: this record, the proposal and [earlier exact-M receipts](T064-successor-plan-inputs.md); raw artifacts are workspace-local, not retained in Git.
- Unresolved: ordinary-cohort coverage, native-writer audit and lead approval; layout coupling remains. Estimates below are conditional, not measured gains.
- Next action: lead reviews targets, budget and native audit, resolves the stale M1/K status, then freezes a separately approved plan; no implementation or timing follows from this draft.

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
