# OwnSlot cache hot answers

- Status: active.
- Implementation: none.
- Verified at: e54e33eb; plan validated against exact brief01 queue and amplified receipts; check-context passed.
- Evidence: this record; local artifacts under target/ are not retained.
- Unresolved: net savings and code placement; no formal promotion authorized.
- Next action: implement after this frozen-plan commit.

## Goal and scope

Deliver successful OwnSlot reads through the existing guarded hot path.
[The frozen plan](performance-units/own-slot-cache-hot-answers.json) owns cases,
thresholds and the two-attempt budget. Lead selected this leaf after T036 pricing
on design branch 6a7bf946: hash-map 3,510,015, cdjs 28,022 and raytrace 78,441
successful OwnSlot reads; tree has none. The 0.9999 target ceiling reflects cdjs's
small reach, not a claimed 5% gain. All targets still need improved median cycles
and every paired target below 1 for a diagnostic pass.

Base e54e33eb, branch agent/own-slot-cache-hot-answers/codex. Runtime boundary:
`bytecode/named_property_cache.rs`, its tests and `compact_fn/property.rs`.
No typed-loop, bindings, class-field, equality, layout-order or budget changes.
Only this plan and its index row are owned in performance-units.

Unlike the rejected T033 prototype pre-probe, this serves an already guarded
own answer; it never assumes a new receiver misses a prototype method name.
Use a borrowed out-of-line helper to limit hot executor and probe growth.

## Verification and result

All measurements use lockf /tmp/qjs-lead/host-timing.lock and wait for competing
build processes to finish. Screens are diagnostic, never formal retention.
A control cycle failure at unchanged instructions is reported with symbol
positions/sizes as placement; it does not justify a second attempt or repinning.
The required full gate, focused negative tests and compare-qjs run before handoff.
