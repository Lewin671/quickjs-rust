# S2 round-2 construction checkpoint verification

Runtime source `90719950`, pinned agents release CLI SHA-256
`d0c2166f70aa6e5d7d8f0337184c5f6bde636a3cdb129326c83c707398545dc4`.
This checkpoint predates persistent pressure, batched fields and string packing;
it is not a claim about later HEAD or final array behavior.

```sh
QJS_CLI_BIN=<pinned-90719950-cli> QJS_AGENTS=1 QJS_CLI_PROFILE=release \
 TEST262_CASE_TIMEOUT_SECONDS=20 TEST262_BASELINE_JOBS=4 \
 ./scripts/find-qjsng-gaps.sh --exact --all --out-dir <report>
./scripts/test262-burndown.sh --report <report> --commit 90719950 \
 --source local-exact-release-agents-s2-r2-construction-on
```

Complete scan exits 0: 53,572 records, 42,672 configured/executed passes,
10,900 unchanged skips (10,621 feature, 279 fixture), zero Rust failures,
timeouts, not-run configured cases or actionable gaps. NG passes 42,602;
its 70 configured failures all pass on Rust. Case JSONL SHA-256
`2263183d8df546cb37338205c107cc4614a7ddee069a711be6181c0e27c2f61a`
is byte-identical to the round-1 complete cached/off scans. No lost passes
or new exclusions. The completed scan has an appended immutable ledger row.

Other construction-stage checks: default full check reaches curated 5,170;
`90719950` IC verification with agents/perf-counters passes 2,504 tests.
`68b539bc` gate passes 2,460 runtime/117 focused Test262; its verified mode
passes 2,507. `74ba5e56` string metadata tests 6, RegExp 131, strict feature
clippy and commit gate pass; production string header assertion is 72 bytes.
Final post-repair full/cached/off/verify, cross-target, fuzz and resource work
remain required. Performance is diagnostic and still being repaired.
