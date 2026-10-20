# T056 task history: superseded owner notes, unit 0 summary, M1 stage 1, design checklist

- Status: closed (history moved out of the active task to keep it within budget).
- Implementation: none; text moved verbatim from `tasks/T056-interpreter-core-v2.md` on 2026-10-20.
- Verified at: `9ec13250` (text as of that revision).
- Evidence: the records each section links.
- Unresolved: none.
- Next action: none; [T056](../T056-interpreter-core-v2.md) owns current work.

## Owners of earlier design-only changes

Original design-only owner: `agent/interpreter-core-design/codex`, base
`6af6230036f917f6b8df4dc51aa6402a7ef367ce`, worktree
`worktrees/quickjs-rust/interpreter-core-design`. Allowed changes are the new
design, this task and its tasks/README row. No runtime, dependency, benchmark,
third_party, other worktree or performance-plan changes; no iMac jobs, push
or merge. Future structural runtime work is serialized because APIs and
ownership span execution, properties and values.

M2 re-plan owner: `agent/t056-m2-replan/codex`, base `290e3f51`, worktree
`worktrees/quickjs-rust/t056-m2-replan`. Only this task and the design changed;
no runtime edits, iMac jobs, push or merge. The design's [frames section](../docs/design/interpreter-core-v2.md#frames-storage-activation-context-and-lazy-names)
owns the source-grounded consumer inventory and target contracts; the M2 rows below own staging.


## Unit 0 evidence and first-unit order

Owner branch: `agent/t056-unit0/codex`; no runtime
optimization, recipe, order, manifest or benchmark changes. The linked records own
every measurement and hash. Held-out programs never selected or profiled anything.

Part 1: leading broad literal-field deficits execute virtual banks, not name lookup;
HashMap and public-field Raytrace reach property helpers through accelerators only.
`ddca5bdf` fixed the schema-3 wrapper so intermediate stages use the cumulative
budget and final/leaf stages keep payoff.

Part 2 counted the development lane with a feature-only overlay (byte-identical
canonical image with the feature off) and sampled it on the canonical image:

- Named-property work owns a mean 15.4% of samples (at least 10% in 7/9 non-papaparse
  programs) and is reached on the VM path, not only through accelerators: moment and
  diff make 31.1M/13.2M VM-path attempts with 4.0/5.7 name comparisons per attempt.
- Call/activation work is as large (mean 18.3%), but nested-Vm construction is only
  3.5%; `CallEnv` construction, its name-keyed frame-cell layer and write-back take
  11.3% (lodash 25.5%, diff 19.3%). These owner groups include non-removable
  semantic work; they bound reach, not an 11.3% promised win. The design assigns that state to M2.
- Value clone/drop glue is 7.8–15.5% everywhere; virtual-field work is at most 1.1%.

Order stays M1, M2, M3, with M2's scope widened; virtual-field admission stays an L4
leaf. M1's frozen plan targets development moment/dates, diff/compare and
meriyah/parse (stage cap 1.025, final 0.97 or 1/B_i, K=6, one attempt). Durable
counters, when needed, become plain `cfg(feature = "perf-counters")` code
admitted with a layout-rule neutrality screen, never text patches.

## M1 stage 1 (value facade + atom linking)

Owner `agent/t056-m1-atoms/claude` from `a0b6bb23`: realm-owned `AtomDomain`, named operands
linked once per body and domain, Value facade; stores still key by text. Its single screen is a
v3 `reject` (development lane, layouts 0 and 2) and a v4 `pass`; v4 governs as main's bound
v2/v3 successor (`HELD_OUT_SUCCESSOR`). See the [stage 1 record](archive/T056-m1-stage1-evidence.md).


## Design acceptance and verification

- [x] One target specifies flat frames, every ordinary named-property storage
  form, the value word/refcounts, unsafe boundaries, env/cells and tiers.
- [x] Ordered migrations have mechanism evidence, expected effects and stop
  conditions; intermediate regression gates differ from final payoff/leaf gates.
- [x] Prior experiments and report inferences are checked against source/history.
- [x] Documentation budgets, links and task index pass `check-context.py`.
- [x] Staged documentation fast gate passes for the three allowed files.

The original design changed only Markdown; unit 0's verification is in its archive
records. Every runtime stage runs full configured Test262, `check.sh` and NG comparisons.
