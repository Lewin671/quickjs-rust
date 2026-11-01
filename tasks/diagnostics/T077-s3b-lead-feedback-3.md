# Lead feedback on S3b, round 3 (2026-11-01): complete independent review findings

Two independent reviews of `55f5d1d6` are complete. Both returned **blocker**.

**Reviewer B (code audit)** found:
- no panics reachable from JS;
- no `unsafe` changes;
- no name-keyed snapshot or apply-back left;
- realm/error origin and stack depth improved over base.

**Reviewer A (differential fuzz)** ran about 7,300 seeded programs across six modes:
NG, default, IC-off, IC-verify, binding-stress and base. Its reproducers are in
`/tmp/qjs-s3b/findings/`, with generators and drivers in `/tmp/qjs-s3b/fuzz/` and
`/tmp/qjs-s3b/run6.sh`. Reviewer B's are in `/tmp/qjs-s3b-b/findings/`. Each item below
was confirmed by the lead or by a reviewer's patched scratch build.

Durable copies of all reproducers and the fuzz generators/drivers are in
`/Users/qingyingliu/Code/co-work-space/tmp/core-rebuild/s3b-findings/` (`a/`, `b/`, `fuzz/`)
in case `/tmp` is cleared.

Check the current state first. Finish and commit your round-2 work (R1, R2) if it is not
committed yet, then continue here.

## Must fix in S3b (candidate-only regressions and stress-mode defects)

- **R1 `this` lost** (round 2). Extra evidence from reviewer A:
  - The trigger is any equality compare on a property-loaded value inside a
    `var`-for/while/do loop (`a.k == 1`, or `q = a.k; if (q == 1)`), not only `this`
    members.
  - `this` is wrong for exactly the iteration after the first back-edge. That gives
    silent wrong values: `if(a.k==1) r+=(this===o)` over 4 iterations returns 3, not 4.
  - In that iteration `typeof this` says 'object' while `this.v` throws, so the fused
    this-property path and the plain path disagree.

  Your fix must make one authority for the receiver on every entry and resume route.
  Prove it with these shapes as tests.
- **R2 dropped discarded calls** and **P1 duplicated helper call**. P1 is pre-existing:
  a typed loop calls a local helper 101 times instead of 100. Fix both under one explicit
  effect model in the numeric/compact plan optimizer: only proven-pure ops may be removed
  or replayed. Audit every DCE, forwarding and deopt-replay path, including what is
  re-executed after a typed-loop deopt.
- **R3 self-referencing NFE binds free names to globals.**
  `F2-REGR-nfe-self-ref-free-names-bind-global.js`.
  - Repro: `var x='global'; function f(x){ return (function nf(){ nf; return x; })(); } f('param')`
    returns 'global'; base and NG return 'param'.
  - Inside a named function expression that mentions its own name, every free identifier
    that also exists as a global resolves to the global, for reads and writes. That includes
    counters like `function inc(){ inc; return ++count }` and callbacks like
    `[1].map(function cb(){ cb; return x })`.
  - Affects plain, strict and generator NFEs, in every mode.
  - Suspect: the round-1 self-binding demand change (539d6b8f) and the NFE scope descriptor
    or static free-name resolution skipping enclosing function scopes.
- **F4 direct-eval `var` hides a parameter.**
  `F4-REGR-eval-var-redeclares-param-nonsimple-no-expr.js`.
  - `function g(c, ...rest){ eval("var c;"); return c }` returns undefined; base, NG and
    the spec return 'kept'. Destructured parameters fail the same way.
  - With an expression-free non-simple parameter list, the var environment is the
    parameter environment. Keep your (correct) separate var environment only for lists
    that contain parameter expressions.
- **F5 `QJS_VERIFY_BINDINGS=1` panics** at `vm_binding_debug.rs:83` ("metadata policy:
  Throw vs Mutable") on valid programs, e.g. an eval-created script-level function writing
  a global `const`. 34 fuzz programs hit it.
- **F3/F6 `QJS_FORCE_EVAL_EXPOSURE=1` defects.**
  - F3: a `for (let j…)` containing a `while` with `this.q !== x` throws ReferenceError
    for `j`.
  - F6: a body `var` that redeclares a rest parameter reads as undefined.

  F3, F5 and F6 contradict the handoff's claim that the stress suites pass. Make the claim
  true and say how the suites missed them.
- Make the reviewer-B note an explicit invariant with a test: after an error in an inline
  frame, `unwind` clears frames without restoring `env.activation`. That is harmless only
  because the activation is discarded.

## Pre-existing binding-semantics bugs (base identical)

These are serious correctness bugs in code S3b now owns. Fix each in its own commit after
the items above, each with a regression test and an NG fixture. If one turns out to lie
outside the binding/scope model or to be large, do not fix it. Record it in the T077
Unresolved line with its repro and continue.
- `F-pre-try-block-function-decl.js`: a function declaration directly inside `try {}` or
  `finally {}` is never instantiated, sloppy or strict.
- `F-pre-inner-block-shadow-hides-captured.js`: in an inner function, a block-level
  `let z`/`catch (z)` breaks references to the outer captured `z` outside that block.
  Reads throw and writes are silently lost.
- `F-pre-default-param-closure-body-var.js`: a closure in a default parameter cannot see
  the outer variable when the body declares a same-named var.
- `F-pre-global-proto-lookup.js`: a global lookup through an `Object.prototype` getter
  throws ReferenceError.
- `with(o){ f\`x\` }` and `with(o){ f?.() }` do not use `o` as `this`.
- A labeled `continue` out of a nested block skips TDZ on re-entry.
- `F-pre-import-param-shadow.mjs`: a parameter that shadows an import resolves to the
  import inside `'lit'+p` and `[p]`.
- `F-pre-arguments-freeze.js`: `Object.freeze(arguments)` does not break the parameter
  mapping.
- `F-pre-evalscript-dup-lexical-over-prop.js`: a second evalScript that redeclares a global
  lexical does not throw SyntaxError.

The NG-side differences where the candidate matches the spec are listed in reviewer A's
notes. Do not "fix" those toward NG.

## Process

- Turn every reproducer above into a permanent test.
- Add reviewer A's binding/`this` generators (adapt from `/tmp/qjs-s3b/fuzz/gen*.py` and
  `drive.py`) to the repository's existing differential fuzz tooling if there is one.
  Otherwise add a small seeded runner under `tools/` that compares candidate modes against
  NG and against a base binary. Document it and run at least 2,000 seeds clean before the
  handoff.
- Then do the single full requalification on the final source:
  - full gate;
  - compare-qjs default/IC-off/verify;
  - the stress suites;
  - complete exact Test262 cached and IC-off with burndown;
  - depth parity, counters;
  - the public-corpus correctness sweep from round 2;
  - regenerate the x86 order;
  - public86 vs `3cc12c6f` (the hostname guard is fixed).

  S3b is a payoff stage: every public86 lane must improve. Report and stop. The lead runs
  formal, held-out and RSS acceptance plus a re-review.

iMac timing window: CST, so no timing jobs at UTC 18:45–21:00. The `imac-bench.sh`
runner ref transfer is allowed. Never push to origin or merge into main.
