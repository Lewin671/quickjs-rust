# S2 round-2 construction measurement method

Compare accepted main `dcee3c56` (runtime `71294a20`) against merge `28206401`
before runtime optimization. The lead's round-2 table fixes the targets; no
held-out input may be read or run. First collect work, then choose mechanisms.

`T077-s2-construction-probe.py` creates diagnostic source copies from Git and
adds the same operation counters to both representations. Never time these
copies. They use perf-counters/alloc-counters; canonical unmodified binaries
supply retired instructions in a separate non-timing iMac job.

- Object/array counts are actual Rc header allocations, not handle clones.
- SchemaRc wraps owned Layout/literal-schema Rc references without allocation
  or unsafe. Acquisitions, explicit strong clones, weak upgrades and releases
  are separate events; borrowed Rc parameters remain borrowed. It tracks schema
  ownership, not every Rc used by strings, values, contexts or prototypes.
- KeyLease/Atom clone/drop/weak-upgrade counters cover the key identity APIs;
  base Small/Dynamic stores also own Rc string names, not atom leases. Do not
  interpret zero base atom activity as zero stored-name ownership.
- Slot growth counts initial allocation/reallocation and capacity-byte increases;
  adopted buffers are reported separately. Base two-value inline literals have
  no separate buffer. Slots are Values in S2 and descriptors/name pairs in base.
- RefCell counts cover object/schema/interner storage; CallEnv/operand borrows
  are excluded. Hash completion counts all internal NameHasher invocations,
  including bindings/rehashes, alongside S2's text/ID/transition counters.
- Allocator API requests/bytes and physical-block counters are whole-process
  totals, not all attributable to ordinary object construction. Normalization
  per created object is explicit; it must not be presented as causal cost.
- Retaining constructor/literal/class/JSON controls at N and 2N cancels fixed
  bootstrap work. Real programs retain their pinned inputs/checksums and report
  raw counts plus per-object normalization. Counts are collected in release mode
  so debug verification does not contaminate the breakdown.

The [repair plan](../proposals/T077-s2-round2-construction.md) was frozen after the before-state.
The probe follows inline slot spills/adoption, direct pair literals, store-add
plans and JSON fills without treating inline capacity as a heap allocation.
Its original Vec path still builds at `28206401`; post-repair source `e320817e`
builds with the same diagnostic ownership wrappers. Non-timing controls validate
stdout; only unmodified canonical images provide retired instructions.
Array header work follows construction repair, as directed. Prior S2 correctness
and measurement standing remains bound to the round-1 revisions in its receipt.

An optional third probe argument `--trace-shapes` prints append-miss width/key
labels for diagnosis. These copies allocate/log while tracing: their allocator
counts and timing are excluded from work/payoff evidence. Canonical stdout is
still checked. The default probe has no tracing code or additional branch.

The immutable-mono cohort additionally instruments named-IC metadata reads/writes
separately from storage borrows, plus allocated IC payload header counts/bytes.
These new fields are absent from older frozen cohorts; a freshly instrumented
main provides their paired comparison. Payload bytes exclude Rc headers and
allocator rounding; bytes/header gives the actual production Rust type size.

`construction-collect.py --external-extras-only` is a separate seven-case
diagnosis of measured public regressions: nbody, nsieve, date-format-xparb,
3d-morph, 3d-raytrace, JetStream gaussian-blur and Kraken json-stringify. It does
not replace or change the frozen 24-case construction inventory.

Owned-cache cohorts count inline site construction/bytes separately from shared
Rc payload headers; local code clones start fresh metadata and retain only their
immutable slot configuration. Additional counters partition local-slot queries
and default-array seed reads, which were absent from earlier frozen records.
