# T056 hot/cold preparation

[Plan](implementation-plan.json) fixes architecture before runtime edits.
`probe.py <exact-HEAD>` builds canonical base/parent/candidate, counts three warmed
whole-process instruction samples for empty/CLI/unpack/class-fields, then uses the
existing static numeric-path decoder to compare normalized bytes and relative PCs.
`native.py` follows in the same no-timing job, captures native allocation histories
for empty/unpack on those images, and maps every requested size through the actual
host's `malloc_good_size`. Raw histories and full stack groups remain in the queue;
the owning task records conclusions, revisions, numbers and hashes.
Neither is timing evidence, a PMU-helper call, or a frozen acceptance attempt.
