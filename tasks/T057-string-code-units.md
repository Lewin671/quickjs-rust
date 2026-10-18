# T057: String code-unit migration

- Status: active; stage-1 promotion re-decision under policy v4 in progress.
- Implementation: present on this branch (`026476d5`), not promoted.
- Verified at: `026476d5` gates as below; v4 fresh held-out natural screen and
  candidate formal continuation complete (2026-10-18); merged main `9b1da2e9`.
- Evidence: stage-1 records below; workspace `outputs/t057-stage1/`.
- Unresolved: held-out lane event needs the K=6 held-out panel
  (job `20261018T053942Z-t057-stage1-v4-heldout-complete`, queued).
- Next action: after the panel, `decide --mode promotion` with the formal
  continuation; record the decision.

## Goal and scope

Remove repeated whole-subject work through the staged
[code-unit authority design](../docs/design/string-code-units.md), starting
with handle preservation and cached ASCII search boundaries. General
operations/types/data flow determine admission; no program recognition.
Stage 5 coordinates primary storage with T056 M1 code-unit atom keys;
stages 1–4 remain String/RegExp-local.

Owner: `agent/t057-string-units/codex`; worktree
`worktrees/quickjs-rust/t057-string-units`, base above. Allowed: String/RegExp
modules, narrowly required coercion helpers, focused tests/fixtures, this
task/design/index rows and the stage-1 performance plan. Forbidden:
interpreter core, property storage, value-word code, dependencies and
third_party edits. No project push or merge; queue transport refs only.

Pre-stage-1 coordination, the registration handoff and the current-main
baseline checkpoint are in [the T057 log](archive/T057-log.md).

## Acceptance

- [x] Register staged design, invariants, memory policy and five base differences.
- [ ] Fresh accepted queue plus development lane and exact-binary profiles.
- [ ] Stage-1 plan validated and committed before runtime edits.
- [ ] Focused semantic/work-count tests, String/RegExp Test262 and NG fixtures.
- [ ] Fast/full gates and full configured promotion conformance record.
- [ ] Budgeted qualified screen, required K=6, formal and held-out decision.
- [ ] Durable attempt results and requested external report completed.

## Stage-1 plan (frozen before runtime edits)

Plan [`string-handles-ascii-search-boundaries`](performance-units/string-handles-ascii-search-boundaries.json):
leaf, schema 3, base/queue candidate 351004f3 (image c7f0f66a), queue SHA-256
`424a1e79...`, policy v3 `f655f25a...`, `cpu-work`, natural screen
(no imprecise v3 case forces K=6), 2 attempts, both targets at most 0.95 and
beyond their reciprocal bands (papaparse 1.0250, regexp-dna 1.0335); all
other 84 policy cases are controls. Profile root (workspace-local):
`outputs/t057-stage1/profiles/`.

| Receipt | Exact-image profile | Stage-1 cost (inclusive) |
| --- | --- | --- |
| papaparse `bd02ec4a` | job `20261017T135712Z-t057-stage1-v3-profiles`, 60 s over two passes, 50158 samples | indexOf 4746 (9.46%), of which uncached ASCII scan in `byte_offset_for_char_index` 4556 |
| regexp-dna `5fe9402e` | job `20261017T082306Z-...-development30`, 10 s of 100 passes, 8331 samples | per-exec subject copy rebuilds matcher view: `prepare_input` 1582 (19%) |

js-yaml was also profiled (receipt `51f88495`, 16646 samples): String/RegExp
handle or boundary costs are below 1%; it stays a control. The papaparse
profile's dominant cost is outside stage 1: the lazy `[^]*?` RegExp replace is
42710 inclusive (85%), 38165 in `SimpleAtom::step`, because
`match_atom_and_continuation` measures the maximal fixed-width run with
`simple_atom_max_count` before trying the lazy minimum, so each match scans
to the end of the subject. That is a separate general matcher unit.

## Stage-1 implementation

Typed ToString `to_js_string_value_with_env` returns the shared `JsString`
(string arguments and strings an object's ToPrimitive returns), with the
same hook order. `this_js_string_value` shares primitive and wrapper data;
`String.prototype.toString/valueOf` return the wrapper's handle and
`is_string_object` no longer copies. Search helpers take the receiver
handle: `byte_offset_for_char_index` answers ASCII from the memoized flag,
`indexOf` returns the ASCII offset directly, needles are not copied.
RegExp `exec`/`test`/`[Symbol.match|matchAll|search|replace|split]`,
the matchAll iterator, the legacy global match and string-needle replace
pass one subject handle to every exec and callback. Generic custom-exec
replace clamps with the memoized UTF-16 length; empty-match advance is
`index + 1` without a subject view in non-Unicode or ASCII text. String
replacement counts the subject once (`substitution_length` takes its
length). Non-ASCII search keeps legacy scalar coordinates; none of the five
base differences is on a changed path, so all remain stage 2/3 work.

Tests: `tests/strings/subject_sharing.rs` (7: hook order, boxed sharing,
matchAll subject across next, one subject view per protocol, every
exec/lastIndex lookup, callback input sharing/substitutions, one ASCII
classification across a cursor search loop); four sharing tests fail on the
base and pass here, the others pin unchanged semantics. The ASCII count test
pins classify-once, not the absence of raw `str` scans. NG fixture
`string-regexp-subject-protocols.js` matches NG. Exhaustive
`find-qjsng-gaps --filter ... --all` (release CLI): String 1221,
RegExp/prototype 485, StringIteratorPrototype 7,
RegExpStringIteratorPrototype 17 cases, zero gaps. Runtime lib: 2278 pass.
Implementation commit `026476d5`: full `check.sh` on the iMac (job
`20261017T141818Z-t057-stage1-full-check`, no-timing) exit 0 / 258 s,
runtime lib 2278 tests, 5170 Test262 subset cases; local `compare-qjs.sh`
249/249 fixtures match NG.

## Stage-1 attempts

| Attempt | Job | Candidate | Result |
| --- | --- | --- | --- |
| 1 | `20261017T142313Z-t057-stage1-screen1` | `026476d5` | **pass**, complete K=6 (exit 0 / 11977 s, no DISTURBED) |

Attempt 1 natural screen: papaparse wall 0.8801 (instructions 0.9298),
regexp-dna 0.7889 (0.8735); every target pair improved, both below 0.95 and
their reciprocal bands. Events opened the full panel: wall events
broad/method_call 1.0558 and broad/property_dynamic_read 1.0760 (instructions
1.0000), and a control instruction event on string-validate-input (every
pair above 1.01, median 1.0118 below the 1.03 hard cap; wall 1.0137 inside
B 1.0300). The six-layout completion resolved all three as nonpersistent;
every lane and the portfolio fit their ceilings on all six layouts (natural:
broad 1.0086, sentinel 1.0091, external 0.9924, development 0.9851,
portfolio 0.9974). Verdict copy: workspace `outputs/t057-stage1/screen1-verdict.json`;
raw panel stays in the iMac runner's `target/layout-attempts/t057-stage1-attempt1/`.

## Stage-1 decision (attempt 1)

Formal job `20261017T174818Z-t057-stage1-formal30` (workers-4, 30 blocks,
exit 0 / 3465 s, no DISTURBED): candidate image `4dffe000...`, base
c7f0f66a. Targets: papaparse 0.8820 [0.8814, 0.8839], regexp-dna 0.7900
[0.7840, 0.7946] (regexp-dna/NG now 0.993). Lanes: broad 1.0085 [1.0076,
1.0095], sentinel 1.0011, external 0.9961, development 0.9882, portfolio
0.9991, all inside ceilings. string-validate-input 1.0164 [1.0120, 1.0227]
(inside B 1.0300). Control `bitops-3bit-bits-in-byte` 0.9974 [0.9387,
1.0072] is imprecise (half-width 6.2%); candidate formals have no
continuation tool (A/A only), so it stays a missing-evidence reason.

Held-out job `20261017T174847Z-t057-stage1-heldout30` (natural, 30 pairs,
exit 0 / 3207 s), acceptance only and not used to shape work: handlebars
1.0233 (band 1.0066, instructions 1.0000), esprima 1.0087 (1.0030,
instructions 0.9994), ramda 1.0090 (1.0065, 0.9996); lane 1.0050 above its
1.0029 ceiling; the other six within bands. Exact-candidate burndown job
`20261017T174848Z-t057-stage1-test262-exact`: 53572 total, 42672
configured, Rust 42672 pass, zero actionable gap (SHA-256 `638cc0eb...`).

`decide --mode promotion` (decision SHA-256 `45a6ec04...`): **rejected**
on the held-out programs and lane; readiness inconclusive for the
imprecise control. The held-out excesses are wall-only (flat or lower
instructions in code the unit does not touch), consistent with code
placement of a new binary; held-out acceptance is single natural layout,
so it has no six-layout persistence test. Not a favorable-rerun or
held-out-shaped attempt 2; outcome goes to the lead.

## Policy correction and v4 re-decision

The attempt-1 held-out rejection exposed a structural defect: held-out bands
and lane ceiling came only from same-binary A/A and had no layout-persistence
check, so code placement of any new binary could fail it. The lead corrected
this in [policy v4](archive/T059-held-out-layout-policy-v4.md) (main
`290e3f51`, SHA-256 `1eb7de46...`), triggered by this unit. The plan keeps
its v3 selection bands; v4 is its held-out successor. Attempt accounting:
no new implementation attempt; the attempt-1 screen and formal bundle stand
(1 of 2 attempts used). The rejected v3 held-out run is neither reused nor
pooled; held-out was re-measured fresh for the same candidate `026476d5`
(the audited exact-commit matrices of the attempt-1 screen).

Fresh held-out job `20261018T013031Z-t057-stage1-v4-heldout30` (natural,
30 pairs, exit 0 / 3203 s): every program inside its v4 band including the
upper interval (largest handlebars 1.0203 [1.0133, 1.0278], B 1.0357;
others 0.9972-1.0117); no instruction excess (medians 0.9907-1.0000). Lane
1.0077 [1.0058, 1.0090] is above the 1.005018 ceiling: a lane event, so the
K=6 held-out panel was queued (`--held-out --complete`, same output).

Formal continuation job `20261018T013032Z-t057-stage1-formal-continuation`
(candidate/base chain on the sealed attempt-1 bundle, exit 0 / 53 s): the
only imprecise case, bitops-3bit-bits-in-byte, needed the predicted 131 then
the 240 cap; at 240 blocks it is 0.9825 [0.9608, 0.9891], precise
(half-width 2.26%). Lane statistics keep the original 30 joint blocks.
