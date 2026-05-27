# T002: Parser Expressions

## Goal

Expand expression parsing precedence while keeping parser behavior deterministic
and testable.

## Scope

- Allowed paths: `crates/qjs-parser/**`
- Forbidden paths: `third_party/**`
- Owner boundary: expression parser and parser tests

## Parallel Assignment

- Base sha:
- Branch: `agent/parser-expressions/<owner-id>`
- Worktree:
- Owner id:
- Integration owner:

## References

- `AGENTS.md`
- `docs/architecture.md`
- QuickJS-NG parser reference in `third_party/quickjs-ng/quickjs.c`

## Acceptance Criteria

- Added precedence levels have focused tests.
- Parser errors remain structured and include spans.
- Runtime behavior is not changed unless explicitly coordinated.

## Verification

```sh
cargo test -p qjs-parser
./scripts/check.sh
```

## Notes

Coordinate before changing AST node shapes.
