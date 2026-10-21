# T056 ops confirmation and JCC compiler-policy diagnosis

- Status: closed diagnostic; no production or acceptance change.
- Implementation: independent flagged build copies and guarded prospective timing
  protocol in [t056-jcc](../diagnostics/t056-jcc/README.md). Timing is **not ready**.
- Verified at: engine source dfff3bb6, diagnostic parent e2b390b5; iMac jobs
  20261021T140336Z build done0/334s and 141235Z audit done0/12s, 2026-10-21.
- Evidence: [ops result](../diagnostics/t056-jcc/ops-confirmation.json),
  [original geometry](../diagnostics/t056-jcc/original-geometry.json),
  [images](../diagnostics/t056-jcc/images.json),
  [audit](../diagnostics/t056-jcc/audit.json),
  [pinned compiler source](../diagnostics/t056-jcc/toolchain-source.json).
- Unresolved: no DSB/MITE events or demonstrated JCC contribution; requested
  LLVM flag leaves all four executable text sections byte-identical.
- Next action: qualify a general Mach-O-capable branch-alignment implementation
  before timing it. Do not run the prepared job with these ineffective images.
  One valid attempt remains across A+B; no owner timing or production edits.

## Ops confirmation: retain formal verdict and observed contrast

Lead job `20261021T135119Z-t056-ops-confirmation` completed done0/174.017s:
170 units, 340 measured executions, 170 preflights, all outputs agree. Original
assessment remains `inconclusive: neutrality/AA/anchor failed; no selection or retry`;
it is not rewritten into a successful formal causal test.

| Source | global600/900 median | local600/900 median |
| --- | ---: | ---: |
| count | 0.993780 | 1.000799 |
| flags | 0.991159 | 0.999145 |
| count_flags | 0.998892 | 0.998632 |

The lead rejects sampled ops600/900 as the explanation: neither restoration nor
reproduction occurred, and slow count_flags remains slow in both phases. This
does not logically exclude every data-side mechanism. The two relative-PC
families survive this probe; the causal hardware mechanism remains unknown.
All contrasts, CIs, sham controls, AA and raw receipt hashes are retained above.

## Intel scope and live CPU

Actual build-job sysctl: i7-10700, family6/model165 (`0xA5`), stepping5,
signature656981 (`0xA0655`), microcode240 (`0xF0`). Read-only; no microcode,
security, PMU access or machine settings changed.

Intel's [JCC guidance](https://www.intel.com/content/www/us/en/developer/articles/technical/software-security-guidance/best-practices/mitigation-strategies-jcc-microcode.html)
describes DSB misses and switches to legacy decode for jumps crossing **or
ending** on32B, including fused pairs. Its affected-processor table names
Comet Lake U42/8E and U62/A6, not this A5 desktop model. The
[white paper341810-001](https://cdrdv2-public.intel.com/841076/mitigations-jump-conditional-code-erratum.pdf)
§2.1 includes conditional/unconditional/indirect jumps, calls and returns;
§4 has the same model distinction. A generic “10th generation” support label
does not establish A5 coverage. Absence from these tables is not an independent
proof that a given microcode implementation is unaffected.

Intel reports0–4% on surveyed full benchmarks and higher subcomponent outliers;
it does not establish a typical10–40% loss for this loop. No observed CPU event
supports DSB/MITE attribution here. The previous limited JCC conclusion stands.

## Exact152 path: both32B phases contain boundary jumps

Every decoded branch is listed with PC, relativePC, length, candidate pair start,
start/end residue, crossing and exact-end flags in the30 indexed records.
End is exclusive: `floor(start/32) != floor((branch_pc+length)/32)`.
Pairs require physically adjacent CMP/TEST+Jcc. These are candidate macro-fusion
spans, not measured fused-uop or cache-residency claims. No hot calls/returns.

| Image | K0 | K16 | K64 | K240 | K512 | K1008 |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| 8f / count / flags | 5 | 5 | 5 | 5 | 5 | 5 |
| count_flags | 4 | 6 | 4 | 6 | 4 | 6 |
| dfff | 6 | 4 | 6 | 4 | 6 | 4 |

Counts are dynamic occurrences; a shared numeric-handler PC executes twice.
Unique counts are4/4 for fast,3/5 for count_flags,5/3 for dfff. The “fast has
none/fewer at both phases” prediction fails. dfff and8f even share the natural
numeric-tag crossing at relativePC0x66c. Boundary presence alone cannot explain
the speed classes; distribution/residency could still matter without this erratum.

dfff K=6 records use the real prior audited uniform shifts and image hashes from
`20261021T082252Z-t056-code-layout-qualification`. Whole-page shifts do not alter
32B geometry. Other four images' shifted records are explicitly **arithmetic
translations** of exact original decoded PCs; those images were not built or
timed at six offsets. At offset0 their underlying paths are real image decodes.
The cross job `20261021T085751Z-t056-code-layout-cross` used f58/dfff, not8f:
dfff code1 is the same audited+16 image and has the K16 list for both zone
settings; code0 has K0. f58 is a different body; [cross-geometry.json](../diagnostics/t056-jcc/cross-geometry.json)
lists all four actual code images for both zone cells, using its byte-identical
flagged decode and the original uniform-shift audits. No invented8f cross result.

## Requested flag: accepted, but ineffective on this Mach-O build

All four sources were rebuilt in separate fresh source archives and target
directories with the canonical fat-LTO/16-CGU/abort recipe plus only
`-Cllvm-args=-x86-branches-within-32B-boundaries`. Count_flags source uses the
already sealed header-factor file hashes on8f. Canonical cache is untouched.
Actual job toolchain is repository-pinned rustc1.95.0/LLVM22.1.2. An ambient
SSH rustc outside the repository reports1.99.0; it was **not** the builder.
LLVM help accepts the option; Cargo fingerprints include the extra flag.
The raw producer's `mitigated` boolean means flag requested, not audit qualified.

| Source | Original SHA prefix | Flag SHA prefix | File bytes, either | __text bytes, either | Hot affected, flag |
| --- | --- | --- | ---: | ---: | ---: |
| f58 | b0dfc65d | ba6c0d66 | 5,983,148 | 4,821,792 | 4 |
| 8f | b0b84489 | ed78c675 | 6,068,028 | 4,890,864 | 5 |
| dfff | 3466c37a | 7f9343a4 | 5,986,812 | 4,808,784 | 6 |
| count_flags | 52ed87e9 | fdf37941 | 5,986,788 | 4,819,696 | 4 |

Full hashes are in the image manifest. Every pair's **entire __text bytes and
SHA-256 are identical**, despite different whole-file hashes. All flagged hot
paths remain152 instructions, zero added NOPs; no branch was repaired.
All eight images pass NG255 and global/local/branch/heterogeneous N+2N outputs,
whole unpack, and empty output. This is not full flagged-image Test262 standing.

Audit job `20261021T140906Z-t056-jcc-audit` first failed on base's nonzero count;
the complete audit then retained all four negative results, rather than dropping
the failing condition or advertising a zero-boundary image. Current collector
still requires zero affected branches and refuses these images before costs.

Pinned rustc commit59807616's LLVM submodule is
`1cb4e3833c1919c2e6fb579a23ac0e2b22587b7e`. Its
[X86AsmBackend.cpp](https://github.com/rust-lang/llvm-project/blob/1cb4e3833c1919c2e6fb579a23ac0e2b22587b7e/llvm/lib/Target/X86/MCTargetDesc/X86AsmBackend.cpp)
routes the padding hook through X86ELFStreamer; its
[Mach-O streamer](https://github.com/rust-lang/llvm-project/blob/1cb4e3833c1919c2e6fb579a23ac0e2b22587b7e/llvm/lib/MC/MCMachOStreamer.cpp)
inherits generic instruction emission without that override. This explains the
observed ineffective flag; source URLs, hashes and line excerpts are sealed.
Equivalent boundary/type options configure that same hook, so do not silently
label an equivalent spelling a remedy. No toolchain/backend patch or alternative
assembler pipeline is introduced in this diagnostic.

## Prepared, deliberately blocked, ≤15min protocol

Four sources +/-flag plus four independent-inode AA clones, six cases, five
shared paired blocks:360 units,600 measured executions+360 preflights. One active
pathname, rotate/reverse order, fixed previous iteration counts, no row deletion.
Report flag/original for each source, both original-base and matched-flagged-base
ratios, recipe interaction and AA. Rules are prospective in protocol.py;
suppression would establish compiler-layout sensitivity, not a JCC event cause.

All selected previous mini case windows total507.873s/720 units; current360
projects253.936s. Twofold reserve+120s=627.873s<720 collection,840 body,890 runner
start cutoff,900 watchdog including quieting. Host-local03–05 plus15min before03
excluded;20min admission headroom. Hard timeout remains authoritative if changed
code is unexpectedly slower. No owner timing was submitted.

Actual config and raw audit both block execution. Mock success uses an isolated,
explicitly synthetic zero-boundary receipt, never the real evidence. Validation
also proves current actual config launches zero processes; failure paths and all
960 simulated calls are checked. Requalification is needed before lead submission.

The flagged base is additional diagnostic data, not a replacement inside frozen
plan SHA-256d184dbcc… . A future effective policy needs a new recipe identity,
matched rebuilt bases, policy/band qualification and prospective lead-approved
contract before acceptance. Historical evidence and the single remaining attempt
cannot be reset or relabelled by changing the base image. No current recipe,
frozen plan, threshold, attempt record or engine source is modified.

## Repository verification

Diagnostic validation passes:8 real NG255 receipts, eight workload-output sets,
all raw image/text hashes,30 original geometry lists plus four actual cross
images, four152-path audits and the complete mock collector/failure suite.
On this diagnostic working tree (engine stilldfff3bb6), `./scripts/check.sh`
passed, including5170 curated Test262 cases; staged
`./scripts/check-touched.sh --staged --explain` and `check-context.py` passed.
Fast gate selects context checks for these task/diagnostic paths; no engine
semantics changed. These checks do not confer full Test262 or performance
acceptance on the flagged copies.
