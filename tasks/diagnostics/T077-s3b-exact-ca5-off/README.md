# Repaired S3b IC-off exact result

Complete unfiltered `--exact --all`, **QJS_DISABLE_IC=1**, exit 0:
**42,672/42,672 configured Rust passes**, zero failure, timeout, not-run or NG
gap. Total 53,572, unchanged 10,900 configuration skips. NG: 42,602 passes,
70 failures, zero timeouts; all NG nonpasses pass in Rust. The complete summary
equals [cached](../T077-s3b-exact-ca5-cached/summary.json) exactly. No lost baseline
passes or new exclusions. [Summary](summary.json), [metadata](metadata.json).

Runtime `1e91be92`, post-order candidate `ca5ab643`, baseline main `3cc12c6f`.
Both scans use the same frozen agents release ARM binary, SHA-256
`fb492869e525b8def3f02b023853481b2faa2f9a1b520c8890dc3d8f9a0ce202`.
Only the IC-disable flag changes. The x86 order is not an ARM build input;
runtime/harness/configuration are identical at both named source revisions.

The complete IC-off burndown is appended for `ca5ab643`, source
`local-exact-s3b-repaired-ic-off-agents`; cached is recorded at `1e91be92`.
Earlier rejected records remain unchanged. Both scan processes finish normally
and the harness removes its generated submodule case files. This establishes
complete configured correctness, not S3b performance acceptance. Public86
hostname qualification remains unresolved; see [owner handoff](../T077-s3b-owner-handoff.md).
