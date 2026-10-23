# T065 rounds 1–3 mechanism ablation

Prospective plan committed as `7453fe25` before scratch implementation:
[Ablation ladder](../T065-interpreter-core-v3.md#ablation-ladder).
Diagnostic only, zero acceptance attempts. No origin push, merge, policy,
plan or production runtime change on the documentation branch.

## Source and limits

Base `db64cab00511033b7818cdfbae5bd32a6b0f7758`; canonical binary SHA-256
`f5822aa32e4c7a3def2aaf5b8eff2107ba005a76252afb25f06077fe25f2e8eb`,
byte-identical to measured main `9eba7a1d`. All builds use Rust 1.95.0,
LLVM 22.1.2, x86_64 macOS, agents/fat LTO/16 CGUs/abort, no perf-counters.
Fixed four targets and 19 controls are declared in the ladder; no case dropped.

| Mechanism | Scratch branch (all `agent/t065-abl-…/codex`) / commit | What is priced | Soundness and interpretation |
| --- | --- | --- | --- |
| Rust size classes | `sizeclass` / `3331cd7c` | CLI global allocator, 16–2048-byte classes; 64-KiB slabs, bump allocation and returned-block free lists; System for larger/over-aligned layouts | Realistic allocator fast-path prototype; unreclaimed slabs and TLS retention preclude production resource claims. No new dependency/FFI. |
| C allocator | `mimalloc` / `b042f9ab` | CLI global mimalloc 0.1.48, libmimalloc-sys 0.1.49, existing dependency versions preserved | Never-landing diagnostic alternative, not a mathematical upper bound. C/FFI violates normal engine policy and is used only under the brief's explicit exception. Five locked packages added (one direct dependency and four transitives) for this experiment. |
| Call entry | `call` / `117082ed` | Existing narrower M2.1 pricing prototype: typed activation services and indexed entry; nested VM, argument vectors, native boundaries remain | Partial realistic prototype, not an oracle for all ordinary calls or later M2 stages. See [M2 price and cohort limits](T056-m2-plan-inputs.md). |
| Value ownership | `ownership` / `6fc9c03d` | Immortal Rc-backed Function, Object, Array, Proxy, Map, Set and JsString handles; bit-copy clone and empty drop, weak upgrades retain std Rc behavior | Deliberately incomplete ownership oracle. Changes weak liveness/uniqueness; strings always copy before mutation. BigInt and auxiliary Rc ownership, allocations, 16-byte Value width and enum dispatch remain. Not a mathematical wall ceiling: extra COW, leaks, RSS/locality and codegen can oppose or amplify removed count work. |

All code remains on scratch branches. The ownership oracle retains original
owners forever, omits normal last-owner teardown, and must never become an
embedding API. Its native Function initialization still uses legacy Rc mutable
access; this is not a soundness audit. Checksums constrain only measured cases.
An 8-byte value representation and production collector are **unpriced**.
Allocator and ownership experiments overlap; their savings must not be added.

## Collections

Every failed/unclear collection is retained. Round-1 wall results (lead-completed record, below) come from the three triage jobs; the first round-2 attempt stopped when Codex hit its usage limit during `20261022T101455Z-t065-ablation-r2-instructions` (no result); round 2 was re-measured by the lead (below).

- `20261022T100343Z-t065-ablation-call-wall`: done 0, triage **unclear**;
  115/115 valid pairs, no invalid pair or heavy disturbance; 405.91 s including
  quieting, 141.88 s builds and 190.52 s collection. Natural layout only.
- `20261022T100535Z-t065-ablation-r1-instructions`: done 1 after 2 s,
  before any measurement. Collector reused one exclusive `binaries-cache.json`
  path for multiple roles, raising `FileExistsError`. V2 gives each role its
  own directory. No adverse timing was discarded or repeated.
- `20261022T100547Z-t065-ablation-sizeclass-wall`: done 0.
- `20261022T100549Z-t065-ablation-mimalloc-wall`: done 0, triage likely loss (controls).
- `20261022T101127Z-t065-ablation-r1-instructions-v2`: done 0.


## Round-1 results (candidate / main, quick triage, natural layout, 5 pairs)

Diagnostic only: five natural-layout pairs per case, no layout panel, no acceptance meaning. Wall ratios below are medians of pair ratios;
95% intervals are in the triage results (`ablation-summary.json`, workspace-local; job IDs above).

| Case | Size classes instr / wall | mimalloc instr / wall | Call prototype instr / wall |
| --- | ---: | ---: | ---: |
| `development/big.js/arith` (target) | 0.984 / 0.981 | 0.984 / 0.985 | 1.000 / 1.009 |
| `development/immutable/persistent` (target) | 0.843 / 0.864 | 0.848 / 0.834 | 0.844 / 0.828 |
| `development/js-yaml/document` (target) | 0.828 / 0.863 | 0.832 / 0.894 | 0.965 / 0.954 |
| `development/graphlib/paths` (target) | 0.853 / 0.888 | 0.853 / 0.856 | 0.998 / 1.009 |
| `sentinel/capturing_closure_call` | 0.999 / 0.984 | 1.000 / 1.113 | 1.000 / 1.017 |
| `sentinel/heterogeneous_property_read` | 1.000 / 1.006 | 1.000 / 0.995 | 1.000 / 0.987 |
| `sentinel/polymorphic_call_site` | 0.999 / 0.971 | 1.000 / 1.032 | 1.000 / 0.997 |
| `sentinel/prototype_method_call` | 0.999 / 1.002 | 1.000 / 1.029 | 1.000 / 0.993 |
| `sentinel/recursive_call_tree` | 1.000 / 1.060 | 1.000 / 1.027 | 1.000 / 1.065 |
| `sentinel/string_key_map_churn` | 1.000 / 0.967 | 1.000 / 1.033 | 1.000 / 1.010 |
| `broad/property_dynamic_read` | 1.007 / 1.039 | 1.000 / 0.958 | 0.997 / 1.047 |
| `broad/object_allocation` | 0.990 / 0.985 | 1.000 / 0.993 | 0.993 / 0.984 |
| `broad/string_slice` | 0.624 / 0.546 | 0.652 / 0.572 | 1.000 / 0.995 |
| `broad/top_level_function_call` | 0.999 / 1.015 | 1.000 / 1.046 | 1.007 / 1.047 |
| `development/lodash/transform` | 0.923 / 0.939 | 0.918 / 0.933 | 0.969 / 0.970 |
| `development/diff/compare` | 0.775 / 0.804 | 0.788 / 0.810 | 0.999 / 0.996 |
| `development/moment/dates` | 0.870 / 0.904 | 0.873 / 0.882 | 0.995 / 0.974 |
| `external/kraken-1.1/json-stringify-tinderbox` | 0.749 / 0.776 | 0.772 / 0.795 | 1.001 / 0.989 |
| `external/kraken-1.1/json-parse-financial` | 0.519 / 0.491 | 0.562 / 0.516 | 1.000 / 0.986 |
| `external/jetstream3-js-subset/raytrace-public-class-fields` | 0.875 / 0.867 | 0.887 / 0.869 | 0.999 / 1.010 |
| `external/sunspider-1.0/access-binary-trees` | 0.885 / 0.891 | 0.906 / 0.899 | 0.985 / 0.988 |
| `external/kraken-1.1/audio-dft` | 0.993 / 1.008 | 0.994 / 1.030 | 1.000 / 0.987 |
| `external/jetstream3-js-subset/cdjs` | 0.885 / 0.891 | 0.887 / 0.888 | 0.953 / 0.940 |

Seven-program development wall geomean: size classes **0.890**, mimalloc **0.883**, call prototype **0.961**.

Reading: a 69-line pure-Rust thread-local size-class global allocator captures essentially all of the never-landing C allocator's gain, so
allocation speed does not need FFI. Much of the allocation cost attributed in earlier profiles is macOS system-malloc cost. The call prototype
is a large win only where its cohort applies (immutable). Lead decision: land a production allocator as performance unit T066; continue
ownership/property/bytecode ablations afterwards.

## Round-2 result: ownership oracle over the allocator (lead, 2026-10-23)

The oracle commits were cherry-picked unchanged onto post-T066 main `9f3ab85c` (branch `agent/t065-abl-ownership-r2/claude`,
candidate `7282babe`), so the price is incremental to the landed allocator. Quick triage `20261023T033351Z-t065-ownership-r2-triage`
(done 0, 261 s, budget met; request SHA-256 `8f49d354…`, triage `76cca264…`), same four targets and 19 controls, five natural-layout pairs.
Verdict label *likely loss* comes from controls, expected for a leaking oracle.

| Case | wall | instr | Case | wall | instr |
| --- | ---: | ---: | --- | ---: | ---: |
| big.js/arith | 1.027 | 0.913 | lodash/transform | 0.907 | 0.948 |
| immutable/persistent | 0.863 | 0.940 | diff/compare | 0.895 | 0.950 |
| js-yaml/document | 0.958 | 0.985 | moment/dates | 0.902 | 0.946 |
| graphlib/paths | 0.855 | 0.935 | object_allocation | 0.856 | 0.872 |
| json-parse-financial | 1.305 | 0.985 | string_slice | 1.100 | 0.915 |
| access-binary-trees | 1.039 | 0.948 | raytrace-public-class-fields | 1.066 | 0.974 |

Seven-program development geomean: wall **0.913**, instructions **0.945**. Sentinels 0.913–1.040 wall, 0.918–1.000 instructions.
Reading: removing count traffic for the covered handles is worth up to roughly 9% wall on real libraries over the allocator,
more than its instruction share (store/cache traffic). Allocation-heavy controls slow down because nothing is freed; a tracing
collector pays its own cost instead. This is an envelope for non-owning values + GC, not a price; most of the 2.38× instruction
gap to NG lies elsewhere (property, call, dispatch).

## Round-3b result: named-property lookup (Codex, 2026-10-23)

Branch `agent/t065-abl-property/codex` over main `95b6f03d`: sound outcome counters `168ce2c0` (perf-counters only; full gate
5170/5170) and never-landing oracle `a01fb4c9` (thread-local 256Ki-entry memo of (object, layout revision, key pointer) → slot or
own-miss replacing text scans/hashes on bytecode named reads/writes and ordinary prototype walks; negative entries unsound).

Counters `20261023T035859Z-t065-property-counts` (non-timing, seven checksums valid), share of entered named handlers:

| Program | hot | probe | proto | own-data | chain walk | non-Object | compares/hashes per entry |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| big.js | 15.6 | 3.9 | 1.4 | 0.0 | 0.0 | 72.6 | 0.35 / 0.03 |
| immutable | 46.9 | 23.1 | 3.9 | 2.6 | 0.0 | 16.7 | 1.56 / 0.23 |
| js-yaml | 41.6 | 3.6 | 0.1 | 34.4 | 0.0 | 20.2 | 0.37 / 0.37 |
| graphlib | 45.9 | 40.8 | 2.5 | 2.6 | 0.4 | 7.8 | 0.29 / 0.04 |
| lodash | 4.2 | 47.1 | 0.0 | 0.0 | 0.0 | 48.6 | 0.02 / 0.01 |
| diff | 5.1 | 54.3 | 8.6 | 4.4 | 14.7 | 12.7 | 7.06 / 0.08 |
| moment | 22.6 | 38.4 | 9.6 | 0.4 | 9.2 | 13.2 | 4.73 / 0.42 |

Non-Object receivers are mostly Arrays (big.js 64.9%, lodash 45.8%) and Functions (moment 11.8%); occurrence, not cost.
Triage `20261023T040414Z-t065-property-triage` exit 2, incomplete: moment failed its checksum (oracle unsoundness), its shard
never reached access-binary-trees; 21 recovered cases replay-validated (105 pairs). Development wall / instructions: big.js
1.001/1.000, immutable 1.022/1.001, js-yaml 0.991/0.990, graphlib 1.007/0.998, lodash 1.015/1.000, **diff 0.947/0.934**;
controls json-parse 1.162/1.036, json-stringify 1.079/1.030 (memo overhead). No seven-program geomean exists.

Lead reading: text compare/hash in named lookup is a lever only for diff (and probably moment); for the other five libraries
caches already answer most accesses. Named-key authority is **not** the main source of the 2.38× instruction gap; a full
canonical-atom store is deferred. A narrow sound receiver-miss proof / Small-store canonical key for diff/moment stays a
candidate incremental unit.

## Round-3a result: general `Op` size (dsh, 2026-10-23)

Branch `agent/t065-abl-opsize/dsh`, sound prototype `9401c478` over main `95b6f03d`: `size_of::<Op>()` 96 → 32 bytes (cold
payloads boxed: NewFunction, EnterTry, template/with/sloppy-store records; named get/set carry a u32 pool index; 32-bit slot
operands). Executed op counts on all seven development programs are bit-identical to base; full gate passes (5170/5170).
Static out-of-line sites 0.8–2.9% (mostly NewFunction). Triage `20261023T041959Z-t065-opsize-triage` (done 0, 115/115 pairs,
not disturbed): seven-program development geomean **wall 0.998, instructions 0.993**; 19 controls 0.999/0.992; sentinels
1.028 wall (polymorphic_call_site 1.091, capturing_closure_call 1.141, instructions ≤1.005: layout/noise). Op fetch footprint is
not a lever on real programs, consistent with T024. Not integrated; branch retained.

## Correctness and cleanup

All three new variants pass `cargo check -p qjs-cli --features agents` and
18 CLI unit/integration tests with agents. The ownership source before its
empty-Drop lint adjustment also passes a JS object/array/string alias smoke.
Post-adjustment fast gate passes format/clippy/context/size, then fails four
intentional lifetime assertions: `replacing_registers_releases_old_ownership_and_retains_new_aliases`
(`old.is_unique()`), `named_property_cache_weakly_caches_object_values`
(`child_weak.upgrade().is_none()`), `views_add_no_size_or_owner`
(`view.is_unique()`), `duplicate_heavy_numeric_keys_release_replaced_values_during_build`
(`replaced.upgrade().is_none()`). Other **2331/2335** default runtime tests pass.
The prior no-Drop version failed clippy `drop_non_drop` at seven explicit drop
sites; an empty Drop implementation fixes the lint without releasing counts.
The failed assertions are retained, not converted to expected Test262 failures.


Local arm64/Rust 1.95.0 full gates for `3331cd7c` and `b042f9ab` both pass:
2335 default runtime tests, 21 agents tests, 24 perf-counter tests, workspace
and CLI tests, 723 benchmark-tool tests, 35 script tests, context/size/hooks,
and **5170/5170 curated Test262** each. Full gate for `6fc9c03d` passes lint,
feature checks/tests, then stops at the same four intentional default-runtime
lifetime assertions (2331 pass). It does not reach Test262. These are local
correctness results, not an x86_64 conformance run or production approval.

Both allocator full gates first stopped on absent Test262 inputs in the new
worktrees (`error: Test262 allowlist validation failed`); all earlier stages
passed. Reruns set `TEST262_DIR` to main's existing read-only checkout, whose
`4249661388e5d3f92a85186213da140a6481490f` equals the pinned gitlink. No
third-party edit, nested submodule initialization or expected-failure change.

Full gate log SHA-256 (workspace-local raw logs; the outcomes above are durable):

| Variant | Log SHA-256 |
| --- | --- |
| Rust size classes | `03cc016343ce80575494f19af78aa89500fd1c072eebdd3f23c4fc42a2c9f64c` |
| mimalloc | `cedac3e9b1949ec434b4d0a10ea9b7e49f3eef0fba66b4edf82a5b7bfe47c5d9` |
| ownership (expected failure) | `a3b6b72d4719972dbd4e64308de9e2e5185d47ab3b36e8573fadcedd4272f3f1` |

Created local sizeclass/mimalloc/ownership worktrees were cleaned with Cargo
and removed, retaining all four scratch branches (call needs no new worktree).
Local observed minimum free disk was 59 GiB, returning to 69 GiB after cleanup;
no unrelated worktree was changed. Runner builds use disposable clean worktrees
removed by the canonical builder; instruction collectors enforce 8-GiB free
space before builds and process executions.
