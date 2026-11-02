# C4 common suspension owner

The commit containing this receipt pins its runtime in [source.json](source.json).
Parent is **34bf08ac**. C3's single runner gate passed before this work began.

`GeneratorStart` and all four production constructors are removed. One
`prepare_suspended_start` factory builds the complete boxed frame and consumes
borrowed inputs into slots/owned argument recipes before publication. Generator,
async function, async generator and TLA use it. Prologue/start/resume consume
that owner and attach RuntimeServices; their APIs cannot consult the resuming
caller's activation. SuspendedActivation adds only the suspension kind to the
frame owner, with no duplicate bytecode Rc or late argument-seeding API.

C1 already retired the 19-field snapshot and field reconstruction. C4 removes
the remaining start-state family and duplicate code ownership; it does not
claim those earlier deletions twice. A synchronous prologue now returns a true
SuspendedStart, so first next ignores its input without a synthetic yield value.
All seven suspension kinds and all Resume branches remain supported.

The ownership tests prove box/code identity through all kinds, pending cold
completion retention, drop of private values, abrupt completion before start,
and stable identity across a real prologue and repeated yields. Existing class
key/finally/disposal/mapped-arguments/capture/async FIFO/re-entry/TLA tests remain.
Ordinary synchronous frames stay unboxed and no directory or FrameId is added.

[Checks](checks.json): full gate including **5,174** curated Test262, runtime
**2,746**, full perf-counters **2,798**, x86_64 agents+perf-counters compile
pass. [Stress](stress.json): seven configurations **2,746** each, combined-off
**2,704** with the existing 42 IC-control skips. [NG](compare.json): **322
scripts + 10 modules** in all nine modes. No new exclusions or dependency.

[Depth](depth.json), ordinary CLI stack without policy changes:

| Probe | S3b debug | C4 debug | S3b release | C4 release |
| --- | ---: | ---: | ---: | ---: |
| forEach | 183 | 218 | 935 | 1,012 |
| map | 182 | 216 | 927 | 1,003 |
| reduce | 236 | 245 | 1,026 | 1,084 |
| call | 437 | 670 | 3,448 | 4,096 cap |
| apply | 366 | 522 | 2,335 | 2,699 |
| Reflect.apply | 367 | 523 | 2,491 | 2,911 |
| yield-star | 152 | 227 | 820 | 1,613 |
| plain in callback | 4,096 cap | 4,096 cap | 4,096 cap | 4,096 cap |

C5's audit found that the original per-body operand/local/cold free pools
still exist. Their [retirement plan](../../proposals/T077-s3c-pool-retirement.md)
keeps the approved stack-IR banks and moves only cleared free capacity into the
common budget. This must finish before final source freeze and qualification.
C5's complete matrix and final public86 remain outstanding; the lead owns
formal/held-out/RSS acceptance and review.
