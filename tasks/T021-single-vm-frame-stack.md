# T021: Single-VM Frame Stack And Compact Execution Core

## Goal

Remove recursive per-call VM construction from ordinary synchronous bytecode
calls, then compact the same execution core into register-oriented or
superinstruction dispatch. This is the next structural T018 unit: it must
produce general external wins while preserving the current zero-gap Test262
baseline. It is a foundation for the final every-case `<= 0.50x` QuickJS-NG
contract, not permission to specialize benchmark identities or loop shapes.

## Current Evidence

Trusted-main Performance Preview run `29865188694` at
`b8d0c2385128ad823a18488326a64299cffc3b2a` reports:

- broad overall candidate/QuickJS-NG `0.3322x`, but the apparent aggregate win
  hides `top_level_function_call 9.9898x`, `dynamic_method_call 7.0351x`,
  `array_write 4.7977x`, `array_allocation 3.3963x`, and
  `object_allocation 2.6901x`;
- critical families still above the final per-family boundary are allocation
  `2.6066x`, array `0.6048x`, and string `0.5402x`;
- external candidate/QuickJS-NG is `8.160x` on JetStream (5/5 comparable),
  `4.912x` on Kraken (7/14 comparable), and `7.649x` on SunSpider (26/26
  comparable), with zero qjs-rust wins. The two focused call gates are
  JetStream `hash-map 9.0627x` and SunSpider
  `controlflow-recursive 10.7814x`;
- a local `sample(1)` profile of JetStream `hash-map` attributes about 29% of
  samples to `Vm::run_completion`, about 17% to allocation/free, and about 7%
  to `Value` clone/drop. Per-call `Vm`, `CallEnv`, locals, and operand-buffer
  construction remain visible.

The matching Test262 Coverage run `29865445340` is the correctness floor:
42,672 configured cases passed, with zero failures, timeouts, not-run cases, or
actionable QuickJS-NG gaps. No performance unit may lower that result.

## Architectural Decision

Use one execution semantics and this order:

1. collect the existing VM's per-invocation fields into an owned `FrameState`;
2. trampoline only the existing direct-leaf eligible ordinary synchronous
   bytecode calls on `Vec<FrameState>`;
3. replace dispatch inside that same executor with compact register operations
   and/or superinstructions;
4. attack remaining object/array/string/RegExp/JSON costs from independent
   external profiles.

Do not build a separate numeric/control VM. A second VM would duplicate
completion, exception, binding, call, and deoptimization semantics and would
turn every future Test262 fix into a two-engine maintenance problem. A JIT is
also out of scope while the repository forbids FFI and lacks a stable compact
IR, safepoints, and deoptimization state.

## Scope

- Allowed paths for R1/R2: `crates/qjs-runtime/src/bytecode/`, the minimum
  `crates/qjs-runtime/src/function/` call preparation needed to reuse existing
  direct-call slots, and focused runtime tests.
- Allowed paths for later compact dispatch: the existing bytecode IR/compiler
  and the same executor; each slice gets its own measured commit.
- Forbidden paths: `third_party/`, benchmark identity/path/checksum branches,
  an independent second VM, widening `is_direct_leaf_function` in the same
  commit as trampoline routing, or weakening Test262/benchmark coverage.
- No new dependency, `unsafe`, FFI, or platform-specific executable-memory
  work in R1/R2.
- Global docs, task files, Cargo files, and benchmark protocol files remain
  integration-owner only.

## Parallel Assignment

- Each coding owner starts from an exact recorded `main` SHA in an isolated
  `agent/<task-slug>/<owner-id>` worktree.
- R1 and R2 serialize because both own VM state and call control flow.
- Independent read-only profiling and completed-CI artifact analysis may run
  concurrently. After local verification, push promptly and continue the next
  isolated unit while hosted CI runs asynchronously.
- Integration owner: main agent.

## R1: Owned Frame State

Move the existing per-invocation fields (`bytecode`, instruction pointer,
operand stack, locals/upvalues, `CallEnv`, try/pending completion state,
`with`, disposal, realm/module host, and caches) behind one owned frame value.
Keep all existing opcode handlers and all existing recursive call routing.
Avoid a self-referential frame: operand storage owns its `Vec<Value>`, while a
borrowed root bytecode or shared nested bytecode owner supplies recycling and
instruction access.

Acceptance:

- no observable behavior or call eligibility change;
- focused runtime tests, full `qjs-runtime`, Test262 slices, `compare-qjs.sh`,
  and `check.sh` pass;
- no focused external case is above `1.03x` candidate/base and the complete
  broad call family is not above `1.02x` candidate/base;
- if the state move itself regresses those bounds, fix it before R2.

## R2: Direct-Leaf Trampoline

Route only functions already accepted by `is_direct_leaf_function` through the
explicit frame stack. Numeric-leaf execution keeps its current priority.
Native, Proxy, bound, constructor/super, generator/async, direct-eval, `with`,
closure-producing, and every guard-false call retains the old path.

Each frame owns an independent operand `Vec<Value>` so the parent operand stack
and `TryFrame.stack_depth` remain frame-relative. Entering a child must release
any borrow of the parent opcode before swapping frames. A true child return
runs the child's own `finally` handling before restoring the parent; an
unhandled `RuntimeError` must be offered to each restored parent's existing
catch/finally machinery without rebuilding the thrown value.

Focused coverage includes:

- 10,000-level eligible recursion without Rust-stack growth;
- parent operand preservation such as `10 + leaf(3) * 2`;
- zero/one/two/three-argument order and direct-call slot behavior;
- multi-frame thrown-value identity and caller/callee `finally` ordering;
- repeated frame-buffer reuse without retained values;
- explicit fallback tests for eval, `with`, closures, generators, async, class
  construction, Proxy, and bound functions.

Promotion requires all of the following:

- JetStream `hash-map` and SunSpider `controlflow-recursive` each `<= 0.90x`
  candidate/base;
- broad `top_level_function_call <= 0.80x` candidate/base and complete call
  family `<= 0.90x` candidate/base;
- eligible-call profiles no longer show per-call `Vm` construction, fresh
  operand/local vectors, or a full recursive bytecode-evaluation entry;
- no external case exceeds `1.03x` candidate/base, no critical family exceeds
  `1.02x`, and the Test262 correctness floor remains unchanged.

If both external call cases fail to improve by at least 10%, stop extending
the fast-path predicate. Retain a non-regressing explicit-frame foundation and
move directly to compact dispatch in the same executor.

## R3: Compact Dispatch In The Same Executor

Introduce a deterministic, fixed-width register or superinstruction form
inside the existing executor. Lower only complete, prevalidated functions; an
unsupported function falls back before executing observable work. Expand
coverage in measured semantic families, with differential tests for `NaN`,
`-0`, Infinity, BigInt errors, short-circuiting, TDZ, exceptions, and calls.

Do not close T021 merely because a subset improves. R3 remains open until its
benefit generalizes across at least two independent external suites and the
next dominant shared runtime cost is outside dispatch/call-frame mechanics.

## Final Acceptance

- T018's strict contract is met: every broad and pinned external case is
  runnable and `<= 0.50x` QuickJS-NG, with every suite/family aggregate also
  `<= 0.50x`.
- Two independent complete benchmark runs confirm the result.
- The current configured Test262 inventory remains zero-failure, and focused
  tests cover every changed semantic boundary.
- `./scripts/check.sh` and `./scripts/compare-qjs.sh` pass.

## Verification

```sh
cargo test -p qjs-runtime
./scripts/test262-subset.sh
./scripts/compare-qjs.sh
./scripts/check.sh
```

Fast broad call-path diagnostics are deliberately not formal portfolio claims:

```sh
QJS_FRAME_GATE_DIR="$(mktemp -d /tmp/qjs-frame-gate.XXXXXX)"
./scripts/benchmark.sh \
  --candidate target/release/qjs \
  --base /tmp/qjs-frame-gate-base \
  --quickjs-ng third_party/quickjs-ng/build/qjs \
  --filter call --blocks 5 --seed 20250713 \
  --output "$QJS_FRAME_GATE_DIR/call.jsonl"
```

Snapshot `/tmp/qjs-frame-gate-base` from the exact base commit before building
the candidate. A focused raw file is valid smoke evidence but cannot be passed
to `benchmark-report.sh`: the formal validator intentionally requires all 25
manifest cases, complete coverage, and verified three-role receipts.

For a low-latency independent external gate, derive a temporary manifest that
contains only JetStream `hash-map` and SunSpider `controlflow-recursive`, then
run the existing hash-verified adapter:

```sh
QJS_FRAME_EXT_DIR="$(mktemp -d /tmp/qjs-frame-ext.XXXXXX)"
jq '
  .suites |= map(
    if .id == "jetstream3-js-subset" then
      .cases |= map(select(.id == "hash-map"))
    elif .id == "sunspider-1.0" then
      .cases |= map(select(.id == "controlflow-recursive"))
    else empty end
  )
' benchmarks/external-preview.json > "$QJS_FRAME_EXT_DIR/manifest.json"
./scripts/external-performance-preview.sh \
  --manifest "$QJS_FRAME_EXT_DIR/manifest.json" run \
  --cache-root target/benchmarks/external-cache \
  --work-root "$QJS_FRAME_EXT_DIR/work" \
  --output-dir "$QJS_FRAME_EXT_DIR/result" \
  --candidate target/release/qjs \
  --base /tmp/qjs-frame-gate-base \
  --quickjs-ng third_party/quickjs-ng/build/qjs \
  --blocks 3 --timeout-seconds 15
jq '.suites[] | {id, cases: [.cases[] | {
  id, candidate_over_base, candidate_over_quickjs_ng, capability
}]}' "$QJS_FRAME_EXT_DIR/result/external-report.json"
```

Only complete, healthy same-host portfolio runs may update the formal T018
score. Local gates decide whether to push; completed hosted artifacts confirm
the unit asynchronously and never block work on the next isolated slice.

## Notes

The first correctness prerequisite is stable Realm intrinsic prototype
identity. Object and array literals must not follow a later reassignment of the
global `Object` or `Array` binding; land that regression fix before extracting
VM frames so the structural rewrite starts from correct semantics.
