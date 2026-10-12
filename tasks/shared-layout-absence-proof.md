# Shared-layout absence proof (Brief 07)

- Status: active; plan frozen before runtime implementation.
- Implementation: none; coding base includes the preceding OwnSlot leaf.
- Verified at: `95c3af21`, 2026-10-12; base counters and diagnostic feature
  ladder in progress. Plan validates against the authorized e54 queue.
- Evidence: frozen [plan](performance-units/shared-layout-absence-proof.json);
  conclusions below; raw target/ and /tmp artifacts are local, not retained.
- Unresolved: Part 1 ladder completion, two implementation screens, correctness
  and code-size gates; no promotion evidence.
- Next action: finish the call-route diagnosis and write the lead's findings,
  then implement the frozen absence mechanism and screen it under the host lock.

## Scope and provenance

Branch `agent/shared-layout-absence-proof/codex`, coding base
`95c3af21e3e37d6775e5606d0424314638ce3e42`. Worktree:
`../quickjs-rust-worktrees/wt-quickjs-rust-shared-layout-absence-proof-codex`.
Main's uncommitted typed_loop work is untouched. No other owner's branch is
merged. No closure/capture, class-field, Date, concat, typed_loop, vm_bindings,
wide activation or linker-order edits are authorized.

The lookup boundary consists of `bytecode/named_property_cache.rs` and its
new child proof module, `compact_fn/property.rs`, `bytecode/vm_props.rs`,
`value/object/slot_reads.rs` and its new layout helper, and `value/object.rs`
for deletion-history metadata in existing cold storage. Tests, this record,
the frozen plan and their index rows complete the boundary.

The integration queue in T037 binds af4d0d3c, not the coding base. Unmodified
base-branch `validate-unit` rejected it with exact output:
`error: profile verification requires a current schema-2 queue`.
Calling its identity validation separately produced:
`PerformanceDecisionError: performance unit base SHA must equal the queue candidate SHA`.
No queue or validator was edited. Brief 07 explicitly permits the e54 fallback:
`<main>/target/comparison/brief01-e54e33eb-20261011-queue.json`, SHA-256
`d064c7e9a06d2e7d2d998a26bc1d0febb0dc7e8177ccb7508d0d0f7108442e8c`.
Thus the frozen evidence/formal base is e54e33eb, while the coding base and
supplemental isolated A/B base are 95c3af21. A result against one is never
reported as a measurement against the other.

The e54 exact-binary tree/CDjs sampler receipts in
`<main>/target/profiles/brief01-e54e33eb` validate, including their hashed
profile/workload assets. Existing per-site counts are recorded in
[the diagnosis](archive/e54-named-site-miss-diagnosis.md); e54-to-95c3af21
changes only cache hot OwnSlot answers/tests/docs, leaving call admission,
bytecode and workloads unchanged. Fresh aggregate base counters corroborate
that call inventory. The complete descriptor/name layout, not a key count
or one interned name, is the proposed absence proof.

The ideal supplied lookup budget implies about 4.8% whole-tree and 1.8%
whole-CDjs instructions before layout verification; a 5% whole-CDjs ceiling
would exceed that reach. Before any runtime code or unit screen, freeze a
1% cycles payoff ceiling with every target pair improving. Controls are
judged on instructions and cycles, with placement handled as Brief 07 directs.
The snapshot checks all compact keys/descriptors; it is a structural shared
layout value, not a new canonical hidden-class/id subsystem. Dictionary,
deleted, accessor and exotic layouts decline sharing.

## Commands

```sh
./scripts/performance-decision.sh validate-unit --unit tasks/performance-units/shared-layout-absence-proof.json --queue /Users/qingyingliu/Code/js-engine/quickjs-rust/target/comparison/brief01-e54e33eb-20261011-queue.json --profile-root /Users/qingyingliu/Code/js-engine/quickjs-rust/target/profiles/brief01-e54e33eb
lockf /tmp/qjs-lead/host-timing.lock env PYTHONPATH=. python3 target/brief07/ladder.py diagnose
lockf /tmp/qjs-lead/host-timing.lock env PYTHONPATH=. python3 target/brief07/ladder.py measure
```

No formal 30-block comparison is authorized; the lead integrates/promotes.
