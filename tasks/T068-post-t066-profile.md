# T068: Post-T066 exact-binary development profiles

- Status: active; requested profiling/attribution complete; incremental proposals await lead selection.
- Implementation: none; documentation only, no runtime or tooling change.
- Verified at: measured `f49be95c798475f9ed9bb55b9138f6ca3c081413`, 2026-10-23 UTC; collection and independent content audit passed; documentation checks below.
- Evidence: this report and [method, per-program tables and receipt manifest](archive/T068-profile-evidence.md).
- Unresolved: inlined work and pre-attachment startup are not independently priced; no per-category instruction counter or NG-equivalent causal cost exists.
- Next action: select one proposal below, obtain the current formal opportunity queue, then freeze/validate its finite-budget plan before runtime edits. No rewrite is authorized.

## Result and scope

Fresh all-ten development whole-process retired-instruction geomean is **2.1199081× NG**.
The largest visible incremental families are calls/environments, property paths and Value ownership.
Execution/loop owns 44.21% of sampled self time, but includes inlined operations; it is not a dispatch price.
The [T065 lead decision](T065-interpreter-core-v3.md#lead-decision-2026-10-23) remains unchanged.
The brief's ~2.75× wall standing is historical T066 evidence, not a new wall result from this study.

Owner: this worktree, branch `agent/t068-post-t066-profile/codex`, base `f49be95c`.
Only this report, its evidence appendix and `tasks/README.md` change. No dependency, runtime,
third-party, benchmark, frozen plan, compiler recipe or acceptance-policy change; no held-out program touched.

## Collection identity

All jobs were submitted through the iMac queue from this worktree, with the remote command one quoted argument.
Status was checked first; iMac clock was 2026-10-23 12:28 CST, outside 02:45–05:00.
Only the requested clock check used direct SSH. No other job was cancelled, and no origin push or merge occurred.
Queue-internal ref transport is required by the wrapper and is not publication to origin.

| Job | Exit / role |
| --- | --- |
| `20261023T042914Z-t068-canonical-identity` | 0; canonical clean-worktree build and identity, 70 s |
| `20261023T043027Z-t068-development-exact-profiles` | 0; quiet-host sampling and separate instruction collection, 290 s |
| `20261023T043827Z-t068-profile-audit` | 0; original raw/content audit and taxonomy v1, 3 s |
| `20261023T044006Z-t068-profile-audit-v2` | 0; final taxonomy/content audit, 3 s |

- Rust executable SHA-256: `b3132b40cc52b9b39012089db873c25dc7d2a1802e249322c1891577edee58c1`.
- NG executable SHA-256: `919f7235f61412ba69faf6aa05b534869aa7d69d1583b3d7aab3cd5b67cd6f40`.
- NG source: `f7830186043e4488f2998759d60a514faf07cbc9`; macOS 12.6 x86_64, rustc 1.95.0 / LLVM 22.1.2.
- Canonical existing `build_cache` recipe: agents, opt-level 3, fat LTO, 16 CGUs, abort,
  incremental off, v0 symbols and LLVM function alignment 4. No instrumented/relinked profiler image.

Three independent runs per role/program: `sample <pid> 60 1 -mayDie -file <raw.sample>`.
Every process exited before its sampler returned, so no 60 s truncation; no amplification or tier change.
All 60 sampler/process exits are zero, all workload checksums pass, all 60 profile receipts and original
PID/workload/binary hashes verify. Main-thread samples: Rust **93,984**, NG **31,135**.
Sampling job `intruders.log` is empty. Disk stayed above 8 GiB: local observations 66 GiB;
runner before collection 62,282,219,520 bytes, after final audit 62,209,773,568 bytes. No cargo clean.

## Instruction totals

Separate unprofiled `/usr/bin/time -l` runs, three Rust/NG pairs per program, alternating role order;
60 positive retired-instruction records, stdout/rusage hashes checked. These totals include the whole process.
Rusage elapsed/cycle fields are not used for a wall claim or to allocate instructions to categories.

| Program | Rust median instructions | NG median instructions | Paired median R/NG | Three-pair range |
| --- | ---: | ---: | ---: | --- |
| lodash/transform | 16,269,684,145 | 8,120,373,184 | 2.003599 | 2.003030–2.003955 |
| meriyah/parse | 17,307,351,512 | 7,350,176,604 | 2.354646 | 2.342467–2.360722 |
| immutable/persistent | 24,458,617,580 | 10,693,793,621 | 2.287179 | 2.285365–2.291922 |
| moment/dates | 58,464,212,888 | 25,560,060,957 | 2.287266 | 2.287137–2.288312 |
| diff/compare | 48,040,842,233 | 20,274,888,196 | 2.369449 | 2.369097–2.370073 |
| papaparse/csv | 20,569,933,309 | 11,779,646,329 | 1.746536 | 1.746227–1.749109 |
| js-yaml/document | 27,295,159,450 | 9,219,342,458 | 2.960641 | 2.960489–2.962631 |
| big.js/arith | 24,050,650,303 | 12,919,927,936 | 1.861540 | 1.861234–1.861857 |
| matter-js/simulate | 30,530,189,165 | 17,659,789,809 | 1.728777 | 1.728772–1.728809 |
| graphlib/paths | 9,248,750,915 | 4,909,612,147 | 1.883629 | 1.883573–1.884146 |

## Owner categories and geomean weighting

Self = inclusive node samples minus direct-child samples; conservation asserted per main-thread tree.
Visible semantic leaf ownership wins; transparent mem/hash/vector/TLS helpers go to the nearest recognized caller.
Allocation and Value ownership stay separate when their own frames are visible. Inclusive = union of matching
frames on each sampled stack, with recursion counted once; inclusive columns overlap and must never be added.
Pool all three repeats within each program, then average its shares with weight 1/10.
This is the sensitivity weighting of the development log-geomean, not pooling long and short programs together.
Per-program counts, denominators, category rules and NG's top 25 symbols are in the evidence appendix.

| Owner category | Rust self % | Rust inclusive % | NG self % | NG inclusive % | Zero-cost R/M envelope |
| --- | ---: | ---: | ---: | ---: | ---: |
| Execution/loop, arithmetic and opaque inlining | 44.214 | 99.826 | 51.401 | 98.723 | 0.535764 |
| Named-cache probe/validation machinery | 2.463 | 3.950 | 0.000 | 0.000 | 0.975205 |
| Property storage, scan/hash and construction | 5.102 | 6.178 | 4.807 | 17.341 | 0.948505 |
| Property handlers/prototype/receiver paths | 6.381 | 18.922 | 4.822 | 16.605 | 0.935766 |
| Indexed elements/arrays/TypedArrays | 1.564 | 2.223 | 2.462 | 4.284 | 0.984091 |
| Calls, frames, arguments and tier entry/exit | 6.745 | 99.188 | 0.963 | 37.140 | 0.931869 |
| CallEnv/bindings/closures/upvalues | 8.561 | 10.907 | 0.102 | 1.985 | 0.912462 |
| Value/container clone/drop and ownership transport | 11.906 | 12.734 | 1.883 | 7.431 | 0.880538 |
| Allocation/free, including allocator TLS | 5.035 | 5.035 | 23.982 | 23.984 | 0.949307 |
| Strings, code units and interning | 0.522 | 0.673 | 2.090 | 3.629 | 0.994772 |
| RegExp | 4.242 | 6.671 | 3.499 | 7.563 | 0.953570 |
| Array builtins | 0.442 | 20.773 | 0.858 | 24.356 | 0.995571 |
| Object builtins | 0.197 | 0.720 | 0.086 | 1.548 | 0.998032 |
| String builtins | 0.887 | 3.447 | 0.845 | 4.215 | 0.991050 |
| JSON builtins | 0.000 | 0.000 | 0.000 | 0.000 | 1.000000 |
| Date builtins | 0.089 | 0.211 | 0.033 | 0.075 | 0.999109 |
| Math builtins | 0.033 | 0.110 | 0.041 | 0.041 | 0.999666 |
| Other/opaque native builtins | 1.374 | 29.240 | 1.322 | 98.733 | 0.986186 |
| Exceptions | 0.001 | 0.001 | 0.000 | 0.000 | 0.999991 |
| Visible parse/compile/startup | 0.061 | 0.139 | 0.010 | 98.715 | 0.999392 |
| GC/aggregate teardown | 0.174 | 0.379 | 0.796 | 1.866 | 0.998260 |
| Other | 0.007 | 0.007 | 0.000 | 0.000 | 0.999926 |

For category share f_i, the displayed zero-cost envelope is `exp(sum(log(1 - f_i)) / 10)`.
It holds all other sampled work fixed and deletes the whole visible category. It is a conditional reach ceiling,
not a price, forecast, confidence bound or upper bound on unobserved/inlined costs. Categories cannot be added
as speedups. NG shares describe its own visible implementation; subtracting shares or multiplying them by
total retired instructions would fabricate matched category costs. This study deliberately uses zero-cost,
rather than unsupported NG-equivalent, envelopes.

`named-cache` includes probe/entry validation, inline Weak upgrades and RefCell guards inside those functions;
they cannot be split further by these symbols. Their inlined copies in the loop remain opaque loop cost.
`value-rc` includes enum/container clone/drop and `store_owned`, not just increment/decrement instructions.
`named-other` includes named receiver/prototype paths; NG's generic Get/SetPropertyValue also accepts indices.
`builtin-other` includes Rust's opaque `call_native_family`; it is not all proven call overhead.
Runtime atom interning is strings, not compilation (corrected in audit v2); no program was resampled.

Attachment begins after process launch: early parse/bootstrap may finish before sampling starts.
Visible parse/startup and teardown entries are observations, not bounds on their whole-process cost.
Likewise, 99% inclusive calls/loops and NG eval ancestry include executed bodies, not removable entry work.
No exact division of the remaining instruction gap is identified by time sampling.

The [top 25 Rust and NG symbols](archive/T068-profile-evidence.md#top-25-rust-symbols), with self/inclusive shares and categories, are retained with the per-program evidence.

## Ranked incremental proposals

These ranks use entire-family sampled deletion envelopes. The narrower implementation can remove only a subset;
the first price must measure that subset. No frozen plan, accepted gain or attempt-budget reset is implied.
Runtime paths below are relative to `crates/qjs-runtime/src/` unless qualified with `crates/`.

| Rank | Proposed unit / measured family | R/M floor if visible family vanished | Maximum geomean reduction | Planning size / risk |
| --- | --- | ---: | ---: | --- |
| 1 | Positional call/activation preparation; calls + environments | 0.843670 | 15.63% | Medium staged unit; high binding/reentrancy risk |
| 2 | Live receiver/slot proof shared by cache, handler and storage | 0.858061 | 14.19% | Medium; high descriptor/prototype risk |
| 3 | Ownership-aware operand/register transfer | 0.880538 | 11.95% | Medium; high alias/drop-order risk |
| 4 | Reduce allocator bookkeeping/TLS entries | 0.949307 | 5.07% | Small/medium; high memory-safety/resource risk |
| 5 | Predecoded RegExp atom/group metadata | 0.953570 | 4.64% | Small/medium; high matching/capture-semantics risk |

1. **Calls/activation.** Precompute immutable parameter/capture/frame metadata per bytecode body; initialize
   positional slots directly and materialize observable call context only when required. Preserve shared upvalue
   cells, dynamic eval/with routes and callback restoration; no name-keyed per-call snapshot. Likely files:
   `function/call.rs`, `function/call_frame.rs`, `bytecode/vm_call_env.rs`, `bytecode/vm_frame_init.rs`,
   `bytecode/compact_fn/wide/activation.rs`. [T056 M2](archive/T056-m2-plan-inputs.md) already scoped entry metadata;
   [T065's call prototype](archive/T065-ablation-r1.md) priced a narrower cohort at 0.961, not this whole family.
   T021's shared-frame rewrite was rejected; do not repeat it under this unit. Lodash has 25.88% visible
   calls+environment share, immutable 22.28%, graphlib 18.44%; big.js only 1.87%.
2. **Property proof.** Return a short-lived typed proof of receiver identity/layout/storage and a live slot,
   consumed once by a named get/set; keep guard invalidation and exotic fallback, and never hold a RefCell borrow
   across user code. Likely `bytecode/named_property_cache.rs`, `bytecode/compact_fn/property.rs`, `bytecode/vm_props.rs`,
   `value/object.rs`/`value/object/dynamic_storage.rs`. The cache+handler-only cap is **8.94%**;
   14.19% also deletes storage/construction that this leaf may not affect. [T049](archive/T049-property-access-path-study.md)
   removed a duplicate probe but missed its target and regressed controls; [T041](archive/T041-own-data-cache-hot-answers.md)
   had a non-improving pair; [T053](archive/T053-ordinary-own-slot-hot-read-fat-lto.md) gained 8.3% on its target but
   failed six controls. T065's unsound lookup memo improved only diff and kept cache guards. New reach must be
   proved in validation/live-slot work, not assumed from text lookup. Full atom-store replacement stays deferred.
3. **Value transport.** Use liveness/consumption facts to move a dead source operand into its destination or call
   arguments, skipping redundant clone/drop pairs; keep cloning for aliases, captured cells and observable stores.
   Likely `bytecode/compact_fn/execute.rs`, `bytecode/compact_fn/wide/activation.rs`, `bytecode/typed_loop/execute.rs`,
   `bytecode/vm_call.rs`, `value.rs`. Clone + Value drop + `store_owned` alone occupy 11.03% weighted self time;
   all ownership transport is 11.91%. T065's count-free oracle gave 0.913 wall / 0.945 instructions on seven
   programs, but leaked and changed allocation behavior: it is no implementation price. T056 M1.2 failed on
   ownership/object-layout regressions ([diagnosis](archive/T056-m1-stage2-diagnosis.md)); this proposal changes
   transfer decisions, not Value width, heap ownership or the binding architecture.
4. **Allocator.** Simplify owner-local fast paths and amortize repeated pool access where Rust's allocation
   contract permits; preserve remote-free publication, page live counts, alignment, bounded retention and System
   fallback. Likely `crates/qjs-cli/src/allocator.rs` and its tests only. Allocate/release LocalKey closures
   contribute 3.23% self; allocator-owned TLS lookup another 0.73%. [T066](archive/T066-size-class-allocator.md)
   already landed the large allocator change; its [evidence](archive/T066-allocator-evidence.md) records repeated
   resource/sentinel failures and the eventual lead exception. No automatic exception transfers. A new unit
   must use the qualified x86 resource manifest (T067), not reopen the old allocator attempt.
5. **RegExp.** Predecode atom ends, group continuations, quantifiers and simple-atom descriptors once per
   compiled pattern; consume that metadata in the existing bounded scans and prepared-input view, preserving
   backtrack/capture order. Likely `regexp/matcher/first.rs`, `regexp/matcher/fast_scan.rs`, `regexp/matcher.rs`.
   RegExp self shares are papaparse 27.38%, js-yaml 8.87%,
   moment 3.87%, diff 1.82%; the lane-wide cap is only 4.64%. [T058](archive/T058-regexp-lazy-quantifiers.md)
   already removed the lazy-quantifier catastrophe; [T025](archive/T025-allocation-free-regexp-backtracking.md)
   stopped when controls regressed. Single-code-point scanning already exists; price repeated metadata decoding
   (`atom_end` alone 0.75% weighted self), not the fixed T058 shape or T025 replay.

Pure dispatch/Op compaction is not ranked: the 44.21% loop bucket does not isolate it, and
T065's 3× smaller Op gave 0.998 wall / 0.993 instructions; T024's preamble split was about 1% externally.
Strings without RegExp and separately visible builtins have small self reach here; opaque/inlined bodies
and missing startup samples prevent concluding that their complete costs are small. This is development
ranking only; every future unit still needs the whole selection portfolio and acceptance-only held-out gates.

## Documentation verification

At the documentation tree over measured `f49be95c`, `git diff --check`, `./scripts/check-context.py`
and `PYTHONDONTWRITEBYTECODE=1 ./scripts/check-touched.sh --staged --explain` passed on 2026-10-23 UTC.
The staged gate selected context checking and reported `docs-only change; skipping Rust and Test262 checks`.
No semantic Test262 slice applies; no scripts, Cargo inputs or executable examples changed, so the full
runtime gate is not rerun. The commit is local to the requested branch; no push or merge.
