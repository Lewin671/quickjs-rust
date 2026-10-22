# T064: Kernel isolation prerequisite

- Status: active; lead feedback 1 approved the K→A→B successor freeze; stage K not implemented or screened. Old layout-edges continuation closed-superseded.
- Implementation: diagnostic archive transformations only; production runtime/Cargo/recipe unchanged. No kernel crate.
- Verified at: spike at main `06e199fec9e9e51742cdae7633c5dc758c326752`; successor receipts bind exact M9eba7a1d; initial full gate at `4fa3f739`, freeze validation/checks on the `4a2c7993` documentation follow-up recorded in the review below.
- Evidence: [spike result/identities](archive/T064-causal-outlining-spike.md), [design/ABI](../docs/design/interpreter-core-v2.md#kernel-isolation-prerequisite-k0-before-m1-resumes),
  [allocation/startup inputs](archive/T064-kernel-isolation-inputs.md), [producer](diagnostics/t064-k-spike/README.md), [lead decision](#lead-decision-2026-10-21), [successor inputs](archive/T064-successor-plan-inputs.md), [lead review/freeze](proposals/T064-successor-review.md).
- Unresolved: K cost remains unpriced; amended K0 checker integration/qualification and separate T056 allocation/startup input review precede implementation. Manual ledger/stage-band/RSS gates remain mandatory.
- Next action: hand the frozen successor to the lead for K0/input integration and an explicit K1 handoff; this freeze starts no implementation or timing. Use the fixed-M plan and preserve the carried failure identity.

## Lead decision (2026-10-21)

The user delegated this decision to the lead. Decisions, binding on later owners until a new recorded decision:

1. **Contract.** K's static invariant for protected bodies is: identical instruction boundaries and intra-function relative PCs, identical
   non-address bytes, identical owned table bytes/targets/relative positions and stack layout, and every RIP-relative operand resolving to a referent
   with identical bytes and width. Displacement-only changes of RIP operands and call/import fixups are classified, reported, and allowed.
   Raw whole-body byte equality is not required. The spike's H+8 differences are all of the allowed class; the T056 failure mechanism
   (an off-path 3B shortening moving Add `29ca→29be`) is a relative-PC change and stays a hard failure.
   This changes only the static artifact contract; no timing, band, attempt or correctness gate is relaxed.
   **Amendment (lead feedback 1, 2026-10-21):** pointer-sized referent slots compare relocated target identity
   (section, symbol or import, offset within symbol) and target contents under the same rule, not raw pointer values.
   The checker traverses local target graphs to depth2; cycles revisit already checked nodes, and new nodes beyond the bound,
   unknown/unresolvable slots and partial slots fail closed. Non-pointer bytes stay raw; owned tables remain fully protected.
   Exact external dyld import bindings are leaves: their definitions are outside the linked executable.
   Identical resolved targets with moved pointer values are reported in the allowed displacement class.
   LEA spans end at the containing symbol's extent when known; through-next-data-boundary is the anonymous fallback.
2. **No crate, no FFI, no linker/literal ownership work.** K1 is production-quality cold outlining of the object-bearing typed arms for both
   Vm and WideLoopFrame on fresh main, following the spike's boundary. A type/field firewall check may be source-level; a crate is not required.
3. **Plan.** Adopt the K→A→B successor migration and the failure sub-budgets proposed below (at most one K failure; A+B keep one shared failure;
   max_attempts ≤ 2). Close `interpreter-core-v2-layout-edges` as closed-superseded when the successor plan is committed.
4. **Time box.** A valid K screen failure stops kernel isolation; the lead then moves T056 effort to M2 (frames/calls) rather than redesigning K.
5. **Parallel work.** T056 unpack allocation and startup-instruction attribution continue without timing; findings land as A-stage inputs.

## Goal and mechanism after review

The target is unchanged: object/cold-runtime changes cannot alter protected numeric/dispatch bytes or intra-function hot PCs.
Start with typed execution for Vm and WideLoopFrame; wide compact isolation is a later migration, not part of this spike.
Static equality establishes neither speed nor hardware placement immunity. General VM and other numeric executors are outside the claim.
The safe Rust ABI, service-yield ownership, exact deopt/publication protocol and no-per-numeric-op bridge remain design proposals.
No FFI, unsafe, new dependency, GC/arena, name-keyed frame snapshot, tier-order change or benchmark-template admission.

The independent dsh review identifies **type-clean hot IR**, not a crate boundary, as the mechanism to test.
Fat LTO merges bitcode and re-partitions CGUs; `inline(never)` does not prevent IPO/IPSCCP, function/constant merging or table movement.
Cargo cannot override LTO per package. A std-only non-generic crate can enforce the source/type boundary auditably, but is not a codegen firewall.
Four off-path brand checks in the same old typed function each shortened by3B, moving Add29ca→29be; the repeated152-instruction path itself was unchanged.
That is a concrete object-offset coupling, not evidence for a particular hardware latency cause.

Only an **LTO-opaque compiled object** (LTO off, embed-bitcode=no, extern C/repr C boundary) gives the proposed compiler-level decoupling.
It conflicts with the repository's no-FFI rule and requires separate architecture/recipe review; it is an unapproved fallback, not this task's implementation.
Failure of this finite spike would not prove that fallback is the only possible solution. Do not search padding/inline settings for a favorable PC.

## Authorized causal spike, before crate/CI investment

Base is fresh main06e199fe, branch `agent/k-spike/codex`, isolated worktree created with the standard helper (owner codex).
Only diagnostic scripts and documents are retained; transformations apply to clean source archives, never production Rust files.
Frozen matrix, submitted as iMac job `20261021T235604Z-t064-k-spike-v4` through `imac-bench run` **without --timing**:

| Role | Source / validity |
| --- | --- |
| r0a/r0b | Identical original sources; independent clean canonical builds at identical source/target paths. Compare full typed bodies and raw selected hot bytes/relative PCs first. |
| h8/c8 | Unoutlined positive controls: add initialized native-width ObjectData word / defaulted ObjectColdData word. |
| o-r0/o-h8/o-c8 | Same perturbations after complete object-bearing arms move to a cold non-inlined helper; execute is non-inlined and deopt/writeback/materialization cold non-inlined. No crate. |

Both typed roles are required. Separate test builds measure actual object/cold sizes and relevant field offsets; canonical builds restore probe-free sources.
All seven canonical targets are deleted/rebuilt, no executable cache reuse. Same fat-LTO/16-CGU/abort/agents recipe and toolchain.
Outlining retains each arm's operations/guards/effects in order; guard failure returns to the caller's existing deopt site, success continues dispatch.
Scalar operations, admission, residency and observer/cell ownership remain unchanged. This broad cold service boundary is diagnostic, not a performance candidate.
Require raw bytes and intra-function relative instruction PCs; retain table positions/targets, RIP referents, full body bytes and source/build identities.
Normalize addresses only for attribution, never to turn a strict failure into a pass. Unknown decoder behavior fails the analysis.
If A/A does not reproduce, stop interpreting perturbations. If unoutlined controls do not drift, report that limitation; no positive-control claim from old receipts.

## Prospective CI, reduced after review

No CI/checker infrastructure is implemented by this spike. K1 remains blocked on the result and a separately frozen implementation contract.
Per PR: structural runtime-type/field-access firewall plus **one targeted perturbation, two clean builds on the production target**.
Full A/A/features/header/cold/helper/combined matrix runs nightly and on compiler/linker/recipe/ABI changes; avoid a full multi-target matrix on every PR.
Use canned binaries for checker negative fixtures:3B insertion/relative-PC drift, hot byte, RIP literal, jump-table target, missing/duplicate/cloned role,
unknown relocation/toolchain/features and optimized-away perturbation. Existing four-shortening history is a negative fixture, not a winning golden.
Final linked artifacts, complete CFG/tables/literals/stack layout and exact role inventories decide; pre-link IR/object inspection only diagnoses.
Linux coverage cannot certify the actual Darwin fat16/agents production artifact; arm64 needs its own audited byte contract before any claim.
Cold-only actual PR fingerprints supplement the targeted probe; kernel/ABI edits require reviewed fingerprints, never automatic refreshes.

## Later migration and costs (not authorized here)

After review: K0 checker contract; K1 non-generic typed boundary if evidence warrants it; K2 wide numeric segments/batched services; K3 full qualification.
Before runtime implementation, obtain fresh main queue/exact profiles, reviewed structural-prerequisite override and validated committed successor plan.
Preserve scalar/op width/alignment, admission, NaN/−0/coercion order, cumulative residency, exact failure PC/stack and shared cell/TDZ authority.
Publish before observers/throws, execute services once, release borrows and reacquire afterwards; test Proxy/accessor/TypedArray/detachment, callbacks, eval/with and cross-realm paths.
Track service exits, projections/publications, moved bytes, allocations and scratch high-water/retention; numeric-only loops require zero per-iteration services.
One predeclared K1 diagnostic and one K2 final screen were proposed, not run or approved here. A valid failure stops the approach for review.
Full check/NG/subset/exact configured Test262 precede exact-image86, complete K=6 neutrality, formal precision/resource/held-out gates and any main integration.
All86 cases must meet min(policy band,1.025) and original lane/portfolio/instruction/resource ceilings; no favorable layout or pair deletion.
Static equality cannot waive boundary/spill/projection/startup costs. The allocation/startup input remains live T056 work:
final diagnostic startup instructions were +7.1% versus its historical base, and unpack allocations remain unresolved. These likely offer more practical payoff than robustness alone.

## Frozen-plan transition and attempt ledger

Lead feedback 1 (2026-10-21) approved the priority override, revised targets,
manual failure accounts, stricter stage-band audit and per-case RSS ceiling.
The [frozen schema-3 successor](performance-units/interpreter-core-v2-kernel-layout-authority.json)
is the K→A→B migration on fixed M9eba7a1d; its mechanism owns the full stage,
static, scope and manual gate contract. [Lead review](proposals/T064-successor-review.md)
records target selection and final validation. K/A advance only; B earns payoff.

The old `interpreter-core-v2-layout-edges.json` is **closed-superseded** by the
[closure record](archive/T064-layout-edges-closure.md). Its f58c696e base and
M1-only isolation cannot include current main and K. Keep its JSON SHA d184dbcc…
and valid failed candidate0a9af81b permanently; one shared A+B failure remains,
with no reset from static/instruction/mini diagnostics. Final standing remains
unresolved; closure fabricates no `decide` or abort result.

All successor decisions include K cost against M. After K integrates at I,
refresh exact I profiles to confirm M1 reach, then evaluate A/B cumulatively
against M; I→A attribution is supplemental. Changed scope/priority stops for
review without retargeting or replenishing budgets. Do not automatically port
5dd3's112B experiment or substitute historical gains/receipts for new evidence.
No K1 implementation or timing screen starts in this freeze handoff.

## Historical spike result and verification

[Complete record](archive/T064-causal-outlining-spike.md): A/A whole images reproduce; unoutlined H/C both detect raw drift.
Outlining stabilizes all selected loop PCs/bytes, tables and spills, but H still changes entry constants and numeric math call fixups;
therefore **outlining alone fails the full artifact contract**. Final section-distance movement explains the residual; no internal IPO/table/spill rewrite was observed.
C's selected closure including entry loads passes; its whole body still has cold cleanup/deopt fixup changes. No normalization converts H to a pass.
A crate is an enforcement option for the field/type boundary, not a cure for these final literal/import distances; the opaque-object fallback is not proved uniquely necessary.
The spike alone approved no K1/CI investment. The amended lead contract and frozen-plan handoff above now own the next action; K1 remains unstarted here.
All7 images pass256 NG cases; generated outlined C source passes2346 runtime tests; production full gate includes5170 subset cases.
Context/file-size/staged checks and exact identities are in the record/commit. No performance result or old-attempt reset;5dd3 stays unpromoted and untimed.
