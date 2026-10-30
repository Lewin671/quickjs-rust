# T076: Executor bundle

- Status: closed-rejected; lead review 2026-10-29: all three units missed their frozen gates; closed, candidates preserved on their branches, no retry.
- Implementation: reverted; this branch's runtime exactly matches `19125d48`. Each tested candidate is preserved on its named side branch.
- Verified at: U1 `e5dcaef0`, U2 `12aa95b8`, U3 `08bb19a7`, 2026-10-23 UTC; all full/focused/65-case fast gates and 257 NG fixtures pass. Restore commit `eb6afe9e` has the exact `19125d48` runtime and passes full/5,170 curated plus 65-case fast gates; instruction misses, cleanup and hashes are retained.
- Evidence: [completion and job ledger](../diagnostics/t076/completion.md), [unit measurements](../diagnostics/t076/measurements.md), [frozen plan](../performance-units/executor-bundle.json), [validation](../diagnostics/t076/plan-validation.json), [preparation](../diagnostics/t076/preparation.md).
- Unresolved: audio-beat-detection instruction variability is not causally diagnosed; no combined timing, formal/held-out/RSS/resource or new exact Test262 standing. No required candidate-acceptance step remains after the empty-survivor stop.
- Next action: none; no retry. [T077](../T077-core-rebuild.md) is the separately authorized structural successor, not reuse of these budgets or candidates.

## Scope and architecture

Owner branch `agent/t076-executor-bundle/codex`, base above, this worktree only.
No origin push, merge, dependency, third-party or acceptance-tooling changes.
Do not edit T071's `compiler_class.rs`, `compiler_arguments.rs`, `vm_class.rs`.
Preserve the slot/upvalue-cell binding architecture, Value representation,
existing typed-loop scalar/boxed namespaces and exact recovery state.

The lead selected three general mechanisms from T075; hot/cold cache splitting
is excluded after T072/T073's layout diagnosis. Stages:

1. Freeze one schema-3 bundle plan, bound to a fresh exact-base comparison and
   profile receipts, qualified workers-4 collection/v6 analysis, two screens.
2. U1: destination-aware primitive register publication. Borrow the destination
   before publishing immediate constants/copies/numeric results; retain owned
   fallback, self-copy behavior, destruction order and TDZ semantics.
3. U2: retain numeric indices in the scalar namespace when receiver/result is
   boxed. First inspect actual final-IR paths and quantify eligible accesses.
   T045's `dae07218` is an unintegrated identity-provenance candidate, not this
   base; do not import its broader join rewrite. Use existing indexed operations
   and conservative proof/boxing at genuine boxed uses, with exact guard exits.
4. U3: immutable wide-slot to received-cell index metadata. Keep runtime owner
   admission and the same live shared cells; never snapshot a binding value.
5. Each U has its own tested commit/series and a full correctness gate, then a
   non-timing iMac retired-instruction gate. Compare it against the preceding
   surviving bundle, not against an unrelated image. Target geomean <=0.995;
   every other policy selection case is a control and must be <=1.003. A miss
   is preserved on a side branch and dropped, without wall-based tuning.
6. Run one combined quick triage after surviving units: the frozen seven-program
   development wall geomean must be <=0.97 to spend a screen. Then K=6 screen,
   formal 30 blocks, held-out 30 pairs K=6, RSS 15 alternating pairs and T067
   derived resources. Two screen attempts; incomplete collections are recorded.
   No final decide/promotion command: report complete evidence to the lead.

## Selection rule (before any candidate measurement)

Use all three canonical T075 repeats, pooled per program and equally weighted
across the ten development programs. For each mechanism select programs above
its corresponding equal-program mean, without held-out inputs:

| Unit | T075 population / mean | Own instruction targets |
| --- | --- | --- |
| U1 | transport / 7.927330% | graphlib, immutable, lodash, matter-js |
| U2 | indexed-access / 4.068792%, plus eligible baseline final IR | big.js |
| U3 | received_index / 0.423576% | immutable, js-yaml, lodash, papaparse |

For U2, additionally require an existing final-IR Box-derived scalar key at
a ComputedRead: six qualify in big.js; matter-js has none and is a control.
This refinement precedes runtime code and plan freeze; no candidate result
selects work. The bundle and triage target union remains these seven programs. New exact-base
profiles bind identity, without reordering this selection. Sample reach includes
necessary work and is not predicted savings. All 86 policy selection cases are
measured as unit instruction targets/controls; held-out is acceptance only.

## Acceptance and collection contract

T022 overall-first is frozen prospectively: overall gates are development,
external, equal-case portfolio and held-out. Lane ceilings, individual sentinel/
microbenchmark results and RSS are local gates, all reported for lead judgment.
Following T069, a complete K=6 screen whose development, external and portfolio
overall aggregates improve can proceed to the remaining evidence under this
prospective rule even if strict local payoff gates reject; preserve that strict
label and every local miss for the lead. Incomplete/case-health evidence cannot
advance. Correctness and zero Test262 gaps are never waivable. RSS defaults: per-case
median <=1.10, development geomean <=1.05, >=15 alternating pairs; resources
use the T067 derived iMac manifest. No promotion is inferred from instruction
or triage results.

Every iMac build/collection uses the workspace `imac-bench.sh` queue, with a
single literal command argument; check status first, never cancel other jobs.
No timing job starts between iMac local 02:45 and 05:00. Instruction jobs are
explicitly non-timing, with five alternating pairs and checksum-verified raw
process counts; internal cases use fixed N=2^20 / 2N instruction increments, whole
programs use process instructions. No timing-based calibration or selection.
T073-style bimodal tails in triage/screen must retain raw process numbers and
be reported, not retried away. DISTURBED timing jobs are replaced whole.

## Verification

Use `TEST262_DIR=/Users/qingyingliu/Code/co-work-space/projects/quickjs-rust/third_party/test262`
for the shared read-only corpus. Each commit runs the staged fast gate; each
unit and final handoff run `./scripts/check.sh`; runtime changes also run
`./scripts/compare-qjs.sh`. Run exact configured Test262 before any acceptance
claim. Finish with `cargo clean`, restore the tracked `target/` fixture and
confirm >=8 GiB free disk. Record every result and revision here or in the
linked retained evidence; external artifacts alone do not establish standing.
