# S3b x86 order after the global-store fix

Job **20261101T140029Z-t077-s3b-final3-order**, source **a84252bd** (runtime
**0148b2eb**, the global-store repair), baseline **3cc12c6f**. The workspace iMac
queue was idle before submission, outside the CST blackout. Exit **0** after 128
seconds. The [command](command.sh) guards blackout entry and the next cutoff;
[status](status) and [log](log) retain the runner result.

Default `python3 -m tools.benchmark.order_file` profiles exactly the default
**76 external/sentinel/broad cases** ([cases](cases.json)), yielding **468
symbols**. No development or held-out profiling, offset search or policy change.
[Request](order-request.json) and [result](order-result.json) pin producer,
binary (`1fbeaaa9…`) and order hashes. Order SHA-256:
`981a0ee6df5769dc92fd90076ac3ee66a898f76b9f3351e00aec6108ffd06e53`.
It replaces `crates/qjs-cli/hot-functions-x86_64.order`, which the CLI build
script consumes only on x86_64; it supersedes the 689efca2 order, which predated
the runtime change. Profiling/layout evidence, not a performance acceptance result.
