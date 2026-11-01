# Lead feedback on S3b, round 4 (2026-11-01): recover the payoff without losing correctness

Your final qualification `7b3ed258` passes correctness: exact 42,672/42,672 in both
modes, 318 NG × 7, the public sweep, depth parity and 6,000 differential programs.
The lead accepts that correctness standing as the floor for this round.

Performance fails. Public86 `20261101T043615Z-t077-s3b-final-public86` vs `3cc12c6f`:

| Lane | Wall | Instructions |
| --- | ---: | ---: |
| development | 1.087 | 1.108 |
| external | 1.253 | 1.276 |
| broad | 1.079 | 1.096 |
| sentinel | 1.005 | 1.052 |

The lead bisected the loss: `55f5d1d6` (before the review fixes) already shows
development 1.083, external 1.250 and broad 1.067. The loss is structural to S3b, not
caused by the correctness repairs.

An independent investigator attributed the loss with deterministic instruction counts,
coverage and traces. Read **`/Users/qingyingliu/Code/co-work-space/tmp/core-rebuild/s3b-perf/attribution.md`**
in full. The reproducers are in `s3b-perf/repro/` and the per-case counter data is in
`s3b-perf/allcnt/`. The five root causes, ranked by share of the loss:

1. **~50%: configurable or missing global names are treated as unstable references.**
   `reference_plan.rs` `stable()` then disables typed loops, compact and wide entry,
   and every op pays a by-name reference event. `stable()` is per site, but `can_skip`
   is re-proved on every call. A2: the typed-loop sloppy-global sink is declined after
   the first call because canonical realm-cell routing clears its local value copy.
2. **~20–25%: every JS call builds a full ActivationContext**, adding 435–650
   instructions per call: eager identity, an Rc clone, three cold borrows, and a
   move into the WideFrame and back. Captured writes also pay TDZ/metadata checks and
   read `env.caller().strict` every time.
3. **~7%: closures in eval-extensible scopes lose direct-leaf/wide admission**
   (`lexical_scope().is_some()`, `indexed_bindings_only`).
4. **~4%: closed-form leaf calls rescan plan purity on every call**, and build
   `loop_context()` per call.
5. **~10–15%: per-op VM checks** (pc_scopes, reference hooks and event cursor,
   authority compares) and by-name global routing in VM code.

## Do, in order

Fix causes 1→5. Each fix must be structural and general. Use the attribution's
proposals as starting points; replace one with a better design if you have one, and say
why. Hard constraints:

- **Keep the S3b architecture.** That means the typed Context, compiled binding
  plans/leases/cells, and the protected env-model boundary (no per-call name-keyed
  snapshot or apply-back).
- **Keep every correctness fix and test from rounds 1–3.** In particular, the B3b2
  reference-before-RHS semantics must stay correct. The proposed shape for cause 1 is
  compile-time-stable sloppy and bound-strict sites, plus a realm
  `global_lexical_epoch` checked after calls that pins the affected interval and enables
  hooks only for that frame. Prove it with tests for a global lexical created during an
  RHS, delete or recreate during an RHS, strict missing names, and eval-declared
  globals.
- No benchmark recognition. Admission must depend only on what the code does.
- Add reason tracing and counters for the currently silent rejections (`can_skip`,
  `indexed_bindings_only`, hook-blocked loop edges) so this cannot hide again.

**Measuring progress.** Measure each cause with deterministic local instruction counts
on the reproducers and base/candidate counter maps. After causes 1 and 2, and again at
the end, run a public86 quick triage vs `3cc12c6f` through the iMac queue.

**Target.** S3b is a payoff stage, so every public86 lane must be below 1.00 vs
`3cc12c6f`, wall and instructions. If all five causes are fixed and a lane is still
above 1.00, stop and report a per-case classification instead of iterating.

## Then requalify once on the final source

Correctness must remain at the `7b3ed258` standing. Run:

- full gate;
- NG in all seven modes;
- stress suites;
- the public correctness sweep;
- the seeded differential runner, ≥2,000 seeds per profile, triaged;
- complete exact Test262 cached and IC-off with burndown;
- depth parity and counter maps.

Then regenerate the x86 order and run the final public86. Report and stop; the lead runs
formal, held-out and RSS, plus a re-review.

The iMac runs CST, so no timing jobs at UTC 18:45–21:00. The `imac-bench.sh` runner ref
transfer is allowed. Never push to origin or merge into main.
