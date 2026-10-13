# T042: Consume prepared read-only eval environments once

- Status: active.
- Implementation: integrated at `67766d1c` on `agent/performance-campaign/lead`;
  no projection-index cache, COW or T041 implementation is included.
- Verified at: `1bbbd875`, 2026-10-12; full local gate with 5170 Test262 cases,
  2264 runtime tests, 22 focused perf-counter tests, 23 diagnostics and 220 NG
  fixtures. Production code equals `9ad0ab76`; owner `f1c1b612` CI is green, and
  the integrated full local gate plus NG comparison passed. The layout
  commit full gate and integrated CI at `1bbbd875` are green.
- Evidence: [plan](performance-units/prepared-cached-readonly-eval-owned-execution.json)
  binds the T040 queue and six receipts. Raw files are workspace-local,
  not retained; repeated-source profiles are diagnostic, not formal timings.
- Unresolved: measurement-startup diagnosis and promotion. Screen attempt 1 of 2 failed;
  no formal measurement is authorized by that result.
- Next action: verify the shared symmetric launch-preflight method from
  T017, then spend the remaining single screen attempt; no gate changes.

## Scope and target architecture

The current cached direct-eval caller already obtains an immutable blueprint
and an independent owned projection. It then discards the blueprint handle,
enters native eval, repeats context/cache work, clones the projection, and
clones it again into the evaluator. The proposed route consumes that existing
projection once in the same evaluator; it never moves the caller's own env.

Admission is a direct call to the current realm's stable eval intrinsic,
string argument, cache hit with the original parse-context key, declaration-
free/closure-free/template-free blueprint, no binding writes or deletes,
nested eval, with or suspension, and no parameter/dynamic-realm context.
All other cases retain the native chain. Global eval rebinding cannot change
intrinsic authority; store a weak identity once per realm, with conservative
fallback for unregistered synthetic realms and foreign/bound/Proxy callees.

Read-only bytecode can call side-effecting functions. Current canonical cells,
objects, realm and private/module context remain live. The caller refreshes
realm-backed slots on success and error, then uses its existing result/throw
handler. The ordinary evaluator initializes `this` and creates fresh runtime
values. The proof removes redundant ownership layers, not actual execution.

Implementation stages within this leaf:

1. Register stable intrinsic identity and introduce prepared owned execution;
   do not change projection scanning or FrameBindings storage.
2. Prove hit/fallback behavior and semantics, then use the plan's bounded
   screens and complete formal promotion. No stage alone is a speed claim.

Owner paths: `bytecode/mod.rs`, `bytecode/vm_call.rs`, `global.rs`, `function/env.rs`, their
existing eval-cache/direct-eval tests, and necessary test/perf counter hooks
under `crates/qjs-runtime/src/`. The lead owns plans, docs and order files.
No AST, dependency, general call ABI, COW, projection-slot cache or writes
optimization belongs to this unit.

The owned evaluator lives with the bytecode execution entries; `global.rs`
only shares its existing initializer and registers the stable intrinsic.
The env test module was extracted to stay within source-size limits. Direct
SuperCall/Spread are rejected as implicit binding writes; live-slot/cell
inspection also excludes uninitialized derived-this contexts. [T043](T043-derived-constructor-eval-context.md)
records two original-native semantic gaps reproduced at the exact base and
unchanged by the complete guarded candidate delta. They are not claimed fixed.

## Layout diagnostic checkpoint

Jobs `20261013T050459Z-prepared-eval-layout` and
`20261013T052238Z-prepared-eval-layout-relinks` use exact production revision
`9ad0ab76`; the latter preserves every order/nm snapshot. After warmup, orders
alternate between SHA `f989072deb9e11c6c16973eb6557e420d1414b63f68bd48c7b31cd03b2c2bbda`
and `fec37734e2b6a5848662ba6afb1c91ab4c5c3438b628cb2e7d221773b94edb8e`.
Hot callee order/budgets and selected filler sizes stay unchanged; std filler
selection and head count differ. Actual VM/wide page offsets alternate
`0xc30/0x3f0` and `0xc50/0x410`, bracketing requested `0xc40/0x400`.
First text symbols are at `0x100004850` and `0x100000860`. Full A/B
headers have the same 15 commands, header end `0x840`, text size `0x514620`
and other TEXT section sizes. Compact-unwind differs by 8 bytes: the first
compressed page in A has one more entry and local encoding. TEXT sizes become
`0x60c000/0x608000`, and front padding becomes `0x4010/0x20`. Reordering
therefore changes unwind metadata and segment packing, which moves text's
starting page offset; repinning follows the previous link and repeats the cycle.
Filler sizes, hot budgets and ranked tail are identical, and pure-Python replay
reproduces both next heads. Extra identical relinks cannot resolve the feedback.

Job `20261013T055306Z-prepared-eval-sectalign-layout` uses production `9ad0ab76`
with only the proposed linker section alignment. `-sectalign __TEXT __text 1000`
fixes the starting page offset at zero: otool reports `0x100004000`, align
`2^12`. The regenerated order stabilizes after one relink, and actual VM/wide
page offsets are `0xc40/0x400`, unchanged from the requested targets. Order SHA
`3314cfdd69c20a6faf919c0929f818c89f4b8cb1b21950cd2df594e41d31eccd`; binary SHA
`e079067920853be584e0c022c854ccee46e6992fbdd0ddfe05752817e9e5bdf3`.
The integrated flag applies only to macOS x86_64; arm64's existing layout remains.
Raw headers, orders and metadata are workspace-local, not retained. These are
build/address checks, not screen attempts or performance qualification.

## Screen record

- Attempt 1, source `1bbbd875`, job `20261013T060218Z-prepared-eval-screen-attempt1`:
  **FAIL**, no DISTURBED flag and an empty intruders log. Exact candidate bytes
  equal the layout probe; exact base bytes equal the frozen queue. Tofte wall
  median `0.8744`, pairs `[3.7384, 0.8492, 0.8744, 0.8745, 0.8514]`, instruction
  median `0.8886`. Every control/sentinel median is at most `1.0195`, within
  `1.03`. The first target pair fails the required every-pair improvement;
  no outlier is removed, no formal run follows, and identical reruns are not
  justified by the other pairs. Raw JSON is workspace-local, not retained.

Byte-identical fresh/cached controls subsequently reproduce new-file first-launch
wall overhead with unchanged child CPU; symmetric tiny preflight removes that
penalty in independent sets for both exact executable hashes. See
[the shared-method diagnosis](archive/T017-launch-preflight-diagnostic.md).
Screen 1 remains failed and consumes one attempt. The shared preflight records
cold launch separately without changing production bytes, thresholds, measured
pairs or the remaining budget. Only its verified method change justifies using
attempt 2; a second failure stops this unit without a formal run.

## Profile interpretation

Original tofte call-lifecycle envelopes are 44/116, 53/102 and 45/103.
Thirty-two top-level source copies yield 1266/3300, 1299/3296 and 1238/3294;
their native-eval subsets are 640, 701 and 699, with clone subsets 115, 106
and 109. These nested numbers are never added or called wholly removable.
The larger diagnostic distinguishes repeated environment ownership from a
small slot-index scan; the original samples independently show the same path.

## Required coverage and verification

Confirm cold fallback and warmed prepared hits; changing caller values,
different callers sharing a source, prior eval-added names, callbacks mutating
cells/realm and callbacks throwing; strict mode, TDZ and writes/deletes;
declarations, parameters, with and nested eval fallback; stable intrinsic
rebinding, foreign/bound/Proxy and dynamic-realm fallback; extra arguments,
RegExp identity, arguments, private/super/new.target and module context.
Loop tests have in-script guard counters.

Follow [the performance workflow](../docs/performance-workflow.md). The frozen
plan owns thresholds and attempts. Record every screen, complete external and
broad results, sentinels and exact-candidate Test262 evidence before promotion.
