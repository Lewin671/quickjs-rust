# T041: Ordinary own-data cache hot answers on x86_64

- Status: closed-rejected; the frozen single screen failed.
- Implementation: reverted on the lead branch at `570b4d83`; retained for
  diagnosis at owner commit `33c7dfed`. The mechanism family is not closed.
- Verified at: `33c7dfed`, 2026-10-12; 12 cache tests, 11 NG comparisons,
  2255 runtime tests and 65 selected Test262 cases in the fast gate; full
  `scripts/check.sh` passed, including 5170 curated Test262 cases.
- Evidence: [frozen plan](../performance-units/own-data-cache-hot-answers-x86.json)
  binds queue SHA `7a4b49bbec9dc64a8d545a3d3b89fd85995b6fdc3144b543e220256b82cbf00f`.
  Raw comparison/profile files are workspace-local, not retained here.
- Unresolved: the cause of one slow wall-time pair is not established; there
  is no formal promotion evidence and the attempt budget is exhausted.
- Next action: none for this implementation; a future unit needs fresh
  profiling and a new plan, not another run of this frozen screen.

## Candidate layout

Job `20261013T042209Z-own-data-layout` built `33c7dfed`, profiled all three
portfolios and regenerated the x86_64 hot set, including the new OwnSlot
helper. Three relinks stabilized order SHA
`85eaf9984717c59fd14502e94ae141e954f78e6d6e576064fa62068352cb5a98`.
The linked VM/wide executor page offsets equal the requested `0xc40`/`0x400`.
This is address verification, not qualification of those offsets as fast.

The CLI selects a separate macOS x86_64 order file; arm64 keeps its prior file.
The layout tools use the corresponding native default and permit an explicit
other-target path. This prevents an x86_64 regeneration from replacing the
arm64 symbol list. Tool tests and the full local gate passed after this build
configuration change. No runtime screen or formal attempt has occurred.

## Scope and evidence

Owner paths: `crates/qjs-runtime/src/bytecode/named_property_cache.rs` and
`crates/qjs-runtime/src/bytecode/named_property_cache/tests.rs`. The lead owns
this task, the plan/indexes, any additional integration-level test location,
and layout tooling/order files. Do not import the old branch's plan or tasks.

The [T040](../T040-imac-general-performance.md) queue ranks hash-map seventh among
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

Use [the workflow](../../docs/performance-workflow.md). The plan owns the targets,
thresholds and attempt budget. The complete external portfolio, broad lane,
six generic sentinels and exact-candidate zero-gap Test262 evidence are
mandatory for promotion. Record every screen here, including failures.

## Screen attempt 1 and closure

Candidate `f2a6b4bf`, base `06f0ecd3`; job
`20261013T043629Z-own-data-screen-attempt1`, 5 paired samples. The host's
cycles are not a clock, so the frozen screen judges wall time. No disturbance
warning was emitted; raw screen output is local, not retained.

Hash-map median wall/base is **0.9393**, range **[0.9319, 1.4360]**;
instructions/base is **0.9322** in every printed pair. All control medians
are within 1.03; the largest is string-key-map-churn **1.0244**. The screen
is nevertheless **FAIL** because a target pair did not improve:

```text
target external/jetstream3-js-subset/hash-map: a pair did not improve (max 1.4360)
```

The implementation did not receive a formal run. The plan, thresholds and
samples were not changed, and no favorable rerun was taken. `570b4d83` removes
the two runtime/test files' experiment delta from the lead candidate; the
portable layout infrastructure remains. This is a rejected implementation on
this frozen gate, not proof that the general cache mechanism cannot improve
performance. At `33c7dfed`, exact-candidate CI also passes all **42672**
configured Test262 cases with zero actionable gap (coverage run 38346273370).
