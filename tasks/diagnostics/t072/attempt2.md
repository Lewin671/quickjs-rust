# T072: Lead-authorized second screen

The lead authorizes the remaining attempt on 2026-10-23, after reviewing
attempt 1's slow N process. The [task](../../archive/T072-live-property-proof.md)
records the decision and infrastructure finding before submission, in
`6226323c`. The [first measurement record](measurements.md) retains the
charged incomplete attempt, triage, correctness and exact-binary identities.
No partial screen samples are reused.

## Frozen inputs and launch

- Candidate: `6c122407146614e5d480b642551d18e782c430f5`.
- Base: `6e899d24c167d1393c077bcd69a01b61571dbcd1`.
- Plan SHA-256: `476cb44858ba0ca74de9ba94a3b200676fb1d4c069cc865c9f47c6a581d381b4`.
- Policy SHA-256: `be123f7c5cfd46aa863467256eb8bf6947ff64f72541efc371f05a36c5535a31`.
- Queue checked idle; iMac clock `Fri Oct 23 23:39:40 CST 2026`.
  Expected screen duration 55 minutes clears the 02:45–05:00 quiet window.
- Job `20261023T154044Z-t072-screen-attempt2`, started 15:40:46 UTC.
  Exact candidate and plan hash are asserted before collection.
- Command: `./scripts/perf-loop.sh --plan tasks/performance-units/live-named-property-proof.json --output "$BENCH_OUT/attempt2" --workers 4`.

Attempt 2 is a fresh full screen with new output and the frozen workers-4
schedule. Candidate, targets, policy, thresholds and runtime code are unchanged.
If this screen completes, collect formal 30, held-out 30 K=6, RSS 15 and
T067-derived resources for the lead; any second screen failure closes the unit.
No final decide, promotion, origin push or merge is authorized.

## Result and closure

Job `20261023T154044Z-t072-screen-attempt2` exits 1 after 1,137 seconds.
All twelve images build and pass static audits; their revisions match the
frozen base/candidate. Natural image hashes reproduce the triage and first
screen: base `cbb471c7047917f02e13d0ecf83516d9d6e57c050f2170ddde3b5e6e17ed735e`,
candidate `f6d5220992f3dd7d05afaad4af2e169a01258294e7995347d28f81e4ca74a5e3`.

The log has 68 natural-case collection starts. Shard 3 fails again at
`broad/local_read`, with `invalid screen increment`. The raw packet has
`complete: false`; all five attempt indexes are load-valid, with the frozen
`case-shards-v1` workers-4 schedule. Four light intruder samples remain below
the disturbance threshold; no DISTURBED warning occurs. No `panel.json`,
`complete.json` or `verdict.json` is produced. No subset geomean is reported.

N remains 23,068,672; 2N is 46,137,344. Candidate raw pairs are:

| Pair | N wall ns | 2N wall ns | Increment ns | Instruction increment |
| --- | ---: | ---: | ---: | ---: |
| 0 | 572438206 | 522485450 | -49952756 | 4544284580 |
| 1 | 264221519 | 521521532 | 257300013 | 4547366707 |
| 2 | 263591603 | 524706667 | 261115064 | 4547564893 |
| 3 | 263534475 | 611722219 | 348187744 | 4548347669 |
| 4 | 265044113 | 519541033 | 254496920 | 4547282958 |

Pair 0 N uses 560 ms user CPU and 4,575,767,338 instructions, versus 510 ms
and 9,120,051,918 instructions for 2N. The other candidate N processes use
250 ms user CPU. Pair 3 also has a slower 2N process (600 ms user CPU).
All five base increments are 248.765–256.562 ms. These observations do not
identify the cause or attribute it to the optimization, layout or OS activity.

The fixed rule fails valid short/nonpositive increments instead of replacing
them. No pair is dropped, substituted, relabeled or recollected. This fresh
screen spends the remaining attempt; both attempts stay charged. Under the
lead's explicit second-failure stop condition, **T072 closes without promotion**.
No formal 30, held-out 30 K=6, RSS 15 or resource run is submitted. Their
standing is unavailable; moment wall 1.076441 and binary-trees instructions
1.0447 from triage remain reported diagnostic losses. No performance rejection
or promotion conclusion is inferred from incomplete screens. The lead routes
the repeated whole-screen invalidation finding to T017/T038; no measurement
infrastructure or runtime change is made in response here.

## Export and retained identities

Non-timing job `20261023T160114Z-t072-attempt2-failure-export` exits 0 in 0 s after
waiting behind T071. It retains all available screen JSON/JSONL, both matrix
identities, original command/status/log and intruder log in
`out/screen-evidence.tgz`. Local extraction validates all 143 manifest hashes.
Raw data is retained in that iMac queue job; the table and hashes here remain
reviewable after local build/scratch cleanup.
The 167,930-byte archive SHA-256 is
`5905f5f27356b27a714ded002beca6690be20cbb574533f986b8d80cbbfc2b0c`.

| Asset | SHA-256 |
| --- | --- |
| Screen job log | `160576b74a521a0623ed364a31993393db8c5b8f0cbaf4d6a4f18e9d9764ef15` |
| Intruder log | `778e68053e1ff36593c74f2b431f7b7a13c430b9a11307a7c2d1d0423c5d455b` |
| Failed packet JSON | `a80ea014415d92c87b29ef8184aca85ab6f08880aabe3275324fba45f6a00497` |
| Failed packet JSONL | `f7d633dcfafc95c88109f21752380647f1361ce2f5660d6a4163bb34888d5201` |
| Base matrix identity | `585cfe4ed2eb97c2be2f5e3564be515b9bc41cabd01326fa26067aaca10a4d89` |
| Candidate matrix identity | `99687a16d1050dcfff8047d24a24e1c7317b7217f0b98671c240af91c3eab2de` |
| Shard error | `54194e60957ef4296d59ddc31dbe15f20092651c86ad0bbec055bce6215de039` |
| Export hash manifest | `e1f056defca55a64095926b3c690baa096476971858b15f6a0105a69406ee42f` |

## Local verification

At documentation revision `6226323c`, `git diff 6c122407 -- crates tools
scripts benchmarks` is empty. `TEST262_DIR=<shared pinned corpus>
./scripts/check.sh` passes: feature/default clippy and tests, 2,395 runtime
tests, 732 benchmark-tool tests, 35 script tests, context/size/hooks and all
5,170 curated Test262 cases. Log SHA-256:
`db9a764e71b673341f8596de56188abf6bb1aa9b670119f76f2761d55c5d6fcb`.
The earlier complete exact scan at candidate `6c122407` remains 42,672/42,672
configured passes with zero gaps; no new exact scan record is appended.

After preparing closure and archive/index updates, the same full command
passes again: all Rust/tool/script checks and 5,170 curated Test262 cases.
Closure full-gate log SHA-256:
`d8bdd25fe4dab29c2d2a33ba587af880a93fd069f178f43bb2abdd4bb6a17c68`.
Only documentation evidence changes afterward; its staged fast gate is
required before commit. Frozen plan/policy/runtime remain unchanged.

Original shared Test262 and NG tracked trees remain clean. Branch scope
validation passes against `6e899d24`; the archive move is documentation only.
Cleanup runs `cargo clean` (5,678 files / 3.3 GiB removed), then
`git checkout -- target` restores the tracked rejected-screen fixture; free
disk is 48 GiB. The staged documentation gate follows cleanup, without
rebuilding deleted binaries. Both new T072 jobs finish; no origin push or
merge occurs. The task and plan indexes record closure, retaining the source
for lead review and the infrastructure finding for T017/T038.
