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

## Initial arm64 diagnostics and guard publication

`635ca6f8` (production `6a9c615d`), three alternating pairs, frozen isolated
release `agents` builds. RSS in decimal MB: million-object chain
119.49 pre-S2 / 1324.37 integration / 119.05 candidate; 200k one-child prototypes
58.31 / 287.03 / 54.53; 100k ten-property prototypes 46.91 / 201.67 / 46.45.
All seven memory programs are within 1.017 of pre-S2 RSS. Whole-process
reparent time 2.465 → 0.078 s; cascade 1 0.692 → 0.070 s; cascade 2
0.415 → 0.087 s; 6000-deep setPrototypeOf construction 11.772 → 1.231 s.
The inherited-write case is 0.458 → 0.491 s (1.072), with its deferred guard
still in cold metadata. Final correction publishes that guard into the direct
layout field once at cache admission; no eager root is added to creation.
It applies to all ordinary/Map/Set receivers. Array/function empty authorities
already have their final root and retain their allocation-free probe path.
A focused test preserves an issued guard through publication and retires it on
reparenting. This correction requires a new full qualification and measurement.


`9cd9c467` passes the full gate and 268 NG fixtures in all three modes; native
RSS remains within target. Its publication alone leaves inherited-write at
0.475 s versus integration 0.449 s (1.058). Final correction separates the
never-issued neutral seed identity from all semantic roots, including the
null-prototype root. Published hits can compare direct layout identity first;
cold deferred guards are consulted only after a miss. A test proves that two
different prototypes sharing a seed cannot match each other's guards or a
null-prototype guard. No benchmark-specific admission rule is added.


The first `3dc2a684` timing run is **disturbed**, not qualified performance
evidence: an independent S3b release build started at 2026-10-30 23:26:48 PDT
during collection. Its raw report remains local at
`target/t077-followup/review-arm64-seeds.json`; no timings are pooled into a
replacement. The retained collector now requires an initially quiet host and
monitors external compilers/qjs processes every 0.5 seconds, excluding its own
process tree. Any observed overlap marks the whole report disturbed. Complete
cached and IC-off exact scans run first; final timings will be collected after
their load ends. Runtime source stays frozen at `3dc2a684`.
