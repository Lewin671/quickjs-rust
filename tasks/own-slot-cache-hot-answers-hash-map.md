# OwnSlot hot answers, hash-map target

- Status: active.
- Implementation: none; temporary diagnostic hooks restored to e54e33eb.
- Verified at: e54e33eb, 2026-10-12; three corpus checksums and per-site counter coverage, matched 4/8 instruction prices, fresh exact-base sampling receipt. Details in the diagnostic record.
- Evidence: [site diagnosis](archive/e54-named-site-miss-diagnosis.md); raw target artifacts are local, not retained.
- Unresolved: the new OwnSlot selector must avoid the rejected shape's miss-path and code-generation costs; no new screen yet.
- Next action: freeze and validate the hash-map-only replacement plan before production runtime edits.

## Scope and predecessor

Base e54e33eb, branch agent/own-slot-cache-hot-answers-hash-map/codex, isolated
worktree created by the agent script. Reopens own-slot-cache-hot-answers, frozen
in 2e879910 and rejected/reverted in 86e1a462 on its own branch. The lead corrected
its target selection: hash-map is the sole target; cdjs, 3d-raytrace, trees,
ai-astar and the six sentinels are controls. A fresh exact-base sampler receipt
and the opcode-counter diagnosis precede the replacement plan.

Production boundary: named_property_cache.rs, compact_fn/property.rs and their
tests; this plan, task and index rows. Diagnostic-only VM read/call hooks were
restored before this documentation commit. No typed-loop, vm_bindings, vm_ops,
class-field, order-file or layout budget changes. B stays held at 644e1de4 with
one unused attempt; this branch neither imports nor changes B.

## Diagnosis established before the reopen

Hash-map's object methods are already hot: 2,069,997 / 2,070,029 method-site reads.
Its OwnSlot cold answers number 3,510,015. The all-cold method price multiplied
by all method calls overstated its gap share. In trees, both recursive itemCheck
sites miss every time on different receivers despite one identical layout and
an already-cached holder. CDjs combines that receiver-proof issue with 596,719
primitive-string localeCompare reads outside the object hot path, and a VM
cached-prototype path that never remembers its own-miss proof. The full record
ranks every observed read site and lists the resolved-call sites and admissions.

Old A adds ~39 instructions to Exact/literal reads and ~10 to prototype reads;
its cdjs mix explains ~46M of the ~49M added instructions. This is real work,
not a cycle-placement diagnosis. Keep the old three hot-entry dispatch cases
and predecode OwnSlot eligibility in the existing hot-index byte; only a marked
OwnSlot calls the borrowed helper. All cache mutation/probe paths must maintain
the hint; identity, layout and live data-slot guards remain mandatory.
