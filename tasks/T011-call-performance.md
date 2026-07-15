# T011: Call and harness hot-path performance

> **Status (2026-06-21): subsumed by `T016-environment-model-rewrite.md`.**
> The remaining per-call locals-map clone is removed by T016 S5 (shared upvalue
> cells), which is the structural fix behind the `TypedArray/*` timeouts. Prior
> landed slices stay; new perf work on the binding path goes through T016.

## Goal

Keep Test262 harness-heavy cases from timing out by making ordinary JavaScript
calls, method dispatch, prototype property reads, and dense array element reads
cheap without changing observable semantics. This task tracks the remaining
performance work after the broad environment-model redesign landed.

## Scope

- Allowed paths: `crates/qjs-runtime/**`, focused Test262 allowlist/baseline
  updates when a performance timeout is promoted into a reliable pass, and this
  task note.
- Forbidden paths: `third_party/**`.
- Owner boundary: runtime call/VM/property work is shared architecture; serialize
  broad edits on one branch. Small, isolated builtin fast paths may be split
  only after a focused gap scan identifies independent subtrees.

## Current State

The original flat `HashMap<String, Value>` call-environment model has been
replaced by `CallEnv` (`crates/qjs-runtime/src/function/env.rs`):

- `realm`: shared `Rc<RefCell<HashMap<String, Value>>>` for intrinsics and true
  globals, so reassigned builtins and sloppy globals are visible across frames
  without copying or write-back scans.
- `locals`: per-frame bindings (`this`, parameters, captured locals, caller
  scope bindings). Cloning a `CallEnv` shares the realm and copies only locals.
- Reads clone values out of a short realm borrow. Do not hold a realm borrow
  across user callbacks, getters, setters, Proxy traps, iterators, or
  `valueOf`/`toString`.

Related landed performance work:

- `Function` values are shared `Rc` handles, so operand-stack, property, capture,
  and argument clones no longer copy the function's vectors and maps.
- Leaf function calls skip activation captured-env snapshots when the body
  cannot create nested closures/classes.
- Prototype-chain data-property gets use VM fast paths for ordinary reads.
- Dense array present-index reads use `ArrayRef::direct_dense_index_value`
  before falling back to generic property resolution.
- `eval_function_bytecode` returns the live `CallEnv` directly instead of
  rebuilding a frame env at every function return.

## Current Evidence

At the `b320d151295cd6d306e8f1760199c11996698365` baseline, a local macOS arm64
three-block run of `core-black-box-v4` measured an 86.57x candidate/QuickJS-NG
geometric-mean ratio. A same-machine exploratory run after changing `Function`
to a shared handle measured 0.732x candidate/base (26.8% lower wall ns/op) and
62.91x candidate/QuickJS-NG. Every critical case improved:

| Case | candidate/base |
| --- | ---: |
| `plain_function_call` | 0.716x |
| `method_call` | 0.729x |
| `captured_read` | 0.721x |
| `captured_write` | 0.723x |
| `many_locals_call` | 0.691x |
| `property_read` | 0.768x |
| `array_read` | 0.782x |

The exploratory candidate was built with the frozen hosted-preview Rust recipe
but had no clean-source receipt; use the post-commit Performance Preview artifact
for provenance-backed reporting.

At commit `9c48b0e4b5165d83700e1afab41b38fe6effd5cc`, the next local optimization
kept uncaptured `FrameBindings` values inline and promoted them to shared
`Upvalue` cells only when binding identity was requested. Two same-recipe
candidate/base runs reproduced a 0.969x geometric-mean ratio (3.1% lower wall
ns/op). The five-block confirmation used seed `20250714`; all seven critical
cases improved or stayed effectively flat, with `method_call` at 0.937x and
`property_read` at 0.900x. This was also exploratory dirty-source evidence;
the post-commit Performance Preview remains the provenance-backed result. That
preview ran successfully at `a95962d64754a17cf58d1076f25fc1d31273c289`, but
its variable GitHub-hosted Linux runner measured 1.0225x overall with a 95%
confidence interval of [1.0153x, 1.0261x] and classified health as
inconclusive. The hosted direction therefore disagreed with both local runs;
retain both results rather than treating the three-block hosted preview as a
fixed-hardware claim.

Starting from `a95962d64754a17cf58d1076f25fc1d31273c289`, wrapping the many
independent `ObjectRef` storage cells in one shared handle made each object
clone a single reference-count update and reduced the largest `Value` variant.
A three-block local run measured a 0.884x candidate/base geometric mean; an
independent five-block confirmation with seed `20250715` measured 0.887x
(11.3% lower wall ns/op). All seven critical cases improved in the confirmation:
`method_call` was 0.868x, `property_read` was 0.873x, and even the least-improved
`array_read` case was 0.923x. These are exploratory dirty-source measurements;
use the post-commit Performance Preview for provenance-backed reporting. The
preview at `715c830f2c4223b6745355eeec46cdac0cba6f48` confirmed the direction on
hosted Linux at 0.8193x overall (18.07% lower wall ns/op), with a 95% confidence
interval of [0.8193x, 0.8327x]. All seven cases improved, and the resulting
candidate/QuickJS-NG overall ratio was 67.2446x. The job remained informational
and health was inconclusive because it used only three variable-host blocks.

Starting from `715c830f2c4223b6745355eeec46cdac0cba6f48`, applying the same
single-handle layout to `ArrayRef` removed nine reference-count updates per
array clone and reduced the largest `Value` variant again. A three-block local
run measured 0.827x candidate/base overall; an independent five-block run with
seed `20250717` confirmed 0.833x (16.7% lower wall ns/op). Every critical case
improved: `array_read` was 0.748x, `many_locals_call` was 0.813x, and the
least-improved `property_read` case was still 0.874x. These are exploratory
dirty-source measurements. The Performance Preview at
`1b5373d06f08e2d32315ed3febb044485907f9a5` confirmed the direction on hosted
Linux at 0.8414x overall (15.86% lower wall ns/op), with a 95% confidence
interval of [0.8339x, 0.8487x]. All seven cases improved, and the resulting
candidate/QuickJS-NG overall ratio was 56.2095x. The first attempt was rejected
when the QuickJS-NG `method_call` linearity probe exceeded its health threshold;
the automatic rerun passed linearity and produced the reported artifact.

Sampling the resulting plain-call workload identified
`FunctionParams::names()` in every ordinary function entry: it rebuilt two
vectors and cloned all parameter names merely to detect an `arguments`
parameter. Replacing that allocation with a borrowed recursive binding-pattern
scan measured 0.960x candidate/base in a three-block local run. An independent
five-block run with seed `20250720` confirmed 0.965x overall (3.5% lower wall
ns/op): `plain_function_call` was 0.920x, `captured_read` was 0.915x,
`captured_write` was 0.967x, `many_locals_call` was 0.978x, and `method_call`
was 0.981x. `property_read` was 0.991x and `array_read` was 1.009x, so the
non-call cases stayed effectively flat. These remain exploratory dirty-source
measurements. The Performance Preview at
`5f2e65385f56787e3e7ac8005c2780b49c46c42c` confirmed the direction on hosted
Linux at 0.9496x overall (5.04% lower wall ns/op), with a 95% confidence
interval of [0.9411x, 0.9587x]. All seven cases improved, and the resulting
candidate/QuickJS-NG overall ratio was 54.7519x. The first attempt was rejected
when the base `array_read` linearity probe reached 1.1628x; the single rerun
passed all linearity probes and produced the reported artifact.

Starting from that parameter-scan change, reserving the ordinary call frame's
short binding vector before inserting `this`, positional parameters, and
internal context markers measured 0.992x candidate/base in a three-block local
run. An independent five-block run with seed `20250722` reproduced 0.991x
overall (0.9% lower wall ns/op). Six paired case effects improved; the sole
exception was `method_call` at 1.009x, although its candidate absolute median
was also slightly lower than base. Treat this as a small allocation-path gain,
not a broad performance step. The Performance Preview at
`b8414ba46b3e0c5e63c94f47627f439bb63ab220` measured 1.008x overall with a
95% confidence interval of [0.997x, 1.017x], so the hosted result was neutral
and did not confirm the local direction. The resulting candidate/QuickJS-NG
overall ratio was 54.1772x.

The next retained slice stopped materializing the internal direct-eval function
context marker for leaf functions whose bytecode contains neither direct eval
nor nested closure creation. A three-block local run measured 0.983x overall
with all seven critical cases improved. An independent five-block run with seed
`20250726` confirmed 0.984x overall (1.6% lower wall ns/op): six cases improved,
led by `property_read` at 0.946x, while `array_read` regressed to 1.025x. The
Performance Preview at `bd3f03ea` contradicted those local results: hosted Linux
measured 1.0384x overall with a 95% confidence interval of [1.0358x, 1.0515x],
a real 3.84% regression, and the resulting candidate/QuickJS-NG ratio worsened
to 56.4006x. Do not retain this slice by itself.

Building on that slice, ordinary synchronous leaf functions with simple
parameters can seed `this` and positional parameters directly into VM local
slots instead of first constructing name-keyed frame bindings and then copying
them into slots. The semantic guard excludes constructors, lexical
`this`/`arguments`, generators, async functions, classes, direct eval, `with`,
closures, `super` operations, deoptimized bindings, and arguments-object users.
Against `bd3f03ea`,
three- and five-block local runs measured 0.981x and 0.984x overall. More
importantly, a five-block net comparison against the pre-regression
`b8414ba4` baseline with seed `20250729` measured 0.969x overall, with all seven
cases improving: `plain_function_call` was 0.983x, `many_locals_call` was
0.975x, and `property_read` was 0.871x. These remain exploratory dirty-source
measurements. Retain the combined direction only if the post-commit Performance
Preview also offsets the hosted regression above; otherwise revert both slices.

The Performance Preview at `ce7e25cc555ce1698be7bf8913ba8c12adf8e177`
measured 0.9851x overall against `bd3f03ea`, with a 95% confidence interval of
[0.9781x, 1.0012x]. The interval crossed 1.0, so this is not an isolated hosted
performance claim. However, the resulting candidate/QuickJS-NG ratio fell from
the pre-regression `b8414ba4` preview's 54.1772x to 53.5414x, which offsets the
intermediate hosted regression and satisfies the pre-set retention rule for the
combined context-marker/direct-slot change.

Removing the per-field shared handles inside `ObjectData` was also tested: an
`ObjectRef` already owns the whole record behind one shared handle, so the inner
handles appeared redundant. A three-block same-machine run instead measured
1.010x overall and 1.006x for `property_read`; the experiment was discarded.

The next local slice specializes statically named member reads in bytecode.
Previously `object.name` loaded a string `Value`, pushed it as a second operand,
then converted it back into an owned property-key `String` on every execution.
`GetPropNamed` embeds the immutable key in bytecode and dispatches directly to
the existing ordinary-object fast path, while computed properties retain the
generic observable `ToPropertyKey` path. Storing the embedded key as `Rc<str>`
also avoids allocating when the VM clones an instruction for dispatch. An exact
five-block comparison against `ce7e25cc` with seed `20250803` measured 0.876x
overall (12.35% lower wall ns/op). `property_read` was 0.566x and `method_call`
was 0.727x; five unrelated cases remained near the baseline, from 0.976x to
1.011x. The Performance Preview at `992e7ed63d248414055c247027e31fc0ab1373ab`
confirmed and strengthened the direction on hosted Linux at 0.8335x overall
(16.65% lower wall ns/op), with a 95% confidence interval of [0.8167x,
0.8392x]. All seven cases improved: `property_read` was 0.556x and
`method_call` was 0.720x. The resulting candidate/QuickJS-NG ratio fell from
53.5414x to 45.0203x; `property_read` remains the largest individual gap at
62.4767x.

Two follow-up attempts did not meet the overall retention threshold. Fusing a
local load with `GetPropNamed` measured 0.988x for `property_read` but 1.003x
overall in a three-block run, so the extra bytecode operation was discarded.
Reading an own ordinary data value before cloning its full descriptor reproduced
a 0.949x `property_read` ratio in an independent five-block run, but the other
cases offset it and the overall ratio was only 0.999x. That experiment was also
discarded rather than accepting a case-specific win with no portfolio progress.

The VM dispatch loop previously cloned the complete `Op` before executing every
instruction. That made even scalar local loads, jumps, and arithmetic clone the
largest enum representation, including reference-count traffic for embedded
data. Dispatch now borrows the current instruction directly and clones owned
fields only in handlers that actually consume them, such as function creation
and try-scope setup. An exact three-block comparison against
`992e7ed63d248414055c247027e31fc0ab1373ab` with seed `20250807` measured
0.776x overall, with all seven critical cases improved. An independent
five-block run with seed `20250808` confirmed 0.779x overall (22.1% lower wall
ns/op): `array_read` was 0.709x, `property_read` was 0.745x, and the five call
and binding cases ranged from 0.751x to 0.817x. The Performance Preview at
`83dbe893c42af26287f87a7dc7a5820a6dfafaeb` confirmed a larger 0.7421x
overall ratio (25.79% lower wall ns/op), with a 95% confidence interval of
[0.7370x, 0.7486x]. All seven cases improved, led by `array_read` at 0.659x;
the resulting candidate/QuickJS-NG ratio fell from 45.0203x to 33.6414x.

A follow-up profile showed that ordinary loop-local writes still cloned the
binding name and probed frame, module-import, module-live-binding, immutable,
and global synchronization state on every iteration. Local reads likewise
probed module imports even when an indexed slot was the only possible source.
The VM now directly reads, initializes, and assigns mutable slot-only locals
when the current frame has no same-named compatibility binding, dynamic-scope
map, module binding, immutable function-name binding, shared upvalue, global
scope, or sloppy-global fallback. The guard is checked against current runtime
state rather than cached at frame construction, so `eval`, `with`, closures,
modules, globals, and dynamically materialized same-named bindings retain the
full synchronization path. An initial whole-frame guard measured 0.6483x
overall in an independent five-block run but left `method_call` at 1.007x
because its unrelated `this` binding disabled the parameter-slot fast path.
Narrowing that guard to same-named frame bindings made `method_call` 0.736x in
a focused three-block run. The final independent five-block comparison against
`83dbe893` with seed `20250812` measured 0.6354x overall (36.5% lower wall
ns/op), with all seven cases improved: `array_read` 0.417x, `property_read`
0.525x, `many_locals_call` 0.669x, `plain_function_call` 0.722x,
`captured_write` 0.723x, `method_call` 0.738x, and `captured_read` 0.741x. These
are local dirty-source measurements. The Performance Preview at
`ecff7f88ccd5f4ba29bb1815ddf8926a11bc2581` confirmed the direction at
0.7645x overall (23.55% lower wall ns/op), with a 95% confidence interval of
[0.7626x, 0.7776x]. All seven hosted cases improved: `array_read` 0.565x,
`property_read` 0.664x, `many_locals_call` 0.765x, `method_call` 0.847x,
`captured_write` 0.848x, `plain_function_call` 0.860x, and `captured_read`
0.862x. The resulting candidate/QuickJS-NG overall ratio fell from 33.6414x
to 24.8457x. CI and the full Test262 Coverage workflow were also green at
that commit.

Profiling the resulting `property_read` binary showed that plain objects still
paid a string-property HashMap probe in `is_typed_array_object` before every
ordinary named read; TypedArray brand checks accounted for roughly 14% of the
sampled workload. TypedArray identity now lives in a dedicated `ObjectData`
internal brand bit installed with the view slots, making the ordinary-object
rejection an O(1) cell read. This also stops a JavaScript-accessible NUL-prefixed
property from serving as the internal brand. A final independent five-block
comparison against `ecff7f88` with seed `20250817` contained all 70 expected
measurement samples and measured 0.9745x overall (2.55% lower wall ns/op).
`property_read` improved to 0.842x and `method_call` to 0.979x; the other five
cases stayed between 0.998x and 1.013x. These are exploratory local binaries
without provenance receipts. The post-commit Performance Preview at
`bee0303937bc889320c2ea782ea0bf379aa676e1` produced no performance conclusion:
the first attempt had two timer-limited QuickJS-NG `property_read` blocks, and
the single rerun failed QuickJS-NG `array_read` linearity at 1.2105x. The rerun's
otherwise complete measurements pointed in the same direction at 0.9613x
candidate/base overall and 0.8803x for `property_read`, but health was invalid,
so these ratios are diagnostics rather than a provenance-backed result. CI and
the full Test262 Coverage workflow were green at this commit.

The next property-read profile put `ObjectRef::own_property` descriptor cloning
on the dominant ordinary-object path. The VM now probes each ordinary object's
string-keyed property table once and, for a plain data property, clones only its
`Value`; accessor descriptors, module namespace objects, Proxy chains, and
typed-array exotic descriptors retain the existing observable fallback paths.
Unlike the earlier discarded own-data experiment, this does not probe the same
HashMap a second time before cloning a descriptor. A three-block local run with
seed `20250818` measured 0.9849x candidate/base overall and 0.9443x for
`property_read`. An independent five-block confirmation with seed `20250819`
contained all 70 expected measurements and completed at 0.9784x overall (2.16%
lower wall ns/op): `property_read` was 0.9429x, `many_locals_call` 0.9538x,
`captured_write` 0.9773x, `method_call` 0.9832x, `captured_read` 0.9909x, and
`plain_function_call` 0.9951x. The unrelated `array_read` case was 1.0073x.
These remain exploratory dirty-source binaries without provenance receipts;
the post-commit Performance Preview at
`1d266237348e2878d1088ba3717a0684dfdce850` confirmed the focused
`property_read` direction at 0.9417x, but measured 1.0142x overall with a 95%
confidence interval of [1.0091x, 1.0190x]. The other hosted cases ranged from
0.9957x for `captured_read` to 1.0487x for `many_locals_call`, so the hosted
portfolio direction contradicted both local runs. The preview was informational
and health was inconclusive because its three blocks did not satisfy the frozen
precision policy. The resulting candidate/QuickJS-NG ratio was 24.2364x. CI
and the full Test262 Coverage workflow were green at this commit.

The next profile showed that ordinary named reads still called
`is_symbol_primitive` before the TypedArray check, and Symbol identity used up
to two NUL-prefixed string-property probes. Primitive and boxed Symbol identity
now live in a dedicated internal brand cell, so ordinary objects reject this
path without hashing and string properties can neither forge nor erase the
brand. A three-block local comparison against `1d266237` with seed `20250820`
measured 0.9867x candidate/base overall. An independent five-block confirmation
with seed `20250821` contained all 70 expected measurements and completed at
0.9852x overall (1.48% lower wall ns/op): `property_read` was 0.9434x,
`method_call` 0.9727x, `captured_read` 0.9871x, and the other four cases stayed
between 0.9956x and 1.0003x. These are exploratory local binaries without
provenance receipts. The Performance Preview at
`f3f565f6671d0fc477ab57c573b446e931d45d6e` confirmed the direction on hosted
Linux at 0.9834x overall, with a 95% confidence interval of [0.9663x, 0.9898x].
All cases except `many_locals_call` at 1.0026x improved; `property_read` was
0.9516x and `array_read` was 0.9702x. The resulting candidate/QuickJS-NG ratio
was 23.8918x. The preview remained informational and health was inconclusive
because three variable-host blocks did not satisfy the frozen precision policy.
CI and the full Test262 Coverage workflow were green at this commit.

The next profile showed that ordinary local loads and stores recomputed the
same binding-authority predicate on every bytecode operation, including frame,
module, immutable-name, and dynamic-scope checks that normally stay unchanged
for the frame's lifetime. The VM now precomputes authority for its common first
128 local slots in an inline bit mask, without a per-call allocation. Slots
beyond that range conservatively retain the full path, and creating an upvalue
clears the corresponding bit; generator setup and resume refresh the mask when
their environment or captured-slot state changes. A first `Vec<bool>` prototype
proved the loop benefit but regressed call cases by 2.5%-3.2% due to its frame
allocation, so it was discarded. The zero-allocation mask measured 0.9680x
overall in a three-block comparison. An independent five-block confirmation
with seed `20250824` contained all 70 expected measurements and completed at
0.9699x overall (3.01% lower wall ns/op), with all seven cases improved:
`property_read` 0.9369x, `array_read` 0.9403x, `method_call` 0.9679x,
`plain_function_call` 0.9808x, `captured_write` 0.9841x,
`many_locals_call` 0.9884x, and `captured_read` 0.9927x. These are exploratory
local binaries without provenance receipts. The Performance Preview at
`b6d3dbbb3d2bd8aaee9f28d0be6626aa6040b9ce` confirmed a larger 0.9581x
overall improvement on hosted Linux, with a 95% confidence interval of
[0.9486x, 0.9691x]. All seven hosted cases improved: `array_read` 0.9267x,
`property_read` 0.9288x, `method_call` 0.9545x, `plain_function_call` 0.9607x,
`captured_read` 0.9736x, `many_locals_call` 0.9803x, and `captured_write`
0.9839x. The resulting candidate/QuickJS-NG ratio fell to 22.2481x. The
preview remained informational and health was inconclusive because three
variable-host blocks did not satisfy the frozen precision policy. CI and the
full Test262 Coverage workflow were green at this commit.

Sampling that binary showed that successful local loads, stores, and
assignments still entered the generic runtime-error conversion helper on every
operation; its two result monomorphizations accounted for roughly 9% of the
property-read sample. The helper now inlines only the successful `Result` match
and delegates errors to one cold, non-inlined function, preserving throw and
native-error conversion while keeping that code out of the VM dispatch loop. A
plain whole-function inline experiment measured only 0.9968x overall and
regressed two call cases because it copied the error machinery into the loop,
so it was discarded. The hot/cold split measured 0.9032x overall in a
three-block comparison. An independent five-block confirmation with seed
`20250827` contained all 70 expected measurements and completed at 0.8992x
overall (10.08% lower wall ns/op), with all seven cases improved: `array_read`
0.8261x, `property_read` 0.8443x, `many_locals_call` 0.9174x,
`plain_function_call` 0.9192x, `method_call` 0.9264x, `captured_write` 0.9287x,
and `captured_read` 0.9391x. These are exploratory local binaries without
provenance receipts. The Performance Preview at
`0b7305015c63c785ac6657689188932543154951` confirmed 0.8951x overall on
hosted Linux, with a 95% confidence interval of [0.8849x, 0.9003x]. All seven
hosted cases improved: `array_read` 0.7830x, `property_read` 0.8572x,
`many_locals_call` 0.8978x, `method_call` 0.9307x, `plain_function_call`
0.9321x, `captured_write` 0.9382x, and `captured_read` 0.9388x. The resulting
candidate/QuickJS-NG ratio fell to 19.6484x. The preview remained informational
and health was inconclusive because three variable-host blocks did not satisfy
the frozen precision policy. CI and the full Test262 Coverage workflow were
green at this commit.

The next sample made the ordinary `load_local` and `store_local` function
boundaries visible after their binding-authority checks became simple bit
tests. Their short authoritative-slot prefixes now inline into VM consumers,
while captured, dynamic-scope, module, global, and immutable-binding behavior
remains in explicit non-inlined slow functions. Splitting `assign_local` as
well was rejected because it regressed the captured-write case by 1.3% without
improving the portfolio. The retained load/store pair measured 0.9882x overall
in a three-block comparison. An independent five-block confirmation with seed
`20250903` contained all 70 expected measurements and completed at 0.9893x
overall (1.07% lower wall ns/op): `array_read` was 0.9613x, `property_read`
0.9843x, `method_call` 0.9879x, `many_locals_call` 0.9928x, `captured_read`
0.9972x, and `plain_function_call` 0.9996x. `captured_write` was 1.0024x, a
0.24% local regression within the run's noise. These are exploratory local
binaries without provenance receipts. The Performance Preview at
`6c60830fc293a74f48af05bede4fcdca696f1b6c` confirmed 0.9850x overall on
hosted Linux, with a 95% confidence interval of [0.9816x, 0.9860x]. Six hosted
case ratios were below 1.0: `plain_function_call` 0.9583x, `property_read`
0.9714x, `captured_write` 0.9786x, `array_read` 0.9845x, `many_locals_call`
0.9949x, and `captured_read` 0.9917x. `method_call` measured 1.0166x in the
three variable-host blocks. The resulting candidate/QuickJS-NG ratio was
19.9338x. The preview remained informational and health was inconclusive; CI
and the full Test262 Coverage workflow were green at this commit.

The compiler did not honor the ordinary `#[inline]` hint for the now-small
load/store prefixes: a follow-up profile still attributed about 7.5% of top-of-
stack samples to those two call boundaries. Requiring those prefixes to inline
removed both symbols from the same property-read profile while preserving the
explicit non-inlined slow functions. A three-block comparison measured 0.9846x
overall with all seven cases improved. An independent five-block confirmation
with seed `20250905` contained all 70 expected measurements and reproduced
0.9843x overall (1.57% lower wall ns/op): `array_read` was 0.9650x,
`property_read` 0.9727x, `many_locals_call` 0.9837x, `captured_write` 0.9899x,
`method_call` 0.9901x, `captured_read` 0.9938x, and `plain_function_call`
0.9954x. These are exploratory local binaries without provenance receipts;
use the post-commit Performance Preview for hosted evidence. That preview at
`94de171dab6598f8bc82f6280f56f37d7e7966c3` produced a 0.9859x
candidate/base ratio and a 19.1159x candidate/QuickJS-NG ratio, but its
QuickJS-NG `method_call` linearity check was 0.7477x, outside the frozen
0.85..1.15 bounds. Overall health was therefore invalid and the workflow
correctly failed; do not treat either ratio as a valid hosted baseline. CI and
the full Test262 Coverage workflow were green at this commit.

The subsequent plain-call profile showed that a non-strict function created in
the active realm still hashed `globalThis` in both the function and caller
environments on every call merely to prove that the two realms were identical.
The common same-`Realm` case now returns immediately on shared-handle identity;
genuinely cross-realm calls retain the existing global-object identity check.
A three-block comparison measured 0.9560x overall. An independent five-block
confirmation with seed `20250907` contained all 70 expected measurements and
reproduced 0.9576x overall (4.24% lower wall ns/op): `captured_write` was
0.9295x, `plain_function_call` 0.9305x, `method_call` 0.9333x,
`captured_read` 0.9375x, and `many_locals_call` 0.9707x. The two non-call cases
were neutral: `array_read` 1.0019x and `property_read` 1.0031x. These are
exploratory local binaries without provenance receipts. The post-commit
Performance Preview at `6eb791f30122b413ce13fc22f3054e9a8729d572`
confirmed a larger 0.9605x overall improvement on hosted Linux, with a 95%
confidence interval of [0.9602x, 0.9704x]. The call cases improved most:
`method_call` was 0.9182x and `plain_function_call` was 0.9183x;
`captured_read` was 0.9275x, `captured_write` 0.9729x, and
`many_locals_call` 0.9839x. The two non-call cases were neutral within their
confidence intervals: `array_read` 1.0015x and `property_read` 1.0057x. The
resulting candidate/QuickJS-NG ratio was 19.4039x. All three linearity probes
passed and all three measurement blocks were valid; the preview remained an
informational `non_claim` because variable-host precision was inconclusive.
CI and the full Test262 Coverage workflow were green at this commit.

The next plain-call profile still attributed substantial samples to string
hashing while setting up ordinary function frames. Dynamic `Function`
constructors mark their synthetic realm once, before compiling the resulting
function, but every subsequent call re-read the same hidden realm-map key.
`FunctionData` now caches the marked realm's global object at function
creation; ordinary functions store `None`, so their call path avoids both the
realm lookup and the marked-function override probe. A mutation-maintained bit
preserves the existing post-creation function-property override without making
ordinary calls hash the hidden property name. A three-block comparison of the
final semantics-correct implementation measured 0.9678x overall. An independent
five-block confirmation with seed `20250913` contained all 70 expected
measurements and reproduced 0.9680x overall (3.20% lower wall ns/op), with all
seven case ratios below 1.0: `method_call` was 0.9394x,
`plain_function_call` 0.9531x, `captured_read` 0.9544x,
`captured_write` 0.9577x, and `many_locals_call` 0.9877x. The two non-call cases
remained neutral: `property_read` 0.9911x and `array_read` 0.9946x. These are
exploratory local binaries without provenance receipts. The post-commit
Performance Preview at `0a3fb27c6486cf11b8f91deec05947fed0987648`
confirmed 0.9707x overall on hosted Linux, with a 95% confidence interval of
[0.9646x, 0.9707x]. The call and captured-binding cases improved:
`plain_function_call` was 0.9446x, `captured_read` 0.9475x,
`captured_write` 0.9524x, `method_call` 0.9572x, and
`many_locals_call` 0.9858x. `array_read` was neutral at 1.0017x and
`property_read` measured 1.0082x in the three variable-host blocks. The
resulting candidate/QuickJS-NG ratio was 17.9810x. All linearity probes passed
and all three measurement blocks were valid; the preview remained an
informational `non_claim` because variable-host precision was inconclusive.
CI and the full Test262 Coverage workflow were green at this commit.

The next profile showed `CallEnv::new_function_frame_with_capacity` as the
largest named call-setup cost. Even the already restricted direct-leaf path
cloned the caller's catch names, direct-eval conflict set, private environment,
with stack, and module imports, although its contract excludes dynamic scope
and function-owned private/module state replaces those fields before execution.
Direct-leaf calls now start those transient fields empty while retaining shared
realm, module-host, and agent context. A three-block comparison measured
0.9958x overall. An independent five-block confirmation with seed `20250915`
contained all 70 expected measurements and reproduced 0.9944x overall (0.56%
lower wall ns/op). All five affected call/binding cases improved:
`captured_write` was 0.9839x, `captured_read` 0.9904x,
`many_locals_call` 0.9907x, `plain_function_call` 0.9905x, and `method_call`
0.9910x. The two untouched cases measured `array_read` 1.0064x and
`property_read` 1.0083x. These are exploratory local binaries without
provenance receipts. The post-commit Performance Preview at
`acb9282db1005fbf6c4a12855658d715b65de22f` confirmed 0.9891x overall on
hosted Linux, with a 95% confidence interval of [0.9818x, 0.9911x]. The five
affected cases all remained below 1.0: `captured_write` was 0.9701x,
`captured_read` 0.9831x, `plain_function_call` 0.9844x,
`many_locals_call` 0.9866x, and `method_call` 0.9971x. `array_read` was neutral
at 1.0001x and `property_read` measured 1.0031x in the three variable-host
blocks. The resulting candidate/QuickJS-NG ratio was 18.1565x. All linearity
probes passed and all three measurement blocks were valid; the preview remained
an informational `non_claim` because variable-host precision was inconclusive.
CI and the full Test262 Coverage workflow were green at this commit.

The following profile still showed hashing below `function_call_this`: every
sloppy call with a nullish receiver re-read the same private
`\0global_this` realm-map entry, even though that internal identity is fixed
when the realm is created. `RealmState` now caches the internal global-this
slot directly, and sloppy-this conversion plus the cross-realm identity guard
read that slot without hashing its private string key. Replacing the public
`globalThis` property still does not replace the realm's internal global-this
identity, covered by a focused regression test. A three-block comparison with
seed `20250916` measured 0.9814x overall. An independent five-block comparison
with seed `20250917` contained all 70 expected measurements and reproduced
0.9951x overall. The four cases that exercise this nullish-receiver path were
all below 1.0 in the confirmation: `many_locals_call` was 0.9778x,
`captured_read` 0.9879x, `plain_function_call` 0.9935x, and
`captured_write` 0.9966x. The unrelated cases measured `array_read` 0.9913x,
`property_read` 1.0071x, and `method_call` 1.0119x. These are exploratory local
binaries without provenance receipts; use the post-commit Performance Preview
for hosted evidence.

The post-commit Performance Preview at
`f8e6994aad503608d39c546070978668c6a94179` produced no valid performance
conclusion: candidate and base linearity probes passed, but the hosted
QuickJS-NG `array_read` probe measured 1.1931x, outside the [0.85x, 1.15x]
acceptance interval, so overall health was invalid. Its raw report is retained
only as diagnostic evidence, not as a new hosted baseline. CI and the full
Test262 Coverage workflow were green at this commit.

The next profile exposed a larger duplicated frame cost in the VM call
adapter. A user-bytecode call first built an empty compatibility `CallEnv`;
`call_function` then built the real direct-leaf frame, and return handling
snapshotted and applied the still-empty outer shell. The VM now bypasses that
shell only when the existing strict direct-leaf predicate proves the callee
cannot require caller binding write-back; native, dynamic, bound, generator,
async, constructor, closure-creating, `eval`, `with`, and other complex calls
retain the general path. A focused regression verifies that a callee parameter
cannot overwrite a same-named caller slot while shared global writes remain
visible. The bypass explicitly carries the VM's dynamic-import host and
feature-gated agent context; the agents feature's 20 focused Atomics/worker
tests cover that handoff. A three-block comparison of the final implementation
with seed `20250920` measured 0.9012x overall. An independent five-block
comparison with seed `20250921` contained all 70 expected measurements and
reproduced 0.9013x overall (9.87% lower wall ns/op). The five affected cases
all improved substantially: `method_call` was 0.8338x,
`captured_write` 0.8394x, `plain_function_call` 0.8463x,
`captured_read` 0.8621x, and `many_locals_call` 0.9401x. The two untouched
cases stayed neutral: `array_read` was 1.0021x and `property_read` 1.0045x.
These are exploratory local binaries without provenance receipts; use the
post-commit Performance Preview for hosted evidence.

The post-commit Performance Preview at
`a249e726734a46c4c17192eb9a94880ad0db6873` confirmed the direct-leaf VM
bypass on hosted Linux at 0.9021x overall (9.79% lower wall ns/op), with a 95%
confidence interval of [0.8989x, 0.9065x]. The five affected hosted cases were
`captured_read` 0.8200x, `captured_write` 0.8468x,
`plain_function_call` 0.8418x, `method_call` 0.8579x, and
`many_locals_call` 0.9317x. The two untouched cases measured `array_read`
1.0173x and `property_read` 1.0226x. All 21 linearity probes passed and all
three requested blocks were valid; the informational three-block cohort's
precision policy remained inconclusive. The same run measured 16.0050x
candidate/QuickJS-NG overall. CI and the full Test262 Coverage workflow were
green at this commit, which is the new hosted baseline.

Profiling that baseline found that every user-bytecode VM call still eagerly
constructed a cloned realm environment solely to probe a native-only fast
path before reaching direct-leaf dispatch. The VM now constructs that
environment only when the callee is actually native; fallback behavior for
native functions remains unchanged. A three-block comparison with seed
`20250922` measured 0.9643x overall. An independent five-block comparison with
seed `20250923` contained all 70 expected measurements and reproduced 0.9592x
overall (4.08% lower wall ns/op). The five affected cases all improved:
`method_call` was 0.9244x, `plain_function_call` 0.9351x,
`captured_read` 0.9377x, `captured_write` 0.9409x, and
`many_locals_call` 0.9719x. The two unaffected cases stayed neutral:
`property_read` was 1.0014x and `array_read` 1.0063x. These are exploratory
local binaries without provenance receipts; use the post-commit Performance
Preview for hosted evidence.

The post-commit Performance Preview at
`9ee468a55e09d3413cc48c90a2060655ffc79179` confirmed the deferred native
realm environment on hosted Linux at 0.9460x overall (5.40% lower wall ns/op),
with a 95% confidence interval of [0.9405x, 0.9542x]. The five affected hosted
cases were `plain_function_call` 0.8910x, `captured_read` 0.9123x,
`method_call` 0.9134x, `captured_write` 0.9256x, and `many_locals_call`
0.9751x. The two unaffected cases measured `array_read` 0.9995x and
`property_read` 1.0120x. All 21 linearity probes passed and all three requested
blocks were valid; the informational three-block cohort's precision policy
remained inconclusive. The same run measured 15.6116x candidate/QuickJS-NG
overall. CI and the full Test262 Coverage workflow were green. This commit
replaces `a249e726` as the hosted baseline.

The next profile also showed an avoidable shared-handle copy at both ordinary
and direct-leaf function entry. Call dispatch previously cloned the callee
`Value::Function` before inspecting it, even though `function_env` only needs
an owned callee for the uncommon internal-name and derived-constructor
bindings. Dispatch and `function_env` now borrow the existing callee and clone
it only when one of those bindings is actually installed. A three-block
comparison with seed `20250924` measured 0.9922x overall. An independent
five-block comparison with seed `20250925` contained all 70 expected
measurements and reproduced 0.9874x overall (1.26% lower wall ns/op). All seven
cases were below 1.0: `plain_function_call` was 0.9763x, `captured_read`
0.9814x, `method_call` 0.9859x, `property_read` 0.9871x, `array_read` 0.9895x,
`many_locals_call` 0.9944x, and `captured_write` 0.9974x. These are exploratory
local binaries without provenance receipts; use the post-commit Performance
Preview for hosted evidence.

The post-commit Performance Preview at
`3d46556b6440c528a03aab9f98dc69f5c262cfb4` confirmed the borrowed callee
handle on hosted Linux at 0.9903x overall (0.97% lower wall ns/op), with a 95%
confidence interval of [0.9903x, 0.9926x]. The hosted cases were
`captured_write` 0.9676x, `method_call` 0.9757x, `captured_read` 0.9894x,
`plain_function_call` 0.9924x, `array_read` 0.9941x, `property_read` 1.0063x,
and `many_locals_call` 1.0074x. All 21 linearity probes passed and all three
requested blocks were valid; the informational three-block cohort's precision
policy remained inconclusive. The same run measured 14.6463x
candidate/QuickJS-NG overall. CI and the full Test262 Coverage workflow were
green. This commit replaces `9ee468a5` as the hosted baseline.

Profiling that baseline showed the direct-leaf eligibility predicate still ran
three times per VM call: once before dispatch, again inside the direct helper,
and again while selecting the frame shape. The VM now performs the guard once
and passes an explicit internal `GuardedDirectLeaf` mode through call setup;
debug builds assert that the trusted mode still satisfies the full predicate.
General calls retain the complete eligibility check. A three-block comparison
with seed `20250926` measured 0.9916x overall. An independent five-block
comparison with seed `20250927` contained all 70 expected measurements and
reproduced 0.9941x overall (0.59% lower wall ns/op). All five call and binding
cases were at or below 1.0: `captured_read` was 0.9813x,
`captured_write` 0.9877x, `plain_function_call` 0.9902x, `method_call` 0.9970x,
and `many_locals_call` 0.9994x. The unrelated cases were `property_read`
0.9978x and `array_read` 1.0057x. These are exploratory local binaries without
provenance receipts; use the post-commit Performance Preview for hosted
evidence.

The post-commit Performance Preview at
`e6a325055850230244edd9ab3eb35c8148292bbd` reproduced the small direction at
0.9949x overall (0.51% lower wall ns/op), but its 95% confidence interval of
[0.9924x, 1.0060x] crossed 1.0. The hosted cases were `captured_read` 0.9700x,
`many_locals_call` 0.9867x, `method_call` 0.9873x, `captured_write` 0.9886x,
`property_read` 1.0003x, `array_read` 1.0154x, and `plain_function_call`
1.0166x. All 21 linearity probes passed and all three requested blocks were
valid, but this remains neutral hosted evidence rather than a new baseline.
The same run's candidate/QuickJS-NG point estimate was 15.0367x; retain
`3d46556b` and its 14.6463x result as the latest confirmed hosted baseline. CI
and the full Test262 Coverage workflow were green at `e6a32505`.

The following profile showed that direct-leaf setup still normalized and
stored `this` even when the compiled body never read it. Direct calls now omit
that value when bytecode contains no own `this` or `super` operation; the
existing direct-leaf contract already excludes closures, direct eval, and
other paths that could observe the binding indirectly. The first prototype
queried `uses_lexical_this` by scanning every opcode on every call: although it
measured 0.9874x overall in a five-block run, a focused ten-block run confirmed
that this regressed `many_locals_call` to 1.0171x. The retained implementation
caches the immutable predicate once when `Bytecode` is built, matching the
existing per-call metadata caches. A three-block comparison with seed
`20250931` measured 0.9806x overall. An independent five-block comparison with
seed `20251001` contained all 70 expected measurements and reproduced 0.9828x
overall (1.72% lower wall ns/op). All five call and binding cases improved:
`captured_read` was 0.9634x, `method_call` 0.9662x,
`plain_function_call` 0.9766x, `captured_write` 0.9766x, and
`many_locals_call` 0.9907x. The unrelated cases were `array_read` 1.0008x and
`property_read` 1.0064x. These are exploratory local binaries without
provenance receipts; use the post-commit Performance Preview for hosted
evidence.

The post-commit Performance Preview at
`2840537ee9479aaf38047a2b83c4d62d32df5f47` confirmed a larger 0.9570x
overall improvement on hosted Linux (4.30% lower wall ns/op), with a 95%
confidence interval of [0.9423x, 0.9636x]. The five affected hosted cases all
improved: `captured_read` was 0.9135x, `plain_function_call` 0.9241x,
`captured_write` 0.9260x, `method_call` 0.9580x, and `many_locals_call`
0.9778x. The unrelated cases were `property_read` 0.9967x and `array_read`
1.0071x. All 21 linearity probes passed and all three requested blocks were
valid; the informational three-block cohort's precision policy remained
inconclusive. The same run measured 14.4507x candidate/QuickJS-NG overall,
making this the latest confirmed hosted baseline. CI was green and the full
Test262 Coverage workflow was green at this commit.

With the direct-leaf predicate now authoritative, the next profile showed that
guarded calls still traversed the general `function_env` prologue and all of
its already-excluded constructor, name-binding, arguments, eval, and closure
branches. Direct leaf calls now use a dedicated prologue that retains the
observable dynamic-Function realm, module host/imports, private environment,
optional `this`, and parameter slots, while general calls keep the full path.
A three-block comparison with seed `20251002` measured 0.9964x overall. An
independent five-block comparison with seed `20251003` contained all 70
expected measurements and reproduced 0.9943x overall (0.57% lower wall
ns/op). All five call and binding cases improved: `many_locals_call` was
0.9843x, `method_call` 0.9858x, `captured_read` 0.9917x,
`plain_function_call` 0.9941x, and `captured_write` 0.9949x. The unrelated
cases were `array_read` 1.0048x and `property_read` 1.0049x. These are
exploratory local binaries without provenance receipts; use the post-commit
Performance Preview for hosted evidence.

The post-commit Performance Preview at
`0d6bd8f24308097f740bea10c77c7b38c2f6b386` confirmed a 0.9832x overall
improvement on hosted Linux (1.68% lower wall ns/op), with a 95% confidence
interval of [0.9790x, 0.9900x]. The affected hosted cases were
`method_call` 0.9630x, `captured_read` 0.9664x, `captured_write` 0.9687x,
`plain_function_call` 0.9825x, and `many_locals_call` 0.9878x. The unrelated
cases were `array_read` 1.0078x and `property_read` 1.0074x. All 21 linearity
probes passed and all three requested blocks were valid; the informational
three-block cohort's precision policy remained inconclusive. The same run
measured 14.0830x candidate/QuickJS-NG overall, making this the latest
confirmed hosted baseline. CI and the full Test262 Coverage workflow were
green at this commit.

The direct-leaf frame still initialized every local through the general
environment lookup path before authoritative parameter, `this`, and upvalue
slots replaced those results. Direct-leaf bytecode now initializes hoisted
locals directly to `undefined` and lexical locals to the uninitialized state;
the general VM path remains unchanged. A three-block comparison with seed
`20251004` measured 0.9772x overall. An independent five-block comparison
with seed `20251005` contained all 70 expected measurements and reproduced
0.9837x overall (1.63% lower wall ns/op). Four affected cases improved:
`many_locals_call` was 0.9666x, `captured_write` 0.9681x,
`captured_read` 0.9741x, and `method_call` 0.9806x;
`plain_function_call` was neutral at 1.0005x. The unrelated cases were
`array_read` 0.9979x and `property_read` 0.9990x. These are exploratory local
binaries without provenance receipts; use the post-commit Performance Preview
for hosted evidence.

The post-commit Performance Preview at
`fb59d3a9421127b852a017bb20c2eafc5c49aae4` confirmed a 0.9823x overall
improvement on hosted Linux (1.77% lower wall ns/op), with a 95% confidence
interval of [0.9807x, 0.9842x]. The affected hosted cases were
`captured_read` 0.9311x, `many_locals_call` 0.9651x,
`plain_function_call` 0.9802x, `captured_write` 0.9846x, and `method_call`
1.0021x. The unrelated cases were `property_read` 1.0008x and `array_read`
1.0145x. All 21 linearity probes passed and all three requested blocks were
valid; the informational three-block cohort's precision policy remained
inconclusive. The same run measured 13.8714x candidate/QuickJS-NG overall,
making this the latest confirmed hosted baseline. CI and the full Test262
Coverage workflow were green at this commit.

Direct-leaf calls cannot create closures, arguments aliases, or dynamic-scope
cells, but their frame setup still ran the general upvalue resolver over the
entire opcode stream and sorted its empty cell plan on every invocation. The
specialized initializer now installs only state that remains observable under
the direct-leaf guard: module-import cells, sloppy-global cells, and received
upvalues. A three-block comparison with seed `20251006` measured 0.9148x
overall. An independent five-block comparison with seed `20251007` contained
all 70 expected measurements and reproduced 0.9161x overall (8.39% lower wall
ns/op). All five affected cases improved: `many_locals_call` was 0.7791x,
`captured_read` 0.8751x, `captured_write` 0.9108x, `plain_function_call`
0.9377x, and `method_call` 0.9377x. The unrelated cases were `property_read`
0.9915x and `array_read` 0.9999x. These are exploratory local binaries without
provenance receipts; use the post-commit Performance Preview for hosted
evidence.

The post-commit Performance Preview at
`178e5a4ff8e42e09ce73b95155bc37658653d053` confirmed a 0.9076x overall
improvement on hosted Linux (9.24% lower wall ns/op), with a 95% confidence
interval of [0.9010x, 0.9128x]. All seven hosted cases improved:
`many_locals_call` was 0.7811x, `captured_read` 0.8740x, `method_call`
0.9064x, `plain_function_call` 0.9081x, `captured_write` 0.9299x,
`property_read` 0.9820x, and `array_read` 0.9883x. All 21 linearity probes
passed and all three requested blocks were valid; the informational
three-block cohort's precision policy remained inconclusive. The same run
measured 12.4314x candidate/QuickJS-NG overall, making this the latest
confirmed hosted baseline. CI and the full Test262 Coverage workflow were
green at this commit.

The specialized upvalue initializer initially queried the module-import map
for every local, even though ordinary functions have no imports and only
`from_env` slots can denote an import binding. Direct frames now check for any
module imports once and probe individual cells only for `from_env` locals. A
three-block comparison with seed `20251009` measured 0.9919x overall. An
independent five-block comparison with seed `20251010` contained all 70
expected measurements and reproduced 0.9919x overall (0.81% lower wall
ns/op). `many_locals_call` improved to 0.9695x, `method_call` to 0.9904x,
`captured_read` to 0.9905x, and `plain_function_call` to 0.9965x;
`captured_write` was neutral at 1.0002x. The unrelated cases were `array_read`
0.9941x and `property_read` 1.0025x. These are exploratory local binaries
without provenance receipts; use the post-commit Performance Preview for
hosted evidence.

The post-commit Performance Preview at
`a6db47633bc59ee41f8006b45ce805633c77e149` rejected that local result on
hosted Linux: candidate/base was 1.0135x (1.35% higher wall ns/op), with a 95%
confidence interval of [1.0018x, 1.0158x]. `method_call` regressed to 1.0402x,
`captured_write` to 1.0315x, `plain_function_call` to 1.0238x,
`property_read` to 1.0214x, and `captured_read` to 1.0136x; only
`many_locals_call` 0.9825x and `array_read` 0.9829x improved. The same run
measured 12.5696x candidate/QuickJS-NG overall. All 21 linearity probes, CI,
and the full Test262 Coverage workflow were green, so this was a performance
rejection rather than invalid evidence or a correctness failure. The import
probe optimization was reverted; `178e5a4f` remains the latest confirmed
hosted performance baseline.

An alternative attempt to store immutable BigInts behind shared handles did
reduce `Value` from 32 to 24 bytes, but a three-block same-machine run regressed
the seven-case geometric mean to 1.022x and slowed six cases. That experiment
was discarded rather than committed.

Precomputing whether bytecode parameters shadow `arguments` also failed the
retention threshold: its three-block run was 0.995x overall, but an independent
five-block run reversed to 1.003x and slowed four cases. That experiment was
discarded rather than committed.

At commit `18be69650953106355d425fd64412a13c384c648`:

- Latest CI and Test262 Coverage are green.
- Full CI aggregate burndown is recorded in
  `docs/conformance/burndown.jsonl` for 2026-06-15.
- Test262 comparison moved to:
  - quickjs-rust pass: 39781
  - quickjs-rust fail: 2429
  - quickjs-rust timeout: 396
  - quickjs-rust not-run: 66
  - actionable gap: 2773
- Local release probes on this machine:
  - `function f(x){return x+1}` loop, 20k calls: ~0.54s
  - `a.indexOf(4)` loop, 50k calls: ~0.20s
- The known
  `test/built-ins/TypedArray/prototype/set/typedarray-arg-src-backed-by-resizable-buffer.js`
  case still times out at the default 10s case timeout, despite measurable array
  index improvements. Remaining cost is still dominated by high-frequency
  Test262 harness calls such as `assert._isSameValue` and `compareArray`.

## Remaining Work

1. Profile one remaining timeout subtree at a time with `QJS_CLI_PROFILE=release`
   and a focused `find-qjsng-gaps.sh --filter <area> --all` run.
2. Prefer runtime fast paths that are generally valid for ordinary execution:
   function call setup, argument binding, local/global lookup, dense arrays,
   typed-array indexed reads/writes, and non-accessor method dispatch.
3. Keep semantic guard tests near the affected runtime behavior before removing
   any timeout exclusion or adding Test262 cases to the curated subset.
4. After a complete unfiltered CI coverage run, append the generated
   `test262-burndown` artifact with `./scripts/test262-burndown.sh --entry`.

Avoid papering over real runtime cost with broader timeouts or xfail updates
unless the case is intentionally stress-shaped and documented as such.

## Verification Gates

For runtime changes:

```sh
cargo test -p qjs-runtime
./scripts/compare-qjs.sh
./scripts/check.sh
```

For timeout/gap work, also run the focused subtree before and after the change:

```sh
QJS_CLI_PROFILE=release ./scripts/find-qjsng-gaps.sh --filter test/<area> --all --recommend-queue 20
```

For complete conformance accounting, use the CI `Test262 Coverage` artifact or a
local complete unfiltered scan. Do not record partial probes in the burndown
time series.
