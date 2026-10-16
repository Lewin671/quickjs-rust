# T055: Layout noise control, phase A

- Status: blocked; phase A timing is incomplete and invalid.
- Implementation: none in production; workspace-only diagnostic tools/binaries.
- Verified at: source 153c1fe5, plan 82a21c62, 2026-10-16; full gate,
  static audit, both exact natural binaries' 5170 Test262 cases and 247 NG
  fixtures passed. No complete timing matrix was obtained.
- Evidence: frozen design and revision-bound results below; supplementary
  workspace report: tmp/layout-noise-control/REPORT-phase-a.md.
- Unresolved: valid 76-case layout band; R1 produced no machine-code change;
  frozen N/2N wall sampling failed despite tiny-launch preflight.
- Next action: lead decides whether to freeze a replacement sampling protocol
  after A/A validation and worker-failure handling; no phase B work or further
  timing under this invalid run.

## Scope and frozen recipes

Owner: agent/layout-noise-control/codex in its existing worktree; no push to
origin, merge, main/other-owner changes, runtime/parser/AST/workload changes,
production recipe adoption or phase B implementation. Queue transport refs
are used only by the required workspace scripts/imac-bench.sh runner.
Only this task and its tasks/README.md index entry are repository changes.
Runtime/build inputs start at exactly 153c1fe5; temporary CLI build glue and
linker-order inputs generate diagnostic placement variants outside the repo.

R0 is T047/T046's exact canonical recipe: opt-level 3, fat LTO, 16 codegen
units, panic abort, agents, explicit x86_64-apple-darwin target, incremental
0, target-default CPU, Rust flags in order -Csymbol-mangling-version=v0 then
-Cllvm-args=-align-all-functions=4. R1 appends exactly
-Cllvm-args=-x86-branches-within-32B-boundaries. No R2.

## Frozen placement generator and static stop

Six variants per recipe: v0 is the natural canonical build; v1..v5 insert
one never-called retained NOP atom of respectively 16, 64, 240, 512 and
1008 bytes before the entire natural text-function order, retaining every
original function in that order. Atom alignment is 16 bytes. The order is
obtained independently from each recipe's v0, without looking at timing.
This samples both 32-byte phases and several cache-line/page residues using
one generic rule, without matching benchmark source or bytecode.

Build glue may assemble/link the padding and replay the natural order only;
all runtime inputs and compiler settings within a recipe remain identical.
Before timing, audit every original text symbol: names/order, function sizes,
actual address shifts and disassembled instructions normalized only for
relocated branch/RIP references. All original bodies must match their own
recipe's v0; added NOP atom and final section alignment are the only allowed
new instructions. Unexpected body/name/order/relative-address drift stops
all timing and is reported, rather than repaired from performance results.
Different code generation between R0 and R1 is expected and reported.

## Frozen measurement and precision

Use the iMac only via workspace scripts/imac-bench.sh; inspect status before
CPU work. Build/check jobs use the shared queue. Timing uses run --timing,
existing measurement lock, four workers partitioning cases (one worker owns
each case), wall time, all 76 unmodified cases: 25 broad, 6 sentinel, 45
external. No iMac build/test work during timing. Preserve DISTURBED runs and
replace the entire disturbed run, never pool them.

Before timing, run existing exact-binary correctness checks on v0 of each
recipe: optimized 5170-case Test262 subset and every QuickJS-NG comparison
fixture, UTC/C locale, using the pinned reference inputs. Static audit covers
all other variants. Record full hashes, sizes, toolchain, commands and scripts.

Collect 60 rounds per variant per case, chosen conservatively against the
historical ~0.8% paired wall noise and the known less stable audio case. Each
round interleaves all 12 binaries: rotate the list R0v0..R0v5,R1v0..R1v5 by
round modulo 12, reverse odd rounds. One literal executable path per worker,
atomically replaced outside timing and launch-preflight warmed after each
replacement (T050); argv/environment/workload paths remain fixed. Calibrate
internal N/2N with existing screen calibration, target 200 ms and require
all measured increments >=50 ms; external cases use whole-process wall time.
Keep all N/2N/whole raw samples and checksums; remove no outliers. Report
bootstrap 95% median intervals (10000 resamples, fixed seed 5500); flag any
variant whose relative interval half-width exceeds 1%. Insufficient precision
is a limitation, not permission for a favorable rerun or changed design.

## Frozen metrics and reading

For each case/recipe report max/min of the six variant medians, sorted by
spread. For each case take the geometric mean of its six variant medians;
report its R1/R0 ratio and aggregate ratios geometrically per lane and overall.
Also report each individual variant's lane geomean against R0v0.

R1 reduces layout noise only if the median per-case spread excess above 1
falls by at least half, its worst case spread is below 1.03, and no lane's
R1/R0 geomean exceeds 1.01. ("Half" applies to max/min minus one, since the
ratio itself cannot drop below one.) Otherwise report the observed band for
phase B. This diagnostic neither promotes runtime code nor reopens rejected
unit budgets. No offset search, extra recipe or post-result tuning.

## Phase A results and blocker (2026-10-16)

Frozen plan commit: 82a21c6219977b63659f30131e5098d102bf02a1; the design
above is unchanged. All runtime/build inputs start at the recorded source.
No production change, origin push, merge or phase B implementation occurred.

Jobs (workspace outputs/imac-bench/<job>/):

- 20261016T150418Z-layout-noise-matrix-build: exit 1 before timing; the first
  normalizer mistook integer immediates/embedded-data disassembly for relocated
  addresses. Its first identity file is retained. Corrected address recognition
  does not change recipes, padding, ordering or statistical rules.
- 20261016T150643Z-layout-noise-full-gate: full check at 82a21c62 passed, 170 s.
- 20261016T150830Z-layout-noise-matrix-audit-fixed: exit 0, 734 s. All 12
  binaries built; each recipe's 4205 original functions survive, and each of
  its five variants has exactly the declared uniform displacement, original
  order/sizes and relocation-normalized instructions. Zero real static drift.
  R0v0/R1v0 each pass 5170 subset cases and all 247 NG fixtures, UTC/C locale.
- 20261016T151909Z-layout-noise-loaded-metadata: exit 0; all six corresponding
  R0/R1 entire text sections are byte-identical at identical addresses.
- 20261016T152218Z-layout-noise-matrix-timing: cancelled (130) after three
  workers hit the frozen minimum-increment guard. No complete case/matrix.

The iMac is i7-10700 @ 2.90GHz, macOS 12.6 (21G115). R0v0 is the canonical
6081812-byte binary, SHA-256
926ee634ca3901b89c778478702a4fe993288ef7e8e69cd93c238c694595aa85.
R1v0 has the same size, SHA-256
543884ecf407d1a831e052bcd919f236a0a66f4c67b2baa229be76a16f20781e.
Both natural text sections are 4916832 bytes, SHA-256
6c2e7c07cb9ccd2f4a4b2c2c68358a2c844afee215b9ce6c51bd8d15228fc85b.
All six padded pairs also have identical text; full hashes/sizes are in the
workspace report. Cargo's runtime fingerprint confirms all three R1 flags.
Thus R1 is a machine-code no-op here; the JCC hardware hypothesis remains
untested, and no mitigation is adopted.

| Case | Round (zero-based) | Variant | N-to-2N wall ms |
| --- | ---: | --- | ---: |
| broad/plain_function_call | 1 | R0v4 | -56.458329 |
| broad/captured_write | 4 | R0v2 | -190.853849 |
| broad/method_call | 6 | R0v1 | -162.518805 |

Every offending execution followed a successful 80-84 ms tiny-launch warmup.
For plain_function_call, N wall/user time is 633.976/290 ms and 2N is
577.518/560 ms: extra non-user time corrupts the wall difference. Its specific
system/validation/wait source is not resolved. Worker exceptions were delayed
by executor shutdown waiting for the surviving worker; the queue job was
cancelled after raw-data diagnosis and no child process remained.

Retained partial rounds: plain_function_call 2, method_call 7, captured_write 5,
captured_read 15, each below 60; 0/76 cases complete. All original samples and
checksums remain; none were removed, pooled, or used for a spread/CI/verdict.
The cancelled queue job has no final disturbance classification (observed
intruder samples total at most 82% CPU, below its 200% threshold).

Per-case spreads, all lane geomeans, precision and the predeclared noise-
reduction reading are unavailable. Complete artifact/table inventory and
blocking evidence: workspace tmp/layout-noise-control/REPORT-phase-a.md and
outputs/layout-noise-control/blocked-result.json. These are supplementary;
the conclusions, numbers and revisions are recorded here.

The brief forbids changing the frozen design after seeing results. Obtaining
a valid band now requires a newly frozen sampling/path-warmup protocol and
prompt worker-error termination, first checked by A/A. That decision belongs
to the lead; no replacement timing, additional recipe or favorable rerun was
silently substituted. Phase A remains blocked, rather than claimed complete.
