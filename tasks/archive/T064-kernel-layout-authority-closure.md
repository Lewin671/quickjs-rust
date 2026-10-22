# Closure: interpreter-core-v2-kernel-layout-authority

- Status: closed-rejected; lead decision 2026-10-22 aborts stage K before any timed screen and closes this frozen successor implementation shape.
- Implementation: K0 checker remains on main. K1 `671e9943` and Repair 1 `773c42f7` are retained only on `agent/t064-k1/codex`; no runtime changes are imported or promoted here.
- Verified at: the [owner record](T064-k1-record.md), imported from branch evidence commit `06682ecc`, records both candidates' full/fast correctness, 256 NG fixtures, 5,170 curated cases, both-role static matrix passes and complete 86-case instruction pre-gates. This closure is documentation over `d1e80fec`; context and staged fast gates passed before commit.
- Evidence: [final lead decision](T064-kernel-isolation.md#lead-decision-2026-10-22), [K1/repair evidence](T064-k1-record.md), [frozen plan](../performance-units/interpreter-core-v2-kernel-layout-authority.json), [earlier ledger](T064-layout-edges-closure.md).
- Unresolved: M1 A/B remain blocked by object-layout → typed-code coupling; no accepted remedy. No timing, formal resource cohort, full exact Test262 scan or final performance standing is claimed for K.
- Next action: no further K repair or screen. [T056](../T056-interpreter-core-v2.md) moves to a separately drafted M2 frames/activation plan.

## Reason and scope of abort

The frozen successor is `interpreter-core-v2-kernel-layout-authority`, fixed
M `9eba7a1d6ae8b67397d20e0d6c93ac79037f5e30`, plan SHA-256
`a23cba70f9ffe45870d171f6065192eda06604befb26e2e7eae978e5c39822fb`.
Its JSON stays byte-identical at `migration.current_stage = 1`.

K1 and Repair 1 both pass A/A, H+8 and C+8 for Vm and WideLoopFrame, but
both exact-image instruction pre-gates block the same 46/86 cases. Removing
the false cold hint does not remove the excess. The lead's dated decision
attributes the cost to the out-of-line object/boxed service boundary and
abandons kernel isolation by outlining under the earlier time box. The
owner record retains all case ratios, hashes, sampling limits and checks.

Under [Migration stages](../../docs/performance-workflow.md#migration-stages),
abort closes this implementation shape, not the mechanism family. The shape
closed here is K's per-object/boxed-op outlined service boundary in typed
execution for both roles, and thus this K→A→B continuation. This is a lead
abort before timing, not a fabricated `decide --mode stage` result or a
failed timed screen. No stage advances, runtime promotion or timing result
is recorded. A different isolation remedy or future M1 continuation needs
new review and a newly frozen contract, preserving the ledger below.

## Failure ledger carried without reset

- K spent zero timed attempts. Its independent at-most-one failed-screen
  allowance is untouched but moot because the lead stopped this shape.
  No unused K allowance transfers to A/B.
- The original layout-edges valid failed candidate
  `0a9af81bebcef80e9239954f8db191a71f10316e` remains spent. Its complete K=6
  rejection and receipt hashes remain in the [earlier closure](T064-layout-edges-closure.md).
- Exactly one unused failure remains shared by A+B for any future M1
  continuation; neither K1 nor Repair 1 changes that reserve. Pre-gate
  diagnostic runs do not spend or replenish timed attempts. This pre-timing
  lead abort closes the shape without recording an additional failed screen.
- M1 A/B remain blocked, with no accepted coupling remedy. M2 planning is
  separate and does not reset or borrow from the M1 ledger.

Only documentation, imported diagnostic producers and links change in this
closure. Runtime code stays on `agent/t064-k1/codex`; no branch merge or push
is part of this handoff. Historical checks apply to their named candidate
sources, not to an assertion that K code exists on main.
