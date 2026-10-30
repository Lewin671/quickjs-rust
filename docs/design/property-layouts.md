# Property layouts (superseded proposal)

The 2026-10-29 lead/user decision replaces the T056 payload-encoding/layout
migration with [core rebuild objects](core-rebuild-objects.md) and
[T077 S1/S2](../../tasks/T077-core-rebuild.md).
One runtime key-ID authority, shape/dictionary descriptors and keyless slots
replace all four ordinary storage forms and every exotic named table.

Keep the lessons, not the old implementation shape: transition hits do not
copy/hash entire field vectors, builders share domains, payload indexes are
deleted, and cache retention is bounded. Revision-bound failure/ablation evidence
is in [T056 diagnosis](../../tasks/archive/T056-m1-stage2-diagnosis.md).
The old design is preserved in Git before this change; its 120-byte overlay
budget and text-probe migration are not the rebuild's physical layout contract.
