# S2 type-specialized owning slot loads

Freeze before implementation. Public 86-case triage
`20261030T210813Z-t077-s2-r2-public86-triage`, `cdda4bd4` versus `dcee3c56`,
has development .970817 wall, external 1.036402 and equal-case selection
1.010599; automatic verdict likely loss. Native-method-heavy losses include
string_slice 1.071114 instructions, 3d-morph 1.0909, access-nbody 1.29 and
access-nsieve 1.1621. The ordinary slot API currently invokes general derived
Value cloning after every successful layout/version guard.

Inline the owning slot load and copy live Number/Function variants directly;
other types retain the same Value clone. This admits by the current value type,
not by source, name, opcode template or benchmark. No cached type/value, no new
entry/receiver pin, key/hash/descriptor shortcut or unsafe. The same guard and
bounds proof apply; changing value types must remain observable immediately.

Focused live-value/cache tests, strict gates, a fresh same-case instruction
cohort and public triage decide whether this reduces work. Preserve this failed
cohort and all final-gate evidence at its exact source revision. No held-out run.
