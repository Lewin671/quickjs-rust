# C5 free-storage retirement

The commit containing this receipt pins runtime/binaries in [source](source.json).
Parent is **cf8cb8f0**. This implements the [frozen retirement plan](../../proposals/T077-s3c-pool-retirement.md).
Measurements precede final order regeneration; they are an intermediate gate.

Bytecode's three per-body Rc pool allocations, fields, constructors, accessors,
OperandStackRecycler, LocalSlotRecycler, ColdFramePool and ir_pools are deleted.
RuntimeServices' StackPool and its data-only GeneralPool share one byte/count
authority across register/header, operand, local and cold storage. FrameState
Drop returns cleared private locals/cold state through OperandStack's handle;
manual body-specific return protocols are gone. No frame field, code owner,
directory lookup or per-op pool access is added.

The existing 256 KiB/64-record/16 KiB bounds now cover all families, including
conservatively charged directories and retained cold-vector capacities. Free
storage drains at service teardown even when a suspended frame retains the
handle. Active/suspended/native-borrowed storage remains outside the free pool.
Resource drops precede bin borrows, including nested teardown of another frame
sharing that handle. Cells, mapped arguments and scope owners never recycle.
The [current design](../../../docs/design/frame-storage-pool.md) owns the invariant.

Two old recycler tests move their assertions to the common owner; nine storage
tests now cover reuse across body/length, poison corruption, shared byte/count
bounds, cold inner-capacity rejection, service teardown and nested frame drop.
The native-argument borrow test uses the shared owner. The escaped-cell test
alternates forced exposure on one runtime to exercise actual cross-entry reuse.

[Checks](checks.json): full gate/5,174 curated Test262, default runtime **2,753**,
Clippy pass. [Stress](stress.json): seven modes **2,753**, combined-off **2,711**
with the existing 42 IC-control skips. [NG](compare.json): **322 + 10 modules**
in all nine modes. [Layouts](layouts.json) agree on arm64/x86: header 64,
FrameStorage 56, general FrameState 480, ColdFrame 288, suspension record 16,
GeneralPool 120 bytes. These are type sizes, not native stack frames or RSS.

[N/2N calls](call-cost.json) save **112.90 / 196.21 / 422.28** instructions per
upw0/1/4 call versus S3b, preserving compact-call counters. [Controls](controls.json):
hash-map **0.992056**, method_call **0.992867**, tofte **0.918378**, supplied
method repro **0.966107** meet <=1. Fannkuch **1.000115** remains visible above
one; final x86 recheck is still required. No instruction excess is rounded away.

[Depth](depth.json) meets S3b for all eight probes in debug and release without
CLI policy changes. Debug reduce **246 vs 236**, call **670 vs 437**,
yield-star **227 vs 152**; release yield-star **1,622 vs 820**. Plain recursion
and release call reach the 4,096 probe cap.

Next: regenerate order files, complete the final full/stress/NG/public/fuzz/
exact cached+IC-off matrix with burndown, freeze runtime and run only
`t077-s3c2-final`. This is not final S3c acceptance; the lead owns formal,
held-out, RSS and independent review. No dependency, unsafe or public API added.
