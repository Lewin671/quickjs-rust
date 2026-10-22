# T066: Production size-class global allocator for the CLI

- Status: active; screen attempt 2 of 2 (`20261022T172940Z-t066-screen-attempt2`, candidate `9420a6a5`, allocator source `f4405037`)
  rejected only on the sentinel lane ceiling; continued under the [user sentinel override](#user-sentinel-override-2026-10-22). Formal, held-out
  and exact Test262 next.
- Implementation: present at `f4405037`: 64-KiB pages, O(1) allocation (free-page queue, one bump page per class, remote-free epoch scan),
  out-of-line `GlobalAlloc` entry, per-page cache coloring (`HEADER_SIZE + (c mod 16)*64`). Attempt-1 regressions (O(pages) scan,
  4K aliasing) are repaired; see the evidence record.
- Verified at: `9420a6a5` (same allocator source as `f4405037`): iMac correctness (compare-qjs, 5170/5170 Test262), 86-case pre-gate PASS,
  15-pair RSS max 1.091 / development geomean 1.036 (amended rule), complete K=6 panel (attempt 2).
- Evidence: [plan](performance-units/cli-size-class-allocator.json), this file, [evidence record](archive/T066-allocator-evidence.md),
  [user RSS amendment](#user-rss-amendment-2026-10-22), [user sentinel override](#user-sentinel-override-2026-10-22).
- Unresolved: formal 60-block run, held-out 30 pairs, exact configured Test262 burndown and `decide --mode promotion` are pending.
- Next action: lead runs formal → held-out → exact Test262 on the iMac, then records the decision and integrates if every non-overridden gate passes.

## User sentinel override (2026-10-22)

Attempt 2's complete K=6 panel passes every case (targets diff 0.818, graphlib 0.885, js-yaml 0.897, immutable 0.847; imaging 0.954/0.975;
every sentinel case within its band). Lane six-layout ratios: development 0.891–0.901, external 0.925–0.930, portfolio 0.945–0.951,
broad 0.982–0.992, **sentinel 1.000–1.049 vs ceiling 1.019** — the sole failure. Before this result, while repairs were still open, the user
authorized accepting "real libraries 10–19% faster with several call microbenchmarks 5–19% slower". The lead applies that authorization
to the sentinel lane ceiling only: it is a user policy override recorded as such, the frozen plan JSON is unchanged, the screen label stays
`reject` in the evidence, and every other gate (formal targets/precision, every other lane and case, held-out, RSS amended rule, Test262 zero gap)
still binds. Two of two attempts are now used; no further screens exist for this plan.

## User RSS amendment (2026-10-22)

The 3-run RSS cohort was too noisy to discriminate 5% (same base binary moved 4–10% between rounds). The lead declared and committed a
15-alternating-pair collector before running it (`tasks/diagnostics/t066/rss15.sh`, metric unchanged: median(candidate)/median(base)).
Result: big.js 1.0706 (paired IQR 1.068–1.088, +0.52 MB) and matter-js 1.0682 (1.058–1.071, +1.7 MB) exceed 1.05; graphlib 1.042,
lodash 1.024, js-yaml 1.024, meriyah 1.015, immutable 1.011, moment 1.005, papaparse 0.973, diff 0.937; ten-program geomean ≈1.016.
The lead put the trade-off to the user, who **accepted it after seeing these results**: for this unit the manual RSS check becomes
per-case median ratio ≤1.10 and development geomean ≤1.05 (the independent T065 review's recommended form). This is a user policy
decision recorded as such, not a threshold derived by the lead from the data; the frozen plan JSON is unchanged and every timing,
band, attempt and correctness gate stays as frozen.

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
  `tasks/diagnostics/t066/rss15.sh`, `tasks/diagnostics/t066/h1.sh`,
  `tasks/diagnostics/t066/h3_profile.sh`, `tasks/diagnostics/t066/counters.sh`,
  `tasks/diagnostics/t066/out_of_line.sh`,
  `crates/qjs-cli/Cargo.toml` (diagnostic `alloc-counters` feature),
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
  bump area and live count. `block & !(PAGE_SIZE - 1)` finds the header. A
  32-KiB variant was measured and reverted; see the round-2 record.
- A thread owns its pages through per-class lists, a queue of pages with free
  blocks, and at most one page with bump space. Allocation is O(1): a queued
  free block first, then the bump page, then a new page. The class list is
  scanned only when a per-class remote-free epoch changed, so single-threaded
  programs never pay that scan. An empty bump page is kept rather than
  recycled, so a 0-live/1-live oscillation cannot recycle it repeatedly.
- Page coloring (declared rule, no offset search): the c-th page a thread creates
  starts its block area at `HEADER_SIZE + (c mod 16) * 64` bytes and holds
  correspondingly fewer blocks. Every page is 64-KiB aligned with the same
  header offset, so without coloring the first (hot, long-lived) blocks of every
  class share identical low address bits; on the Intel measurement host that
  aliases L1 sets and lets equal low 12 bits create 4K store/load false
  dependencies.
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
