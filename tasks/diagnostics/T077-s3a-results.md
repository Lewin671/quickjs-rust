# T077 S3a implementation and review repair

- Status: independent-review repairs present; final full gate/counters/exact scan pending.
- Implementation: direct operation entries and borrowed arguments retained; no native/realm depth or stack limit. The rejected original report is [historical](../archive/T077-s3a-initial-results.md).
- Verified at: guard removal `43a0a5be`; hot-path worktree after it: 2,391 runtime tests and clippy pass, debug/release depth diagnostics below. Final revision will be recorded after commit.
- Evidence: [depth records](T077-s3a-depth-parity.json), [depth reproducer](T077-s3a-depth-probe.py); older entry/copy counters remain bound to `27d503a6` until the fresh run below.
- Unresolved: final full gate, comparison fixtures, deterministic counters and complete configured exact scan/burndown; lead formal performance acceptance.
- Next action: commit the reviewed call routing, run final gates on that revision, then hand off without push/merge.

## What the review corrected

The earlier native limit was an unjustified import from the broader S3 inventory,
not a response to a demonstrated S3a-only crash. The code added a limit before
comparing against base. The later 300-level reduce stress **already aborts in base
in debug**, while both base and repaired release complete it with result 0.
The depth-only guard, debug-specific variant, 1 MiB realm anchor and tests that
required those errors are deleted. No substitute limit is introduced.

Deep Rust recursion remains a known base gap. Base and repaired CLI both abort
for a forEach walk at 1,000 levels. S3d owns a runtime-wide anchored stack-pointer
guard with host-configured budget, a CLI engine thread with explicit stack size,
and frame-counted JS depth after pure-JS calls no longer recurse in Rust. This
S3a change does not claim protection from stack overflow.

## Hot-path repair and scope

- CallInfo borrows callee/receiver and records `is_construct`. CallEnv is the
  single service/context authority. There is no per-call realm Rc clone/drop.
- No entry used the old new_target value, only its presence. The construct flag
  replaces that eager lookup/clone; semantic constructors still fetch actual
  new.target from CallEnv at the point their existing algorithms need it.
- The proven NativeEntry pointer is passed to the thin inline call adapter;
  it never reloads/unwraps native_entry with an expect.
- Common dispatch borrows callee; detached VM operand slots transfer callee and
  receiver by ownership. call_function_slice no longer clones a native callee
  again to enter the dispatcher.
- The base callback direct-leaf shortcut is restored before general dispatch.
  In debug, the rejected slice wrapper entered the 3,936-byte generic dispatch
  frame even for admitted JS callbacks. Base's shortcut used a 224-byte slice
  frame; the repaired shortcut uses 192 bytes and bypasses that generic frame.
  These are disassembly stack reservations, not timing measurements.
- No stack_native_callee probe remains. Existing fixed-arity JS fast paths keep
  their original order; generic call/direct-eval/resolved-call fallthroughs and
  constructors borrow their owning operand block independent of callee kind.
  Therefore no new native-entry header load is inserted into JS->JS admission.
  This also removes some ordinary JS Vec transport earlier than S3d, without an
  activation, binding or opcode rewrite. Remaining semantic/virtual/super lists
  will be counted separately; no borrowed list escapes its caller owner.
- The recycler repair is retained: restore the actual owner directly, without
  recycling the empty detachment placeholder. Constructor new.target restoration
  on prototype-getter throw is retained. No S1 property/layout path changed.

CallInfo's construct-mode shape is the lead-approved correction to the earlier
borrowed-new.target/realm proposal. S3b changes the CallEnv service to Context;
actual new.target/realm remain lazy service capabilities, not an eager adapter.

## Base/head depth parity

Default-feature CLI, aarch64 macOS, same toolchain/recipes, ordinary main-thread
stack (no stack override). Base Rust sources were temporarily restored from
`d19d3aa6` **only in this worktree**, verified byte-for-byte by git diff, built,
then restored before edits. A second pair of binaries removes only the guard and
unused realm field; a third pair includes all hot-path fixes. Core dumps were
disabled in the external probe runner. No elapsed-time/cycle measurement.

Values are largest successful depths found by exponential/binary probing on this
host; they are not portable limits or promises. All base-success maxima and
explicit feedback landmarks also succeed on repaired head. `>=4096` is the
probe cap, not an engine-added limit.

| Case | Base debug | Repaired debug | Base release | Repaired release |
| --- | ---: | ---: | ---: | ---: |
| forEach tree walk | 143 | 173 | 712 | 802 |
| map tree walk | 142 | 172 | 706 | 796 |
| reduce tree walk | 176 | 225 | 744 | 828 |
| f.call recursion | 341 | 433 | 2,034 | 3,337 |
| f.apply recursion | 338 | 353 | 1,879 | 2,108 |
| Reflect.apply recursion | 309 | 354 | 1,674 | 2,234 |
| generator yield* chain | 142 | 146 | 665 | 712 |
| plain recursion inside callback | >=4096 | >=4096 | >=4096 | >=4096 |

The guard-removed intermediate also improves seven routes, but release reduce
was 733 versus base 744. Restoring the callback shortcut and reducing metadata/
callee ownership fixes that residual frame-growth regression (final 828).
The original 300-level reduce stress aborts in **both** debug binaries and passes
in **both** release binaries: it was not evidence for a new S3a limit.

```sh
python3 tasks/diagnostics/T077-s3a-depth-probe.py \
  --root target/t077-s3a-review/base-build --label base-d19d3aa6 \
  --output target/t077-s3a-review/base-depth.json
python3 tasks/diagnostics/T077-s3a-depth-probe.py \
  --root target/t077-s3a-review/fixed-build --label repaired \
  --output target/t077-s3a-review/repaired-depth.json
```

## Final gates and counters

Pending on the reviewed source commit. Original full coverage and counters in
[the historical report](../archive/T077-s3a-initial-results.md) establish only that
old revision; they do not establish current HEAD or adequate recursion behavior.
No performance acceptance, push, merge or held-out access is claimed.
