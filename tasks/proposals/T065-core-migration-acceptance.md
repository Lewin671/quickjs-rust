# T065 proposal: core-migration amendment to T022

**PROPOSAL FOR LEAD REVIEW — NOT ACTIVE POLICY.** This supplements
[T022](../T022-performance-priority-controller.md), not its existing migration
kind. No validator/schema/decision behavior changes in this documentation unit.
[T065](../T065-interpreter-core-v3.md) and its [migration design](../../docs/design/interpreter-core-v3-migration.md)
propose the first use. Existing frozen plans, failed attempts and reserves stay
unchanged. A design approval alone cannot make an unsupported JSON kind valid.

## Proposed new kind and invariant

Add `unit_kind: core_migration` in a prospective, versioned schema, plus validator,
ledger, stage and final-decision support before runtime implementation. Unlike a
leaf or existing per-target migration, this kind prices a whole runtime replacement
on complete portfolios. No intermediate ≥5% per-case or reciprocal B_i target
payoff; no requirement for each structural layer to improve a selected case.
Correctness, resource bounds, identities and final complete-portfolio gains remain
mandatory. Historical T056/T064 budgets cannot be borrowed, restored or relabeled.

Freeze one baseline M (source, canonical executable, recipe, host and policy), all
86 selection cases and nine acceptance-only held-out cases, source/input hashes,
GC/word/root contracts, milestones, stage budgets and failure/formal ledger before
coding. Start from 0ab98e39's runtime, verified identical to measured 9eba7a1d; if
main changes before freeze, refresh all ten development comparisons and exact
profiles. The old 9a0eeeca comparison lacks lodash: it cannot be the complete
candidate/base baseline for this migration. Bind qualified policy v6 or its reviewed
successor to the same build recipe/schedule; requalify if either changes.

## Proposed final payoff (all ratios candidate/fixed M)

| Independent gate | Proposed bound | Interpretation |
| --- | --- | --- |
| Development, all ten equal-weight programs | wall geomean ≤0.80 | At least 20% less elapsed time, i.e. ≥1.25x throughput; stronger and less ambiguous than “20% faster”. Formal upper bound must pass, not just point estimate. |
| Every lane / external suite | ≤1.02 | Broad, six sentinels, development, SunSpider, Kraken and JetStream each pass independently; no lane can be hidden by another. |
| Complete 86-case selection portfolio | equal-case wall geomean ≤0.95 | A development win must yield a material whole-portfolio win too. |
| Every selection/held-out case | ≤1.10 | Hard isolated-regression ceiling, independent of aggregates; use layout confirmation/precision rules below, never silently drop a failing case. |
| Held-out nine-case lane | wall geomean ≤1.00 | Transfer check: no aggregate regression; held-out stays acceptance-only and never selects mechanisms, ports or profiles. |
| Test262 | zero configured Rust-vs-NG gaps, no lost current passes | Same pinned input, options and exact complete inventory; no new skips/xfails; record complete scan and burndown. Curated subset is insufficient. |
| Resource | per-case median paired max-RSS ≤1.05 for all 95 cases | Five paired resource runs/case on exact release images; byte-normalized direct-child collection, no missing coverage. Also bound post-GC live/retained bytes in deterministic churn/root-release tests. |

These numeric values beyond the user's whole-development intent are proposals,
not measured feasibility. A 0.80 development ratio would take the recorded 3.071
Rust/NG wall geomean to roughly 2.46 at unchanged NG; it does not promise parity.
Report candidate/NG as well as candidate/M for every case and suite.

## Proposed intermediate gates and budgets

The opt-in backend may be semantically incomplete before M3. Every commit keeps
the shipped default at current conformance; supported v3 slices must pass without
new exclusions. Unsupported v3 programs remain explicit incomplete rows. M3 and
later require the full new-backend configured exact zero-gap scan. No partial
backend can replace the default, no old heap services it through hidden adapters.

Use the [M0/M1 instruction checkpoints](../../docs/design/interpreter-core-v3-migration.md#m0-retire-the-combined-core-risk-before-the-full-builtin-port)
to stop cheap failures early. Instructions are diagnosis, not final acceptance;
GC/value layout may trade instruction count for locality. During experimental M0–M3,
run only applicable coarse comparisons and per-program RSS ≤1.25 M; do not score
incomplete portfolios. M4 requires all lanes runnable and coarse lane wall ≤1.05 M;
M5 replaces these temporary bounds with every final gate above. A stage failure
blocks advancement; incomplete data is never a passed stage.

Proposed engineering budget: M0 six weeks, including at most two three-day repairs;
M1–M4 use the effort ceilings in the design and report at every exit. Missing the
M0 checkpoint stops that implementation shape for lead review before a bulk port.
Permit at most two failed *whole-core final candidates*, and at most two complete
formal selection+held-out candidate bundles (one per attempt, each with the two
predeclared v6 looks/continuation caps). A valid hard-cap/semantic failure spends
an attempt; inconclusive results spend the fixed measurement allocation but earn
no new implementation attempt or budget. Stop at exhausted engineering, failure
or formal budget. Any later extension needs explicit prospective lead approval.

## Formal protocol / changes to existing policy

Keep sealed three-engine evidence, all cases, exact executable hashes, qualified
host/recipe, fixed equal-length paths, serialized timing, resource completeness,
K=6 layouts, load recovery and prospective v6 formal precision. Freeze a 60-block
initial formal allocation and its v6 continuation cap before implementation;
no post-result case, layout, band or threshold selection. No source-shape or
benchmark-name admission. Same-host diagnostics are not an official suite score.

For this kind only, replace T022's per-target reciprocal-B payoff and automatic
instruction-control hard veto with the whole-portfolio and hard-case wall gates
above. A new architecture is expected to change instruction distributions; retain
all instruction alerts and explain them, but do not reject solely because an old
typed control executes >3% more instructions. A case/lane wall excess opens an
event; a complete six-layout panel resolves nonpersistence or establishes excess
under v6's lane/held-out convention. Formal intervals still must satisfy the
aggregate payoff/ceiling with reviewed precision; unresolved events/intervals
mean inconclusive, never retained. Flat instructions cannot waive an RSS or wall
failure. Any valid zero-gap, RSS or confirmed hard-case failure blocks promotion.

Tooling must expose which gates differ by kind and reject this kind under older
schemas rather than silently reuse `migration`/leaf semantics. Add meaningful
validator/ledger/decision tests and document the new schema in its owning runbooks
as a separate policy change after lead approval. Do not mutate any frozen JSON,
policy artifact, archived evidence or burndown record to enact this proposal.

## Lead decisions requested

Approve or amend: (1) whole-core kind and .80/.95 lane/portfolio payoff with
1.02 lane, 1.10 case and held-out 1.00 ceilings; (2) all-case RSS 1.05 and temporary
1.25; (3) six-week M0, two-repair and two-final-candidate/formal budgets;
(4) replacing per-target/instruction vetoes while retaining layout/precision;
(5) explicit context-bound public root API, audited packed-word/GC boundary and
experimental-backend partial coverage only before the full M3 parity checkpoint.
