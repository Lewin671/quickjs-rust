# T077 S3c C2 checkpoint

Base: `8322fc983da20800ce612ad148781aea30e28d08`. Candidate: `63b2347fce60d615acd37b674941f1ecffba27b1` (C2), parent `52422927`. This checkpoint
implements C0–C2 under the [lead decision](../../proposals/T077-s3c-brief.md#lead-decision-2026-11-01).
It is not S3c acceptance. C3–C5, formal four-aggregate/held-out/RSS qualification
and complete exact Test262 have not run for this candidate.

## Source and receipts

- `count.py` replays the original, unchanged 109-program receipt: 86 public,
  21 reproducers and two fixed-clock diagnostics. Program/stdout hashes must
  match; counters run twice. The two original clock-sensitive programs retain
  their declared unstable-counter qualification. Only retired instructions and
  event counts are retained, never local timing/cycles. No held-out input is read.
- `call_cost.py` takes the median of three retired-instruction samples at N=1M
  and 2N, then divides their difference by N. Counter reach proves the same
  compact tier and N calls. `call-cost.json` binds all four binary hashes.
- `depth.json` contains ordinary CLI debug/release probes with no stack override.
  Cases: forEach/map/reduce/call/apply/Reflect.apply/yield/plain. Before:
  debug 183/182/236/437/366/367/152/4096; release
  935/927/1026/3448/2335/2491/820/4096. After: debug
  254/251/277/806/594/596/187/4096; release
  1043/1032/1210/3472/2346/2504/1227/4096. Plain is censored at 4096.
- `assembly.json` records final arm64 prologues and directory/memcpy call sites.
  The wide opcode segment has no directory call or memcpy; register indexing
  uses its current zero-based slice, not FrameId resolution on backedges.
  Numeric/wide driver native frames are still 976/1616 bytes including saved
  registers; wide opcode preparation has a separate 720-byte frame that returns
  before callbacks. No frame-sized switch memcpy remains. These costs are not
  claimed eliminated and must be re-priced in the next design.

## Instruction result

The 109-program before/after maps are in `counts.json`. Retired-instruction
geomeans C2/base: development 1.155770, external 1.103118, broad 1.051066,
sentinel 1.011626; public86 equal-case selection 1.087053. These are arm64
instruction counts, not iMac wall ratios or formal acceptance aggregates.

| Reproducer | Before instructions/call | C2 | Added |
| --- | ---: | ---: | ---: |
| upw0 | 1003.086 | 1941.346 | 938.260 |
| upw1 | 1333.380 | 2283.644 | 950.264 |
| upw4 | 2275.146 | 3243.278 | 968.132 |

For upw0 N=1M, both execute 1,000,002 ordinary attempts, 1,000,000 compact
direct calls, 9,000,009 compact ops and 30 general ops. C2 materializes one
Context and allocates 105 binding cells. Removing eager Context did not pay for
the common record/directory lifecycle. Initial draft ratios are retained in
`initial-instructions.json` as diagnostics only, not candidate evidence.

## Reproduction

Build default debug/release CLI and release `perf-counters` separately. Run:

```sh
python3 tasks/diagnostics/T077-s3c-c2/count.py --before BASE_RELEASE --after C2_RELEASE --before-counters BASE_COUNTER --after-counters C2_COUNTER --revision C2_SHA --output OUTPUT
python3 tasks/diagnostics/T077-s3c-c2/call_cost.py --before BASE_RELEASE --after C2_RELEASE --before-counters BASE_COUNTER --after-counters C2_COUNTER --output OUTPUT
python3 tasks/diagnostics/T077-s3c-c2/verify.py stress --unit C2_TEST_EXE --binary C2_DEBUG --output OUTPUT
python3 tasks/diagnostics/T077-s3c-c2/verify.py compare --binary C2_DEBUG --output OUTPUT
python3 -m tools.benchmark.public_correctness --binary C2_RELEASE --verify-binary C2_OPTIMIZED_VERIFY --output-dir OUTPUT
```

Build `C2_OPTIMIZED_VERIFY` with `cargo build --release -p qjs-cli --config
profile.release.debug-assertions=true --config profile.release.overflow-checks=true`
in its own target directory, per [the public sweep workflow](../../../docs/public-workload-correctness.md).
The counter/default binaries are distinct; poison/verification builds are never
timed. Fuzz request and summary bind 500 seeds per each of the four profiles:
2,000 programs, MATCH 1,791, pre-existing differences 209, stable inputs, no new
regression or mode disagreement. Reproduce fuzz with `python3 -m tools.binding_fuzz.run --binary C2_RELEASE
--verify-binary C2_DEBUG --base-binary BASE_RELEASE --ng NG --base-revision
8322fc98 --output-dir OUTPUT --count 500 --jobs 4`. The earlier full S3b qualification remains
historical evidence and is not attributed to C2.

## Checkpoint disposition

Final-source full gate passes (2,739 runtime tests, all tooling/agents checks,
5,174 curated Test262); staged fast gate passes with 117 focused cases. Full
perf-counters passes 2,790; x86_64 check passes. `checks.json` binds source/log
hashes. Seven stress/poison modes pass 2,739 each; combined IC-off passes 2,697 with
the unchanged 42 mechanism exclusions. The corrected public sweep passes 351/351; public86 triage is complete below. Stop
here for lead review; no C3–C5 runtime implementation is authorized by this
checkpoint. Recommend re-freezing combined S3c+S3d: the added roughly 940–970
instructions per call exceeds the original priced recovery envelope. Re-price
entry/parameter placement and record lifecycle before committing to another
standalone S3c payoff trial. Do not credit a future stage with this regression or
relax the all-four <1 gate. C0 `86dbf2cc` remains independently reviewable.

## Verification attempts

The first public sweep incorrectly used an unoptimized verifier: 344/351,
seven IC-verification timeouts. Its complete request, rows and summary are
preserved in `public-unoptimized/`. This is a build-configuration mistake,
not a passing gate. An interrupted extended-budget baseline replay produced
no conclusion. The fresh full sweep passes **351/351** (`public/`), using the documented
optimized verifier with debug assertions, overflow checks, IC verification and
all original process deadlines enabled. `source-binding.json` identifies the
committed C2 source behind the pre-commit request revisions.

NG comparisons pass 320 scripts and 10 module entries in all nine modes.
Seven modes pass at the ordinary 10-second fixture budget. Combined-all and
combined-off pass with a 300-second fixture budget and one worker; the earlier
combined-all 10-second and 60-second iterative-release timeouts remain in
`timeout-attempts.json`. These are correctness results, not performance evidence.
`compare.json` records every mode and its budget; no fixture is skipped.

## iMac public86 checkpoint

Job **20261101T210008Z-t077-s3c-c2**, C2 **63b2347f** vs **8322fc98**.
The runner was idle before submission; submitted at UTC 21:00:08, after the
blackout. Only temporary refs to the measurement runner were transferred.
Five natural-layout pairs per case, workers-4 v5 policy: **430/430 valid**,
zero invalid/heavy samples, budget met; command 503.1 s/end-to-end 563.9 s.
Verdict: **likely loss**. [Summary](public86/summary.json) derives unweighted
geometric means from the frozen [per-case result](public86/triage.json).

| Lane | Cases | Wall C2/base | Instructions C2/base |
| --- | ---: | ---: | ---: |
| development | 10 | 1.1003 | 1.1275 |
| external | 45 | 1.0986 | 1.0908 |
| broad | 25 | 1.0305 | 1.0283 |
| sentinel | 6 | 0.9886 | 0.9886 |
| selection | 86 | 1.0706 | 1.0690 |

These are exploratory public triage ratios, not the formal four-aggregate,
held-out or RSS gates. The complete request, log, command and status are in
`public86/`. No additional timing or runtime tuning followed this result.
The largest wall loss is public access-fannkuch (5.2939); captured_write is
1.6862, hash-map 1.3483 and date-format-tofte 1.2970. Their causes have not been
isolated at this stop point; do not attribute all loss to one entry mechanism.

**Recommendation: re-freeze combined S3c+S3d.** The new entry/return cost and
broad public regressions exceed the originally priced C3 recovery. C3's write
budget of 80–100 saved instructions per proven write cannot alone pay roughly
940–970 added instructions per call in the N/2N reproducers. Re-price shared
record lifecycle and entry/parameter placement in that plan. Continuing C3–C5
as a standalone payoff trial would need a newly supported envelope; no such
plan is frozen here. C0 remains independently ready. Stop for the lead choice;
C3–C5 and final acceptance remain unperformed.
