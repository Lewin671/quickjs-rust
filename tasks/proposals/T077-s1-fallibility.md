# T077 S1: key lifetime and allocation failure

Status: **decided by the lead, 2026-10-30**. The former fallible-API proposal is
superseded. Property mutation/definition, native constructor creation, builtin
installation and module-realm initialization keep their existing error ABIs.

KeyIds are recycled after the last KeyLease dies. The domain's free list is
used before minting another ID; the weak ID directory never pins a record.
Strings and symbols share these rules. A bare ID borrows its lease's lifetime.
Shapes, dictionaries and linked code retain leases; caches retain a lease or
validate a live layout identity/version. Layout identities/versions do not
recycle. An old structure therefore either prevents reuse by owning the lease,
or has expired and cannot validate a new key with a recycled numeric ID.

Production u32 exhaustion requires more than 2^32 simultaneously live records
(well over 100 GB). Treat it as allocation failure: the public interning path
uses Rust's allocation-error abort path; internal try_intern may report an
error for test-only small domains. No JS RangeError or new fallibility is added.

Own-slot probes stay pure. String indices are exotic virtual properties and
must not intern keys on reads. Test reuse for both key kinds, lease retention
by shapes/dictionaries/ICs, rejection by expired layout guards after reuse,
and simultaneous-live exhaustion in a test-capped domain.

The canonical contract is [objects](../../docs/design/core-rebuild-objects.md).
This is the approved S1 design correction resolving the previous error-boundary
blocker; no legacy store fallback or correctness waiver is introduced.
