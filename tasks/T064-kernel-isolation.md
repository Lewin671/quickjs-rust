# T064: Kernel isolation prerequisite

- Status: active; planning only, awaiting independent review before implementation.
- Implementation: none for isolation. Stage-A runtime5dd3a810 remains present/unpromoted on its branch; it is not the implementation base.
- Verified at: planning inputs99ce7e91,2026-10-21; refreshed main06e199fec9e9e51742cdae7633c5dc758c326752. No new candidate build/timing.
- Evidence: [target/ABI/static contract](../docs/design/interpreter-core-v2.md#kernel-isolation-prerequisite-k0-before-m1-resumes),
  [allocation/startup inputs](archive/T064-kernel-isolation-inputs.md), [source-coupling evidence](archive/T064-kernel-isolation-inputs.md#machine-code-coupling-input).
- Unresolved: fat-LTO/literal closure feasibility, wide projection/service costs, fresh queue/profiles and reviewed successor-plan budgets.
- Next action: independent review of this plan; then freeze/validate the prospective contract before any runtime implementation. Do not time5dd3.

## Goal and boundary

Enforce the design's artifact invariant without timing: object/cold-runtime changes cannot change protected numeric/dispatch bytes or hot-block relative PCs.
First cover the shared typed kernel entered from both Vm and WideLoopFrame, then the wide compact dispatch/numeric core.
A static pass is not proof of speed, physical cache placement or general hardware immunity. Every performance/correctness gate remains separate.

This task is an architectural prerequisite for M1, not M2's frame-store/native-capability migration or M3's Value backend.
Its one proposed internal std-only crate removes Runtime-typed generics and cold ownership from protected bodies; no third-party runtime dependency.
No FFI, unsafe pointer/value reinterpretation, source/benchmark-template admission, new GC/arena, frame-name snapshots or tier-order changes.
Existing numeric compact/mutation executors stay separate; unprotected general VM code acquires no invariance claim.

## Scope and owner

Planning change: existing `docs/design/interpreter-core-v2.md`, this task/index/T056 resume and allocation evidence only.
No runtime/Cargo/recipe/policy/CI/tool implementation in this commit. The lead integrates these docs to main after review, not this branch's runtime history.
Main integration takes the core design, T064 and its self-contained evidence; add the index row/update main's T056 resume without copying stage-A-only history links.
Future owner works serially from a freshly recorded main SHA (planning anchor06e199fe), never ports the stage-A snapshot as the kernel baseline.
Proposed future paths: `crates/qjs-kernel/**`, runtime typed/wide lowering/adapters, `tools/benchmark/kernel_*`,
`scripts/check-kernel-isolation.sh`, CI, recipe identity/order resolution and their owning docs. Freeze their exact boundary before coding.
`third_party`, parser semantics, shared binding authority and benchmark workloads/acceptance bands are forbidden paths.

## Proposed static command and CI acceptance

These commands/tools do not exist yet; they are implementation deliverables, not checks claimed to have run:

```text
scripts/check-kernel-isolation.sh --revision <candidate> --target x86_64-unknown-linux-gnu \
  --feature-set default --feature-set agents --matrix cold-v1 --output <exclusive-dir>
python3 -m tools.benchmark.kernel_isolation compare --base-manifest <base> --candidate-manifest <candidate>
```

The shell wrapper creates isolated source archives and independent target dirs, asserts expected source deltas, forces kernel recompilation and records compiler commands.
Reuse `layout_symbols.py`, the raw readers and decoder methodology in `t056-header`; extract a supported common tool, not an import from a historical task directory.
Add ELF support with pinned `llvm-tools-preview` disassembler/readobj; keep Mach-O support. Unknown encoding/relocation/symbol/clone is an error, not a skipped row.
The manifest records source/kernel/ABI/recipe hashes, target/features, tools, object/final-binary hashes and all instruction/CFG/table/constant/spill comparisons.
Inspect final linked CLI artifacts, not only a cached rlib. A reused kernel object is insufficient: independent clean recompilation must reproduce it too.

Blocking CI job: pinned-toolchain x86_64 Linux matrix, default and agents features, source-identical A/A then every row below; no CPU clock/counter/root permission needed.
Darwin is a separate required production qualification on the serialized iMac with the actual fat16/abort/agents recipe and Mach-O linker; Linux cannot certify it.
Run Darwin static qualification before any iMac performance gate and on compiler/linker/recipe/ABI changes. CI adoption needs a runner/receipt integration choice at review.
Arm64 gets semantic/build coverage initially; do not advertise the x86 byte contract there without its own audit support.

| cold-v1 row | Probe and validity assertion |
| --- | --- |
| R0/R0-repeat | Same production sources, independent clean builds at the same source path; exact kernel/ABI/recipe identity must reproduce before interpreting perturbations. |
| H+8 | Add one initialized, unused native-width cold diagnostic word to ObjectData; compile-time size assertion confirms+8. Kernel sources/ABI unchanged. |
| H16 | Add16B diagnostic ObjectData padding as a separate probe reference; assert actual size. |
| H16−8 | Reduce that padding16→8, confirming a real−8 delta relative to H16. Compare both with R0 too; do not pretend a production field was safely removed. |
| C+8 | Add one defaulted cold ObjectColdData word; assert its size delta and unchanged ObjectData size. |
| helper | A reachable cold tag/error helper uses an out-of-line diagnostic black_box path with identical results; disassembly must prove emitted code/call changed, not DCE. |
| combined | H+8,C+8 and helper together; no row/layout may be selected from results. |

Patches live only in probe archives; original files, kernel/ABI/op inventory and toolchain hashes must remain unchanged.
Compile-time layout assertions certify size changes without adding logging to hot code. Exact patch preimages and changed-path allowlists fail on drift.
Each probe runs focused brand/index/prototype/callback tests and NG output checks; kernel migration candidates additionally require full configured exact Test262.
The finite matrix is supplemented by actual base-vs-PR fingerprint comparison whenever kernel source/ABI/recipe is unchanged; cold-only drift fails CI.
A kernel change never silently refreshes goldens: review source/ABI/fingerprint deltas, then rerun all rows.

Checker self-tests must reject: same normalized mnemonics with a3B internal insertion; changed hot byte; moved hot RIP literal; changed jump-table target;
missing/duplicate/cloned role; unknown relocation; wrong toolchain/features; optimized-away perturbation. A uniform translation of the complete closure may pass.
Keep the existing8f898→5dd3 four-shortening/29ca→29be record as a negative fixture. New reference fingerprints come from the isolated kernel, not the old winning PC.

## Mechanism recommendation and review stops

Select separate non-generic crate + service-yield ABI under the **unchanged** production fat/16/abort recipe for the first prototype.
No reverse dependency on runtime/AST and no generic `F: LoopFrame` instantiation in protected code; both callers adapt outside one typed body.
Boxed runtime state stays outside; wrappers batch services and release all borrows before observable code. Wide numeric projections have one active authority and publish on exit.
This is a candidate mechanism, not a claim that fat LTO respects crate boundaries or that inline hints prove isolation.
A protected literal/table must keep its final relative location; hot external calls/fixups cannot be masked to manufacture a pass.
If IPO cloning, merged constants, backend table placement or wide boundary costs prevent the contract, stop this approach for review.
Do not search padding/inline settings. A whole-profile no-LTO alternative requires a separately frozen compiler study, changed identity/cache key and newly qualified policy.
Cargo cannot express a per-package lto override; hiding bitcode from a fat-LTO consumer is not an approved isolation recipe.
No opaque Value byte copying, retaining all temporaries in a handle arena or per-op virtual callback is an acceptable shortcut.

## Smallest steps and migration gates

| Step | Deliverable / stop condition |
| --- | --- |
| K0 audit infrastructure | Build today's unmodified main and test the negative fixtures/probes. Production runtime/binary inputs stay identical, hence no routing/performance change. This validates the checker and documents current failures; it does **not** claim today's kernel is isolated. |
| K1 typed kernel | Move scalar ops/dispatch to the narrow crate; adapt Vm and WideLoopFrame, preserving scalar/op size/alignment, admissions, iteration limits and exact deopt sites. Services stop before effects. No per-numeric-op bridge. Pass every typed-role perturbation and focused semantics before timing. |
| K2 wide completion | Isolate the wide dispatch/numeric core with batched cold segments and reused scalar scratch. Prove publication/re-entry/throw/alias behavior and bounded scratch/drop costs. No new allocation per short callback. Both protected role families now pass the whole matrix. |
| K3 qualify/integrate | Full gate, NG, subset and full configured exact Test262, then exact-image86 pre-gate, complete K=6 neutrality screen,30-block stage formal evidence/precision and independent resource checks. Only then may main advance; no speedup/retained claim from a neutrality stage. |

K1 is the smallest production-routed proof on today's main; it cannot honestly promise no regression before measurement.
Freeze one K1 diagnostic screen and one K2 final screen in advance; a valid failure at either stops this approach, not a new variation.
Before K3, require all86 original cases (including prospective final targets) inside min(policy wall band,1.025), plus every original lane/portfolio ceiling,
all six placements, original instruction hard caps and applicable resource/held-out gates. Formal intervals/continuations follow v6; no favorable layout/pair deletion.
Thus “without perf regression” is a measured acceptance condition, never inferred from static equality. K1 alone cannot resume M1 or be integrated as the completed prerequisite.

Semantic obligations: NaN/−0/coercion order; loop residency cumulative across service resumes and exact guard-failure PC/stack; writable/captured/global cells and TDZ;
property/accessor/Proxy/TypedArray/detachment guards; nested native callbacks/throws; class initializers; eval/with, recursion and cross-realm behavior.
Track numeric ops, service exits, projections/publications, bytes moved, allocations and peak/retained scratch. Numeric-only loops must have zero service exits per iteration.
Unpack's short wide callbacks are a mandatory cost check; kernel isolation alone does not remove their native string/regexp or layout allocations.

## Frozen-plan transition and attempt ledger (proposal for review)

Do not edit the old `interpreter-core-v2-layout-edges.json` (SHA d184dbcc…); its fixed f58c696e base and M1-only candidate isolation forbid rebasing it onto today's main/kernel.
Keep its valid0a9af81b failed screen recorded. No later static/instruction/mini qualification consumed the remaining A+B attempt;5dd3 stays untimed.
**Recommendation:** at approved implementation handoff, close the old continuation as **closed-superseded**, with final performance standing unresolved,
not a fabricated `decide=inconclusive`/abort and not a rewrite of the valid failure. No old JSON/decision/raw evidence changes in this planning commit.
An inconclusive diagnostic is not a new failed screen or an acceptance decision. A closure documents why the old contract cannot be continued with a new baseline.

Current schema3 requires final improvement (`target_max_candidate_over_base <1`); a standalone neutral final unit cannot be truthfully marked retained.
Use the existing migration mechanism rather than silently adding a neutral decision mode: propose one fresh three-stage successor **K→A→B**,
with T064 owning prerequisite K, T056 owning A/B and final normal payoff. K's neutrality is an intermediate `advance`, not final retention.
Before runtime edits: fresh complete comparison/queue on recorded main M, exact-image profiles, explicit structural-prerequisite priority override,
then a reviewed/validated/committed schema3 plan with final targets/standing payoff min(.97,1/B_i) and all stage/resource gates.
Predeclare instruction-blind classification/memory wall targets if scratch/transport ownership changes; do not assume instruction counts price those costs. Do not fabricate that JSON before those receipts exist.

Proposed failure subbudgets: at most **one** new K failure, independently reviewed; A+B retains **one** unused failure across both stages,
not two fresh failures. A K failure does not debit/replenish that A+B reserve. Overall successor max_attempts≤2 plus the stricter per-stage caps;
freeze those caps in its mechanism/ledger, check before each screen, and carry the old failed attempt's identity permanently. No borrowing unused K allowance into A+B.
Static failure/correctness failure stops before timing; incomplete evidence follows existing completion rules without resetting any cap.
These are prospective budgets awaiting review, not an approved kernel attempt or a reset of T056.

All successor stage/formal decisions keep base **M** fixed, including K cost. After K integrates as revision I, rebase/port M1 semantics onto I,
rerun full correctness/static probes/86/K=6 and evaluate cumulatively against M; I→A comparison is supplemental attribution, never a replacement base.
Refresh exact I profiles to confirm M1 reach; if scope/priority changed, stop for a new reviewed plan without increasing the A+B reserve or retargeting after timing.
Do not automatically port5dd3's112B experiment; start from the reviewed layout-authority design and justify each header/batch/key change from the new profiles.
Historical M1 gains (.904 global/.960 local etc.) and T046 recipe gains are not promised on I; remeasure all controls/development/held-out and make no old-receipt substitution.
If a profile change is selected instead, first study old/new recipes on identical M source and qualify the new policy/identity;
then freeze successor M and both base/candidate builds under that same recipe. Preserve the recipe-only cost separately and never compare mixed recipes under the old plan.

## Acceptance / present verification

- [ ] Independent review accepts the ABI, byte/relocation scope, staged successor and non-resetting budgets.
- [ ] Fresh main queue/profiles and a valid frozen successor plan exist before runtime coding.
- [ ] K1/K2 final-linked static matrix and actual cold-PR guard pass, not only pre-link/normalized comparisons.
- [ ] Complete correctness, prospective neutrality/performance/resource gates pass before main integration/M1 resume.
- [x] Planning only: no runtime/tool/recipe/policy implementation or timing submission; old unit unchanged.

Present checks: context/link/budget checks and full `check.sh` pass on the unchanged99ce7e91 runtime; all six site/family/count/byte/class totals reconcile.
A read-only main06e199fe archive overlaid with this design/T064/evidence plus adapted main index/resume passes check-context; this is not a merge or runtime integration.
The staged documentation fast gate is recorded at commit; no CI isolation implementation or kernel performance result is claimed.
Proposed implementation checks (not executed here): `./scripts/check.sh`, `./scripts/compare-qjs.sh`, full exact configured scans,
new static CI command and the frozen lead-owned performance chain. Planning branch has no approval to execute that chain.
