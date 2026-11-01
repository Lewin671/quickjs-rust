# S3b x86 order for the final requalification

Job **20261101T122522Z-t077-s3b-final2-order**, source **680eb9dc** (runtime
**f7057a51**), baseline **3cc12c6f**. The workspace iMac queue was idle before
submission, outside the CST blackout. Exit **0** after 129 seconds. The
[command](command.sh) guards blackout entry and the next cutoff; [status](status)
and [log](log) retain the runner result.

Default `python3 -m tools.benchmark.order_file` profiles exactly the default
**76 external/sentinel/broad cases** ([cases](cases.json)), yielding **474
symbols**. No development or held-out profiling, offset search or policy change.
[Request](order-request.json) and [result](order-result.json) pin producer,
binary (`bbe4f7bc…`) and order hashes. Order SHA-256:
`d95d17b14495973ca123b13ae398260cf43c30b7096c0e2a2e1c2305b2a9da11`.
It replaces `crates/qjs-cli/hot-functions-x86_64.order`, which the CLI build
script consumes only on x86_64; it supersedes the d6633cd1 order, which predated
f7057a51. Profiling/layout evidence, not a performance acceptance result.
