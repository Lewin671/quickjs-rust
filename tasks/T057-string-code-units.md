# T057: String code-unit migration

- Status: active; stage 1 evidence and plan preparation.
- Implementation: none; target architecture and stage boundaries documented.
- Verified at: base `351004f386f4c2c1d870467ae6e1e481952a330c`,
  2026-10-17; source/tooling inspection; registration checks pending.
- Evidence: design below and the evidence/coordination record in this file.
- Unresolved: development-target layout-gate support; queue/profile evidence,
  frozen plan, implementation and performance/correctness decisions.
- Next action: validate baseline reuse, collect development evidence and exact
  profiles through the shared iMac queue, then freeze the stage-1 plan.

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
  `20261017T061057Z-t056-unit0-baseline30` therefore offers an identical-runtime
  source; reuse still requires current queue validation and exact-image
  binding. Its candidate is `b481894d351d933e94717a9c295c15fe4a466804`,
  binary SHA-256 `c7f0f66a519181b214bdc5ca6404cf428f0d18cbd1ab1d0e26386097619b949e`.
- Initial tooling audit: development ids are accepted/ranked by queue/plan
  loaders, but `layout_rule.evaluate` requires targets in the adopted policy's
  original 76-case inventory and raises `unknown layout target` otherwise.
  The collector/formal bundle has no development lane. Lead clarification
  requested before selecting a formal target or altering a qualified policy.
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
