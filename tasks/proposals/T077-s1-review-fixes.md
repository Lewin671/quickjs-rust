# T077 S1 review repair architecture

Scope: the owner brief at base `b2daec46`; layout internals, atom indexing and
explicit host construction order plus direct callers. S2-owned caches/compiler
paths remain untouched. No push or merge. Formal timing remains lead-owned.

Implementation stages:

1. Separate weak transition lookup (small vector, then NameMap; prune dead
   children on destruction, with amortized sweep as a backstop) from byte-bounded
   strong retention. Live children retain their
   append parents, so eviction cannot break lookup of a live shape chain.
2. Share append-only descriptor/key storage and its incremental ID index across
   prefix shapes. A shape owns a prefix length; lookups exclude later positions.
   Tail extension appends in amortized constant time; branching copies the prefix
   once. Dictionaries and reconfiguration detach before editing. Root never
   shares a nonempty buffer. Incremental allocation accounting covers live shapes, shared backing
   capacities, key text and weak-edge capacity. Above 1 MiB, strong keep-alive
   roots are evicted; externally live shapes remain valid and discoverable.
3. Cache attribute reconfiguration and bulk seal/freeze transitions separately
   from append edges. Bulk changes copy once per source/result pair; last-key
   deletion returns to the append parent. Other deletion/churn policy stays.
4. Remove production atom ID directory, use NameMap for symbol identities,
   disperse ID-index buckets with multiplication high bits, and port unordered
   host inputs to an explicit ordered constructor.

Verification: deterministic perf-counter tests for live fan-out, linear tail
construction, byte retention, bulk sharing and last-delete reuse; focused
attribute/accessor/branch/lifetime tests and NG host-order fixtures. Capture
baseline/candidate counters and process peak RSS on reproducible versions of
all review scenarios. Then fast gates per commit, full check, all NG fixtures,
combined agents/perf-counters tests, native/x86 checks, complete configured
release/agents exact Test262 scan and burndown. Record revisions, counts and
measurement limits in a new durable receipt and update the active resume block.
