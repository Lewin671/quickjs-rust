# T055: Layout noise control, phase A2

- Status: active; lead-amended A2 protocol frozen before timing.
- Implementation: none in production; diagnostic R0 binaries already audited.
- Verified at: source 153c1fe5, phase A plan 82a21c62; static audit and exact
  natural-binary correctness passed. A2 starts from repository 09e8e0d3.
- Evidence: [phase A results](archive/T055-phase-a-log.md), frozen A2 below.
- Unresolved: complete A/A and five layout screens; data-derived B proposal.
- Next action: verify all six R0 hashes, run A/A then v1..v5 through the iMac
  queue, record all results and write the workspace-only phase B proposal.

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


## Lead amendment and frozen phase A2 (2026-10-16)

The lead's brief-phase-a2.md supersedes phase A's sampling and R1 decision
above. R1 is dropped: mitigation not available via this toolchain on macOS;
hardware hypothesis untested. No further JCC work. The fixed-path binary-
replacement sampler is retired; do not debug or reuse it. Phase A's invalid
partial samples do not enter A2. Historical results are in the linked log.

Reuse the six already audited R0 executables (source 153c1fe5), verifying
SHA-256 against the phase A report before any measurement. Rebuild only if
the originals cannot be verified. Each retains its existing immutable,
equal-length path under the build job's out/binaries/R0v0..R0v5/qjs;
never swap or overwrite them. New repository revision is a tooling context,
not a claim that these executables were built from it.

Exactly six complete sequential screens: v0/v0 A/A, then v1/v0 through
v5/v0. Each uses T054's tools.benchmark.screen CLI, all 76 original cases
(25 broad, 6 sentinel, 45 external), five alternating pairs, unchanged
calibration, launch preflight, quiet checks, timeout and measurement lock.
Five pairs matches T054; this is a diagnostic band, not formal precision.
Use workspace scripts/imac-bench.sh run --timing for each complete screen;
retain/replace DISTURBED runs in full, never pool or remove observations.
No held-out programs are used to select or profile work in this diagnostic.

A workspace wrapper only records the existing screen.sample return values
and case boundaries outside timed subprocesses, preserving wall/instruction
costs as well as the CLI's ratios; it changes no tools or sampler decisions.
Require all five positive wall/instruction pairs per case; missing/short or
nonpositive increments are reported as invalid, never silently discarded.
Keep calibration samples separate. All raw samples, driver, identities,
CLI JSON and logs are retained. A/A must complete validly before v1 timing.

Per case report every paired wall/instruction median ratio vk/v0. For a
cross-screen layout spread normalize each role's cost by iterations (internal
only), divide each candidate's median cost by its same-screen base median,
include v0=1, and take max/min over v0..v5. Also report the literal unnormalized
six wall medians using the v0 role from A/A, flagging their cross-run drift;
these cannot separate host drift from layout. A/A width is max/min of its
five ratios, with min/max and median bias reported for both metrics. Per
lane and portfolio report geometric means of case paired-median ratios per
variant and their max/min spread; retain all cases with equal case weights.

Phase B is a proposal only at workspace tmp/layout-noise-control/
PROPOSAL-phase-b.md. Derive thresholds from A/A and the layout band, assess
T053/T054 using their recorded pairs, cover instruction-blind effects and
reserve held-out acceptance. Do not change production tools, workflow docs,
rejected budgets or acceptance policy. Commit on this branch only; no push
or merge (private queue transport refs are the existing authorized runner).
