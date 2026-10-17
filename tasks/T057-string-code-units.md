# T057: String code-unit migration

- Status: active; stage-1 screen passed, formal decision next.
- Implementation: present (stage 1 on this branch, see Stage-1 implementation).
- Verified at: plan commit `47a8042e` (validate-unit valid against v3 queue
  `424a1e79`); implementation fast gate, focused tests and String/RegExp
  Test262 zero-gap slices at the implementation commit (numbers below).
- Evidence: [stage-1 plan](performance-units/string-handles-ascii-search-boundaries.json)
  and the stage-1 records in this file.
- Unresolved: screen attempt 1 passed (K=6 complete); formal 30-block run,
  held-out panel, exact-candidate zero-gap burndown and promotion decision.
- Next action: formal `026476d5` vs `351004f3`, then held-out and `decide --mode promotion`.

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

## Evidence and coordination

- The lead-reviewed investigation inspected runtime `b481894d`. Diff to
  this branch's base changes development-lane/tooling/docs only, with no
  runtime or build-recipe change. T056 baseline job
  `20261017T061057Z-t056-unit0-baseline30` offers an identical-runtime
  source but **current tooling rejected reuse**, exit 2:
  `error: external report differs from raw replay`. Its candidate is
  `b481894d351d933e94717a9c295c15fe4a466804`,
  binary SHA-256 `c7f0f66a519181b214bdc5ca6404cf428f0d18cbd1ab1d0e26386097619b949e`.
- Initial tooling audit: development ids are accepted/ranked by queue/plan
  loaders, but `layout_rule.evaluate` requires targets in the adopted policy's
  original 76-case inventory and raises `unknown layout target` otherwise.
  The collector/formal bundle has no development lane. Lead clarification
  requested before selecting a formal target or altering a qualified policy.
  Reproduction uses a synthetic, explicitly non-evidence packet of five unit
  ratios for each policy case and `development/papaparse/csv` as target;
  `evaluate` raises `ValueError: unknown layout target` before evaluating gates.
  Qualified v2 SHA-256:
  `22a1907dae06eb963dcb25592a05973e3bd69153e3c23a70ddec08ef64f07ca6`.
- Lead feedback `tmp/string-index-cost/feedback-t057-1.md` selects extension
  and iMac qualification of policy v3 before runtime work. The separate
  linked task owns the frozen calibration; no runtime plan or attempt spent.
- Queued fresh current-main 30-block comparison:
  `20261017T074644Z-t057-main-baseline30`, candidate `351004f3`, base `b481894d`.
  Follow-up `20261017T074651Z-t057-stage1-evidence-fresh` validates the queue,
  records service state, samples the exact image on papaparse first, diff,
  regexp-dna and validate-input, runs all development cases for 30 blocks,
  then builds the development-bearing queue. These are pending jobs, not
  results. Prior reuse job `20261017T074527Z-t057-stage1-evidence` was cancelled
  before it ran because its queue validation would fail identically.
- iMac timing is serialized through the workspace queue. T056 has active
  collection and queued jobs; do not bypass that queue or profile held-out.

## Acceptance

- [x] Register staged design, invariants, memory policy and five base differences.
- [ ] Fresh accepted queue plus development lane and exact-binary profiles.
- [ ] Stage-1 plan validated and committed before runtime edits.
- [ ] Focused semantic/work-count tests, String/RegExp Test262 and NG fixtures.
- [ ] Fast/full gates and full configured promotion conformance record.
- [ ] Budgeted qualified screen, required K=6, formal and held-out decision.
- [ ] Durable attempt results and requested external report completed.

## Handoff

No runtime, benchmark policy, dependency, third_party, interpreter-core,
property or value-word code was changed. No plan commit, screen, K=6 panel,
formal candidate decision or held-out promotion result exists yet. Runtime
tests, Test262, NG fixtures and full `check.sh` have not run for this
documentation-only registration. Workspace-local report:
`tmp/string-index-cost/REPORT-t057-stage1.md`; queued artifacts are pending,
and are not durable evidence until their conclusions are recorded here.

## Current-main baseline checkpoint

`20261017T074644Z-t057-main-baseline30` completed exit 0 / 1424 s:
30 blocks, workers-4, all 76 cases, no measurement-health issue or DISTURBED
warning. Candidate 351004f3 and base b481894d share exact image
c7f0f66a (full hash above); NG is 919f7235 / f7830186. Readiness remains
inconclusive solely on regexp-dna interval precision; the raw evidence and
interval are retained. Current queue replay on the iMac accepted this bundle.
This is baseline evidence, not a candidate promotion. The later v3 canonical
A/A uses identical source revisions and its automatic precision continuation.
The policy task owns replacement profile/reference job ids and their
pre-calibration driver amendment; no runtime plan or attempt is spent.

Provisional exact-image profile in job
`20261017T082306Z-t057-stage1-profiles-and-development30`: papaparse was sampled
first for 10 seconds on c7f0f66a, source 351004f3, two complete unmodified
bundle passes. Of 8359 main-thread samples, the collapsed stack-top table
attributes 1058 to `byte_offset_for_char_index` (12.7%), 3912 to
`SimpleAtom::step` and 1668 to `class_match_positive`. These are disjoint
stack-top counts, not additive inclusive caller costs or a speedup forecast.
The ASCII driver supports cached boundary classification as a general target;
real matching work remains outside stage 1. Receipt SHA-256:
`8c191b466278e0315410e92bdf0650246ec13b943e69ad5727acff0b5bd791b5`.
Selection still awaits the complete clean development job, v3 qualification
and the validated frozen runtime plan.

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
