# T058: Attempt 1 closed incomplete

- Status: closed-incomplete; no promotion or rejection established.
- Implementation: present on the T058 branch; no revert or retention claim.
- Verified at: `0f61bbab` original review; closure checked from raw logs and queue status on 2026-10-19 UTC at rebased `0da2f4e1`.
- Evidence: this record, original frozen plan and job IDs below; raw outputs are local, not retained in Git.
- Unresolved: formal precision fails at the cap; cancelled held-out completion supplies no acceptance evidence.
- Next action: the single remaining attempt is prospectively bound in [T058](../T058-regexp-lazy-quantifiers.md); never resume this attempt or pool its samples into that run.

## Closure and deviations

Formal continuation `20261019T033833Z-t058-formal-bitops-continuation`
reached 240 blocks with relative half-width 3.543300%, above 3%.
This attempt is **incomplete**, with no PROMOTE. The original plan bytes
remain frozen; this outcome is recorded here and in the plan index.
The governing v4 completion passed screen 1; the previously recorded
attempt 2 is unspent. This closure does not reinterpret the natural v3
label as an additional valid failed attempt or grant a third attempt.

Coordinator cancelled both `20261019T123407Z-t058-heldout-k6-complete-r2`
and `20261019T123416Z-t058-promotion-decide-r2`: status `done 130
(cancelled)`, status-file time 2026-10-19 13:08:43 UTC. The held-out
replacement had begun layout 1; the decision produced no result. Retain
partial output as cancelled evidence, never as a complete panel.

An ad-hoc SIGSTOP pause of iMac daemons was a protocol deviation, not the
qualified quieting state. Workspace wrapper commits `8d2352a` (12:43:50 UTC)
and `29516ec` (12:45:45 UTC) added repeated STOP of photo/media analysis,
then other named daemons and an optional root helper; `ec6501a` and
`78bbe79` reverted them at 12:50:27 UTC. That interval overlaps running
`20261019T123407Z-t058-heldout-k6-complete-r2`; its samples are excluded
in full. The associated `20261019T123416Z-t058-promotion-decide-r2`
was queued/cancelled and measured nothing. Job logs do not seal the exact
manual STOP/CONT boundaries or which individual processes were paused;
no pair-level assertion about that interval is possible. Treat all T058
old-attempt timing as historical only for the new acceptance, including
`20261017T222653Z-t058-screen1`, `20261018T013229Z-t058-screen1-complete`,
`20261018T080835Z-t058-formal30`, `20261018T080836Z-t058-heldout30`,
`20261019T033833Z-t058-formal-bitops-continuation`, and disturbed
`20261019T033847Z-t058-heldout-k6-complete`. Their logs do not establish
that the later wrapper patch applied to those earlier jobs.

T061 triage `20261019T132248Z-t061-t058-triage` is diagnostic only
(papaparse 0.1848 [0.1812, 0.1869], unclear due to control intervals).
It did not select this acceptance run, change targets/payoffs, reset the
attempt budget or waive any obligation. The authorized new attempt replaces
the closed incomplete attempt prospectively; no old/new timing is pooled.

## Original evidence (recorded at 0f61bbab)

- Status: active; INCONCLUSIVE, no PROMOTE/REJECT verdict yet: formal precision
  fails at the 240-block cap; disturbed held-out completion replaced.
- Implementation: present (`0dc5d6a1`, `1b8c2159`, `cec613d2`; measured
  candidate `392ca717`, runtime-identical to them); no runtime edits in this review.
- Verified at: review `06cb0f35`; raw continuation checked locally; queue status
  confirmed 2026-10-19 12:34 UTC: replacement running, decision queued after it.
  Original review `5503cfff`: base/candidate `351004f3`/`392ca717`; all three jobs exit 0, no DISTURBED.
- Evidence: deciding numbers and rule retained here; local artifacts, not
  retained: `outputs/imac-bench/<job>/`, `outputs/t058-screen1/`, `outputs/t058-heldout30/`,
  `outputs/t058-promotion-review/` under the workspace root.
  SHA-256: formal summary `ee2d0c2f...cc8d9f`, held-out `panel.json`
  `d5f5f461...6c0cf02`, screen `complete.json` `fff35cfc...0cdb`,
  Test262 entry `9b24e29a...641f7`. Sealed bundle, both panels' raw packets,
  identities and frozen unit/profile validation checked locally.
- Unresolved: formal precision still >3% at 240 blocks, so promotion remains
  incomplete under the frozen rule; held-out replacement pending; RGI_Emoji template.
  Local replay's 2.22e-16 mismatch still requires strict original-host replay;
  the previous decision stopped at its disturbance guard, before report replay.
- Next action: fetch replacement and decision below, check exit/DISTURBED and
  record the canonical verdict. No further formal blocks beyond the frozen cap.
- Jobs: `20261018T080835Z-t058-formal30` (2759 s, 3 light background samples),
  `20261018T080836Z-t058-heldout30` (3204 s, 6 light samples), and
  `20261018T080837Z-t058-test262-exact` (1688 s, no timing monitor).
- Follow-up results: `20261019T033833Z-t058-formal-bitops-continuation`
  exit 0, 36 s, no DISTURBED; checkout `392ca717`, published v4 collector
  separately loaded from `766a3431`. Sealed pilot/images/argv preserved.
  `20261019T033847Z-t058-heldout-k6-complete`: exit 0, 15972 s, DISTURBED
  (one >=200% sample, mediaanalysisd/photoanalysisd); whole completion discarded.
  `20261019T033909Z-t058-promotion-decide-followup`: exit 2, 0 s;
  correctly refused the disturbed prerequisite; no decision artifact.
- Replacement jobs: `20261019T123407Z-t058-heldout-k6-complete-r2`
  (timing, running), then `20261019T123416Z-t058-promotion-decide-r2` (non-timing, queued);
  both checkout `766a3431`, submitted only via imac-bench.sh.
  Same clean natural seed hash `d5f5f461...6c0cf02`, raw-replayed before copying;
  original job `out/` is empty, so recover from `target/layout/t058-heldout`.
  Fresh `$BENCH_OUT/heldout-panel-r2`; rerun every layout 1-5, no disturbed samples
  pooled. Decision requires clean exit 0 and consumes this panel plus the retained
  continuation at the cap. Original seed and all frozen evidence remain unchanged.
- Continuation evidence: 30 retained pilot + 101 + 109 new blocks = 240;
  630 fresh processes (303 + 327), 9.527132 s measured process time.
  131 blocks: ratio 0.9336393833, CI [0.9224241286, 0.9765210847], half-width 4.592962%.
  240 blocks: ratio 0.9439697640, CI [0.9299819376, 0.9774174438], half-width 3.543300%.
  Both fail 3%; exit 0 establishes collection success, not precision. Raw hashes,
  order, original argv, retained-ratio conservation, source bindings and clean
  load seals checked; 36 s is legitimate for this one ~15 ms case plus overhead.
  Continuation-set SHA-256 `1304eb6d...2341d3`; no formal remeasurement queued.
- Results: papaparse 0.1865569292, 95% CI [0.1854727456, 0.1874598998] < frozen 0.70;
  SunSpider `bitops-3bit-bits-in-byte` ratio 0.9282498417,
  CI [0.9167765124, 0.9863025702], relative half-width 6.253998% > 3%:
  pilot power rule allocated 131 blocks, then 240; completed outcome recorded above.
  Held-out lane 1.0086606337, CI [1.0066319824, 1.0103259991], exceeds
  ceiling 1.0050182542. Esprima upper CI 1.0273564277 > band 1.0250249555
  (median 1.0209777832, instructions median 0.9939391379). Both need K=6.
  Exact Test262: 42672/42672 Rust passes; NG 42602 passes / 70 failures;
  actionable gaps 0, Rust failures/timeouts/not-run 0 (entry dated 2026-10-19).
- Rule: plan requires complete broad/external evidence and zero Test262 gap;
  target <= min(0.70, reciprocal wall band), every screen pair improves.
  T022: "a wall-only program or lane excess needs the K=6 held-out panel and
  rejects only if all six exceed." Formal precision uses
  n=min(240,ceil(30*(h/.03)^2)), retaining the pilot; a candidate still
  imprecise at 240 stays incomplete. No threshold or attempt budget changed.

Expected replacement: about 4.44 h plus quieting/load recovery; decision a few minutes.
Commands/artifacts: runner `tmp/bench-queue/<job>/`, workspace `outputs/imac-bench/<job>/`.


## Screen attempt 1

iMac job `20261017T222653Z-t058-screen1` (`perf-loop.sh`, policy v3, natural layout only, 2490 s,
exit 3; light background activity in 6 samples, below the disturbance threshold). Verdict SHA-256
`9489c3e9...c9c807`: label `reject`, failure `layout 0 broad: wall lane ceiling exceeded`, K=6 not
run because the natural layout already failed and the gate is not mandatory-panel.

Target `development/papaparse/csv`: wall 0.1862, instructions 0.1146 (frozen threshold 0.70). Wall
events: `broad/top_level_function_call` 1.4316 (band 1.0250, instructions 1.0002),
`broad/string_slice` 1.0407 (band 1.0319, instructions 1.0000), `external/jetstream3-js-subset/cdjs`
1.0336 (band 1.0250, instructions 0.9885). The broad lane failure comes from
`top_level_function_call`, whose five pairs are 1.046, 1.018, 1.491, 1.526, 1.432 with instructions
1.0001-1.0002 throughout: a step change between pairs 2 and 3 on a 9 ms workload, which code that
only changed the RegExp matcher cannot explain by instructions. No other case regressed in
instructions beyond 1.0002.

### Completion under policy v4 (lead decision 2026-10-17)

Policy v4 (`tasks/archive/T059-held-out-layout-policy-v4.md`) makes a natural lane excess a lane
event that needs the complete K=6 panel. The v3-bound plan is evaluated with v4's successor rule;
re-evaluated, the natural packet is `wall event` with `broad lane: lane event`, no failures. This
completes attempt 1; attempt 2 is unspent. Job `20261018T013229Z-t058-screen1-complete` (exit 0,
8795 s) first re-hashed all twelve audited binaries (base natural `c7f0f66a...`, candidate natural
`d2bc9b8d...`, revisions `351004f3`/`392ca717`), then appended layouts 1-5 to the same `screen/`
directory. Verdict `pass`, complete, event resolution `nonpersistent`. SHA-256: verdict
`ce385522...abd9ae`, `complete.json` `fff35cfc...0cdb`, natural-v4 verdict `83005b47...4455`.

| Lane | layouts 0-5 | geomean | ceiling |
| --- | --- | --- | --- |
| broad | 1.0172 1.0028 1.0021 1.0006 1.0068 0.9973 | 1.0044 | 1.0129 |
| sentinel | 0.9693 1.0123 0.9869 0.9965 0.9929 0.9980 | 0.9926 | 1.0157 |
| external | 0.9974 0.9967 1.0016 0.9978 0.9987 0.9998 | 0.9987 | 1.0074 |
| development | 0.8541 0.8470 0.8570 0.8513 0.8435 0.8455 | 0.8497 | 1.0096 |
| portfolio | 0.9832 0.9808 0.9827 0.9803 0.9812 0.9796 | 0.9813 | 1.0072 |

Wall-event cases, median per layout: `top_level_function_call` 1.4316 1.0354 1.0113 1.0112 1.0413
1.0189; `string_slice` 1.0407 1.0212 0.9950 1.0170 1.0194 1.0390; `cdjs` 1.0336 1.0127 1.0175 0.9985
1.0168 1.0099. Target papaparse 0.1862 0.1857 0.1862 0.1853 0.1858 0.1836.
