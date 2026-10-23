# T073 measurement record

Owner: [T073](../../T073-local-read-bimodal.md). All runs are diagnostic;
raw assets remain in the named serialized iMac queue jobs, outside Git.
All samples are retained; no acceptance data is replaced or pooled.

## First observations (diagnostic)

Canonical 60 repeats finish successfully in 94 s. Base N/2N wall medians
0.256988/0.506546 s, candidate 0.262068/0.516578 s. Candidate 2N user CPU
has two >1.5x-median tails (0.87 and 1.05 s versus median 0.50); base has
none in 120 processes. A further candidate process uses 0.73 s. Every slow
sample has zero major faults; instructions remain within 0.07% of the normal
2N count. This reproduces the tail in serial, without four-worker contention.
It does not establish a startup-selected binary mode.

ASLR launcher 40 repeats per mode/count/image: off has no >1.5x CPU tail;
on has smaller candidate tails (N max 0.32 s, 2N max 0.65 s). The new
launcher's first on/base N wall is 1.237801 s with 0.25 s user, a separate
first-file launch delay; it is retained, and later collector versions preflight
the launcher itself. This wall delay does not explain T072's doubled user CPU.
Vmmap verifies fixed off code/stack maps and changing on maps. One initial
vmmap attempt exits 255; subsequent on/off maps succeed, with no target failure.

The local feature-only trace admits region 13..34, eight scalar registers,
zero boxed registers; the initial six-operation lowering becomes five final
operations (compare, exit, two adds, increment) in WideLoopFrame's typed executor. All twelve exact x86 profiles (six per image) assign every sampled
leaf to that executor. No repeated named-property read occurs in this loop.
The local full gate passes on unchanged `f75bec10` runtime: 2,382 runtime,
732 benchmark-tool, 35 script tests and 5,170 curated Test262. Log SHA-256
`4938f09c9a83a82aa9ac33db1236323b027ca5a28555e23db2b70e97d7ebe71f`.

Before inferring address causation, expand to interleaved on/off 200 repeats
at 2N per image; observe live segments, all six frozen code layouts, and a
full-page environment sweep (256 pads, 16-byte steps, three N repeats per
image/pad, ASLR off). The small 0..240-byte sweep alone cannot exclude a
specific 4K alias. Every sample and every layout remains in the evidence.

The address debugger pilot fails before launching the engine: macOS reports
"this is a non-interactive debug session, cannot get permission to debug
processes." No permission or machine setting is changed. Supplemental probe
builds therefore log safe `as_ptr()` addresses of the allocated scalar bank,
immutable operation stream, program and outer scratch frame after taking the
scratch, before the executor call. The inner dispatch source is untouched.
Build both exact revisions in isolated queue worktrees, keep their patch and
binary hashes, check every checksum, and compare the probe's own fast/slow
populations. If logging removes the tail, its addresses cannot explain stock
samples. No probe is a production candidate or acceptance measurement.

## Queue ledger

| Job suffix (20261023 UTC) | Purpose | Result |
| --- | --- | --- |
| `161241Z-t073-canonical60` | 60 fresh processes per count/image | exit 0, 94 s; 2 candidate 2N CPU tails |
| `161402Z-t073-aslr40` | matched process-local ASLR on/off | exit 0, 126 s |
| `161444Z-t073-exact-profiles` | exact-image 1 ms stack samples | exit 0, 50 s; all leaf samples in typed executor |
| `161824Z-t073-env-sweep` | mapping verification + 0..240 pad sweep | exit 0, 150 s; 10 N processes/pad/image |
| `162213Z-t073-address-pilot` | LLDB addresses | exit 1, 6 s; OS refuses non-interactive debugging |
| `162257Z-t073-controls40` | three initial controls, 40/image | exit 0, 30 s |
| `162514Z-t073-segments120` | live trajectories, 120/image | exit 0, 125 s |
| `162616Z-t073-six-layouts40` | 40 2N processes/image/layout | exit 0, 245 s; natural candidate 1 tail, all other cells 0 |
| `162742Z-t073-aslr-interleaved200` | 200 2N/mode/image, alternating | exit 0, 415 s; candidate on/off tails 4/0 |
| `163013Z-t073-stack-page-sweep` | 256 pads, 3 N/pad/image, ASLR off | exit 0, 404 s; 0 large tails in 1,536 processes |
| `163817Z-t073-address-probes120` | throwaway pointer loggers, 120 2N/image | exit 0, 331 s; no large tails, altered placement |
| `164452Z-t073-controls-duration40` | half-second controls, 40/image | exit 0, 132 s; no large tail in 240 processes |
| `164728Z-t073-relocation-confirm240` | candidate natural versus +16, 240/image | exit 0, 250 s; natural/+16 large tails 5/0 |
| `170507Z-t073-code-aslr-control200` | same text, PIE header/mapping control | exit 0, 11 s; non-independent pilot, no distribution |

## Process trajectories and controls

Live 5 ms rusage polling (`162514Z-t073-segments120`, 125 s) preserves 120
2N processes per image. Base user CPU min/median/max 0.496198/0.499073/0.509989 s;
candidate 0.505331/0.508964/0.696165 s. Neither series has a >1.5x whole-process
tail. Candidate repeat 113 has CPU ns/instruction across successive billion-
instruction intervals: .0585/.0559/.0558/.0911/.0924/.1255/.1021/.0558/.0565.
Repeat 4 has .1093/.0597/.0540/.1208/.0611/.0596/.0510/.0558/.0548.
The register/op arrays do not intentionally move during this region. Rates
can change inside an observed execution; this is not evidence of one constant
startup-selected multiplier. Polling can perturb execution and live counters
have snapshot quantization, so these trajectories do not establish the stock
process's precise hardware mechanism. End user time agrees with wait4.

The first controls (40 processes/image each) have no >1.5x user tail:
global-read medians 0.20/0.20 s, branch arithmetic 0.07/0.07 s, dynamic property
read 0.06/0.06 s (base/candidate). These are shorter than local-read; matched-
duration controls are collected separately, not substituted for this run.

The proposed T017 change is [case-health failure containment](protocol.md).
It does not discard a process, change timing statistics or promote T072.

The six-layout diagnostic finishes all images. Variant 0 has one candidate
2N tail (0.99 s user versus 0.50 median); variants 1..5 have none in their
first 40 processes/image. Because tails are rare, this observation alone is
not a remedy or a reliable absence claim. Confirm **the smallest predefined
relocation, +16 bytes**, against the unchanged natural candidate in 240
alternating 2N processes per image. Both images are the frozen candidate,
with identical normalized function bodies; `base`/`candidate` raw labels in
this within-candidate job mean natural/+16 respectively. This is a causal
code-placement diagnostic, never selection of an acceptance layout or a
T072 attempt. Preserve both full distributions and all failures.

## Exact executor code and ASLR intervention

The sampled hot symbol is the WideLoopFrame `typed_loop::execute::run`
closure. In both canonical images it is 13,696 bytes / 3,000 listing lines (including
jump-table data, not a dynamic instruction count).
[Reproducer](compare_code.py) compares the exact binaries.
Base starts at `0x100260bd0`; candidate at `0x100261d20` (delta `0x1150`).
Instruction text is identical after converting local branch targets to offsets
within the function and ignoring RIP-relative displacements/external numeric
addresses. Operands, register allocation and internal branch destinations
match; this normalization is not a claim of byte-identical linked text.
Normalized listing SHA-256 in both is
`3ba984b5b8029300c9ac4ec9a7dc43ee1e9ef541585a9a350e2b3fe1657f31c0`.
The hot raw symbol is live in the order file; it is not a missing-executor
lookup. Exact stack samples and live counter source paths agree on this body.

Interleaved ASLR job `20261023T162742Z-t073-aslr-interleaved200` exits 0 in
415 s, no intruder samples. It alternates mode and executable ordering, with
200 fresh 2N processes in each of the four cells. No process is excluded.

| Image / ASLR | User CPU min / median / max s | Wall min / median / max s | >1.5x CPU tails |
| --- | --- | --- | ---: |
| base / on | .49 / .49 / .50 | .501061 / .508078 / .516374 | 0/200 |
| base / off | .49 / .49 / .51 | .502212 / .507997 / .520594 | 0/200 |
| candidate / on | .50 / .50 / 1.06 | .510557 / .518048 / 1.077360 | 4/200 |
| candidate / off | .50 / .50 / .54 | .512661 / .517667 / .552384 | 0/200 |

Candidate user CPU mean / sample SD: on .512300 / .053993 s,
off .502200 / .004825 s. On has additional .58/.64/.64/.64 s processes;
large tails are .76/.77/.79/1.06 s. Its full instruction range is
9.119573–9.125818 G, versus off 9.119559–9.120178 G. Disabling ASLR removes
the large observed tail in this fixed-budget run, not a universal absence
proof; zero events in 200 still permits a rare event rate. ASLR changes code
and stack placement together, so this experiment alone cannot identify which
addresses matter or a specific cache/predictor/4K-alias mechanism.
Raw JSONL SHA-256:
`77ea6b727025bc9532ab6fbb9eaa0737bc4e90ede567eadafb73ccd5c95b963f`.


Retained raw stream hashes for earlier experiments:

| Stream | SHA-256 |
| --- | --- |
| canonical60 | `3aa6d9e7c09b5903c2b1b23cf3e7852f989593a015f0cb846c7bab582df12b2b` |
| ASLR40 on | `f578baf02d788ee5fc15f45c16f1a15b49fafa4fe812f287a9acec21bfd285d1` |
| ASLR40 off | `6b57440093920a51dd9cf6e41efc583f501f26b7f286113a0fa17493af2d2876` |
| short pads | `4b36e4b7d88086fe338c34a57c65b1f094c622ad09046d36aeecd9aa1816dff6` |
| trajectories | `5c7f405ea3eb29ce3e53b1532f3841a89c7b18819b7d2378ff092832bffe4ed0` |


## Placement-only layout distributions

Job `20261023T162616Z-t073-six-layouts40` uses the original audited six-image
matrices; no rebuild or runtime edit. Padding displaces all linked functions
uniformly and the original audit preserves their normalized bodies. Each cell
has 40 fresh 2N processes; every sample is retained. Times are seconds.

| Padding bytes | Base wall median / max | Candidate wall median / max | Base / candidate >1.5x CPU tails |
| ---: | --- | --- | --- |
| 0 | .507035 / .512406 | .517017 / 1.011428 | 0 / 1 |
| 16 | .517792 / .524920 | .509852 / .518646 | 0 / 0 |
| 64 | .506848 / .512228 | .516670 / .522354 | 0 / 0 |
| 240 | .478750 / .486756 | .506909 / .515461 | 0 / 0 |
| 512 | .507191 / .510153 | .517354 / .521117 | 0 / 0 |
| 1008 | .478575 / .487878 | .507585 / .517322 | 0 / 0 |

Base's median shifts from .507035 to .478575 at unchanged work, confirming
that placement sensitivity exists beyond T072's property proof. The rare
large tail remains candidate/natural-specific in this run. Forty zero-tail
samples per relocation do not prove a remedy; the separately predeclared
natural/+16 confirmation retains 240 processes per image.


A separate prospective header control copies the unchanged candidate twice,
ad-hoc signs both, and clears only MH_PIE in one. It asserts identical complete
`__text` bytes before and after signing. Two vmmap trials per copy must show
fixed non-PIE code but randomized stacks (and randomized PIE code); only then
collect 200 alternating 2N processes per image. If mapping behavior is not
independent, retain the mapping pilot and do not run a redundant distribution.
No PIE/security setting or production binary is changed. This distinguishes
code-address randomization from the private launch flag's joint code/stack
intervention if the live loader supports that distinction.


## Full-page environment sweep

ASLR off, padding lengths 0..4080 in 16-byte steps, three N processes per
length/image; 768 samples per image, none excluded. User CPU min/median/max:
base .24/.25/.25 s; candidate .25/.25/.27 s. Wall min/median/max:
base .255093/.258403/.266333 s; candidate .259730/.263434/.284544 s.
Instruction ranges 4.572258–4.572862 G base and 4.572303–4.573030 G candidate.
No large CPU tail or intruder sample occurs. This does not reproduce a
simple fixed stack-offset-only hazard. The knob is environment-string length;
stock stack pointers were not accessible through the rejected debugger, so
this is not a claim of measured coverage of every actual stack address.
It also does not exclude time-dependent alias/predictor/cache hazards.
Raw JSONL SHA-256:
`a5e5e176c84b83882f519cac11aad97183d984ce9ec6fa406f1ee454d595997e`.


## Supplemental pointer logger

Safe pointer logging changes outer code/frames but leaves the normalized hot
executor listing identical (same hash above, 13,696 bytes). Probe executor
addresses: base `0x100260da0`, candidate `0x100261b20`. Binary hashes:
base `48d3389024f5d0b534a1a476561490ab83bf0c6e10c69976a8c8db196a793b62`,
candidate `ce277984672ee4a76222e0f2dfdd843039e544bfbf76c2ac0532d4325efe4d2c`.
Both safe patches hash to
`8de96be7d20ab9c42c2ec09bdaf388c69c1ac997c623e232070d20db793df1df`.

All 120 2N processes per image have exactly one address log and a correct
checksum. Both user CPU distributions are .50/.50/.51 s min/median/max;
wall base .512173/.516962/.524900, candidate .513102/.517019/.523304 s.
No large tail is reproduced, so these addresses cannot identify stock slow
placements. All observed residues mod 4096 are bank `0x260`, ops `0xd20`,
program `0x560`; scratch base `0x130`, candidate `0x0f0`, in all 120 each.
The shared bank/ops/program residues and 64-byte frame difference describe
these probes only; absolute ASLR addresses vary.

Base repeat 116 reports 8,965,330,431 instructions, against a usual 9.1198 G,
with .50 s user CPU, correct 138,412,032 checksum and the same final op count
(5). This isolated counter anomaly is retained and unresolved; it is not a
speedup and is not removed from the stream. Other base counts reach 9.120196 G;
candidate range 9.119709–9.120546 G. It does not account for the stock doubled
CPU-time samples, which are independently visible in user and wall time.
Raw probe JSONL SHA-256:
`ce6234bdf00187694b7e85b696077ff1ca986752a8fe75e19171e1fede8cb007`.


## Duration-matched controls

Forty fresh processes per image/case, all retained, no >1.5x user CPU tail or
intruder sample. These are independent diagnostic controls, not replacements
for the shorter first control run. Iterations: global 57,671,680; branch
33,554,432; dynamic property 4,194,304.

| Case | Base user median / max s | Candidate user median / max s | Base / candidate wall median s |
| --- | --- | --- | --- |
| global_read | .49 / .50 | .50 / .53 | .507329 / .512146 |
| branch_arithmetic | .58 / .59 | .60 / .61 | .594820 / .614131 |
| property_dynamic_read | .50 / .51 | .51 / .51 | .516371 / .523080 |

Dynamic-property instruction medians 7.693796/7.664521 G still show the
candidate doing slightly less work, with no accompanying large tail here.
These controls constrain scope; 40 samples cannot certify rare tails absent.
Raw stream hashes: global
`727008ad9d5adc91bb647407eb75c2806d076d63f2e54e0ff309e28225c80e84`,
branch `64953c62434916510a347e9c322fdf6153235bf9bfb0c5c1f9f6ef30c259c862`,
property `3e59575b9ca962bc0fa534a444302acca0fba6ef363c6632b58d02cb8760d9ef`.


## Placement confirmation: same candidate, natural versus +16

Job `20261023T164728Z-t073-relocation-confirm240`, exit 0 / 250 s, no intruders.
All 480 processes have the same T072 source/allocator/operations and pass the
exact checksum. Paths have equal lengths; ordering alternates by repeat.
The original immutable variant audit establishes one uniform text displacement
and preserved normalized bodies. Raw labels `base`/`candidate` mean the
candidate's natural/+16 images here, not the T072 base/candidate source pair.

| Candidate image | User min / median / max s | Wall min / median / max s | Large CPU tails |
| --- | --- | --- | ---: |
| natural | .50 / .50 / 1.04 | .510402 / .517210 / 1.058508 | 5/240 |
| +16 text | .49 / .50 / .51 | .503566 / .509789 / .520034 | 0/240 |

Natural mean / sample SD .516500 / .064745 s; +16 .498417 / .003771 s.
Natural large-tail repeats 1/68/70/191/216 use .86/1.04/.84/.84/.93 s CPU.
Its instruction range is 9.119565–9.125635 G; +16 9.119496–9.120179 G.
The scalar operations and algorithm do not change. This confirms executable
placement as a causal variable for the observed tail, with the usual finite-
sample limit on zero events. It does not identify a silicon stall counter,
establish a universal safe offset or qualify a production layout policy.
No T072 acceptance sample, attempt or verdict is replaced by this diagnostic.
Raw JSONL SHA-256:
`25a5f3130c62ee1cbd4fc901f9ab9efc2c1815c117966807394110d2777a4733`.


## Header control: code/stack randomization cannot be separated here

Job `20261023T170507Z-t073-code-aslr-control200` retains its mapping pilot
but correctly runs no 200-process distribution. Both re-signed copies preserve
the entire 4,959,168-byte `__text` section at file offset 12,288, SHA-256
`824cfac9f1fdd208742d084f6651942befa9f867ce7fb35192d60a5dc4c6696d`.
PIE on maps text at `0x1057ff000` / `0x10b4b2000` and distinct stacks.
PIE off maps text at `0x100000000` in both trials **and also fixes the stack**
to `0x7ff7bf701000–0x7ff7bff00000`. Thus MH_PIE is not an independent
code-only ASLR control on this host; the predeclared condition fails.
No source/production executable or machine setting is changed. Original
PIE and launch-flag experiments cannot by themselves separate code-ASLR from
stack-ASLR contributions. The independent placement confirmation above still
establishes text geometry as a causal control.
Identity JSON SHA-256:
`db2bb18728b6005e708dd2b88d8d3e242d7169526d12e26992e15f31a14c3048`.

## Completion boundary

All fourteen T073 jobs finish. The sole failed job is the retained OS-denied
LLDB pilot; no job is cancelled or silently replaced. Every timing/build runs
through the shared queue, between local 00:16 and 01:24 CST, outside the
02:45–05:00 exclusion. Other owners' queued jobs run normally between these
jobs. No counter tail is relabeled load-invalid, and no partial run becomes
acceptance standing. No probe, PIE flag, pad or offset is a production fix.
The [owning task](../../T073-local-read-bimodal.md) gives the classification
and the [T017 proposal](protocol.md) gives the reviewable protocol response.
