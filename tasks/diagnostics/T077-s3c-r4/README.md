# S3c round 4: recursive native-frame repair

Runtime is frozen at **400a14de** (repair base c80ac690); accepted depth
reference: **8322fc98**. The [source seal](qualification/runtime-source.json)
matches every runtime file used by the instruction collector. The default instruction request
records pre-commit parent c80ac690; the canonical request records 400a14de.
Both source maps equal the frozen runtime. Runtime
semantics and tier admission are unchanged. The dormant multi-frame branch
is outlined from run_completion, so its full inline-frame move temporaries
do not remain on every production recursion level. No extra allocation or
stack-policy change. The direct-leaf entry/fallback split remains unchanged.
A debug-only assertion checks active binding_scope against pc_scopes before
ordinary operations, including control transfers; checkpoints install scopes
before the next ordinary operation. A negative unit test proves detection.

The permanent depth collector now covers all 14 paths (the original eight
plus try/catch, construct, spread, arguments, super and general VM), defaults
to the full inventory, and --baseline rejects any lost depth or inventory drift.
The [complete matrix](qualification/depth-final.json) passes --baseline with
zero losses. Representative maximum successful depths on this host:

| Path | S3b debug | Repair debug | S3b release | Repair release |
| --- | ---: | ---: | ---: | ---: |
| forEach | 183 | 253 | 935 | 1248 |
| map | 182 | 250 | 927 | 1233 |
| reduce | 236 | 290 | 1026 | 1355 |
| call | 437 | 670 | 3448 | 4096+ |
| apply | 366 | 522 | 2335 | 2699 |
| reflect_apply | 367 | 523 | 2491 | 2910 |
| yield_star | 152 | 229 | 820 | 1685 |
| plain_in_callback | 4096+ | 4096+ | 4096+ | 4096+ |
| try_catch | 513 | 631 | 1872 | 2455 |
| construct | 502 | 619 | 1832 | 2388 |
| spread | 224 | 244 | 1159 | 1359 |
| arguments | 393 | 526 | 1706 | 2410 |
| super | 390 | 521 | 1706 | 2410 |
| general_vm | 513 | 631 | 1872 | 2455 |

`+` means the 4,096 probe cap, not a measured ceiling. Native stack limits
and the CLI entry are unchanged. Source, output, failure and binary hashes
are in the complete receipts. The predecessor failed the six added paths
[here](qualification/depth-before.json). The [three original reviewer files](qualification/reviewer-repros.json)
abort on c80ac690 and produce the expected R lines on both S3b and the repair.


Exact cached/IC-off Test262 is not repeated for this frame-layout-only repair:
no JS operation, frame contents, scope transition, admission or binding policy
changes; the existing complete production scans each pass 42,672 configured
cases. The debug assertion adds observation only. Round4 still repeats the
requested stress, semantic and depth gates; the source-pinned results below
all qualify the same frozen runtime.

Pre-existing reproducers are retained in [s3c-findings](s3c-findings/). They
remain out of scope: nested finally/continue skips outer finally, duplicate
sloppy parameters select the first argument, native stack overflow aborts
instead of RangeError (S3d). Lead round4 also reports a healthy public86 on
c80ac690: development wall/instructions 0.962/0.985, external 0.988/0.997,
broad wall 0.950, sentinel wall 0.956. These are lead-reported predecessor
results, not a performance claim for this repair.


The decisive native reduction is the inline-frame run_completion driver:
arm64 debug 5,504 -> 432 bytes; x86_64 debug 5,464 -> 392 bytes. Release
previously reserved 1,632/1,592 bytes (arm64/x86_64); that driver now inlines
into its caller, whose measured prologue stays unchanged. The smaller boxed
owner driver shrinks 208 -> 112 / 184 -> 120 bytes. Full before/after prologues,
including unchanged direct-leaf/construct/eval fallback entries and the newly
outlined helper, are in qualification/native-*.json. A missing standalone
symbol means inlining, not a zero-byte native frame. x86 counts exclude the
8-byte return address consistently. The depth table is native arm64. The
x86 prologues are static cross-build evidence; optional local cross-CLI
smokes did not complete; both children remain in OS uninterruptible wait
after SIGTERM/SIGKILL (PIDs13661/14047, recorded in qualification).
No x86 execution-depth result is claimed.

Two exploratory inlining layouts are retained as trial depth JSON. Inlining
the direct-call fallback grew admitted-call paths; the complete conditional
trial lost release call depth (3,426 vs S3b 3,448) and was discarded. Final
function/call.rs is byte-identical to c80ac690. The driver-only repair meets
all paths without any new allocation or tier condition.

Completed semantic gates: fast gate at 400a14de; nine NG modes each 322 scripts
plus 10 modules; seven stress modes each 2,754 tests and combined IC-off 2,712
(the existing 42 IC controls skipped); public 351/351; fuzz 500/profile,
2,000 total, 1,791 MATCH / 209 PRE, no new regression or mode failure.
[Profile counts](qualification/fuzz-profile-counts.json) and raw-result hash
preserve the distinction. The negative checkpoint assertion unit test passes.
The initial unoptimized-debug NG run timed out a fixture at 10 seconds;
its failure/log are retained. The complete optimized assertion-enabled run
passes all nine modes with the same case timeout.

Full gate passes, including 5,174 curated Test262 cases and the default,
agents and perf-counters configurations. x86_64 all-targets agents+perf-counters
compile passes. Default-feature instructions across all109 frozen receipt
cases have geomean **0.999410338** vs predecessor; sentinel **1.000104818**
and fixed-clock diagnostics **1.000237208** retain their tiny positive ratios.
No per-case or per-lane zero-regression claim is inferred from that aggregate.
[Canonical production instructions](qualification/instructions-canonical/summary.json)
are complete over the identical109 inputs, three interleaved AB/BA/AB pairs,
with pinned binary/source/input hashes and matching semantic output. The corpus
geomean is **0.999866287**, so aggregate instructions do not worsen. Lane
ratios: development0.999669954, external0.999975214, broad1.000207618,
sentinel0.999966439; diagnostic1.000011846 and repro0.999277806 are separate.
The small positive lane/case ratios are retained without rounding to passes.
Both production binaries use the canonical agents/fat-LTO/16-unit recipe;
the new ARM order removes only the no-longer-emitted inline driver symbol.
No runtime change or measurement-driven tuning follows either collection. ARM order is
a strict-audited symbol refresh of the prior ranking (no new profile claim).
[Fresh native x86 public profiling](qualification/x86-order/summary.json) completes
as job **20261102T102424Z-t077-s3c-r4-order**, exit0, source400a14de:
537-line order, one relink, zero stale/executor/strict failures. Seven optional
inlining/alias findings remain (the preceding receipt also has seven), all
preserved in the raw audit. Binary SHA1879388fc450b2fbaa3634b72ab9782fc52da7de4acde2a0cf7ca5c8f6f4afe5;
order SHAd582777240e44534d017d8a56d3ea46f936495541f0bbae4eee70b7d3a7ad0ef.
The job checks every runtime hash unchanged and restores its generated order
before exit; the persistent runner checkout was verified clean at400a14de.
Artifacts were fetched directly into this worktree. The attempted cross-build
shortcut was rejected for host-dependent raw symbol identities; it changed no
order. The published order comes from the completed native job.

[The final seal](freeze.json) records the runtime, both native orders and every
qualification artifact. **Owner STOP.** Lead redoes formal, held-out and RSS
acceptance on this frozen source/order pair. No further owner runtime tuning
or timing collection, no origin push and no main merge. The earlier c80ac690
lead results and all tiny positive instruction ratios remain observations,
not final acceptance for400a14de.
