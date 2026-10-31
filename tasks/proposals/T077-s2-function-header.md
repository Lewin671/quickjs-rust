# S2 function header allocation class

Freeze before implementation at 061e7f28. The cdda4bd4 resource cohort
`20261030T205608Z-t077-s2-r2-array-x86-rss` retains 300,000 functions:
no-call RSS is 130,342,912 bytes versus main 110,481,408 (1.179772).
FunctionData is 320 bytes; adding Rc's two counters crosses the CLI allocator's
320-byte class. Main's checked header bound is 304. The class explanation is
an inference to test, not an allocation attribution established by RSS alone.

The 24-byte optional class-inner-name String is absent from ordinary closures.
Box that optional String (8-byte option) so FunctionData returns to at most
304 bytes. Only a function that actually carries a class inner name allocates
the additional box; the String contents and the existing Upvalue cell remain
unchanged. Keep the compiler's construction API accepting Option<String> and
expose a borrowed str accessor to consumers. This changes neither binding
lookup nor shared-cell authority, native entries, named storage or lazy keys.
No new dependency, unsafe, public API or source/bytecode recognizer.

Check focused class/name/arrow/eval tests, header assertions and strict clippy;
then measure canonical instructions, construction work and the original retained
function/array RSS scripts on x86_64. Repeat final stage gates for retained code.
Reject the allocation explanation if the measured request classes do not match.
