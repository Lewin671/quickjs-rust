# T066 allocator evidence

- Status: evidence record; 2026-10-22.
- Implementation: present at `51ef019d` on `agent/t066-size-class-allocator/cmd`;
  allocator and its unit tests are in `crates/qjs-cli`.
- Verified at: `51ef019d`, 2026-10-22; local full gate with Test262 deferred
  (worktree submodules uninitialized) plus iMac jobs for Test262/compare-qjs,
  the 86-case instruction pre-gate, the RSS cohort and diagnostic triage.
- Evidence: this file; frozen [plan](../performance-units/cli-size-class-allocator.json);
  raw artifacts under workspace `outputs/imac-bench/<job>/` (not retained in Git).
- Unresolved: peak RSS is above 1.05 on three development cases; triage is
  `unclear` with four sentinel possible regressions. Acceptance is the lead's.
- Next action: lead runs the K=6 screen and formal decision, and reviews the
  RSS overage; T066 does not claim promotion.

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

## Implementation

`crates/qjs-cli/src/allocator.rs`, 549 lines including 281 lines of tests;
`main.rs` gains the module and the `#[global_allocator]` static. No runtime
crate, workspace manifest, dependency, policy or third-party file changed.

- 32 classes: 16-byte steps to 128 B, then four equal classes per octave to
  8 KiB (160, 192, 224, 256, 320, ..., 8192). Worst internal fragmentation is
  one 16-byte step at or below 128 B and one quarter-step above it.
- A layout with size above 8 KiB or alignment above 16 is System-managed, as
  is a zero-sized layout. Class blocks are 16-aligned and are multiples of 16
  bytes apart, so they satisfy every alignment up to 16.
- Per-thread free lists (singly linked through the block's first word) and
  per-class 16-KiB bump slabs. Slab allocation is `#[cold] #[inline(never)]`,
  keeping the pop/carve path small.
- `realloc` returns the same block within one class, forwards System-to-System
  to `System.realloc` (the trait fixes alignment), and otherwise allocates,
  copies `min(old, new)` and frees.
- `alloc_zeroed` clears the whole class of a recycled block and forwards
  System-managed layouts.
- The pool is const-initialized with no destructor, so TLS is never observed
  destroyed; slabs and free lists are retained to process exit. A cross-thread
  free pushes onto the freeing thread's list, transferring ownership; a thread
  that exits leaks its retained blocks, which is sound.
- Nine unit tests: class-table density/monotonicity/fragmentation bounds,
  System routing for over-aligned/large layouts, every class's alignment and
  usable bytes, same-class realloc, all cross-class/System realloc paths,
  over-aligned realloc, recycled-block zeroing, 4,000 cross-thread frees with
  per-thread recycling, and a 20,000-step mixed alloc/realloc/free stress with
  content and alignment checks.

Design exploration (local, non-canonical builds): geometric 4→64 KiB first
slabs did not change peak RSS and a local 8-classes-per-octave table raised
absolute RSS slightly (js-yaml 33.8 → 34.1 MB), so the frozen 32-class,
16-KiB-slab shape was kept. These probes are not timing or acceptance evidence.

## Verification

Local gate at `51ef019d`: `QJS_CHECK_SKIP_TEST262=1 ./scripts/check.sh` passed
(format, clippy, default/agents/perf-counters lints and tests, workspace tests,
tool tests, plan checks, context, script tests, hooks, file size). Test262 was
deferred only because this worktree's `third_party` submodules are not
initialized; the exact candidate ran it on the iMac.

| Check | Result |
| --- | --- |
| `cargo test -p qjs-cli` | 27 passed (22 unit + 5 integration), 0 failed |
| `cargo clippy -p qjs-cli --all-targets -- -D warnings` | clean |
| iMac job `20261022T111044Z-t066c-correctness` | exit 0: `compare-qjs` all cases ok, Test262 subset 5170/5170 |
| Candidate binary (canonical recipe) | `3fafdd9beab455a3704eab88d9f1315383661b8817cca0eee52810531d210f1a` |
| Base binary | `f5822aa32e4c7a3def2aaf5b8eff2107ba005a76252afb25f06077fe25f2e8eb` |

## Instruction pre-gate (all 86 policy cases, one pair, non-timing)

Final candidate `51ef019d`: **PASS**, no case over `max(1.03, I_i)` (all cutoffs
1.03). 6 ratios are above 1. Worst first:

| Case | Instructions candidate/M |
| --- | ---: |
| `external/kraken-1.1/audio-beat-detection` | 1.023785 |
| `broad/captured_write` | 1.007373 |
| `broad/property_dynamic_read` | 1.001563 |
| `external/kraken-1.1/audio-oscillator` | 1.001211 |
| `broad/local_read` | 1.000093 |
| `broad/global_read` | 1.000026 |
| `sentinel/recursive_call_tree` | 0.999978 |
| `broad/math_abs` | 0.999973 |

Median ratio 0.985685. Largest reductions: `json-parse-financial` 0.5308,
`string_slice` 0.6297, `string-unpack-code` 0.6362, `date-format-tofte`
0.7543, `string-fasta` 0.7646. Job `20261022T111042Z-t066c-pregate`
(exit 0, 264 s); artifacts `images.json` `e082eb21…`, `pregate.json`
`4fb65b36…`, `verdict.json` `da9e67a4…`.

Superseded runs are retained: `f606fcee` (64-KiB slabs) also passed, worst
`broad/captured_write` 1.007460, candidate `61cbf91c…`, job
`20261022T105205Z-t066-pregate`. `41300aa4` (geometric slabs) **blocked** on
`broad/array_index_of` 1.032872, candidate `b196dd58…`, job
`20261022T110326Z-t066b-pregate`; that shape was replaced.

## Peak RSS (max RSS from wait4, three interleaved runs, medians)

Final candidate `51ef019d` versus M. The unit's manual ceiling is 1.05.

| Development case | Base bytes | Candidate bytes | Ratio | Result |
| --- | ---: | ---: | ---: | --- |
| `big.js/arith` | 7,417,856 | 8,163,328 | 1.100497 | **over** |
| `matter-js/simulate` | 24,895,488 | 26,640,384 | 1.070089 | **over** |
| `js-yaml/document` (target) | 28,401,664 | 30,253,056 | 1.065186 | **over** |
| `graphlib/paths` (target) | 28,889,088 | 29,650,944 | 1.026372 | ok |
| `immutable/persistent` (target) | 37,785,600 | 38,756,352 | 1.025691 | ok |
| `diff/compare` (target) | 13,299,712 | 13,541,376 | 1.018171 | ok |
| `meriyah/parse` | 130,551,808 | 131,600,384 | 1.008032 | ok |
| `moment/dates` | 19,095,552 | 19,238,912 | 1.007508 | ok |
| `papaparse/csv` | 104,886,272 | 103,907,328 | 0.990667 | ok |
| `lodash/transform` | 76,718,080 | 75,288,576 | 0.981367 | ok |

Three of ten are over the ceiling; the absolute excess is 0.75 MB (big.js),
1.74 MB (matter-js) and 1.85 MB (js-yaml). Slab sizing is not the cause: the
64-KiB job measured 1.098434 and 1.057842 for big.js/js-yaml, and the geometric
job 1.097887 and 1.061156; matter-js measured 1.000658, 1.008886 and 1.070089
across the three shapes, so its ratio is also sensitive to the allocator's
address/retention behavior. The residual is retained free blocks and class
quantization, which a slab-size change cannot remove. Job
`20261022T111043Z-t066c-rss` (exit 3), artifacts `images.json` `6454b3ca…`,
`rss.json` `5fe8352b…`, `verdict.json` `c62ca215…`.

## Diagnostic quick triage (five natural pairs, v5 workers-4)

Candidate `51ef019d` versus M, job `20261022T111045Z-t066c-triage`, exit 0,
283 s end-to-end, budget met, 110/110 valid samples, no obvious regressions,
verdict `unclear` (four sentinel possible regressions). Request SHA-256
`144c36da…`, `triage.json` `804f0ca1…`, `triage.md` `50bb8df6…`.

| Case | Wall ratio [95% exploratory] | Instructions |
| --- | ---: | ---: |
| `development/diff/compare` (target) | 0.8381 [0.7876, 0.8952] | 0.7782 |
| `development/graphlib/paths` (target) | 0.9130 [0.8708, 0.9272] | 0.8486 |
| `development/js-yaml/document` (target) | 0.9162 [0.8689, 0.9291] | 0.8305 |
| `development/immutable/persistent` (target) | 0.8852 [0.8654, 0.8909] | 0.8442 |
| `sentinel/capturing_closure_call` | 1.0037 [0.9928, 1.0515] | 0.9989 |
| `sentinel/heterogeneous_property_read` | 1.0051 [0.9313, 1.0062] | 1.0000 |
| `sentinel/polymorphic_call_site` | 1.0333 [0.9837, 1.0667] | 0.9988 |
| `sentinel/prototype_method_call` | 1.0082 [0.9667, 1.0196] | 0.9993 |
| `sentinel/recursive_call_tree` | 1.0352 [1.0041, 1.0502] | 1.0000 |
| `sentinel/string_key_map_churn` | 1.0152 [0.9841, 1.1876] | 1.0001 |
| `broad/property_dynamic_read` | 1.0135 [1.0047, 1.0276] | 1.0016 |
| `broad/object_allocation` | 1.0053 [0.9576, 1.0086] | 0.9863 |
| `broad/string_slice` | 0.5981 [0.5754, 0.7141] | 0.6296 |
| `broad/top_level_function_call` | 0.9904 [0.9151, 1.0226] | 0.9991 |
| `development/lodash/transform` | 0.9240 [0.9101, 0.9390] | 0.9224 |
| `development/moment/dates` | 0.9217 [0.9082, 0.9571] | 0.8707 |
| `external/kraken-1.1/json-stringify-tinderbox` | 0.7639 [0.7462, 0.7831] | 0.7526 |
| `external/kraken-1.1/json-parse-financial` | 0.4915 [0.4893, 0.4986] | 0.5309 |
| `external/jetstream3-js-subset/raytrace-public-class-fields` | 0.8506 [0.8298, 0.8672] | 0.8810 |
| `external/sunspider-1.0/access-binary-trees` | 0.8882 [0.8758, 0.9104] | 0.8903 |
| `external/kraken-1.1/audio-dft` | 0.9741 [0.9722, 1.0076] | 0.9921 |
| `external/jetstream3-js-subset/cdjs` | 0.8845 [0.8652, 0.8958] | 0.8826 |

Every target's exploratory interval lies below its frozen payoff, and the two
previously suspected sentinels are far smaller than in the ablation:
`recursive_call_tree` 1.0352 (was 1.06) and `property_dynamic_read` 1.0135 (was
1.04). This is diagnostic: it cannot promote, spend an attempt, or replace the
K=6 screen, formal run or held-out acceptance. Two earlier triage submissions
(`20261022T105229Z` on `f606fcee`, `20261022T110333Z` on `41300aa4`) were
cancelled by the owner before timing and produced no result.

## Open risks

- Peak-RSS ceiling: `big.js/arith` 1.100, `matter-js/simulate` 1.070 and the
  target `js-yaml/document` 1.065 exceed 1.05. A production allocator that
  retains freed blocks cannot return them without per-slab liveness tracking;
  that is a follow-up design, not a retune of this unit. The lead decides
  whether the ceiling blocks promotion pending review.
- Peak-RSS medians are sensitive to allocator layout: `matter-js` moved from
  1.001 to 1.070 across three allocator shapes with the same class table, so a
  single 3-run median is not a tight estimate.
- Four sentinels are triage "possible regressions" (1.004-1.035 wall); the
  frozen screen's control rule, not this diagnostic, decides them.
- The allocator is CLI-only by design; library embedders keep their allocator,
  so engine-library consumers do not get these gains.
