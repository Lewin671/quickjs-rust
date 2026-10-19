# T058: RegExp lazy quantifiers and pattern-template removal

- Status: active; attempt 1 closed incomplete, no promotion; one v5 remeasurement frozen prospectively.
- Implementation: present; original measured candidate `392ca717`; rebased branch `0da2f4e1` includes main's T057, so it is not substituted into this experiment.
- Verified at: rebase onto `89a13699` completed at `0da2f4e1` (2026-10-19 UTC); full/fast gate results recorded below before handoff.
- Evidence: [attempt 1 closure](archive/T058-attempt1-incomplete.md); [original plan](performance-units/regexp-lazy-quantifier-steps.json); [v5 remeasurement plan](performance-units/regexp-lazy-quantifier-steps-v5-remeasure.json).
- Unresolved: fresh v5 acceptance pending; original formal case was imprecise at 240; RGI_Emoji template remains separate work.
- Next action: coordinator reads the queued decision and all prerequisite statuses using the contract below; no automatic favorable retry or promotion of the rebased tip.

## Goal

Make first-match RegExp traversal of lazy quantifiers do work proportional to the match it returns.
Remove matcher/replace admission by literal pattern source; use parsed atom and quantifier structure.
Preserve backtracking order, captures, lookaround, backreferences and the `u`, `v`, `y`, `g` flags.

## Scope

- Allowed paths: `crates/qjs-runtime/src/regexp/matcher/**`,
  `crates/qjs-runtime/src/regexp/matcher.rs`,
  `crates/qjs-runtime/src/regexp/symbol_replace.rs`, their focused tests,
  `tests/fixtures/compare-qjs/` RegExp fixtures, this file, its index rows,
  the original and v5 remeasurement plans, and the attempt-1 archive.
- Forbidden paths: interpreter core, property and value code, String
  search code (owned by `agent/t057-string-units/codex`), `symbol_match.rs`
  subject sharing (also T057), dependencies, `third_party/`.
- Owner boundary: `agent/t058-regexp-lazy/claude`; commit only on this branch; no push or merge.
- Original base/worktree: `a0b6bb23`, `worktrees/quickjs-rust/t058-regexp-lazy`; branch above.

## Root causes (confirmed 2026-10-17, local M1 Pro, release build of `a0b6bb23`)

Probe at n=4000 and 4n: ratio near 4 is linear, near 16 is quadratic; `tmp/t058-probes/scaling.js`.

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

At the original task base, `String.prototype.match` with `g` was also quadratic for every pattern because `symbol_match.rs`
passes a fresh subject copy to each `exec`; T057 stage 1 owns that fix and it is not part of this
unit.

## Acceptance

- [x] Frozen plan validated and committed before runtime edits.
- [x] Lazy simple-atom steps extend one repetition at a time.
- [x] Literal-source recognizers removed (`(.+)`, Date rewrite); RGI_Emoji template open.
- [x] Focused tests, NG fixtures, RegExp Test262 slices, fast and full gates.
- [x] Original qualified screen recorded; attempt 1 closed incomplete.
- [ ] Fresh v5 attempt passes all promotion gates.

## Verification

At `cec613d2` (2026-10-17, local M1 Pro): full `check.sh` passed (2737 Rust tests,
5170 Test262 subset), compare-qjs 250/250 including two new fixtures; exact zero-gap slices:
RegExp 1573, regexp literals 155, String replace/match/split 54/48/120,
Date toString/toDateString/toUTCString 8/7/9; staging/sm/String/replace-math.js passed directly.

## Original plan

Frozen leaf plan [regexp-lazy-quantifier-steps](performance-units/regexp-lazy-quantifier-steps.json), validated
(`validate-unit` status `valid`, plan SHA-256
`3fa32ec4280395a6e6b393fc0e928e0e8e876424e1abc2f43c992ad4e5683622`) against the v3 queue
`20261017T095818Z-t057-v3-qualify` (SHA-256 `424a1e79...ea56`, candidate `351004f3`) with the
papaparse receipt `bd02ec4a...e672aec` from profile root
`outputs/imac-bench/20261017T135712Z-t057-stage1-v3-profiles/out/profiles`. Development target
`development/papaparse/csv` (queue rank 1, 34.06x NG); policy v3 controls; two attempts.

## Original implementation (`0dc5d6a1`, `1b8c2159`, `cec613d2`)

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
`a.*?,` 3/14, `([^x]*?);` y 15/54 (NG 7/29), `(\w+?)=\1;` 8/32: all linear. At that revision, still quadratic and
outside this unit: `String.prototype.match` `g` (subject copy per exec) and empty-match advancement
(both changed on the T057 branch), and `u`-mode replace (O(n) code-unit/scalar index conversion in
`regexp.rs` per match, T057 code-unit design).

## Prospective v5 remeasurement (attempt 2 of 2)

The original attempt is closed as incomplete, not promoted. T022 permits
at most two attempts and the [screen-1 record](archive/T058-attempt1-incomplete.md)
explicitly left attempt 2 unspent. The new plan caps it at **one** remaining
attempt; policy adoption and triage grant no extra attempts. A valid failed
screen closes this leaf's budget. A second cap-imprecise formal result stays
incomplete and stops; do not start another acceptance run to seek precision.
No committed original-plan field or historical sample is changed.

This is a measurement-only successor, not a new optimization selected from
a stale queue. Preserve the original queue/profile, base and unchanged
implementation; validate their hashes again before timing. Freeze candidate
`392ca717f13fd97ca1bc8361fefb2c1900186602`, base
`351004f386f4c2c1d870467ae6e1e481952a330c`. Using the rebased branch tip would
also measure T057, obscuring attribution and invalidating the original image
and correctness bindings. Any later performance claim about a combined main
integration needs its own evidence.

| Frozen choice | Value and reason |
| --- | --- |
| Schema/policy | Schema 3, qualified iMac workers-4 v5; policy SHA-256 `f3e8ef40b8b45601b5b86edaf4b47c2d9d904d0045eb024e837f1c447a52bec4`. T060 qualifies this schedule independently; no hand-tuned bands. |
| Target/payoff | Only `development/papaparse/csv`, rank 1 in the original queue, <=0.70 as originally frozen; also reciprocal B_i and every-pair improvement. No result-based target/threshold selection. |
| Controls/mechanism | All other 85 selection cases, all six sentinels and all lane/portfolio gates; `cpu-work`, empty memory targets, unchanged atom traversal. |
| Layouts | Mandatory K=6, offsets 0/16/64/240/512/1008; five pairs per selection case/layout. Complete natural plus five appended layouts regardless of the natural verdict; no winning placement selection. |
| Held-out | All nine programs, 30 pairs at all six matching layouts, frozen before new results; v5 persistence and precision gates remain binding. |
| Formal | One fresh 30-block, 86-case, three-engine workers-4 bundle after a passing screen. Same original budget, not 60 chosen from the observed variance. |
| Power | Every deficient formal case follows min(240,ceil(30*(h/.03)^2)), predicted stage then cap; retain pilot and original lane cohort. No pooling with attempt 1, no widened 3% precision limit. |
| Stop/recovery | One remaining attempt. Pair recovery only under the existing 20%/60 s load caps; cap-invalid panels or DISTURBED formal jobs replace whole, at most three load-only replacements. Non-load errors stop; no rerun after a valid failure or cap-imprecision. |
| Correctness | Reuse exact-candidate zero-gap entry from `20261018T080837Z-t058-test262-exact`, SHA-256 below; all promotion completeness gates stay enabled. Local full gate checks the rebased branch separately. |

Reuse only the original immutable audited **executables**, never their timing.
Matrix paths are `target/layout-attempts/t058-attempt1/{base000,cand000}/identity.json`.
SHA-256: base `a323746e4ad6e41d8e6a9cf7141c1eac1bf82c0546ab45826bd644b277b9d8e7`,
candidate `76427b2286c72c9289656636540da6bfe646ab0c870d3601d4c3a27037c48521`.
Natural images must reproduce `c7f0f66a519181b214bdc5ca6404cf428f0d18cbd1ab1d0e26386097619b949e`
and `d2bc9b8d755bcb70ebb02d8a3e0bde21d30d89add36dd66cbea64cebae72b7c8`.
Preflight re-hashes all twelve binaries and checks static audits, identities,
host, equal-length paths and plan/queue/profile bindings; missing/drifted inputs
stop rather than silently rebuilding another image. New output directories
are exclusive. Plan is committed before dispatch; runtime is unchanged.

The reverted queue wrapper has no STOP/CONT hook. At 13:45 UTC on 2026-10-19,
iMac Spotlight was disabled, photo/media launchctl disable flags were true,
and listed daemons were in S/Ss state (none T/stopped). Each timed job records
quieting again without altering services. Local free disk 88 GiB, iMac 101 GiB.

New plan `validate-unit`: status `valid`, SHA-256
`957f29f056bb65ea3b556e177b8117e0d303ea95643bed5ca3f070f1376c7b94`.
Exact-candidate Test262 entry SHA-256
`9b24e29adcef679290473002a6fa3bf16140434a9569cdd4111524afe67641f7`.

## Dispatch and coordinator contract

Pending dispatch after plan validation/commit. All timed jobs use workspace
`scripts/imac-bench.sh run --timing`; decision is chained after measurement.
Only runner `refs/bench/` transport is authorized; no origin push or merge.

Estimate: selection K=6 ~74 min (T060 measured 4,457 s); held-out K=6
~125 min (T061 extrapolates the measured 2.55x speedup, not yet a direct
held-out measurement); formal ~46-58 min; power/replay a few minutes.
Total ~4.1-4.3 h plus validation, quieting, queue delay, precision work and
any permitted load-only replacement. Reusing audited matrices avoids builds.

Fetch every job after completion. A zero collector exit proves collection,
not acceptance. The decision replays exact raw evidence on the original iMac,
including the frozen queue/profile, new complete selection and held-out panels,
new sealed formal bundle and latest power chains, plus exact-candidate Test262.
Read `decision.json` (`decision`, `decision_readiness`, `reasons`) and policy/
unit/evidence hashes. Only `retained` with complete clean evidence can support
promotion of the original candidate. `inconclusive`, a >3% interval at 240,
missing evidence, failed guards or disturbance require an incomplete record;
a valid rejection is retained as negative evidence. Do not pool old timing,
choose between runs, drop cases, apply a schema-1/2 unstable-base waiver, or
report the rebased main+T057 tip as performance-verified by this decision.
