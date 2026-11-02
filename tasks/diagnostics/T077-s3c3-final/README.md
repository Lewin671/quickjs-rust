# C3 method routes and measurement checkpoint

This receipt's commit contains the qualified C3 runtime. Comparison milestone
is **8322fc98**; parent is **17de7ed7**. Source and binary hashes in the
109-case request identify the measured builds. Earlier C3 receipts remain
frozen. This checkpoint does not establish final S3c acceptance.

## Method attribution and replacement

[Paired coverage](method-coverage.json) uses the unchanged public hash-map and
method_call inputs against an archived S3b source/build. Counts sum positive
non-closure monomorphizations by source function, without timing them. Hash-map
executes **2,160,029** wide entries and **2,070,040** receiver preparations in
the candidate. Receiver-seed reads remain **90,003**, numeric-leaf probes
**180,015**; full call Context builds fall **8 to 1**, with one materialization.
The method control executes **1,023** typed leaf calls in both revisions.
Its candidate dispatches these to the nongeneric user helper instead of also
probing the native-call facade. Receiver/home/this semantics and property IC
guards retain their existing authorities; no property value cache is added.

Wide execution borrows its selected Function once per scalar segment and uses
the typed FrameContext view directly. Pure numeric shortcuts consume only used
Number argument registers; fallback constructs the existing typed/value views
away from the driver. Named reads may be hoisted across user calls only under
the existing direct-leaf admission plus compiled numeric purity. Unsupported
operands deoptimize before source effects, and captures are read live. Tests
exercise coercing captured values and a method replacing itself during a call.
Cold compilation also avoids redundant scope maps and per-op tree lookups.

The immutable body owns the captured-write mask; a closure's instance bit
enables it only after every indicated actual cell validates. No per-closure
mask allocation remains. InitializedMutable annotates Mutable semantics; the
debug verifier normalizes only these two policies, preserving Throw and
IgnoreSloppy distinctions and all capability validation exclusions.

## Local deterministic measurements

[Call slopes](call-cost.json), N/2N with three samples and matching tier counters:

| Calls | S3b instructions/call | C3 | Saved |
| --- | ---: | ---: | ---: |
| upw0 | 1004.025193 | 902.724246 | 101.300947 |
| upw1 | 1334.578185 | 1156.143231 | 178.434954 |
| upw4 | 2274.776535 | 1873.970259 | 400.806276 |

Both captured-write targets exceed 70. Relative to the frozen C2 slopes,
subtracting the zero-write improvement prices each additional `+=` at **81.74**
instructions for upw1 and **84.74** for upw4. This is the combined indexed-read/
proved-write route, not an isolated measurement of RefCell replacement.

[Controls](controls.json) retain five interleaved before/after samples and
semantic counter maps. Hash-map **0.998012440**, method_call **0.988471263**,
tofte **0.917448918**, supplied method repro **0.972242585** meet their targets.
Fannkuch is **1.000503091**, a small excess retained without rounding to a pass;
the preceding sample set was 0.999919453 and the sample ranges overlap.
There was no runtime tuning against that residual. These local instruction
observations are not formal timing, held-out or RSS acceptance.

## Correctness and depth

[Checks](checks.json) record the full gate and x86_64 compile check. Default
runtime tests **2,743**, curated Test262 **5,174**, public sweep **351/351**
pass. [Stress](stress.json) passes seven configurations at 2,743 each and
combined-off at **2,701**, retaining the existing 42 IC-control skips. The
[NG matrix](compare.json) passes **322 scripts + 10 modules** in all nine
modes. [Global-store fuzz](fuzz-summary.json) completes **2,000** seeds,
**1,933 MATCH / 67 PRE**, stable inputs, no new regression or mode failure.
PRE remains a visible baseline/NG difference.

[Depth](depth.json) uses the ordinary CLI stack, without a stack-policy change:

| Probe | S3b debug | C3 debug | S3b release | C3 release |
| --- | ---: | ---: | ---: | ---: |
| forEach | 183 | 216 | 935 | 1,001 |
| map | 182 | 214 | 927 | 991 |
| reduce | 236 | 243 | 1,026 | 1,071 |
| call | 437 | 670 | 3,448 | 4,096 cap |
| apply | 366 | 517 | 2,335 | 2,658 |
| Reflect.apply | 367 | 518 | 2,491 | 2,863 |
| yield-star | 152 | 211 | 820 | 1,258 |
| plain in callback | 4,096 cap | 4,096 cap | 4,096 cap | 4,096 cap |

All sixteen probes meet S3b. The [structural repair](../T077-s3c3-depth/README.md)
owns the before/after native-frame prologues; no CLI limit change hides them.

## Complete public receipt

[All 109 records](counts.json) and their [request](count-request.json) retain
the exact frozen IDs, input hashes, arguments, stdout hashes, instructions,
both counter observations and coverage. All stdout hashes match S3b. The
[summary](corpus-summary.json) reports historical instruction ratios:
development **0.989170**, external **0.993441**, broad **0.978829**, sentinel
**0.980161**. These are local diagnostic geomeans against the original receipt,
not a contemporaneous paired timing protocol or an acceptance claim.

The [collector](../T077-s3c3-counts/collect.py) replays the frozen 109 input IDs,
source hashes and arguments with default instructions, two counter maps and one
LLVM coverage map. Matching toolchain LLVM tools are required. The
[method collector](../T077-s3c3-counts/method_coverage.py) records attribution
with the same public control invocations. No held-out program is consulted.

Default/counter recipes use `cargo build --release -p qjs-cli`, adding
`--features qjs-runtime/perf-counters` only for counters, in separate targets.
Coverage uses `RUSTFLAGS=-Cinstrument-coverage`,
`CARGO_PROFILE_RELEASE_LTO=off` and
`CARGO_PROFILE_RELEASE_CODEGEN_UNITS=16`. The optimized verifier enables
release debug assertions and overflow checks as in the C2 receipt. No
instrumented build supplies default instruction or runner timing observations.

The single `t077-s3c2-c3` runner checkpoint is next. Its external instruction
ratio must be strictly below 1 before C4 begins. Final C5 formal/held-out/RSS
acceptance remains with the lead.
