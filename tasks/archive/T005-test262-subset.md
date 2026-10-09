# T005: Test262 Subset Harness

> Historical record. Describes the repository at the revisions named below; not current guidance.

- Status: historical-campaign (open-ended bootstrap work item; it never had a
  finite exit).
- Implementation: present (`scripts/test262-subset.sh`,
  `tests/test262/allowlist.txt`); this file never recorded which slices
  landed.
- Verified at: not recorded in this file. Later conformance record: 42,672 of
  42,672 configured Test262 cases pass at `9d344a0f`, 2026-09-06
  (`docs/conformance/burndown.jsonl`, last entry).
- Evidence: `docs/conformance/burndown.jsonl`.
- Unresolved: none recorded. The note below that full Test262 runs are not yet
  useful is obsolete: full scans are recorded in
  `docs/conformance/burndown.jsonl`.
- Next action: none. New work in this area is selected through the gap queue
  (`AGENTS.md`), not this file.

## Goal

Evolve the Test262 subset metadata into a deterministic runner for curated
standard tests.

## Scope

- Allowed paths: `tests/test262/**`, `scripts/test262-subset.sh`
- Forbidden paths: `third_party/**`
- Owner boundary: Test262 allowlist, expected failures, subset runner

## Parallel Assignment

- Base sha:
- Branch: `agent/test262-subset/<owner-id>`
- Worktree:
- Owner id:
- Integration owner:

## References

- `AGENTS.md`
- `docs/harness.md`
- `third_party/test262/INTERPRETING.md`

## Acceptance Criteria

- Allowlist entries are validated against the pinned Test262 checkout.
- Expected failures include reasons.
- Runner behavior is deterministic and suitable for CI.

## Verification

```sh
./scripts/test262-subset.sh
./scripts/check.sh
```

## Notes

Keep the selected subset small. Full Test262 runs are not useful until the
engine supports enough language surface.
