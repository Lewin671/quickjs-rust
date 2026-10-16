# T046: Compiler configuration study at preview.6

- Status: closed-landed.
- Implementation: none in the production recipe; the compiler study is complete.
- Verified at: runtime `4f6f82cc582febf3fcb2011af386257dea9612de`,
  2026-10-16; 30-pair confirmation, exact-binary semantic checks and reproduction.
- Evidence: revision-bound results and hashes below. Raw artifacts are local,
  not retained in this repository (`outputs/quickjs-compiler-study/` in the workspace).
- Unresolved: cross-platform performance and build-specific order-file matching.
- Next action: none for this study. Applying the recipe and repairing order-file
  resolution are separate changes requiring their own verification.

## Recommendation and boundary

For the measured macOS x86_64 CLI, retain optimization level 3, use fat LTO,
16 codegen units and panic abort, with the target-default CPU. Do not add the
PGO profiles or native CPU tuning tested here. The exact measured recipe also
fixes feature `agents`, an explicit `x86_64-apple-darwin` target and Rust flags
in this order: `-Csymbol-mangling-version=v0`, then
`-Cllvm-args=-align-all-functions=4`. Use the full recipe when reproducing;
equivalent-looking defaults did not produce byte-identical executables.

This is a standalone CLI recommendation. Rust panic abort terminates the
process; JavaScript exceptions still use the engine's error model. An embedding
host owns its own Cargo panic strategy. No runtime, parser, workload, dependency,
release workflow or production Cargo configuration was changed by this study.

Host: Intel i7-10700, 8 cores/16 threads, 32 GiB, macOS 12.6 (21G115).
Toolchain: rustc 1.95.0 (59807616e), cargo 1.95.0, LLVM 22.1.2.
The results do not establish Linux or Apple Silicon performance, nor the
performance of GitHub's cross-built binary on this host. The final reference
was rebuilt locally from the existing release command.

## Final confirmation

All recipes execute the same 25 broad cases, six dynamic sentinels and 45
pinned external programs. Every case has 30 new paired observations with
alternating A/B and B/A order; all samples and stdout/checksum checks are kept.
Four qualified workers own whole cases. Builds do not overlap measurements.
Internal confirmation counts are four times baseline calibration, capped at
134217728; whole-process external inputs are unchanged. Wall time decides:
this host's cycle counter is not a clock. Instructions are supplemental.

Ratios below are candidate/reference wall time; lower is faster. Each lane
uses the geometric mean of its case ratios. Lanes are not pooled into a score.
The reference uses `cargo build --locked --release -p qjs-cli --features agents
--target x86_64-apple-darwin`, with repository flags and no profile overrides.

| Recipe | Broad | Dynamic | External | SunSpider | Kraken | JetStream subset |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| fat/16 + abort | 0.901393 | 0.951534 | 0.926525 | 0.922033 | 0.936512 | 0.922143 |
| fat/1 + abort | 0.875730 | 0.946498 | 0.936755 | 0.932559 | 0.944029 | 0.938338 |
| fat/16 + abort + native | 0.916753 | 0.957194 | 0.923725 | 0.919091 | 0.939237 | 0.905003 |

The recommended fat/16 recipe's worst point estimate is 1.024079. The largest
individual 95% interval upper bound is 1.029515 (`controlflow-recursive`), so
all cases meet the study's 5% regression constraint. Intervals use paired log
ratios and are not adjusted for multiple comparisons.

Fat/1 loses `controlflow-recursive`: 1.089190, 95% interval
[1.076686, 1.101839]. Native loses `imaging-gaussian-blur`: 1.105289,
[1.090534, 1.120244]. Their aggregate wins do not waive those regressions.

Fat/16 startup/evaluate-zero ratio is 0.954785, 95% interval
[0.937415, 0.972477]. Its binary is 6,081,812 bytes versus reference
7,648,212 bytes (20.5% smaller). Observed builds took 75.1 and 43.9 seconds;
these are cost observations, not a controlled compilation-time benchmark.

## Exploration and PGO

The initial 15 recipes cover default release; ThinLTO at 16/8/4/1 units; fat
LTO at 16/1; local LTO at 8/4/1; optimization level 2; abort; native CPU;
compiler-default function alignment; and 32-byte alignment. After abort led
that screen, exactly three interactions were frozen: fat/1+abort,
fat/16+abort and thin/16+abort. Screens use three paired observations and
per-case medians, then lane geometric means. All 76 cases remain visible.
The initial explicit-default reference is distinct from the final release
reference; its results were not relabelled as final comparisons.

PGO trains only on 5170 curated Test262 cases and an independently authored
semantic workload. No evaluation benchmark source enters training. Generic
and native CPU recipes train separately. The generic merged profile contains
4549 functions. The full build logged 2809 missing-profile warnings, including
engine functions and duplicates; 44 was only the final CLI subtotal. The
typed-loop property helper `get_named_object` is absent from the sparse merged
profile. No instruction-level attribution of the regressions was established. Both normal PGO binaries pass correctness but
fail screen regression constraints: generic PGO median ratios include
1.569 on `array_dynamic_read` and 1.472 on `audio-dft`; native PGO includes
1.427 and 1.342. Neither spends a final 30-pair run. This rejects these
profiles at this revision, not PGO as a general technique.

All six semantic candidates pass 5170 curated Test262 cases and every NG
comparison fixture: both fat/abort recipes, generic PGO, the native combination,
native PGO and standalone native. Full configured conformance of these new
binaries was not rescanned; the configured full CI result at the runtime base
is a separate source-level record.

## Order-file audit and adoption work

The checked-in x86_64 order file has 625 entries. The locally rebuilt release
matches only 50; none of its 541 runtime entries or 16 budget entries match.
Its VM/wide executor page offsets are 0x300/0xac0 rather than the tool's
requested 0xc40/0x400. The tested candidates likewise require build-specific
symbol matching. This was audited, not repaired or credited as a speedup here.

Before adopting a production recipe, unify release and performance build
inputs, including features, explicit target, ordered Rust flags, LTO, codegen
units and panic strategy. The hosted preview currently forces the old recipe;
editing only Cargo.toml would not update that preview. Generate/validate the
order file for the actual build recipe and remeasure after layout changes.
Cross-platform rollout needs separate measurements.

## Reproduction and evidence identity

Reference SHA-256:
`fe203546f3def819109389af383aa6cc1e3f3b6e4509ea40eb1f41cbc9559654`.
Recommended binary SHA-256:
`926ee634ca3901b89c778478702a4fe993288ef7e8e69cd93c238c694595aa85`.
The reproduction helper reran the complete recipe and matched this binary hash.
Raw case data, build receipts, merged profiles, binary artifacts and the
reproduction helper are preserved with the local report; conclusions above
stand independently of those local paths. This is compiler-recipe evidence,
not a sealed T018 promotion decision or fixed-hardware qualification.

Preflight history is retained: unequal-length role paths were replaced before
selection; differential N-to-2N timing was abandoned after a non-positive
increment; a fresh ten-pair whole-process A/A produced broad/external ratios
1.00009/1.00010 (largest case point 1.01624). A missing custom-training path
was fixed and PGO retrained from a clean profile directory before timing.
Abandoned attempts do not enter ranking. The completed whole-process timed jobs had no DISTURBED result. The matrix
and final confirmation jobs each recorded one light-background sample below
the disturbance threshold. No sample was trimmed or silently replaced.
