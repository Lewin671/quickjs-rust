# T056 stage A: equivalent scalar code, data/launch diagnosis

- Status: active diagnosis; d130's smaller frame did not fix local_read wall.
- Implementation: production remains d130c77b on the stage-A branch; only
  isolated diagnostic images add the same-image bank-window policy below.
- Verified at: d130 / image 66ffb46f: complete exact 42,672/42,672 and bound
  86/86 pre-gate pass; lead T061 is done0/unclear. Exact entry is now recorded.
- Evidence: [codegen history](T056-m1-stageA-codegen-diagnosis.md), byte/CPU
  facts below and the versioned [lead protocol](../diagnostics/t056-data-probe/README.md).
- Unresolved: local_read, unpack and heterogeneous_property_read wall excess;
  data/launch factors are unmeasured, and common cause is not established.
- Next action: lead submits the prepared three-case, 15-block data probe with
  its 25-minute watchdog. Do not spend the remaining K=6 attempt yet.

## Completed d130 chain

All three jobs completed done0 without DISTURBED, bound to
`d130c77b7b2e216a3f8e6cf68892ee28dd632c22` and canonical image
`66ffb46f31c226076b6b4fde1405c84dcef789ed382613d8c67a2f10bdd7136e`:

- Correctness `20261021T024349Z-t056-m1a-codegen-correctness`: full gate,
  NG255/subset5170; eight disjoint complete exact shards, 53,572 pinned paths
  exactly once, Rust42,672 pass/0 fail/timeout/not-run. The zero-gap and four
  artifact hashes were checked locally; its burndown entry is appended once.
- Pre-gate `20261021T030549Z-t056-m1a-codegen-pregate86`: VERDICT pass, all86
  policy IDs exactly once, original per-case instruction thresholds.
- Triage `20261021T030903Z-t056-m1a-codegen-triage`: 27 cases, five pairs,
  workers4, unclear; command304.96s/end-to-end366.07s, budget met. Request SHA
  `e480d48d34dbff6cea6fd05de0ceef06bb993aada6e08ba2591976ea0920f3f3`, result SHA
  `e8782a3a8daabe26fcc06aa40a12f1c0e4aff80010eba50a3638d4fc6cb610e4`.

| Triage case | wall C/B [exploratory 95% interval] | instructions |
| --- | ---: | ---: |
| local_read | 1.066049 [1.052157, 1.074267] | 1.000004 |
| branch_arithmetic | 1.018669 [0.997319, 1.031443] | 0.999953 |
| string-unpack-code | 1.032059 [1.002066, 1.049938] | 1.001737 |
| heterogeneous_property_read | 1.034113 [1.024157, 1.044951] | 0.977283 |
| class-fields | 1.016175 [0.996712, 1.030120] | 1.015515 |
| bitwise-and | 1.018801 [0.968376, 1.109355] | 1.009544 |

Gains remain: global0.905768, dynamic0.936739, moment0.958689,
meriyah0.946042, diff0.989064. These are diagnostic results, not stage
acceptance. Frame/spill repair does not explain local_read; one valid attempt
is still spent and one remains across A+B. Runtime CI39534230493 was cancelled,
not green; no main integration is attempted.

## Decisive local_read byte check

Images are physical base b0dfc65d (f58c696e) and candidate66ffb46f (d130).
The repeated program is Lt, Exit, Add, Add, Update, with common dispatch and
backedge. New exact-image CPU sampling job
`20261021T031937Z-t056-data-cpu-profiles` confirms the WideLoopFrame closure:
952/953 base and1025/1025 candidate main-thread self samples. Reported hot PCs
include base offsets150/134 and candidate166/177 (decimal), all in dispatch.
Samples identify the body, not timing ratios. Prior per-site trace
20261020T232947Z (4842f823) identifies the op stream; those compiler files are
unchanged in d130. Dispatch tables map its scattered arithmetic/update/control
blocks. This is not a hardware per-instruction retirement trace.

The [byte audit producer](../diagnostics/t056-data-probe/audit.py) reproduced
report SHA `c0aa75b88441906216ac7ccc9bcc3a58199f4aaf05f9a81742f6e2bf7a03336f`.
It compares actual encoding bytes/lengths, not just disassembly strings:
verify the encoded rel8/rel32 resolves to the decoded target, zero that field;
verify RIP displacement and equal referenced numeric constant bytes, zero the
displacement. LEA table addresses remain explicit placement facts below.
Dispatch data is not decoded as instructions. The repeated hot paths are
**byte-equivalent modulo addresses**, with the same registers and constants.
This does not assert identical block distances, cache placement, cold exits,
whole functions or microarchitectural behavior.

| Selected path | instructions | encoding bytes | normalized byte SHA-256 (both images) |
| --- | ---: | ---: | --- |
| Lt | 32 | 140 | `fd5444cf62325cd3652af0d59c20ac96797623749374a1e66a3fa0dd81a85c4f` |
| Exit | 12 | 51 | `318c44ec1cf2aa50735d8711be6c07a04dc9b2b356d3bb75cce4b4da30b0925e` |
| Add | 32 | 141 | `356dce6c26fab202a8edff770e89e39b14f1d19f32c9c5b8bd44b728ef0b35cd` |
| Update | 17 | 81 | `f56e64669324532e2ab61708ec7fcfc1f222f913e3163ddb1e247cdc84486b9b` |
| Dispatch | 13 | 48 | `379ba7c462f4e7484785b68292047518f6d17b9382bb06da0ce9d437beaa4a9f` |
| Backedge | 4 | 22 | `5c04a3518c8ad95d3bc57ca4bf06aed3908a4e332e95c57d796b6fa342377af9` |

| Placement | Base | d130 |
| --- | --- | --- |
| function entry | 0x10025bd90 | 0x100004d90 |
| dispatch header | 0x10025be16 | 0x100004e16 |
| header mod64 / mod32 | 22 / 22 | 22 / 22 |
| main TypedOp table | 0x10025f1d0 | 0x100008238 |
| main table mod64 / mod32 | 16 / 16 | 56 / 24 |
| numeric BinaryOp table | 0x10025f2bc | 0x1000082bc |
| numeric table mod64 / mod32 | 60 / 28 | 60 / 28 |

No selected jump ends exactly on a32-byte boundary. Candidate has two
**unique** hot JCC encodings crossing one: numeric tag `jne` at
0x1000053fc (mod32=28, length6), used by Lt and Add; Exit's `jne` at
0x10000593f (mod32=31, length6). Base has no individually crossing hot jump,
but cmp/test+branch pairs cross at binary switch `ja`0x10025d860 and Update
`je`0x10025d2a3. All these facts, including raw bytes and branch lengths,
remain in the audit; equal header residues alone are insufficient.

## CPU and JCC scope

The same non-timing job reads live sysctl: Intel(R) Core(TM) i7-10700 CPU @
2.90GHz, family6/model165 (0xA5)/stepping5, microcode240 (0xF0).
Intel's [JCC mitigation guidance](https://www.intel.com/content/www/us/en/developer/articles/technical/software-security-guidance/best-practices/mitigation-strategies-jcc-microcode.html)
and [white paper341810-001, pp13–15](https://cdrdv2-public.intel.com/841076/mitigations-jump-conditional-code-erratum.pdf)
list affected Comet Lake U42/U62 models8E/A6, **not this A5 desktop model**.
The page's list is not evidence that this host loaded the described JCC MCU.
No DSB/MITE counters or host-specific erratum status were obtained, so do not
claim JCC caused the regression or add mitigation flags as a production fix.
Byte equivalence plus unsuccessful prior placement interventions justifies
next testing data/launch state; it does not logically exclude all address-side
code/table effects. The table residues and branch differences remain limits.

## Do the other cases share it?

The new exact-image sampling job runs all three original workloads (unpack
repeated40 times for attribution only). Heterogeneous reads also enter the
Wide typed closure: base86/139 self samples there,22 literal_data_slot_value,
19 get_named_object; d130101/144 there,19 get_named_object and13 in the new
get_named_typed miss helper. Its property path is **not** the scalar-only
byte-equivalent path: layout/slot validation and cold calls changed.

Unpack has no dominant typed-closure self entry: ordinary wide activation is
234/3195 base vs266/3233 d130, with allocation/free and regexp/string work
prominent. Its path differs from local_read. Neither profile proves one
shared cause. The prospective canonical zone/env factors cover all three;
bank displacement only covers local_read, avoiding a new-code confound for
the property/string cases. Common factor response would support a direction,
not establish identical roots.

## Controlled bank factor and qualification

Canonical images remain untouched. Two safe-Rust diagnostic images are built
from f58/d130 in isolated source trees; no engine production file is changed.
Each uses one page-aligned8192-byte backing holding260 initialized Typed
cells. A4096-byte logical bank borrows cells0..256 or4..260, chosen once at
construction by equal-length env valuespage00/page64. Both states use the
**same file/inode, machine code and allocation size/alignment**; only the
register window starts at page offset0 or64. Borrowing/indexing uses safe
Box/slices/try_into; no unsafe, FFI, extra dependency or global mutable state.
This applies to every typed bank, with no source/bytecode/benchmark recognizer.
The extra header/backing and code placement differ from canonical: bank00/
canonical is explicitly a bias control, never evidence of a production fix.

Build `20261021T033544Z-t056-data-window-bank-build` done0/247s:

- base diagnostic SHA `e1a12130921770ee9de246c0269ad3fdfd7865c9d2342b16ecc8a35b6ae23448`;
- d130 diagnostic SHA `9692cbafb5d3a1d62a87505ccac8f446e25f1d72b7e1f5b312fd6ab006619d8f`;
- same canonical recipe; mechanical reference pin retains both executor and
  all15 surviving callee residues. Original build metadata calls these keys
  b_page/c_page; physical directories are b_bank/c_bank. Qualification records
  this mapping explicitly without rewriting the frozen build output.

Qualification `20261021T034036Z-t056-data-window-qualify` done0/107s:
all36 smoke case/condition outputs match; both windows on both images each
pass NG255 and focused typed-loop tests. Trace observes page-aligned bank
pointers ending000 and shifted pointers ending040, both mod64=0.
Diagnostic Wide frames are376(base)/360(d130), arithmetic paths32 instructions;
its dispatch starts at entry+0x96, unlike canonical+0x86. This is why the
canonical-vs-diagnostic contrast must remain a bias control.
Qualification SHA `0dbfb015cdb3d56c858d64661bcda643f909abda9b98fe2c41c3ec4df98d24d8`.

Non-timing instruction smoke `20261021T035429Z-t056-data-bank-instruction-smoke`
done0/2s uses10M/20M increments: bank64/bank00 instructions1.000005(base)
and1.000031(d130); bank00/canonical0.999894/0.999870. Wall values in its raw
counter logs are not decision evidence and are not analyzed.

Earlier build032308 failed before compilation (missing PYTHONPATH), retained;
032615 built page-only variantsbde0f68c/2355fbe5 but they are **not used** for
timing. Separately linked page-only binaries would mix code-layout changes
with allocation policy. The same-image window design replaces that idea;
no offset or timing results were searched or selected.

## Prepared lead job and stopping rule

The versioned [protocol](../diagnostics/t056-data-probe/README.md) contains
collector, guards, source/receipt hashes, renderer, budget/quiet-window wrapper,
watchdog and fake-process validator. It remains unsubmitted. Existing complete
d130 correctness/pre-gate/triage are mandatory inputs; it does not weaken the
old B packet's requirement for a complete old probe A.

Three cases, all15 blocks: local16 conditions, heterogeneous10, unpack10;
540 case/condition/block units versus A's1215 in1800s. Nominal800s ×1.3 for
case mix +120s reserve =1160s. Collector hard limit1260s, body1380s,
running-job watchdog1500s including quieting (queue wait excluded). Body reads
the runner start stamp and leaves10s termination margin; the watchdog freezes
and kills only that named job's process tree before closing its tmux session.
No blocks are dropped to meet budget. Exhaustion means whole-job incomplete,
retain every pair and do not patch missing blocks. No automatic retry or
acceptance is implemented.

All factors use one literal execution path with a warm preflight; env keys
are cleared/set on every activation. Natural and bank A/A use independent
byte-identical copies; other roles of each image use shared inodes. Preset
comparisons include canonical C/B, within-image zone/env, bank64/bank00 and
interactions. Intervals are exploratory/unadjusted95%, not v6 acceptance.
Local validation passes: the real collector over fake processes/counters and
real sealed prerequisite artifacts exercises all540 units, environment reset,
raw output, same-inode and A/A checks without launching an engine; temporary
fake evidence is deleted. The rendered commands/remote stop payload parse,
Shanghai/LA/DST and budget guards pass. The byte audit reproduces exactly;
`./scripts/check.sh` passes, including runtime2346, tool663, script35 and
subset5170 tests; the staged fast gate/context/whitespace checks also pass. No owner timing, main push or last
K=6 attempt is authorized or submitted by this handoff.
