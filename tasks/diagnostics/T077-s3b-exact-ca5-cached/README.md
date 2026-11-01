# Repaired S3b cached exact result

Complete unfiltered `--exact --all` scan, exit 0: **42,672 / 42,672 configured
Rust passes**, zero failure, timeout, not-run or NG gap. Total upstream inventory
53,572, unchanged configuration skips 10,900 (10,621 feature / 279 fixture).
NG passes 42,602, fails 70, times out 0; all 70 NG nonpasses pass in Rust.
No lost baseline passes or new exclusions. [Summary](summary.json).

Runtime `1e91be92`, post-order candidate `ca5ab643`, baseline main `3cc12c6f`.
The candidate changes only x86 order/documentation; ARM runtime/harness/config
remain identical and the post-order ARM rebuild is byte-identical. Agents
release binary SHA-256:
`fb492869e525b8def3f02b023853481b2faa2f9a1b520c8890dc3d8f9a0ce202`.
[Metadata](metadata.json) retains commands, environment, source/pin and raw-file
hashes. Full case JSONL and log are generated artifacts; all conclusions and
counts are recorded here.

The complete burndown row is appended for runtime `1e91be92` with source
`local-exact-s3b-repaired-cached-agents`. The rejected `73c93d8f` row remains
unchanged. IC-off has started on the same frozen binary; it still requires a
complete independent result and burndown. Public86 hostname qualification is
still pending. This cached result alone does not establish S3b acceptance.
