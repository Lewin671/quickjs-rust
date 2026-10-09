# T001: Lexer Coverage

> Historical record. Describes the repository at the revisions named below; not current guidance.

- Status: historical-campaign (open-ended bootstrap work item; it never had a
  finite exit).
- Implementation: present (`crates/qjs-lexer/`); this file never recorded
  which slices landed.
- Verified at: not recorded in this file. Later conformance record: 42,672 of
  42,672 configured Test262 cases pass at `9d344a0f`, 2026-09-06
  (`docs/conformance/burndown.jsonl`, last entry).
- Evidence: `docs/conformance/burndown.jsonl`.
- Unresolved: none recorded.
- Next action: none. New work in this area is selected through the gap queue
  (`AGENTS.md`), not this file.

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
