# S2 packed array word orientation: implementation decision

The preceding [diagnosis](T077-s2-array-word.md) is now measured. Public extras
job `20261030T231113Z-t077-s2-r2-public-regression-counts` has nsieve instructions
336,775,988 /307,860,716 (1.09395), with **zero named-IC reads** in both roles.
The b2cd575c seven-case instruction cohort confirms 1.09402 after String repair.
Canonical x86 image SHA e1bf9c646a21266595b818973039efbe53445647563ce2bf79122e673fc9015c
contains movabs of 0x0fffffffffffffff in array hot reads and dense eligibility.
[Selected otool snippets](../diagnostics/T077-s2-r2-array-mask-before.json) bind
that finding to the actual canonical image. No timed/layout-selected build.

Implement the same 60-bit length shifted left four, with integrity bits in the
low nibble. Getter is a right shift; flag operations use small immediates. Host
capacity/overflow assertions and 56-byte hot header are unchanged. This targets
array representation work, not a source or opcode template. Do not add a u32
length restriction, allocation, unsafe, dependency or scope/binding API change.

Verify zero/word/u32/large-host/max lengths with every flag combination and
length updates, plus array suites and strict feature clippy. Compare the same
seven public instruction inputs and canonical getter assembly; required final
gates and public86 wall comparison decide retention. No held-out run.
