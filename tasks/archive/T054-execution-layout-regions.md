# T054: Reference-preserving execution regions

- Status: closed-rejected; the single complete neutrality screen failed.
- Implementation: none retained; workspace prototype only, never integrated.
  Production runtime/build implementation remain unchanged from main 153c1fe5.
- Verified at: frozen plan 5b72febf, prototype 0a331beb (full SHA below);
  actual staged fast/full, optimized Test262 subset and all NG fixtures pass.
- Evidence: frozen contract and complete result below; raw workspace-local
  outputs are retained separately, not repository promotion evidence.
- Unresolved: none for this strategy; preserving page residues did not qualify
  neutral maintenance. This does not identify a single causal predictor/cache.
- Next action: none for this experiment; any new structural approach needs
  an independently frozen contract. T053's exhausted budget remains closed.

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
6131524 bytes versus base 6081812 (+49712 bytes, about 0.82%). The initial
prototype job failed correctness preparation because its disposable worktree
lacked Test262 sources; no timing occurred. Job
20261016T142632Z-execution-regions-neutrality refused before measurement.
Preparation failures and original logs remain retained. Reusing this exact
binary, 20261016T143849Z-execution-regions-correctness passed the actual staged
fast/full gates, optimized 5170-case subset and all 247 NG fixtures with fixed
UTC/C locale and existing read-only reference sources. Its temporary source
worktree was restored and removed normally.

## Sole neutrality screen and rejection

20261016T144029Z-execution-regions-neutrality-valid completed all 76 cases,
five wall/instruction pairs each, no DISTURBED warning. One light background
sample was below the queue disturbance threshold and remains in the log.
Base 926ee634 and candidate 0a331beb are immutable equal-length-path copies;
normal launch preflight and alternating pairs were retained.

| Portfolio lane | Median-ratio geometric mean |
| --- | ---: |
| Broad | 0.999638 |
| Sentinel | 1.007214 |
| External | 0.999117 |

The sentinel lane exceeds its frozen 1.005 limit. Its recursive_call_tree
control also exceeds the 1.03 case limit: median 1.067917; all five wall
ratios are 1.031694, 1.133349, 1.067917, 1.147127, 0.990077. Instruction median
is 1.000096. Those retained pairs show variability; this failed screen is not
an isolated causal attribution or a claim of a universal 6.8% slowdown.

Outcome: reject maintenance, no formal comparison and no production padding
objects/order entries. Never select another offset from these results or
reopen T053. Static equality of page residues alone does not establish timing
neutrality. Full binary-address/predictor effects or call/stack interactions
remain hypotheses, not an explanation proven by this experiment.

The producer, exact source patch, binary, full symbol tables, correctness
logs and all 76-case pairs remain in workspace outputs/imac-bench under the
job IDs above. The implementation never entered the owner branch or main;
no code revert is required. Only the frozen contract and outcome are retained.
