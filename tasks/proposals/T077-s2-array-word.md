# S2 packed array word orientation

Frozen diagnosis, not implementation. The 14abe3f3 public cohort still has
access-nsieve 1.093487 main instructions. Array header repair packs a full
60-bit host logical length below four high integrity bits. Its hot length
extract requires clearing four high bits, and stores preserve high flags.
Investigate the canonical x86 code/counts before selecting another change.

A representation-preserving alternative puts the same four flags in the low
bits and shifts the same 60-bit length left four. A length read becomes one
right shift; masks/flag updates use small immediates. Capacity, 56-byte header,
all valid host lengths and overflow checks stay identical. Do not narrow length
to u32 or add overflow storage, allocation, unsafe or JS behavior changes.
If the pending immutable-IC cohort still loses, measure its array/native cases
before deciding whether this encoding is a worthwhile next step. Any retained
change needs boundary/flag isolation tests, all array suites and final gates.
