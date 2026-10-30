# Core rebuild: owning values and backup cycle collection

Target for [core rebuild](core-rebuild.md). S1–S4 keep Rust Rc ownership;
S5 introduces a common counted allocation, S6 collects cycles. No precise root
discipline is introduced into builtin code. Paths below are under runtime `src/`.

## Value and common allocation (S5)

`Value` is private, owning, **non-Copy**. Move transfers one strong ownership;
Clone increments a heap count, Drop decrements it, last strong destroys once.
An immediate has no count. Borrowed `ValueView<'a>`/typed payload views are tied
to an owner; neither creates a count nor outlives the owner. A native/host keeps
a Value normally across callbacks/collections. Public handles remain owning,
not unrooted words. Heap identity is never an integer that can silently recycle.
Symbol language type stays distinct even if its payload remains Object-like.

Qualified 64-bit target: 8-byte NaN-tagged word; canonical numeric NaN
`0x7ff8000000000000`, reserved negative quiet-NaN tags for immediates and a
low-48-bit header pointer. Preserve signed zero/infinities and raw ArrayBuffer
bytes. TDZ/hole state has private nonescaping tags or frame bitmaps. No Copy
bit-pattern API outside the representation module. An int32 tag is optional,
not a prerequisite; Number arithmetic still follows exact JS rules.

One allocation contains header+aligned payload, never a new box around old Rc.
Proposed 64-bit `HeapHeader`: strong u32 + weak u32 (8), kind u16 + flags u16 +
collector state u32 (8), `ops:&'static HeapOps` (8), owning HeapDomain handle (8)
= **32 bytes**, excluding payload/alignment. `HeapOps` supplies destroy,
trace-edges and clear-edges. Weak count includes a live-payload sentinel; last
strong tombstones the payload before weak upgrade can succeed. Last weak frees
the allocation. Collector pins use separate bookkeeping, never user-visible
strong counts. Counter overflow aborts rather than wrapping, as infallible Clone
cannot report a JS exception; allocation/engine-limit APIs return structured errors.

All JS graph ownership uses this header: objects/arrays/functions/proxies,
Map/Set entries, shared upvalue cells, dynamic scopes, suspension/job/native
contexts and Value-bearing code/template metadata. Immutable leaf metadata may
keep std Rc only if it cannot own an edge back into the JS graph. Layouts and
KeyRecords that own symbol handles must be traceable graph nodes too; retaining
unregistered auxiliary Rc owners would hide cycles as external roots.
Strings and BigInts can be untraced leaf payloads with the same count ABI.

Keep RefCell or equivalent checked interior borrows. Do not replace borrow checks
with raw mutable references. String COW checks true strong uniqueness, excluding
weak observers/collector pins; interned strings never mutate. Teardown drains
an iterative release queue as `value/teardown.rs` does now, not recursive drops.
Weak IC handles neither pin receivers nor upgrade dead payloads; tombstones and
non-reused layout IDs prevent ABA. Escaping values keep their HeapDomain alive;
its allocation registry is weak, so it does not retain its own payload graph.
Each agent has its own non-Send/non-Sync heap; only existing shared buffer backing
crosses threads under `agents`. No global/default heap table.

## Pointer portability and unsafe audit

Only `value/repr/` (proposed) may encode/dereference pointers, manage custom
headers, change counts or reconstruct typed views. Local `allow(unsafe_code)`
and every block's SAFETY explanation cover tag/kind, alignment, original
allocation provenance, live counts, aliasing, destruction and weak upgrades.
Use provenance-preserving Rust APIs; never guess std Rc's header layout or cast
an arbitrary integer into a live reference. Miri exercises owning operations,
weak reuse and COW; sanitizers, randomized graph tests, deep-drop tests and both
production architectures complement it. Passing Test262 is not a memory audit.

Qualify pointer width/alignment for each build target and check every allocation
before encoding. Unsupported builds select a portable owning tagged layout at
compile time (budget 16 bytes), not a runtime engine backend. High-address
allocation on a packed build returns an explicit engine-limit error before
publication; never truncate, mask, or change representation mid-run. This remains
a production portability risk, not recoverable OOM support. An indexed-word
alternative needs a priced design for context-free Clone/Drop/escaping handles
and its extra indirection; no implicit process-global table is allowed.
S5 cannot close until pointer/count proofs and measured value traffic justify
unsafe over the safe enum. Default Vec/Box OOM may abort; no new universal
fallible-allocation promise is made.

## Trial-deletion collector (S6)

HeapDomain keeps a weak registry of traceable live headers, candidate debt and
reusable scratch. First implementation scans the complete registered graph at
allocation-debt/job boundaries; candidate-only refinement is later work. Collect
only outside payload borrows and user code. Ordinary safe Rust locals, host
handles, frame values and pending exceptions contribute external strong counts:
they need no enumeration or stack map to stay alive.

1. Pin live registered nodes without changing their snapshotted strong counts;
   initialize `trial[n] = strong[n]`. Enumerate **every physical owning edge**,
   subtracting its multiplicity, including conditional WeakMap value ownership.
   Assert no negative counts, missed/double edges or unregistered owning wrappers.
2. Nodes with residual counts are externally owned. Walk strong semantic edges
   from them; mark reachable nodes, but do not follow WeakMap conditional values.
   Compute ephemeron fixed point: a reachable map and reachable key make its
   value reachable. Weak key/target/token edges never contribute strong counts.
3. Unmarked nodes are garbage. Remove entries for dead weak keys, clear weak
   references before reuse, and record FinalizationRegistry cleanup work while
   retaining holdings/callbacks according to their strengths. Respect unregister
   tokens and WeakRef's job-level kept-alive set. Collection itself runs no JS.
4. Pin the entire garbage component, sever its outgoing owned edges once, then
   iteratively destroy payloads and release pins/tombstones. External counts
   cannot reach zero on a live borrowed owner. Clear/destroy/registry operations
   cannot re-enter JS or recursively collect; defer threshold checks while busy.
5. Enqueue cleanup only after the heap is consistent. Drain according to host
   job policy, never synchronously inside sweep. No finalizer can resurrect a
   reclaimed payload. Holdings that keep targets alive continue to do so.

Native strong references not registered as graph nodes are conservative external
owners and can retain garbage, never free it early; therefore enumerate all
engine-owned Value containers before claiming cycles stop leaking. Document host
extension edge hooks explicitly. Debug edge verification compares trial counts
to a test graph oracle and poisons dead payloads/generations. Stress collection
at every permitted allocation boundary, with re-entry/borrow exclusion asserted.

`weak_ref.rs` currently stores its target as a strong Property;
`weak_map.rs` uses Map storage and `finalization_registry.rs` stores cell arrays.
S6 replaces these with typed weak slots/ephemerons, deleting the strong stand-ins.
Simply adding trial deletion while leaving them strong does not meet this stage.

Required tests: self/mutual object cycles, closure↔cell, realm/code/template,
prototype/symbol-key, suspended async/generator and native-context cycles; host
escape survives; dropping last host owner permits collection; ephemeron chains
and dead-key/value cycles; kept-alive job boundary; unregister and deferred
cleanup; weak address reuse; long acyclic drops; count-overflow handling.
Track allocated/live/external/scratch/retained bytes and pause p50/p99/max.
Repeated run/drop/collect returns live nodes to root baseline and releases empty
pool capacity under a bounded reserve. RSS and collector work are measured,
never assumed to improve because cycles now die. Future tracing can reuse typed
edge visitation, but is neither an S6 dependency nor a hidden root API migration.
