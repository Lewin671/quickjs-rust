# T003: Runtime Values

## Goal

Introduce a stronger JavaScript value model as the foundation for production
runtime semantics.

## Scope

- Allowed paths: `crates/qjs-runtime/**`
- Forbidden paths: `third_party/**`
- Owner boundary: runtime values, coercion helpers, runtime tests

## Parallel Assignment

- Base sha:
- Branch: `agent/runtime-values/<owner-id>`
- Worktree:
- Owner id:
- Integration owner:

## References

- `AGENTS.md`
- `docs/architecture.md`
- QuickJS-NG runtime reference in `third_party/quickjs-ng/quickjs.c`

## Acceptance Criteria

- Value operations are typed and deterministic.
- Runtime errors remain structured.
- New behavior has focused runtime tests and, when useful, QuickJS comparison
  fixtures.

## Verification

```sh
cargo test -p qjs-runtime
./scripts/compare-qjs.sh
./scripts/check.sh
```

## Notes

Coordinate before changing parser or AST contracts.
