# T066 allocator evidence

- Status: evidence record; 2026-10-22; screen attempt 1 rejected, repair in progress.
- Implementation: present at `52dd420d` on `agent/t066-size-class-allocator/cmd`;
  64-KiB pages with per-class free-page queues, page recycling and an atomic
  cross-thread remote list. The compiled candidate is byte-identical to
  `7e837fa8…` (verified by the pre-gate's candidate hash).
- Verified at: `52dd420d` / binary `7e837fa8…`, 2026-10-22; local full gate with
  Test262 deferred (submodules uninitialized), iMac correctness (compare-qjs and
  5170/5170 Test262), 86-case instruction pre-gate PASS, RSS cohort and
  diagnostic triage.
- Evidence: this file; frozen [plan](../performance-units/cli-size-class-allocator.json);
  raw artifacts under workspace `outputs/imac-bench/<job>/` (not retained in Git).
- Unresolved: RSS remains above 1.05 on `lodash/transform`, `big.js/arith` and
  `matter-js/simulate`; the per-case ratio is dominated by whole-process variance
  in the base medians (4-10% between cohorts with the same base binary).
  `graphlib/paths` fell from 1.073 to 1.018, so the diagnosed stranding is fixed.
- Next action: record attempt 1, diagnose the wall regressions (H1 page churn,
  H2 recycle thrash, H3 aliasing), apply the smallest supported fix and verify;
  the second and final attempt is the lead's call.

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

## Implementation, round 0 (superseded)

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

## Verification (round 0)

Local gate: `QJS_CHECK_SKIP_TEST262=1 ./scripts/check.sh` passed at
`f606fcee` and again at `13507c15`, the documentation-only commit whose code is
the final `51ef019d` (format, clippy, default/agents/perf-counters lints and
tests, workspace tests, tool tests, plan checks, context, script tests, hooks,
file size). At `51ef019d` the focused `cargo test -p qjs-cli` and clippy also
passed. Test262 was deferred only because this worktree's `third_party`
submodules are not initialized; the exact candidate ran it on the iMac.

| Check | Result |
| --- | --- |
| `cargo test -p qjs-cli` | 27 passed (22 unit + 5 integration), 0 failed |
| `cargo clippy -p qjs-cli --all-targets -- -D warnings` | clean |
| iMac job `20261022T111044Z-t066c-correctness` | exit 0: `compare-qjs` all cases ok, Test262 subset 5170/5170 |
| Candidate binary (canonical recipe) | `3fafdd9beab455a3704eab88d9f1315383661b8817cca0eee52810531d210f1a` |
| Base binary | `f5822aa32e4c7a3def2aaf5b8eff2107ba005a76252afb25f06077fe25f2e8eb` |

## Instruction pre-gate, round 0 (all 86 policy cases, one pair, non-timing)

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

## Peak RSS, round 0 (max RSS from wait4, three interleaved runs, medians)

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

## Diagnostic quick triage, round 0 (five natural pairs, v5 workers-4)

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

## RSS round 1: page-structured recycling (lead decision, 2026-10-22)

Authorised change: replace per-class retaining slabs with mimalloc-style pages.
The allocator now takes one 64-KiB, 64-KiB-aligned page per class from System,
keeps a page header at the base (so `block & !(PAGE_SIZE - 1)` finds it), and
gives each page its own free list, bump area and live count. A thread owns its
pages through per-class lists and a current page; allocation takes the current
page's free list, then its bump space, then its drained remote list. A free
updates the owning page's live count; when it reaches zero the page is unlinked
and either parked in a thread-local empty pool of at most 8 pages (reusable by
any class) or returned to System with its own page layout. Cross-thread frees
push onto the page's atomic remote list and leave `live` alone; the owner
subtracts the drained count, so `live == 0` proves no block is allocated, no
remote entry is queued and no future remote push is possible. The page is only
recycled under that proof. Class sizes, `realloc`, `alloc_zeroed`, OOM-null,
destructor-free TLS and the audited-unsafe discipline are unchanged.

Code: `crates/qjs-cli/src/allocator.rs` (908 lines including 400 test lines);
commits `5afe88ed` (page structure) and `abd6451b` (bump before the remote list,
avoiding a locked exchange per allocation).

### Verification

| Check | Result |
| --- | --- |
| `cargo test -p qjs-cli` | 30 passed (25 unit + 5 integration), 0 failed |
| `QJS_CHECK_SKIP_TEST262=1 ./scripts/check.sh` | passed locally (Test262 deferred: submodules uninitialized) |
| iMac job `20261022T115136Z-t066e-correctness` | exit 0: `compare-qjs` all cases ok, Test262 subset 5170/5170 |
| Candidate binary (canonical recipe) | `eb442f1f075d1ec46db6cee5d9edb1552cc9c2250b4f7e32de1af1edcfcfd656` |
| Base binary | `f5822aa32e4c7a3def2aaf5b8eff2107ba005a76252afb25f06077fe25f2e8eb` |

Page-specific tests: empty pages are reused by another class, a remote-free
block is drained and returned to the owner with its payload intact, and live
counts track local allocations and frees on a shared page. The earlier tests
for class mapping, alignment, realloc paths, zeroing, cross-thread frees and the
mixed stress run still pass.

### Instruction pre-gate

Candidate `abd6451b`: **PASS** over all 86 policy cases; 12 ratios above 1 and
none over `max(1.03, I_i)`. Worst first: `external/kraken-1.1/audio-beat-detection`
1.029090, `broad/array_index_of` 1.025529, `broad/captured_write` 1.013356,
`external/kraken-1.1/audio-oscillator` 1.011402, `broad/property_dynamic_read`
1.009233; median 0.991394. Job `20261022T115139Z-t066e-pregate` (exit 0,
266 s); artifacts `images.json` `85fc30b1…`, `pregate.json` `e16e7386…`,
`verdict.json` `7d72aa15…`. The margin on `audio-beat-detection` is 0.0009.

### Peak RSS

| Development case | Base bytes | Candidate bytes | Ratio | Result |
| --- | ---: | ---: | ---: | --- |
| `graphlib/paths` (target) | 29,675,520 | 31,850,496 | 1.073292 | **over** |
| `big.js/arith` | 7,516,160 | 7,946,240 | 1.057221 | **over** |
| `lodash/transform` | 78,598,144 | 81,145,856 | 1.032414 | ok |
| `immutable/persistent` (target) | 38,445,056 | 39,153,664 | 1.018432 | ok |
| `js-yaml/document` (target) | 28,839,936 | 29,200,384 | 1.012498 | ok |
| `papaparse/csv` | 108,945,408 | 110,223,360 | 1.011730 | ok |
| `meriyah/parse` | 130,854,912 | 131,870,720 | 1.007763 | ok |
| `moment/dates` | 19,222,528 | 19,206,144 | 0.999148 | ok |
| `matter-js/simulate` | 27,680,768 | 26,714,112 | 0.965078 | ok |
| `diff/compare` (target) | 13,930,496 | 12,738,560 | 0.914437 | ok |

Job `20261022T115140Z-t066e-rss` (exit 3), artifacts `images.json`
`23b7747b…`, `rss.json` `11de68f7…`, `verdict.json` `c55e33d3…`.

The round-1 shape fixed two of the three round-0 overages: `js-yaml` 1.065 →
1.012 and `matter-js` 1.070 → 0.965. Two remain: `big.js` 1.100 → 1.057 (still
0.43 MB over) and `graphlib` 1.026 → 1.073 (now 2.17 MB over). The cause is the
page allocator's reuse order, not class sizing: allocation prefers the current
page's bump space, so free blocks sitting on non-current pages of the same class
are reused only when the current page fills. Graphlib's bursty graph
construction leaves a few live blocks on many pages, pinning them; the freed
blocks on those pages cannot be pooled class-wide as they were in round 0, so
the same live set needs more page memory. `big.js` remains over from class
quantization on a 7.5-MB process. Per the lead decision this is reported, not
retuned.

### Diagnostic quick triage

Candidate `abd6451b` versus M, job `20261022T115141Z-t066e-triage`, exit 0,
277 s end-to-end, budget met, 110/110 valid samples, no obvious regressions,
verdict `unclear` (six possible regressions). Request SHA-256 `9b7706da…`,
`triage.json` `cce5f36b…`, `triage.md` `169e36f5…`.

| Case | Wall ratio [95% exploratory] | Instructions |
| --- | ---: | ---: |
| `development/diff/compare` (target) | 0.8095 [0.7880, 0.8860] | 0.8136 |
| `development/graphlib/paths` (target) | 0.8726 [0.8542, 0.9174] | 0.8689 |
| `development/js-yaml/document` (target) | 0.9331 [0.8983, 0.9592] | 0.8512 |
| `development/immutable/persistent` (target) | 0.8714 [0.8526, 0.8805] | 0.8659 |
| `sentinel/capturing_closure_call` | 1.0283 [0.9937, 1.0491] | 0.9988 |
| `sentinel/heterogeneous_property_read` | 1.0224 [0.9967, 1.0459] | 1.0000 |
| `sentinel/polymorphic_call_site` | 1.0539 [1.0283, 1.0771] | 0.9988 |
| `sentinel/prototype_method_call` | 0.9937 [0.9718, 1.0102] | 0.9993 |
| `sentinel/recursive_call_tree` | 1.0329 [0.9504, 1.0765] | 1.0001 |
| `sentinel/string_key_map_churn` | 1.0496 [0.6897, 1.1304] | 1.0002 |
| `broad/property_dynamic_read` | 1.0162 [1.0048, 1.1387] | 1.0093 |
| `broad/object_allocation` | 0.9956 [0.9763, 1.1374] | 1.0009 |
| `broad/string_slice` | 0.6390 [0.5530, 0.6470] | 0.6974 |
| `broad/top_level_function_call` | 1.0301 [0.9900, 1.0598] | 0.9991 |
| `development/lodash/transform` | 0.9256 [0.9089, 0.9410] | 0.9373 |
| `development/moment/dates` | 0.9073 [0.8864, 0.9132] | 0.8926 |
| `external/kraken-1.1/json-stringify-tinderbox` | 0.7837 [0.7746, 0.7919] | 0.7837 |
| `external/kraken-1.1/json-parse-financial` | 0.5322 [0.5299, 0.5406] | 0.5847 |
| `external/jetstream3-js-subset/raytrace-public-class-fields` | 0.8649 [0.8489, 0.8898] | 0.8953 |
| `external/sunspider-1.0/access-binary-trees` | 0.9220 [0.8975, 0.9369] | 0.9037 |
| `external/kraken-1.1/audio-dft` | 0.9915 [0.9834, 1.0091] | 0.9937 |
| `external/jetstream3-js-subset/cdjs` | 0.9023 [0.8929, 0.9121] | 0.9035 |

All four targets still beat their frozen payoffs and their exploratory intervals
stay below them; diff, graphlib and immutable improved against the round-0
triage, js-yaml was slightly slower. `polymorphic_call_site` (1.0539, lower bound
1.0283) is the only sentinel whose exploratory wall interval excludes 1.
Diagnostic only; it cannot promote, spend an attempt, or replace K=6, the formal
run or held-out acceptance.

## RSS round 2: reuse before extend, fixed overhead probe (lead decision 2)

Authorised steps: (1) keep a per-class queue of pages with free blocks and reuse
them before extending; (2) reduce fixed per-class overhead only if (1) leaves
`big.js` over.

### Step 1: reuse before extend

Each class now has a doubly linked queue of its pages with free blocks, most
recently freed first. A local free enqueues its page; allocation takes a queued
free block before bump-allocating, so blocks freed on non-current pages are no
longer stranded until the current page fills. The current page is bumped only
when no page has a free block; the allocator then scans remote lists and
stranded bump space and takes an empty or new page last. Commit `15e158fe`,
with a new test that a free block on a non-current page is returned before bump
space. The rest of the round-1 page design is unchanged.

### Step 2: fixed-overhead probe and its rejection

`big.js` was still over after step 1, so the fixed-overhead option was tried:
uniform 32-KiB pages (commit `b50a1673`). A uniform size preserves the mask and
one-class-per-page invariants, halves each class's first-page slack, and still
gives the 8-KiB class three blocks per page; 16 KiB would hold one block and put
a System allocation on that path. It was reverted at `52dd420d`: its 86-case
pre-gate **blocked** on `external/kraken-1.1/imaging-darkroom` at 1.033707
(the same code with 64-KiB pages measured 0.994910), a 3.9% whole-process
instruction change caused by fat-LTO codegen from the constant, and the local
peak-RSS probe showed no clear `big.js` benefit. Correctness passed on the
32-KiB build; its RSS and triage jobs were cancelled by the owner as superseded.
The final allocator source is byte-identical to `15e158fe`'s.

### Verification (final candidate)

| Check | Result |
| --- | --- |
| `cargo test -p qjs-cli` | 31 passed (26 unit + 5 integration), 0 failed |
| `QJS_CHECK_SKIP_TEST262=1 ./scripts/check.sh` | passed locally (Test262 deferred: submodules uninitialized) |
| iMac correctness `20261022T122233Z` | exit 0: `compare-qjs` all cases ok, Test262 subset 5170/5170 |
| iMac pre-gate `20261022T124829Z` | PASS, 86/86; candidate `7e837fa8…` |
| Base binary | `f5822aa32e4c7a3def2aaf5b8eff2107ba005a76252afb25f06077fe25f2e8eb` |

The pre-gate's candidate hash `7e837fa8899ef5ec0ee6dc45fe61d0d083e88b947c6feb456908f91573b833fb`
matches the candidate used by the RSS and triage jobs below, so those jobs
describe this exact executable. Pre-gate worst cases: `broad/array_index_of`
1.025582, `broad/captured_write` 1.013318, `external/kraken-1.1/audio-oscillator`
1.011874, `broad/property_dynamic_read` 1.009241, `jetstream3-js-subset/hash-map`
1.006023; median 0.993166, 16 ratios above 1. Artifacts `images.json`
`8dc9de42…`, `pregate.json` `fa916e76…`, `verdict.json` `57aeec49…`. An earlier
run of the same binary (`20261022T122237Z`) also passed with worst 1.025492.

### Peak RSS (job `20261022T122239Z`, same candidate)

| Development case | Base bytes | Candidate bytes | Ratio | Result |
| --- | ---: | ---: | ---: | --- |
| `lodash/transform` | 73,416,704 | 80,666,624 | 1.098750 | **over** |
| `big.js/arith` | 7,217,152 | 7,892,992 | 1.093644 | **over** |
| `matter-js/simulate` | 24,858,624 | 26,779,648 | 1.077278 | **over** |
| `meriyah/parse` | 130,732,032 | 134,541,312 | 1.029138 | ok |
| `graphlib/paths` (target) | 29,556,736 | 30,089,216 | 1.018016 | ok |
| `js-yaml/document` (target) | 28,549,120 | 29,024,256 | 1.016643 | ok |
| `immutable/persistent` (target) | 38,240,256 | 38,772,736 | 1.013925 | ok |
| `papaparse/csv` | 108,810,240 | 108,625,920 | 0.998306 | ok |
| `moment/dates` | 19,300,352 | 19,185,664 | 0.994058 | ok |
| `diff/compare` (target) | 13,451,264 | 12,476,416 | 0.927527 | ok |

Job `20261022T122239Z-t066f-rss` (exit 3); artifacts `images.json`
`843d9800…`, `rss.json` `0ed7c1c4…`, `verdict.json` `f751ba62…`.

Reuse-before-extend fixed the diagnosed stranding: `graphlib/paths`'s candidate
fell from 31.85 MB to 30.09 MB and its ratio from 1.073 to 1.018. The remaining
over-set is not explained by the candidate growing. Against round 1 the
candidate bytes are flat or lower (`big.js` 7.95 → 7.89 MB, `matter-js` 26.64 →
26.78 MB, `lodash` 81.15 → 80.67 MB), while the *base* medians moved by 4-10%
with the same base binary (`big.js` 7.52 → 7.22 MB, `matter-js` 27.68 → 24.86 MB,
`lodash` 78.60 → 73.42 MB), consistent with per-process randomized hash
seeds/ASLR changing heap layout. The per-case ratio at three runs is therefore a
noisy discriminator near the 1.05 ceiling; `matter-js` measured 1.070 for the
round-0 allocator, 0.965 for round 1 and 1.077 for round 2.

### Diagnostic quick triage (job `20261022T122240Z`, same candidate)

Exit 0, 277 s end-to-end, budget met, 110/110 valid, verdict `likely loss`
(not `unclear`): `capturing_closure_call`'s wall lower bound (1.0510) exceeds
its policy band, which the heuristic counts as a loss. Request SHA-256
`0be9f789…`, `triage.json` `31d96ae8…`, `triage.md` `297a23bc…`; candidate
`7e837fa8…`.

| Case | Wall ratio [95% exploratory] | Instructions |
| --- | ---: | ---: |
| `development/diff/compare` (target) | 0.8323 [0.7896, 0.8933] | 0.8195 |
| `development/graphlib/paths` (target) | 0.8940 [0.8546, 0.9661] | 0.8734 |
| `development/js-yaml/document` (target) | 0.9393 [0.8934, 0.9637] | 0.8569 |
| `development/immutable/persistent` (target) | 0.9030 [0.8776, 0.9065] | 0.8699 |
| `sentinel/capturing_closure_call` | 1.0663 [1.0510, 1.1346] | 0.9990 |
| `sentinel/heterogeneous_property_read` | 0.9754 [0.8774, 1.0630] | 1.0000 |
| `sentinel/polymorphic_call_site` | 1.0343 [0.9853, 1.0568] | 0.9988 |
| `sentinel/prototype_method_call` | 1.0428 [0.9339, 1.0896] | 0.9992 |
| `sentinel/recursive_call_tree` | 1.0199 [0.9739, 1.1133] | 1.0000 |
| `sentinel/string_key_map_churn` | 0.9945 [0.9337, 1.0743] | 1.0000 |
| `broad/property_dynamic_read` | 1.0145 [0.7681, 1.0357] | 1.0093 |
| `broad/object_allocation` | 0.9941 [0.9736, 1.0068] | 1.0009 |
| `broad/string_slice` | 0.6261 [0.5970, 0.6361] | 0.7068 |
| `broad/top_level_function_call` | 1.0269 [1.0113, 1.1145] | 0.9992 |
| `development/lodash/transform` | 0.9446 [0.9273, 0.9571] | 0.9402 |
| `development/moment/dates` | 0.9176 [0.9167, 0.9285] | 0.8963 |
| `external/kraken-1.1/json-stringify-tinderbox` | 0.8091 [0.7992, 0.8196] | 0.7939 |
| `external/kraken-1.1/json-parse-financial` | 0.5522 [0.5239, 0.5679] | 0.5971 |
| `external/jetstream3-js-subset/raytrace-public-class-fields` | 0.8823 [0.8615, 0.8940] | 0.8987 |
| `external/sunspider-1.0/access-binary-trees` | 0.9145 [0.9007, 0.9251] | 0.9069 |
| `external/kraken-1.1/audio-dft` | 0.9846 [0.9794, 1.0269] | 0.9941 |
| `external/jetstream3-js-subset/cdjs` | 0.9351 [0.9244, 0.9432] | 0.9092 |

All four targets keep wall medians below their frozen payoffs; `graphlib`'s
exploratory upper bound (0.9661) crosses its payoff (0.9523), so it is not a
clean five-pair win in this round. `capturing_closure_call` (1.0663, lower bound
1.0510) is the single "loss" that makes the verdict `likely loss`;
`top_level_function_call` (1.0269, lower bound 1.0113) is the only other case
whose wall interval excludes 1. Diagnostic only.

### Stop

The lead declared this the final round. Both authorised steps were carried out;
step 2 was measured and reverted on pre-gate evidence. No further retuning and no
plan change. The unit still has to be judged by the K=6 screen, formal run and
held-out acceptance, which were not run here.

## Attempt 1 of 2: valid K=6 rejection

Screen job `20261022T140141Z-t066-screen-attempt1` (candidate `9b26bf52`, whose
allocator source is `52dd420d`; unit SHA-256 `be05d5e8…`) completed a full K=6
panel and was labelled **reject**. This is a valid failed screen: attempt 1 of 2
is spent. Artifacts: `verdict.json` `50fa5e4a…`, `screen/panel.json`
`f06758b1…`, `screen/complete.json` `7df1b19c…`.

| Case | Wall median | Wall band | Instructions | Label |
| --- | ---: | ---: | ---: | --- |
| `external/kraken-1.1/imaging-darkroom` | 1.534911 | 1.070312 | 0.992548 | reject: persistent six-layout wall cost |
| `external/kraken-1.1/imaging-desaturate` | 1.418010 | 1.064339 | 0.996851 | reject: persistent six-layout wall cost |
| `sentinel/capturing_closure_call` | 1.067181 | 1.050127 | 0.998908 | reject: persistent six-layout wall cost |
| `development/diff/compare` (target) | 0.836267 | 1.099110 | 0.819517 | pass |
| `development/graphlib/paths` (target) | 0.895801 | 1.050127 | 0.873768 | pass |
| `development/js-yaml/document` (target) | 0.888757 | 1.050127 | 0.856288 | pass |
| `development/immutable/persistent` (target) | 0.872222 | 1.076243 | 0.870358 | pass |

The sentinel lane exceeded its 1.018672 ceiling with per-layout ratios
1.0070-1.0354, so the verdict also carries a lane event. Targets met both their
wall payoff and the six-layout aggregate; the development lane measured
0.9049-0.9152 (ceiling 1.009347), the portfolio 0.9623-0.9645 (ceiling
1.006849), broad 0.9825-0.9915 and external 0.9550-0.9567. The three failures
have flat instructions (0.992-0.999) against +7% to +53% wall, which points at
kernel/memory-system cost rather than user code.

## Open risks

- Peak-RSS ceiling: after round 2 the over-set is `lodash/transform` 1.099,
  `big.js/arith` 1.094 and `matter-js/simulate` 1.077. The candidate's absolute
  bytes did not grow between rounds; the base medians moved 4-10% with the same
  base binary. A three-run per-process RSS ratio is not a stable discriminator
  at the 1.05 ceiling, and the lead's stop rule applies.
- The 32-KiB fixed-overhead probe blocked the instruction pre-gate on
  `imaging-darkroom` (1.0337 versus 0.9949 for the same code with 64-KiB pages),
  a fat-LTO codegen effect from a constant change, and was reverted.
- Triage: `capturing_closure_call` (1.066) and `top_level_function_call` (1.027)
  have exploratory intervals above 1; `graphlib`'s target interval crosses its
  payoff. The frozen screen, not this diagnostic, decides them.
- The allocator is CLI-only by design; library embedders keep their allocator,
  so engine-library consumers do not get these gains.
