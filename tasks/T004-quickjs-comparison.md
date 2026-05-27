# T004: QuickJS Comparison Coverage

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
