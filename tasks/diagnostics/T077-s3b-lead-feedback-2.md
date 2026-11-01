# Lead feedback on S3b, round 2 (2026-11-01): a correctness regression on a public workload

The hostname blocker is resolved generically. Main `ed022fa1` compares host nodes by
short hostname, and the lead merged it into your branch as `55f5d1d6`. Do not apply
your alias proposal. Leave it as a record, or delete it if it is now stale.

The lead's public86 triage `20261101T013633Z-t077-s3b-lead-public86` (55f5d1d6 vs
3cc12c6f) aborted on a case-health failure. **kraken-1.1/audio-beat-detection fails on
the candidate**: `TypeError: Cannot read properties of undefined (reading 'vu_levels')`.
Base passes.

A 20-line reproducer is at `/tmp/qjs-lead/s3b-missing-arg-this.js`. It fails on the
first call in both default and `QJS_DISABLE_IC=1` modes, even when both arguments are
passed. Base prints `[1,1,0]`. Inside a prototype method, `this` evaluates to
undefined. The method has `this.lv[i] -= …` compound member assignments in if/else
branches, loops, and a `typeof(lus)=='undefined'` parameter reassignment. Simpler
shapes do not reproduce it.

Exact Test262, the NG fixtures and your counters all missed this. Do, in order:

1. **Root-cause it.** Identify which compiler, binding-plan, reference-plan or
   activation path mis-resolves `this`. Fix the class of bug, not this shape. Then
   audit the sibling implicit bindings (`this`, `arguments`, `new.target`,
   `super`/home object, the function self-name) on the same path.
2. **Add regression tests at the lowest layer**, plus a permanent NG fixture
   derived from the reproducer.
3. **Add a correctness sweep over the public workload corpus.** Run every workload in
   `benchmarks/manifest.json`, `benchmarks/generic-sentinels-manifest.json` and the
   external and development corpora (the same programs public86 runs, and all of
   development) with the candidate in default, IC-off and IC-verify modes. Compare
   stdout with each case's expected output. Use the existing tooling if something
   already does this; otherwise add a small script under `tools/benchmark/` or
   `scripts/` and document it. Held-out programs are acceptance-only: do not run them.
   Every case must pass.
4. Run the fast gate, the NG fixtures (default/IC-off/verify) and the sweep, then
   commit. **Do not start full or exact requalification yet.** Two independent lead
   reviews of `55f5d1d6` are in progress. The lead will send their findings so you
   can fix everything before a single requalification. Report the root cause, the
   fix, the tests and the sweep results, then stop.

iMac timing window: CST; no timing jobs at UTC 18:45–21:00. The `imac-bench.sh`
runner ref transfer is allowed. Never push to origin or merge into main.

## Addendum (lead, from an independent reviewer's minimization)

Minimal reproducer: `/tmp/qjs-s3b/findings/F1-REGR-this-member-after-equality-in-var-loop.js`

    function p(){var r=0;for(var i=0;i<3;i++){if(this.w==0) r+=this.v;}return r}
    var o={w:0,v:2,p}; function calc(){return o.p()} calc()

TypeError (reading 'v') in default/IC-off/IC-verify/binding-stress; base and NG return 6.
Trigger class: inside a `var`-for / while / do loop, an equality compare (==, ===, !=,
!==) on a `this` member, followed later in the same loop by any `this.P`/`this["P"]`
read or write (need not be in the branch: `r += this.w==0 ? this.v : 0` and
`if(this.w==0) r+=c; r+=this.v;` both fail). Fails for function, method shorthand,
class method and `new P()` bodies when called from JS code (function, arrow, or a
native callback such as map); top-level and eval-level calls work. Does not trigger
with `<`/`>`, `for (let …)`, `var t=this`, `typeof this`, `r=this`, arrow `this`;
`arguments`/`new.target` in place of `this` pass. A `var R=[]` array literal anywhere
in the function hides it — suggesting a specific compiled tier (typed loop / compact
or wide dispatcher admission, or the receiver slot in the tier's frame layout).

## Addendum 2 (lead, independent code-audit reviewer; both confirmed by the lead)

**R1 — root cause of the `this` regression.** When the wide compact tier enters a callee
inline it builds the activation with `enter_indexed_activation(function)`
(`function/call.rs:1424`), i.e. `CallOwner::root(Value::Undefined, ..)`; the real
receiver lives only in the driver's `current_this`. Sites: `compact_fn/wide/activation.rs:495`
(inline_entry, used by `wide/activation/entry.rs` try_enter_call) and `enter_constructor`
(`activation.rs:546/588`). If that inline frame takes a `WideOp::Exit`
(`exits.rs` exit_to_interpreter → resume_in_interpreter → `vm/wide_resume.rs`
resume_direct_call_bytecode), the VM is seeded with `env.activation.retain_continuation()`
(this = undefined); `slots.this_value` reaches only `Vm.direct_this`, and
`Vm::load_global("this")` reads `env.this_value()` from the call owner
(`vm_bindings.rs:163`). Exits come from SetProp/SetPropIndex/object-literal/sloppy
global-store fallbacks or typed-loop deopts. Second repro:
`/tmp/qjs-s3b-b/findings/F2-wide-exit-loses-this.js` (method and constructor variants).
The reviewer verified that setting the call owner with the real receiver (and
`new_target` for constructors) in try_enter_call/enter_constructor fixes all repros.
**Lead requirement:** do not stop at that patch. The receiver currently has two
authorities (call owner vs driver `current_this`/`Vm.direct_this`); make the
activation's call owner the single authority for `this`/`new.target`/home object on
every entry and resume route (inline wide entry, constructor entry, exits/resume,
standalone native-callback entry), delete the side channel or make it a derived view,
and add tests for each route.

**R2 — discarded call statements are deleted (also a pre-existing bug on main).**
`typed_loop/helper_graph/numeric.rs` `remove_dead_writes` treats a `NumOp::Call` whose
dst is never read as pure and removes it, so the callee never runs (side effects and
exceptions lost). Main/base already does this for top-level callees on direct calls —
`var n=0; function side(){n++} function h(a,b){side(); return a-b} h(1,2)` leaves
n = 0 and `function g(a,b){t(); return a-b}` swallows t's throw (NG: 1 / caught). The
candidate extends it to local closures and to call/apply/Reflect.apply/bound/sort/
toSorted/TypedArray sort/Array.from/Promise reactions (native callbacks now reach
`compact_fn::try_run_standalone`). Repro: `/tmp/qjs-s3b-b/findings/F1-dropped-call-statements.js`,
`/tmp/qjs-lead/dropped-top.js`. Fix it in its own commit (the lead may cherry-pick it
to main): give the numeric/compact plan optimizer an explicit effect model — only ops
proven pure may be removed; calls, property reads that can reach getters/proxies,
coercions that can call valueOf/toString, and anything that can throw are effectful —
and audit every DCE/forwarding pass in typed_loop/compact plans against it. Tests for
each route above, both before/after the fix.

Item 4 still applies: fix, test, sweep, commit, report, then stop before full/exact
requalification (reviewer A's fuzz results will follow).
