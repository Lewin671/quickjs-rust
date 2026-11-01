# T077 S3c v2 progress

Base main: `e60e7cfa`; measurement base: `8322fc98`. Owner branch:
`agent/t077-s3c-v2/codex`. The old worktree is untouched. The revised design
addendum is committed at `3fe3943c`; lead decision 2 is the authority.

## C1: statically selected general-frame ownership

Ordinary FrameState stays a value. A private VmFrame storage bound selects
InlineFrame or Box<FrameState> statically, without an ownership-tag branch per
opcode. Suspendable execution boxes before its prologue; SuspendedActivation
moves the same box into/out of the generator state. Resume attaches services,
resets typed-loop declines and wide handback, and retains semantic handlers,
cells, class regions, disposal and pending completions in place. The old
GeneratorSnapshot field decomposition/reconstruction and operand replacement
helper are deleted. Root/eval public borrows retain the original compilation
owner rather than copying Bytecode. TLA now retains the module's existing Rc.

New tests prove stable box identity across prologue and two yields, segment
reset without loss of a pending completion, and repeated root/eval entry's
original code identity. `sticky-decline.js` and `eval-root-clone.js` NG fixtures
retain the attributed operation shapes. All previous tests remain.

At this C1 source: runtime **2,732** passes, combined binding/forced-exposure/
IC-verification/both-poisons **2,732** passes, Clippy all targets passes.
The fast gate passes with 65 focused Test262 cases. Full check passes (5,174
curated Test262, tooling and feature checks); x86_64 check passes. NG default,
IC-off and IC-verify each pass **322/322**, using a 300-second correctness
fixture budget. C2's
contiguous common header/lazy Context and storage-protocol deletions are next;
no performance target is claimed yet.
