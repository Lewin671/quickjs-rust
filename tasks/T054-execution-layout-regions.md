# T054: Reference-preserving execution regions

- Status: active.
- Implementation: workspace-only prototype; no production build change yet.
- Verified at: base 9a0eeeca, executable 926ee634; T050 isolated placement and
  rejected T053 independently expose unchanged interpreter-loop regressions.
- Evidence: the contract below; no build or timing of this strategy yet.
- Unresolved: exact prototype correctness and frozen neutrality screen.
- Next action: collect prototype correctness and its one complete neutrality
  screen; only a pass permits formal comparison.

## Architecture and scope

This is a generic code-layout maintenance mechanism, not a reopened T053
screen. Runtime code remains exactly 9a0eeeca. All rejected unit budgets stay
closed. No JS program identity, source shape or data selects an optimization.

Protect the existing general execution drivers as linker regions: ordinary VM
activation, wide activation, typed-loop Vm/WideLoopFrame bodies, compact numeric
call-tree driver, and numeric helper-graph driver. Resolve their exact names
from the measured reference binary; do not guess an ambiguous body. Their
start residues and their following-function residues modulo 4096 come only
from that reference, never from a timing scan.

Each boundary uses a private, never-called, retained padding atom aligned to
4096, with positive NOP byte length equal to the desired residue (4096 when
zero). Compile ONE assembly object per atom: putting several aligned labels
in one object would attach alignment bytes to the previous atom and invalidate
its requested length. Interleave the atoms around each driver in the full
natural order. This preserves following code residues even when a driver
changes size. It does not promise identical absolute addresses or hot-code
invariance if the driver's own instructions change.

Prototype allowed paths: crates/qjs-cli/build.rs and the x86 order; temporary
producer/analysis scripts stay in workspace tmp. No runtime/AST/parser,
reference source, workloads, CPU feature flags or performance gate changes.
Only x86_64 macOS release builds need the assembly; other platforms keep their
normal build path. Rust calls none of these atoms: no FFI API, runtime unsafe,
threads or new Cargo dependencies. Record binary-size growth as a cost.

## Frozen experiment and acceptance

- One strategy/offset policy; no alternative residues or favorable-run search.
- Build the original runtime using the canonical fat/16/abort recipe.
- Before timing, all surviving original raw text-function start residues must
  equal the reference, all requested execution roles must remain present, and
  nonmatching names must be reported. Unexpected static drift stops timing.
- Check full correctness and the exact optimized CLI before measuring.
- One complete 76-case, five-pair diagnostic: every median <=1.03, each lane
  geometric mean <=1.005 relative to the original 926ee634 binary. Retain all
  pairs. Only a pass justifies a 30-block full comparison under the same limits.
- Preserve all cases/outputs and record disturbance; invalid host runs are
  retained/replaced, never pooled. No runtime speedup is claimed by maintenance.
- If neutrality fails, restore the prototype; do not adjust offsets from its
  case results. If it qualifies, any future runtime cache work still needs new
  exact-binary profiles, a new frozen plan, correctness and full promotion.

## Ownership and verification

Owner branch agent/execution-layout-regions/root; worktree under the shared
workspace worktrees/quickjs-rust/execution-layout-regions. Host owns the
prototype/integration and iMac queue; Command Code's T052 tooling is separate.
Run context/fast/full checks appropriate to retained code. Prototype static
results alone cannot publish a production layout or promote a runtime change.

## Current prototype evidence

Plan committed at 5b72febf before producing any layout binary. Job
20261016T142226Z-execution-regions-prototype rebuilt the unchanged runtime:
all 4205 original functions survive with their page residues unchanged,
including all six requested execution roles; zero static drift. Binary
0a331beb69c236c4c0b40df22dc6a5aaae78be3b1d01c03d2dd5c9821b88845d,
6131524 bytes versus base 6081812 (+49712 bytes). Correctness is pending.
Job 20261016T142632Z-execution-regions-neutrality is queued and refuses to
measure if prototype correctness fails. No score or neutrality claim yet.
The build implementation remains in the workspace producer and a disposable
iMac worktree; only this contract is retained until it qualifies.
