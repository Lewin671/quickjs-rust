# S2 string key metadata packing

Frozen before implementation. Main `dcee3c56` stores a RegExp cache as
`Option<(bool, Rc<[char]>)>` (24 bytes plus RefCell guard). S2 adds an inline
8-byte key lease memo beside it: production StringData grows 72 to 80 bytes
(the round-1 receipt verifies 80 with arm64 DWARF). The `90719950` count cohort
still has string_slice instructions 1.08340 main, with about two million
allocator requests at one million iterations. No constructor work occurs in
the steady state; the overhead includes reads and string materialization.

Keep the inline key memo and matcher slice, but move the matcher Unicode-mode
boolean into existing scalar-header padding. `Option<Rc<[char]>>` is 16 bytes;
with the 8-byte key memo the cache returns to the old 24-byte payload. Target
production StringData 72 bytes on both macOS architectures, asserted in live
non-test compilation. No new pointer, allocation or mutable global state.

Preserve matcher mode validation (ASCII shares either view), COW cloning,
mutation invalidation, key-domain checks and find-only reads. Mode publication
and view replacement execute without JS callbacks. Add a focused mode/COW/key
coexistence proof; run string/RegExp suites and required later gates. A fresh
instruction/requested-byte cohort decides payoff; no source recognizer, unsafe,
dependency, public API/S3 binding change or held-out work.
