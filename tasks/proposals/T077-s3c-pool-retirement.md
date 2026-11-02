# S3c C5: finish the free-storage retirement

Design frozen before its runtime implementation, after C4. This closes the
per-body recycler row of the [approved inventory](T077-s3c-inventory.md), not a
new execution backend. Revised ABI general stack-IR banks remain semantic frame
state until S3e; free allocation ownership is the remaining S3c deletion.

## Present sites and deletion

Bytecode owns OperandStackRecycler, LocalSlotRecycler and ColdFramePool;
ir_pools exposes them to frame construction, completion, wide resume and the
dormant frame-entry test. Their three per-body Rc allocations and all fields,
constructors/accessors/types must go. Tests move to the storage owner. No name,
callee or body matching is allowed for reuse.

## Sole budget and owners

RuntimeServices' existing StackPool keeps the register-stack directory and a
shared GeneralPool handle. That handle owns typed free operand/local/cold bins
and one accounting authority used by all four storage families. An active
general frame retains the handle through OperandStack, enabling lazy cold
storage and return after suspension without retaining RuntimeServices or code.
No new frame field or per-op pool lookup; ordinary inline push/pop is unchanged.
Only entry, lazy cold allocation and completed-storage return touch the pool.

Keep the existing total **256 KiB**, **64 free records**, **16 KiB per record**
bounds; charge directory capacity conservatively as well as payload. General
sub-bounds remain operand/local vectors <=256 elements, <=32 of either kind,
and <=8 cold boxes. Cold charge includes every retained inner Vec capacity
after clearing owned contents. No bytes under active/native-borrowed/suspended
owners count as free, and none can be evicted. Runtime teardown drains free
storage even when a suspended owner retains the small pool handle.

Reserve/release accounting uses private Cell counters; typed general bins use
short RefCell borrows. Clear values, cells, handlers, references and owned
resources before acquiring a free-bin borrow. No borrow spans JS work or drop
of an owned JS value. Storage take/return adjusts the one budget exactly once.
Root register runs retain direct StackPool access without a new RefCell borrow.

Local vectors may cross bodies and lengths: every taken slot is None before
the current BindingPlan initializes it. Poison stays recognizable scalar
storage until take checks and clears it. Cold reset drops all semantic owners;
only empty capacities recycle. Context, cells, scope leases, mapped arguments,
code and pending completions are never pooled identities or copied snapshots.

## Verification before final freeze

Test cross-body/length/poison reuse, escaped cell/arguments separation, cold
reset and oversized rejection, combined-family byte/count bounds, and pool
teardown with a retained suspended handle. Preserve all earlier tests by
moving their assertions to the new owner. No unsafe, dependency or source-input
policy change. Run full/default/agents/perf-counters, stress+poison, NG nine
modes, public351, four fuzz profiles >=2,000 each, complete exact Test262 cached
and IC-off with burndown, both target layouts/depth, and deterministic call and
named control checks. Regenerate x86 order before the final source/binary seal.
Then run only the authorized final public86 and stop for lead acceptance.
