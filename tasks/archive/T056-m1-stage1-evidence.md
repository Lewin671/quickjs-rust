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
  `target/layout-attempts/t056-m1s1-attempt1-replacement`: pending. (Job
  `20261017T181321Z` was cancelled before running: it had been submitted from a
  working tree with an uncommitted documentation change.)
