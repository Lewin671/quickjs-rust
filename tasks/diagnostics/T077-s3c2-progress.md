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

## C2: contiguous indexed frames and the revised checkpoint

C1 is committed at **a8ae898c**. C2 replaces both tier-specific/TLS owners with
one RuntimeServices pool of cleared contiguous value/header stacks. The header
is 64 bytes on both production targets; callee/used call inputs live in its
prefix, and returns clear compiler-declared written ranges and used prefix
slots. Argument borrows pin the detached owning stack across native callbacks.
Lazy Context borrows creation/receiver facts; no per-inline-call boxing, code
copy, directory or activation-rebind protocol remains. Seven stress modes,
combined-off, all-mode NG, public sweep, full/counter checks and fuzz pass at the
frozen source; [the checkpoint receipt](T077-s3c2-checkpoint/README.md) owns numbers.

The revised checkpoint **misses**: upw0 saves 71.1 instructions/call, upw1 66.0,
upw4 33.0 (each required >=70). Hash-map instructions are 1.0220; the supplied
method reproducer and tofte improve, but strict fannkuch/broad-method controls
also miss. Counters prove the same tier/semantic reach and only one Context per
process; the 45+3a+3w lifecycle estimate is not isolated from the remaining dispatch
cost, and its expected net payoff is not demonstrated. Debug depth falls in six probes while release depth improves. Native
debug disassembly shows wide driver's frame 15,408→19,952 bytes, numeric
5,504→7,664, explaining the new native-stack pressure. Runtime work stopped at
these misses. C2 is committed at **cd020e8e**. The single iMac job
**20261101T235231Z-t077-s3c2-c2** is complete: 430/430 valid, zero invalid/heavy,
likely loss. Development instructions 0.987267 pass, external 1.000115 miss.
All public wall geomeans improve; none waive the local/depth/instruction gates.
The receipt owns full lane ratios and originals. No runtime changes or C3–C5
followed the misses; stop for lead review.

## Round 3: depth prerequisite

Lead decision 3 conditionally accepts C2 and authorizes C3–C5, carrying every
miss forward. The first repair outlines debug operand/entry temporaries and
borrows native-site metadata. All eight debug/release depth probes now meet
S3b, without a stack-policy change. Wide debug frame is 8,816 bytes (C2 19,952),
numeric 4,016 (7,664), wide release 1,104 (1,264). Release per-call instruction
cost is unchanged within count noise. The [repair receipt](T077-s3c3-depth/README.md)
pins runtime source, binaries, depth traces and prologues. C3 captured writes,
event boundaries/method attribution, C4 and C5 remain required.

## C3: static captured writes (partial)

The first C3 cut links compiler-proved write capabilities and compiled capture
indices. All three deterministic calls now save >=70 versus S3b; the paired
read/write improvement is about 88 instructions per added += operation versus
C2 after removing the zero-write improvement. [The receipt](T077-s3c3-bindings/README.md)
owns source/binary identities, counts, policy exclusions and method coverage.
Hash-map 1.003722 and broad/method_call 1.002384 remain open. Explicit boundary
Ops/indexed cell routes and all remaining C3–C5 gates are not complete.
