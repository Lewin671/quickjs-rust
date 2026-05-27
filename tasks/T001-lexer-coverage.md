# T001: Lexer Coverage

## Goal

Expand `qjs-lexer` toward QuickJS-compatible tokenization in small verified
slices.

## Scope

- Allowed paths: `crates/qjs-lexer/**`
- Forbidden paths: `third_party/**`
- Owner boundary: lexer tokenization and lexer tests

## Parallel Assignment

- Base sha:
- Branch: `agent/lexer-coverage/<owner-id>`
- Worktree:
- Owner id:
- Integration owner:

## References

- `AGENTS.md`
- `docs/architecture.md`
- QuickJS-NG lexer/parser reference in `third_party/quickjs-ng/quickjs.c`

## Acceptance Criteria

- New tokens preserve byte spans.
- Malformed input returns `LexError` rather than panicking.
- Focused lexer tests cover each new token class.

## Verification

```sh
cargo test -p qjs-lexer
./scripts/check.sh
```

## Notes

Coordinate before changing shared AST or parser behavior.
