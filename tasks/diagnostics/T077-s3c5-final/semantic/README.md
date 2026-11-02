# Final S3c semantic and structural matrix

Runtime content **2aa117b7**, native orders **476f1bea**. [Source/binary pins](source.json)
identify the exact rebuilt executables. These files are a frozen completed
matrix; exact cached/IC-off scans and the single final public86 are separate.

[Checks](checks.json): full gate including **5,174** curated Test262 and
**2,753** default runtime tests; full perf-counters **2,805**; x86 all-targets
agents+perf-counters compile pass. [Stress](stress.json): seven modes **2,753**
each, combined-off **2,711** with the existing 42 IC-control skips.
[NG](compare.json): **322 scripts + 10 modules** in all nine modes.
[Public](public-summary.json): **351/351** invocations, including effective
IC-off/verification. No new exclusion or expected failure is introduced.

[Fuzz](fuzz-summary.json) completes **8,000** seeds, 2,000 in each profile:

| Profile | MATCH | PRE |
| --- | ---: | ---: |
| binding | 1,634 | 366 |
| binding-simple | 1,624 | 376 |
| receiver | 2,000 | 0 |
| global-store | 1,933 | 67 |

Inputs are stable, with no new regression or mode failure. PRE remains a
visible base/NG difference; it is not counted as all engines agreeing.

[Call slopes](call-cost.json) save **114.68 / 196.64 / 420.65** instructions
per upw0/1/4 call versus S3b, preserving the expected compact-call counts.
[Controls](controls.json): hash-map **0.991924**, method_call **0.996377**,
tofte **0.917708**, supplied method repro **0.965801** meet <=1. Fannkuch's
small **1.000865** excess stays visible for the final x86 report, without
rounding to a pass. Its receipt coverage has zero checked cell assignments,
so checked binding-write dispatch does not explain it.

[Depth](depth.json) meets S3b on all eight probes in both profiles, with the
ordinary CLI stack and no policy change. [Prologues](native-frames.json): wide
debug **8,832** bytes versus S3b **15,408** and C2 **19,952**; numeric debug
**4,016** versus **5,504 / 7,664**. Wide release **1,120** versus C2 **1,264**.
The numeric release driver is inlined, not a claimed zero-byte native frame.
The [collector](../native_frames.py) records emitted prologues directly.

These checks establish correctness/structure at the pinned source and builds.
They do not replace complete exact scans or formal/held-out/RSS acceptance.
