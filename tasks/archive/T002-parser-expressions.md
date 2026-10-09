# T002: Parser Expressions

> Historical record. Describes the repository at the revisions named below; not current guidance.

- Status: historical-campaign (open-ended bootstrap work item; it never had a
  finite exit).
- Implementation: present (`crates/qjs-parser/`); this file never recorded
  which slices landed.
- Verified at: not recorded in this file. Later conformance record: 42,672 of
  42,672 configured Test262 cases pass at `9d344a0f`, 2026-09-06
  (`docs/conformance/burndown.jsonl`, last entry).
- Evidence: `docs/conformance/burndown.jsonl`.
- Unresolved: none recorded.
- Next action: none. New work in this area is selected through the gap queue
  (`AGENTS.md`), not this file.

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
