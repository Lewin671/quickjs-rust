# T058: RegExp lazy quantifiers and pattern-template removal

- Status: active; root causes confirmed, leaf plan frozen, no runtime change yet.
- Implementation: none.
- Verified at: registration on `a0b6bb23` (runtime-identical to queue
  candidate `351004f3`), 2026-10-17; scaling probes below.
- Evidence: this file; exact-image profile receipt from iMac job
  `20261017T135712Z-t057-stage1-v3-profiles`.
- Unresolved: implementation, gates and screen.
- Next action: implement lazy simple-atom steps and remove the literal-source recognizers.

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
- [ ] Lazy simple-atom steps extend one repetition at a time.
- [ ] Literal-source recognizers removed or replaced structurally.
- [ ] Focused tests, NG fixtures, RegExp Test262 slices, fast and full gates.
- [ ] Qualified screen recorded; formal decision left to policy v4.

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
