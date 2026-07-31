# Performance-unit plans

Every change that is presented as performance campaign progress gets one JSON
plan in this directory before implementation. A plan is intentionally small:
raw profiles and benchmark JSONL stay in artifacts, while the plan records the
artifact hashes and the decision criteria that must not move after timing.

Generate a current queue from a complete preview bundle:

```sh
./scripts/performance-decision.sh queue \
  --summary /path/to/summary.json \
  --broad-report /path/to/report.json \
  --external-report /path/to/external-report.json \
  --output target/performance-opportunity.json
```

Then create `<unit>.json` with this shape (replace every placeholder before
validation):

```json
{
  "schema_version": 1,
  "artifact_type": "quickjs-performance-unit",
  "unit_id": "shared-call-setup",
  "base_sha": "<queue candidate SHA>",
  "queue": {"candidate_sha": "<queue candidate SHA>", "sha256": "<queue SHA-256>"},
  "priority": {
    "mode": "queue",
    "opportunity_ids": ["external/suite/case"],
    "rank_ceiling": 3,
    "override_reason": null
  },
  "mechanism": {
    "summary": "Remove one shared runtime cost.",
    "generality": "Explain why it applies beyond one benchmark shape.",
    "semantic_risks": ["direct-eval"]
  },
  "profile_evidence": [{
    "source": "artifact identifier or profile path",
    "sha256": "<profile SHA-256>",
    "base_sha": "<queue candidate SHA>",
    "opportunity_ids": ["external/suite/case"],
    "shared_cost": "Named shared cost from the profile.",
    "inclusive_fraction": 0.2
  }],
  "fast_gate": {
    "target_ids": ["external/suite/case"],
    "control_ids": ["broad/call-case"],
    "target_max_candidate_over_base": 0.95,
    "control_max_candidate_over_base": 1.03,
    "max_attempts": 2
  },
  "promotion_gate": {
    "require_complete_broad": true,
    "require_complete_external": true,
    "require_test262_zero_gap": true
  }
}
```

Validate before implementation:

```sh
./scripts/performance-decision.sh check-unit --unit tasks/performance-units/<unit>.json
./scripts/performance-decision.sh validate-unit \
  --unit tasks/performance-units/<unit>.json \
  --queue target/performance-opportunity.json
```

After a fast screen or promotion run, write a non-overwriting decision artifact:

```sh
./scripts/performance-decision.sh decide \
  --mode fast \
  --unit tasks/performance-units/<unit>.json \
  --queue target/performance-opportunity.json \
  --summary /path/to/summary.json \
  --broad-report /path/to/report.json \
  --external-report /path/to/external-report.json \
  --require-retained \
  --output target/performance-decision.json
```

Use `--mode promotion --test262-burndown <burndown.json>` only for the complete
portfolio. `rejected` is useful evidence and must be recorded in the task;
`inconclusive` means refresh or complete the evidence rather than calling it a
win.

## Staged migrations

The shape above is a `leaf` plan: one mechanism that must pay for itself
inside its attempt budget. An architectural migration cannot meet that gate at
its first commit, because its early stages move execution onto a new
representation before any of it is faster. Declare those as schema 2 with
`"unit_kind": "migration"` and a stage budget:

```json
{
  "schema_version": 2,
  "unit_kind": "migration",
  "migration": {
    "stages": 8,
    "current_stage": 1,
    "cumulative_target_ids": ["external/sunspider-1.0/controlflow-recursive"],
    "stage_max_candidate_over_base": 1.10
  }
}
```

Keep `base_sha` fixed at the migration base for every stage; that is what makes
each stage's evidence cumulative rather than a comparison against the
scaffolding commit before it. Bump only `current_stage`.

```sh
./scripts/performance-decision.sh decide --mode stage \
  --unit tasks/performance-units/<unit>.json \
  --queue target/performance-opportunity.json \
  --summary /path/to/summary.json \
  --broad-report /path/to/report.json \
  --external-report /path/to/external-report.json \
  --require-retained \
  --output target/performance-stage-<n>.json
```

A stage is `advance`, `abort`, or `inconclusive`. `advance` earns the right to
continue and is never a performance claim. `abort` closes that stage's
implementation shape and explicitly not its mechanism family — record which,
in those words. The migration itself is judged only at `current_stage ==
stages`, by the ordinary `--mode fast` / `--mode promotion` gate.
