# Captured case-health failure

`t072-attempt2-local-read.json` is the unmodified raw case packet from
`20261023T154044Z-t072-screen-attempt2`, exported by the existing non-timing job
`20261023T160114Z-t072-attempt2-failure-export` in `out/screen-evidence.tgz`.
SHA-256: `a80ea014415d92c87b29ef8184aca85ab6f08880aabe3275324fba45f6a00497`.
Source revisions, exact image hashes and historical standing live in
[the T072 record](../../../../../tasks/diagnostics/t072/attempt2.md).

The fixture retains all calibration/attempt samples, timestamps, argv and load
snapshots. The test checks its original incomplete state, then analyzes a copy
in memory to prove that prospective health reporting still fails on the same
signed increment. It grants no new historical acceptance standing.
