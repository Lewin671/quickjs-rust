# T041: Ordinary own-data cache hot answers on x86_64

- Status: active.
- Implementation: none; the existing unpromoted production diff will be reused
  only after this plan is committed.
- Verified at: base `06f0ecd3`, 2026-10-12; exact-binary profile receipts and
  `validate-unit --profile-root` passed for the new x86_64 queue.
- Evidence: [frozen plan](performance-units/own-data-cache-hot-answers-x86.json)
  binds queue SHA `7a4b49bbec9dc64a8d545a3d3b89fd85995b6fdc3144b543e220256b82cbf00f`.
  Raw comparison/profile files are workspace-local, not retained here.
- Unresolved: implementation, additional state-transition coverage, x86_64
  layout verification, the single screen, and formal promotion evidence.
- Next action: apply only `95c3af21`'s runtime/cache-test diff in an isolated
  owner checkout, add the missing guard/fallback tests, and run correctness.

## Scope and evidence

Owner paths: `crates/qjs-runtime/src/bytecode/named_property_cache.rs` and
`crates/qjs-runtime/src/bytecode/named_property_cache/tests.rs`. The lead owns
this task, the plan/indexes, any additional integration-level test location,
and layout tooling/order files. Do not import the old branch's plan or tasks.

The [T040](T040-imac-general-performance.md) queue ranks hash-map seventh among
external opportunities. Three uninstrumented samples of the exact executable
show ordinary named-read envelopes of 64/528, 71/526 and 67/528. Within that
combined envelope, 101/1582 samples pass through cold cache probing and 46
through its live own-data slot reader; those are nested subsets, not additive
removable fractions. Current cold-path code and these stacks justify testing
the existing general cache mechanism, not assuming its previous ARM64 result.

The existing implementation is `95c3af21`; its old screen and attempt counts
remain history. This plan gives one implementation screen on the new host.
It does not authorize repeated layout/style variations until one passes.

## Correctness boundaries

Keep live value, Weak receiver identity, layout revision, slot-storage and
accessor checks. Cover selected OwnSlot replacement by other entry kinds,
non-selected Exact-to-OwnSlot conversion and subsequent selection, clearing,
and an observable own-data → own getter → inherited getter → Proxy fallback
chain. Cache borrows must never span user-code execution.

## Verification and decision

Use [the workflow](../docs/performance-workflow.md). The plan owns the targets,
thresholds and attempt budget. The complete external portfolio, broad lane,
six generic sentinels and exact-candidate zero-gap Test262 evidence are
mandatory for promotion. Record every screen here, including failures.
