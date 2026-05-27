# T005: Test262 Subset Harness

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
