# T004: QuickJS Comparison Coverage

> Historical record. Describes the repository at the revisions named below; not current guidance.

- Status: historical-campaign (open-ended bootstrap work item; it never had a
  finite exit).
- Implementation: present (`scripts/compare-qjs.sh`,
  `tests/fixtures/compare-qjs/`); this file never recorded which slices
  landed.
- Verified at: not recorded in this file. Later conformance record: 42,672 of
  42,672 configured Test262 cases pass at `9d344a0f`, 2026-09-06
  (`docs/conformance/burndown.jsonl`, last entry).
- Evidence: `docs/conformance/burndown.jsonl`.
- Unresolved: none recorded.
- Next action: none. New work in this area is selected through the gap queue
  (`AGENTS.md`), not this file.

## Goal

Grow the local QuickJS-NG comparison suite with small smoke programs that match
implemented runtime features.

## Scope

- Allowed paths: `tests/fixtures/compare-qjs/**`, `scripts/compare-qjs.sh`
- Forbidden paths: `third_party/**`
- Owner boundary: comparison fixtures and comparison runner behavior

## Parallel Assignment

- Base sha:
- Branch: `agent/quickjs-comparison/<owner-id>`
- Worktree:
- Owner id:
- Integration owner:

## References

- `AGENTS.md`
- `docs/harness.md`
- `scripts/compare-qjs.sh`

## Acceptance Criteria

- Fixtures are small and inspectable.
- Runner output is deterministic.
- New fixtures only cover behavior currently implemented by `qjs-cli`.

## Verification

```sh
./scripts/compare-qjs.sh
./scripts/check.sh
```

## Notes

Do not turn this into a full Test262 runner.
