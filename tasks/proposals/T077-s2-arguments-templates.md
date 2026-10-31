# S2 design correction: realm-owned arguments schemas

Frozen before implementation. Append-label job
`20261030T194632Z-t077-s2-r2-shape-labels`, source `74ba5e56`, identifies
223,722 graphlib append misses: length 42,070; index 0 42,058; callee
28,481 and iterator symbols 28,484, plus larger arities. Moment's 234,228
misses include length 43,210, index 0 43,211, callee 43,200 and iterator
symbols 43,203. These are arguments schemas, not dynamic adjacency maps.
The trace allocates/logs and is excluded from allocation/timing evidence.

String-only code recipes deliberately do not retain arbitrary symbol keys.
Arguments schemas use the realm's already-owned intrinsic iterator symbol.
Add an explicitly realm-owned bounded cache (32 shared layouts) for argument
construction. Layouts retain weak prototypes, key IDs/leases and descriptor
metadata only; accessor closures, callee values, argument values and parameter
cells remain owned by each actual arguments object. Admit only shared layouts,
the intrinsic Object prototype/domain and symbols already in this realm's
well-known-symbol vector. No user-symbol or receiver pin and no domain cycle.

First retain completed argument layouts by identity, without cloning on a
cache hit. Existing construction then reuses the transition chain instead of
rebuilding it after keep-alive eviction. If measured work still requires it,
add direct per-arity/template filling through the same NamedStore; do not add
a second authority. Large/private layouts stay on its ordinary edit API.
Mapped parameter accessors and shared cells, strict poison callee, iterator
identity, tag and cross-realm behavior stay unchanged. No binding snapshot,
new dependency, unsafe, benchmark template or held-out work.

The same evidence leaves constructor work in class-heavy programs. A subsequent
own-prototype slot link may guard function layout/version and read the live slot
through NamedStore, avoiding interning/descriptor reconstruction; it must never
cache the prototype value or bypass lazy/accessor handling.
