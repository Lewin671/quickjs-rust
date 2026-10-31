# S2 follow-up: prototype memory and complexity

- Status: active; implementation in progress.
- Implementation: all five requested mechanisms present; prototype roots and validity graphs are demand-driven; shared holders no longer copy descriptors.
- Verified at: `f35bf831` first-increment fast gate passes (2,497 tests, 65 selected Test262); memory increment fast gate passes (2,502 tests, no matching curated slice). Full gates pending.
- Evidence: this record and [retained review programs/collector](t077-s2-followup/measure.py).
- Unresolved: final acceptance gates and measured memory/performance standing.
- Next action: qualify the committed memory migration with full correctness and performance gates.

## Frozen scope and migration

Base `1e6e2a2b`, branch `agent/t077-s2-followup/codex`. Runtime scope is
`value/layout/`, prototype cycle checks in `value/object.rs`, named-property
cache constants and their tests. No function/env/call/context or bindings edits.
No dependency, unsafe, source-template recognizer, push or merge.

1. Replace recursive linear-visited cycle checking with an iterative identity
   set; reject virtual constant admission before fetching; inline constant load.
2. Keep prototype holders on immutable shared shapes. Put validity state and
   accessor slots in per-store cold metadata. Create validity graphs only when
   a dependency is requested, and create prototype-sensitive empty roots only
   when a guard or named mutation needs them. Empty construction must retain
   only an inline live prototype edge, with no per-object shape/state/edge box.
   The dictionary epoch moves into cold metadata, making room for the inline
   edge without growing NamedStore; array/function empty seeds keep their
   existing compact shared representation. Guards must always use
   the materialized prototype-sensitive layout, never the deferred empty seed.
3. Reparent through stale weak links with amortized compaction; invalidate only
   armed subtrees. Issuing a token arms every ancestor, so a previously invalid
   ancestor cannot hide a newly issued descendant token. Preserve the existing
   four-overwrite policy and trackability through proxy ancestors.

Each increment receives focused tests and the fast gate before commit. Final
qualification: full check, all NG fixtures default/off/verify, complete exact
cached/off scans with burndown, review time/RSS on arm64 and x86, and iMac
public86 against the integration base. The iMac wrapper itself pushes runner
refs; clarification of the explicit no-push constraint is pending.

## Implementation checks

`f35bf831` is the committed cycle/constant increment. The memory migration
passes 2,502 runtime tests; the five new deterministic graph/laziness tests also
pass separately, including the 16,000-node nonrecursive demand/invalidation walk
and an explicit visit-count assertion for repeated invalidation. Clippy with
all runtime targets and `-D warnings` passes. Shared append plans now execute
the common invalidation barrier, including when an existing receiver becomes
a prototype after the plan was admitted. Physical assertions remain 64-byte
NamedStore, 72-byte RefCell and 96-byte ordinary payload.

## Frozen candidate qualification (in progress)

Production source `6a9c615d`, integration base `1e6e2a2b`, review reference
`d38f30a7`. Isolated release `agents` CLI SHA-256
`759a0896898b5e98dbf52428f31fa1fc03be53d623d0a1ca8ec0ca702059333c`;
actual `--agent` smoke prints `agent-ready`. All 268 NG fixtures pass in each
of release cached, release IC-off, and debug IC-verify modes. The five new
prototype tests pass on arm64 and x86_64 (local Rosetta); this checks both
representations but is not iMac performance evidence. A stronger virtual
admission test uses a genuinely lazy user-function prototype with an installed
validity state, and proves both laziness and the named version remain unchanged.

The iMac queue was read successfully and was idle; local clock there was
2026-10-31 13:58 CST, outside the prohibited window. No runner job has been
submitted: `imac-bench.sh` uses `git push` internally, and the explicit no-push
instruction remains in force pending the requested clarification.
