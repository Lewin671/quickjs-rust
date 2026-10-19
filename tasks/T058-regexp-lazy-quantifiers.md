# T058: RegExp lazy quantifiers and pattern-template removal

- Status: active; INCONCLUSIVE, no PROMOTE/REJECT verdict yet: the frozen
  rule requires follow-up measurements, which were not queued.
- Implementation: present (`0dc5d6a1`, `1b8c2159`, `cec613d2`; measured
  candidate `392ca717`, runtime-identical to them); no runtime edits in this review.
- Verified at: review checkout `5503cfff`; measured base/candidate
  `351004f3`/`392ca717`. All three fetched jobs exit 0, no DISTURBED.
- Evidence: deciding numbers and rule retained here; local artifacts, not
  retained: workspace `outputs/imac-bench/<job>/`, `outputs/t058-screen1/`,
  `outputs/t058-heldout30/`, `outputs/t058-promotion-review/`.
  SHA-256: formal summary `ee2d0c2f...cc8d9f`, held-out `panel.json`
  `d5f5f461...6c0cf02`, screen `complete.json` `fff35cfc...0cdb`,
  Test262 entry `9b24e29a...641f7`. Sealed bundle, both panels' raw packets,
  identities and frozen unit/profile validation checked locally.
- Unresolved: held-out completion and formal precision continuation; RGI_Emoji
  template. Local promotion CLI on Python 3.14.2 and 3.13.4 exits 2:
  `error: report.json: report differs from raw replay`. Broad
  `property_dynamic_read` ratio / half-width differ by about 2.22e-16 from
  the sealed x86 report; no artifacts rewritten and no CLI verdict produced.
  Independent raw/panel rule evaluation finds a held-out wall event, no failures;
  it cannot establish promotion. Replay must succeed before the final decision.
- Next action: after separate scheduling authorization, run these exact commands
  in serialized iMac timing jobs at `5503cfff` (not run or queued here), then
  replay promotion on the measurement host with screen `complete.json`, held-out
  `complete.json`, the formal continuation and the exact Test262 entry.
- Jobs: `20261018T080835Z-t058-formal30` (2759 s, 3 light background samples),
  `20261018T080836Z-t058-heldout30` (3204 s, 6 light samples), and
  `20261018T080837Z-t058-test262-exact` (1688 s, no timing monitor).
- Results: formal papaparse ratio 0.1865569292, 95% CI [0.1854727456, 0.1874598998],
  below frozen 0.70; SunSpider `bitops-3bit-bits-in-byte` ratio 0.9282498417,
  CI [0.9167765124, 0.9863025702], relative half-width 6.253998% > 3%:
  power rule allocates 131 total blocks (101 appended), then 240 if needed.
  Held-out lane 1.0086606337, CI [1.0066319824, 1.0103259991], exceeds
  ceiling 1.0050182542. Esprima upper CI 1.0273564277 > band 1.0250249555
  (median 1.0209777832, instructions median 0.9939391379). Both need K=6.
  Exact Test262: 42672/42672 Rust passes; NG 42602 passes / 70 failures;
  actionable gaps 0, Rust failures/timeouts/not-run 0 (entry dated 2026-10-19).
- Rule: plan requires complete broad/external evidence and zero Test262 gap;
  target <= min(0.70, reciprocal wall band), every screen pair improves.
  T022: "a wall-only program or lane excess needs the K=6 held-out panel and
  rejects only if all six exceed." Formal precision uses
  n=min(240,ceil(30*(h/.03)^2)), retaining the pilot; a candidate still
  imprecise at 240 stays incomplete. No threshold or attempt budget changed.

```sh
python3 -m tools.benchmark.layout_formal_continuation \
  --bundle "$BENCH_QUEUE/20261018T080835Z-t058-formal30/out/bundle" \
  --case external/sunspider-1.0/bitops-3bit-bits-in-byte --output "$BENCH_OUT/bitops-power"
python3 -m tools.benchmark.layout_panel \
  --base-matrix target/layout-attempts/t058-attempt1/base000/identity.json \
  --candidate-matrix target/layout-attempts/t058-attempt1/cand000/identity.json \
  --held-out --complete --pairs 30 --output target/layout/t058-heldout
```

## Goal

Make first-match RegExp traversal of lazy quantifiers do work proportional to the match it returns.
Remove matcher/replace admission by literal pattern source; use parsed atom and quantifier structure.
Preserve backtracking order, captures, lookaround, backreferences and the `u`, `v`, `y`, `g` flags.

## Scope

- Allowed paths: `crates/qjs-runtime/src/regexp/matcher/**`,
  `crates/qjs-runtime/src/regexp/matcher.rs`,
  `crates/qjs-runtime/src/regexp/symbol_replace.rs`, their focused tests,
  `tests/fixtures/compare-qjs/` RegExp fixtures, this file, its index rows
  and `tasks/performance-units/regexp-lazy-quantifier-steps.json`.
- Forbidden paths: interpreter core, property and value code, String
  search code (owned by `agent/t057-string-units/codex`), `symbol_match.rs`
  subject sharing (also T057), dependencies, `third_party/`.
- Owner boundary: `agent/t058-regexp-lazy/claude`; commit only on this branch; no push or merge.
- Base sha, branch and worktree: `a0b6bb23`,
  `agent/t058-regexp-lazy/claude`, `worktrees/quickjs-rust/t058-regexp-lazy`.

## Root causes (confirmed 2026-10-17, local M1 Pro, release build of `a0b6bb23`)

Probe: each case at n=4000 and 4n; ratio near 4 is linear, near 16 is quadratic. Script kept at
workspace `tmp/t058-probes/scaling.js`.

| Case | ours n / 4n ms | ratio | NG ratio |
| --- | --- | --- | --- |
| `"([^]*?)"` replace `g` | 135 / 1623 | 12.0 | 3.5 |
| `"([^]*?)"` replace `gu` | 214 / 3301 | 15.4 | 4.0 |
| `x[\s\S]*?y` match `g` | 507 / 8115 | 16.0 | 4.0 |
| `a.*?,` match `g` | 67 / 1062 | 15.9 | 3.9 |
| `([^x]*?);` exec `y` loop | 176 / 2782 | 15.8 | 4.0 |
| `a.{2,}?;` match `g` | 109 / 1669 | 15.3 | 4.0 |
| greedy `"([^"]*)"` replace `g` | 2 / 8 | 4.0 | 4.0 |

1. `FirstMatcher::match_atom_and_continuation` (`matcher/first.rs`) answers a quantified
   single-code-point atom by scanning its longest run first (`simple_atom_max_count`, or
   `simple_atom_boundaries_into` when steps can be two units wide) and only then trying counts from
   the minimum for a lazy quantifier. Every lazy match therefore scans to the end of the atom's run,
   which for `[^]`, `[\s\S]` or a negated class is the rest of the subject.
2. `dot_plus_global_fast_path` (`symbol_replace.rs`) compares the RegExp source with the literal
   `"(.+)"`. It was added for the `staging/sm/String/replace-math.js` stress case before T025's
   allocation-free matcher.
3. The audit found two more source-text recognitions: `matcher/normalization.rs` rewrites one
   literal Date-format pattern into a different pattern before compilation (observable: the compact
   pattern then requires spaces), and `matches_rgi_emoji_repetition` in `matcher.rs` compares the
   whole pattern with `^\p{RGI_Emoji}+$` and answers it with an approximate code-point range test.

`String.prototype.match` with `g` is also quadratic for every pattern because `symbol_match.rs`
passes a fresh subject copy to each `exec`; T057 stage 1 owns that fix and it is not part of this
unit.

## Acceptance

- [x] Frozen plan validated and committed before runtime edits.
- [x] Lazy simple-atom steps extend one repetition at a time.
- [x] Literal-source recognizers removed (`(.+)`, Date rewrite); RGI_Emoji template open.
- [x] Focused tests, NG fixtures, RegExp Test262 slices, fast and full gates.
- [x] Qualified screen recorded; formal decision left to policy v4.

## Verification

At `cec613d2` (2026-10-17, local M1 Pro): full `check.sh` passed (2737 Rust tests,
5170 Test262 subset), compare-qjs 250/250 including two new fixtures; exact zero-gap slices:
RegExp 1573, regexp literals 155, String replace/match/split 54/48/120,
Date toString/toDateString/toUTCString 8/7/9; staging/sm/String/replace-math.js passed directly.

```sh
./scripts/check-touched.sh --staged --explain
./scripts/find-qjsng-gaps.sh --filter test/built-ins/RegExp --all
./scripts/find-qjsng-gaps.sh --filter test/language/literals/regexp --all
./scripts/check.sh
```

## Plan

Frozen leaf plan
[regexp-lazy-quantifier-steps](performance-units/regexp-lazy-quantifier-steps.json), validated
(`validate-unit` status `valid`, plan SHA-256
`3fa32ec4280395a6e6b393fc0e928e0e8e876424e1abc2f43c992ad4e5683622`) against the v3 queue
`20261017T095818Z-t057-v3-qualify` (SHA-256 `424a1e79...ea56`, candidate `351004f3`) with the
papaparse receipt `bd02ec4a...e672aec` from profile root
`outputs/imac-bench/20261017T135712Z-t057-stage1-v3-profiles/out/profiles`. Development target
`development/papaparse/csv` (queue rank 1, 34.06x NG); policy v3 controls; two attempts.

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

After `0dc5d6a1`+`1b8c2159` (local, `tmp/t058-probes/scaling-replace.js`, n=16000 / 4n, ms):
`"([^]*?)"` g 9/30 (NG 15/57, base 1816/29135), `x[\s\S]*?y` 4/16 (NG 10/38, base 8755/139111),
`a.*?,` 3/14, `([^x]*?);` y 15/54 (NG 7/29), `(\w+?)=\1;` 8/32: all linear. Still quadratic and
outside this unit: `String.prototype.match` `g` (subject copy per exec) and empty-match advancement
(both changed on the T057 branch), and `u`-mode replace (O(n) code-unit/scalar index conversion in
`regexp.rs` per match, T057 code-unit design).

## Screen attempt 1

iMac job `20261017T222653Z-t058-screen1` (`perf-loop.sh`, policy v3, natural layout only, 2490 s,
exit 3; light background activity in 6 samples, below the disturbance threshold). Verdict SHA-256
`9489c3e9...c9c807`: label `reject`, failure `layout 0 broad: wall lane ceiling exceeded`, K=6 not
run because the natural layout already failed and the gate is not mandatory-panel.

Natural lane ratios and ceilings are retained in the complete panel table below.

Target `development/papaparse/csv`: wall 0.1862, instructions 0.1146 (frozen threshold 0.70). Wall
events: `broad/top_level_function_call` 1.4316 (band 1.0250, instructions 1.0002),
`broad/string_slice` 1.0407 (band 1.0319, instructions 1.0000), `external/jetstream3-js-subset/cdjs`
1.0336 (band 1.0250, instructions 0.9885). The broad lane failure comes from
`top_level_function_call`, whose five pairs are 1.046, 1.018, 1.491, 1.526, 1.432 with instructions
1.0001-1.0002 throughout: a step change between pairs 2 and 3 on a 9 ms workload, which code that
only changed the RegExp matcher cannot explain by instructions. No other case regressed in
instructions beyond 1.0002.

### Completion under policy v4 (lead decision 2026-10-17)

Policy v4 (`tasks/archive/T059-held-out-layout-policy-v4.md`) makes a natural lane excess a lane
event that needs the complete K=6 panel. The v3-bound plan is evaluated with v4's successor rule;
re-evaluated, the natural packet is `wall event` with `broad lane: lane event`, no failures. This
completes attempt 1; attempt 2 is unspent. Job `20261018T013229Z-t058-screen1-complete` (exit 0,
8795 s) first re-hashed all twelve audited binaries (base natural `c7f0f66a...`, candidate natural
`d2bc9b8d...`, revisions `351004f3`/`392ca717`), then appended layouts 1-5 to the same `screen/`
directory. Verdict `pass`, complete, event resolution `nonpersistent`. SHA-256: verdict
`ce385522...abd9ae`, `complete.json` `fff35cfc...0cdb`, natural-v4 verdict `83005b47...4455`.

| Lane | layouts 0-5 | geomean | ceiling |
| --- | --- | --- | --- |
| broad | 1.0172 1.0028 1.0021 1.0006 1.0068 0.9973 | 1.0044 | 1.0129 |
| sentinel | 0.9693 1.0123 0.9869 0.9965 0.9929 0.9980 | 0.9926 | 1.0157 |
| external | 0.9974 0.9967 1.0016 0.9978 0.9987 0.9998 | 0.9987 | 1.0074 |
| development | 0.8541 0.8470 0.8570 0.8513 0.8435 0.8455 | 0.8497 | 1.0096 |
| portfolio | 0.9832 0.9808 0.9827 0.9803 0.9812 0.9796 | 0.9813 | 1.0072 |

Wall-event cases, median per layout: `top_level_function_call` 1.4316 1.0354 1.0113 1.0112 1.0413
1.0189; `string_slice` 1.0407 1.0212 0.9950 1.0170 1.0194 1.0390; `cdjs` 1.0336 1.0127 1.0175 0.9985
1.0168 1.0099. Target papaparse 0.1862 0.1857 0.1862 0.1853 0.1858 0.1836.
