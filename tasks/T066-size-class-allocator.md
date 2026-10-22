# T066: Production size-class global allocator for the CLI

- Status: active.
- Implementation: present at `abd6451b`; RSS round 1 page-structured allocator
  with page headers, per-page free lists/live counts, empty-page recycling and
  an atomic cross-thread remote list.
- Verified at: `abd6451b`, 2026-10-22; local full gate with Test262 deferred
  (worktree submodules uninitialized), iMac correctness (compare-qjs and
  5170/5170 Test262), 86-case instruction pre-gate PASS, RSS cohort and
  diagnostic triage. Round-1 details:
  [evidence](archive/T066-allocator-evidence.md#rss-round-1-page-structured-recycling-lead-decision-2026-10-22).
- Evidence: [plan](performance-units/cli-size-class-allocator.json), this file,
  [evidence record](archive/T066-allocator-evidence.md).
- Unresolved: RSS is still above 1.05 on `graphlib/paths` (1.073, target) and
  `big.js/arith` (1.057); triage is `unclear` with six possible regressions.
  The lead stopped further retuning after this round; the K=6 screen, formal
  run, held-out acceptance and the RSS review remain the lead's.
- Next action: lead reviews the round-1 numbers and cause and decides the unit;
  do not retune further and do not change the frozen plan.

## Goal

Replace the CLI process's global allocator (std `System`, i.e. macOS system
malloc) with a production thread-local size-class allocator and show, under the
frozen layout policy, that whole-program wall time on the selected development
targets improves without breaking controls. The allocator is a CLI concern:
`qjs-runtime` and every library crate keep their allocator so embedders decide
their own policy.

## Scope

- Allowed paths: `crates/qjs-cli/src/allocator.rs` (new),
  `crates/qjs-cli/src/main.rs` (module + `#[global_allocator]` wiring),
  `tasks/T066-size-class-allocator.md`, `tasks/README.md`,
  `tasks/performance-units/cli-size-class-allocator.json`,
  `tasks/performance-units/README.md`, `tasks/diagnostics/t066/rss.sh`,
  `tasks/archive/T066-allocator-evidence.md`.
- Forbidden paths: `crates/qjs-runtime`, `crates/qjs-ast`, `crates/qjs-lexer`,
  `crates/qjs-parser`, `third_party/**`, workspace `Cargo.toml`/`Cargo.lock`
  dependency changes, benchmark policies/manifests.
- Owner boundary: single owner; base `9eba7a1d`, branch
  `agent/t066-size-class-allocator/cmd`, worktree
  `worktrees/quickjs-rust/t066-allocator`.

## Target selection (frozen before implementation)

Rule: rank every development program that has an exact-M profile receipt by
its allocator self-sample share, then take the highest shares whose frozen v6
payoff `min(0.97, 1/B_i)` is credible given the measured share. The mechanism
is instruction-blind, so the payoff is a memory-sensitive wall target and every
target is also a memory wall target. The T065 ablation is diagnostic priority
evidence only: it is not used to choose or order targets.

Allocator self samples are the self time of the macOS malloc family
(`nanov2_*`, `free`, `default_zone_*`, `_malloc_zone_*`, `malloc`, `calloc`,
`realloc`) plus Rust alloc shims (`__rust_alloc*`, `alloc::alloc::*`,
`RawVecInner::reserve/grow`), computed from each receipt's `profile.sample`
call tree (self = inclusive minus direct children). Fractions below are pooled
over the three windows; reach is not a removable fraction or a speed forecast.

| Development program | Exact-M receipts | Allocator share | B_i | Payoff | Allocator time that must disappear | Decision |
| --- | --- | ---: | ---: | ---: | ---: | --- |
| `diff/compare` | J0 | 0.25164 | 1.099110 | 0.909827 | 39.4% | target |
| `graphlib/paths` | J1 | 0.18906 | 1.050127 | 0.952265 | 26.5% | target |
| `js-yaml/document` | J0 | 0.17833 | 1.050127 | 0.952265 | 28.1% | target |
| `immutable/persistent` | J0 | 0.17606 | 1.076243 | 0.929158 | 43.3% | target |
| `moment/dates` | J0 | 0.13827 | 1.050127 | 0.952265 | 36.3% | control |
| `meriyah/parse` | J0 | 0.13584 | 1.071895 | 0.932927 | 52.9% | control |

`big.js/arith`, `papaparse/csv`, `matter-js/simulate` and `lodash/transform`
have no exact-M profile receipt, so their share is unmeasured and they cannot
be selected here; they stay controls and are named as a limitation. J0 is
`20261022T050553Z-t064-successor-exact-profiles`; J1 is
`20261022T072707Z-t056-m2-plan-graphlib-exact`. The ablation's own development
geomean 0.89 and per-program 0.80-0.90 support the credibility of these
payoffs, but the ranking above stands without it.

Queue binding: candidate `9eba7a1d`, queue SHA-256 `f6666a68...`; development
ranks js-yaml 1, immutable 2, moment 3, diff 4, meriyah 5, graphlib 6. All four
targets are inside the plan's rank ceiling 6.

## Design (frozen)

- 32 size classes: 16-byte steps to 128 B, then four equal classes per power of
  two up to 8 KiB (160, 192, 224, 256, 320, ..., 8192). Largest internal
  fragmentation is one 16-byte step at 128 B and one quarter-step above it.
- A layout with `size > 8192` or `align > 16` is System-managed; zero-sized
  layouts forward to System as well.
- Page-structured, mimalloc-style: a 64-KiB System allocation aligned to 64 KiB
  per page, header at the base, one class per page, and a per-page free list,
  bump area and live count. `block & !(PAGE_SIZE - 1)` finds the header.
- A thread owns its pages through per-class lists and a current page. Allocation
  takes the current page's free list, then bump space, then its drained
  remote list; when the current page is full it scans the class list and otherwise
  takes an empty page or a new one.
- Free updates the owning page's live count; when it reaches zero the page is
  unlinked and parked in a thread-local empty pool of at most 8 pages, or
  returned to System. Empty pages are reusable by any class.
- Cross-thread free is sound without a lock: the freer pushes the block onto
  the page's atomic remote list and leaves `live` alone. The owner subtracts
  the drained count, so `live == 0` proves no block is allocated, no remote
  entry is queued and no future remote push is possible; only then is the page
  recycled. A remote push happens-before the owner's drain.
- `thread_local!` is const-initialized and registers no destructor, so no thread
  ever observes a destroyed pool; pages retained by a thread that exits leak,
  which is sound (no pointer outlives a live owner).
- `realloc`: same class returns the same block; System-managed to System-managed
  with equal alignment forwards to `System.realloc`; every other case allocates
  the new layout, copies `min(old, new)`, and deallocates the old block.
- `alloc_zeroed`: System-managed forwards to `System.alloc_zeroed`; a size-class
  block has its whole class cleared, so recycled content never leaks.
- Audited unsafe in the allocator module only; no dependency, no FFI, no global
  mutable state beyond the `#[global_allocator]` static and the thread-id
  counter.

## Acceptance

- [x] Plan committed and `validate-unit` accepted before runtime code.
- [x] Unit tests cover class mapping, alignment, every class, all realloc paths,
      zeroing, cross-thread frees and a mixed stress run.
- [x] `./scripts/check.sh` passes; `./scripts/compare-qjs.sh` and the Test262
      subset agree with base (both ran on the iMac at the exact candidate).
- [x] Exact-image instruction pre-gate over all 86 policy cases and per-case
      max-RSS for all development cases are recorded.
- [x] Diagnostic quick triage on targets and all sentinels is recorded.
- [x] Evidence and plan-index row are committed; the lead runs acceptance.

## Verification

```sh
cargo test -p qjs-cli
./scripts/check.sh
./scripts/compare-qjs.sh
./scripts/test262-subset.sh
./scripts/performance-decision.sh validate-unit \
  --unit tasks/performance-units/cli-size-class-allocator.json \
  --queue /Users/qingyingliu/Code/co-work-space/tmp/qjs-lead-briefs/current-main-9eba7a1d-queue.json \
  --profile-root /Users/qingyingliu/Code/co-work-space/outputs/imac-bench
```
