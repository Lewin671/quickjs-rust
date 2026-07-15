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
use the post-commit Performance Preview for hosted evidence.

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
