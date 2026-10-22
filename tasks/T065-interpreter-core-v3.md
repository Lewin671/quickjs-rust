# T065: Interpreter core v3

- Status: active; design reviewed (dsh), not approved. User decision 2026-10-22: run a pre-M0 mechanism ablation on real programs before choosing between this rewrite and an incremental path.
- Implementation: none; documentation only. Existing runtime remains unchanged;
  no new crate, dependency, unsafe block, schema kind or performance plan exists.
- Verified at: design tree over 0ab98e3920629da1d7d6efae46f24f0b43bfa018,
  2026-10-22; context and staged fast-gate results recorded below. Instruction
  evidence uses exact M=9eba7a1d binaries, not an implemented v3 candidate.
- Evidence: [gap anatomy and instruction record](archive/T065-gap-anatomy.md),
  [target architecture](../docs/design/interpreter-core-v3.md),
  [migration](../docs/design/interpreter-core-v3-migration.md),
  [acceptance proposal](proposals/T065-core-migration-acceptance.md),
  [independent review and lead outcome](archive/T065-design-review.md).
- Unresolved: target level (the proposed 0.80×M leaves ~2.46× NG), shared-window frames (T021), collector choice, effort (review: 45–70 one-owner weeks);
  per-mechanism wall attribution is unmeasured.
- Next action: plan and run the ablation ladder (value word ownership, global allocator control, compact general bytecode, zero-based
  per-activation entry) on big.js, immutable and js-yaml with wall and instruction evidence; then decide rewrite vs incremental.
  T056's remaining M2/M3 work stays superseded pending that decision.

## Intent

The user selected a core rewrite after current real-library development standing
remained about 3x NG and object-layout changes perturbed accelerated kernels.
Replace shared structural costs together; judge the finished runtime by real
whole-portfolio gains plus zero conformance gaps, not each layer's ≥5% target.
This task does not automatically reopen T056/T064 attempts or activate new rules.

Fresh whole-process retired-instruction baseline (all ten development programs,
three paired runs, exact M/NG binaries, checksums valid): **2.3763x NG geomean**.
Recorded separate wall standing is 3.071x. These are distinct metrics/collections.
Existing samples identify named property ownership and call/context construction;
visible historical gap-equivalent reach is about 23% and 27%, with about 15%
direct Value glue overlapping both. These are conditional opportunity envelopes,
not additive causal percentages. The evidence file states denominator and limits.

## Scope and decisions proposed

- Allowed: this task, tasks/README.md, T056/T022 resume/proposal links,
  docs/design/interpreter-core-v3*.md, T065 evidence/proposal documents.
- Forbidden: runtime/parser/AST changes, dependencies/build tooling, frozen
  plans/policies, third-party edits, timing acceptance, origin push or merge.
- Owner: single design owner; shared runtime architecture and docs serialized.
- Branch: agent/t065-core-v3-design/codex; base 0ab98e39; this worktree.

Recommended destination: non-owning packed words; runtime arena + tracing GC;
uniform atoms/shapes/slots/ICs; compact register bytecode and one frame driver;
typed native capabilities and root scopes; canonical slot/shared-cell bindings;
primary 8/16-bit strings; heap suspension records and engine job queue. Keep
parser/AST and useful typed analyses, retire separate compact/wide ownership.

Recommend an alternate backend inside qjs-runtime with vertical builtin-family
ports, no live cross-heap adapter and no silent old-VM fallback. M0 runs complete
big.js/arith before bulk porting; graphlib follows to test object/call generality.
The milestone design estimates 24–40 engineer-weeks plus contingency (30–50),
with a six-week M0 stop checkpoint; these are planning estimates, not a schedule.

Proposed final contract: development wall geomean ≤.80 M; selection portfolio
≤.95; every lane/suite ≤1.02; every case ≤1.10; held-out geomean ≤1.00; per-case
RSS ≤1.05 M for all 95 cases; full configured exact Test262 zero gaps. Proposed
intermediate instruction/coverage/resource gates and finite budgets are in the
amendment. Current tooling does not implement `core_migration`; approval and
policy support must precede a valid plan. No runtime implementation authorized
by this design commit alone.

## Review checklist

- [x] Compare pinned NG and Rust across values, heap, objects, dispatch, calls,
  environments, strings, exceptions and suspension/native boundaries.
- [x] Reuse existing T056/T064 receipts, with historical/current limits explicit.
- [x] Collect non-timing paired instructions for both engines on all development
  programs; retain numeric results and exact identities in repository evidence.
- [x] State kept/replaced/retired architecture, unsafe/root and binding invariants.
- [x] Compare migration options, scope a real-program M0 and estimate milestones.
- [x] Draft the T022 amendment, clearly separate proposal from current policy.
- [ ] Lead approval, supported policy/schema and frozen M0 plan (next work).
- [ ] Runtime implementation, exact conformance and final acceptance (future).

## Verification and handoff

At the design tree over 0ab98e39, run:

```sh
./scripts/check-context.py
PYTHONDONTWRITEBYTECODE=1 ./scripts/check-touched.sh --staged --explain
```

Both commands passed on 2026-10-22 for the design tree over 0ab98e39;
`git diff --check` also passed. The staged fast gate selected context checking
and explicitly skipped Rust/Test262 as documentation-only. No runtime behavior changed; no semantic Test262
slice applies. Full runtime gate is not rerun for this docs-only change (no
scripts, Cargo files or executable examples changed). No timing claim, push or
merge. Local free disk was 69 GiB; no builds were performed. One non-timing
collector failed on role-specific stdout validation, then the corrected job
completed 60 records; both job identities and correction are in the evidence.

## Ablation ladder

Prospective diagnostic plan, 2026-10-22, base `db64cab0` (runtime/build
inputs unchanged from measured main `9eba7a1d`). No acceptance attempt,
policy change, runtime integration or origin push. Scratch code lives only
on `agent/t065-abl-<name>/codex`; this branch owns documentation.

Fixed targets for every experiment: `development/big.js/arith`,
`development/immutable/persistent`, `development/js-yaml/document`, and
`development/graphlib/paths`. Fixed controls: all six sentinels and the
19 controls already hard-coded in `tools/benchmark/quick_triage.py`, including
JSON parse/stringify, raytrace, binary trees, audio-dft and cdjs external
cases. No held-out programs, optional case removal or result-based retargeting.

1. **Allocator (round 1).** Pure-Rust thread-local power-of-two size classes
   (16–2048 bytes), chunk-backed bump allocation with returned-block reuse;
   larger/over-aligned allocations use System. Slabs retained to process exit:
   a realistic fast-path prototype, not a production memory estimate. Compare
   separately with a pinned mimalloc global allocator. The C allocator uses
   FFI, forbidden in a landing engine by AGENTS.md; explicitly authorized here
   only as a never-landing diagnostic alternative, not a mathematical bound.
2. **Call/activation (round 1).** Reuse exact M2.1 pricing branch `117082ed`;
   measure its implemented narrower cohort without extrapolating to unpriced
   native/argument/frame work. No shared-window rewrite.
3. **Value ownership (round 2).** Try a deliberately leaky ownership oracle:
   cheap handle copies and omitted refcount release for measured runs. Record
   which wrappers are covered and retain allocation/width work that remains.
   Checksums must match; mutation/uniqueness behavior must remain protected.
   If a full bit-copy oracle cannot preserve checksums, report that failure
   and use a narrower safe-lifetime leak experiment. This is no price of an
   8-byte word, and results cannot be added to allocator/call savings.
4. **Named authority (future, lead review required).** Bound string/hash/cache
   reject work while preserving receiver/prototype validation; then price an
   atom/slot prototype if worthwhile.
5. **Bytecode/dispatch (future).** Bound the 112-byte Op fetch/dispatch cost
   independently of Value and call storage; compare compact general encoding.
6. **Startup (optional future).** Separate process launch/parse/compile from
   repeated execution before attributing whole-process gains to the VM.

For each variant collect three alternating whole-process retired-instruction
pairs in a non-timing iMac queue job, with validated stdout/checksums and exact
source/workload/binary hashes. Separately run `scripts/quick-triage.sh` through
`imac-bench.sh run --timing`, five natural-layout pairs, unchanged fixed
controls and canonical agents/fat16/abort recipe. Check queue status first;
never direct SSH, cancel another job, or overlap timing. Batch all four targets
per variant. Preserve failed/unclear/budget-missed jobs; report per-program
candidate/main ratios, instruction ranges and wall exploratory intervals,
request/result hashes and stage costs in [rounds 1–2](archive/T065-ablation-r1.md).
Disk must remain at least 8 GiB locally and on the runner. Remove created
scratch worktrees and their target/debug outputs; retain branches. Stop after
round 2 for lead review. No production conformance or performance claim.
