# S3b merged-source x86 order

Runner job `20261031T232541Z-t077-s3b-merged-order` completed with exit 0.
Source `4f6c0444` includes accepted main `3cc12c6f`. The default
`python3 -m tools.benchmark.order_file` selection profiles all 76 external,
sentinel and broad cases; development and held-out inputs are excluded.
The producer kept default coverage, sampling window and pin offset.

The canonical x86 release input binary SHA-256 is
`7d473a28bc19a00a4c4b3f8063328bcc97193a44a6a92873ebb2eb61abe57df2`.
The resulting 485-symbol order SHA-256 is
`d5f127ba4b711c302bc890fe3bab221fd111f2775d8b0fe565711d175196b88c`.
It replaces `crates/qjs-cli/hot-functions-x86_64.order` before final triage;
this profiling binary is not the final relinked timing binary.

[Request](request.json), [result](result.json), [cases](cases.json) and the
[exact command](command.sh) retain provenance. The command checks China
Standard Time and stops its own profiling group before the blackout. There
was no formal performance, held-out or RSS acceptance in this job.
