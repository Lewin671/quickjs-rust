# C5 native order qualification

Runtime source remains **2aa117b7**. Layout roles at **e71c82a4** add the shared
user-call entries and FrameState owner-drop function. Both native orders use
current live raw symbols and pass strict audit after one relink. Executor
offsets stay **0x400 / 0xc40**; no offset scan or case-based layout selection.

## x86 fresh public profile

Job **20261102T071157Z-t077-s3c2-order**, source **e71c82a4**, idle queue and
submission outside the blackout. The runner used the canonical agents/fat-LTO/
16-CGU recipe and public external/sentinel/broad cases only. The resulting
order has **536 lines**, binary SHA **4779454e0943db30519e52e50ca30cc178b5136d2b7ec9f3cee5cae4ee7df90c**.
The job completed in **189 s**; light activity in 11 samples was below the
runner's disturbance threshold. [Summary](x86_64/summary.json), original command,
log/status, request, order and strict audit retain the evidence unchanged.

## ARM symbol refresh

Fresh local sampling did not qualify: macOS sample stalled on an exited child;
after the profiler was changed to reap children and bound sampler waits, the
attempt completed with no usable profile symbols. Both failure logs are kept.
They are not a successful profile or timing claim.

ARM therefore retains its previous ranking, rebound only to unique current
live definitions. Missing/ambiguous old tail entries are recorded in
[symbol-refresh](arm64/symbol-refresh.json); new semantic roles get fresh
budgeted slots. The resulting **304-line** order passes strict audit, binary
SHA **0a46f40e87186f20cb575e501c0197d3cee62470274a0d4f40ba0e006be84881**.
[Summary](arm64/summary.json), request, order and audit identify that narrower
result. The required fresh x86 profile above is complete.

The [collector](regenerate.py) uses the canonical recipe and asserts that every
runtime source byte is unchanged across generation. Run it through the iMac
runner on x86; locally use `PYTHONPATH=.` and `--symbols-only` for the documented
ARM refresh. Separate target/output directories keep verification/instrumented
binaries out of profiling. Final correctness, complete exact cached+IC-off
scans with burndown and the frozen final public86 remain to be recorded.
