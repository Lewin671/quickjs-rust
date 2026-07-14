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
1.011x. This is an exploratory dirty-source result; use the post-commit
Performance Preview for the provenance-backed decision.

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
