# Final S3b x86 order

Job **20261101T043145Z-t077-s3b-final-order**, source **7ce2ab30**, runtime
**e1405670**, baseline **3cc12c6f**. The workspace iMac queue was idle before
submission; remote date was checked outside the CST blackout. Exit **0** after
129 seconds. The [command](command.sh) guards blackout entry and the next cutoff;
[status](status) and [log](log) retain the actual runner result.

Default `python3 -m tools.benchmark.order_file` profiles exactly the default
**76 external/sentinel/broad cases**, yielding **527 symbols**. No development
or held-out profiling, offset search or policy change. [Cases](cases.json),
[request](order-request.json) and [result](order-result.json) pin producer/binary
and order hashes. Order SHA-256:
`85bff4f31111a279763a5e4299179f9ad62a6af718e51f18bbabae4fc05a6bd9`.
The regenerated order replaces `crates/qjs-cli/hot-functions-x86_64.order`.
Its CLI build script consumes it only on x86_64; ARM runtime/build inputs and
binaries remain unchanged. Public86 will rebuild the final x86 binary with it.
This is profiling/layout evidence, not a performance acceptance result.
