# T056 M1 stage 1: value facade and atom linking

Stage 1 of the frozen three-stage migration
[`interpreter-core-v2-atoms-layouts`](../performance-units/interpreter-core-v2-atoms-layouts.json)
(base `351004f3`, stage cap 1.025 cumulative). Owner
`agent/t056-m1-atoms/claude`, branched from main `a0b6bb23`; implementation
commit `11c9fc08ecda19cff3b729f8c01b654ddac99c84`.

## What the stage built

| Piece | Where | Contract |
| --- | --- | --- |
| Atom domain | `property/atom.rs` | Interns the canonical code units of a string key (a pair stored whole or as two escaped halves is one key; lone surrogates and sentinel-range scalars stay distinct). One live atom per key, so equality is entry identity. Ids come from a monotonic `u32` counter, are never reused, and exhaustion is a `RangeError`, never a wrap. The table holds weak references and sweeps when it doubles past the live size. `"01"` and `"1"` are distinct atoms; only canonical indices below 2^32-1 carry `array_index`. Symbols/private names never enter a domain. |
| Domain ownership | `function/env.rs` `RealmState::atoms` | Each independent root realm owns a domain. Dynamic `Function` scope realms (`function/prototype.rs`, `Function::new_user`) share their creator's domain because they exchange objects. Script dynamic import already shares the script realm; module graphs and detached conversion environments are roots. |
| Symbolic pool | `bytecode/linked_code.rs` `PropertyNamePool` | `Bytecode` construction assigns every `GetPropNamed`/`SetPropNamed` operand an index into the body's deduplicated name pool. The pool holds names, never ids, so reused eval/Function/module code carries no domain integers. |
| Cached link | `bytecode/linked_code.rs` `AtomLinks` | First named dispatch under a domain interns the whole pool once. The primary link (first domain) is a `OnceCell`, so later dispatches borrow the atom with no count update. Other or dead domains use at most four foreign links, re-interned by content and cloned out per dispatch. Links record their domain weakly, which pins its allocation, so a reused address never matches a dead domain. |
| Interpreter use | `bytecode/vm/general_ops.rs` | `op_get_prop_named`/`op_set_prop_named` resolve the operand through the frame realm's domain and key the existing text-based path with the atom's canonical text. Compact, typed-loop and leaf tiers still read the op's symbolic `key`; stores still compare text (M1.2/M1.3). |
| Value facade | `value/access.rs` | Public `ValueKind` (language type, symbols separate), `kind`, `is_undefined/null/nullish/object`, `as_number/boolean/js_string`, constructors; crate `as_ordinary_object`. Every view borrows or copies an immediate. `Value` stays 16 bytes; `Op` stays within its size bound. Adopted by the CLI and the named-property handlers. |
| Counters | `diagnostics.rs` | `property_atoms_created`, `named_operand_links` (perf-counters builds only). |

Super named operands (`SuperGet/SuperSet/SuperMethod`), literal shapes and
class keys are not yet in the pool; they join when their stores adopt layout
authority.

## Verification at `11c9fc08`

- Lowest-layer tests: atom identity, numeric spellings, lone surrogates and
  escaped pairs, sentinel-range scalars, domain separation, id non-reuse,
  10,000-key churn bounded by live atoms, exhaustion (`property/atom/tests.rs`);
  pool assignment, link once per domain (100 re-dispatches, zero new atoms),
  foreign re-interning, dead-domain bounds, function/module pools, steady-state
  loops, repeated eval and `Function` bodies, two roots on one body, dynamic
  `Function` domain sharing (`bytecode/linked_code/tests.rs`); facade size and
  ownership (`value/access.rs`); JS semantics of named vs computed keys,
  numeric spellings, surrogates, symbols/private names, getters/Proxy traps,
  enumeration order and objects crossing eval/Function code
  (`tests/objects/named_keys.rs`); a perf-counters test that 3 versus 3,000
  interpreted calls give equal link and atom counts (`tests/diagnostics.rs`).
- Fast gate (`check-touched.sh --staged --explain`): context, fmt, clippy,
  file sizes, 13 CLI tests, 2,299 runtime tests, 117 selected Test262 cases.
  Clippy with `agents` and with `perf-counters` clean.
- Local (M1 Pro, correctness only): curated Test262 subset 5,170/5,170;
  `compare-qjs.sh` 248 fixtures ok; `find-qjsng-gaps.sh --exact --all` on a
  non-`agents` debug build: 53,572 total, 42,672 configured, 0 Rust failures,
  0 timeouts, 62 not run (agent-only cases), recorded as the `local-exact`
  burndown line for this commit.
- iMac job `20261017T145515Z-t056-m1s1-full-check` (no-timing): every
  `check.sh` stage passed, including the 5,170-case Test262 subset, agents and
  perf-counters tests and the tool tests. The chained `compare-qjs.sh` then
  reported one mismatch, `date-basic.js`'s local-time constructor upper bound
  under the iMac's non-UTC host zone: the known host-zone difference recorded in
  T044/T047 (Rust's local Date policy is UTC). Date is untouched here; the local
  run passes all 248 fixtures.

## Known pre-existing discrepancy (not introduced here)

A computed key built by joining two surrogate halves (`o['\uD83D' + '\uDE00']`)
does not reach the literal pair's property (`o['\u{1F600}']`), although the
strings are equal: stores compare stored bytes. Observed on the base build;
M1.2's atom-keyed stores remove it because atoms canonicalize code units.

## Screen

Command: `perf-loop.sh --plan tasks/performance-units/interpreter-core-v2-atoms-layouts.json`
through `imac-bench.sh loop`, candidate `11c9fc08` against plan base `351004f3`.

- Job `20261017T151858Z-t056-m1s1-screen1` (output
  `target/layout-attempts/t056-m1s1-attempt1` on the iMac runner) built and
  audited all twelve variants, measured natural-placement broad cases, then
  stopped at `sentinel/recursive_call_tree` with the tool error `load snapshot
  does not cover guard/monitor completion`: the snapshot was sealed 7.99853 s
  after the last pair, against the 8 s settle (`pair_disturbance.eligibility`
  compares wall-clock stamps taken around `sleep(8)`). No verdict exists, so no
  attempt is consumed; it is replaced whole, never pooled.
- Replacement job `20261017T181346Z-t056-m1s1-screen1-replacement2`, output
  `target/layout-attempts/t056-m1s1-attempt1-replacement`, exit 1 after 5,077 s
  with the same tool error: variants 0 and 1 complete (86 cases each), variant 2
  stopped at `external/jetstream3-js-subset/raytrace-public-class-fields` with
  the snapshot sealed 7.95109 s after the last pair. No verdict; no attempt
  consumed. Timing samples of either job were not read.
- Blocker: twice the iMac wall clock advanced less than the 8 s `sleep` it
  surrounded (by 1.5 ms and 49 ms), so the guard fails on host clock
  adjustment rather than on load. Repeating the whole screen is a gamble of
  about 2.5 h per try; the screen waits for a lead decision on the tool
  (for example a monotonic settle measurement) before a third run.
- The lead fixed the settle guard on main (`f1dc1032`); merge `5d47ea8b`
  (runtime identical to `11c9fc08`, only tools changed) was screened as job
  `20261017T222657Z-t056-m1s1-screen-fixed`, output
  `target/layout-attempts/t056-m1s1-attempt1-fixed`. It exited 1 after
  2,639 s: natural placement complete (86 cases), then the first variant-1
  case `broad/plain_function_call` raised `load monitor timestamps out of
  range/order` in `pair_disturbance.heavy_samples`. The job crossed UTC
  midnight; the monitor log holds `23:46:38`..`23:55:31` rows and then
  `00:21:32`.. rows, and a case anchored after midnight places the
  pre-midnight rows on its own day (only rows more than 12 h before the anchor
  are moved), so they appear out of order. A second tooling defect, again no
  verdict; stopped as instructed, samples not read. The same log shows
  `mediaanalysisd` at about 100% CPU from 00:21, although it is recorded as
  disabled on this host. (Job
  `20261017T181321Z` was cancelled before running: it had been submitted from a
  working tree with an uncommitted documentation change.)
- The lead fixed the midnight decoding on main (`f6fdd25b`). Merge
  `a8c533ef` (runtime again identical to `11c9fc08`) was screened once as job
  `20261018T003151Z-t056-m1s1-screen-final`, output
  `target/layout-attempts/t056-m1s1-attempt1-final`, exit 3 after 12,087 s,
  complete, no DISTURBED report. `verdict.json` SHA-256
  `ef41a810c3374cd7e7068a0069bfa26bea59ee2088031dc2705f2af6375f20f1`
  (copy in workspace `outputs/t056-m1s1-screen/`).

### Verdict: `reject` (policy v3, stage cap 1.025)

All 86 cases pass their case rule after the K=6 panel (wall band 1.025 =
the stage cap; 19 natural events, all resolved). The only failures are
lane gates: `layout 0 development` and `layout 2 development: wall lane
ceiling exceeded`.

| Layout | broad | sentinel | external | development | portfolio |
| --- | ---: | ---: | ---: | ---: | ---: |
| ceiling | 1.01295 | 1.01569 | 1.00738 | 1.00963 | 1.00725 |
| 0 (natural) | 0.99089 | 0.99784 | 0.99601 | **1.01276** | 0.99658 |
| 1 | 1.00954 | 1.00832 | 1.00112 | 1.00311 | 1.00429 |
| 2 | 0.99000 | 0.99737 | 1.00088 | **1.01285** | 0.99884 |
| 3 | 0.99191 | 1.00687 | 1.00013 | 1.00403 | 0.99865 |
| 4 | 0.99263 | 0.99394 | 1.00040 | 1.00041 | 0.99768 |
| 5 | 0.99286 | 1.01294 | 1.00408 | 1.00050 | 1.00100 |

Targets (natural wall median / instruction median, candidate over base):
moment/dates 0.9993 / 1.0088, diff/compare 1.0326 / 1.0051, meriyah/parse
1.0282 / 1.0029. Instruction counts rise slightly, as expected for a stage
that adds linking without yet removing name comparisons.

### Governing verdict: v4 replay `pass` (lead decision, 2026-10-18)

Main `290e3f51` publishes qualified policy v4 and binds it, through
`tools/benchmark/layout_policy.py::HELD_OUT_SUCCESSOR`, as the successor of
this plan's frozen v3 policy (`f655f25a`): v4's held-out bands/rule and its
lane-event rule now govern v2/v3 plans, whose selection bands stay unchanged.
`layout_iteration` applies the successor on every new screen of this plan, so
the same packets judged today are judged under v4. The packets were complete
(six layouts x 86 cases, 2,580 pairs, 0 invalid, 0 heavy samples), so after
merging main (`a8c533ef` is the screened head; the merge commit adds main's
tooling) they were re-judged without new timing by
`evaluate_attempt` on `complete.json`
(SHA-256 `16735eee8c268524bf51ca2a118e886c488930a4394d83973e46a03520e79e96`,
every case-file digest re-verified). This is the plan's single attempt, not a
rerun.

| Rule | Label | Failures | Event resolution |
| --- | --- | --- | --- |
| v3 raw (frozen policy alone) | `reject` | development lane, layouts 0 and 2 | none |
| v4 successor (governing) | `pass` | none | `nonpersistent` |

The v3 replay reproduces the iMac `verdict.json` byte-for-byte in content.
Under v4 the natural-layout development excess is a `development lane` lane
event: the six-layout development geomean is 1.00560 against the 1.00963
ceiling and only two of six layouts exceed it, so it does not persist. The
three development wall events (meriyah, diff, js-yaml) also resolve within the
stage cap. Replay output: workspace `outputs/t056-m1s1-screen/replay.json`
(SHA-256 `23dcc2def9294987068afc154b7721885dc26ef499c465906ade05b70f3aae14`),
script `tmp/t056_m1s1_replay.py`.

Stage 1 is complete at its screen gate. The formal three-engine run and
`decide --mode stage` that the migration workflow names for an intermediate
stage have not been run; `migration.current_stage` is unchanged (1).
