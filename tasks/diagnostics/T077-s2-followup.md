# S2 follow-up: prototype memory and complexity

- Status: active; implementation in progress.
- Implementation: cycle walk and constant admission repairs present; memory migration planned below.
- Verified at: first increment on `1e6e2a2b`: fast gate passes (2,497 runtime tests, 65 selected Test262 cases); deep-chain and virtual-admission tests pass.
- Evidence: this record; review programs originally supplied in `/tmp/qjs-s2r/t` will be retained with measurements.
- Unresolved: memory migration and all requested acceptance gates.
- Next action: implement demand-driven prototype metadata and run focused tests.

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
   The inline edge adds eight bytes to NamedStore; array/function empty seeds
   keep their existing compact shared representation. Guards must always use
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
