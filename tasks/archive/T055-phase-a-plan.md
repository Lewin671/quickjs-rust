# T055: Superseded frozen phase A contract

Frozen at 82a21c6219977b63659f30131e5098d102bf02a1. Preserved verbatim
from the owning task; the later lead amendment and A2 protocol supersede
its R1 work and sampling. This is historical evidence, not current procedure.

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


## Current procedure

Follow the lead-amended [T055 protocol](../T055-layout-noise-control.md).
