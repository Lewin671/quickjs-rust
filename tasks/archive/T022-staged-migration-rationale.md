# T022: why staged migrations exist

Rationale moved out of [T022](../T022-performance-priority-controller.md) on 2026-10-22 to keep that file within its budget; the rule itself is unchanged and lives in T022 and `docs/benchmarking.md`.

The one-attempt leaf rule is correct for a recognizer and fatal for an
architecture. Every structural attempt this campaign made was closed after a
single implementation: the contiguous direct-call frame stack (correct, 1,920
tests passing, regressed two targets), the realm object arena (1.016x/1.088x),
transition-shape object storage, the compact generic bytecode core, and
default data-property storage (two of three targets improved 3%, the third
regressed 2.2%). Each rejection recorded "do not retry this mechanism", so the
only changes able to close a 3-6x generic-path gap were made unavailable one
at a time.

A migration's early stages move real execution onto a new representation
before any of it is faster. Requiring each to pay for itself is requiring a
bridge to carry traffic after the first pier. The stage gate keeps every final
standard — complete broad and external evidence, zero Test262 gap, a real
improvement — and changes only how the work is allowed to reach that gate. An
`abort` closes one implementation shape; it does not close "frames", "shapes",
or "arenas".
