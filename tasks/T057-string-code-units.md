# T057: String code-unit migration

- Status: active; development policy v3 qualified, stage-1 plan next.
- Implementation: none; target architecture and stage boundaries documented.
- Verified at: registration `25158c8b98555efcbfccee1a70df9c5e7817a719`,
  2026-10-17; staged documentation fast gate, context and whitespace checks pass.
- Evidence: design below and the evidence/coordination record in this file.
- Unresolved: stage-1 profile evidence on the exact queue binary, frozen plan,
  implementation and performance/correctness decisions.
- Next action: development targets are now eligible under qualified
  [policy v3](archive/T057-development-policy.md); validate and commit the stage-1
  plan from the v3 queue and exact-binary profiles.

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
