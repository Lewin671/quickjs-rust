# T057: String code-unit migration

- Status: active; stage 1 promoted and integrated; stage-2 plan frozen before runtime edits.
- Implementation: stage 1 present on main; stage 2 not implemented.
- Verified at: base `a68a9f46`, 2026-10-19; 45 String unit tests and the
  exhaustive String Test262 slice passed (zero actionable gaps).
- Evidence: [stage-1 record](archive/T057-stage1-evidence.md); stage-2 baseline
  and profiles below. Raw workspace artifacts are not retained in Git.
- Unresolved: stage-2 implementation and acceptance pending.
- Next action: implement the frozen stage-2 plan, then correctness gates and triage.

## Goal and scope

Implement stage 2 of the [code-unit authority design](../docs/design/string-code-units.md):
one lazy compact unit view for String primitives, code-unit positions and
selected-output encoding. Admission uses operations, types and data flow.
RegExp matcher coordinates remain stage 3; builders are stage 4; primary
storage must coordinate with T056 M1 in stage 5.

Owner: `agent/t057-stage2/codex`, worktree `worktrees/quickjs-rust/t057-stage2`,
base `a68a9f469a1c36c8080b71e1546851a30f448da6`. Allowed: String/RegExp,
shared string view/cache helpers, focused tests/fixtures, this task/design/index
rows and the stage-2 performance plan. No interpreter, property-storage,
value-word, dependency or third_party changes. No project push or merge;
queue transport refs only.

## Stage-2 evidence

The existing formal bundle at the exact starting revision is reusable under
workflow step 1: job `20261019T174302Z-t058-v6-current-base-formal30`, 30 blocks,
workers-4, exit 0 / 3347 s, no DISTURBED, no measurement issues or imprecise
cases. Candidate `a68a9f46` and base `89a13699` share image
`897eaa88f9b10fb684ce9b65e63e842113ae7ef82fac0e6e58fc6b337c2204a9`;
NG image `919f7235...`, pinned `f7830186`. A new queue is replayed from that
sealed bundle, without a redundant timing run.

Fresh queue/profile job `20261019T232925Z-t057-stage2-fresh-evidence` profiles
js-yaml (rank 2) and diff (rank 5). Follow-up
`20261019T233209Z-t057-stage2-external-profiles` profiles ranked string-base64
and string-fasta. Neither job profiles held-out programs. Jobs use the iMac
queue; submission checked host local time (07:28 CST) and an empty queue,
well outside the excluded 03:00-05:00 window.

The four documented String differences reproduce at the base as
`[1,false,false,[55357]]`, where NG returns `[2,true,true,[55357,56832]]`.
The exhaustive base String slice passes 1221 Rust cases and 1216 NG cases
with zero actionable gaps; these differences need focused coverage beyond
that upstream slice. Base RegExp slice passes 1579 Rust / 1573 NG cases with zero actionable gaps.

## Stage-1 standing

**PROMOTE** under v4, candidate `026476d5` vs `351004f3`, attempt 1/2;
implementation integrated into main. All details, rejected/disturbed
observations, hashes and exact-revision gates are preserved in the
[stage-1 record](archive/T057-stage1-evidence.md).
Pre-stage-1 coordination is in [the log](archive/T057-log.md).

## Stage-2 frozen plan

Plan [string-unit-views-primitives](performance-units/string-unit-views-primitives.json)
is a separate schema-3 leaf unit, bound to the fresh queue and exact diff
receipt above: v6, workers-4, mandatory K=6 (5 pairs), formal 30 blocks,
held-out 30 pairs, two attempts, diff <=0.95 and below reciprocal B_i;
every other policy case is a control. V6 formal continuations use the two
fixed looks (30/240), never an adaptive middle look. Held-out is acceptance
only. The profile supports the operation family, not a promised speedup;
the payoff hypothesis may fail. Triage is diagnostic and spends no attempt.

The compact view and legacy matcher occupy alternative cache states; existing
matcher coordinates and their mode-keying stay until stage 3. This avoids
permanent duplicate dense storage without pulling matcher coordinates into
stage 2. Alternating String and RegExp use can replace the cache; stage 3
must remove that transitional eviction. No T056 change is needed before
primary storage (stage 5).
