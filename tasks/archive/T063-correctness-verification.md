# T063: Initial canonical string correctness verification

- Status: historical-campaign; initial correctness checks complete.
- Implementation: present at `8e5a2c3f`; later instruction repairs are owned by
  [active T063](../T063-string-canonical.md).
- Verified at: `8e5a2c3f`, 2026-10-20; the exact commands and results below.
- Evidence: committed fixtures/tests and durable counts below; raw iMac logs
  are local workspace artifacts, not retained in this repository.
- Unresolved: pinned NG fails five String cases; no Rust failure in these checks.
- Next action: follow active T063 for the lead rule and instruction follow-up.

This records the original correctness handoff. Its closing performance-screen
proposal preceded the lead's fixed rule; only the lead rule in active T063
controls acceptance. No timing result is established here.

## Verification and merge screening

First iMac no-timing job `20261020T081603Z-string-canonical-gates` at
`3504e994`: full gate exit 1, `String/prototype/padEnd/normal-operation.js`
failed with `Expected SameValue to be true`. All preceding stages passed.
Canonical literals made the old scalar-based filler truncation observable;
padding now streams code units, and repeat's length limit also counts units.
The dependent compare/slice commands did not run after that failure.

Final code revision `8e5a2c3f`, no-timing job
`20261020T082922Z-string-canonical-gates-r2` (461 s). All commands ran even
when one failed, with independent exit statuses:

| Command | Result |
| --- | --- |
| `check-touched.sh --staged --explain` (local pre-commit) | exit 0 on both code commits; first selected 163 language Test262 cases; correction matched no automatic slice |
| `test262-subset.sh --filter test/built-ins/String` (local) | exit 0, 668/668; supplies the correction's focused slice |
| `check.sh` | exit 0; runtime 2323, lexer 64, parser 274+59; agents/perf-counters checks, 659 tool tests, 35 script tests and all 5170 allowlist cases pass |
| `compare-qjs.sh` | exit 0, 252/252 fixtures match, including the new 28-producer cross-product |
| `test262-subset.sh --filter test/built-ins/String --filter test/built-ins/Object --filter test/built-ins/JSON --filter test/built-ins/Proxy` | exit 0, 671/671 |

All commands above use the `scripts/` prefix; the pre-commit command also
passes on the final documentation-only evidence commit (context check only).
The standalone `cargo test --workspace` development run passed before the
padding correction; the final revision's workspace tests ran in `check.sh`.

Four scoped upstream scans ran with
`QJS_CLI_PROFILE=release ./scripts/test262-baseline.sh --all --filter test/built-ins/<area> --engine both`:

| Area | Rust pass / fail / timeout | NG pass / fail | Exit |
| --- | --- | --- | --- |
| String | 1221 / 0 / 0 | 1216 / 5 | 1 |
| Object | 3411 / 0 / 0 | 3411 / 0 | 0 |
| JSON | 165 / 0 / 0 | 165 / 0 | 0 |
| Proxy | 310 / 0 / 0 | 310 / 0 | 0 |

Total Rust 5107/5107; zero NG-pass/Rust-nonpass cases in every area. This is
scoped coverage, not a complete Test262 burndown scan. Job exit 1 is retained
because the String command checks both engines and NG fails five cases:
`String/prototype/{match,matchAll,replace}/regexp-prototype-<method>-v-u-flag.js`
and `String/prototype/search/regexp-prototype-search-v[-u]-flag.js`.
Each was also reproduced locally with the unchanged pinned NG `qjs --script`,
`-I` for `sta.js`, `assert.js`, and (where needed) `compareArray.js`.
All fail at Unicode property matching with `v` (Script=Han on U+20BB7):
null/empty matches, unchanged replacement or search result -1 instead of 0.
The reference source/configuration was not changed to hide these failures.

The integration lead should screen the exact candidate against `cb8ec3ad`
using the qualified v6 workers-4 policy: all 86 selection cases and lane
ceilings, required K=6 layout persistence, the 30-block formal/precision rules,
and all nine held-out programs. This is a regression screen for a correctness
fix, with no claimed performance win. Watch
string-append/concat, join/repeat/padding, string-validate-input, papaparse,
regexp-dna and js-yaml as diagnostics, without selecting programs to shape a
correctness special case. Keep instruction counts alongside wall time to
separate helper work from binary placement. No timing jobs are submitted by
this owner. This change adds constant seam work on appends and code-unit
emission; repeat retains Rust's bulk path when its seam cannot merge. Debug
construction alone adds a full validation scan. Existing non-ASCII search
coordinate and half-surrogate search differences remain later T057 work.
