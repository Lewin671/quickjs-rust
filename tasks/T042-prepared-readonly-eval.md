# T042: Consume prepared read-only eval environments once

- Status: active.
- Implementation: integrated at `67766d1c` on `agent/performance-campaign/lead`;
  no projection-index cache, COW or T041 implementation is included.
- Verified at: `67766d1c`, 2026-10-12; full local gate with 5170 Test262 cases,
  2264 runtime tests, 22 focused perf-counter tests, 23 diagnostics and 220 NG
  fixtures. Production code equals `9ad0ab76`; owner `f1c1b612` CI is green, and
  the integrated full local gate plus NG comparison passed.
- Evidence: [plan](performance-units/prepared-cached-readonly-eval-owned-execution.json)
  binds the T040 queue and six receipts. Raw files are workspace-local,
  not retained; repeated-source profiles are diagnostic, not formal timings.
- Unresolved: integrated CI, the layout pin's reproducible two-cycle, screens
  and promotion. No screen attempt has been used.
- Next action: inspect the saved A/B link evidence, fix the address model,
  verify a stable target layout, then use the frozen screen.

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
First text symbols are at `0x100004850` and `0x100000860`; a static header
diagnostic (job `20261013T053409Z-layout-header-diagnostic`) verifies B's
`__text` start is `0x100000860`, aligned to 16 bytes. The exact cause still
needs A/B evidence; increasing relinks alone does not resolve this cycle.
These are build/address checks, never screen attempts or performance evidence.
Raw files are workspace-local, not retained; do not accept the requested
offsets merely because they appear in the order-file header.

Pure Python replay of both snapshots reproduces their next heads exactly.
All 149 admissible filler size/name/count tuples and the 556-line ranked tail
are identical. A start low bit pattern `0x850` chooses mmap `0x200` plus
read_dir `0x1f0` and links B; B start `0x860` chooses print_raw_with_column
`0x3e0` and links A. Thus the pin follows the previous binary's text start;
alias, filler-span and ranked-tail drift do not explain this run. B's Mach-O
header ends at `0x840` and its final eh_frame ends at the TEXT segment end.
Next diagnostic: relink once with saved `10-before.order`, save A's complete
otool/nm/binary without repinning, then compare A/B segment/section/header,
unwind and function-starts data. More identical iterations cannot fix a cycle.

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
