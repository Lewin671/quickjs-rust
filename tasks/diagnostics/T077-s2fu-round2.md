# T077 S2 follow-up, lead feedback round 2

- Status: active; checked clean branch at `3c6bec4e`; executing A, then B, C, D.
- Implementation: A present: direct own guards, private-layout epochs and inline inherited constants; B–D pending.
- Verified at: A increment on `3c6bec4e` passes 2,507 runtime tests and the fast gate, including 65 selected Test262 cases; iMac lane measurements pending.
- Evidence: this record; lead jobs `20261031T083411Z-t077-s2fu-public86` and `20261031T083411Z-t077-s2fu-rss15`.
- Unresolved: all round-2 acceptance gates.
- Next action: remove A's hot-path overhead and measure public86 before B.

## Authority and ordered scope

Runner-ref transfer through `imac-bench.sh` is explicitly authorized by the
2026-10-31 feedback. No push to origin, no merge into main, no cancellation of
others' jobs. Check status/time first; no iMac timing in local 02:45–05:00.
Base for performance is integration `1e6e2a2b`; baseline owner tip `3c6bec4e`.
C permits only the function-target cycle helper and its focused tests under
`function/`; the other S3b paths remain outside this owner's boundary.

Lead public86 lane wall/instruction ratios: development 1.0178/1.0138,
external 1.0040/1.0086, broad 0.9974/1.0111, sentinel 0.9973/1.0087.
Lead 15-pair development RSS geomean 1.0016 passes. Round 1 is not performance
accepted: ~1% added instructions must be removed.

## A target and migration

Own-data guards compare the directly published layout only; deferred empty
receivers cannot contain own data. Full/missing/inherited guards retain the
cold deferred-root fallback. Keep the four-way shared-own identity mask free
of seed tests. Move the dictionary epoch from receiver cold storage into its
private Layout, beside the metadata already loaded for identity guarding;
shared layouts keep epoch zero. The existing structural version/rollover
contract remains, and no hot payload grows. This also removes incidental cold
boxes on ordinary dictionary stores, keeping ordinary value writes on the
single absent-cold-pointer branch. Inline the inherited constant decoder into
its hit caller. Verify private-version invalidation, deferred guard semantics
and holder value-write barriers before committing and measuring.

B then makes tiny property metadata cheap and adds all three owned-child
patterns to the memory gate (<=1.5 pre-S2 peak, goal <=1.25). C then removes
allocation/recursion from both cycle walks. D then corrects the admission
comment. Final qualification covers full check, all NG modes, complete exact
cached/off with burndown, native/x86 memory/complexity and final public86.
