# T066 allocator evidence

- Status: evidence record; 2026-10-22 (plan stage).
- Implementation: plan frozen; allocator implementation recorded below at handoff.
- Verified at: plan SHA-256 `be05d5e8…` validated 2026-10-22 against queue
  `f6666a68…`; no runtime code or timing yet.
- Evidence: this file; frozen [plan](../performance-units/cli-size-class-allocator.json);
  exact-M profile receipts under workspace `outputs/imac-bench/`.
- Unresolved: implementation, pre-gate, RSS and triage results not recorded yet.
- Next action: implement, verify, then append the result sections.

## Plan stage: target selection

Base M is `9eba7a1d6ae8b67397d20e0d6c93ac79037f5e30` (binary `f5822aa3…`),
main's runtime unchanged since it (`git diff --stat 9eba7a1d main -- crates
Cargo.toml Cargo.lock` is empty). Queue
`current-main-9eba7a1d-queue.json` SHA-256
`f6666a68cb8e1841418d6f5012e6f7810b0c25ebf982b45ef6d5564855e0ae13`.
`validate-unit` accepted the frozen plan with status `valid`; plan SHA-256
`be05d5e8b0cee724114e51428c5aa0593d693f99c718c0e31b009bbda92e177a`.

Allocator self-sample shares were computed from the exact-M receipts' call
trees (self = inclusive minus direct children; macOS malloc/free/realloc family
and Rust alloc shims). Pooled over the three windows per program:

| Program | Receipts | Self samples / total | Share |
| --- | --- | ---: | ---: |
| diff/compare | J0 | 4591 / 18244 | 0.251644 |
| graphlib/paths | J1 | 688 / 3639 | 0.189063 |
| js-yaml/document | J0 | 2217 / 12432 | 0.178330 |
| immutable/persistent | J0 | 1883 / 10695 | 0.176064 |
| moment/dates | J0 | 3434 / 24836 | 0.138267 |
| meriyah/parse | J0 | 993 / 7310 | 0.135841 |

J0 = `20261022T050553Z-t064-successor-exact-profiles`, J1 =
`20261022T072707Z-t056-m2-plan-graphlib-exact`. `big.js/arith`,
`papaparse/csv`, `matter-js/simulate` and `lodash/transform` have no exact-M
receipt and were not ranked or selected. The T065 ablation informed priority
only; the four highest measured shares (`diff`, `graphlib`, `js-yaml`,
`immutable`) were selected by the frozen rule and their v6 payoffs
(0.909827, 0.952265, 0.952265, 0.929158) require removing 39.4%, 26.5%,
28.1% and 43.3% of measured allocator time respectively.

## Implementation and verification

Not recorded yet.

## Instruction pre-gate

Not recorded yet.

## RSS

Not recorded yet.

## Diagnostic quick triage

Not recorded yet.
