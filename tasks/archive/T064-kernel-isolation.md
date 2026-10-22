# T064: Kernel isolation prerequisite

- Status: closed-rejected; lead decision 2026-10-22 aborts successor stage K before any timed screen and abandons kernel isolation by outlining. M1 A/B remain blocked; T056 moves to M2 planning.
- Implementation: K0 checker retained on main; K1 `671e9943` and Repair 1 `773c42f7` remain only on `agent/t064-k1/codex`. No production runtime/Cargo/recipe change is imported by this closure.
- Verified at: K1 and Repair 1 pass full/fast correctness gates, 256 NG fixtures, 5,170 curated cases and A/A/H+8/C+8 static checks for both roles; both exact-image instruction pre-gates block the same 46/86 cases. Receipts and limits are in the K1 record; no full exact Test262 scan or timing result is claimed.
- Evidence: [K1/repair record](T064-k1-record.md), [successor closure and ledger](T064-kernel-layout-authority-closure.md), [K0 record](T064-k0-checker.md), [lead review/freeze](../proposals/T064-successor-review.md), [final lead decision](#lead-decision-2026-10-22).
- Unresolved: object-layout → typed-code coupling has no accepted remedy; exact per-symbol instruction/spill deltas remain unmeasured. Static isolation alone does not qualify K's cost.
- Next action: none for T064. [T056](../T056-interpreter-core-v2.md) drafts the M2 frames/activation plan; any future M1 continuation needs a new accepted remedy and preserves the carried ledger.

## Lead decision (2026-10-22)

Stage K of the frozen successor is aborted before any timed screen. K1
`671e9943` and Repair 1 `773c42f7` (false-cold-hint removal) both pass the
static isolation checker (A/A, H+8, C+8; both roles) and all correctness
gates run, but fail the exact-image instruction pre-gate on the same 46/86
cases. Repair's separate whole-process attribution ratios are big.js 1.636,
access-nbody 1.493, math_abs 1.464 and string_key_map_churn 1.443; the
[K1 record](T064-k1-record.md) distinguishes them from policy pre-gate ratios.

The lead attributes the cost to the boundary: every object/boxed operation
in a typed loop (property/array reads and writes, MoveBoxed, native calls)
becomes an out-of-line call with argument passing and a second op dispatch,
estimated at about 2–3x the inline operation cost. Removing the cold hint did
not remove the excess. This is the lead's boundary-cost interpretation;
sampling does not measure an exact per-operation instruction multiplier or
apportion the excess among calls, argument transport, dispatch and spills.

Per the earlier time box, kernel isolation by outlining is abandoned.
Abort closes this implementation shape, not the mechanism family. Zero timed
attempts were spent; K's failure allowance is untouched but moot. The
[closure ledger](T064-kernel-layout-authority-closure.md) preserves the one
shared A+B reserve and carried `0a9af81b` failure for any future M1
continuation. M1 stages A/B remain blocked on object-layout → typed-code
coupling, with no accepted remedy. T056 effort moves to M2
(frames/activation), whose plan is being drafted separately.

The remaining sections record the earlier contract and proposals; they do
not authorize another K repair or screen after this closure.

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
   The checker traverses local target graphs to depth 2; cycles revisit already checked nodes, and new nodes beyond the bound,
   unknown/unresolvable slots and partial slots fail closed. Non-pointer bytes stay raw; owned tables remain fully protected.
   Exact external dyld import bindings are leaves: their definitions are outside the linked executable.
   Identical resolved targets with moved pointer values are reported in the allowed displacement class.
   LEA spans end at the containing symbol's extent when known; through-next-data-boundary is the anonymous fallback.
   **Amendment 2 (lead feedback 2, 2026-10-21):** confirmed code targets outside the protected-body set are identity-only
   (same symbol/import and offset within symbol); their function bytes are excluded. Code inside the protected set retains full comparison;
   data targets retain depth 2. Mach-O instruction flags, unique function identities and inline-data ranges classify local targets;
   imports require a matching x86 code stub/bind identity. Unknown classification fails closed. Owned tables remain fully protected.
   H+8/C+8 transformations insert fields/initializers on existing lines and assert unchanged per-file line counts and every original
   line's number/content after removing only the declared insertions. No panic-location/line-number allowance is added.
   Run the final current-main matrix once; report any residual and stop checker iteration after that run.
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

The spike implemented no CI/checker infrastructure; the subsequent [K0 checker](T064-k0-checker.md) is tooling only, with no CI hook. K1 was subsequently frozen and implemented on its branch; the final decision above closes it.
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
The [frozen schema-3 successor](../performance-units/interpreter-core-v2-kernel-layout-authority.json)
is the K→A→B migration on fixed M9eba7a1d; its mechanism owns the full stage,
static, scope and manual gate contract. [Lead review](../proposals/T064-successor-review.md)
records target selection and final validation. K/A advance only; B earns payoff.

The old `interpreter-core-v2-layout-edges.json` is **closed-superseded** by the
[closure record](T064-layout-edges-closure.md). Its f58c696e base and
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

## Historical spike (original raw-byte contract)

[Complete record](T064-causal-outlining-spike.md): A/A whole images reproduce; unoutlined H/C both detect raw drift.
Outlining stabilized selected loop PCs/bytes, tables and spills, but H changed entry constants/math call fixups and failed the spike's original raw-byte contract.
The later lead decision/amendments above supersede that static interpretation; the outlined images were not requalified by this final checker run.
C's selected closure including entry loads passed raw equality; whole bodies retained cold cleanup/deopt fixup differences.
A crate alone does not fix final literal/import distances; the opaque-object fallback was not proved uniquely necessary.
The spike implemented no K1/CI. Current unoutlined-main qualification is owned by the [K0 record](T064-k0-checker.md); the later K1/repair results are in the record linked above.
All7 images passed256 NG cases; generated outlined C source passed2346 runtime tests; that production full gate included5170 subset cases.
No performance result or old-attempt reset;5dd3 stays unpromoted and untimed.
