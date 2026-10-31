# S2 guarded numeric stores

Frozen before implementation. `4658892c` instruction job
`20261030T215045Z-t077-s2-r2-slot-load-instructions-r2` still reports access-nbody
1.2814 main instructions after read refinement. Main's SetNamedTyped arm calls
shared_data_slot_write_number directly before its boxed fallback. S2 migrated
that consumer entirely to the generic boxed write, losing this typed path.

Restore an inline numeric write through NamedPropertyCache and NamedStore:
weak layout/version + own writable data flags + slot guard, one mutable borrow,
then update the live numeric payload. A non-number old value receives a normal
Value::Number replacement/drop; changing types never invalidates layout.
No hashing, descriptor construction, cached value, receiver-address guard or
unsafe. Actual realm globals remain excluded before any effect, preserving
binding-cell synchronization; readonly/accessor/exotic misses keep fallback.

Use mono/four-way entries already present; do not add a second slot index.
Validate type changes, dictionary slot movement/epochs, readonly descriptors,
global aliases and native comparisons, then remeasure the same public programs.
No native ABI/S3 binding change, dependency, benchmark template or held-out run.
