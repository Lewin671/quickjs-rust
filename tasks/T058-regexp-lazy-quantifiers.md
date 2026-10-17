# T058: RegExp lazy quantifiers and pattern-template removal

- Status: active; screen attempt 1 of 2 labelled `reject` (broad wall lane);
  lead decision pending.
- Implementation: present (`0dc5d6a1`, `1b8c2159`, `cec613d2`; screened at
  merge `392ca717`, runtime-identical to them).
- Verified at: `cec613d2`, 2026-10-17, local M1 Pro: full `check.sh` ok
  (2737 Rust tests, Test262 subset 5170), compare-qjs 250/250 including the
  two new fixtures, exact zero-gap slices `test/built-ins/RegExp` (1573 both
  pass), `test/language/literals/regexp` (155), String `replace` (54),
  `match` (48), `split` (120), Date `toString`/`toDateString`/`toUTCString`
  (8/7/9); `staging/sm/String/replace-math.js` passes directly.
- Evidence: this file; screen job `20261017T222653Z-t058-screen1`; raw
  samples and verdict at workspace `outputs/t058-screen1/`.
- Unresolved: whether the broad failure is a host timing shift (flat
  instructions, bimodal pairs) or a layout effect; RGI_Emoji template.
- Next action: lead decides between attempt 2 (complete K=6) and treating
  attempt 1 as an event; formal/promotion only under policy v4 (T059).

## Goal

Make first-match RegExp traversal of lazy quantifiers do work proportional
to the match it returns, and remove matcher/replace code that admits a
program by comparing its literal pattern source. Admission depends only on
the parsed atom and quantifier structure, never on source text. ECMAScript
backtracking order, captures, lookaround, backreferences and the `u`, `v`,
`y`, `g` flags stay unchanged.

## Scope

- Allowed paths: `crates/qjs-runtime/src/regexp/matcher/**`,
  `crates/qjs-runtime/src/regexp/matcher.rs`,
  `crates/qjs-runtime/src/regexp/symbol_replace.rs`, their focused tests,
  `tests/fixtures/compare-qjs/` RegExp fixtures, this file, its index rows
  and `tasks/performance-units/regexp-lazy-quantifier-steps.json`.
- Forbidden paths: interpreter core, property and value code, String
  search code (owned by `agent/t057-string-units/codex`), `symbol_match.rs`
  subject sharing (also T057), dependencies, `third_party/`.
- Owner boundary: `agent/t058-regexp-lazy/claude`; commit only on this
  branch; no push or merge.
- Base sha, branch and worktree: `a0b6bb23`,
  `agent/t058-regexp-lazy/claude`, `worktrees/quickjs-rust/t058-regexp-lazy`.

## Root causes (confirmed 2026-10-17, local M1 Pro, release build of `a0b6bb23`)

Probe: each case at n=4000 and 4n; ratio near 4 is linear, near 16 is
quadratic. Script kept at workspace `tmp/t058-probes/scaling.js`.

| Case | ours n / 4n ms | ratio | NG ratio |
| --- | --- | --- | --- |
| `"([^]*?)"` replace `g` | 135 / 1623 | 12.0 | 3.5 |
| `"([^]*?)"` replace `gu` | 214 / 3301 | 15.4 | 4.0 |
| `x[\s\S]*?y` match `g` | 507 / 8115 | 16.0 | 4.0 |
| `a.*?,` match `g` | 67 / 1062 | 15.9 | 3.9 |
| `([^x]*?);` exec `y` loop | 176 / 2782 | 15.8 | 4.0 |
| `a.{2,}?;` match `g` | 109 / 1669 | 15.3 | 4.0 |
| greedy `"([^"]*)"` replace `g` | 2 / 8 | 4.0 | 4.0 |

1. `FirstMatcher::match_atom_and_continuation` (`matcher/first.rs`) answers a
   quantified single-code-point atom by scanning its longest run first
   (`simple_atom_max_count`, or `simple_atom_boundaries_into` when steps can
   be two units wide) and only then trying counts from the minimum for a lazy
   quantifier. Every lazy match therefore scans to the end of the atom's run,
   which for `[^]`, `[\s\S]` or a negated class is the rest of the subject.
2. `dot_plus_global_fast_path` (`symbol_replace.rs`) compares the RegExp
   source with the literal `"(.+)"`. It was added for the
   `staging/sm/String/replace-math.js` stress case before T025's
   allocation-free matcher.
3. The audit found two more source-text recognitions:
   `matcher/normalization.rs` rewrites one literal Date-format pattern into a
   different pattern before compilation (observable: the compact pattern
   then requires spaces), and `matches_rgi_emoji_repetition` in `matcher.rs`
   compares the whole pattern with `^\p{RGI_Emoji}+$` and answers it with an
   approximate code-point range test.

`String.prototype.match` with `g` is also quadratic for every pattern
because `symbol_match.rs` passes a fresh subject copy to each `exec`; T057
stage 1 owns that fix and it is not part of this unit.

## Acceptance

- [x] Frozen plan validated and committed before runtime edits.
- [x] Lazy simple-atom steps extend one repetition at a time.
- [x] Literal-source recognizers removed (`(.+)`, Date rewrite); RGI_Emoji template open.
- [x] Focused tests, NG fixtures, RegExp Test262 slices, fast and full gates.
- [x] Qualified screen recorded; formal decision left to policy v4.

## Verification

```sh
./scripts/check-touched.sh --staged --explain
./scripts/find-qjsng-gaps.sh --filter test/built-ins/RegExp --all
./scripts/find-qjsng-gaps.sh --filter test/language/literals/regexp --all
./scripts/check.sh
```

## Plan

Frozen leaf plan
[regexp-lazy-quantifier-steps](performance-units/regexp-lazy-quantifier-steps.json),
validated (`validate-unit` status `valid`, plan SHA-256
`3fa32ec4280395a6e6b393fc0e928e0e8e876424e1abc2f43c992ad4e5683622`) against
the v3 queue `20261017T095818Z-t057-v3-qualify` (SHA-256 `424a1e79...ea56`,
candidate `351004f3`) with the papaparse receipt
`bd02ec4a...e672aec` from profile root
`outputs/imac-bench/20261017T135712Z-t057-stage1-v3-profiles/out/profiles`.
Development target `development/papaparse/csv` (queue rank 1, 34.06x NG);
policy v3 controls; two attempts.

## Implementation (`0dc5d6a1`, `1b8c2159`, `cec613d2`)

- `0dc5d6a1`: `FirstMatcher::match_lazy_simple_atom` steps the minimum,
  tries the continuation, then extends one repetition per failure, bounded by
  `max` and the first failed atom step. Greedy keeps the longest-run scans.
- `1b8c2159`: `dot_plus_global_fast_path` removed; the prepared global loop
  answers `(.+)` linearly (local M1 Pro, 1M-char replace: 4.8 ms vs NG 5.05).
- `cec613d2`: `matcher/normalization.rs` removed; sources compile as written.
- Not changed: `matches_rgi_emoji_repetition`. The general matcher has no
  property-of-strings support (`/^(?:\p{RGI_Emoji})+$/v` fails), so
  removing the template needs real RGI_Emoji sequence data; separate unit.

After `0dc5d6a1`+`1b8c2159` (local, `tmp/t058-probes/scaling-replace.js`,
n=16000 / 4n, ms): `"([^]*?)"` g 9/30 (NG 15/57, base 1816/29135),
`x[\s\S]*?y` 4/16 (NG 10/38, base 8755/139111), `a.*?,` 3/14,
`([^x]*?);` y 15/54 (NG 7/29), `(\w+?)=\1;` 8/32: all linear. Still
quadratic and outside this unit: `String.prototype.match` `g` (subject copy
per exec) and empty-match advancement (both changed on the T057 branch), and
`u`-mode replace (O(n) code-unit/scalar index conversion in `regexp.rs` per
match, T057 code-unit design).

## Screen attempt 1

iMac job `20261017T222653Z-t058-screen1` (`perf-loop.sh`, policy v3, natural
layout only, 2490 s, exit 3; light background activity in 6 samples, below
the disturbance threshold). Verdict SHA-256 `9489c3e9...c9c807`: label
`reject`, failure `layout 0 broad: wall lane ceiling exceeded`, K=6 not
run because the natural layout already failed and the gate is not
mandatory-panel.

| Lane | ratio | ceiling |
| --- | --- | --- |
| broad | 1.0172 | 1.0129 |
| sentinel | 0.9693 | 1.0157 |
| external | 0.9974 | 1.0074 |
| development | 0.8541 | 1.0096 |
| portfolio | 0.9832 | 1.0072 |

Target `development/papaparse/csv`: wall 0.1862, instructions 0.1146
(frozen threshold 0.70). Wall events: `broad/top_level_function_call` 1.4316
(band 1.0250, instructions 1.0002), `broad/string_slice` 1.0407 (band
1.0319, instructions 1.0000), `external/jetstream3-js-subset/cdjs` 1.0336
(band 1.0250, instructions 0.9885). The broad lane failure comes from
`top_level_function_call`, whose five pairs are 1.046, 1.018, 1.491, 1.526,
1.432 with instructions 1.0001-1.0002 throughout: a step change between
pairs 2 and 3 on a 9 ms workload, which code that only changed the RegExp
matcher cannot explain by instructions. No other case regressed in
instructions beyond 1.0002. The attempt counts against the budget under the
written rule unless the lead classifies it otherwise.
