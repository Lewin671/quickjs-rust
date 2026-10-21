# T056 stage A: equal-instruction wall diagnosis

This is a diagnostic continuation of [stage A](T056-m1-stageA-record.md), not a new
performance attempt. Runtime remains 4842f823, canonical image 807867f6; base is f58c696e,
image b0dfc65d (full hashes in the stage record). Frozen unit d184dbcc and its 1/2 spent
ledger are unchanged. The owner submits no timing jobs. No production fix is established here.

## New wall evidence

Lead pre-gate `20261021T000300Z-t056-m1a-last-pregate86` passes 86/86 instructions. Lead
T061 job `20261021T000303Z-t056-m1a-last-diagnostic-triage` completes done 0, natural layout,
five pairs, 27 cases, verdict unclear; no DISTURBED marker. Selected medians below are
candidate/base; intervals are unadjusted exploratory 95%, not acceptance evidence.

| Case | Wall [interval] | Instructions |
| --- | --- | ---: |
| local_read | 1.0728 [1.0553,1.0943] | 1.0001 |
| branch_arithmetic | 1.0361 [0.5548,1.0777] | 1.0000 |
| global_read | 0.8928 [0.8756,0.9069] | 0.9999 |
| string_slice | 1.0393 [0.9596,1.0600] | 0.9981 |
| string-unpack-code | 1.0385 [1.0243,1.0512] | 1.0136 |
| bitops-bitwise-and | 1.0383 [0.9565,1.0710] | 1.0257 |
| raytrace-public-class-fields | 1.0270 [1.0029,1.0361] | 1.0158 |
| property_dynamic_read | 0.9406 [0.8908,0.9595] | 0.9251 |
| binary-trees | 0.9991 [0.9922,1.0065] | 0.9865 |
| moment / diff / meriyah | 0.9493 / 0.9968 / 0.9528 | 0.9651 / 0.9680 / 0.9667 |

The local_read excess is persistent and remains unexplained by equal retired instructions.
Instruction repair and successful entry pinning did not establish a wall fix. The last screen
must remain unspent while the cause and a general repair are unresolved.

## Exact text layout

`nm`, `otool` and `tools.benchmark.layout_variant_audit.inspect` read the immutable canonical
files, not new compilation variants. The order pin covers the actual WideLoopFrame dispatcher;
it preserves entry residue, not the position of interior blocks or their relative layout.

| Wide typed body | Base | 4842f823 |
| --- | --- | --- |
| Entry | 0x10025bd90 | 0x100006d90 |
| Extent | 13,696 bytes | 14,960 bytes |
| Prologue stack reservation | 0x178 (376) | 0x2a8 (680) |
| Dispatch header | entry+0x86 = 0x10025be16 | entry+0xb6 = 0x100006e46 |
| Header mod 32 / mod 64 | 22 / 22 | 6 / 6 |
| Dispatch indirect jump | entry+0xb4, mod64=4 | entry+0xe4, mod64=52 |
| Binary arm | entry+0x661, mod64=49 | entry+0x6b1, mod64=1 |
| Update arm | entry+0x36f, mod64=63 | entry+0x3ca, mod64=26 |
| Exit arm | entry+0xba9, mod64=57 | entry+0xbfe, mod64=14 |
| JumpIfFalsy arm | entry+0xc31, mod64=1 | entry+0xc8d, mod64=29 |

Thus both entries remain at page offset 0xd90, but the header moved by 48 bytes relative
to its entry; handlers moved by different amounts. A single entry residue is an insufficient
invariant. This is an observed difference, not proof that header residue 22 is universally faster.
T050 proved a typed-body placement effect on older code; T054 rejected global region preservation.
Neither historical result proves the cause on 4842f823 or authorizes an offset search.

An additional check of the exact rejected 0a9af81b image (SHA 1970bb69, recorded previously)
finds entry 0x10046e630, header entry+0x86 = 0x10046e6b6: mod32=22, mod64=54. Its original
local_read wall excess therefore occurred even with the same header mod32 as the base. A simple
"header mod32 alone" explanation is insufficient for the rejection history. The current reciprocal
move tests broader interior placement on 4842f823; it is not a declaration that residue 22 is the fix.

## Static data layout and limits

Non-timing job `20261021T002216Z-t056-placement-type-facts` passes. It compiles immutable
base/candidate archives with the pinned x86 release compiler, using RUSTC_BOOTSTRAP solely for
`-Zprint-type-sizes --emit=metadata`. These are type reports, not canonical executables or timing
images. Both retain the canonical target and recipe options; the extra analysis flag is explicit.

| Type | Base size/alignment | Candidate size/alignment |
| --- | --- | --- |
| Typed / Value | 16 / 8 | 16 / 8 |
| TypedOp | 16 / 4 | 16 / 4 |
| Property | 32 / 8 | 32 / 8 |
| RegisterFile Box handle | 8 / 8 | 8 / 8 |
| Backing scalar array (256 Typed) | 4096 / 8 | 4096 / 8 |
| TypedLoopScratch | 80 / 8 | 80 / 8 |
| TypedLoopProgram | 528 / 8 | 528 / 8 |
| WideLoopFrame | 192 / 16 | 192 / 16 |
| Vm | 1024 / 16 | 1024 / 16 |
| ObjectData | 112 / 8 | 120 / 8 |
| ShapeWays | 48 / 8 | 80 / 8 |

The 4 KiB bank has no new cache-line alignment guarantee. Program/operand sizes and the frame's
size did not grow, so a claim that those arrays grew is false. ObjectData does grow; adding Rc's
two word counters requests 128 versus 136 bytes before allocator rounding. Actual allocator
classes, bank/ops/stack page offsets and false-alias frequency are **not measured** by sizeof.
The pure controls use no object/cache operations in their repeated body; ObjectData can affect
startup heap state, but not add a direct per-iteration object load. The larger executor stack
reservation and different spill locations can change stack/data aliasing even at equal counts.

Exact-image read-only LLDB capture was attempted: job `20261021T002148Z-t056-placement-exact-captures`
stopped on an unsupported SBLaunchInfo redirection method; corrected job
`20261021T002242Z-t056-placement-exact-captures-fixed` was refused by macOS:
"non-interactive debug session, cannot get permission to debug processes". No debuggee memory
capture was obtained. No system setting/privilege was changed. The corrected script is retained
for an interactive Developer Tools-authorized terminal; its results must not be invented from
an instrumented build with different allocation/stack behavior.

## Ranked hypotheses and discriminating evidence

1. **Interior code/branch layout and instruction mix.** Strongest existing lead: entry pinning
   leaves observed header/handler drift, and T050 demonstrates a mechanism of the necessary size.
   Equal retired counts do not fix instruction bytes, branch addresses, dependencies or micro-ops.
   A reciprocal one-function address move at identical normalized bodies isolates placement from
   heap/frame/source changes. A same-low-12-bit page move distinguishes some higher-address effects.
2. **Data-side placement/stack aliasing.** Bank/value/program dimensions stay fixed, but ObjectData
   allocation size, ShapeWays and the executor stack reservation change. A launch-only allocator /
   stack-cookie factorial keeps the exact image and code offsets fixed. Actual bank/ops/stack
   observations would be needed to identify a specific 4 KiB alias, rather than launch-state sensitivity.
3. **Different hot paths for strings and allocator cost.** Examine the exact-image profiles separately;
   common wall signs do not establish one shared cause. String allocation makes allocator probes
   particularly confounded unless instructions and execution paths are also reported.

The host is family 6/model 165/stepping 5, microcode 240 (i7-10700), L1I/L1D 32768 bytes,
cache line 64 bytes. Intel documents 32-byte branch/DSB boundary effects and relevant counters in
[its JCC guidance](https://www.intel.com/content/www/us/en/developer/articles/technical/software-security-guidance/best-practices/mitigation-strategies-jcc-microcode.html).
Its published affected-processor table does not list this Comet Lake-S model/stepping. Therefore
"JCC microcode causes this regression" is **not established**; no DSB/MITE/branch-miss PMCs were
collected. Boundary geometry is a hypothesis, not a hardware-counter diagnosis.

## Prospective diagnostic contract (before any new timing)

No residue sweep or favorable retry. All results, including incomplete/refused runs, remain retained.
These diagnostics cannot spend/reset attempts, advance stage A or replace its K=6/formal gates.

### A: isolated interior-placement transposition

A non-timing builder archives each immutable revision, verifies its exact canonical SHA, then
adds two never-executed linker NOP atoms around the sampled WideLoopFrame body in one constant
12,288-byte slot. No runtime source, recipe, benchmark or policy changes. Five links are fixed:

- b_ref: 4096 bytes before the base body; b_swp: 4080 (header residue 22 -> 6).
- c_ref: 4096 before the candidate body; c_swp: 4112 (header residue 6 -> 22).
- c_p4k: 8192 before the candidate body (same low-12-bit placement as c_ref, body +4096).

The 16-byte difference is derived from the two existing headers, not from timing. Every link
must retain every original boundary-correct normalized function body, the same function inventory/order, and the
recorded page residue. Within each implementation, **only the typed body's address may differ**;
all other original text symbols must have identical addresses. Unexpected drift stops timing.
Natural-versus-padded links record the three-page slot displacement plus the whole text-section shift; T054 means
this is not assumed neutral. b_nat/c_nat plus byte-identical b_cpy/c_cpy are controls: nine images.

Lead-only timing: 15 serial balanced rotating/reversing blocks, one warmed literal executable path,
all raw N/2N (whole input for external), output equality, positive increments and base increments
>=50ms. Thirteen cases fixed before timing: local_read, branch_arithmetic, global_read, property_dynamic_read,
string_slice, binary-trees, string-unpack-code, bitwise-and, class-fields, moment, diff, meriyah, nbody.
Budget 30 minutes; iMac ZoneInfo quiet-window/deadline wrapper retained. No outlier deletion.

Report paired medians and exploratory bootstrap intervals for both A/A controls, each padded/native
control, b_swp/b_ref, c_swp/c_ref, c_p4k/c_ref, and the candidate/base comparisons. Reciprocal effects
above A/A variability, with opposite signs and intervals excluding 1, support low-bit placement as
a contributor. Non-neutral padded/native results limit attribution to the isolated padded matrix;
they cannot prove that the original canonical excess is removed. A page effect without a residue
change rules out a simple "mod64 alone" explanation. Unstable A/A leaves the case unresolved.
Do not retain c_swp merely because local_read improves: inspect global/branch, strings and all gains.

### B: launch-state/data factorial, only if still needed

Use only exact canonical b0dfc65d/807867f6 files, no rebuild or code relocation. For each image
cross child-only MallocNanoZone=0/1 with a 64/128-byte inert environment cookie; keep TZ=UTC,
LC_ALL=C and key order fixed. Include byte-identical copies at zone1/cookie64 as A/A. Fifteen
serial balanced blocks, same warmed literal path and raw N/2N rules, cases local/branch/global,
string_slice and unpack, 30-minute bound. Every environment override is recorded; inherited
values are hashed rather than exposing secrets. No OS service/configuration is changed.

Allocator policy can change strings' executed work; report this instead of calling it equal-instruction
placement. The three allocation-free repeated scalar bodies are the primary discriminator.
Without actual pointers/PMCs, an effect establishes launch/data-state sensitivity, not a specific
store-forwarding or 4 KiB alias. If neither diagnostic explains local_read, do not spend the last
attempt: obtain finer native observations or investigate instruction dependencies/backend effects.

## Conditional general repair

If A establishes a useful robust effect, change the invariant to audit the actual dispatch header
and important branch blocks, not only entry residues, then validate one generic alignment policy
across the complete portfolio. Do not hardcode a winning benchmark offset. Compiler branch-padding
flags would change the frozen recipe and need an explicit new contract/amendment, not silent adoption.
If B implicates bank placement, consider an explicitly aligned scalar-bank allocation for every
admitted loop, while preserving indexed locals/upvalue cells and checking emitted hot bodies.
Neither proposed repair is production code until its causal test and cross-case validation pass.

## Boundary-audit correction before timing

Build `20261021T003035Z-t056-placement-fixed-slot-build` stops on its import path before compiling.
`20261021T003300Z-t056-placement-fixed-slot-build-fixed` reproduces b0dfc65d exactly, then refuses
b_ref on two apparent linear-disassembly differences. The base dispatcher ends in a Mach-O
DATA_IN_CODE kind-4 jump table at 0x10025f1d0..0x10025f30c, followed by four padding bytes and
publish_deferred_globals at 0x10025f310. Linear otool decodes table bytes as instructions and spans
into that next function: it loses the next push and changes a synthetic tail operand when NOP atoms
are inserted. It is not a runtime instruction change.

The diagnostic comparator retains the original linear reports, honors LC_DATA_IN_CODE, compares
jump-table/tail bytes literally, and starts code decoding at exact symbol boundaries (offline LLDB,
no process launch/permission required). Whole-prefix offline redecoding of the same immutable
b_nat and refused b_ref gives **zero** body differences. The optimized reader reuses valid linear
code prefixes, redecodes broken starts and retains literal data; it changes no repository formal
policy/producer. Fresh fixed-slot links still require every function body to match and every
non-target address within each implementation to remain fixed. No refused output is timing evidence.

Before the first timing submission, the diagnostic A list adds nbody (13 cases total), retaining
that earlier repaired numeric-field control. This is an explicit coverage addition to the initial
12-case prospective contract, with no timing data collected and no unit/policy change.

## Completed static matrix and smoke verification

Non-timing `20261021T004946Z-t056-placement-boundary-matrix` completes done 0 in 357s.
Identity SHA-256 is `bcb92039f209699493e7dac572a4d3dff9b58191302823014127a2d56f231021`.
All 4203 original base functions / 4225 candidate functions retain their boundary-correct code
prefixes and literal jump-table/tail bytes. Only the target address differs within either matrix;
all other original symbol addresses are identical. The extra assembly object moves the __text
section start by 4096, so canonical/padded comparison is explicitly a control, not assumed neutral.
The previously refused b_ref is byte-identical to the re-audited b_ref: fixing data/function boundary
decoding did not change or tune the binary. No repository formal producer/policy was edited.

| Link | Target address | SHA-256 |
| --- | --- | --- |
| b_ref | 0x10025dd90 | c0346d7800b34cec098277149fc976c199e11634b4641ce8efe2c35d17b9ff76 |
| b_swp | 0x10025dd80 | 5853b4880e2e6aecf698231e7e22c5d815ea768bbbe08fb53bbc87d8ef392298 |
| c_ref | 0x100008d90 | 2abcea0c0283b144f719da9340ccd557aa403cdfb236981e9d0c6b7f252e9c76 |
| c_swp | 0x100008da0 | 8641800307055fd4bf51b894578da1eefd83d1f15287411054efe0fc0a7c96c9 |
| c_p4k | 0x100009d90 | 39053d216ea71b58c7fc066a50c04803a6eb35ac5d9b63751d03e0cd11f3b989 |

Non-timing `20261021T005727Z-t056-placement-matrix-smoke`, done 0 in 163s, runs the 13 complete
diagnostic workloads on all seven exact files (two canonical, five links), with matching outputs.
Each of the five links separately passes all 255 NG fixtures. Smoke-ready SHA-256 is
`df79ea28d027e9a6977a390f3557006d639415f6bfa82fe28d4324eada205cb7`.
No benchmark wall timing was collected by either job, and no new full Test262 claim is made for
these diagnostic links. The canonical 4842f823 full configured scan remains the standing result.

### String attribution and other concrete checks

Exact-image non-timing profiles `20261021T002551Z-t056-placement-string-profiles`, done 0 in 51s,
use internal N=100,000,000 and 40 original unpack passes only for sampling. Main-thread sample
counts: string_slice base 6692/candidate 6701; unpack 3216/3264. Reported allocator self samples
are 3053/3123 (45.6%/46.6%) for slice and 1431/1482 (44.5%/45.4%) for unpack. The typed body itself
has 738/759 self samples for slice (11.0%/11.3%); no typed-dispatch self symbol reaches the five-sample
report threshold for unpack. This supports checking allocator/native string work independently;
it does not prove a shared scalar-loop cause or quantify a speed difference from sample counts.

The wide activation's own stack reservation is unchanged at 0x388 in both images. In the numeric
Binary fast paths, both use r8 for the tag-combine temporary and restore it once from stack;
the bank stays live in r9 (base) / r11 (candidate). Thus another bank-pointer indirection is not an
observed explanation in 4842f823. Instruction dependencies/encoding/backend behavior remain a
fallback hypothesis if isolated placement and launch-state factors do not explain the excess.

### Handoff and stopping rule

Workspace packet `tmp/t056-m1a-placement/probes/` contains the two exact self-contained commands,
collector, fixed configs, ZoneInfo guard, static validator and hashes. It binds both completed
static/smoke receipts, every image SHA, immutable production inputs and the recorded protocol.
A is 13 cases/nine images/15 serial blocks; B is five cases/ten launch conditions/15 blocks and
requires a complete same-binding A job plus manual review before submission. Each has a 30-minute
budget and 45-minute quiet-window headroom, all raw samples and output checks. The package renders
but never submits jobs. The owner has submitted **zero new timing jobs**.

The last attempt stays unspent. No production fix has been applied: the evidence now discriminates
entry-versus-interior pinning and array dimensions, but cannot choose between code placement,
actual data/stack placement, backend dependencies and measurement-context interactions without
those probes. If native C/B does not reproduce serially, investigate schedule/path interactions;
non-reproduction is not a dismissal of the valid four-worker rejection/triage.
