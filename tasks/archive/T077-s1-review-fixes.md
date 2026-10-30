# T077 S1 review repair verification

Owner branch `agent/t077-s1-property-authority/codex`, repair base `b2daec46`.
The [repair design](../proposals/T077-s1-review-fixes.md) replaces the reviewed
layout cache policy. The [previous receipt](T077-s1-verification.md) remains
revision-bound history. No push or merge is authorized.

## Implementation and focused evidence

Weak vector/hashed transition edges preserve all live children. Children retain
source paths; only append edges support last-key rollback. Prefix shapes share
append-only descriptors and an incremental ID index; branches detach once.
Individual attribute changes and bulk seal/freeze share cached result shapes.
No-op integrity changes copy nothing; preventExtensions changes only its flag.
The keep-alive cache evicts against a 1 MiB incrementally tracked allocation
budget, including shared buffer growth, key text estimates and weak-edge capacity.
Live layouts may exceed this budget; then strong retention drains. Dictionary
width/non-last-delete/edit policy and physical object/store budgets are preserved.
Production atom interning omits the ID directory; symbol identities use NameMap.
ID buckets use high multiplication bits and have a strided-ID regression.

Regression tests cover live fan-out beyond cache capacity, prefix isolation,
branching, descriptor/accessor integrity, linear tail copies, deletion rollback,
long-key/shared-tail retention and unchanged slot/value semantics. Initial full
perf-counter run passed 2,450 tests; combined agents/perf-counters passed 2,463.
The layout increment fast gate passed (default runtime tests, clippy, format,
context and size checks); no focused Test262 filter matched these paths, so
full/NG/exact gates remain required. Final source verification is pending below.
Host ordering, final counters/RSS and complete exit gates are still in progress.
